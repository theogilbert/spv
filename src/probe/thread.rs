use std::collections::HashSet;
use std::sync::mpsc::{Receiver, Sender};

use crate::probe::{Error, Probe, ProcessMetric};
use crate::process::PID;
use crate::values::{Bitrate, Percent};

/// Contains messages that can be sent to or from a `ProbeThread`
pub enum ProbeInput {
    Kill(),
    AddProcess(PID),
    DelProcess(PID),
    Probe(),
}

pub enum ProbeOutput {
    NewMetrics(Metrics),
}

#[derive(PartialEq, Debug)]
pub enum Metrics {
    Percents(Vec<ProcessMetric<Percent>>),
    Bitrates(Vec<ProcessMetric<Bitrate>>),
}

pub struct ProbeRunner {
    monitored_pids: HashSet<PID>,
    // probe has to be dyn as we must be able to interact with probes in a polymorphous manner
    probe: Box<dyn Probe>,

    parent_rx: Receiver<ProbeInput>,
    parent_tx: Sender<ProbeOutput>,
}

impl ProbeRunner {
    pub fn new(probe: Box<dyn Probe>,
               rx: Receiver<ProbeInput>,
               tx: Sender<ProbeOutput>) -> Self {
        ProbeRunner {
            monitored_pids: HashSet::new(),
            probe,
            parent_rx: rx,
            parent_tx: tx,
        }
    }

    pub fn run(&mut self) -> Result<(), Error> {
        loop {
            match self.parent_rx.recv() {
                Ok(msg) => {
                    match self.handle_message(msg) {
                        Ok(()) => (),
                        Err(Error::ThreadKilledError) => break,
                        Err(e) => Err(e)?
                    }
                }
                Err(e) => Err(Error::MPSCError(e.to_string()))?
            }
        }

        Ok(())
    }

    fn handle_message(&mut self, msg: ProbeInput) -> Result<(), Error> {
        match msg {
            ProbeInput::Kill() => Err(Error::ThreadKilledError)?,
            ProbeInput::AddProcess(pid) => { self.monitored_pids.insert(pid); }
            ProbeInput::DelProcess(pid) => { self.monitored_pids.remove(&pid); }
            ProbeInput::Probe() => self.probe()?,
        };

        Ok(())
    }

    fn probe(&mut self) -> Result<(), Error> {
        let metrics = self.probe.probe_processes(&self.monitored_pids)
            .map_err(|e| Error::ProbingError(e.to_string()))?;
        self.parent_tx.send(ProbeOutput::NewMetrics(metrics))
            .map_err(|e| Error::MPSCError(e.to_string()))
    }
}

#[cfg(test)]
mod test_probe_runner {
    use std::collections::HashSet;
    use std::sync::mpsc::channel;

    use crate::probe::{Error, Probe, ProcessMetric};
    use crate::probe::thread::{Metrics, ProbeInput, ProbeOutput, ProbeRunner};
    use crate::process::PID;
    use crate::values::Bitrate;

    struct ProbeFake {}

    impl ProbeFake {
        fn new() -> Self {
            ProbeFake {}
        }
    }

    impl Probe for ProbeFake {
        fn probe_processes(&mut self, pids: &HashSet<PID>) -> Result<Metrics, Error> {
            Ok(Metrics::Bitrates(pids.iter().map(|p| {
                ProcessMetric { pid: *p, value: Bitrate::new(123) }
            }).collect()))
        }
    }

    fn launch_probe_runner_with_msg_sequence(call_sequence: Vec<ProbeInput>) -> Metrics {
        let probe = ProbeFake::new();
        let probe_box = Box::new(probe);

        let (main_tx, probes_rx) = channel();
        let (probes_tx, main_rx) = channel();

        for msg in call_sequence {
            main_tx.send(msg).unwrap();
        }
        main_tx.send(ProbeInput::Kill()).unwrap();

        let mut runner = ProbeRunner::new(probe_box, probes_rx, probes_tx);
        runner.run().expect("Error while running runner");

        if let Ok(ProbeOutput::NewMetrics(metrics)) = main_rx.recv() {
            return metrics;
        } else {
            panic!("Not received new metrics");
        }
    }

    #[test]
    fn test_should_return_no_metrics_when_no_pid() {
        let call_seq = vec![ProbeInput::Probe()];

        let metrics = launch_probe_runner_with_msg_sequence(call_seq);

        assert_eq!(metrics, Metrics::Bitrates(vec![]));
    }

    #[test]
    fn test_should_return_2_metrics_when_2_pids_added() {
        let call_seq = vec![
            ProbeInput::AddProcess(123),
            ProbeInput::AddProcess(124),
            ProbeInput::Probe()
        ];

        let metrics = launch_probe_runner_with_msg_sequence(call_seq);

        if let Metrics::Bitrates(mut pms) = metrics {
            pms.sort_by_key(|m| m.pid);
            assert_eq!(pms,
                       vec![ProcessMetric { pid: 123, value: Bitrate::new(123) },
                            ProcessMetric { pid: 124, value: Bitrate::new(123) }]);
        } else {
            panic!("Did not receive bitrate metrics");
        }
    }

    #[test]
    fn test_should_return_no_metrics_when_pid_added_and_removed() {
        let call_seq = vec![
            ProbeInput::AddProcess(123),
            ProbeInput::DelProcess(123),
            ProbeInput::Probe()
        ];

        let metrics = launch_probe_runner_with_msg_sequence(call_seq);

        assert_eq!(metrics, Metrics::Bitrates(vec![]));
    }

    #[test]
    fn test_should_not_fail_when_deleting_inexistant_pid() {
        let call_seq = vec![
            ProbeInput::DelProcess(123),
            ProbeInput::Probe()
        ];

        let metrics = launch_probe_runner_with_msg_sequence(call_seq);

        assert_eq!(metrics, Metrics::Bitrates(vec![]));
    }

    #[test]
    fn test_should_not_probe_process_twice_when_pid_added_twice() {
        let call_seq = vec![
            ProbeInput::AddProcess(123),
            ProbeInput::AddProcess(123),
            ProbeInput::Probe()
        ];

        let metrics = launch_probe_runner_with_msg_sequence(call_seq);

        let exp_metrics = vec![ProcessMetric { pid: 123, value: Bitrate::new(123) }];
        assert_eq!(metrics, Metrics::Bitrates(exp_metrics));
    }
}