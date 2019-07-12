#![feature(async_await)]

extern crate futures;
extern crate zephyr;
extern crate zephyr_sys;
extern crate zephyr_uart_buffered;
extern crate zephyr_macros;
extern crate zephyr_futures;

use futures::{StreamExt, AsyncBufReadExt, AsyncWriteExt};
use futures::io::BufReader;

use zephyr_sys::raw::{uart_buffered_rx_handle, uart_buffered_tx_handle};
use zephyr_uart_buffered::{UartBufferedRx, UartBufferedTx};
use zephyr_futures::Executor;

zephyr_macros::k_mutex_define!(EXECUTOR_MUTEX);

async fn echo<R: AsyncBufReadExt + Unpin, W: AsyncWriteExt + Unpin>(rx: R, mut tx: W) {
    let mut lines = rx.lines();
    while let Some(line) = lines.next().await {
        let line = line.unwrap();
        println!("got line: {}", line);
        let line = line.into_bytes();
        tx.write_all(&line).await.unwrap();
    }
}

#[no_mangle]
pub extern "C" fn rust_main(rx: uart_buffered_rx_handle, tx: uart_buffered_tx_handle) {
    use zephyr::context::Kernel as C;

    let rx = unsafe { UartBufferedRx::new(rx) };
    let rx = BufReader::with_capacity(32, rx.into_async());

    let tx = unsafe { UartBufferedTx::new(tx) }.into_async();

    let mut executor = unsafe { Executor::new(&EXECUTOR_MUTEX) };
    executor.spawn(C, echo(rx, tx));
    executor.run::<C>();
}
