#include <kernel.h>
#include <drivers/uart.h>

#include "uart_buffered.h"

static void k_poll_signal_wait(struct k_poll_signal *signal)
{
	struct k_poll_event event;

	k_poll_event_init(&event, K_POLL_TYPE_SIGNAL, K_POLL_MODE_NOTIFY_ONLY,
			  signal);
	k_poll(&event, 1, K_FOREVER);
	k_poll_signal_reset(signal);
}

int uart_buffered_write_nb(struct uart_buffered_tx_handle *tx, const uint8_t *buf,
			   size_t len)
{
	fifo_index_t current_read;
	struct fifo_handle *fifo = &tx->fifo;
	fifo_index_t last_read = fifo->fifo->read;
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

void uart_buffered_write(struct uart_buffered_tx_handle *tx, const uint8_t *buf,
			 size_t len)
{
	struct fifo_handle *fifo = &tx->fifo;

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

int uart_buffered_read_nb(struct uart_buffered_rx_handle *rx, uint8_t *buf,
			  size_t len)
{
	fifo_index_t current_write;
	struct fifo_handle *fifo = &rx->fifo;
	fifo_index_t last_write = fifo->fifo->write;
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

size_t uart_buffered_read(struct uart_buffered_rx_handle *rx, uint8_t *buf,
			  size_t len)
{
	struct fifo_handle *fifo = &rx->fifo;
	uint8_t *orig_buf = buf;

	while (len) {
		int ret = uart_buffered_read_nb(rx, buf, len);
		if (ret < 0) {
			if (buf != orig_buf) {
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

	return buf - orig_buf;
}
