use core::convert::{TryFrom, TryInto};
use core::fmt;
use core::num::TryFromIntError;
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
        let other = other.try_into().ok()?;
        self.checked_add(other)
    }

    pub fn checked_sub_duration(self, other: &Duration) -> Option<Self> {
        let other = other.try_into().ok()?;
        self.checked_sub(other)
    }
}

impl TryFrom<&Duration> for InstantMs {
    type Error = TryFromDurationError;

    #[inline(always)]
    fn try_from(dur: &Duration) -> Result<Self, Self::Error> {
        let secs = dur.as_secs();
        // Allow for a very large duration, but not so large that multiplying by 1000 will
        // overflow. This is cheaper on 32-bit than doing a checked 64-bit multiply.
        let secs = if secs & 0x000f_ffff_ffff_ffffu64 == secs {
            secs as i64 * 1000 + dur.subsec_millis() as i64
        } else {
            return Err(TryFromDurationError);
        };
        Ok(InstantMs(secs))
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

impl DurationMs {
    pub fn new(ms: i32) -> Self {
        DurationMs(ms)
    }
}

impl From<DurationMs> for Duration {
    fn from(dur: DurationMs) -> Self {
        let secs = dur.0 / 1000;
        let ms = (dur.0 % 1000) as u32;
        Duration::new(secs as u64, ms * 1000 * 1000)
    }
}

impl TryFrom<&Duration> for DurationMs {
    type Error = TryFromDurationError;

    fn try_from(dur: &Duration) -> Result<Self, Self::Error> {
        let secs = i32::try_from(dur.as_secs())?;
        let ms = i32::try_from(dur.subsec_millis())?;
        secs.checked_mul(1000)
            .and_then(|secs| secs.checked_add(ms))
            .ok_or(TryFromDurationError)
            .map(DurationMs)
    }
}

/*
#[test]
fn try_from_duration() {
    let ms = DurationMs::try_from(Duration::new(50, 6 * 1000 * 1000)).unwrap();
    assert_eq!(ms, DurationMs(50 * 1000 + 6));
}
*/

#[derive(Clone, Copy, Debug)]
pub struct TryFromDurationError;

impl fmt::Display for TryFromDurationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        "Duration to ms conversion out of range".fmt(f)
    }
}

impl From<TryFromIntError> for TryFromDurationError {
    fn from(_e: TryFromIntError) -> Self {
        TryFromDurationError
    }
}
