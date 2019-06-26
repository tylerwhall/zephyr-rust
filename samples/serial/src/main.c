#include <zephyr.h>
#include <assert.h>
#include <uart.h>
#include <drivers/console/uart_pipe.h>

extern void rust_main(void);
extern u8_t *rust_uart_cb(u8_t *buf, size_t *off);

// TODO: move these and the device under an API struct
static struct {
	// Must be power of 2;
	u8_t buf[16];
	u16_t write;
	u16_t read;
} uart_rx_fifo;

static struct k_poll_signal uart_rx_signal;
static struct k_timer uart_buffered_rx_timer;

static struct {
	// Must be power of 2;
	u8_t buf[16];
	u16_t write;
	u16_t read;
} uart_tx_fifo;

K_SEM_DEFINE(uart_tx_sem, 0, 1);
static struct k_poll_signal uart_tx_signal;

#define fifo_capacity(fifo) (sizeof((fifo)->buf))
#define fifo_mask(fifo, val) ((val) & (fifo_capacity(fifo) - 1))
#define fifo_write_advance(fifo) fifo_mask(fifo, (fifo)->write++)
#define fifo_read_advance(fifo) fifo_mask(fifo, (fifo)->read++)
#define fifo_read(fifo) fifo_mask(fifo, (fifo)->read)

#define fifo_used(fifo) ((fifo)->write - (fifo)->read)
#define fifo_full(fifo) (fifo_used(fifo) >= fifo_capacity(fifo))
#define fifo_empty(fifo) (fifo_used(fifo) == 0)
#define fifo_push(fifo, val)                                                   \
	{                                                                      \
		assert(!fifo_full(fifo));                                      \
		(fifo)->buf[fifo_write_advance(fifo)] = val;                   \
	}
#define fifo_pop(fifo)                                                         \
	({                                                                     \
		assert(!fifo_empty(fifo));                                     \
		(fifo)->buf[fifo_read_advance(fifo)];                          \
	})
#define fifo_peek(fifo)                                                        \
	({                                                                     \
		assert(!fifo_empty(fifo));                                     \
		(fifo)->buf[fifo_read(fifo)];                                  \
	})

static void k_poll_signal_wait(struct k_poll_signal *signal)
{
	struct k_poll_event event;

	k_poll_event_init(&event, K_POLL_TYPE_SIGNAL, K_POLL_MODE_NOTIFY_ONLY,
			  signal);
	k_poll(&event, 1, K_FOREVER);
	k_poll_signal_reset(&uart_rx_signal);
}

static void uart_buffered_tx(struct device *uart)
{
	uart_irq_tx_disable(uart);

	while (!fifo_empty(&uart_tx_fifo)) {
		u8_t c = fifo_peek(&uart_tx_fifo);
		if (uart_fifo_fill(uart, &c, 1) == 1) {
			fifo_pop(&uart_tx_fifo);
		} else {
			uart_irq_tx_enable(uart);
			break;
		}
		compiler_barrier(); /* Should be a CPU barrier on SMP, but no Zephyr API */
	}

	/* Wake the writer if the fifo is half full or less */
	if (fifo_used(&uart_tx_fifo) <= fifo_capacity(&uart_tx_fifo) / 2) {
		k_poll_signal_raise(&uart_tx_signal, 0);
	}
}

static int uart_buffered_write_nb(struct device *uart, const u8_t *buf,
				  size_t len)
{
	u16_t current_read, current_write;
	u16_t last_read = uart_tx_fifo.read;
	u16_t last_write = uart_tx_fifo.write;
	int pos = 0;

	compiler_barrier(); /* Should be a CPU barrier on SMP, but no Zephyr API */

	while (pos < len) {
		if (fifo_full(&uart_tx_fifo)) {
			if (pos == 0)
				pos = -EAGAIN;
			break;
		}
		fifo_push(&uart_tx_fifo, buf[pos++]);
	}

	compiler_barrier(); /* Should be a CPU barrier on SMP, but no Zephyr API */

	/*
	 * To avoid making a syscall on every write, determine if it's possible the tx irq is disabled.
	 * - If fifo is non-empty, we might need to enable (current_read != current_write)
	 * - If the fifo was observed empty before we added something, we need to
	 *   enable because the transition to fifo empty would have disabled it.
	 *   (last_read == last_write)
	 * - If the fifo was observed non-empty initially, we may still need to
	 *   enable. The irq could have run and drained it at some point while we
	 *   were filling it. We can detect this by the read index changing.
	 *   (last_read != current_read)
	 */
	current_read = uart_tx_fifo.read;
	current_write = uart_tx_fifo.write;
	if (current_read != current_write &&
	    (last_read == last_write || last_read != current_read)) {
		uart_irq_tx_enable(uart);
	}

	return pos;
}

static void uart_buffered_write(struct device *uart, const u8_t *buf,
				size_t len)
{
	while (len) {
		int ret = uart_buffered_write_nb(uart, buf, len);
		if (ret < 0) {
			k_poll_signal_wait(&uart_tx_signal);
			continue;
		}
		buf += ret;
		len -= ret;
	}
}

static void uart_buffered_rx_fifo_timeout(struct k_timer *timer)
{
	k_poll_signal_raise(&uart_rx_signal, 0);
}

static void uart_buffered_rx(struct device *uart)
{
	bool pushed = false;

	while (true) {
		u8_t c;
		int ret = uart_fifo_read(uart, &c, 1);
		if (ret <= 0)
			break;

		if (!fifo_full(&uart_rx_fifo)) {
			printk("Uart got %c\n", c);
			fifo_push(&uart_rx_fifo, c);
			pushed = true;
		} else {
			printk("Uart dropped %c\n", c);
		}
	}

	if (fifo_used(&uart_rx_fifo) >= fifo_capacity(&uart_rx_fifo) / 2) {
		/* Wake reader now if more than half full */
		k_timer_stop(&uart_buffered_rx_timer);
		k_poll_signal_raise(&uart_rx_signal, 0);
	} else if (!fifo_empty(&uart_rx_fifo)) {
		/* Make sure reader is woken eventually if any data is available */
		k_timer_start(&uart_buffered_rx_timer, K_MSEC(1), 0);
	}
}

static void uart_buffered_irq(struct device *uart)
{
	uart_irq_update(uart);
	if (uart_irq_is_pending(uart)) {
		if (uart_irq_rx_ready(uart)) {
			uart_buffered_rx(uart);
		}
		if (uart_irq_tx_ready(uart)) {
			uart_buffered_tx(uart);
		}
	}
}

static int uart_buffered_read_nb(struct device *uart, u8_t *buf, size_t len)
{
	int pos = 0;

	while (pos < len) {
		if (fifo_empty(&uart_rx_fifo)) {
			if (pos)
				return pos;
			else
				return -EAGAIN;
		}
		buf[pos++] = fifo_pop(&uart_rx_fifo);
	}

	return pos;
}

static size_t uart_buffered_read(struct device *uart, u8_t *buf, size_t len)
{
	size_t pos = 0;

	while (fifo_empty(&uart_rx_fifo)) {
		k_poll_signal_wait(&uart_rx_signal);
	}

	while (!fifo_empty(&uart_rx_fifo) && pos < len) {
		buf[pos++] = fifo_pop(&uart_rx_fifo);
	}

	return pos;
}

static void uart_buffered_setup(struct device *uart)
{
	u8_t c;

	uart_irq_rx_disable(uart);
	uart_irq_tx_disable(uart);

	while (uart_fifo_read(uart, &c, 1)) {
	};

	k_timer_init(&uart_buffered_rx_timer, uart_buffered_rx_fifo_timeout,
		     NULL);
	k_poll_signal_init(&uart_rx_signal);
	k_poll_signal_init(&uart_tx_signal);

	uart_irq_callback_set(uart, uart_buffered_irq);
	uart_irq_rx_enable(uart);
}

static const char output[] = "Test output string\n";

void main(void)
{
	struct device *uart = device_get_binding(CONFIG_UART_PIPE_ON_DEV_NAME);

	if (!uart) {
		printk("Failed to get uart\n");
		return;
	}

	uart_buffered_setup(uart);

	while (true) {
		char buf[4];
		int len;
		/* Test nonblocking */
		len = uart_buffered_read_nb(uart, buf, sizeof(buf));
		if (len == -EAGAIN) {
			printk("Waiting\n");
			k_poll_signal_wait(&uart_rx_signal);
			continue;
		}
		printk("Got: ");
		k_str_out(buf, len);
		printk("\n");

		/* Test blocking */
		len = uart_buffered_read(uart, buf, sizeof(buf));
		printk("Got: ");
		k_str_out(buf, len);
		printk("\n");

		/* Blocking write */
		uart_buffered_write(uart, (const u8_t *)output, sizeof(output));
	}

	//rust_main();
}
