#include <zephyr.h>
#include <drivers/console/uart_pipe.h>

extern void rust_main(void);
extern u8_t *rust_uart_cb(u8_t *buf, size_t *off);

void main(void)
{
	u8_t buf[256];
	uart_pipe_register(buf, sizeof(buf), rust_uart_cb);
	rust_main();
}
