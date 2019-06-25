extern crate zephyr;

use std::io::{Read, Write};
use std::time::Duration;

#[no_mangle]
pub extern "C" fn rust_main() {
    let mut serial = zephyr::uart::UartDevice {};

    loop {
        let x = "hello\n";
        let mut buf = [0u8; 32];
        let res = serial.write(x.as_bytes());
        println!("write: result {:?}", res);
        match serial.read(&mut buf) {
            Ok(n) => {
                let mystr = std::str::from_utf8(&buf).unwrap();
                println!("read: n={} mystr={}", n, mystr);
            },
            Err(rc) => println!("read: err rc={}", rc),
        }
        std::thread::sleep(Duration::from_millis(1000));
    }
}
