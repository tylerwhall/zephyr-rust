#define __ZEPHYR_SUPERVISOR__

#include <kernel.h>
#include <drivers/uart.h>
#include <logging/log.h>

#include "uart_buffered.h"

LOG_MODULE_REGISTER(uart_buffered);

/* TX interrupt handler */
static void uart_buffered_tx(struct uart_buffered_tx *uart)
{
	struct fifo_handle *fifo = &uart->fifo;
	bool disable_irq = true;

	while (!fifo_empty(fifo)) {
		uint8_t c = fifo_peek(fifo);
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
	LOG_DBG("rx wake on timer");
	k_poll_signal_raise(signal, 0);
}

/* RX interrupt handler */
static void uart_buffered_rx(struct uart_buffered_rx *uart)
{
	struct fifo_handle *fifo = &uart->fifo;
	bool disable_irq = true;

	while (!fifo_full(fifo)) {
		uint8_t c;
		if (uart_fifo_read(fifo->device, &c, 1) == 1) {
			fifo_push(fifo, c);
			LOG_DBG("uart byte 0x%x write = %d read = %d used = %d\n",
				c, fifo->fifo->write,
				fifo->fifo->read,
				fifo->fifo->write - fifo->fifo->read);
		} else {
			/* Still want to receive more (fifo not full) */
			disable_irq = false;
			break;
		}
	}

	if (disable_irq) {
		LOG_DBG("disable rx irq");
		uart_irq_rx_disable(fifo->device);
	}

	if (fifo_used(fifo) >= fifo_capacity(fifo) / 2) {
		/* Wake reader now if more than half full */
		k_timer_stop(uart->timer);
		LOG_DBG("rx wake");
		k_poll_signal_raise(fifo->signal, 0);
	} else if (!fifo_empty(fifo)) {
		/* Make sure reader is woken eventually if any data is available */
		LOG_DBG("rx timer start");
		k_timer_start(uart->timer, K_MSEC(1), Z_TIMEOUT_NO_WAIT);
	}
}

/* UART IRQ handler (RX and TX) */
void uart_buffered_irq(struct device *uart, struct uart_buffered_rx *rx_fifo,
		       struct uart_buffered_tx *tx_fifo)
{
	while (uart_irq_update(uart) && uart_irq_is_pending(uart)) {
		int err = uart_err_check(uart);
		if (err & UART_ERROR_OVERRUN) {
			LOG_ERR("overrun");
		}
		if (err & UART_ERROR_FRAMING) {
			LOG_ERR("framing");
		}
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
