//! Probe trait definition, used to implement metrics collection

use std::collections::HashMap;

use log::warn;

use crate::core::Error;
use crate::core::metrics::Metric;
use crate::core::process::Pid;

/// Types which can probe processes for a specific implementation of [`Metric`](crate::core::metrics::Metric)
pub trait Probe<M> where M: Metric + Copy + Default {
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
    /// If a process is not probed correctly, a default value for the given probe is returned
    /// and a WARNING level log is produced
    ///
    /// # Arguments
    ///  * `pids`: A set of `PIDs` to monitor
    ///
    fn probe_processes(&mut self, pids: &[Pid]) -> Result<HashMap<Pid, M>, Error> {
        self.init_iteration()?;

        let metrics = pids.iter()
            .map(|pid| {
                let metric = self.probe(*pid)
                    .unwrap_or_else(|e| {
                        warn!("Could not probe {} metric for pid {}: {}",
                              self.name(), pid, e.to_string());
                        M::default()
                    });

                (*pid, metric)
            })
            .collect();

        Ok(metrics)
    }
}


#[cfg(test)]
mod test_probe_trait {
    use std::collections::HashMap;

    use rstest::*;

    use crate::core::Error;
    use crate::core::metrics::PercentMetric;
    use crate::core::probe::Probe;
    use crate::core::process::Pid;

    struct FakeProbe {
        probe_responses: HashMap<Pid, PercentMetric>,
    }

    impl FakeProbe {
        pub fn new(probe_responses: HashMap<Pid, PercentMetric>) -> Self {
            Self { probe_responses }
        }
    }

    impl Probe<PercentMetric> for FakeProbe {
        fn name(&self) -> &'static str { "fake-probe" }

        fn probe(&mut self, pid: u32) -> Result<PercentMetric, Error> {
            self.probe_responses.remove(&pid)
                .ok_or(Error::InvalidPID(pid))
        }
    }

    #[rstest]
    fn test_should_return_all_probed_values() {
        let mut probe = FakeProbe::new(hashmap!(
            1 => PercentMetric::new(10.),
            2 => PercentMetric::new(20.)
        ));

        let results = probe.probe_processes(&[1, 2]).unwrap();

        assert_eq!(results.len(), 2);
        assert_eq!(results.get(&1), Some(&PercentMetric::new(10.)));
        assert_eq!(results.get(&2), Some(&PercentMetric::new(20.)));
    }

    #[rstest]
    fn test_should_return_default_value_if_probing_fails() {
        let mut probe = FakeProbe::new(hashmap!(
            1 => PercentMetric::new(10.)
        ));

        let results = probe.probe_processes(&[1, 2]).unwrap();

        assert_eq!(results.len(), 2);
        assert_eq!(results.get(&1), Some(&PercentMetric::new(10.)));
        assert_eq!(results.get(&2), Some(&PercentMetric::default()));
    }
}

