use std::num::{IntErrorKind, TryFromIntError};
use std::time::Duration;

pub type PeriodLength = Duration;
pub type PeriodCount = u64;
pub type PeriodTotalTime = Duration;
pub type PeriodSinceUnixEpoch = Period;

/// A `Period` type with a `length` as `Duration` and a `count` as `u64`,
/// to represent a time duration and how many times that duration will occur or has occurred.
#[derive(Debug, Default, Hash, PartialEq)]
pub struct Period {
    pub length: PeriodLength,
    pub count: PeriodCount,
}

impl Period {
    /// Returns a new `Period`.
    pub fn new(length: &PeriodLength, count: PeriodCount) -> Self {
        Self {
            length: *length,
            count,
        }
    }

    /// Returns a new `Period` from seconds.
    pub const fn from_secs(seconds: u64, count: PeriodCount) -> Self {
        Self {
            length: Duration::from_secs(seconds),
            count,
        }
    }

    /// Return `Self.length` * `Self.count` as `Duration`.
    pub fn total_time(&self) -> Result<Option<PeriodTotalTime>, TryFromIntError> {
        match u32::try_from(self.count) {
            Err(error) => Err(error),
            Ok(count) => Ok(self.length.checked_mul(count)),
        }
    }

    /// Return `Self.length` * (`Self.count` + 1) as `Duration`.
    pub fn total_time_end(&self) -> Result<Option<PeriodTotalTime>, TryFromIntError> {
        match u32::try_from(self.count) {
            Err(e) => Err(e),
            Ok(count) => match count.checked_add(1) {
                None => Ok(None),
                Some(count) => match self.length.checked_mul(count) {
                    None => Ok(None),
                    Some(time) => Ok(Some(time)),
                },
            },
        }
    }

    /// Increase `Self.count` by `u64`.
    pub fn add_count(&self, add: PeriodCount) -> Result<Period, IntErrorKind> {
        match self.count.checked_add(add) {
            None => Err(IntErrorKind::PosOverflow),
            Some(count) => Ok(Self {
                length: self.length,
                count,
            }),
        }
    }

    /// Decrease `Self.count` by `u64`.
    pub fn sub_count(&self, sub: PeriodCount) -> Result<Period, IntErrorKind> {
        match self.count.checked_sub(sub) {
            None => Err(IntErrorKind::NegOverflow),
            Some(count) => Ok(Self {
                length: self.length,
                count,
            }),
        }
    }
}

#[cfg(test)]
mod test {

    use std::time::Duration;

    use crate::protocol::clock::period::Period;

    #[test]
    fn it_should_get_the_total_time_of_a_period() {
        assert_eq!(Period::default().total_time().unwrap().unwrap(), Duration::ZERO);

        assert_eq!(
            Period::from_secs(12, 12).total_time().unwrap().unwrap(),
            Duration::from_secs(144)
        );
        assert_eq!(
            Period::from_secs(12, 12).total_time_end().unwrap().unwrap(),
            Duration::from_secs(156)
        );
    }
}
