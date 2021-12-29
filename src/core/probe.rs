//! Probe trait definition, used to implement metrics collection

use std::collections::HashMap;

use log::warn;

use crate::core::metrics::Metric;
use crate::core::process::Pid;
use crate::core::Error;

/// Types which can probe processes for a specific implementation of [`Metric`](crate::core::metrics::Metric)
pub trait Probe<M>
where
    M: Metric + Copy + Default,
{
    /// The name of the probe, as displayed in the application tab
    fn name(&self) -> &'static str;

    /// Called on each probe refresh, before all processes are probed
    fn init_iteration(&mut self) -> Result<(), Error> {
        Ok(())
    }

    /// Probe a given process for a [`Metric`](crate::core::metrics::Metric)
    fn probe(&mut self, pid: Pid) -> Result<M, Error>;

    /// Returns a map associating a [`Metric`](crate::core::metrics::Metric) instance to each PID
    ///
    /// If an error occurs while probing a process, a default metric is returned for this process,
    /// and a WARNING level log is produced
    ///
    /// # Arguments
    ///  * `pids`: A set of `PIDs` to monitor
    ///
    fn probe_processes(&mut self, pids: &[Pid]) -> Result<HashMap<Pid, M>, Error> {
        self.init_iteration()?;

        let metrics = pids
            .iter()
            .map(|pid| {
                let metric = self.probe(*pid).unwrap_or_else(|e| {
                    warn!(
                        "Could not probe {} metric for pid {}: {}",
                        self.name(),
                        pid,
                        e.to_string()
                    );
                    M::default()
                });

                (*pid, metric)
            })
            .collect();

        Ok(metrics)
    }
}

#[cfg(test)]
pub mod fakes {
    use std::collections::HashMap;

    use crate::core::metrics::{Metric, PercentMetric};
    use crate::core::probe::Probe;
    use crate::core::process::Pid;
    use crate::core::Error;

    pub struct FakeProbe<M>
    where
        M: Metric + Copy + Default,
    {
        probed_metrics: HashMap<Pid, Result<M, Error>>,
    }

    impl FakeProbe<PercentMetric> {
        pub fn from_percent_map(map: HashMap<Pid, f64>) -> Self {
            let probed_metrics = map
                .into_iter()
                .map(|(pid, val)| (pid, Ok(PercentMetric::new(val))))
                .collect();

            FakeProbe { probed_metrics }
        }
    }

    impl<M> FakeProbe<M>
    where
        M: Metric + Copy + Default,
    {
        pub fn new() -> Self {
            Self {
                probed_metrics: hashmap!(),
            }
        }
    }

    impl<M> FakeProbe<M>
    where
        M: Metric + Copy + Default,
    {
        pub fn make_pid_fail(&mut self, pid: Pid) {
            self.probed_metrics.insert(pid, Err(Error::InvalidPID(pid)));
        }
    }

    impl<M> Probe<M> for FakeProbe<M>
    where
        M: Metric + Copy + Default,
    {
        fn name(&self) -> &'static str {
            "fake"
        }

        fn probe(&mut self, pid: Pid) -> Result<M, Error> {
            self.probed_metrics
                .remove(&pid)
                .expect("No metric has been set for this pid")
        }
    }
}

#[cfg(test)]
mod test_probe_trait {
    use rstest::rstest;

    use crate::core::metrics::PercentMetric;
    use crate::core::probe::fakes::FakeProbe;
    use crate::core::probe::Probe;

    #[rstest]
    fn test_should_return_all_probed_values() {
        let mut probe = FakeProbe::from_percent_map(hashmap!(1 => 10., 2 => 20.));

        let results = probe.probe_processes(&[1, 2]).unwrap();

        assert_eq!(results.len(), 2);
        assert_eq!(results.get(&1), Some(&PercentMetric::new(10.)));
        assert_eq!(results.get(&2), Some(&PercentMetric::new(20.)));
    }

    #[rstest]
    fn test_should_return_default_value_if_probing_fails() {
        let mut probe = FakeProbe::from_percent_map(hashmap!(1 => 10.));
        probe.make_pid_fail(2);

        let results = probe.probe_processes(&[1, 2]).unwrap();

        assert_eq!(results.len(), 2);
        assert_eq!(results.get(&1), Some(&PercentMetric::new(10.)));
        assert_eq!(results.get(&2), Some(&PercentMetric::default()));
    }
}
