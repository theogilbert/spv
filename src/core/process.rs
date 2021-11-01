//! Process discovery utilities

use std::collections::HashMap;

use log::warn;

use crate::core::Error;

/// Represents the unique ID of a running process
///
/// On Linux 64 bits, the maximum value for a PID is 4194304, hence u32
pub type Pid = u32; // TODO add new type UPID (Unique PID) through the entire execution of spv

/// Basic metadata of a process (PID, command, etc...)
#[derive(Eq, PartialEq, Debug, Clone)]
pub struct ProcessMetadata {
    pid: Pid,
    command: String,
    status: Status,
}

#[derive(Eq, PartialEq, Debug, Copy, Clone)]
pub enum Status {
    RUNNING,
    DEAD,
}

/// Describes a process
impl ProcessMetadata {
    /// Returns a new instance of a ProcessMetadata
    pub fn new<T>(pid: Pid, command: T) -> ProcessMetadata
    where
        T: Into<String>,
    {
        ProcessMetadata {
            pid,
            command: command.into(),
            status: Status::RUNNING,
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

    /// Marks a process as dead, indicating that it is not running anymore
    pub fn set_dead(&mut self) {
        self.status = Status::DEAD
    }
}

#[cfg(test)]
mod test_process_metadata {
    use crate::core::process::{ProcessMetadata, Status};

    #[test]
    fn test_pid_should_be_pm_pid() {
        assert_eq!(ProcessMetadata::new(123, "command").pid(), 123);
    }

    #[test]
    fn test_command_should_be_pm_command() {
        assert_eq!(ProcessMetadata::new(123, "command").command(), "command");
    }

    #[test]
    fn test_status_should_be_running_by_default() {
        assert_eq!(ProcessMetadata::new(123, "command").status(), Status::RUNNING);
    }

    #[test]
    fn test_status_should_be_dead_once_marked_as_dead() {
        let mut pm = ProcessMetadata::new(123, "command");
        pm.set_dead();
        assert_eq!(pm.status(), Status::RUNNING);
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

    /// Returns a Vec of [`ProcessMetadata`](struct.ProcessMetadata), each corresponding to process (running or dead)
    pub fn collect_processes(&mut self) -> Result<Vec<ProcessMetadata>, Error> {
        let running_pids = self.scanner.scan()?;

        self.mark_dead_processes(&running_pids);

        for pm in self.parse_new_processes(&running_pids) {
            self.registered_processes.insert(pm.pid(), pm);
        }

        Ok(self.registered_processes.values().map(|pm| pm.clone()).collect())
    }

    fn parse_new_processes(&self, running_pids: &[Pid]) -> Vec<ProcessMetadata> {
        running_pids
            .iter()
            .filter(|p| !self.registered_processes.contains_key(*p))
            .filter_map(|pid| match self.scanner.fetch_metadata(*pid) {
                Err(e) => {
                    warn!("Error fetching process metadata: {:?}", e);
                    None
                }
                Ok(pm) => Some(pm),
            })
            .collect()
    }

    fn mark_dead_processes(&mut self, running_pids: &Vec<Pid>) {
        self.registered_processes
            .values_mut()
            .filter(|pm| !running_pids.contains(&pm.pid()))
            .for_each(|pm| pm.set_dead());
    }
}

#[cfg(test)]
mod test_process_collector {
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

        fn fetch_metadata(&self, pid: Pid) -> Result<ProcessMetadata, Error> {
            if self.failing_processes.contains(&pid) {
                Err(InvalidPID(pid))
            } else {
                Ok(ProcessMetadata::new(pid, "command"))
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

    #[test]
    fn test_should_collect_no_process_when_no_pid_scanned() {
        let mut collector = build_process_collector(vec![]);

        let procesess = collector.collect_processes().unwrap();

        assert_eq!(procesess, vec![]);
    }

    #[test]
    fn test_should_collect_processes_when_pids_are_scanned() {
        let scanned_pids = vec![1, 2, 3];
        let mut collector = build_process_collector(scanned_pids.clone());
        let processes = collector.collect_processes().unwrap();

        assert_eq!(processes.len(), 3);

        let mut processes_pids = processes.iter().map(|pm| pm.pid).collect::<Vec<Pid>>();
        processes_pids.sort();

        assert_eq!(processes_pids, scanned_pids);
    }

    #[test]
    fn test_should_ignore_processes_for_which_scanning_fails() {
        let mut collector = build_collector_which_fails(vec![1, 2, 3], vec![2]);
        let processes = collector.collect_processes().unwrap();

        let processes_pids = processes.iter().map(|pm| pm.pid).collect::<Vec<Pid>>();

        assert_eq!(processes_pids.len(), 2);
        assert!(!processes_pids.contains(&2))
    }

    #[test]
    fn test_should_set_status_of_running_processes_to_running() {
        let mut collector = build_process_collector(vec![1]);

        let processes = collector.collect_processes().unwrap();

        assert_eq!(processes[0].status, Status::RUNNING);
    }

    #[test]
    fn test_should_mark_dead_process_as_dead() {
        let mut boxed_scanner = Box::new(ScannerStub::new(vec![3]));
        boxed_scanner.set_next_scanned_pids(vec![]);

        let mut collector = ProcessCollector::new(boxed_scanner);

        collector.collect_processes().unwrap(); // Process pid=3 is collected
        let processes = collector.collect_processes().unwrap(); // Process pid=3 is not running anymore

        assert_eq!(processes[0].status, Status::DEAD);
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
    fn fetch_metadata(&self, pid: Pid) -> Result<ProcessMetadata, Error>;
}
