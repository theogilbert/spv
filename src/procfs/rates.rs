use std::collections::{HashMap, VecDeque};
use std::collections::hash_map::Entry;
use std::ops::Add;
use std::time::Duration;
#[cfg(not(test))]
use std::time::Instant;

#[cfg(test)]
use sn_fake_clock::FakeClock as Instant;

use crate::core::process_view::PID;
use crate::procfs::ProcfsError;

#[derive(Clone)]
struct DatedValue
{
    date: Instant,
    value: usize,
}

pub enum ProcessRatesMode {
    ACCUMULATIVE,
    ///
    INCREMENT,
}

/// Keeps tracks of dated accumulative values of processes to calculate their rate
pub struct ProcessesRates {
    acc_values: HashMap<PID, VecDeque<DatedValue>>,
    precision: Duration,
    mode: ProcessRatesMode,
}


impl ProcessesRates {
    /// Creates a new ProcessRates structure, configured to calculate the frequency from the last
    /// data covered by the given retention duration
    ///
    /// # Arguments
    ///  * `precision`: Indicates over how much time to calculate the rate. This class will always
    ///         return rates in Hertz, but we calculate it from data over this time span.
    pub fn new(mode: ProcessRatesMode, data_retention: Duration) -> Self {
        ProcessesRates { acc_values: HashMap::new(), precision: data_retention, mode }
    }

    /// Pushes a new data associated to the given PID
    pub fn push(&mut self, pid: PID, value: usize) {
        let values = match self.acc_values.entry(pid) {
            Entry::Occupied(o) => o.into_mut(),
            Entry::Vacant(v) => v.insert(VecDeque::new()),
        };
        let now = Instant::now();

        let actual_value = match self.mode {
            ProcessRatesMode::ACCUMULATIVE => value,
            ProcessRatesMode::INCREMENT => {
                values.back()
                    .map(|dv| dv.value)
                    .unwrap_or(0)
                    .add(value)
            }
        };

        values.push_back(DatedValue { date: now, value: actual_value });
        self.remove_outdated_values(pid, now);
    }

    /// Removes all outdated values but the latest one, for the given PID
    pub fn remove_outdated_values(&mut self, pid: PID, now: Instant) {
        let values = self.acc_values.get_mut(&pid).unwrap();
        let data_retention = self.precision;

        let last_outdated = values.iter()
            .filter(|dv| dv.date < now - data_retention)
            .max_by(|dv_1, dv_2| dv_1.date.cmp(&dv_2.date))
            .map(|dv| dv.clone());

        values.retain(|dv| (now - dv.date) <= data_retention);

        if let Some(last_outdated) = last_outdated {
            values.push_front(last_outdated);
        }
    }

    /// Calculates a rate (in hertz) from the known accumulative values of the associated PID.
    ///
    /// This value is computed by calculating the increment between the first and last values within
    /// the span of the given retention. The projected rate over one second is then normalized over
    /// 1 second.
    ///
    /// # Arguments
    ///  * `pid`: The PID of the process for which to calculate the rate
    ///
    pub fn rate(&self, pid: PID) -> Result<f64, ProcfsError> {
        let values = self.acc_values.get(&pid)
            .ok_or(ProcfsError::UnknownPID(pid))?;

        if values.len() < 2 {
            return Ok(0.);
        }

        let cur_val = values.back().unwrap();
        let origin_val = self.estimate_origin_value(values, cur_val.date).unwrap();

        let rate = (cur_val.value as f64 - origin_val) / self.precision.as_secs_f64();

        Ok(rate)
    }

    /// Estimate the value at the date `now - self.precision`
    /// This very simple implementation estimates this value by performing a regression from the
    /// first two values of `values`
    fn estimate_origin_value(&self, values: &VecDeque<DatedValue>, now: Instant) -> Option<f64> {
        let origin = now - self.precision;

        if values.len() < 2 {
            return None;  // If there is not two values
        }

        let first = values.front().unwrap();
        let second = values.get(1).unwrap();

        if first.date == second.date {
            return None;  // If the two dates are identical, we don't want to divide by zero below
        }

        let slope = (second.value - first.value) as f64 / (second.date - first.date).as_secs_f64();

        let origin_time_delta = Self::get_delta_as_secs_f64(origin, first.date);
        let regression = first.value as f64 + slope * (origin_time_delta);

        Some(regression)
    }

    /// Gets the difference, in second, between the two Instant instances, allowing negative
    /// values
    fn get_delta_as_secs_f64(instant_1: Instant, instant_2: Instant) -> f64 {
        if instant_1 < instant_2 {
            -(instant_2 - instant_1).as_secs_f64()
        } else {
            (instant_1 - instant_2).as_secs_f64()
        }
    }


    // TODO Clear PID from this structure when not needed anymore
}

#[cfg(test)]
mod test_process_rates {
    use std::time::{Duration, Instant};

    use rstest::*;
    use sn_fake_clock::FakeClock;

    use crate::procfs::ProcfsError;
    use crate::procfs::rates::{ProcessesRates, ProcessRatesMode};

    #[fixture]
    fn process_rates() -> ProcessesRates {
        FakeClock::set_time(10000);
        ProcessesRates::new(ProcessRatesMode::ACCUMULATIVE,
                            Duration::from_secs(1))
    }

    #[rstest]
    fn test_rate_returns_error_if_pid_not_known(mut process_rates: ProcessesRates) {
        assert!(process_rates.rate(123).is_err());
    }

    #[rstest]
    fn test_rate_should_be_zero_when_acc_values_are_zero(mut process_rates: ProcessesRates) {
        process_rates.push(123, 0);
        FakeClock::advance_time(500);
        process_rates.push(123, 0);

        assert_eq!(process_rates.rate(123).unwrap(), 0.);
    }

    #[rstest]
    fn test_rate_should_be_projected_increase_over_retention(mut process_rates: ProcessesRates) {
        process_rates.push(123, 0);
        FakeClock::advance_time(1000);
        process_rates.push(123, 100);

        assert_eq!(process_rates.rate(123).unwrap(), 100.);
    }

    #[rstest]
    fn test_rate_should_be_zero_when_only_one_value(mut process_rates: ProcessesRates) {
        process_rates.push(123, 0);

        assert_eq!(process_rates.rate(123).unwrap(), 0.);
    }

    #[rstest]
    fn test_should_ignore_outdated_values(mut process_rates: ProcessesRates) {
        process_rates.push(123, 0);
        FakeClock::advance_time(50);
        process_rates.push(123, 100);
        FakeClock::advance_time(2000);
        process_rates.push(123, 100);
        FakeClock::advance_time(500);
        process_rates.push(123, 100); // Over the last second, the value remained at 100

        assert_eq!(process_rates.rate(123).unwrap(), 0.);
    }

    #[rstest]
    fn test_should_compute_rate_from_outdated_and_recent_value(mut process_rates: ProcessesRates) {
        // In this test, we have one outdated data from 2s ago, and one data from 0s ago
        process_rates.push(123, 0);
        FakeClock::advance_time(2000);
        process_rates.push(123, 100);

        assert_eq!(process_rates.rate(123).unwrap(), 50.);  // 100 over 2s -> 50/s
    }

    #[rstest]
    fn test_should_compute_rate_from_out_dated_and_multiple_recent_values(mut process_rates: ProcessesRates) {
        // In this more complex test, we have:
        // - One outdated data from 2s ago
        process_rates.push(123, 0);
        FakeClock::advance_time(1500);
        // - One data from 0.5s ago
        process_rates.push(123, 150);
        FakeClock::advance_time(500);
        // - One data from 0s ago
        process_rates.push(123, 250);

        // 100 in the last 0.5 sec + 150 in the previous 1.5 sec -> 100 + 150/3 -> 150/s
        assert_eq!(process_rates.rate(123).unwrap(), 150.);
    }
}