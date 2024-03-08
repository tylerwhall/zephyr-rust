#include <uart_buffered.h>

extern void rust_main(struct uart_buffered_rx_handle rx,
		      struct uart_buffered_tx_handle tx);

static FIFO_DEFINE(test_uart_buffered_rx, 16);
static FIFO_DEFINE(test_uart_buffered_tx, 16);
UART_BUFFERED_DEFINE(test_uart, test_uart_buffered_rx, test_uart_buffered_tx);

void main(void)
{
	struct uart_buffered_rx_handle rx;
	struct uart_buffered_tx_handle tx;

	const struct device *uart = device_get_binding("UART_1");

	if (!uart) {
		printk("Failed to get uart\n");
		return;
	}

	UART_BUFFERED_INIT(&test_uart, uart);
#if CONFIG_USERSPACE
	uart_buffered_access_grant(&test_uart, k_current_get());
#endif
	rx = uart_buffered_rx_handle(&test_uart);
	tx = uart_buffered_tx_handle(&test_uart);

	rust_main(rx, tx);
}
