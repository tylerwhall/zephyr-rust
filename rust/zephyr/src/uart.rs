use super::NegErr;
use crate::device::Device;

pub trait UartSyscalls {
    fn uart_poll_out(device: &Device, out_char: char);

    fn uart_poll_in(device: &Device) -> Result<Option<char>, u32>;

    fn uart_err_check(device: &Device) -> Option<u32>;

    fn uart_config_get(device: &Device) -> Result<UartConfig, u32>;

    fn uart_configure(device: &Device, config: &UartConfig) -> Result<(), u32>;
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
