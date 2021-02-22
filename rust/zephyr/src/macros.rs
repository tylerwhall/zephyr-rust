#![macro_use]

macro_rules! offset_of {
    ($container:ty, $field:ident) => {
        unsafe { &(*(0usize as *const $container)).$field as *const _ as usize }
    };
}

macro_rules! container_of {
    ($ptr:ident, $container:ty, $field:ident) => {{
        (($ptr as usize) - offset_of!($container, $field)) as *const $container
    }};
}
