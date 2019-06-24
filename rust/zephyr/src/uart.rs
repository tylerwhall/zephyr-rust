use super::NegErr;
use crate::device::Device;

use std::cmp;
use std::io;
use std::io::{Read, Write, Error, ErrorKind};

pub trait UartSyscalls {
    fn uart_poll_out(device: &Device, out_char: char);

    fn uart_poll_in(device: &Device) -> Result<Option<char>, u32>;

    fn uart_err_check(device: &Device) -> Option<u32>;

    fn uart_config_get(device: &Device) -> Result<UartConfig, u32>;

    fn uart_configure(device: &Device, config: &UartConfig) -> Result<(), u32>;
}

type UartCallback = unsafe extern "C" fn(*mut u8, *mut usize) -> *mut u8;

fn uart_pipe_register(buf: &mut [u8], cb: UartCallback) {
    unsafe {
        zephyr_sys::raw::uart_pipe_register(buf.as_mut_ptr(), buf.len(), Some(cb))
    };
}

pub struct UartDevice;

const SERIAL_BUF_SIZE: usize = 4096;
static mut SERIAL_BUF: [u8; SERIAL_BUF_SIZE] = [0; SERIAL_BUF_SIZE];
static mut SERIAL_WRITE_INDEX: usize = 0;
static mut SERIAL_READ_INDEX: usize = 0;

/// Returns the amount of space that can be written into the serial buffer including wrapping.
unsafe fn serial_buf_space() -> usize {
    (SERIAL_READ_INDEX - SERIAL_WRITE_INDEX - 1) & (SERIAL_BUF_SIZE - 1)
}

/// Returns the amount of data available for reading in the serial buffer without wrapping.
unsafe fn serial_buf_count_to_end() -> usize {
    let end = SERIAL_BUF_SIZE - SERIAL_READ_INDEX;
    let n = (SERIAL_WRITE_INDEX + end) & (SERIAL_BUF_SIZE - 1);
    if n < end { n } else { end }
}

#[no_mangle]
unsafe extern "C" fn rust_uart_cb(buf: *mut u8, off: *mut usize) -> *mut u8 {
    let array = std::slice::from_raw_parts(buf, *off);
    let space = serial_buf_space();
    let idx = SERIAL_WRITE_INDEX;
    if space < *off {
        // XXX: do we need to handle this?
        panic!("serial overflow")
    }
    if idx + *off < SERIAL_BUF_SIZE {
        // easy case, copy in one chunk
        SERIAL_BUF[idx..idx+*off].copy_from_slice(&array);
    } else {
        // wrap, copy in two chunks
        let len = SERIAL_BUF_SIZE - idx;
        SERIAL_BUF[idx..SERIAL_BUF_SIZE].copy_from_slice(&array[..len]);
        SERIAL_BUF[0..*off-len].copy_from_slice(&array[len..]);
    }
    SERIAL_WRITE_INDEX = (SERIAL_WRITE_INDEX + *off) & (SERIAL_BUF_SIZE - 1);
    *off = 0;
    return buf;
}

impl Read for UartDevice {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        unsafe {
            let avail = serial_buf_count_to_end();
            let idx = SERIAL_READ_INDEX;
            let amt = cmp::min(buf.len(), avail);
            if amt == 0 {
                return Err(Error::new(ErrorKind::WouldBlock, "would block"));
            }
            buf[..amt].copy_from_slice(&SERIAL_BUF[idx..idx+amt]);
            SERIAL_READ_INDEX = (SERIAL_READ_INDEX + amt) & (SERIAL_BUF_SIZE - 1);
            return Ok(amt);
        }
    }
}

impl Write for UartDevice {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        unsafe {
            zephyr_sys::raw::uart_pipe_send(
                buf.as_ptr(),
                buf.len() as i32
            )
        };

        // uart_pipe_send returns an int but it is always 0.
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}


macro_rules! trait_impl {
    ($context:ident, $context_struct:path) => {
        impl UartSyscalls for $context_struct {
            #[inline(always)]
            fn uart_poll_out(device: &Device, out_char: char) {
                unsafe {
                    zephyr_sys::syscalls::$context::uart_poll_out(
                        device as *const _ as *mut _,
                        out_char as u8,
                    )
                };
            }

            #[inline(always)]
            fn uart_poll_in(device: &Device) -> Result<Option<char>, u32> {
                let mut munge: u8 = 0;
                let rc = unsafe {
                    zephyr_sys::syscalls::$context::uart_poll_in(
                        device as *const _ as *mut _,
                        &mut munge,
                    )
                }
                .neg_err()
                .map(|_| munge as char);

                // remap a return value of -1 from uart_poll_in() to Ok(None)
                match rc {
                    Ok(c) => Ok(Some(c)),
                    Err(1) => Ok(None),
                    Err(e) => Err(e),
                }
            }

            #[inline(always)]
            fn uart_err_check(device: &Device) -> Option<u32> {
                let rc = unsafe {
                    zephyr_sys::syscalls::$context::uart_err_check(device as *const _ as *mut _)
                }
                .neg_err();

                match rc {
                    Ok(_) => None,
                    Err(e) => Some(e),
                }
            }

            #[inline(always)]
            fn uart_config_get(device: &Device) -> Result<UartConfig, u32> {
                let mut config = UartConfig::default();
                unsafe {
                    zephyr_sys::syscalls::$context::uart_config_get(
                        device as *const _ as *mut _,
                        &mut config.0,
                    )
                }
                .neg_err()
                .map(|_| config)
            }

            #[inline(always)]
            fn uart_configure(device: &Device, config: &UartConfig) -> Result<(), u32> {
                unsafe {
                    zephyr_sys::syscalls::$context::uart_configure(
                        device as *const _ as *mut _,
                        &config.0,
                    )
                }
                .neg_err()
                .map(|_| ())
            }
        }
    };
}

pub struct UartConfig(zephyr_sys::raw::uart_config);

impl UartConfig {
    pub fn set_flow_control_rts_cts(&mut self) {
        self.0.flow_ctrl =
            zephyr_sys::raw::uart_config_flow_control_UART_CFG_FLOW_CTRL_RTS_CTS as u8;
    }

    pub fn set_flow_control_dtr_dsr(&mut self) {
        self.0.flow_ctrl =
            zephyr_sys::raw::uart_config_flow_control_UART_CFG_FLOW_CTRL_DTR_DSR as u8;
    }

    pub fn disable_flow_control(&mut self) {
        self.0.flow_ctrl = zephyr_sys::raw::uart_config_flow_control_UART_CFG_FLOW_CTRL_NONE as u8;
    }

    pub fn get_baud_rate(&self) -> u32 {
        self.0.baudrate
    }

    pub fn set_baud_rate(&mut self, baud_rate: u32) {
        self.0.baudrate = baud_rate;
    }

    pub fn get_stop_bits(&self) -> u8 {
        self.0.stop_bits
    }

    pub fn set_stop_bits(&mut self, stop_bits: u8) {
        self.0.stop_bits = stop_bits
    }

    pub fn get_data_bits(&self) -> u8 {
        self.0.data_bits
    }

    pub fn set_data_bits(&mut self, data_bits: u8) {
        self.0.data_bits = data_bits
    }
}

impl Default for UartConfig {
    fn default() -> Self {
        Self(zephyr_sys::raw::uart_config {
            baudrate: 0,
            parity: 0,
            stop_bits: 0,
            data_bits: 0,
            flow_ctrl: 0,
        })
    }
}

trait_impl!(kernel, crate::context::Kernel);
trait_impl!(user, crate::context::User);
trait_impl!(any, crate::context::Any);
