#include <zephyr.h>
#include <assert.h>
#include <uart.h>
#include <drivers/console/uart_pipe.h>

extern void rust_main(void);
extern u8_t *rust_uart_cb(u8_t *buf, size_t *off);

static void uart_buffered_rx_timeout(struct k_timer *timer);

struct fifo {
	u16_t write;
	u16_t read;
	u8_t buf[];
};

#define FIFO_DEFINE(name, size) u8_t name[offsetof(struct fifo, buf) + size]
#define FIFO_CAPACITY(name) (sizeof(name) - offsetof(struct fifo, buf))

struct fifo_handle {
	struct fifo *fifo;
	/* capacity - 1. Capacity must be power of 2 */
	size_t capacity_mask;
	struct device *device;
	struct k_poll_signal *signal;
};

static inline size_t fifo_capacity(struct fifo_handle *fifo)
{
	return fifo->capacity_mask + 1;
}

static inline size_t fifo_used(struct fifo_handle *fifo)
{
	return fifo->fifo->write - fifo->fifo->read;
}

static inline bool fifo_full(struct fifo_handle *fifo)
{
	return fifo_used(fifo) >= fifo_capacity(fifo);
}

static inline bool fifo_empty(struct fifo_handle *fifo)
{
	return fifo_used(fifo) == 0;
}

static inline void fifo_push(struct fifo_handle *fifo, u8_t val)
{
	assert(!fifo_full(fifo));
	fifo->fifo->buf[fifo->fifo->write++ & fifo->capacity_mask] = val;
}

static inline u8_t fifo_peek(struct fifo_handle *fifo)
{
	assert(!fifo_empty(fifo));
	return fifo->fifo->buf[fifo->fifo->read & fifo->capacity_mask];
}

static inline u8_t fifo_pop(struct fifo_handle *fifo)
{
	assert(!fifo_empty(fifo));
	return fifo->fifo->buf[fifo->fifo->read++ & fifo->capacity_mask];
}

static void fifo_handle_init(struct fifo_handle *fifo, struct device *device)
{
	/* Size must be power of 2 */
	assert(fifo->capacity_mask & (fifo->capacity_mask + 1) == 0);
	fifo->device = device;
	k_poll_signal_init(fifo->signal);
}

/* Kernel memory storage for kobjects and the kernel's fifo handle */
struct uart_buffered_rx {
	struct fifo_handle fifo;
	struct k_poll_signal signal;
	struct k_timer *const timer;
};

/* Kernel memory storage for kobjects and the kernel's fifo handle */
struct uart_buffered_tx {
	struct fifo_handle fifo;
	struct k_poll_signal signal;
};

#define FIFO_INITIALIZER(fifo_name, signal_name)                               \
	{                                                                      \
		.fifo = (struct fifo *)&fifo_name,                             \
		.capacity_mask = FIFO_CAPACITY(fifo_name) - 1, .device = NULL, \
		.signal = signal_name,                                         \
	}

#define UART_RX_FIFO_DEFINE(name, fifo_name)                                   \
	K_TIMER_DEFINE(name##_timer, uart_buffered_rx_timeout, NULL);          \
	struct uart_buffered_rx name = {                                       \
		.fifo = FIFO_INITIALIZER(fifo_name, &name.signal),             \
		.timer = &name##_timer,                                        \
	}

#define UART_TX_FIFO_DEFINE(name, fifo_name)                                   \
	struct uart_buffered_tx name = { .fifo = FIFO_INITIALIZER(             \
						 fifo_name, &name.signal) }

/* 
 * Devices can't have user data, so this function passes the static fifo
 * pointers for this specific fifo to the generic IRQ function.
 */
#define UART_FIFO_IRQ_DEFINE(name, rx, tx)                                     \
	static void name##_irq(struct device *uart)                            \
	{                                                                      \
		uart_buffered_irq(uart, &rx, &tx);                             \
	}

#define UART_BUFFERED_DEFINE(name, rx_fifo, tx_fifo)                           \
	UART_RX_FIFO_DEFINE(name##_rx, rx_fifo);                               \
	UART_TX_FIFO_DEFINE(name##_tx, tx_fifo);                               \
	UART_FIFO_IRQ_DEFINE(name, name##_rx, name##_tx)

#define UART_BUFFERED_INIT(name, uart)                                         \
	uart_buffered_init(uart, &name##_rx, &name##_tx, name##_irq)

static void uart_buffered_rx_init(struct uart_buffered_rx *fifo,
				  struct device *uart)
{
	fifo_handle_init(&fifo->fifo, uart);
	k_timer_user_data_set(fifo->timer, &fifo->signal);
}

static void uart_buffered_tx_init(struct uart_buffered_tx *fifo,
				  struct device *uart)
{
	fifo_handle_init(&fifo->fifo, uart);
}

static inline struct fifo_handle *
uart_buffered_rx_handle(struct uart_buffered_rx *fifo)
{
	return &fifo->fifo;
}

static inline struct fifo_handle *
uart_buffered_tx_handle(struct uart_buffered_tx *fifo)
{
	return &fifo->fifo;
}

static void k_poll_signal_wait(struct k_poll_signal *signal)
{
	struct k_poll_event event;

	k_poll_event_init(&event, K_POLL_TYPE_SIGNAL, K_POLL_MODE_NOTIFY_ONLY,
			  signal);
	k_poll(&event, 1, K_FOREVER);
	k_poll_signal_reset(signal);
}

static void uart_buffered_tx(struct uart_buffered_tx *uart)
{
	struct fifo_handle *fifo = uart_buffered_tx_handle(uart);

	uart_irq_tx_disable(fifo->device);

	while (!fifo_empty(fifo)) {
		u8_t c = fifo_peek(fifo);
		if (uart_fifo_fill(fifo->device, &c, 1) == 1) {
			fifo_pop(fifo);
		} else {
			uart_irq_tx_enable(fifo->device);
			break;
		}
		compiler_barrier(); /* Should be a CPU barrier on SMP, but no Zephyr API */
	}

	/* Wake the writer if the fifo is half full or less */
	if (fifo_used(fifo) <= fifo_capacity(fifo) / 2) {
		k_poll_signal_raise(fifo->signal, 0);
	}
}

static int uart_buffered_write_nb(struct fifo_handle *fifo, const u8_t *buf,
				  size_t len)
{
	u16_t current_read, current_write;
	u16_t last_read = fifo->fifo->read;
	u16_t last_write = fifo->fifo->write;
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
	 * - If fifo is non-empty, we might need to enable (current_read != current_write)
	 * - If the fifo was observed empty before we added something, we need to
	 *   enable because the transition to fifo empty would have disabled it.
	 *   (last_read == last_write)
	 * - If the fifo was observed non-empty initially, we may still need to
	 *   enable. The irq could have run and drained it at some point while we
	 *   were filling it. We can detect this by the read index changing.
	 *   (last_read != current_read)
	 */
	current_read = fifo->fifo->read;
	current_write = fifo->fifo->write;
	if (current_read != current_write &&
	    (last_read == last_write || last_read != current_read)) {
		uart_irq_tx_enable(fifo->device);
	}

	return pos;
}

static void uart_buffered_write(struct uart_buffered_tx *uart, const u8_t *buf,
				size_t len)
{
	struct fifo_handle *fifo = uart_buffered_tx_handle(uart);

	while (len) {
		int ret = uart_buffered_write_nb(fifo, buf, len);
		if (ret < 0) {
			k_poll_signal_wait(fifo->signal);
			continue;
		}
		buf += ret;
		len -= ret;
	}
}

static void uart_buffered_rx_timeout(struct k_timer *timer)
{
	struct k_poll_signal *signal = k_timer_user_data_get(timer);
	k_poll_signal_raise(signal, 0);
}

/* RX interrupt handler */
static void uart_buffered_rx(struct uart_buffered_rx *uart)
{
	struct fifo_handle *fifo = uart_buffered_rx_handle(uart);

	while (true) {
		u8_t c;
		int ret = uart_fifo_read(fifo->device, &c, 1);
		if (ret <= 0)
			break;

		if (!fifo_full(fifo)) {
			printk("Uart got %c\n", c);
			fifo_push(fifo, c);
		} else {
			printk("Uart dropped %c\n", c);
		}
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
	struct fifo_handle *fifo = uart_buffered_rx_handle(rx);
	int pos = 0;

	while (pos < len) {
		if (fifo_empty(fifo)) {
			if (pos)
				return pos;
			else
				return -EAGAIN;
		}
		buf[pos++] = fifo_pop(fifo);
	}

	return pos;
}

static size_t uart_buffered_read(struct uart_buffered_rx *rx, u8_t *buf,
				 size_t len)
{
	struct fifo_handle *fifo = uart_buffered_rx_handle(rx);
	size_t pos = 0;

	while (fifo_empty(fifo)) {
		k_poll_signal_wait(fifo->signal);
	}

	while (!fifo_empty(fifo) && pos < len) {
		buf[pos++] = fifo_pop(fifo);
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
