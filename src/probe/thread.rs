use std::collections::HashSet;
use std::sync::mpsc::{Receiver, Sender};

use crate::probe::{Error, Probe, ProcessMetric};
use crate::process::PID;
use crate::values::{BitrateValue, PercentValue};

/// Contains messages that can be sent to or from a `ProbeThread`
pub enum ProbeInput {
    Kill(),
    AddProcess(PID),
    DelProcess(PID),
    Probe(),
}

pub enum ProbeOutput {
    NewFrame(ProbedFrame),
}

#[derive(PartialEq, Debug)]
pub enum ProbedFrame {
    PercentsFrame(Vec<ProcessMetric<PercentValue>>),
    BitratesFrame(Vec<ProcessMetric<BitrateValue>>),
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
                    if !self.handle_message(msg)? {
                        break;
                    }
                }
                Err(e) => Err(Error::MPSCError(e.to_string()))?
            }
        }

        Ok(())
    }

    // TODO Not a big fan of this whole "return false if msg is kill, else true"
    fn handle_message(&mut self, msg: ProbeInput) -> Result<bool, Error> {
        let mut ret_value = true;

        match msg {
            ProbeInput::Kill() => ret_value = false,
            ProbeInput::AddProcess(pid) => { self.monitored_pids.insert(pid); }
            ProbeInput::DelProcess(pid) => { self.monitored_pids.remove(&pid); }
            ProbeInput::Probe() => self.probe()?,
        };

        Ok(ret_value)
    }

    fn probe(&mut self) -> Result<(), Error> {
        let frame = self.probe.probe_frame(&self.monitored_pids)
            .map_err(|e| Error::ProbingError(e.to_string()))?;
        self.parent_tx.send(ProbeOutput::NewFrame(frame))
            .map_err(|e| Error::MPSCError(e.to_string()))
    }
}

#[cfg(test)]
mod test_probe_runner {
    use std::collections::HashSet;
    use std::sync::mpsc::channel;

    use crate::probe::{Error, Probe, ProcessMetric};
    use crate::probe::thread::{ProbedFrame, ProbeInput, ProbeOutput, ProbeRunner};
    use crate::process::PID;
    use crate::values::BitrateValue;

    #[derive(Eq, PartialEq, Debug)]
    enum ProbeFakeCall {
        Init(),
        Probe(PID),
    }

    struct ProbeFake {}

    impl ProbeFake {
        fn new() -> Self {
            ProbeFake {}
        }
    }

    impl Probe for ProbeFake {
        fn probe_frame(&mut self, pids: &HashSet<PID>) -> Result<ProbedFrame, Error> {
            Ok(ProbedFrame::BitratesFrame(pids.iter().map(|p| {
                ProcessMetric { pid: *p, value: BitrateValue::new(123) }
            }).collect()))
        }
    }

    fn launch_probe_runner_with_msg_sequence(call_sequence: Vec<ProbeInput>) -> ProbedFrame {
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

        if let Ok(ProbeOutput::NewFrame(probed_frame)) = main_rx.recv() {
            return probed_frame;
        } else {
            panic!("Not received new frame");
        }
    }

    #[test]
    fn test_should_return_empty_frame_when_no_pid() {
        let call_seq = vec![ProbeInput::Probe()];

        let probed_frame = launch_probe_runner_with_msg_sequence(call_seq);

        assert_eq!(probed_frame, ProbedFrame::BitratesFrame(vec![]));
    }

    #[test]
    fn test_should_return_2_elt_in_frame_when_2_pids_added() {
        let call_seq = vec![
            ProbeInput::AddProcess(123),
            ProbeInput::AddProcess(124),
            ProbeInput::Probe()
        ];

        let mut probed_frame = launch_probe_runner_with_msg_sequence(call_seq);

        if let ProbedFrame::BitratesFrame(mut metrics) = probed_frame {
            metrics.sort_by_key(|m| m.pid);
            assert_eq!(metrics,
                       vec![ProcessMetric { pid: 123, value: BitrateValue::new(123) },
                            ProcessMetric { pid: 124, value: BitrateValue::new(123) }]);
        } else {
            panic!("Not a bitrates frame");
        }
    }

    #[test]
    fn test_should_return_no_probed_frame_when_pid_added_and_removed() {
        let call_seq = vec![
            ProbeInput::AddProcess(123),
            ProbeInput::DelProcess(123),
            ProbeInput::Probe()
        ];

        let probed_frame = launch_probe_runner_with_msg_sequence(call_seq);

        assert_eq!(probed_frame, ProbedFrame::BitratesFrame(vec![]));
    }

    #[test]
    fn test_should_not_fail_when_deleting_inexistant_pid() {
        let call_seq = vec![
            ProbeInput::DelProcess(123),
            ProbeInput::Probe()
        ];

        let probed_frame = launch_probe_runner_with_msg_sequence(call_seq);

        assert_eq!(probed_frame, ProbedFrame::BitratesFrame(vec![]));
    }

    #[test]
    fn test_should_not_probe_process_twice_when_pid_added_twice() {
        let call_seq = vec![
            ProbeInput::AddProcess(123),
            ProbeInput::AddProcess(123),
            ProbeInput::Probe()
        ];

        let probed_frame = launch_probe_runner_with_msg_sequence(call_seq);

        let exp_metrics = vec![ProcessMetric { pid: 123, value: BitrateValue::new(123) }];
        assert_eq!(probed_frame, ProbedFrame::BitratesFrame(exp_metrics));
    }
}