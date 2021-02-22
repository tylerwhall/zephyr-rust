extern crate zephyr_core;
extern crate zephyr_sys;

use std::io;

pub use zephyr_core::*;

mod macros;

pub mod device;
pub mod eeprom;
pub mod gpio;
pub mod uart;

trait NegErrno: NegErr {
    fn neg_errno(&self) -> io::Result<u32>;
    fn zero_or_neg_errno(&self) -> io::Result<()>;
}

impl NegErrno for i32 {
    fn neg_errno(&self) -> io::Result<u32> {
        self.neg_err()
            .map_err(|e| io::Error::from_raw_os_error(e as i32))
    }

    fn zero_or_neg_errno(&self) -> io::Result<()> {
        self.neg_errno().map(|_| ())
    }
}
