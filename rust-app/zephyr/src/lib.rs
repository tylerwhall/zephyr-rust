#![no_std]

macro_rules! zephyr_bindings {
    ($context:ident) => {
        #[inline(always)]
        pub fn k_str_out(s: &str) {
            unsafe { zephyr_sys::syscalls::$context::k_str_out(s.as_ptr() as *mut _, s.len()) };
        }
    }
}


pub mod kernel {
    zephyr_bindings!(kernel);
}

pub mod user {
    zephyr_bindings!(user);
}

pub mod any {
    zephyr_bindings!(any);
}