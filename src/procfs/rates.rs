use std::collections::{HashMap, VecDeque};
use std::collections::hash_map::Entry;
use std::ops::Sub;
use std::time::Duration;
#[cfg(not(test))]
use std::time::Instant;

#[cfg(test)]
use sn_fake_clock::FakeClock as Instant;

use crate::core::process_view::PID;
use crate::procfs::ProcfsError;

struct DatedValue
{
    date: Instant,
    value: usize,
}

/// Keeps tracks of dated accumulative values of processes to calculate their rate
pub struct ProcessesRates {
    acc_values: HashMap<PID, VecDeque<DatedValue>>,
    data_retention: Duration,
}


impl ProcessesRates {
    /// Creates a new ProcessRates structure, configured to calculate the frequency from the last
    /// data covered by the given retention duration
    ///
    /// # Arguments
    ///  * `data_retention`: Indicates from how far back to use data to calculate rates
    pub fn new(data_retention: Duration) -> Self {
        ProcessesRates { acc_values: HashMap::new(), data_retention }
    }

    /// Pushes a new data associated to the given PID
    pub fn push(&mut self, pid: PID, value: usize) {
        let values = match self.acc_values.entry(pid) {
            Entry::Occupied(o) => o.into_mut(),
            Entry::Vacant(v) => v.insert(VecDeque::new()),
        };

        values.push_back(DatedValue { date: Instant::now(), value })
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
    pub fn rate(&mut self, pid: PID) -> Result<f64, ProcfsError> {
        let values = self.acc_values.get_mut(&pid)
            .ok_or(ProcfsError::UnknownPID(pid))?;

        Self::clean_outdated_data(values, self.data_retention);

        if values.len() > 1 {
            let front = values.front().unwrap();
            let back = values.back().unwrap();

            let increase = back.value - front.value;
            let span = back.date - front.date;

            let projected_rate = increase as f64 / span.as_secs_f64();

            Ok(projected_rate)
        } else {
            Ok(0.)
        }
    }

    /// Gets rid of outdated data for the given PID
    /// # Arguments
    ///  * `acc_values`: The VecDeque containing all dated values associated to a process
    ///  * `retention`: The maximum duration for which to keep values
    fn clean_outdated_data(acc_values: &mut VecDeque<DatedValue>, retention: Duration) {
        let now = Instant::now();
        acc_values.retain(|dv| (now - dv.date) <= retention);
    }

    // TODO Clear PID from this structure when not needed anymore
}

#[cfg(test)]
mod test_process_rates {
    use std::time::{Duration, Instant};

    use rstest::*;
    use sn_fake_clock::FakeClock;

    use crate::procfs::ProcfsError;
    use crate::procfs::rates::ProcessesRates;

    #[fixture]
    fn process_rates() -> ProcessesRates {
        ProcessesRates::new(Duration::from_secs(1))
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
        FakeClock::advance_time(2000);
        process_rates.push(123, 100);
        FakeClock::advance_time(500);
        process_rates.push(123, 100); // Over the last second, the value remained at 100

        assert_eq!(process_rates.rate(123).unwrap(), 0.);
    }
}