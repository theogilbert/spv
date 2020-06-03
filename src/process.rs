use std::fs::{DirEntry, File, read_dir};
use std::io::Read;
use std::path::PathBuf;

use crate::spv::{Error, Result};

/// On Linux 64 bits, the theoretical maximum value for a PID is 4194304
type PID = u32;

/// Trait with methods to retrieve basic information about running processes
pub trait ProcessScanner {
    /// Returns a list containing the PIDs of all currently running processes
    fn scan_processes(&self) -> Result<Vec<PID>>;

    /// Returns The ProcessMetadata of the currently running process with the given PID
    ///
    /// # Arguments
    ///
    /// * pid: The process identifier of the currently running process
    fn process_metadata(&self, pid: PID) -> Result<ProcessMetadata>;
}

/// Basic metadata of a process (PID, command, etc...)
#[derive(Eq, PartialEq, Debug)]
pub struct ProcessMetadata {
    pid: PID,
    command: String,
}

/// Describes a process
impl ProcessMetadata {
    /// Returns the process identifier assigned to the process by the OS
    ///
    /// At any given time in the lifetime of a running process, this PID should be unique
    pub fn pid(&self) -> PID {
        self.pid
    }

    /// Returns the command used to execute the given process
    ///
    /// This method does not return the arguments sent to the command
    pub fn command(&self) -> &str {
        self.command.as_str()
    }
}


/// Implementation of ProcessScanner that uses the `/proc` Linux virtual directory as source
pub struct ProcProcessScanner {
    proc_dir: PathBuf
}

impl ProcProcessScanner {
    /// Creates a new ProcProcessScanner instance
    ///
    pub fn new() -> ProcProcessScanner {
        ProcProcessScanner { proc_dir: PathBuf::from("/proc") }
    }


    /// Parses a PID from a directory name, if it represents an unsigned integer
    ///
    /// # Arguments
    ///
    /// * `dir_name` - An optional string slice that holds the name of a directory
    ///
    fn pid_from_proc_dir(dir_name: Option<&str>) -> Result<PID> {
        let pid_ret = match dir_name {
            Some(dir_name) => dir_name.parse::<PID>(),
            None => Err(Error::InvalidPidDirName)?
        };

        pid_ret.or_else(|_| Err(Error::InvalidPidDirName))
    }
}

impl ProcessScanner for ProcProcessScanner {
    /// Scan all running processes on a Linux host by scanning the content of /proc directory
    /// Returns the PIDs of currently running processes
    fn scan_processes(&self) -> Result<Vec<PID>> {
        let dir_iter = read_dir(self.proc_dir.as_path())
            .or_else(|e| Err(Error::ProcessScanningFailure(e.to_string())))?;

        let pids = dir_iter
            // only retrieve dir entry which are not err
            .filter_map(|r| r.ok())
            // only retrieve directories
            .filter(|de| {
                de.file_type().is_ok() && de.file_type().unwrap().is_dir()
            })
            // retrieve Result<PID> from dir name
            .map(|de: DirEntry| Self::pid_from_proc_dir(de.file_name().to_str()))
            // Discard all dir names which could not be converted to PID
            .filter_map(|pid_ret| pid_ret.ok())
            .collect();


        Ok(pids)
    }

    /// Creates a ProcessMetadata instance given the process' PID
    fn process_metadata(&self, pid: PID) -> Result<ProcessMetadata> {
        let mut command = String::new();
        let comm_file_path = self.proc_dir
            .join(pid.to_string())
            .join("comm");

        let mut file = File::open(comm_file_path)
            .or_else(|io_err| Err(Error::ProcessParsingError(io_err.to_string())))?;

        file.read_to_string(&mut command)
            .or_else(|io_err| Err(Error::ProcessParsingError(io_err.to_string())))?;

        Ok(ProcessMetadata { pid, command })
    }
}

#[cfg(test)]
mod test_pid_from_proc_dir {
    use super::*;

    #[test]
    fn test_pid_from_valid_proc_dir_name() {
        let valid_pid = ProcProcessScanner::pid_from_proc_dir(Some("123"));

        assert_eq!(valid_pid, Ok(123));
    }

    #[test]
    fn test_pid_from_invalid_proc_dir_name() {
        let invalid_pid = ProcProcessScanner::pid_from_proc_dir(Some("abc"));

        assert_eq!(invalid_pid, Err(Error::InvalidPidDirName));
    }

    #[test]
    fn test_pid_from_no_proc_dir_name() {
        let invalid_pid = ProcProcessScanner::pid_from_proc_dir(None);

        assert_eq!(invalid_pid, Err(Error::InvalidPidDirName));
    }
}

#[cfg(target_os = "linux")]
#[cfg(test)]
mod test_pid_scanner {
    use std::fs;
    use std::io::Write;
    use std::os::unix::fs::PermissionsExt;
    use std::path::Path;

    use tempfile::{NamedTempFile, tempdir};

    use super::*;

    fn create_tempdir<T: Into<PathBuf>>(name: &str, dir: T) -> std::io::Result<()> {
        fs::create_dir(dir.into().join(name))
    }

    fn create_tempfile<T: Into<PathBuf>>(name: &str, dir: T) -> std::io::Result<fs::File> {
        let fp = dir.into().join(name);

        match NamedTempFile::new() {
            Ok(ntf) => Ok(ntf.persist(fp).expect("Could not persist file")),
            Err(e) => Err(e)
        }
    }

    fn set_dir_permissions(path: &Path, mode: u32) -> std::io::Result<()> {
        let mut perms = fs::metadata(path)?
            .permissions();

        perms.set_mode(mode);
        fs::set_permissions(path, perms)
    }

    #[test]
    fn test_scan_process() {
        // given we have a fake /proc dir with the following dirs
        // 123 456 abc 1ec 1.2
        // And the following files
        // 987 46a
        let test_proc_dir = tempdir().expect("Could not create tmp dir");

        let proc_subdirs = vec![
            create_tempdir("123", test_proc_dir.path()),
            create_tempdir("456", test_proc_dir.path()),
            create_tempdir("abc", test_proc_dir.path()),
            create_tempdir("1ec", test_proc_dir.path()),
            create_tempdir("1.2", test_proc_dir.path()),
        ];
        let proc_subfiles = vec![
            create_tempfile("987", test_proc_dir.path()),
            create_tempfile("46a", test_proc_dir.path())
        ];

        if proc_subdirs.iter().any(|i| i.is_err()) || proc_subfiles.iter().any(|i| i.is_err()) {
            panic!("Could not create all temp dir/files: {:?} / {:?}", proc_subdirs, proc_subfiles);
        }

        let proc_scanner = ProcProcessScanner {
            proc_dir: test_proc_dir.path().to_path_buf()
        };

        // when we scan processes
        let mut pids = proc_scanner.scan_processes()
            .expect("Could not scan processes");
        pids.sort();

        // The PIDs are only those represented by a dir with an integer name
        assert_eq!(vec![123, 456], pids);
    }

    #[test]
    fn test_scan_process_without_permissions() {
        // Given we do not have read access to test /proc dir
        let test_proc_dir = tempdir().expect("Could not create tmp dir");
        set_dir_permissions(test_proc_dir.path(), 0o000).expect("Could not set dir permissions");

        let proc_scanner = ProcProcessScanner {
            proc_dir: test_proc_dir.path().to_path_buf()
        };

        // when we scan processes
        let pids = proc_scanner.scan_processes();

        println!("Scanning result: {:?}", pids);

        // reset permission to allow dir removal
        set_dir_permissions(test_proc_dir.path(), 0o755).expect("Could not set dir permissions");

        assert!(pids.is_err());
    }

    #[test]
    fn test_process_metadata() {
        let test_proc_dir = tempdir().expect("Could not create tmp dir");

        create_tempdir("123", test_proc_dir.path())
            .expect("Could not create process dir");

        let mut comm_file = create_tempfile("comm", test_proc_dir.path().join("123").as_os_str())
            .expect("Could not create comm file");

        comm_file.write(b"test_cmd")
            .expect("Could not write to comm file"); // The process 123's command is test_cmd

        let proc_scanner = ProcProcessScanner {
            proc_dir: test_proc_dir.path().to_path_buf()
        };

        let process_metadata = proc_scanner.process_metadata(123)
            .expect("Could not get processes metadata");

        assert_eq!(process_metadata, ProcessMetadata {
            pid: 123,
            command: String::from("test_cmd"),
        });
    }
}
