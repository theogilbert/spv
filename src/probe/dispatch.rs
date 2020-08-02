use std::collections::{HashMap, HashSet};

use crate::probe::{Error, Probe, ProcessMetric};
use crate::probe::values::{Bitrate, Percent, Value};
use crate::process::PID;

#[derive(PartialEq, Debug)]
pub enum Metrics {
    Percents(Vec<ProcessMetric<Percent>>),
    Bitrates(Vec<ProcessMetric<Bitrate>>),
}

#[cfg(test)]
impl Metrics {
    fn sort_by_pid(&mut self) {
        match self {
            Metrics::Percents(pms) => pms.sort_by_key(|pm| pm.pid),
            Metrics::Bitrates(pms) => pms.sort_by_key(|pm| pm.pid),
        }
    }
}

#[derive(PartialEq, Debug)]
pub struct LabelledMetrics {
    label: String,
    metrics: Metrics,
}

type PercentType = <Percent as Value>::ValueType;
type BitrateType = <Bitrate as Value>::ValueType;

#[cfg(test)]
impl LabelledMetrics {
    /// Helper function to construct a Percent containing LabelledMetrics
    /// # Arguments
    ///  * `label`: The label to associate to the metrics
    ///  * `metrics`: A slice of tuple, each containing the PID and its associated Percent value
    pub fn from_percents<L>(label: L, metrics: &[(PID, PercentType)]) -> Result<Self, Error>
        where L: Into<String> {
        let process_metrics: Vec<ProcessMetric<_>> = metrics.iter()
            .map(|(pid, pct_val)| {
                Ok(ProcessMetric::new(*pid, Percent::new(*pct_val)?))
            })
            .collect::<Result<_, _>>()?;

        Ok(LabelledMetrics {
            label: label.into(),
            metrics: Metrics::Percents(process_metrics),
        })
    }

    /// Helper function to construct a Bitrate containing LabelledMetrics
    /// # Arguments
    ///  * `label`: The label to associate to the metrics
    ///  * `metrics`: A slice of tuple, each containing the PID and its associated Bitrate value
    pub fn from_bitrates<L>(label: L, metrics: &[(PID, BitrateType)]) -> Self
        where L: Into<String> {
        LabelledMetrics {
            label: label.into(),
            metrics: Metrics::Bitrates(metrics.iter()
                .map(|(pid, pct_val)| {
                    ProcessMetric::new(*pid, Bitrate::new(*pct_val))
                })
                .collect()),
        }
    }
}

#[derive(PartialEq, Debug)]
pub struct Frame {
    labelled_metrics: Vec<LabelledMetrics>,
}

impl<'a> Frame {
    pub fn new(metrics: Vec<LabelledMetrics>) -> Self {
// TODO if a PID is not in one of the metrics, remove it from all others
        Self { labelled_metrics: metrics }
    }

    pub fn labels(&'a self) -> Vec<&'a str> {
        self.labelled_metrics.iter()
            .map(|lm| lm.label.as_str())
            .collect()
    }

    pub fn metrics(&'a self, label: &str) -> Option<&'a Metrics> {
        self.labelled_metrics.iter()
            .find(|lm| lm.label == label)
            .map(|lm| &lm.metrics)
    }
}

#[cfg(test)]
impl Frame {
    fn sort_by_label(&mut self) {
        self.labelled_metrics.sort_by(|lm1, lm2| {
            lm1.label.as_str().cmp(&lm2.label)
        });
    }

    fn sort_metrics_by_pid(&mut self) {
        self.labelled_metrics.iter_mut()
            .for_each(|lm| lm.metrics.sort_by_pid());
    }
}

#[cfg(test)]
mod test_frame {
    use crate::probe::dispatch::{Frame, LabelledMetrics, Metrics};
    use crate::probe::ProcessMetric;
    use crate::probe::values::Percent;

    #[test]
    fn test_should_return_correct_labels() {
        let metrics = vec![
            LabelledMetrics::from_bitrates("metrics_1", &[(123, 100)]),
            LabelledMetrics::from_bitrates("metrics_2", &[(123, 100)]),
        ];

        assert_eq!(Frame::new(metrics).labels(),
                   vec!["metrics_1", "metrics_2"]);
    }

    #[test]
    fn test_should_return_correct_values() {

        let metrics = vec![
            LabelledMetrics::from_percents("metrics_1", &[(123, 50.)]).unwrap(),
            LabelledMetrics::from_percents("metrics_2", &[(123, 100.)]).unwrap(),
        ];

        assert_eq!(Frame::new(metrics).metrics("metrics_1"),
                   Some(&Metrics::Percents(vec![ProcessMetric {
                       pid: 123,
                       value: Percent::new(50.).unwrap(),
                   }])))
    }
}

pub struct ProbeDispatcher {
    last_frame: Option<Frame>,
    processes: HashSet<PID>,
    probes: HashMap<String, Box<dyn Probe>>,
}

impl ProbeDispatcher {
    pub fn new() -> Self {
        Self { last_frame: None, processes: HashSet::new(), probes: HashMap::new() }
    }

    pub fn add_probe<L>(&mut self, label: L, probe: Box<dyn Probe>) where L: Into<String> {
        self.probes.insert(label.into(), probe);
    }

    pub fn add_process(&mut self, pid: PID) {
        self.processes.insert(pid);
    }

    pub fn probe(&mut self) -> Result<(), Error> {
        let processes = &self.processes;

        let labelled_metrics = self.probes.iter_mut()
            .map(|(l, p)| {
                Ok(LabelledMetrics {
                    label: l.to_string(),
                    metrics: p.probe_processes(processes)?,
                })
            })
            .collect::<Result<_, _>>()?;

        self.last_frame = Some(Frame::new(labelled_metrics));

        Ok(())
    }

    pub fn frame(&mut self) -> Option<Frame> {
        self.last_frame.take()
    }
}

#[cfg(test)]
mod test_probe_dispatcher {
    use std::collections::HashSet;

    use crate::probe::{Error, Probe, ProcessMetric};
    use crate::probe::dispatch::{Frame, LabelledMetrics, Metrics, ProbeDispatcher};
    use crate::probe::values::Percent;

    struct ProbeFake {
        value: Percent,
    }

    impl ProbeFake {
        fn new(percent_val: f32) -> Self {
            Self { value: Percent::new(percent_val).unwrap() }
        }
    }

    impl Probe for ProbeFake {
        fn probe_processes(&mut self, pids: &HashSet<u32>) -> Result<Metrics, Error> {
            Ok(Metrics::Percents(pids.iter()
                .map(|p| ProcessMetric::new(*p, self.value))
                .collect()))
        }
    }

    #[test]
    fn test_should_collect_nothing_when_probe_not_called() {
        let mut dispatcher = ProbeDispatcher::new();

        assert_eq!(dispatcher.frame(), None);
    }

    #[test]
    fn test_should_collect_empty_frame_when_no_probe_or_process_added() {
        let mut dispatcher = ProbeDispatcher::new();

        dispatcher.probe().expect("Error while probing");

        assert!(dispatcher.frame()
            .expect("No frame received")
            .labels()
            .is_empty());
    }

    #[test]
    fn test_should_collect_empty_frame_when_no_probe_added() {
        let mut dispatcher = ProbeDispatcher::new();

        dispatcher.add_process(123);
        dispatcher.probe().expect("Error while probing");

        assert!(dispatcher.frame()
            .expect("No frame received")
            .labels()
            .is_empty());
    }

    #[test]
    fn test_should_collect_empty_metrics_when_no_process_added() {
        let mut dispatcher = ProbeDispatcher::new();

        dispatcher.add_probe("my-probe", Box::new(ProbeFake::new(50.)));
        dispatcher.probe().expect("Error while probing");

        assert_eq!(dispatcher.frame()
                       .expect("No frame received")
                       .metrics("my-probe"),
                   Some(&Metrics::Percents(vec![])));
    }

    #[test]
    fn test_should_collect_one_frame_when_one_probe_added() {
        let mut dispatcher = ProbeDispatcher::new();

        dispatcher.add_probe("my-probe", Box::new(ProbeFake::new(50.)));
        dispatcher.add_process(123);
        dispatcher.probe().expect("Error while probing");

        assert_eq!(dispatcher.frame(),
                   Some(Frame::new(vec![
                       LabelledMetrics::from_percents("my-probe", &[(123, 50.)]).unwrap()
                   ])));
    }

    #[test]
    fn test_should_collect_correct_frame_with_two_probes_and_two_processes() {
        let mut dispatcher = ProbeDispatcher::new();

        dispatcher.add_probe("my-probe-1", Box::new(ProbeFake::new(50.)));
        dispatcher.add_probe("my-probe-2", Box::new(ProbeFake::new(25.)));

        dispatcher.add_process(123);
        dispatcher.add_process(124);

        dispatcher.probe().expect("Error while probing");

        let mut frame = dispatcher.frame().expect("Frame is none");
        frame.sort_by_label();
        frame.sort_metrics_by_pid();

        assert_eq!(frame,
                   Frame::new(vec![
                       LabelledMetrics::from_percents("my-probe-1", &[(123, 50.), (124, 50.)]).unwrap(),
                       LabelledMetrics::from_percents("my-probe-2", &[(123, 25.), (124, 25.)]).unwrap(),
                   ]));
    }
}
