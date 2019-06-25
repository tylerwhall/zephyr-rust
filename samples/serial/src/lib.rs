extern crate zephyr;
extern crate zephyr_sys;
extern crate zephyr_uart_buffered;

use zephyr_sys::raw::{uart_buffered_rx_handle, uart_buffered_tx_handle};
use zephyr_uart_buffered::{UartBufferedRx, UartBufferedTx};

#[no_mangle]
pub extern "C" fn rust_main(rx: uart_buffered_rx_handle, tx: uart_buffered_tx_handle) {
    let mut rx = unsafe { UartBufferedRx::new(rx) };
    let mut tx = unsafe { UartBufferedTx::new(tx) };

    loop {
        const X: &str = "hello\n";
        tx.write(X.as_bytes());

        let mut buf = [0u8; 32];
        let n = rx.read(&mut buf);
        let mystr = std::str::from_utf8(&buf[..n]).unwrap();
        println!("read: n={} mystr={}", n, mystr);
    }
}
