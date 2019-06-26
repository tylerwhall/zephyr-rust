#include <zephyr.h>
#include <assert.h>
#include <uart.h>
#include <drivers/console/uart_pipe.h>

extern void rust_main(void);
extern u8_t *rust_uart_cb(u8_t *buf, size_t *off);

static struct {
    // Must be power of 2;
    u8_t buf[16];
    u16_t write;
    u16_t read;
} uart_rx_fifo;

K_SEM_DEFINE(uart_rx_sem, 0, 1);

#define fifo_capacity(fifo) (sizeof((fifo)->buf))
#define fifo_mask(fifo, val) ((val) & (fifo_capacity(fifo) - 1))
#define fifo_write_advance(fifo) fifo_mask(fifo, (fifo)->write++)
#define fifo_read_advance(fifo) fifo_mask(fifo, (fifo)->read++)

#define fifo_used(fifo) ((fifo)->write - (fifo)->read)
#define fifo_full(fifo) (fifo_used(fifo) >= fifo_capacity(fifo))
#define fifo_empty(fifo) (fifo_used(fifo) == 0)
#define fifo_push(fifo, val) { assert(!fifo_full(fifo)); (fifo)->buf[fifo_write_advance(fifo)] = val; }
#define fifo_pop(fifo) ({ assert(!fifo_empty(fifo)); (fifo)->buf[fifo_read_advance(fifo)]; })

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

    if (pushed) {
        k_sem_give(&uart_rx_sem);
    }
}

static void uart_buffered_irq(struct device *uart)
{
    uart_irq_update(uart);
    if (uart_irq_is_pending(uart)) {
        if (uart_irq_rx_ready(uart)) {
            uart_buffered_rx(uart);
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
        k_sem_take(&uart_rx_sem, K_FOREVER);
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

    while (uart_fifo_read(uart, &c, 1)) {};

    uart_irq_callback_set(uart, uart_buffered_irq);
    uart_irq_rx_enable(uart);
}

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
            k_sem_take(&uart_rx_sem, K_FOREVER);
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
    }

    //rust_main();
}
