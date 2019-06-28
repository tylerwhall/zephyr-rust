#include <zephyr.h>
#include <assert.h>
#include <uart.h>
#include <drivers/console/uart_pipe.h>

#include "uart_buffered.h"

extern void rust_main(void);
extern u8_t *rust_uart_cb(u8_t *buf, size_t *off);

void uart_buffered_rx_timeout(struct k_timer *timer);

void uart_buffered_rx_init(struct uart_buffered_rx *fifo, struct device *uart)
{
	fifo_handle_init(&fifo->fifo, uart);
	k_timer_user_data_set(fifo->timer, &fifo->signal);
}

static void uart_buffered_tx_init(struct uart_buffered_tx *fifo,
				  struct device *uart)
{
	fifo_handle_init(&fifo->fifo, uart);
}

static void k_poll_signal_wait(struct k_poll_signal *signal)
{
	struct k_poll_event event;

	k_poll_event_init(&event, K_POLL_TYPE_SIGNAL, K_POLL_MODE_NOTIFY_ONLY,
			  signal);
	k_poll(&event, 1, K_FOREVER);
	k_poll_signal_reset(signal);
}

/* TX interrupt handler */
static void uart_buffered_tx(struct uart_buffered_tx *uart)
{
	struct fifo_handle *fifo = uart_buffered_tx_handle(uart);
	bool disable_irq = true;

	while (!fifo_empty(fifo)) {
		u8_t c = fifo_peek(fifo);
		if (uart_fifo_fill(fifo->device, &c, 1) == 1) {
			fifo_pop(fifo);
		} else {
			/* Still want to send more (fifo not empty) */
			disable_irq = false;
			break;
		}
		compiler_barrier(); /* Should be a CPU barrier on SMP, but no Zephyr API */
	}

	if (disable_irq) {
		uart_irq_tx_disable(fifo->device);
	}

	/* Wake the writer if the fifo is half full or less */
	if (fifo_used(fifo) <= fifo_capacity(fifo) / 2) {
		k_poll_signal_raise(fifo->signal, 0);
	}
}

static int uart_buffered_write_nb(struct uart_buffered_tx *tx, const u8_t *buf,
				  size_t len)
{
	u16_t current_read;
	struct fifo_handle *fifo = uart_buffered_tx_handle(tx);
	u16_t last_read = fifo->fifo->read;
	bool was_empty = fifo_empty(fifo);
	int pos = 0;

	compiler_barrier(); /* Should be a CPU barrier on SMP, but no Zephyr API */

	while (pos < len) {
		if (fifo_full(fifo)) {
			if (pos == 0)
				pos = -EAGAIN;
			break;
		}
		fifo_push(fifo, buf[pos++]);
	}

	compiler_barrier(); /* Should be a CPU barrier on SMP, but no Zephyr API */

	/*
	 * To avoid making a syscall on every write, determine if it's possible the tx irq is disabled.
	 * - If fifo is non-empty, we might need to enable
	 * - If the fifo was observed empty before we added something, we need to
	 *   enable because the transition to fifo empty would have disabled it.
	 * - If the fifo was changed by the irq handler between observations, we can't
	 *   be sure if it became empty in the handler and was disabled, so we must
	 *   enable it.
	 */
	current_read = fifo->fifo->read;
	if (!fifo_empty(fifo) && (was_empty || last_read != current_read)) {
		uart_irq_tx_enable(fifo->device);
	}

	return pos;
}

static void uart_buffered_write(struct uart_buffered_tx *tx, const u8_t *buf,
				size_t len)
{
	struct fifo_handle *fifo = uart_buffered_tx_handle(tx);

	while (len) {
		int ret = uart_buffered_write_nb(tx, buf, len);
		if (ret < 0) {
			k_poll_signal_wait(fifo->signal);
			continue;
		}
		buf += ret;
		len -= ret;
	}
}

void uart_buffered_rx_timeout(struct k_timer *timer)
{
	struct k_poll_signal *signal = k_timer_user_data_get(timer);
	k_poll_signal_raise(signal, 0);
}

/* RX interrupt handler */
static void uart_buffered_rx(struct uart_buffered_rx *uart)
{
	struct fifo_handle *fifo = uart_buffered_rx_handle(uart);
	bool disable_irq = true;

	while (!fifo_full(fifo)) {
		u8_t c;
		if (uart_fifo_read(fifo->device, &c, 1) == 1) {
			printk("Uart got %c\n", c);
			fifo_push(fifo, c);
		} else {
			/* Still want to receive more (fifo not full) */
			disable_irq = false;
			break;
		}
	}

	if (disable_irq) {
		uart_irq_rx_disable(fifo->device);
	}

	if (fifo_used(fifo) >= fifo_capacity(fifo) / 2) {
		/* Wake reader now if more than half full */
		k_timer_stop(uart->timer);
		k_poll_signal_raise(fifo->signal, 0);
	} else if (!fifo_empty(fifo)) {
		/* Make sure reader is woken eventually if any data is available */
		k_timer_start(uart->timer, K_MSEC(1), 0);
	}
}

/* UART IRQ handler (RX and TX) */
static void uart_buffered_irq(struct device *uart,
			      struct uart_buffered_rx *rx_fifo,
			      struct uart_buffered_tx *tx_fifo)
{
	uart_irq_update(uart);
	if (uart_irq_is_pending(uart)) {
		if (uart_irq_rx_ready(uart)) {
			assert(uart == rx_fifo->fifo.device);
			uart_buffered_rx(rx_fifo);
		}
		if (uart_irq_tx_ready(uart)) {
			assert(uart == tx_fifo->fifo.device);
			uart_buffered_tx(tx_fifo);
		}
	}
}

static int uart_buffered_read_nb(struct uart_buffered_rx *rx, u8_t *buf,
				 size_t len)
{
	u16_t current_write;
	struct fifo_handle *fifo = uart_buffered_rx_handle(rx);
	u16_t last_write = fifo->fifo->write;
	bool was_full = fifo_full(fifo);
	int pos = 0;

	compiler_barrier(); /* Should be a CPU barrier on SMP, but no Zephyr API */

	while (pos < len) {
		if (fifo_empty(fifo)) {
			if (pos == 0)
				pos = -EAGAIN;
			break;
		}
		buf[pos++] = fifo_pop(fifo);
	}

	compiler_barrier(); /* Should be a CPU barrier on SMP, but no Zephyr API */

	/*
	 * To avoid making a syscall on every read, determine if it's possible the rx irq is disabled.
	 * - If fifo is not full, we might need to enable
	 * - If the fifo was observed full before we added something, we need to
	 *   enable because the transition to fifo full would have disabled it.
	 * - If the fifo was changed by the irq handler between observations, we can't
	 *   be sure if it became full in the handler and was disabled, so we must
	 *   enable it.
	 */
	current_write = fifo->fifo->write;
	if (!fifo_full(fifo) && (was_full || last_write != current_write)) {
		uart_irq_rx_enable(fifo->device);
	}

	return pos;
}

static size_t uart_buffered_read(struct uart_buffered_rx *rx, u8_t *buf,
				 size_t len)
{
	struct fifo_handle *fifo = uart_buffered_rx_handle(rx);
	size_t pos = 0;

	while (len) {
		int ret = uart_buffered_read_nb(rx, buf, len);
		if (ret < 0) {
			if (pos > 0) {
				/* Return if we have anything */
				break;
			} else {
				/* Wait until we have something */
				k_poll_signal_wait(fifo->signal);
				continue;
			}
		}
		buf += ret;
		len -= ret;
	}

	return pos;
}

static void uart_buffered_init(struct device *uart,
			       struct uart_buffered_rx *rx_fifo,
			       struct uart_buffered_tx *tx_fifo,
			       void (*irq_handler)(struct device *uart))
{
	u8_t c;

	uart_irq_rx_disable(uart);
	uart_irq_tx_disable(uart);

	while (uart_fifo_read(uart, &c, 1)) {
	};

	uart_buffered_rx_init(rx_fifo, uart);
	uart_buffered_tx_init(tx_fifo, uart);

	uart_irq_callback_set(uart, irq_handler);
	uart_irq_rx_enable(uart);
}

static FIFO_DEFINE(test_uart_buffered_rx, 16);
static FIFO_DEFINE(test_uart_buffered_tx, 16);
UART_BUFFERED_DEFINE(test_uart, test_uart_buffered_rx, test_uart_buffered_tx);

static const char output[] = "Test output string\n";

void main(void)
{
	struct device *uart = device_get_binding(CONFIG_UART_PIPE_ON_DEV_NAME);

	if (!uart) {
		printk("Failed to get uart\n");
		return;
	}

	UART_BUFFERED_INIT(test_uart, uart);

	while (true) {
		char buf[4];
		int len;
		/* Test nonblocking */
		len = uart_buffered_read_nb(&test_uart_rx, buf, sizeof(buf));
		if (len == -EAGAIN) {
			printk("Waiting\n");
			k_poll_signal_wait(&test_uart_rx.signal);
			continue;
		}
		printk("Got: ");
		k_str_out(buf, len);
		printk("\n");

		/* Test blocking */
		len = uart_buffered_read(&test_uart_rx, buf, sizeof(buf));
		printk("Got: ");
		k_str_out(buf, len);
		printk("\n");

		/* Blocking write */
		uart_buffered_write(&test_uart_tx, (const u8_t *)output,
				    sizeof(output));
	}

	//rust_main();
}
