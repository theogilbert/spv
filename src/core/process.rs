//! Process discovery utilities

use std::collections::HashMap;
use std::fmt::{Display, Formatter};

use log::warn;

use crate::core::iteration::{Iteration, Span};
use crate::core::Error;

/// Represents the unique ID of a running process
///
/// On Linux 64 bits, the maximum value for a PID is 4194304, hence u32
pub type Pid = u32; // TODO add new type UPID (Unique PID) through the entire execution of spv, as PIDs might rollover

/// Basic metadata of a process (PID, command, etc...)
#[derive(Eq, PartialEq, Debug, Clone)]
pub struct ProcessMetadata {
    pid: Pid,
    command: String,
    status: Status,
    running_span: Span,
}

#[derive(Eq, PartialEq, Debug, Copy, Clone)]
pub enum Status {
    RUNNING,
    DEAD,
}

impl Display for Status {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Status::RUNNING => f.write_str("running"),
            Status::DEAD => f.write_str("dead"),
        }
    }
}

/// Describes a process
impl ProcessMetadata {
    /// Returns a new instance of a ProcessMetadata
    pub fn new<T>(pid: Pid, spawn_iteration: Iteration, command: T) -> Self
    where
        T: Into<String>,
    {
        ProcessMetadata {
            pid,
            command: command.into(),
            status: Status::RUNNING,
            running_span: Span::from_begin(spawn_iteration),
        }
    }

    /// Returns the process identifier assigned to the process by the OS
    ///
    /// Whilst a PID can be recycled, two running processes can not share the same PID
    pub fn pid(&self) -> Pid {
        self.pid
    }

    /// Returns the command used to execute the given process
    ///
    /// This method does not return the arguments passed to the command
    pub fn command(&self) -> &str {
        self.command.as_str()
    }

    /// Returns the status of the process, indicating if it is still running or not
    pub fn status(&self) -> Status {
        self.status
    }

    /// Marks a process as dead
    pub fn mark_dead(&mut self) {
        self.status = Status::DEAD;
    }

    /// Indicates the span of iterations during which the process is running
    pub fn running_span(&self) -> &Span {
        &self.running_span
    }

    /// Updates the span of the process, indicating that it is still running at the given iteration
    ///
    /// # Arguments
    /// * `current_iteration`: The iteration at which the process is still running
    fn mark_alive_at_iteration(&mut self, current_iteration: Iteration) {
        self.running_span.set_end_and_resize(current_iteration);
    }
}

#[cfg(test)]
mod test_process_metadata {
    use crate::core::process::{ProcessMetadata, Status};

    #[test]
    fn test_pid_should_be_pm_pid() {
        assert_eq!(ProcessMetadata::new(123, 0, "command").pid(), 123);
    }

    #[test]
    fn test_command_should_be_pm_command() {
        assert_eq!(ProcessMetadata::new(123, 0, "command").command(), "command");
    }

    #[test]
    fn test_status_should_be_running_by_default() {
        assert_eq!(ProcessMetadata::new(123, 0, "command").status(), Status::RUNNING);
    }

    #[test]
    fn test_status_should_be_dead_once_marked_as_dead() {
        let mut pm = ProcessMetadata::new(123, 0, "command");
        pm.mark_dead();
        assert_eq!(pm.status(), Status::DEAD);
    }

    #[test]
    fn test_span_should_only_include_spawn_iteration_by_default() {
        let pm = ProcessMetadata::new(456, 42, "command");
        let running_span = pm.running_span();

        assert_eq!(running_span.begin(), 42);
        assert_eq!(running_span.end(), 42);
        assert_eq!(running_span.size(), 1);
    }

    #[test]
    fn test_span_should_increase_when_process_marked_alive() {
        let mut pm = ProcessMetadata::new(456, 50, "command");
        pm.mark_alive_at_iteration(59);
        let running_span = pm.running_span();

        assert_eq!(running_span.begin(), 50);
        assert_eq!(running_span.end(), 59);
        assert_eq!(running_span.size(), 10);
    }
}

/// Collects the running processes
pub struct ProcessCollector {
    scanner: Box<dyn ProcessScanner>,
    registered_processes: HashMap<Pid, ProcessMetadata>,
}

impl ProcessCollector {
    pub fn new(scanner: Box<dyn ProcessScanner>) -> Self {
        Self {
            scanner,
            registered_processes: HashMap::new(),
        }
    }

    /// Returns the list of all processes, regardless of their status (running or not)
    pub fn processes(&self) -> Vec<ProcessMetadata> {
        self.registered_processes.values().cloned().collect()
    }

    /// Returns the list of processes that were still running as of the last collection
    pub fn running_processes(&self) -> Vec<ProcessMetadata> {
        self.registered_processes
            .values()
            .filter(|pm| pm.status == Status::RUNNING)
            .cloned()
            .collect()
    }

    /// Returns the list of pids of the processes that were still running as of the last collection
    pub fn running_pids(&self) -> Vec<Pid> {
        self.registered_processes
            .values()
            .filter(|pm| pm.status == Status::RUNNING)
            .map(|pm| pm.pid())
            .collect()
    }

    /// Scans and retrieves information about running processes
    pub fn collect_processes(&mut self, current_iteration: Iteration) -> Result<(), Error> {
        let running_pids = self.scanner.scan()?;

        for pm in self.parse_new_processes(&running_pids, current_iteration) {
            self.registered_processes.insert(pm.pid(), pm);
        }

        self.update_processes_statuses(&running_pids, current_iteration);

        Ok(())
    }

    fn parse_new_processes(&self, running_pids: &[Pid], current_iteration: Iteration) -> Vec<ProcessMetadata> {
        running_pids
            .iter()
            .filter(|p| !self.registered_processes.contains_key(*p))
            .filter_map(|pid| match self.scanner.fetch_metadata(*pid, current_iteration) {
                Err(e) => {
                    warn!("Error fetching process metadata: {:?}", e);
                    None
                }
                Ok(pm) => Some(pm),
            })
            .collect()
    }

    /// Mark new dead processes as dead, and update the running span of processes still running
    fn update_processes_statuses(&mut self, running_pids: &[Pid], current_iteration: Iteration) {
        self.registered_processes
            .values_mut()
            .filter(|pm| pm.status() == Status::RUNNING) // No need to mark dead processes
            .for_each(|pm| {
                if !running_pids.contains(&pm.pid()) {
                    pm.mark_dead();
                } else {
                    pm.mark_alive_at_iteration(current_iteration);
                }
            });
    }
}

#[cfg(test)]
mod test_process_collector {
    use crate::core::iteration::{Iteration, Span};
    use crate::core::process::{Pid, ProcessCollector, ProcessMetadata, ProcessScanner, Status};
    use crate::core::Error;
    use crate::core::Error::InvalidPID;

    struct ScannerStub {
        scan_count: usize,
        scanned_pids: Vec<Vec<Pid>>,
        failing_processes: Vec<Pid>,
    }

    impl ScannerStub {
        fn new(scanned_pids: Vec<Pid>) -> Self {
            Self::new_with_failing_processes(scanned_pids, vec![])
        }

        fn new_with_failing_processes(scanned_pids: Vec<Pid>, failing_processes: Vec<Pid>) -> Self {
            ScannerStub {
                scan_count: 0,
                scanned_pids: vec![scanned_pids],
                failing_processes,
            }
        }

        fn set_next_scanned_pids(&mut self, scanned_pids: Vec<Pid>) {
            self.scanned_pids.push(scanned_pids);
        }
    }

    impl ProcessScanner for ScannerStub {
        fn scan(&mut self) -> Result<Vec<Pid>, Error> {
            self.scan_count += 1;
            Ok(self.scanned_pids[self.scan_count - 1].clone())
        }

        fn fetch_metadata(&self, pid: Pid, spawn_iteration: Iteration) -> Result<ProcessMetadata, Error> {
            if self.failing_processes.contains(&pid) {
                Err(InvalidPID(pid))
            } else {
                Ok(ProcessMetadata::new(pid, spawn_iteration, "command"))
            }
        }
    }

    fn build_process_collector(scanned_pids: Vec<Pid>) -> ProcessCollector {
        let boxed_scanner = Box::new(ScannerStub::new(scanned_pids));
        ProcessCollector::new(boxed_scanner)
    }

    fn build_collector_which_fails(scanned_pids: Vec<Pid>, failing_processes: Vec<Pid>) -> ProcessCollector {
        let boxed_scanner = Box::new(ScannerStub::new_with_failing_processes(scanned_pids, failing_processes));
        ProcessCollector::new(boxed_scanner)
    }

    fn build_collector_with_sequence(mut pids_sequence: Vec<Vec<Pid>>) -> ProcessCollector {
        pids_sequence.reverse();

        let mut boxed_scanner = Box::new(ScannerStub::new(pids_sequence.pop().unwrap()));
        pids_sequence
            .into_iter()
            .for_each(|pids| boxed_scanner.set_next_scanned_pids(pids));

        ProcessCollector::new(boxed_scanner)
    }

    fn build_collector_with_sequence_and_collect(pids_sequence: Vec<Vec<Pid>>) -> ProcessCollector {
        let sequence_count = pids_sequence.len();
        let mut collector = build_collector_with_sequence(pids_sequence);
        for iteration in 0..sequence_count {
            collector
                .collect_processes(iteration)
                .expect("Could not collect processes");
        }

        collector
    }

    #[test]
    fn test_should_collect_no_process_when_no_pid_scanned() {
        let mut collector = build_process_collector(vec![]);
        collector.collect_processes(1).unwrap();
        let processes = collector.running_processes();

        assert_eq!(processes, vec![]);
    }

    #[test]
    fn test_should_collect_processes_when_pids_are_scanned() {
        let scanned_pids = vec![1, 2, 3];
        let mut collector = build_process_collector(scanned_pids.clone());
        collector.collect_processes(1).unwrap();
        let processes = collector.running_processes();

        assert_eq!(processes.len(), 3);

        let mut processes_pids = processes.iter().map(|pm| pm.pid).collect::<Vec<Pid>>();
        processes_pids.sort();

        assert_eq!(processes_pids, scanned_pids);
    }

    #[test]
    fn test_should_ignore_processes_for_which_scanning_fails() {
        let mut collector = build_collector_which_fails(vec![1, 2, 3], vec![2]);
        collector.collect_processes(1).unwrap();
        let processes = collector.running_processes();

        let processes_pids = processes.iter().map(|pm| pm.pid).collect::<Vec<Pid>>();

        assert_eq!(processes_pids.len(), 2);
        assert!(!processes_pids.contains(&2))
    }

    #[test]
    fn test_should_set_status_of_running_processes_to_running() {
        let mut collector = build_process_collector(vec![1]);

        collector.collect_processes(1).unwrap();
        let processes = collector.running_processes();

        assert_eq!(processes[0].status, Status::RUNNING);
    }

    #[test]
    fn test_should_correctly_marked_dead_process() {
        let pids_sequence = vec![
            vec![3], // Iteration=0 -> Process pid=3 is collected
            vec![],  // Iteration=1 -> Process pid=3 is not running anymore
        ];
        let collector = build_collector_with_sequence_and_collect(pids_sequence);

        let dead_process = &collector.processes()[0];

        assert_eq!(dead_process.status(), Status::DEAD);
    }

    #[test]
    fn test_should_not_classify_dead_processes_as_running() {
        let pids_sequence = vec![
            vec![3], // Process pid=3 is collected
            vec![],  // Process pid=3 is not running anymore
        ];
        let collector = build_collector_with_sequence_and_collect(pids_sequence);

        assert_eq!(collector.running_processes().len(), 0);
        assert_eq!(collector.processes().len(), 1);
    }

    #[test]
    fn test_running_processes_should_only_return_running_processes() {
        let pids_sequence = vec![
            vec![1, 2, 3], // Processes 1, 2 and 3 are running
            vec![1],       // Only process 1 is still running
        ];
        let collector = build_collector_with_sequence_and_collect(pids_sequence);

        let running_processes = collector.running_processes();
        assert_eq!(running_processes.len(), 1);
        assert_eq!(running_processes[0].pid(), 1);
    }

    #[test]
    fn test_running_pids_should_only_return_running_processes() {
        let pids_sequence = vec![
            vec![1, 2, 3], // Processes 1, 2 and 3 are running
            vec![1],       // Only process 1 is still running
        ];
        let collector = build_collector_with_sequence_and_collect(pids_sequence);

        assert_eq!(collector.running_pids(), vec![1]);
    }

    #[test]
    fn test_span_of_running_processes_should_be_updated_when_collected() {
        let mut collector = build_collector_with_sequence(vec![vec![1], vec![1]]);

        collector.collect_processes(0).unwrap();
        let running_process = &collector.running_processes()[0];
        assert_eq!(running_process.running_span(), &Span::from_end_and_size(0, 1));

        collector.collect_processes(1).unwrap();
        let running_process = &collector.running_processes()[0];
        assert_eq!(running_process.running_span(), &Span::from_end_and_size(1, 2));
    }
}

/// Trait with methods to retrieve basic information about running processes
pub trait ProcessScanner {
    /// Returns a list containing the PIDs of all currently running processes
    fn scan(&mut self) -> Result<Vec<Pid>, Error>;

    /// Returns The ProcessMetadata of the currently running process with the given PID
    ///
    /// # Arguments
    ///
    /// * `pid`: The process identifier of the currently running process
    /// * `spawn_iteration`: The process identifier of the currently running process
    fn fetch_metadata(&self, pid: Pid, spawn_iteration: Iteration) -> Result<ProcessMetadata, Error>;
}
