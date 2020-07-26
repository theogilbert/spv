use std::collections::HashSet;
use std::sync::mpsc::{Receiver, Sender};

use crate::probe::{Error, Probe, ProcessMetric};
use crate::process::PID;
use crate::values::Value;

/// Contains messages that can be sent to or from a `ProbeThread`
pub enum ProbeInput {
    Kill(),
    AddProcess(PID),
    DelProcess(PID),
    Probe(),
}

pub enum ProbeOutput<T> where T: Value {
    NewFrame(ProbedFrame<T>),
}


/// Contains a list of `ProcessMetric`, one for each probed process
pub struct ProbedFrame<T> where T: Value {
    metrics: Vec<ProcessMetric<T>>,
}

pub struct ProbeRunner<T, P> where T: Value, P: Probe<ValueType=T> {
    monitored_pids: HashSet<PID>,
    // probe has to be dyn as we must be able to interact with probes in a polymorphous manner
    probe: P,

    parent_rx: Receiver<ProbeInput>,
    parent_tx: Sender<ProbeOutput<T>>,
}

impl<T, P> ProbeRunner<T, P> where T: Value, P: Probe<ValueType=T> {
    pub fn new(probe: P, rx: Receiver<ProbeInput>, tx: Sender<ProbeOutput<T>>) -> Self {
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
        let mut metrics = Vec::new();

        self.probe.init_iteration()?;

        for pid in self.monitored_pids.iter() {
            match self.probe.probe(*pid) {
                Ok(process_metric) => {
                    metrics.push(process_metric);
                }
                Err(_) => ()
            }
        };

        self.parent_tx.send(ProbeOutput::NewFrame(ProbedFrame { metrics }))
            .map_err(|e| Error::MPSCError(e.to_string()))
    }
}

#[cfg(test)]
mod test_probe_runner {
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

    struct ProbeFake {
        call_seq: Vec<ProbeFakeCall>,

    }

    impl ProbeFake {
        fn new() -> Self {
            ProbeFake { call_seq: Vec::new() }
        }
    }

    impl Probe for ProbeFake {
        type ValueType = BitrateValue;

        fn init_iteration(&mut self) -> Result<(), Error> {
            self.call_seq.push(ProbeFakeCall::Init());

            Ok(())
        }

        fn probe(&mut self, pid: u32) -> Result<ProcessMetric<BitrateValue>, Error> {
            for call in self.call_seq.iter().rev() {
                match call {
                    ProbeFakeCall::Init() => break,
                    ProbeFakeCall::Probe(pid_arg) => {
                        if *pid_arg == pid {
                            panic!("Probed called multiple times without init in-between")
                        }
                    }
                }
            }
            if !self.call_seq.iter().any(|c| *c == ProbeFakeCall::Init()) {
                panic!("Init was never called");
            }

            self.call_seq.push(ProbeFakeCall::Probe(pid));

            Ok(ProcessMetric { pid, value: BitrateValue::new(10) })
        }
    }

    fn launch_probe_runner_with_msg_sequence(call_sequence: Vec<ProbeInput>) -> ProbedFrame<BitrateValue> {
        let probe_box = ProbeFake::new();
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

        assert!(probed_frame.metrics.is_empty());
    }

    #[test]
    fn test_should_return_2_elt_in_frame_when_2_pids_added() {
        let call_seq = vec![
            ProbeInput::AddProcess(123),
            ProbeInput::AddProcess(124),
            ProbeInput::Probe()
        ];

        let mut probed_frame = launch_probe_runner_with_msg_sequence(call_seq);
        probed_frame.metrics.sort_by_key(|m| m.pid);

        assert_eq!(probed_frame.metrics,
                   vec![ProcessMetric { pid: 123, value: BitrateValue::new(10) },
                        ProcessMetric { pid: 124, value: BitrateValue::new(10) }]);
    }

    #[test]
    fn test_should_return_no_probed_frame_when_pid_added_and_removed() {
        let call_seq = vec![
            ProbeInput::AddProcess(123),
            ProbeInput::DelProcess(123),
            ProbeInput::Probe()
        ];

        let probed_frame = launch_probe_runner_with_msg_sequence(call_seq);

        assert!(probed_frame.metrics.is_empty())
    }

    #[test]
    fn test_should_not_fail_when_deleting_inexistant_pid() {
        let call_seq = vec![
            ProbeInput::DelProcess(123),
            ProbeInput::Probe()
        ];

        let probed_frame = launch_probe_runner_with_msg_sequence(call_seq);

        assert!(probed_frame.metrics.is_empty());
    }

    #[test]
    fn test_should_not_probe_process_twice_when_pid_added_twice() {
        let call_seq = vec![
            ProbeInput::AddProcess(123),
            ProbeInput::AddProcess(123),
            ProbeInput::Probe()
        ];

        let probed_frame = launch_probe_runner_with_msg_sequence(call_seq);

        assert_eq!(probed_frame.metrics,
                   vec![ProcessMetric { pid: 123, value: BitrateValue::new(10) }]);
    }
}