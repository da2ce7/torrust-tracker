use std::{
    convert::TryInto,
    ops::Div,
    time::{Duration, SystemTime},
};

pub trait Time: Sized + Div + From<Duration> + From<u64> + Into<u64> + From<u32> + TryInto<u32> {
    fn now() -> Self;
    fn after(period: &Duration) -> Self;

    fn after_sec(period: u64) -> Self {
        Self::after(&Duration::new(period, 0))
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct UnixTime<const T: usize>(pub Duration);

pub enum Clock {
    SystemTime,
    FixedTime,
}

#[cfg(not(test))]
pub type DefaultTime = UnixTime<{ Clock::SystemTime as usize }>;

#[cfg(test)]
pub type DefaultTime = UnixTime<{ Clock::FixedTime as usize }>;

pub type Periods = u128;

impl Time for UnixTime<{ Clock::SystemTime as usize }> {
    fn now() -> Self {
        Self(SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap())
    }

    fn after(period: &Duration) -> Self {
        Self(
            SystemTime::now()
                .checked_add(period.to_owned())
                .unwrap()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap(),
        )
    }
}

impl Time for UnixTime<{ Clock::FixedTime as usize }> {
    fn now() -> Self {
        Self(Duration::ZERO)
    }

    fn after(period: &Duration) -> Self {
        Self(period.to_owned())
    }
}

impl<const T: usize> Div for UnixTime<{ T }> {
    type Output = Periods;
    fn div(self, rhs: Self) -> Self::Output {
        self.0.as_nanos() / rhs.0.as_nanos()
    }
}

impl<const T: usize> From<Duration> for UnixTime<{ T }> {
    fn from(duration: Duration) -> Self {
        Self(duration)
    }
}

impl<const T: usize> From<u64> for UnixTime<{ T }> {
    fn from(int: u64) -> Self {
        Self(Duration::new(int, 0))
    }
}

impl<const T: usize> From<UnixTime<{ T }>> for u64 {
    fn from(unix_clock: UnixTime<{ T }>) -> Self {
        unix_clock.0.as_secs()
    }
}

impl<const T: usize> From<u32> for UnixTime<{ T }> {
    fn from(int: u32) -> Self {
        Self(Duration::new(int.into(), 0))
    }
}

impl<const T: usize> TryInto<u32> for UnixTime<{ T }> {
    type Error = <u32 as TryFrom<u64>>::Error;

    fn try_into(self) -> Result<u32, Self::Error> {
        self.0.as_secs().try_into()
    }
}
