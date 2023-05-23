#ifndef __UART_BUFFERED_H__
#define __UART_BUFFERED_H__

#include <version.h>
#if KERNEL_VERSION_MAJOR < 3
#include <zephyr.h>
#include <kernel.h>
#else
#include <zephyr/kernel.h>
#endif

typedef uint16_t fifo_index_t;
struct fifo {
	fifo_index_t write;
	fifo_index_t read;
	uint8_t buf[];
};

#define FIFO_DEFINE(name, size)                                                \
	uint8_t name[offsetof(struct fifo, buf) + (size)];                     \
	BUILD_ASSERT(((size) & ((size)-1)) == 0,                               \
		     "fifo size must be a power of 2")

#define FIFO_CAPACITY(name) (sizeof(name) - offsetof(struct fifo, buf))

/*
 * Copyable set of references needed to operate on the fifo. The kernel owns
 * the primary copy in kernel memory which can't be modififed by un-trusted
 * code. This can be copied for access by user space if the thread is granted
 * access to the kernel objects and the fifo memory.
 */
struct fifo_handle {
	struct fifo *fifo;
	/* capacity - 1. Capacity must be power of 2 */
	size_t capacity_mask;
	struct device *device;
	struct k_poll_signal *signal;
};

/* New type to prevent mixing rx and tx functions */
struct uart_buffered_rx_handle {
	struct fifo_handle fifo;
};

/* New type to prevent mixing rx and tx functions */
struct uart_buffered_tx_handle {
	struct fifo_handle fifo;
};

static inline size_t fifo_capacity(struct fifo_handle *fifo)
{
	return fifo->capacity_mask + 1;
}

static inline size_t fifo_used(struct fifo_handle *fifo)
{
	return (fifo_index_t)(fifo->fifo->write - fifo->fifo->read);
}

static inline bool fifo_full(struct fifo_handle *fifo)
{
	return fifo_used(fifo) >= fifo_capacity(fifo);
}

static inline bool fifo_empty(struct fifo_handle *fifo)
{
	return fifo_used(fifo) == 0;
}

static inline void fifo_push(struct fifo_handle *fifo, uint8_t val)
{
	__ASSERT(!fifo_full(fifo), "push to full fifo");
	fifo->fifo->buf[fifo->fifo->write & fifo->capacity_mask] = val;
	compiler_barrier(); /* Should be a CPU barrier on SMP, but no Zephyr API */
	fifo->fifo->write++;
}

static inline uint8_t fifo_peek(struct fifo_handle *fifo)
{
	__ASSERT(!fifo_empty(fifo), "peek from empty fifo");
	return fifo->fifo->buf[fifo->fifo->read & fifo->capacity_mask];
}

static inline uint8_t fifo_pop(struct fifo_handle *fifo)
{
	__ASSERT(!fifo_empty(fifo), "pop from empty fifo");
	uint8_t ret = fifo->fifo->buf[fifo->fifo->read & fifo->capacity_mask];
	compiler_barrier(); /* Should be a CPU barrier on SMP, but no Zephyr API */
	fifo->fifo->read++;
	return ret;
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

struct uart_buffered {
	struct uart_buffered_rx rx;
	struct uart_buffered_tx tx;
};

static inline struct uart_buffered_rx_handle
uart_buffered_rx_handle(struct uart_buffered *uart)
{
	struct uart_buffered_rx_handle handle;

	handle.fifo = uart->rx.fifo;

	return handle;
}

static inline struct uart_buffered_tx_handle
uart_buffered_tx_handle(struct uart_buffered *uart)
{
	struct uart_buffered_tx_handle handle;

	handle.fifo = uart->tx.fifo;

	return handle;
}

#define FIFO_INITIALIZER(fifo_name, signal_name)                               \
	{                                                                      \
		.fifo = (struct fifo *)&fifo_name,                             \
		.capacity_mask = FIFO_CAPACITY(fifo_name) - 1, .device = NULL, \
		.signal = signal_name,                                         \
	}

/* 
 * Devices can't have user data, so this function passes the static fifo
 * pointers for this specific fifo to the generic IRQ function.
 */
#define UART_FIFO_IRQ_DEFINE(name, rx, tx)                                     \
	static void name##_irq(struct device *uart)                            \
	{                                                                      \
		uart_buffered_irq(uart, rx, tx);                               \
	}

#define UART_BUFFERED_DEFINE(name, rx_fifo, tx_fifo)                           \
	K_TIMER_DEFINE(name##_timer, uart_buffered_rx_timeout, NULL);          \
	struct uart_buffered name =                                            \
		{ .rx =                                                        \
			  {                                                    \
				  .fifo = FIFO_INITIALIZER(rx_fifo,            \
							   &name.rx.signal),   \
				  .timer = &name##_timer,                      \
			  },                                                   \
		  .tx = {                                                      \
			  .fifo = FIFO_INITIALIZER(tx_fifo, &name.tx.signal),  \
		  } };                                                         \
	UART_FIFO_IRQ_DEFINE(name, &name.rx, &name.tx)

#define UART_BUFFERED_INIT(name, uart)                                         \
	uart_buffered_init(name, uart, name##_irq)

/* Invoked by macros */
void uart_buffered_rx_timeout(struct k_timer *timer);
void uart_buffered_irq(struct device *uart, struct uart_buffered_rx *rx_fifo,
		       struct uart_buffered_tx *tx_fifo);

/* API */
int uart_buffered_write_nb(struct uart_buffered_tx_handle *tx, const uint8_t *buf,
			   size_t len);
void uart_buffered_write(struct uart_buffered_tx_handle *tx, const uint8_t *buf,
			 size_t len);
int uart_buffered_read_nb(struct uart_buffered_rx_handle *rx, uint8_t *buf,
			  size_t len);
size_t uart_buffered_read(struct uart_buffered_rx_handle *rx, uint8_t *buf,
			  size_t len);

static inline void uart_buffered_rx_access_grant(struct uart_buffered_rx *fifo,
						 struct k_thread *thread)
{
	k_object_access_grant(&fifo->signal, thread);
}

static inline void uart_buffered_tx_access_grant(struct uart_buffered_tx *fifo,
						 struct k_thread *thread)
{
	k_object_access_grant(&fifo->signal, thread);
}

static inline void uart_buffered_access_grant(struct uart_buffered *uart,
					      struct k_thread *thread)
{
	uart_buffered_rx_access_grant(&uart->rx, thread);
	uart_buffered_tx_access_grant(&uart->tx, thread);
	k_object_access_grant(uart->rx.fifo.device, thread);
}

static inline void fifo_handle_init(struct fifo_handle *fifo, struct device *device)
{
	fifo->device = device;
	k_poll_signal_init(fifo->signal);
}

static inline void uart_buffered_rx_init(struct uart_buffered_rx *fifo,
					 struct device *uart)
{
	fifo_handle_init(&fifo->fifo, uart);
	k_timer_user_data_set(fifo->timer, &fifo->signal);
}

static inline void uart_buffered_tx_init(struct uart_buffered_tx *fifo,
					 struct device *uart)
{
	fifo_handle_init(&fifo->fifo, uart);
}

static inline void uart_buffered_init(struct uart_buffered *buffered, struct device *uart,
				      void (*irq_handler)(struct device *uart))
{
	uint8_t c;

	uart_irq_rx_disable(uart);
	uart_irq_tx_disable(uart);

	while (uart_fifo_read(uart, &c, 1)) {
	};

	uart_buffered_rx_init(&buffered->rx, uart);
	uart_buffered_tx_init(&buffered->tx, uart);

	uart_irq_callback_set(uart, irq_handler);
	uart_irq_err_enable(uart);
	uart_irq_rx_enable(uart);
}

#endif
