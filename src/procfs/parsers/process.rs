use std::path::PathBuf;

use crate::core::process::Pid;
use crate::procfs::parsers::{Parse, ProcessData, TokenParser};
use crate::procfs::ProcfsError;

/// Represents data from `/proc/\[pid\]/comm`
#[derive(Eq, PartialEq, Debug, Clone)]
pub struct Comm {
    command: String,
}

impl Comm {
    #[cfg(test)]
    pub fn new<C>(command: C) -> Self
    where
        C: Into<String>,
    {
        Comm {
            command: command.into(),
        }
    }

    /// Returns the command that spawned the process
    pub fn into_command(self) -> String {
        self.command
    }
}

impl Parse for Comm {
    fn parse(token_parser: &TokenParser) -> Result<Self, ProcfsError> {
        Ok(Comm {
            command: token_parser.token(0, 0)?,
        })
    }
}

impl ProcessData for Comm {
    fn filepath(pid: Pid) -> PathBuf {
        let mut pb = PathBuf::new();

        pb.push("/proc");
        pb.push(pid.to_string());
        pb.push("comm");

        pb
    }
}

#[cfg(test)]
mod test_comm {
    use rstest::*;

    use crate::procfs::parsers::process::Comm;
    use crate::procfs::parsers::{Parse, TokenParser};

    #[rstest]
    #[case("bash")]
    #[case("bash\n")]
    fn test_should_correctly_parse_command(#[case] comm_content: String) {
        let parser = TokenParser::new(&comm_content);
        let comm = Comm::parse(&parser).expect("Cannot parse comm");

        assert_eq!(comm.into_command(), "bash")
    }
}

/// Represents data from `/proc/[PID]/stat`
#[derive(Eq, PartialEq, Debug, Copy, Clone)]
pub struct PidStat {
    /// Time spent by the process in user mode
    // scanf format: %lu
    utime: u32,
    /// Time spent by the process in kernel mode
    // scanf format: %lu
    stime: u32,
    /// Time spent by the process waiting for children processes in user mode
    // scanf format: %ld
    cutime: i32,
    /// Time spent by the process waiting for children processes in kernel mode
    // scanf format: %ld
    cstime: i32,
    /// The time the process started after system boot, expressed in clock ticks
    // scanf format: %llu
    starttime: u64,
}

impl PidStat {
    pub fn running_time(&self) -> i64 {
        self.utime as i64 + self.stime as i64 + self.cutime as i64 + self.cstime as i64
    }

    /// Indicates how long after boot time the process started
    /// This value is expressed in clock ticks. It must be divided by `sysconf(_SC_CLK_TCK)` to get its value in seconds
    pub fn starttime(&self) -> u64 {
        self.starttime
    }
}

impl Parse for PidStat {
    fn parse(token_parser: &TokenParser) -> Result<Self, ProcfsError> {
        Ok(PidStat {
            utime: token_parser.token(0, 12)?,
            stime: token_parser.token(0, 13)?,
            cutime: token_parser.token(0, 14)?,
            cstime: token_parser.token(0, 15)?,
            starttime: token_parser.token(0, 21)?,
        })
    }
}

impl ProcessData for PidStat {
    fn filepath(pid: u32) -> PathBuf {
        let mut path = PathBuf::new();

        path.push("/proc");
        path.push(pid.to_string());
        path.push("stat");

        path
    }
}

#[cfg(test)]
impl PidStat {
    /// PidStat constructor for test purposes
    pub fn new(utime: u32, stime: u32, cutime: i32, cstime: i32, starttime: u64) -> Self {
        PidStat {
            utime,
            stime,
            cutime,
            cstime,
            starttime,
        }
    }
}

#[cfg(test)]
mod test_pid_stat {
    use std::string::ToString;

    use super::*;

    #[test]
    fn test_parse_stat_file() {
        let content = "1905 (python3) S 1877 1905 1877 34822 1905 4194304 1096 0 0 \
13 42 11 10 0 20 0 1 0 487679 13963264 2541 18446744073709551615 4194304 7010805 \
140731882007344 0 0 0 0 16781312 134217730 1 0 0 17 0 0 0 0 0 0 9362864 9653016 \
10731520 140731882009319 140731882009327 140731882009327 140731882012647 0"
            .to_string();

        let token_parser = TokenParser::new(&content);

        let pid_stat = PidStat::parse(&token_parser).expect("Could not read PidStat");

        assert_eq!(
            pid_stat,
            PidStat {
                utime: 13,
                stime: 42,
                cutime: 11,
                cstime: 10,
                starttime: 487679
            }
        );
    }

    #[test]
    fn test_running_time() {
        let pid_stat = PidStat {
            utime: 1,
            stime: 2,
            cutime: 4,
            cstime: 8,
            starttime: 10,
        };

        assert_eq!(15, pid_stat.running_time())
    }

    #[test]
    fn filepath_should_contain_pid() {
        assert_eq!(PidStat::filepath(456), PathBuf::from("/proc/456/stat"))
    }
}

#[derive(Eq, PartialEq, Debug, Copy, Clone)]
pub struct PidIO {
    read_bytes: usize,
    write_bytes: usize,
    cancelled_write_bytes: usize,
}

/// Represents data from `/proc/[PID]/io`
impl PidIO {
    pub fn read_bytes(&self) -> usize {
        self.read_bytes
    }

    pub fn written_bytes(&self) -> usize {
        self.write_bytes.saturating_sub(self.cancelled_write_bytes)
    }
}

#[cfg(test)]
impl PidIO {
    pub fn new(read_bytes: usize, write_bytes: usize, cancelled_write_bytes: usize) -> Self {
        PidIO {
            read_bytes,
            write_bytes,
            cancelled_write_bytes,
        }
    }
}

impl Parse for PidIO {
    fn parse(token_parser: &TokenParser) -> Result<Self, ProcfsError> {
        Ok(PidIO {
            read_bytes: token_parser.token(4, 1)?,
            write_bytes: token_parser.token(5, 1)?,
            cancelled_write_bytes: token_parser.token(6, 1)?,
        })
    }
}

impl ProcessData for PidIO {
    fn filepath(pid: Pid) -> PathBuf {
        let mut path_buf = PathBuf::new();

        path_buf.push("/proc");
        path_buf.push(pid.to_string());
        path_buf.push("io");

        path_buf
    }
}

#[cfg(test)]
mod test_pid_io {
    use std::path::PathBuf;

    use crate::procfs::parsers::process::PidIO;
    use crate::procfs::parsers::{Parse, ProcessData, TokenParser};

    #[test]
    fn test_should_produce_correct_file_path() {
        assert_eq!(PidIO::filepath(42), PathBuf::from("/proc/42/io"));
    }

    #[test]
    fn test_should_parse_file_correctly() {
        let io_file_content = "rchar: 323934931
        wchar: 323929600
        syscr: 632687
        syscw: 632675
        read_bytes: 12345
        write_bytes: 323932160
        cancelled_write_bytes: 876";

        let token_parser = TokenParser::new(io_file_content);
        let pid_io = PidIO::parse(&token_parser).unwrap();

        assert_eq!(pid_io.read_bytes(), 12345);
        assert_eq!(pid_io.written_bytes(), 323932160 - 876);
    }
}
