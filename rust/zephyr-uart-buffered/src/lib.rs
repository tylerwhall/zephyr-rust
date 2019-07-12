extern crate zephyr_core;
extern crate zephyr_sys;

use zephyr_core::poll::KPollSignal;
use zephyr_core::NegErr;
use zephyr_sys::raw::{uart_buffered_rx_handle, uart_buffered_tx_handle};

mod futures;

pub use crate::futures::{UartBufferedRxAsync, UartBufferedTxAsync};

pub struct UartBufferedRx {
    handle: uart_buffered_rx_handle,
}

impl UartBufferedRx {
    /// Unsafe because this is passed from C and caller must guarantee there is
    /// only one instance created per handle.
    pub unsafe fn new(handle: uart_buffered_rx_handle) -> Self {
        UartBufferedRx { handle }
    }

    /// Infallible read (lower level than `std::io::Read`)
    pub fn read(&mut self, buf: &mut [u8]) -> usize {
        unsafe {
            zephyr_sys::raw::uart_buffered_read(
                &self.handle as *const _ as *mut _,
                buf.as_mut_ptr(),
                buf.len(),
            )
        }
    }

    /// Non blocking read. Returns None if nothing read, else returns number of bytes read.
    pub fn read_nb(&mut self, buf: &mut [u8]) -> Option<usize> {
        unsafe {
            zephyr_sys::raw::uart_buffered_read_nb(
                &self.handle as *const _ as *mut _,
                buf.as_mut_ptr(),
                buf.len(),
            )
        }
        .neg_err()
        .ok()
        .map(|len| len as usize)
    }

    /// Get reference to signal to wait on for non-blocking readiness. Static
    /// lifetime because uart buffered can only be declared statically.
    pub fn get_signal(&self) -> &'static KPollSignal {
        unsafe { &*self.handle.fifo.signal }
    }

    pub fn into_async(self) -> UartBufferedRxAsync {
        UartBufferedRxAsync::new(self)
    }
}

pub struct UartBufferedTx {
    handle: uart_buffered_tx_handle,
}

impl UartBufferedTx {
    /// Unsafe because this is passed from C and caller must guarantee there is
    /// only one instance created per handle.
    pub unsafe fn new(handle: uart_buffered_tx_handle) -> Self {
        UartBufferedTx { handle }
    }

    /// Infallible write (lower level than `std::io::Write`)
    pub fn write(&mut self, buf: &[u8]) {
        unsafe {
            zephyr_sys::raw::uart_buffered_write(
                &self.handle as *const _ as *mut _,
                buf.as_ptr(),
                buf.len(),
            )
        }
    }

    /// Non blocking write. Returns None if nothing written, else returns number of bytes written.
    pub fn write_nb(&mut self, buf: &[u8]) -> Option<usize> {
        unsafe {
            zephyr_sys::raw::uart_buffered_write_nb(
                &self.handle as *const _ as *mut _,
                buf.as_ptr(),
                buf.len(),
            )
        }
        .neg_err()
        .ok()
        .map(|len| len as usize)
    }

    /// Get reference to signal to wait on for non-blocking readiness. Static
    /// lifetime because uart buffered can only be declared statically.
    pub fn get_signal(&self) -> &'static KPollSignal {
        unsafe { &*self.handle.fifo.signal }
    }

    pub fn into_async(self) -> UartBufferedTxAsync {
        UartBufferedTxAsync::new(self)
    }
}
