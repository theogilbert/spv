use std::collections::{HashMap, HashSet};

use crate::probe::{Error, Probe};
use crate::probe::values::{Bitrate, Percent, Value};
use crate::process::PID;

#[derive(PartialEq, Debug)]
/// Contains for a set of `PID` their associated values measured at a given time
pub enum Snapshot {
    /// Describes the `Percent` values for a set of PID for a given metric
    Percents(HashMap<PID, Percent>),
    /// Describes the `Bitrate` values for a set of PID for a given metric
    Bitrates(HashMap<PID, Bitrate>),
}

#[cfg(test)]
impl Snapshot {
    /// Helper function to construct a Percent containing LabelledMetrics
    /// # Arguments
    ///  * `metrics`: A slice of tuple, each containing the PID and its associated Percent value
    pub fn from_percents(metrics: HashMap<PID, PercentType>) -> Result<Self, Error> {
        Ok(Snapshot::Percents(metrics.iter()
            .map(|(pid, pct_val)| Ok((*pid, Percent::new(*pct_val)?)))
            .collect::<Result<_, _>>()?))
    }

    /// Helper function to construct a Bitrate containing LabelledMetrics
    /// # Arguments
    ///  * `metrics`: A slice of tuple, each containing the PID and its associated Bitrate value
    pub fn from_bitrates(metrics: HashMap<PID, BitrateType>) -> Self {
        Snapshot::Bitrates(metrics.iter()
            .map(|(pid, pct_val)| (*pid, Bitrate::new(*pct_val)))
            .collect())
    }

    /// Returns the PIDs contained in the snapshot
    pub fn pids(&self) -> HashSet<PID> {
        match self {
            Self::Percents(map) => Self::get_pids_from_map(map),
            Self::Bitrates(map) => Self::get_pids_from_map(map),
        }
    }

    fn get_pids_from_map<V>(map: &HashMap<PID, V>) -> HashSet<PID> where V: Value {
        map.iter()
            .map(|(pid, _)| *pid)
            .collect()
    }
}

#[cfg(test)]
mod test_metrics {
    use crate::probe::dispatch::Snapshot;
    use crate::probe::values::{Bitrate, Percent};

    #[test]
    fn test_should_get_no_pid_with_empty_metrics() {
        let metrics = vec![
            Snapshot::Bitrates(hashmap!()),
            Snapshot::Percents(hashmap!()),
        ];

        metrics.iter().for_each(|m| {
            assert_eq!(m.pids(), hashset!());
        });
    }

    #[test]
    fn test_should_get_pids_with_non_empty_metrics() {
        let metrics = vec![
            Snapshot::Bitrates(hashmap!(1 => Bitrate::new(50), 2 => Bitrate::new(75))),
            Snapshot::Percents(hashmap!(1 => Percent::new(50.).unwrap(), 2 => Percent::new(75.).unwrap())),
        ];

        metrics.iter().for_each(|m| {
            assert_eq!(m.pids(), hashset!(1, 2));
        });
    }
}

type PercentType = <Percent as Value>::ValueType;
type BitrateType = <Bitrate as Value>::ValueType;

#[derive(PartialEq, Debug)]
/// A collection of Snapshot
pub struct Frame {
    labelled_snapshots: HashMap<String, Snapshot>,
}

impl<'a> Frame {
    pub fn new(labelled_snapshots: HashMap<String, Snapshot>) -> Self {
// TODO if a PID is not in one of the metrics, remove it from all others
        Self { labelled_snapshots }
    }

    pub fn labels(&'a self) -> HashSet<&'a str> {
        self.labelled_snapshots.keys()
            .map(|s| s.as_str())
            .collect()
    }

    pub fn metrics(&'a self, label: &str) -> Option<&'a Snapshot> {
        self.labelled_snapshots.get(label)
    }
}

#[cfg(test)]
mod test_frame {
    use std::collections::HashMap;

    use crate::probe::dispatch::{Frame, Snapshot};
    use crate::probe::values::{Bitrate};

    #[test]
    fn test_should_return_correct_labels() {
        let mut metrics = HashMap::new();
        metrics.insert("metrics_1".into(), Snapshot::from_bitrates(hashmap!(123 => 50)));
        metrics.insert("metrics_2".into(), Snapshot::from_bitrates(hashmap!(123 => 100)));

        assert_eq!(Frame::new(metrics).labels(), hashset!("metrics_1", "metrics_2"));
    }

    #[test]
    fn test_should_return_correct_values() {
        let mut metrics = HashMap::new();
        metrics.insert("metrics_1".into(), Snapshot::from_bitrates(hashmap!(123 => 50)));
        metrics.insert("metrics_2".into(), Snapshot::from_bitrates(hashmap!(123 => 100)));

        let frame = Frame::new(metrics);

        assert_eq!(frame.metrics("metrics_1"),
                   Some(&Snapshot::Bitrates(hashmap!(123 => Bitrate::new(50)))));

        assert_eq!(frame.metrics("metrics_2"),
                   Some(&Snapshot::Bitrates(hashmap!(123 => Bitrate::new(100)))));
    }

    #[test]
    fn test_should_disregard_pids_not_in_all_metrics() {
        // let metrics = vec![
        //     Snapshot::from_percents("metrics_1", hashset!(1 => 25., 2 => 50.)).unwrap(),
        //     Snapshot::from_percents("metrics_2", hashset!(1 => 25., 3 => 50.)).unwrap(),
        // ];

        // assert_eq!(Frame::new(metrics),
        //            Frame::new(vec![
        //                LabelledMetrics::from_percents()
        //            ]));
        //
        // assert_eq!(frame.metrics("metrics_1"),
        //            Some(&Metrics::Percents(vec![ProcessMetric {
        //                pid: 123,
        //                value: Percent::new(50.).unwrap(),
        //            }])));
        //
        // assert_eq!(frame.metrics("metrics_2"),
        //            Some(&Metrics::Percents(vec![ProcessMetric {
        //                pid: 123,
        //                value: Percent::new(100.).unwrap(),
        //            }])));
    }
}

pub struct ProbeDispatcher {
    last_frame: Option<Frame>,
    processes: HashSet<PID>,
    labelled_probes: HashMap<String, Box<dyn Probe>>,
}

impl ProbeDispatcher {
    pub fn new() -> Self {
        Self { last_frame: None, processes: HashSet::new(), labelled_probes: HashMap::new() }
    }

    pub fn add_probe<L>(&mut self, label: L, probe: Box<dyn Probe>) where L: Into<String> {
        self.labelled_probes.insert(label.into(), probe);
    }

    pub fn add_process(&mut self, pid: PID) {
        self.processes.insert(pid);
    }

    pub fn probe(&mut self) -> Result<(), Error> {
        let processes = &self.processes;

        let mut snapshots = self.labelled_probes.iter_mut()
            .map(|(label, probe)| {
                Ok((label.to_string(), probe.probe_processes(processes)?))
            })
            .collect::<Result<_, _>>()?;

        self.last_frame = Some(Frame::new(snapshots));

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
    use crate::probe::dispatch::{Frame, ProbeDispatcher, Snapshot};
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
        fn probe_processes(&mut self, pids: &HashSet<u32>) -> Result<Snapshot, Error> {
            Ok(Snapshot::Percents(pids.iter()
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
                   Some(&Snapshot::Percents(hashmap!())));
    }

    #[test]
    fn test_should_collect_one_frame_when_one_probe_added() {
        let mut dispatcher = ProbeDispatcher::new();

        dispatcher.add_probe("my-probe", Box::new(ProbeFake::new(50.)));
        dispatcher.add_process(123);
        dispatcher.probe().expect("Error while probing");

        let mut expected = HashMap::new();
        expected.insert("my-probe".into(), Snapshot::from_percents(hashmap!(123 => 50.)).unwrap());

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
        expected.insert("my-probe-1".into(), Snapshot::from_percents(hashmap!(123 => 50., 124 => 50.)).unwrap());
        expected.insert("my-probe-2".into(), Snapshot::from_percents(hashmap!(123 => 25., 124 => 25.)).unwrap());

        assert_eq!(frame, Frame::new(expected));
    }
}
