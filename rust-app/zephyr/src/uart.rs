use crate::device::Device;

pub trait UartSyscalls {
    fn uart_poll_out(device: &Device, out_char: char);

    fn uart_poll_in(device: &Device, in_char: &mut char) -> i32;

    fn uart_err_check(device: &Device) -> i32;
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
            fn uart_poll_in(device: &Device, in_char: &mut char) -> i32 {
                let mut munge: u8 = 0;
                let rc: i32 = unsafe {
                    zephyr_sys::syscalls::$context::uart_poll_in(
                        device as *const _ as *mut _,
                        &mut munge,
                    )
                };
                *in_char = munge as char;
                rc
            }

            #[inline(always)]
            fn uart_err_check(device: &Device) -> i32 {
                (unsafe {
                    zephyr_sys::syscalls::$context::uart_err_check(device as *const _ as *mut _)
                })
            }
        }
    };
}

trait_impl!(kernel, crate::context::Kernel);
trait_impl!(user, crate::context::User);
trait_impl!(any, crate::context::Any);
