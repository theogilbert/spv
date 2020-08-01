use std::collections::{HashMap, HashSet};

use crate::probe::{Error, Probe, ProcessMetric};
use crate::process::PID;
use crate::values::{Bitrate, Percent};

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
    use crate::values::{Bitrate, Percent};

    fn get_example_metrics() -> Vec<LabelledMetrics> {
        vec![
            LabelledMetrics {
                label: "metrics_1".to_string(),
                metrics: Metrics::Percents(vec![ProcessMetric { pid: 123, value: Percent::new(50.).unwrap() }]),
            },
            LabelledMetrics {
                label: "metrics_2".to_string(),
                metrics: Metrics::Bitrates(vec![ProcessMetric { pid: 123, value: Bitrate::new(50) }]),
            },
        ]
    }

    #[test]
    fn test_should_return_correct_labels() {
        assert_eq!(Frame::new(get_example_metrics()).labels(),
                   vec!["metrics_1", "metrics_2"]);
    }

    #[test]
    fn test_should_return_correct_values() {
        assert_eq!(Frame::new(get_example_metrics()).metrics("metrics_1"),
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
    use crate::values::Percent;

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
                       LabelledMetrics {
                           label: "my-probe".to_string(),
                           metrics: Metrics::Percents(vec![
                               ProcessMetric::new(123,
                                                  Percent::new(50.).unwrap())
                           ]),
                       }
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
                       LabelledMetrics {
                           label: "my-probe-1".to_string(),
                           metrics: Metrics::Percents(vec![
                               ProcessMetric::new(123,
                                                  Percent::new(50.).unwrap()),
                               ProcessMetric::new(124,
                                                  Percent::new(50.).unwrap())
                           ]),
                       },
                       LabelledMetrics {
                           label: "my-probe-2".to_string(),
                           metrics: Metrics::Percents(vec![
                               ProcessMetric::new(123,
                                                  Percent::new(25.).unwrap()),
                               ProcessMetric::new(124,
                                                  Percent::new(25.).unwrap())
                           ]),
                       },
                   ]));
    }
}
