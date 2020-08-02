use std::collections::{HashMap, HashSet};

use crate::probe::{Error, Probe};
use crate::probe::values::{Bitrate, Percent, Value};
use crate::process::PID;

#[derive(PartialEq, Debug)]
/// Contains for a set of `PID` their associated values measured at a given time
pub enum Metrics {
    /// Describes the `Percent` values for a set of PID for a given metric
    Percents(HashMap<PID, Percent>),
    /// Describes the `Bitrate` values for a set of PID for a given metric
    Bitrates(HashMap<PID, Bitrate>),
}

#[cfg(test)]
impl Metrics {
    /// Helper function to construct a Percent containing LabelledMetrics
    /// # Arguments
    ///  * `metrics`: A slice of tuple, each containing the PID and its associated Percent value
    pub fn from_percents(metrics: HashMap<PID, PercentType>) -> Result<Self, Error> {
        Ok(Metrics::Percents(metrics.iter()
            .map(|(pid, pct_val)| Ok((*pid, Percent::new(*pct_val)?)))
            .collect::<Result<_, _>>()?))
    }

    /// Helper function to construct a Bitrate containing LabelledMetrics
    /// # Arguments
    ///  * `metrics`: A slice of tuple, each containing the PID and its associated Bitrate value
    pub fn from_bitrates(metrics: HashMap<PID, BitrateType>) -> Self {
        Metrics::Bitrates(metrics.iter()
            .map(|(pid, pct_val)| (*pid, Bitrate::new(*pct_val)))
            .collect())
    }
}

impl Metrics {
    /// Returns the PIDs which has an associated metric
    pub fn pids(&self) -> HashSet<PID> {
        match self {
            Self::Percents(map) => Self::get_pids_from_map(map),
            Self::Bitrates(map) => Self::get_pids_from_map(map),
        }
    }

    pub fn dump(&mut self, pid: PID) {
        match self {
            Self::Percents(map) => Self::dump_pid_from_map(map, pid),
            Self::Bitrates(map) => Self::dump_pid_from_map(map, pid),
        }
    }

    fn get_pids_from_map<V>(map: &HashMap<PID, V>) -> HashSet<PID> where V: Value {
        map.iter()
            .map(|(pid, _)| *pid)
            .collect()
    }

    fn dump_pid_from_map<V>(map: &mut HashMap<PID, V>, pid: PID) {
        map.remove(&pid);
    }
}

#[cfg(test)]
mod test_metrics {
    use crate::probe::dispatch::Metrics;
    use crate::probe::values::{Bitrate, Percent};

    #[test]
    fn test_should_get_no_pid_with_empty_metrics() {
        let metrics = vec![
            Metrics::Bitrates(hashmap!()),
            Metrics::Percents(hashmap!()),
        ];

        metrics.iter().for_each(|m| {
            assert_eq!(m.pids(), hashset!());
        });
    }

    #[test]
    fn test_should_get_pids_with_non_empty_metrics() {
        let metrics = vec![
            Metrics::Bitrates(hashmap!(1 => Bitrate::new(50), 2 => Bitrate::new(75))),
            Metrics::Percents(hashmap!(1 => Percent::new(50.).unwrap(), 2 => Percent::new(75.).unwrap())),
        ];

        metrics.iter().for_each(|m| {
            assert_eq!(m.pids(), hashset!(1, 2));
        });
    }

    #[test]
    fn test_should_no_get_pid_when_pid_dumped() {
        let mut metrics = vec![
            Metrics::Bitrates(hashmap!(1 => Bitrate::new(50), 2 => Bitrate::new(75))),
            Metrics::Percents(hashmap!(1 => Percent::new(50.).unwrap(), 2 => Percent::new(75.).unwrap())),
        ];

        metrics.iter_mut().for_each(|m| {
            m.dump(1);
            assert_eq!(m.pids(), hashset!(2));
        });
    }
}

type PercentType = <Percent as Value>::ValueType;
type BitrateType = <Bitrate as Value>::ValueType;

#[derive(PartialEq, Debug)]
/// A collection of `Metrics`
pub struct Frame {
    labelled_metrics: HashMap<String, Metrics>,
}

impl<'a> Frame {
    /// Returns a new Frame instance containing the given labelled metrics
    /// # Arguments
    ///  * `labelled_metrics`: A map associated to each `Metrics` instance a label
    ///
    /// The given metrics will be normalized. This means that if any `Metrics` contains a `PID` not
    /// present in any other `Metrics`, this `PID` will be discarded.
    pub fn new(labelled_metrics: HashMap<String, Metrics>) -> Self {
        Self { labelled_metrics: Self::normalize_metrics(labelled_metrics) }
    }

    fn normalize_metrics(mut labelled_metrics: HashMap<String, Metrics>) -> HashMap<String, Metrics> {
        let pids_sets: Vec<_> = labelled_metrics.iter()
            .map(|(_, m)| m.pids())
            .collect();

        if let Some(first_pids) = pids_sets.get(0) {
            pids_sets.iter()
                .skip(1)
                .flat_map(|pids| first_pids.symmetric_difference(pids))
                .for_each(|pid_to_dump| {
                    labelled_metrics.values_mut()
                        .for_each(|m| m.dump(*pid_to_dump))
                });
        }

        labelled_metrics
    }

    /// Returns the labels of the metrics from this frame
    pub fn labels(&'a self) -> HashSet<&'a str> {
        self.labelled_metrics.keys()
            .map(|s| s.as_str())
            .collect()
    }

    /// Returns the `Metrics` corresponding to the given label
    /// A `Metrics` instances contains a metric for a set of processes
    /// # Arguments
    ///  * `label`: The name associated to the `Metrics`
    pub fn metrics(&'a self, label: &str) -> Option<&'a Metrics> {
        self.labelled_metrics.get(label)
    }
}

#[cfg(test)]
mod test_frame {
    use std::collections::HashMap;

    use crate::probe::dispatch::{Frame, Metrics};
    use crate::probe::values::Bitrate;

    #[test]
    fn test_should_return_correct_labels() {
        let mut metrics = HashMap::new();
        metrics.insert("metrics_1".into(), Metrics::from_bitrates(hashmap!(123 => 50)));
        metrics.insert("metrics_2".into(), Metrics::from_bitrates(hashmap!(123 => 100)));

        assert_eq!(Frame::new(metrics).labels(), hashset!("metrics_1", "metrics_2"));
    }

    #[test]
    fn test_should_return_correct_values() {
        let mut metrics = HashMap::new();
        metrics.insert("metrics_1".into(), Metrics::from_bitrates(hashmap!(123 => 50)));
        metrics.insert("metrics_2".into(), Metrics::from_bitrates(hashmap!(123 => 100)));

        let frame = Frame::new(metrics);

        assert_eq!(frame.metrics("metrics_1"),
                   Some(&Metrics::Bitrates(hashmap!(123 => Bitrate::new(50)))));

        assert_eq!(frame.metrics("metrics_2"),
                   Some(&Metrics::Bitrates(hashmap!(123 => Bitrate::new(100)))));
    }

    #[test]
    fn test_should_return_none_when_metric_does_not_exist() {
        let mut metrics = HashMap::new();
        metrics.insert("metrics".into(), Metrics::from_bitrates(hashmap!(123 => 50)));

        let frame = Frame::new(metrics);

        assert_eq!(frame.metrics("metrics_invalid"), None);
    }

    #[test]
    fn test_should_disregard_pids_not_in_all_metrics() {
        let mut metrics = HashMap::new();
        metrics.insert("metrics_1".into(),
                       Metrics::from_percents(hashmap!(1 => 25., 2 => 50.)).unwrap());
        metrics.insert("metrics_2".into(),
                       Metrics::from_percents(hashmap!(0 => 30., 2 => 55.)).unwrap());
        metrics.insert("metrics_3".into(),
                       Metrics::from_percents(hashmap!(2 => 30., 3 => 60.)).unwrap());

        let frame = Frame::new(metrics);

        assert_eq!(frame.metrics("metrics_1"),
                   Some(&Metrics::from_percents(hashmap!(2 => 50.)).unwrap()));
        assert_eq!(frame.metrics("metrics_2"),
                   Some(&Metrics::from_percents(hashmap!(2 => 55.)).unwrap()));
        assert_eq!(frame.metrics("metrics_3"),
                   Some(&Metrics::from_percents(hashmap!(2 => 30.)).unwrap()));
    }
}

/// Orchestrates multiple probes to produce a `Frame` instance on demand
pub struct ProbeDispatcher {
    last_frame: Option<Frame>,
    processes: HashSet<PID>,
    labelled_probes: HashMap<String, Box<dyn Probe>>,
}

impl ProbeDispatcher {
    /// Returns a new instance of ProbeDispatcher. By default, this instance contains no probe
    /// and tracks no process
    pub fn new() -> Self {
        Self { last_frame: None, processes: HashSet::new(), labelled_probes: HashMap::new() }
    }

    /// Adds a new probe to measure `Metrics` with
    /// # Arguments
    ///  * `label`: The label to associate to the `Metrics` produced by the probe
    ///  * `probe`: A boxed Probe instance, used to collect `Metrics` of processes
    pub fn add_probe<L>(&mut self, label: L, probe: Box<dyn Probe>) where L: Into<String> {
        self.labelled_probes.insert(label.into(), probe);
    }

    /// Adds a new process to track
    /// # Arguments
    pub fn add_process(&mut self, pid: PID) {
        self.processes.insert(pid);
    }

    pub fn drop_process(&mut self, pid: PID) {
        self.processes.remove(&pid);
    }

    pub fn probe(&mut self) -> Result<(), Error> {
        let processes = &self.processes;

        let metrics = self.labelled_probes.iter_mut()
            .map(|(label, probe)| {
                Ok((label.to_string(), probe.probe_processes(processes)?))
            })
            .collect::<Result<_, _>>()?;

        self.last_frame = Some(Frame::new(metrics));

        Ok(())
    }

    pub fn frame(&mut self) -> Option<Frame> {
        self.last_frame.take()
    }
}

#[cfg(test)]
mod test_probe_dispatcher {
    use std::collections::{HashMap, HashSet};

    use crate::probe::{Error, Probe};
    use crate::probe::dispatch::{Frame, Metrics, ProbeDispatcher};
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
                .map(|p| (*p, self.value))
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
                   Some(&Metrics::Percents(hashmap!())));
    }

    #[test]
    fn test_should_collect_empty_metrics_when_only_process_has_been_removed() {
        let mut dispatcher = ProbeDispatcher::new();

        dispatcher.add_process(123);
        dispatcher.drop_process(123);

        dispatcher.add_probe("my-probe", Box::new(ProbeFake::new(50.)));
        dispatcher.probe().expect("Error while probing");

        assert_eq!(dispatcher.frame()
                       .expect("No frame received")
                       .metrics("my-probe"),
                   Some(&Metrics::Percents(hashmap!())));
    }

    #[test]
    fn test_should_collect_one_frame_when_one_probe_added() {
        let mut dispatcher = ProbeDispatcher::new();

        dispatcher.add_probe("my-probe", Box::new(ProbeFake::new(50.)));
        dispatcher.add_process(123);
        dispatcher.probe().expect("Error while probing");

        let mut expected = HashMap::new();
        expected.insert("my-probe".into(),
                        Metrics::from_percents(hashmap!(123 => 50.)).unwrap());

        assert_eq!(dispatcher.frame(), Some(Frame::new(expected)));
    }

    #[test]
    fn test_should_collect_correct_frame_with_two_probes_and_two_processes() {
        let mut dispatcher = ProbeDispatcher::new();

        dispatcher.add_probe("my-probe-1", Box::new(ProbeFake::new(50.)));
        dispatcher.add_probe("my-probe-2", Box::new(ProbeFake::new(25.)));

        dispatcher.add_process(123);
        dispatcher.add_process(124);

        dispatcher.probe().expect("Error while probing");

        let frame = dispatcher.frame().expect("Frame is none");

        let mut expected = HashMap::new();
        expected.insert("my-probe-1".into(),
                        Metrics::from_percents(hashmap!(123 => 50., 124 => 50.)).unwrap());
        expected.insert("my-probe-2".into(),
                        Metrics::from_percents(hashmap!(123 => 25., 124 => 25.)).unwrap());

        assert_eq!(frame, Frame::new(expected));
    }
}
