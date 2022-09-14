use std::num::TryFromIntError;
use std::time::Duration;

use super::clock::{ClockType, StoppedClock, TimeNow, WorkingClock};
use super::period::{self, Periods};

pub trait ClockPeriods<T>: Sized
where
    T: TimeNow,
{
    fn now(length: &period::PeriodLength) -> Option<Result<period::SinceUnixEpochPeriods, TryFromIntError>> {
        T::now().as_nanos().checked_div(length.as_nanos())
            .map(|count| match period::PeriodCount::try_from(count) {
                Err(error) => Err(error),
                Ok(count) => Ok(period::SinceUnixEpochPeriods::new(length, &count)),
            })
    }

    fn now_add(
        length: &period::PeriodLength,
        add_time: &Duration,
    ) -> Option<Result<period::SinceUnixEpochPeriods, TryFromIntError>> {
        match T::add(add_time) {
            None => None,
            Some(time) => time.as_nanos().checked_div(length.as_nanos())
                .map(|count| match period::PeriodCount::try_from(count) {
                    Err(error) => Err(error),
                    Ok(count) => Ok(period::SinceUnixEpochPeriods::new(length, &count)),
                }),
        }
    }
    fn now_sub(
        length: &period::PeriodLength,
        sub_time: &Duration,
    ) -> Option<Result<period::SinceUnixEpochPeriods, TryFromIntError>> {
        match T::sub(sub_time) {
            None => None,
            Some(time) => time.as_nanos().checked_div(length.as_nanos())
                .map(|count| match period::PeriodCount::try_from(count) {
                    Err(error) => Err(error),
                    Ok(count) => Ok(period::SinceUnixEpochPeriods::new(length, &count)),
                }),
        }
    }
}

#[derive(Debug)]
pub struct PeriodClock<const T: usize> {}

pub type ClockPeriodWorking = PeriodClock<{ ClockType::WorkingClock as usize }>;

pub type StoppedPeriodClock = PeriodClock<{ ClockType::StoppedClock as usize }>;

impl ClockPeriods<WorkingClock> for ClockPeriodWorking {}

impl ClockPeriods<StoppedClock> for StoppedPeriodClock {}

#[cfg(not(test))]
pub type DefaultPeriodClock = ClockPeriodWorking;

#[cfg(test)]
pub type DefaultPeriodClock = StoppedPeriodClock;

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::protocol::clock::clock::{DefaultClock, SinceUnixEpoch, StoppedTime};
    use crate::protocol::clock::clockperiod::{ClockPeriods, DefaultPeriodClock};
    use crate::protocol::clock::period::Period;

    #[test]
    fn it_should_get_the_current_period() {
        assert_eq!(
            DefaultPeriodClock::now(&Duration::from_secs(2)).unwrap().unwrap(),
            Period::from_sec(2, 0)
        );

        DefaultClock::local_set(&SinceUnixEpoch::from_secs(12387687123));

        assert_eq!(
            DefaultPeriodClock::now(&Duration::from_secs(2)).unwrap().unwrap(),
            Period::from_sec(2, 6193843561)
        );
    }
}
