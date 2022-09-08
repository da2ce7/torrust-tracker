use std::{
    convert::TryInto,
    ops::Div,
    time::{Duration, SystemTime},
};

pub trait Clock: Sized + Div + From<Duration> + From<u64> + Into<u64> + From<u32> + TryInto<u32> {
    fn now() -> Self;
    fn after(period: &Duration) -> Self;

    fn after_sec(period: u64) -> Self {
        Self::after(&Duration::new(period, 0))
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct UnixClock<const T: usize>(pub Duration);

pub enum ClockType {
    LocalClock,
    FixedClock,
}

#[cfg(not(test))]
pub type DefaultClock = UnixClock<{ ClockType::LocalClock as usize }>;

#[cfg(test)]
pub type DefaultClock = UnixClock<{ ClockType::FixedClock as usize }>;

pub type Periods = u128;

impl Clock for UnixClock<{ ClockType::LocalClock as usize }> {
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

impl Clock for UnixClock<{ ClockType::FixedClock as usize }> {
    fn now() -> Self {
        Self(Duration::ZERO)
    }

    fn after(period: &Duration) -> Self {
        Self(period.to_owned())
    }
}

impl<const T: usize> Div for UnixClock<{ T }> {
    type Output = Periods;
    fn div(self, rhs: Self) -> Self::Output {
        self.0.as_nanos() / rhs.0.as_nanos()
    }
}

impl<const T: usize> From<Duration> for UnixClock<{ T }> {
    fn from(duration: Duration) -> Self {
        Self(duration)
    }
}

impl<const T: usize> From<u64> for UnixClock<{ T }> {
    fn from(int: u64) -> Self {
        Self(Duration::new(int, 0))
    }
}

impl<const T: usize> Into<u64> for UnixClock<{ T }> {
    fn into(self) -> u64 {
        self.0.as_secs()
    }
}

impl<const T: usize> From<u32> for UnixClock<{ T }> {
    fn from(int: u32) -> Self {
        Self(Duration::new(int.into(), 0))
    }
}

impl<const T: usize> TryInto<u32> for UnixClock<{ T }> {
    type Error = <u32 as TryFrom<u64>>::Error;

    fn try_into(self) -> Result<u32, Self::Error> {
        self.0.as_secs().try_into()
    }
}
