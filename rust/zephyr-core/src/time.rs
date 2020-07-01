use core::time::Duration;

use time_convert::z_tmcvt;
use zephyr_sys::raw::{k_ticks_t, k_timeout_t, Z_HZ_ticks};

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash, From)]
pub struct Ticks(pub k_ticks_t);

impl Ticks {
    pub fn checked_add(self, rhs: Self) -> Option<Self> {
        self.0.checked_add(rhs.0).map(Into::into)
    }

    pub fn checked_sub(self, rhs: Self) -> Option<Self> {
        self.0.checked_sub(rhs.0).map(Into::into)
    }

    pub fn checked_add_duration(self, other: &Duration) -> Option<Self> {
        let other = other.into();
        self.checked_add(other)
    }

    pub fn checked_sub_duration(self, other: &Duration) -> Option<Self> {
        let other = other.into();
        self.checked_sub(other)
    }

    pub fn as_millis(&self) -> u64 {
        ticks_to_ms_near(self.0 as u64)
    }

    /// Subtract two tick instants. Return a Timeout suitable for use as a
    /// timeout parameter for Zephyr system calls.
    pub fn sub_timeout(self, rhs: Self) -> Timeout {
        if rhs.0 > self.0 {
            Ticks(0).into()
        } else {
            Ticks(self.0 - rhs.0).into()
        }
    }
}

impl From<u64> for Ticks {
    #[inline(always)]
    fn from(val: u64) -> Self {
        Ticks(val as k_ticks_t)
    }
}

impl From<&Duration> for Ticks {
    #[inline(always)]
    fn from(dur: &Duration) -> Self {
        Ticks(secs_to_ticks(dur.as_secs()).0 + ns_to_ticks(dur.subsec_nanos().into()).0)
    }
}

impl From<Ticks> for Duration {
    #[inline(always)]
    fn from(ticks: Ticks) -> Self {
        Duration::from_nanos(ticks_to_ns_near(ticks.0 as u64))
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Timeout(pub k_timeout_t);

impl From<Ticks> for Timeout {
    #[inline(always)]
    fn from(ticks: Ticks) -> Self {
        Timeout(k_timeout_t { ticks: ticks.0 })
    }
}

impl From<Timeout> for Ticks {
    #[inline(always)]
    fn from(timeout: Timeout) -> Self {
        Ticks(timeout.0.ticks)
    }
}

impl From<&Duration> for Timeout {
    fn from(dur: &Duration) -> Self {
        Ticks::from(dur).into()
    }
}

impl From<Timeout> for Duration {
    #[inline(always)]
    fn from(timeout: Timeout) -> Self {
        Ticks::from(timeout).into()
    }
}

pub const K_FOREVER: Timeout = Timeout(zephyr_sys::raw::K_FOREVER);
pub const K_NO_WAIT: Timeout = Timeout(zephyr_sys::raw::K_NO_WAIT);

#[allow(unused)]
#[inline(always)]
fn secs_to_ticks(val: u64) -> Ticks {
    z_tmcvt(val, 1, Z_HZ_ticks, true, true, false).into()
}

#[allow(unused)]
#[inline(always)]
fn ms_to_ticks(val: u64) -> Ticks {
    z_tmcvt(val, 1_000, Z_HZ_ticks, true, true, false).into()
}

#[allow(unused)]
#[inline(always)]
fn us_to_ticks(val: u64) -> Ticks {
    z_tmcvt(val, 1_000_000, Z_HZ_ticks, true, true, false).into()
}

#[allow(unused)]
#[inline(always)]
fn ns_to_ticks(val: u64) -> Ticks {
    z_tmcvt(val, 1_000_000_000, Z_HZ_ticks, true, true, false).into()
}

#[allow(unused)]
#[inline(always)]
fn ticks_to_secs_floor(val: u64) -> u64 {
    z_tmcvt(val, Z_HZ_ticks, 1, true, false, false)
}

#[allow(unused)]
#[inline(always)]
fn ticks_to_secs_ceil(val: u64) -> u64 {
    z_tmcvt(val, Z_HZ_ticks, 1, true, true, false)
}

#[allow(unused)]
#[inline(always)]
fn ticks_to_secs_near(val: u64) -> u64 {
    z_tmcvt(val, Z_HZ_ticks, 1, true, false, true)
}

#[allow(unused)]
#[inline(always)]
fn ticks_to_ms_floor(val: u64) -> u64 {
    z_tmcvt(val, Z_HZ_ticks, 1_000, true, false, false)
}

#[allow(unused)]
#[inline(always)]
fn ticks_to_ms_ceil(val: u64) -> u64 {
    z_tmcvt(val, Z_HZ_ticks, 1_000, true, true, false)
}

#[allow(unused)]
#[inline(always)]
fn ticks_to_ms_near(val: u64) -> u64 {
    z_tmcvt(val, Z_HZ_ticks, 1_000, true, false, true)
}

#[allow(unused)]
#[inline(always)]
fn ticks_to_us_floor(val: u64) -> u64 {
    z_tmcvt(val, Z_HZ_ticks, 1_000_000, true, false, false)
}

#[allow(unused)]
#[inline(always)]
fn ticks_to_us_ceil(val: u64) -> u64 {
    z_tmcvt(val, Z_HZ_ticks, 1_000_000, true, true, false)
}

#[allow(unused)]
#[inline(always)]
fn ticks_to_us_near(val: u64) -> u64 {
    z_tmcvt(val, Z_HZ_ticks, 1_000_000, true, false, true)
}

#[allow(unused)]
#[inline(always)]
fn ticks_to_ns_floor(val: u64) -> u64 {
    z_tmcvt(val, Z_HZ_ticks, 1_000_000_000, true, false, false)
}

#[allow(unused)]
#[inline(always)]
fn ticks_to_ns_ceil(val: u64) -> u64 {
    z_tmcvt(val, Z_HZ_ticks, 1_000_000_000, true, true, false)
}

#[allow(unused)]
#[inline(always)]
fn ticks_to_ns_near(val: u64) -> u64 {
    z_tmcvt(val, Z_HZ_ticks, 1_000_000_000, true, false, true)
}

/// 32-bit time in ms. Used for sleep return value.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash, From, Into)]
pub struct DurationMs(pub i32);
