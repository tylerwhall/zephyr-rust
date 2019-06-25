#define __ZEPHYR_SUPERVISOR__

#include <kernel.h>
#include <uart.h>

#include "uart_buffered.h"

/* TX interrupt handler */
static void uart_buffered_tx(struct uart_buffered_tx *uart)
{
	struct fifo_handle *fifo = &uart->fifo;
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

void uart_buffered_rx_timeout(struct k_timer *timer)
{
	struct k_poll_signal *signal = k_timer_user_data_get(timer);
	k_poll_signal_raise(signal, 0);
}

/* RX interrupt handler */
static void uart_buffered_rx(struct uart_buffered_rx *uart)
{
	struct fifo_handle *fifo = &uart->fifo;
	bool disable_irq = true;

	while (!fifo_full(fifo)) {
		u8_t c;
		if (uart_fifo_read(fifo->device, &c, 1) == 1) {
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
void uart_buffered_irq(struct device *uart, struct uart_buffered_rx *rx_fifo,
		       struct uart_buffered_tx *tx_fifo)
{
	uart_irq_update(uart);
	if (uart_irq_is_pending(uart)) {
		if (uart_irq_rx_ready(uart)) {
			__ASSERT(uart == rx_fifo->fifo.device,
				 "mismatched uart and fifo");
			uart_buffered_rx(rx_fifo);
		}
		if (uart_irq_tx_ready(uart)) {
			__ASSERT(uart == tx_fifo->fifo.device,
				 "mismatched uart and fifo");
			uart_buffered_tx(tx_fifo);
		}
	}
}

static void fifo_handle_init(struct fifo_handle *fifo, struct device *device)
{
	fifo->device = device;
	k_poll_signal_init(fifo->signal);
}

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

static void uart_buffered_rx_access_grant(struct uart_buffered_rx *fifo,
					  struct k_thread *thread)
{
	k_object_access_grant(&fifo->signal, thread);
}

static void uart_buffered_tx_access_grant(struct uart_buffered_tx *fifo,
					  struct k_thread *thread)
{
	k_object_access_grant(&fifo->signal, thread);
}

void uart_buffered_access_grant(struct uart_buffered *uart,
				struct k_thread *thread)
{
	uart_buffered_rx_access_grant(&uart->rx, thread);
	uart_buffered_tx_access_grant(&uart->tx, thread);
	k_object_access_grant(uart->rx.fifo.device, thread);
}

void uart_buffered_init(struct uart_buffered *buffered, struct device *uart,
			void (*irq_handler)(struct device *uart))
{
	u8_t c;

	uart_irq_rx_disable(uart);
	uart_irq_tx_disable(uart);

	while (uart_fifo_read(uart, &c, 1)) {
	};

	uart_buffered_rx_init(&buffered->rx, uart);
	uart_buffered_tx_init(&buffered->tx, uart);

	uart_irq_callback_set(uart, irq_handler);
	uart_irq_rx_enable(uart);
}
