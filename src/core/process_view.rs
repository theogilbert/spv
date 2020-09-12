use std::collections::HashSet;

use crate::core::Error;
use crate::probes::process::ProcfsScanner;

/// On Linux 64 bits, the maximum value for a PID is 4194304, hence u32
pub type PID = u32;

/// Basic metadata of a process (PID, command, etc...)
#[derive(Eq, PartialEq, Debug, Clone)]
pub struct ProcessMetadata {
    pid: PID,
    command: String,
}

/// Describes a process
impl ProcessMetadata {
    /// Returns a new instance of a ProcessMetadata
    pub fn new<T>(pid: PID, command: T) -> ProcessMetadata
        where T: Into<String> {
        ProcessMetadata { pid, command: command.into() }
    }

    /// Returns the process identifier assigned to the process by the OS
    ///
    /// Whilst a PID can be recycled, two running processes can not share the same PID
    pub fn pid(&self) -> PID {
        self.pid
    }

    /// Returns the command used to execute the given process
    ///
    /// This method does not return the arguments passed to the command
    pub fn command(&self) -> &str {
        self.command.as_str()
    }
}


/// Lists the running processes
pub struct ProcessView {
    scanner: Box<dyn ProcessScanner>
}

impl Default for ProcessView {
    fn default() -> Self {
        Self {
            scanner: Box::new(ProcfsScanner::new())
        }
    }
}

impl ProcessView {
    pub fn processes(&self) -> Result<Vec<ProcessMetadata>, Error> {
        let pids = self.scanner.scan()?;

        pids.iter()
            .filter_map(|pid| Some(self.scanner.fetch_metadata(*pid)))
            .collect()
    }
}

/// Trait with methods to retrieve basic information about running processes
pub trait ProcessScanner {
    /// Returns a list containing the PIDs of all currently running processes
    fn scan(&self) -> Result<HashSet<PID>, Error>;

    /// Returns The ProcessMetadata of the currently running process with the given PID
    ///
    /// # Arguments
    ///
    /// * `pid`: The process identifier of the currently running process
    fn fetch_metadata(&self, pid: PID) -> Result<ProcessMetadata, Error>;
}