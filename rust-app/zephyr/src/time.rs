use core::time::Duration;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash, From)]
pub struct InstantMs(i64);

impl InstantMs {
    pub const fn zero() -> Self {
        Self(0)
    }

    pub fn checked_add(self, rhs: Self) -> Option<Self> {
        self.0.checked_add(rhs.0).map(Into::into)
    }

    pub fn checked_sub(self, rhs: Self) -> Option<Self> {
        self.0.checked_sub(rhs.0).map(Into::into)
    }

    pub fn checked_add_duration(self, other: &Duration) -> Option<Self> {
        self.checked_add(other.into())
    }

    pub fn checked_sub_duration(self, other: &Duration) -> Option<Self> {
        self.checked_sub(other.into())
    }
}

impl From<&Duration> for InstantMs {
    fn from(dur: &Duration) -> Self {
        InstantMs(dur.as_secs() as i64 * 1000 + dur.subsec_millis() as i64)
    }
}

impl From<InstantMs> for Duration {
    fn from(dur: InstantMs) -> Self {
        let secs = dur.0 / 1000;
        let ms = (dur.0 % 1000) as u32;
        Duration::new(secs as u64, ms * 1000 * 1000)
    }
}

/// 32-bit time in ms. Used for sleep duration.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash, From, Into)]
pub struct DurationMs(i32);

impl From<DurationMs> for Duration {
    fn from(dur: DurationMs) -> Self {
        let secs = dur.0 / 1000;
        let ms = (dur.0 % 1000) as u32;
        Duration::new(secs as u64, ms * 1000 * 1000)
    }
}
