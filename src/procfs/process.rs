//! Process discovery


use std::fs::{DirEntry, File, read_dir};
use std::io;
use std::io::Read;
use std::path::PathBuf;

use thiserror::Error;

use crate::core::Error as CoreError;
use crate::core::process_view::{PID, ProcessMetadata, ProcessScanner};

/// Errors internal to the process module
#[derive(Error, Debug)]
enum Error {
    #[error("Directory PID has invalid syntax: '{0:?}'")]
    NotProcessDir(String),
    #[error("Failed to read the content of directory '{0:?}'")]
    ProcessScanningFailure(PathBuf, #[source] io::Error),
    #[error("Error while parsing process directory '{0:?}'")]
    ProcessParsing(PathBuf, #[source] io::Error),
    #[error("PID is invalid: '{0:?}'")]
    InvalidPID(PID),
}

impl From<Error> for CoreError {
    fn from(e: Error) -> Self {
        CoreError::ScanProcessesError(e.into())
    }
}


/// Implementation of ProcessScanner that uses the `/proc` Linux virtual directory as source
#[derive(Default)]
pub struct ProcfsScanner {
    proc_dir: PathBuf
}

/// Scan running processes on a Linux host by scanning the content of /proc directory
impl ProcfsScanner {
    /// Returns a new ProcfsScanner instance
    pub fn new() -> ProcfsScanner {
        ProcfsScanner { proc_dir: PathBuf::from("/proc") }
    }


    /// Parses a PID from a directory name, if it represents an unsigned integer
    ///
    /// # Arguments
    /// * `dir_name` - An optional string slice that holds the name of a directory
    ///
    fn extract_pid_from_proc_dir(dir_name_opt: Option<&str>) -> std::result::Result<PID, Error> {
        match dir_name_opt {
            Some(dir_name) => dir_name.parse::<PID>()
                .map_err(|_| Error::NotProcessDir(dir_name.to_string())),
            None => Err(Error::NotProcessDir("".to_string()))
        }
    }
}


impl ProcessScanner for ProcfsScanner {
    /// Returns the PIDs of currently running processes
    fn scan(&self) -> std::result::Result<Vec<PID>, CoreError> {
        let path = self.proc_dir.as_path();

        let dir_iter = read_dir(path)
            .map_err(|e| Error::ProcessScanningFailure(path.into(), e))?;

        let pids = dir_iter
            // only retrieve dir entry which are not err
            .filter_map(|r| r.ok())
            // only retrieve directories
            .filter(|de| de.file_type().is_ok() && de.file_type().unwrap().is_dir())
            // retrieve Result<PID> from dir name
            .map(|de: DirEntry| Self::extract_pid_from_proc_dir(de.file_name().to_str()))
            // Discard all dir names which could not be converted to PID
            .filter_map(|pid_ret| pid_ret.ok())
            .collect();

        Ok(pids)
    }

    /// Fetch and returns the metadata of a process
    ///
    /// # Arguments
    ///  * `pid`: The identifier of the process for which to retrieve metadata
    fn fetch_metadata(&self, pid: PID) -> std::result::Result<ProcessMetadata, CoreError> {
        let mut command = String::new();
        let comm_file_path = self.proc_dir
            .join(pid.to_string())
            .join("comm");

        let mut file: File = File::open(comm_file_path.as_path())
            .map_err(|_| Error::InvalidPID(pid))?;

        file.read_to_string(&mut command)
            .map_err(|io_err| Error::ProcessParsing(comm_file_path, io_err))?;

        if command.ends_with('\n') {  // Remove trailing newline
            command.pop();
        }

        Ok(ProcessMetadata::new(pid, command))
    }
}

#[cfg(test)]
mod test_pid_from_proc_dir {
    use super::*;

    #[test]
    fn test_pid_from_valid_proc_dir_name() {
        let valid_pid = ProcfsScanner::extract_pid_from_proc_dir(Some("123"));

        assert!(matches!(valid_pid, Ok(123)));
    }

    #[test]
    fn test_pid_from_invalid_proc_dir_name() {
        let invalid_pid = ProcfsScanner::extract_pid_from_proc_dir(Some("abc"));

        let dir = String::from("abc");
        assert!(matches!(invalid_pid, Err(Error::NotProcessDir(dir))));
    }

    #[test]
    fn test_pid_from_no_proc_dir_name() {
        let invalid_pid = ProcfsScanner::extract_pid_from_proc_dir(None);

        let dir = String::new();
        assert!(matches!(invalid_pid, Err(Error::NotProcessDir(dir))));
    }
}

#[cfg(test)]
mod test_pid_scanner {
    use std::fs;
    use std::io::Write;
    use std::os::unix::fs::PermissionsExt;
    use std::path::Path;

    use tempfile::{NamedTempFile, tempdir};

    use crate::core::Error as CoreError;

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

        let proc_scanner = ProcfsScanner {
            proc_dir: test_proc_dir.path().to_path_buf()
        };

        // when we scan processes
        let pids = proc_scanner.scan()
            .expect("Could not scan processes");

        // The PIDs are only those represented by a dir with an integer name
        assert_eq!(vec![123, 456], pids);
    }

    #[test]
    fn test_scan_process_without_permissions() {
        // Given we do not have read access to test /proc dir
        let test_proc_dir = tempdir().expect("Could not create tmp dir");
        set_dir_permissions(test_proc_dir.path(), 0o000).expect("Could not set dir permissions");

        let proc_scanner = ProcfsScanner {
            proc_dir: test_proc_dir.path().to_path_buf()
        };

        // when we scan processes
        let pids = proc_scanner.scan();

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

        let proc_scanner = ProcfsScanner {
            proc_dir: test_proc_dir.path().to_path_buf()
        };

        let process_metadata = proc_scanner.fetch_metadata(123)
            .expect("Could not get processes metadata");

        assert_eq!(process_metadata,
                   ProcessMetadata::new(123, "test_cmd".to_string()));
    }

    #[test]
    fn test_process_metadata_with_newline() {
        let test_proc_dir = tempdir().expect("Could not create tmp dir");

        create_tempdir("123", test_proc_dir.path())
            .expect("Could not create process dir");

        let mut comm_file = create_tempfile("comm", test_proc_dir.path().join("123").as_os_str())
            .expect("Could not create comm file");

        comm_file.write(b"test_cmd\n")
            .expect("Could not write to comm file"); // The process 123's command is test_cmd

        let proc_scanner = ProcfsScanner {
            proc_dir: test_proc_dir.path().to_path_buf()
        };

        let process_metadata = proc_scanner.fetch_metadata(123)
            .expect("Could not get processes metadata");

        assert_eq!(process_metadata,
                   ProcessMetadata::new(123, "test_cmd".to_string()));
    }

    #[test]
    fn test_get_metadata_with_invalid_pid() {
        let test_proc_dir = tempdir().expect("Could not create tmp dir");

        let proc_scanner = ProcfsScanner { proc_dir: test_proc_dir.path().to_path_buf() };

        let process_metadata_ret = proc_scanner.fetch_metadata(123);

        assert!(matches!(process_metadata_ret, Err(CoreError::ScanProcessesError(_))));
    }
}