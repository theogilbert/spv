//! Process discovery

use std::fs::{read_dir, DirEntry};
use std::io;
use std::path::PathBuf;
use std::time::Duration;

use thiserror::Error;

use crate::core::process::{Pid, ProcessMetadata, ProcessScanner};
use crate::core::time::Timestamp;
use crate::core::Error as CoreError;
use crate::procfs::parsers::{
    Comm, PidStat, ProcessDataReader, ReadProcessData, ReadSystemData, SystemDataReader, Uptime,
};
use crate::procfs::sysconf::clock_ticks;
use crate::procfs::ProcfsError;

/// Errors internal to the process module
#[derive(Error, Debug)]
enum Error {
    #[error("Directory PID has invalid syntax: '{0:?}'")]
    NotProcessDir(String),
    #[error("Failed to parse system data '{0:?}': '{1:?}'")]
    SystemParsingFailure(String, #[source] ProcfsError),
    #[error("Failed to read the content of directory '{0:?}'")]
    ProcessScanningFailure(PathBuf, #[source] io::Error),
    #[error("Error while parsing process data '{1:?}' for process {0:?}: '{2:?}'")]
    ProcessParsing(Pid, String, #[source] anyhow::Error),
}

impl From<Error> for CoreError {
    fn from(e: Error) -> Self {
        CoreError::ScanProcessesError(e.into())
    }
}

/// Implementation of ProcessScanner that uses the `/proc` Linux virtual directory as source
pub struct ProcfsScanner {
    proc_dir: PathBuf,
    comm_reader: Box<dyn ReadProcessData<Comm>>,
    stat_reader: Box<dyn ReadProcessData<PidStat>>,
    boot_time: Timestamp,
}

/// Scan running processes on a Linux host by scanning the content of /proc directory
impl ProcfsScanner {
    /// Returns a new ProcfsScanner instance
    pub fn new() -> Result<ProcfsScanner, CoreError> {
        let boot_time = SystemDataReader::<Uptime>::new()
            .map_err(|e| Error::SystemParsingFailure("uptime".into(), e))?
            .read()
            .map_err(|e| Error::SystemParsingFailure("uptime".into(), e))?
            .boot_time();

        Ok(ProcfsScanner {
            proc_dir: PathBuf::from("/proc"),
            comm_reader: Box::new(ProcessDataReader::new()),
            stat_reader: Box::new(ProcessDataReader::new()),
            boot_time,
        })
    }

    /// Parses a PID from a directory name, if it represents an unsigned integer
    ///
    /// # Arguments
    /// * `dir_name` - An optional string slice that holds the name of a directory
    ///
    fn extract_pid_from_proc_dir(dir_name_opt: Option<&str>) -> Result<Pid, Error> {
        match dir_name_opt {
            Some(dir_name) => dir_name
                .parse::<Pid>()
                .map_err(|_| Error::NotProcessDir(dir_name.to_string())),
            None => Err(Error::NotProcessDir("".to_string())),
        }
    }

    /// Calculates the timestamp at which the process started
    fn calculate_spawn_time(&mut self, pid: Pid) -> Result<Timestamp, CoreError> {
        let clock_ticks = clock_ticks().map_err(|e| Error::SystemParsingFailure("_SC_CLK_TCK".into(), e))?;

        let starttime = self
            .stat_reader
            .read(pid)
            .map_err(|e| Error::ProcessParsing(pid, "stat".into(), e.into()))?
            .starttime();

        Ok(self.boot_time + Duration::from_secs(starttime / clock_ticks))
    }
}

impl ProcessScanner for ProcfsScanner {
    /// Returns the PIDs of currently running processes
    fn scan(&mut self) -> std::result::Result<Vec<Pid>, CoreError> {
        let path = self.proc_dir.as_path();

        let dir_iter = read_dir(path).map_err(|e| Error::ProcessScanningFailure(path.into(), e))?;

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
    fn fetch_metadata(&mut self, pid: Pid) -> std::result::Result<ProcessMetadata, CoreError> {
        let comm = self
            .comm_reader
            .read(pid)
            .map_err(|e| Error::ProcessParsing(pid, "comm".into(), e.into()))?;

        let spawntime = self.calculate_spawn_time(pid)?;

        Ok(ProcessMetadata::new(pid, comm.into_command(), spawntime))
    }
}

#[cfg(test)]
mod test_pid_from_proc_dir {
    use super::*;

    #[test]
    fn test_pid_from_valid_proc_dir_name() {
        let valid_pid = ProcfsScanner::extract_pid_from_proc_dir(Some("123")).unwrap();

        assert_eq!(valid_pid, 123);
    }

    #[test]
    fn test_pid_from_invalid_proc_dir_name() {
        match ProcfsScanner::extract_pid_from_proc_dir(Some("abc")) {
            Err(Error::NotProcessDir(dir)) => assert_eq!(dir, String::from("abc")),
            _ => assert!(false),
        }
    }

    #[test]
    fn test_pid_from_no_proc_dir_name() {
        match ProcfsScanner::extract_pid_from_proc_dir(None) {
            Err(Error::NotProcessDir(dir)) => assert_eq!(dir, String::new()),
            _ => assert!(false),
        }
    }
}

#[cfg(test)]
mod test_pid_scanner {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::Path;

    use sn_fake_clock::FakeClock;
    use tempfile::{tempdir, NamedTempFile};

    use crate::core::Error as CoreError;
    use crate::procfs::parsers::fakes::FakeProcessDataReader;

    use super::*;

    fn create_tempdir<T: Into<PathBuf>>(name: &str, dir: T) -> std::io::Result<()> {
        fs::create_dir(dir.into().join(name))
    }

    fn create_tempfile<T: Into<PathBuf>>(name: &str, dir: T) -> std::io::Result<fs::File> {
        let fp = dir.into().join(name);

        match NamedTempFile::new() {
            Ok(ntf) => Ok(ntf.persist(fp).expect("Could not persist file")),
            Err(e) => Err(e),
        }
    }

    fn set_dir_permissions(path: &Path, mode: u32) -> std::io::Result<()> {
        let mut perms = fs::metadata(path)?.permissions();

        perms.set_mode(mode);
        fs::set_permissions(path, perms)
    }

    fn build_pid_scanner(proc_dir: PathBuf) -> ProcfsScanner {
        ProcfsScanner {
            proc_dir,
            comm_reader: Box::new(FakeProcessDataReader::new()),
            stat_reader: Box::new(FakeProcessDataReader::new()),
            boot_time: Timestamp::now(),
        }
    }

    fn build_metadata_fetcher(
        comm_reader: FakeProcessDataReader<Comm>,
        stat_reader: FakeProcessDataReader<PidStat>,
    ) -> ProcfsScanner {
        ProcfsScanner {
            proc_dir: PathBuf::new(),
            comm_reader: Box::new(comm_reader),
            stat_reader: Box::new(stat_reader),
            boot_time: Timestamp::now(),
        }
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
            create_tempfile("46a", test_proc_dir.path()),
        ];

        if proc_subdirs.iter().any(|i| i.is_err()) || proc_subfiles.iter().any(|i| i.is_err()) {
            panic!(
                "Could not create all temp dir/files: {:?} / {:?}",
                proc_subdirs, proc_subfiles
            );
        }

        let mut proc_scanner = build_pid_scanner(test_proc_dir.path().to_path_buf());

        // when we scan processes
        let mut pids = proc_scanner.scan().expect("Could not scan processes");
        pids.sort();

        // The PIDs are only those represented by a dir with an integer name
        assert_eq!(vec![123, 456], pids);
    }

    #[test]
    fn test_scan_process_without_permissions() {
        // Given we do not have read access to test /proc dir
        let test_proc_dir = tempdir().expect("Could not create tmp dir");
        set_dir_permissions(test_proc_dir.path(), 0o000).expect("Could not set dir permissions");

        let mut proc_scanner = build_pid_scanner(test_proc_dir.path().to_path_buf());

        // when we scan processes
        let pids = proc_scanner.scan();

        // reset permission to allow dir removal
        set_dir_permissions(test_proc_dir.path(), 0o755).expect("Could not set dir permissions");

        assert!(pids.is_err());
    }

    #[test]
    fn test_process_metadata_has_correct_cmd() {
        let mut comm_reader = FakeProcessDataReader::<Comm>::new();
        let mut stat_reader = FakeProcessDataReader::<PidStat>::new();

        comm_reader.set_pid_sequence(123, vec![Comm::new("test_cmd")]);
        stat_reader.set_pid_sequence(123, vec![PidStat::new(0, 0, 0, 0, 0)]);

        let mut proc_scanner = build_metadata_fetcher(comm_reader, stat_reader);

        let process_metadata = proc_scanner
            .fetch_metadata(123)
            .expect("Could not get processes metadata");

        assert_eq!(process_metadata.command(), "test_cmd");
    }

    #[test]
    fn test_process_metadata_has_correct_starttime() {
        let mut comm_reader = FakeProcessDataReader::<Comm>::new();
        let mut stat_reader = FakeProcessDataReader::<PidStat>::new();

        let starttime = 1000;
        comm_reader.set_pid_sequence(123, vec![Comm::new("test_cmd")]);
        stat_reader.set_pid_sequence(123, vec![PidStat::new(0, 0, 0, 0, starttime)]);

        let mut proc_scanner = build_metadata_fetcher(comm_reader, stat_reader);

        FakeClock::advance_time(1000);

        let process_metadata = proc_scanner
            .fetch_metadata(123)
            .expect("Could not get processes metadata");

        let clock_ticks = clock_ticks().unwrap();
        let expected_spawn_time = proc_scanner.boot_time + Duration::from_secs(starttime / clock_ticks);

        assert_eq!(process_metadata.running_span().begin(), expected_spawn_time);
    }

    #[test]
    fn test_get_metadata_with_invalid_pid() {
        let mut comm_reader = FakeProcessDataReader::<Comm>::new();
        let stat_reader = FakeProcessDataReader::<PidStat>::new();

        comm_reader.make_pid_fail(123);

        let mut proc_scanner = build_metadata_fetcher(comm_reader, stat_reader);

        let process_metadata_ret = proc_scanner.fetch_metadata(123);

        assert!(matches!(process_metadata_ret, Err(CoreError::ScanProcessesError(_))));
    }
}
