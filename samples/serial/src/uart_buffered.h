#ifndef __UART_BUFFERED_H__
#define __UART_BUFFERED_H__

#include <zephyr.h>

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

#endif

void uart_buffered_rx_init(struct uart_buffered_rx *fifo, struct device *uart);