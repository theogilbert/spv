//! Running processes discovery

use log::warn;

use crate::core::Error;

/// On Linux 64 bits, the maximum value for a PID is 4194304, hence u32
pub type Pid = u32; // TODO add new type UPID (Unique PID) through the entire execution of spv

/// Basic metadata of a process (PID, command, etc...)
#[derive(Eq, PartialEq, Debug, Clone)]
pub struct ProcessMetadata {
    pid: Pid,
    command: String,
}

/// Describes a process
impl ProcessMetadata {
    /// Returns a new instance of a ProcessMetadata
    pub fn new<T>(pid: Pid, command: T) -> ProcessMetadata
        where T: Into<String> {
        ProcessMetadata { pid, command: command.into() }
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
}


/// Lists the running processes
pub struct ProcessView {
    scanner: Box<dyn ProcessScanner>,
}

impl ProcessView {
    pub fn new(scanner: Box<dyn ProcessScanner>) -> Self {
        Self { scanner }
    }

    pub fn processes(&self) -> Result<Vec<ProcessMetadata>, Error> {
        let pids = self.scanner.scan()?;

        Ok(pids.iter()
            .filter_map(|pid| {
                match self.scanner.fetch_metadata(*pid) {
                    Err(e) => {
                        warn!("Error fetching process metadata: {:?}", e);
                        None
                    }
                    Ok(pm) => Some(pm)
                }
            })
            .collect())
    }
}

/// Trait with methods to retrieve basic information about running processes
pub trait ProcessScanner {
    /// Returns a list containing the PIDs of all currently running processes
    fn scan(&self) -> Result<Vec<Pid>, Error>;

    /// Returns The ProcessMetadata of the currently running process with the given PID
    ///
    /// # Arguments
    ///
    /// * `pid`: The process identifier of the currently running process
    fn fetch_metadata(&self, pid: Pid) -> Result<ProcessMetadata, Error>;
}