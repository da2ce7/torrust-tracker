use std::num::{IntErrorKind, TryFromIntError};
use std::time::Duration;

pub type PeriodLength = Duration;
pub type PeriodCount = u64;
pub type PeriodTotalTime = Duration;

pub trait Periods: Sized + Default {
    fn new(length: &PeriodLength, count: &PeriodCount) -> Self;

    fn add_count(&self, add: PeriodCount) -> Result<Self, IntErrorKind>;
    fn sub_count(&self, sub: PeriodCount) -> Result<Self, IntErrorKind>;
}

pub trait PeriodsTotals: Periods {
    fn total_time(&self) -> Result<Option<PeriodTotalTime>, TryFromIntError>;
    fn total_time_end(&self) -> Result<Option<PeriodTotalTime>, TryFromIntError>;
}

#[derive(Debug, Default, Hash, PartialEq)]
pub struct Period {
    pub length: PeriodLength,
    pub count: PeriodCount,
}

pub type SinceUnixEpochPeriods = Period;

impl Period {
    pub const fn from_sec(length: u64, count: u64) -> Self {
        Self {
            length: Duration::from_secs(length),
            count: count,
        }
    }
}

impl Periods for Period {
    fn new(length: &PeriodLength, count: &PeriodCount) -> Self {
        Self {
            length: *length,
            count: *count,
        }
    }

    fn add_count(&self, add: PeriodCount) -> Result<Period, IntErrorKind> {
        match self.count.checked_add(add) {
            None => Err(IntErrorKind::PosOverflow),
            Some(count) => Ok(Self {
                length: self.length,
                count: count,
            }),
        }
    }

    fn sub_count(&self, sub: PeriodCount) -> Result<Period, IntErrorKind> {
        match self.count.checked_sub(sub) {
            None => Err(IntErrorKind::NegOverflow),
            Some(count) => Ok(Self {
                length: self.length,
                count: count,
            }),
        }
    }
}

impl PeriodsTotals for Period {
    fn total_time(&self) -> Result<Option<PeriodTotalTime>, TryFromIntError> {
        match u32::try_from(self.count) {
            Err(error) => Err(error),
            Ok(count) => Ok(self.length.checked_mul(count)),
        }
    }

    fn total_time_end(&self) -> Result<Option<PeriodTotalTime>, TryFromIntError> {
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
}

#[cfg(test)]
mod test {

    use std::time::Duration;

    use crate::protocol::clock::period::{Period, PeriodsTotals};

    #[test]
    fn it_should_get_the_total_time_of_a_period() {
        assert_eq!(Period::default().total_time().unwrap().unwrap(), Duration::ZERO);

        assert_eq!(
            Period::from_sec(12, 12).total_time().unwrap().unwrap(),
            Duration::from_secs(144)
        );
        assert_eq!(
            Period::from_sec(12, 12).total_time_end().unwrap().unwrap(),
            Duration::from_secs(156)
        );
    }
}
