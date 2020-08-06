extern crate zephyr_sys;

use std::time::Duration;

use futures::io::BufReader;
use futures::task::LocalSpawnExt;
use futures::{AsyncBufReadExt, AsyncWriteExt, StreamExt};

use zephyr_futures::delay::Delay;
use zephyr_futures::Executor;
use zephyr_sys::raw::{uart_buffered_rx_handle, uart_buffered_tx_handle};
use zephyr_uart_buffered::{UartBufferedRx, UartBufferedTx};

zephyr_macros::k_mutex_define!(EXECUTOR_MUTEX);
zephyr_macros::k_poll_signal_define!(EXECUTOR_SIGNAL);

async fn echo<R: AsyncBufReadExt + Unpin, W: AsyncWriteExt + Unpin>(rx: R, mut tx: W) {
    let mut lines = rx.lines();
    while let Some(line) = lines.next().await {
        let line = line.unwrap();
        println!("got line: {}", line);
        println!("sleeping");
        Delay::new(Duration::from_secs(1)).await;
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

    let mut executor = unsafe { Executor::new(&EXECUTOR_MUTEX, &EXECUTOR_SIGNAL) };
    executor.spawn_local(echo(rx, tx)).unwrap();
    executor.run::<C>();
}
