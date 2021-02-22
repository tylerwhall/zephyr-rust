use core::sync::atomic::AtomicBool;
use std::io;

use zephyr_sys::raw;

use super::NegErrno;
use crate::device::Device;
use crate::macros;
use bitflags::bitflags;

#[derive(Clone, Copy, Debug)]
#[repr(u32)]
pub enum IOConfig {
    /// Disables Pin for both Input and Output (Sets pin to zero; thus cannot be combined with other flags)
    Disconnected = raw::GPIO_DISCONNECTED,
    /// Enables pin as Input
    Input = raw::GPIO_INPUT,
    /// Configures GPIO pin as output and initializes it to a low state.
    OutputLow = raw::GPIO_OUTPUT_LOW,
    /// Configures GPIO pin as output and initializes it to a high state.
    OutputHigh = raw::GPIO_OUTPUT_HIGH,
    /// Configures GPIO pin as output and initializes it to a logic 0.
    OutputInactive = raw::GPIO_OUTPUT_INACTIVE,
    /// Configures GPIO pin as output and initializes it to a logic 1.
    OutputActive = raw::GPIO_OUTPUT_ACTIVE,
}

#[derive(Clone, Copy, Debug)]
#[repr(u32)]
pub enum OutputConfig {
    /// Configures GPIO pin as output and initializes it to a low state.
    OutputLow = IOConfig::OutputLow as u32,
    /// Configures GPIO pin as output and initializes it to a high state.
    OutputHigh = IOConfig::OutputHigh as u32,
    /// Configures GPIO pin as output and initializes it to a logic 0.
    OutputInactive = IOConfig::OutputInactive as u32,
    /// Configures GPIO pin as output and initializes it to a logic 1.
    OutputActive = IOConfig::OutputActive as u32,
}
impl From<OutputConfig> for IOConfig {
    fn from(oc: OutputConfig) -> IOConfig {
        match oc {
            OutputConfig::OutputLow => IOConfig::OutputLow,
            OutputConfig::OutputHigh => IOConfig::OutputHigh,
            OutputConfig::OutputInactive => IOConfig::OutputInactive,
            OutputConfig::OutputActive => IOConfig::OutputActive,
        }
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(u32)]
pub enum DriveStrength {
    LowDfltHighDflt = raw::GPIO_DS_DFLT_LOW | raw::GPIO_DS_DFLT_HIGH,
    LowDfltHighAlt = raw::GPIO_DS_DFLT_LOW | raw::GPIO_DS_ALT_HIGH,
    LowAltHighDflt = raw::GPIO_DS_ALT_LOW | raw::GPIO_DS_DFLT_HIGH,
    LowAltHighAlt = raw::GPIO_DS_ALT_LOW | raw::GPIO_DS_ALT_HIGH,
}

#[derive(Clone, Copy, Debug)]
#[repr(u32)]
pub enum InterruptConfig {
    /// GPIO interrupt is edge sensitive.
    Disable = raw::GPIO_INT_DISABLE,
    /// Configures GPIO interrupt to be triggered on pin rising edge and enables it.
    EdgeRising = raw::GPIO_INT_EDGE_RISING,
    /// Configures GPIO interrupt to be triggered on pin falling edge and enables
    EdgeFalling = raw::GPIO_INT_EDGE_FALLING,
    /// Configures GPIO interrupt to be triggered on pin rising or falling edge and
    EdgeBoth = raw::GPIO_INT_EDGE_BOTH,
    /// Configures GPIO interrupt to be triggered on pin physical level low and
    LevelLow = raw::GPIO_INT_LEVEL_LOW,
    /// Configures GPIO interrupt to be triggered on pin physical level high and
    LevelHigh = raw::GPIO_INT_LEVEL_HIGH,
    /// Configures GPIO interrupt to be triggered on pin state change to logical
    EdgeToInactive = raw::GPIO_INT_EDGE_TO_INACTIVE,
    /// Configures GPIO interrupt to be triggered on pin state change to logical
    EdgeToActive = raw::GPIO_INT_EDGE_TO_ACTIVE,
    /// Configures GPIO interrupt to be triggered on pin logical level 0 and enables
    LevelInactive = raw::GPIO_INT_LEVEL_INACTIVE,
    /// Configures GPIO interrupt to be triggered on pin logical level 1 and enables
    LevelActive = raw::GPIO_INT_LEVEL_ACTIVE,
}

#[derive(Clone, Copy, Debug)]
#[repr(u32)]
pub enum Debounce {
    Off = 0,
    On = raw::GPIO_INT_DEBOUNCE,
}

#[derive(Clone, Copy, Debug)]
pub struct ConfigFlags {
    pub drive_strength: DriveStrength,
    pub debounce: Debounce,
    pub interrupt: InterruptConfig,
}
impl ConfigFlags {
    fn to_bitflags(&self) -> u32 {
        self.drive_strength as u32 | self.debounce as u32 | self.interrupt as u32
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(u8)]
pub enum Pin {
    P0 = 0,
    P1,
    P2,
    P3,
    P4,
    P5,
    P6,
    P7,
    P8,
    P9,
    P10,
    P11,
    P12,
    P13,
    P14,
    P15,
    P16,
    P17,
    P18,
    P19,
    P20,
    P21,
    P22,
    P23,
    P24,
    P25,
    P26,
    P27,
    P28,
    P29,
    P30,
    P31,
}
impl Pin {
    //FIXME better solution for this abomination
    pub fn from_u32(p: u32) -> Option<Self> {
        match p {
            0 => Some(Self::P0),
            1 => Some(Self::P1),
            2 => Some(Self::P2),
            3 => Some(Self::P3),
            4 => Some(Self::P4),
            5 => Some(Self::P5),
            6 => Some(Self::P6),
            7 => Some(Self::P7),
            8 => Some(Self::P8),
            9 => Some(Self::P9),
            10 => Some(Self::P10),
            11 => Some(Self::P11),
            12 => Some(Self::P12),
            13 => Some(Self::P13),
            14 => Some(Self::P14),
            15 => Some(Self::P15),
            16 => Some(Self::P16),
            17 => Some(Self::P17),
            18 => Some(Self::P18),
            19 => Some(Self::P19),
            20 => Some(Self::P20),
            21 => Some(Self::P21),
            22 => Some(Self::P22),
            23 => Some(Self::P23),
            24 => Some(Self::P24),
            25 => Some(Self::P25),
            26 => Some(Self::P26),
            27 => Some(Self::P27),
            28 => Some(Self::P28),
            29 => Some(Self::P29),
            30 => Some(Self::P30),
            31 => Some(Self::P31),
            _ => None,
        }
    }
}

bitflags! {
    pub struct Pins: u32 {
        const P0  = (1_u32 << Pin::P0 as u32);
        const P1  = (1_u32 << Pin::P1 as u32);
        const P2  = (1_u32 << Pin::P2 as u32);
        const P3  = (1_u32 << Pin::P3 as u32);
        const P4  = (1_u32 << Pin::P4 as u32);
        const P5  = (1_u32 << Pin::P5 as u32);
        const P6  = (1_u32 << Pin::P6 as u32);
        const P7  = (1_u32 << Pin::P7 as u32);
        const P8  = (1_u32 << Pin::P8 as u32);
        const P9  = (1_u32 << Pin::P9 as u32);
        const P10 = (1_u32 << Pin::P10 as u32);
        const P11 = (1_u32 << Pin::P11 as u32);
        const P12 = (1_u32 << Pin::P12 as u32);
        const P13 = (1_u32 << Pin::P13 as u32);
        const P14 = (1_u32 << Pin::P14 as u32);
        const P15 = (1_u32 << Pin::P15 as u32);
        const P16 = (1_u32 << Pin::P16 as u32);
        const P17 = (1_u32 << Pin::P17 as u32);
        const P18 = (1_u32 << Pin::P18 as u32);
        const P19 = (1_u32 << Pin::P19 as u32);
        const P20 = (1_u32 << Pin::P20 as u32);
        const P21 = (1_u32 << Pin::P21 as u32);
        const P22 = (1_u32 << Pin::P22 as u32);
        const P23 = (1_u32 << Pin::P23 as u32);
        const P24 = (1_u32 << Pin::P24 as u32);
        const P25 = (1_u32 << Pin::P25 as u32);
        const P26 = (1_u32 << Pin::P26 as u32);
        const P27 = (1_u32 << Pin::P27 as u32);
        const P28 = (1_u32 << Pin::P28 as u32);
        const P29 = (1_u32 << Pin::P29 as u32);
        const P30 = (1_u32 << Pin::P30 as u32);
        const P31 = (1_u32 << Pin::P31 as u32);

        const PINS_0_TO_31  = 0xFFFFFFFF;
        const PINS_0_TO_15  = 0x0000FFFF;
        const PINS_15_TO_31 = 0xFFFF0000;
        const PINS_0_TO_7   = 0x000000FF;
        const PINS_8_TO_15  = 0x0000FF00;
        const PINS_16_TO_23 = 0x00FF0000;
        const PINS_24_TO_31 = 0xFF000000;

        const NONE = 0;
    }
}
impl Pins {
    pub fn iter(&self) -> PinIterator {
        PinIterator::new(*self)
    }
}

pub struct PinIterator(u32, u32);
impl PinIterator {
    pub fn new(pins: Pins) -> PinIterator {
        Self(pins.bits(), 1)
    }
}
impl Iterator for PinIterator {
    type Item = Pin;
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.0 >= 31 {
                return None;
            } else {
                let p = 1 << self.1;
                if self.1 & p > 0 {
                    return Pin::from_u32(p);
                } else {
                    self.1 += 1;
                }
            }
        }
    }
}

bitflags! {
    pub struct PinsLevel: u32 {
        const P0_HIGH  = (1_u32 << Pin::P0 as u32);
        const P1_HIGH  = (1_u32 << Pin::P1 as u32);
        const P2_HIGH  = (1_u32 << Pin::P2 as u32);
        const P3_HIGH  = (1_u32 << Pin::P3 as u32);
        const P4_HIGH  = (1_u32 << Pin::P4 as u32);
        const P5_HIGH  = (1_u32 << Pin::P5 as u32);
        const P6_HIGH  = (1_u32 << Pin::P6 as u32);
        const P7_HIGH  = (1_u32 << Pin::P7 as u32);
        const P8_HIGH  = (1_u32 << Pin::P8 as u32);
        const P9_HIGH  = (1_u32 << Pin::P9 as u32);
        const P10_HIGH = (1_u32 << Pin::P10 as u32);
        const P11_HIGH = (1_u32 << Pin::P11 as u32);
        const P12_HIGH = (1_u32 << Pin::P12 as u32);
        const P13_HIGH = (1_u32 << Pin::P13 as u32);
        const P14_HIGH = (1_u32 << Pin::P14 as u32);
        const P15_HIGH = (1_u32 << Pin::P15 as u32);
        const P16_HIGH = (1_u32 << Pin::P16 as u32);
        const P17_HIGH = (1_u32 << Pin::P17 as u32);
        const P18_HIGH = (1_u32 << Pin::P18 as u32);
        const P19_HIGH = (1_u32 << Pin::P19 as u32);
        const P20_HIGH = (1_u32 << Pin::P20 as u32);
        const P21_HIGH = (1_u32 << Pin::P21 as u32);
        const P22_HIGH = (1_u32 << Pin::P22 as u32);
        const P23_HIGH = (1_u32 << Pin::P23 as u32);
        const P24_HIGH = (1_u32 << Pin::P24 as u32);
        const P25_HIGH = (1_u32 << Pin::P25 as u32);
        const P26_HIGH = (1_u32 << Pin::P26 as u32);
        const P27_HIGH = (1_u32 << Pin::P27 as u32);
        const P28_HIGH = (1_u32 << Pin::P28 as u32);
        const P29_HIGH = (1_u32 << Pin::P29 as u32);
        const P30_HIGH = (1_u32 << Pin::P30 as u32);
        const P31_HIGH = (1_u32 << Pin::P31 as u32);
        const P0_LOW   = 0;
        const P1_LOW   = 0;
        const P2_LOW   = 0;
        const P3_LOW   = 0;
        const P4_LOW   = 0;
        const P5_LOW   = 0;
        const P6_LOW   = 0;
        const P7_LOW   = 0;
        const P8_LOW   = 0;
        const P9_LOW   = 0;
        const P10_LOW  = 0;
        const P11_LOW  = 0;
        const P12_LOW  = 0;
        const P13_LOW  = 0;
        const P14_LOW  = 0;
        const P15_LOW  = 0;
        const P16_LOW  = 0;
        const P17_LOW  = 0;
        const P18_LOW  = 0;
        const P19_LOW  = 0;
        const P20_LOW  = 0;
        const P21_LOW  = 0;
        const P22_LOW  = 0;
        const P23_LOW  = 0;
        const P24_LOW  = 0;
        const P25_LOW  = 0;
        const P26_LOW  = 0;
        const P27_LOW  = 0;
        const P28_LOW  = 0;
        const P29_LOW  = 0;
        const P30_LOW  = 0;
        const P31_LOW  = 0;
    }
}

impl From<Pin> for Pins {
    fn from(pin: Pin) -> Self {
        Pins::from_bits_truncate(1_u32 << pin as u32)
    }
}
impl From<PinsLevel> for Pins {
    fn from(level: PinsLevel) -> Self {
        Pins::from_bits_truncate(level.bits)
    }
}
impl From<Pins> for PinsLevel {
    fn from(pins: Pins) -> Self {
        PinsLevel::from_bits_truncate(pins.bits)
    }
}
impl From<raw::gpio_port_value_t> for Pins {
    fn from(pins: raw::gpio_port_value_t) -> Self {
        Pins::from_bits_truncate(pins)
    }
}

/// Raw syscall API
pub trait GpioSyscalls {
    unsafe fn gpio_config(
        device: *mut Device,
        pin: Pin,
        config: IOConfig,
        flags: Option<ConfigFlags>,
    ) -> io::Result<()>;
    unsafe fn gpio_pin_interrupt_configure(
        device: *mut Device,
        pin: Pin,
        config: InterruptConfig,
    ) -> io::Result<()>;
    unsafe fn gpio_port_get_raw(device: *mut Device) -> io::Result<Pins>;
    unsafe fn gpio_port_set_masked_raw(
        device: *mut Device,
        mask: Pins,
        value: PinsLevel,
    ) -> io::Result<()>;
    unsafe fn gpio_port_set_bits_raw(device: *mut Device, pins: Pins) -> io::Result<()>;
    unsafe fn gpio_port_clear_bits_raw(device: *mut Device, pins: Pins) -> io::Result<()>;
    unsafe fn gpio_port_toggle_bits(device: *mut Device, pins: Pins) -> io::Result<()>;
    unsafe fn gpio_get_pending_int(device: *mut Device) -> io::Result<bool>;
}

macro_rules! trait_impl {
    ($context:ident, $context_struct:path) => {
        impl GpioSyscalls for $context_struct {
            #[inline(always)]
            unsafe fn gpio_config(
                device: *mut Device,
                pin: Pin,
                config: IOConfig,
                flags: Option<ConfigFlags>,
            ) -> io::Result<()> {
                zephyr_sys::syscalls::$context::gpio_config(
                    device,
                    pin as u8,
                    config as u32 | flags.map(|f| f.to_bitflags()).unwrap_or(0),
                )
                .zero_or_neg_errno()
            }
            #[inline(always)]
            unsafe fn gpio_pin_interrupt_configure(
                device: *mut Device,
                pin: Pin,
                config: InterruptConfig,
            ) -> io::Result<()> {
                zephyr_sys::syscalls::$context::gpio_pin_interrupt_configure(
                    device,
                    pin as u8,
                    config as u32,
                )
                .zero_or_neg_errno()
            }
            #[inline(always)]
            unsafe fn gpio_port_get_raw(device: *mut Device) -> io::Result<Pins> {
                let mut value: raw::gpio_port_value_t = 0;
                zephyr_sys::syscalls::$context::gpio_port_get_raw(device, &mut value)
                    .neg_errno()
                    .map(|_| Pins::from(value))
            }
            #[inline(always)]
            unsafe fn gpio_port_set_masked_raw(
                device: *mut Device,
                mask: Pins,
                value: PinsLevel,
            ) -> io::Result<()> {
                zephyr_sys::syscalls::$context::gpio_port_set_masked_raw(
                    device,
                    mask.bits(),
                    value.bits(),
                )
                .zero_or_neg_errno()
            }
            #[inline(always)]
            unsafe fn gpio_port_set_bits_raw(device: *mut Device, pins: Pins) -> io::Result<()> {
                zephyr_sys::syscalls::$context::gpio_port_set_bits_raw(device, pins.bits())
                    .zero_or_neg_errno()
            }
            #[inline(always)]
            unsafe fn gpio_port_clear_bits_raw(device: *mut Device, pins: Pins) -> io::Result<()> {
                zephyr_sys::syscalls::$context::gpio_port_clear_bits_raw(device, pins.bits())
                    .zero_or_neg_errno()
            }
            #[inline(always)]
            unsafe fn gpio_port_toggle_bits(device: *mut Device, pins: Pins) -> io::Result<()> {
                zephyr_sys::syscalls::$context::gpio_port_toggle_bits(device, pins.bits())
                    .zero_or_neg_errno()
            }
            #[inline(always)]
            unsafe fn gpio_get_pending_int(device: *mut Device) -> io::Result<bool> {
                const ERR: i32 = -(raw::ENOTSUP as i32);
                match zephyr_sys::syscalls::$context::gpio_get_pending_int(device) {
                    0 => Ok(false),
                    ERR => Err(io::Error::from_raw_os_error(ERR)),
                    _ => Ok(true),
                }
            }
        }
    };
}

trait_impl!(kernel, crate::context::Kernel);
trait_impl!(user, crate::context::User);
trait_impl!(any, crate::context::Any);

type GpioCallbackFunc<T> = &'static (dyn Fn(GpioPort, Pins, &T) + Sync);
pub trait GpioCallback<T> {
    fn handler(port: GpioPort, pins: Pins, data: &T);
}

pub trait GpioCallbackHandler {
    unsafe fn get_cb(&mut self) -> *mut raw::gpio_callback;
    fn is_registered(&self) -> bool;
    fn set_registered(&self, r: bool);
}

/// Creates the neccesary struct and implementations to create a callback handler;
///
///  Use:
///  ```
///  use std::sync::Once;
///  use std::sync::atomic::{AtomicU8, Ordering};
///
///  struct MyGpioData {
///     t: AtomicU8,
///  }
///  impl GpioCallback<MyGpioData> {
///      fn handler(port: GpioPort, pins: Pins, data: &MyGpioData) {
///         let t = data.t.fetch_add(Ordering::SeqCst, 1);
///         println!("I've been here {} times.", t);
///      }
///  }
///
///  static INIT: Once = Once::new();
///  static mut GPIO_HANDLER: CallBackTrampoline<MyGpioData>;
///
///  fn init(port: &GpioPort) {
///      INIT.call_once(|| {
///          unsafe { GPIO_HANDLER = CallBackTrampoline::new(Pins::P0, MyGpioData { t: AtomicU8::new(0), }); }
///      });
///
///      port.register_callback(&GPIO_HANDLER);
///  }
///  ```
#[repr(C)]
pub struct CallBackTrampoline<T: 'static + GpioCallback<T>> {
    cb_handle: raw::gpio_callback,
    registered: AtomicBool,
    priv_data: T,
    priv_func: GpioCallbackFunc<T>,
}

impl<T: 'static + GpioCallback<T>> CallBackTrampoline<T> {
    pub fn new(pin_mask: Pins, data: T) -> Self {
        Self {
            registered: AtomicBool::new(false),
            cb_handle: raw::gpio_callback {
                handler: Some(Self::gpio_callback_trampoline),
                pin_mask: pin_mask.bits(),
                node: raw::sys_snode_t {
                    next: core::ptr::null_mut::<raw::_snode>(),
                },
            },
            priv_data: data,
            priv_func: &<T as GpioCallback<T>>::handler,
        }
    }
    pub fn get_data<'a>(&'a self) -> &'a T {
        &self.priv_data
    }
    unsafe fn get_cb(&self) -> *mut zephyr_sys::raw::gpio_callback {
        &self.cb_handle as *const _ as *mut _
    }
    fn is_registered(&self) -> bool {
        self.registered.load(core::sync::atomic::Ordering::SeqCst)
    }
    fn set_registered(&self, r: bool) {
        self.registered
            .store(r, core::sync::atomic::Ordering::SeqCst);
    }
    unsafe extern "C" fn gpio_callback_trampoline(
        port: *const raw::device,
        cb_handle: *mut raw::gpio_callback,
        pins: raw::gpio_port_pins_t,
    ) {
        // Unwinding over the FFI boundary is UB, so we need to catch panics
        ::std::panic::catch_unwind(move || {
            let cbfieldptr = cb_handle as *const _;
            let pcontainerptr: *const CallBackTrampoline<T> =
                container_of!(cbfieldptr, CallBackTrampoline<T>, cb_handle);

            if let Some(pcontainer) = pcontainerptr.as_ref() {
                if let Some(port) = port.as_ref() {
                    // Call user callback
                    ((*pcontainerptr).priv_func)(
                        GpioPort(port, Pins::PINS_0_TO_31), //FIXME: do we have a way to transport the mask?
                        Pins::from_bits_truncate(pins),
                        &pcontainer.priv_data,
                    );
                }
            }
        })
        .ok();
    }
}

//TODO: typefix GpioPort and PinGroup to prevent merges of unrelated ports
pub trait PinGroup {
    fn get_mask(&self) -> Pins;
}

pub struct GpioPort(&'static Device, Pins);
impl GpioPort {
    /// # Safety
    ///
    /// Caller must ensure the device is an gpio device
    pub unsafe fn new(dev: &'static Device, mask: Pins) -> Self {
        GpioPort(dev, mask)
    }

    pub fn split_into_dynamic<C: GpioSyscalls>(
        &mut self,
        mask: Pins,
    ) -> io::Result<DynamicPinGroup> {
        const ERR: i32 = -(raw::EINVAL as i32);

        if self.1 & mask == mask {
            for p in mask.iter() {
                unsafe {
                    C::gpio_config(
                        self.0 as *const _ as *mut _,
                        p,
                        IOConfig::Disconnected,
                        None,
                    )?;
                }
            }
            self.1 &= !mask;
            Ok(DynamicPinGroup(self.0, mask))
        } else {
            Err(io::Error::from_raw_os_error(ERR))
        }
    }

    /// # Safety
    ///
    /// The caller must ensure both arguments are using the same port.
    pub unsafe fn merge<C: GpioSyscalls, T: PinGroup>(&mut self, pins: T) -> io::Result<()> {
        for p in pins.get_mask().iter() {
            unsafe {
                C::gpio_config(
                    self.0 as *const _ as *mut _,
                    p,
                    IOConfig::Disconnected,
                    None,
                )?;
            }
        }
        self.1 |= pins.get_mask();
        Ok(())
    }
}
impl PinGroup for GpioPort {
    fn get_mask(&self) -> Pins {
        self.1
    }
}

pub struct DynamicPinGroup(&'static Device, Pins);
impl PinGroup for DynamicPinGroup {
    fn get_mask(&self) -> Pins {
        self.1
    }
}
impl DynamicPinGroup {
    /// # Safety
    ///
    /// The caller must ensure both arguments are using the same port.
    pub unsafe fn merge<C: GpioSyscalls, T: PinGroup>(&mut self, pins: T) -> io::Result<()> {
        for p in pins.get_mask().iter() {
            unsafe {
                C::gpio_config(
                    self.0 as *const _ as *mut _,
                    p,
                    IOConfig::Disconnected,
                    None,
                )?;
            }
        }
        self.1 |= pins.get_mask();
        Ok(())
    }

    pub fn split_into_input<C: GpioSyscalls>(&mut self, mask: Pins) -> io::Result<InputPinGroup> {
        const ERR: i32 = -(raw::EINVAL as i32);

        if self.1 & mask == mask {
            for p in mask.iter() {
                unsafe {
                    C::gpio_config(self.0 as *const _ as *mut _, p, IOConfig::Input, None)?;
                }
            }
            self.1 &= !mask;
            Ok(InputPinGroup(self.0, mask))
        } else {
            Err(io::Error::from_raw_os_error(ERR))
        }
    }

    pub fn split_into_input_with_interrupt<C: GpioSyscalls>(
        &mut self,
        mask: Pins,
        interrupt_config: InterruptConfig,
    ) -> io::Result<InterruptPinGroup> {
        const ERR: i32 = -(raw::EINVAL as i32);

        if self.1 & mask == mask {
            for p in mask.iter() {
                unsafe {
                    C::gpio_config(self.0 as *const _ as *mut _, p, IOConfig::Input, None)?;
                    C::gpio_pin_interrupt_configure(
                        self.0 as *const _ as *mut _,
                        p,
                        interrupt_config,
                    )?;
                }
            }
            self.1 &= !mask;
            Ok(InterruptPinGroup(InputPinGroup(self.0, mask)))
        } else {
            Err(io::Error::from_raw_os_error(ERR))
        }
    }

    pub fn split_into_output<C: GpioSyscalls>(
        &mut self,
        mask: Pins,
        config: OutputConfig,
        flags: Option<ConfigFlags>,
    ) -> io::Result<OutputPinGroup> {
        const ERR: i32 = -(raw::EINVAL as i32);

        if self.1 & mask == mask {
            for p in mask.iter() {
                unsafe {
                    C::gpio_config(self.0 as *const _ as *mut _, p, config.into(), flags)?;
                }
            }
            self.1 &= !mask;
            Ok(OutputPinGroup(self.0, mask))
        } else {
            Err(io::Error::from_raw_os_error(ERR))
        }
    }
}

pub struct InputPinGroup(&'static Device, Pins);
impl PinGroup for InputPinGroup {
    fn get_mask(&self) -> Pins {
        self.1
    }
}
impl InputPinGroup {
    /// # Safety
    ///
    /// The caller must ensure both arguments are using the same port.
    #[inline(always)]
    pub unsafe fn merge<T: PinGroup>(&mut self, pins: InputPinGroup) {
        self.1 |= pins.get_mask();
    }

    #[inline(always)]
    pub fn split(mut self, pins: Pins) -> Option<(InputPinGroup, InputPinGroup)> {
        if self.1 & pins == pins {
            self.1 &= !pins;
            Some((InputPinGroup(self.0, pins), self))
        } else {
            None
        }
    }

    pub fn into_dynamic<C: GpioSyscalls>(self) -> io::Result<DynamicPinGroup> {
        let mut d = DynamicPinGroup(self.0, Pins::NONE);
        d.merge::<C, _>(self)?;
        Ok(d)
    }

    pub fn into_interrupt<C: GpioSyscalls>(
        self,
        config: InterruptConfig,
    ) -> io::Result<InterruptPinGroup> {
        for p in self.1.iter() {
            unsafe {
                C::gpio_pin_interrupt_configure(self.0 as *const _ as *mut _, p, config)?;
            }
        }
        Ok(InterruptPinGroup(self))
    }

    #[inline(always)]
    fn port_get_raw<C: GpioSyscalls>(&self) -> io::Result<Pins> {
        unsafe { C::gpio_port_get_raw(self.0 as *const _ as *mut _) }
    }

    #[inline(always)]
    fn port_get<C: GpioSyscalls>(&self) -> io::Result<Pins> {
        const ERR: i32 = -(raw::ENOTSUP as i32);

        let value = self.port_get_raw::<C>()?;

        let invert = if self.0.data.is_null() {
            Err(io::Error::from_raw_os_error(ERR))
        } else {
            let driver_data: *const raw::gpio_driver_data =
                self.0.data as *const raw::gpio_driver_data;
            Ok(unsafe { (*driver_data).invert })
        }?;
        Ok(value ^ Pins::from_bits_truncate(invert))
    }

    #[inline(always)]
    pub fn get<C: GpioSyscalls>(&self) -> io::Result<Pins> {
        self.port_get::<C>().map(|p| p & self.1)
    }

    #[inline(always)]
    pub fn get_raw<C: GpioSyscalls>(&self) -> io::Result<Pins> {
        self.port_get_raw::<C>().map(|p| p & self.1)
    }

    #[inline(always)]
    pub fn get_pending_int<C: GpioSyscalls>(&self) -> io::Result<bool> {
        unsafe { C::gpio_get_pending_int(self.0 as *const _ as *mut _) }
    }
}

pub struct InterruptPinGroup(InputPinGroup);
impl PinGroup for InterruptPinGroup {
    fn get_mask(&self) -> Pins {
        self.0.get_mask()
    }
}
impl InterruptPinGroup {
    /// # Safety
    ///
    /// The caller must ensure both arguments are using the same port.
    pub unsafe fn merge<T: PinGroup>(&mut self, pins: InterruptPinGroup) {
        self.0 .1 |= pins.get_mask();
    }

    #[inline(always)]
    pub fn into_input(self) -> InputPinGroup {
        self.0
    }

    #[inline(always)]
    pub fn as_input<'a>(&'a self) -> &'a InputPinGroup {
        &self.0
    }

    pub fn as_input_mut<'a>(&'a mut self) -> &'a mut InputPinGroup {
        &mut self.0
    }

    pub fn register_callback<C: GpioSyscalls, T: GpioCallback<T>>(
        &self,
        cb: &'static CallBackTrampoline<T>,
    ) -> io::Result<()> {
        const EINVAL: i32 = -(raw::ENOTSUP as i32);
        const ENOTSUP: i32 = -(raw::ENOTSUP as i32);

        if self.0 .0.api.is_null() {
            Err(io::Error::from_raw_os_error(ENOTSUP))
        } else {
            unsafe {
                let api = self.0 .0.api as *const raw::gpio_driver_api;
                if let Some(ref manage_callback) = (*api).manage_callback {
                    if !cb.is_registered() {
                        let ret = manage_callback(self.0 .0, cb.get_cb(), true).zero_or_neg_errno();
                        if ret.is_ok() {
                            cb.set_registered(true);
                        }
                        ret
                    } else {
                        Err(io::Error::from_raw_os_error(EINVAL))
                    }
                } else {
                    Err(io::Error::from_raw_os_error(ENOTSUP))
                }
            }
        }
    }

    pub fn remove_callback<C: GpioSyscalls, T: GpioCallback<T>>(
        &self,
        cb: &'static CallBackTrampoline<T>,
    ) -> io::Result<()> {
        const EINVAL: i32 = -(raw::ENOTSUP as i32);
        const ENOTSUP: i32 = -(raw::ENOTSUP as i32);

        if self.0 .0.api.is_null() {
            Err(io::Error::from_raw_os_error(ENOTSUP))
        } else {
            unsafe {
                let api = self.0 .0.api as *const raw::gpio_driver_api;
                if let Some(ref manage_callback) = (*api).manage_callback {
                    if cb.is_registered() {
                        let ret =
                            manage_callback(self.0 .0, cb.get_cb(), false).zero_or_neg_errno();
                        if ret.is_ok() {
                            cb.set_registered(false);
                        }
                        ret
                    } else {
                        Err(io::Error::from_raw_os_error(EINVAL))
                    }
                } else {
                    Err(io::Error::from_raw_os_error(ENOTSUP))
                }
            }
        }
    }
}

pub struct OutputPinGroup(&'static Device, Pins);
impl PinGroup for OutputPinGroup {
    fn get_mask(&self) -> Pins {
        self.1
    }
}
impl OutputPinGroup {
    /// # Safety
    ///
    /// The caller must ensure both arguments are using the same port.
    #[inline(always)]
    pub unsafe fn merge<C: GpioSyscalls, T: PinGroup>(&mut self, pins: OutputPinGroup) {
        self.1 |= pins.get_mask();
    }

    #[inline(always)]
    pub fn split(mut self, pins: Pins) -> Option<(OutputPinGroup, OutputPinGroup)> {
        if self.1 & pins == pins {
            self.1 &= !pins;
            Some((OutputPinGroup(self.0, pins), self))
        } else {
            None
        }
    }

    #[inline(always)]
    pub fn into_dynamic<C: GpioSyscalls>(self) -> io::Result<DynamicPinGroup> {
        let mut d = DynamicPinGroup(self.0, Pins::NONE);
        d.merge::<C, _>(self)?;
        Ok(d)
    }

    #[inline(always)]
    pub fn toggle<C: GpioSyscalls>(&mut self) -> io::Result<()> {
        unsafe { C::gpio_port_toggle_bits(self.0 as *const _ as *mut _, self.1) }
    }

    #[inline(always)]
    fn port_set_masked_raw<C: GpioSyscalls>(
        &mut self,
        mask: Pins,
        value: PinsLevel,
    ) -> io::Result<()> {
        unsafe { C::gpio_port_set_masked_raw(self.0 as *const _ as *mut _, mask, value) }
    }
    #[inline(always)]
    pub fn set_raw<C: GpioSyscalls>(&mut self) -> io::Result<()> {
        self.port_set_masked_raw::<C>(self.1, PinsLevel::from(self.1))
    }

    #[inline(always)]
    pub fn clear_raw<C: GpioSyscalls>(&mut self) -> io::Result<()> {
        self.port_set_masked_raw::<C>(self.1, PinsLevel::from_bits_truncate(0))
    }

    #[inline(always)]
    fn port_set_masked<C: GpioSyscalls>(&mut self, value: PinsLevel) -> io::Result<()> {
        let invert = if self.0.data.is_null() {
            const ERR: i32 = -(raw::ENOTSUP as i32);
            Err(io::Error::from_raw_os_error(ERR))
        } else {
            let driver_data: *const raw::gpio_driver_data =
                self.0.data as *const raw::gpio_driver_data;
            Ok(unsafe { (*driver_data).invert })
        }?;

        self.port_set_masked_raw::<C>(self.1, value ^ PinsLevel::from_bits_truncate(invert))
    }
    #[inline(always)]
    pub fn set<C: GpioSyscalls>(&mut self) -> io::Result<()> {
        self.port_set_masked::<C>(PinsLevel::from(self.1))
    }

    #[inline(always)]
    pub fn clear<C: GpioSyscalls>(&mut self) -> io::Result<()> {
        self.port_set_masked::<C>(PinsLevel::from_bits_truncate(0))
    }
}
