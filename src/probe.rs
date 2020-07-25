use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::path::PathBuf;

use crate::probe::procfs::{PidStat, ProcfsReader, Stat};
use crate::process::PID;
use crate::values::{BitrateValue, PercentValue};

/// Errors related to probing
#[derive(Debug, Eq, PartialEq, Clone)]
pub enum Error {
    IOError(String),
    ProbingError(String),
}

// Probe stuff

pub trait Probe {
    /// Allow the initialization of the probe for the current iteration
    /// This method should be called before calling the probe() method for each pid
    fn init_iteration(&mut self) -> Result<(), Error> {
        Ok(())
    }

    /// Probe a specific metric value for a given process
    /// # Arguments
    ///  * `pid`: The ID of the process to probe
    ///
    fn probe(&mut self, pid: PID) -> Result<ProcessMetric, Error>;
}

struct ProcessCpuData<T>
    where T: procfs::ProcfsReader {
    reader: T,
    prev_stat: procfs::PidStat,
}

struct StatData<T>
    where T: procfs::ProcfsReader {
    reader: T,
    prev_stat: procfs::Stat,
    run_time_diff: u64,
}

pub(crate) struct CpuProbe<T>
    where T: procfs::ProcfsReader {
    processes_data: HashMap<PID, ProcessCpuData<T>>,
    stat_data: StatData<T>,
}

impl<T> CpuProbe<T> where T: procfs::ProcfsReader {
    pub fn new() -> Result<Self, Error> {
        let mut stat_file = T::new("stat")
            .or_else(|e| Err(Error::IOError(e.to_string())))?;

        let stat_data = stat_file
            .read()
            .or_else(|e| Err(Error::ProbingError(e.to_string())))?;

        Ok(CpuProbe {
            processes_data: HashMap::new(),
            stat_data: StatData {
                reader: stat_file,
                prev_stat: stat_data,
                run_time_diff: 0,
            },
        })
    }

    fn create_process_data(pid: PID) -> Result<ProcessCpuData<T>, Error> {
        let mut stat_file = T::new_for_pid(pid, "stat")
            .or_else(|e| Err(Error::IOError(e.to_string())))?;

        let pid_stat = stat_file
            .read()
            .or_else(|e| Err(Error::ProbingError(e.to_string())))?;

        Ok(ProcessCpuData { reader: stat_file, prev_stat: pid_stat })
    }

    /// Returns a mut reference to `ProcessCpuData` associated with `pid`. If it does not exists,
    /// this function will create it first.
    ///
    /// # Arguments
    ///  * `pid`: The ID of the process forr which to retrieve the `ProcessCpuData`
    ///
    fn get_process_data(&mut self, pid: PID) -> Result<&mut ProcessCpuData<T>, Error> {
        Ok(match self.processes_data.entry(pid) {
            Entry::Occupied(o) => o.into_mut(),
            Entry::Vacant(v) => v.insert(Self::create_process_data(pid)?)
        })
    }
}

impl<T> Probe for CpuProbe<T> where T: procfs::ProcfsReader {
    fn init_iteration(&mut self) -> Result<(), Error> {
        let new_stat: Stat = self.stat_data.reader
            .read()
            .or_else(|e| Err(Error::ProbingError(e.to_string())))?;

        self.stat_data.run_time_diff = new_stat.running_time() - self.stat_data.prev_stat.running_time();
        self.stat_data.prev_stat = new_stat;

        Ok(())
    }

    fn probe(&mut self, pid: PID) -> Result<ProcessMetric, Error> {
        let mut proc_data = self.get_process_data(pid)?;

        let new_pid_stat: PidStat = proc_data.reader
            .read()
            .or_else(|e| Err(Error::ProbingError(e.to_string())))?;

        let pid_runtime_diff = new_pid_stat.running_time() - proc_data.prev_stat.running_time();
        proc_data.prev_stat = new_pid_stat;

        let ratio = pid_runtime_diff as f64 / self.stat_data.run_time_diff as f64;
        let percent = (100. * ratio) as f32;

        let value = PercentValue::new(percent)
            .or_else(|_e| {
                Err(Error::ProbingError(format!("Invalid percent: {}", percent)))
            })?;

        Ok(ProcessMetric { pid, value: Metric::CpuUsage(value) })
    }
}


/// Set of objects to parse and interpret files from `/proc` FS
mod procfs {
    use std::fs::File;
    use std::io::{Read, Seek, SeekFrom};
    use std::path::PathBuf;

    use crate::process::PID;

    #[derive(Eq, PartialEq, Debug)]
    pub(crate) enum ProcfsError {
        InvalidFileContent(String),
        InvalidFileFormat(String),
        IoError(String),
    }

    impl ToString for ProcfsError {
        fn to_string(&self) -> String {
            match self {
                ProcfsError::InvalidFileContent(s) => s.clone(),
                ProcfsError::InvalidFileFormat(s) => s.clone(),
                ProcfsError::IoError(s) => s.clone()
            }
        }
    }

    pub(crate) trait ProcfsData: Sized {
        fn parse(token_parser: &TokenParser) -> Result<Self, ProcfsError>;
    }


    /// Parses space-separated token from a given multi-line string slice
    pub(crate) struct TokenParser<'a> {
        lines: Vec<Vec<&'a str>>
    }

    impl<'a> TokenParser<'a> {
        /// Builds a token parser from a string slice
        /// # Arguments
        ///  * `content` The string slice from which to parse tokens
        fn new(content: &'a str) -> TokenParser<'a> {
            let mut lines = Vec::<Vec<&'a str>>::new();

            for line in content.split('\n') {
                let tokens: Vec<&str> = line.split(' ')
                    .filter(|t| t.len() > 0)
                    .collect();
                lines.push(tokens);
            }

            TokenParser { lines }
        }

        /// Get the value of a token from the parser
        /// # Arguments
        ///  * `line_no`: The line number from which to retrieve the token
        ///  * `pos`: The position of the token in the line (e.g. 1 for token 'b' in line 'a b c')
        fn token<T>(&self, line_no: usize, pos: usize) -> Result<T, ProcfsError>
            where T: std::str::FromStr {
            Ok(self.lines.get(line_no)
                .ok_or({
                    let err_msg = format!("Could not get data at line {} and position {}",
                                          line_no, pos);
                    ProcfsError::InvalidFileFormat(err_msg)
                })?
                .get(pos)
                .ok_or({
                    let err_msg = format!("Could not get token at line {} and position {}",
                                          line_no, pos);
                    ProcfsError::InvalidFileFormat(err_msg)
                })?
                .parse::<T>()
                .or({
                    let err_msg = format!("The token at line {} and position {} \
                                                    could not be parsed", line_no, pos);
                    Err(ProcfsError::InvalidFileContent(err_msg))
                })?)
        }
    }

    #[cfg(test)]
    mod test_token_parser {
        use super::*;

        #[test]
        fn test_extract_data_from_content() {
            let tp = TokenParser::new("1 2 3\n4 5 6");

            assert_eq!(tp.token::<u8>(1, 1), Ok(5));
        }

        #[test]
        fn test_returns_err_when_invalid_line() {
            let tp = TokenParser::new("1 2 3");

            assert!(tp.token::<u8>(1, 1).is_err());
        }

        #[test]
        fn test_returns_err_when_invalid_col() {
            let tp = TokenParser::new("1 2 3\n4 5 6");

            assert!(tp.token::<u8>(1, 4).is_err());
        }

        #[test]
        fn test_returns_err_when_invalid_parse() {
            let tp = TokenParser::new("1 2 3\n4 a 6");

            assert!(tp.token::<u8>(1, 1).is_err());
        }
    }

    /// Handles the IO part of procfs parsing
    pub(crate) trait ProcfsReader where Self: Sized {
        fn new(filename: &str) -> Result<Self, ProcfsError>;
        fn new_for_pid(pid: PID, filename: &str) -> Result<Self, ProcfsError>;

        fn read<T>(&mut self) -> Result<T, ProcfsError>
            where T: ProcfsData + Sized;
    }

    pub(crate) struct ProcFile {
        file: File,
    }

    impl ProcfsReader for ProcFile {
        fn new(filename: &str) -> Result<Self, ProcfsError> {
            let mut path = PathBuf::from("/proc");
            path.push(filename);

            let file = File::open(path.as_path())
                .or_else(|e| Err(ProcfsError::IoError(e.to_string())))?;

            Ok(ProcFile { file })
        }

        fn new_for_pid(pid: u32, filename: &str) -> Result<Self, ProcfsError> {
            let mut path = PathBuf::from("/proc");
            path.push(pid.to_string());
            path.push(filename);

            let file = File::open(path.as_path())
                .or_else(|e| Err(ProcfsError::IoError(e.to_string())))?;

            Ok(ProcFile { file })
        }

        /// Returns the data parsed from the file
        fn read<T>(&mut self) -> Result<T, ProcfsError>
            where T: ProcfsData + Sized {
            // rather than re-opening the file at each read, we just seek back the start of the file
            self.file
                .seek(SeekFrom::Start(0))
                .or_else(|e| Err(ProcfsError::IoError(e.to_string())))?;

            // Might be optimized, by not reallocating at each call
            let mut stat_content = String::new();
            self.file.read_to_string(&mut stat_content)
                .or_else(|io_err| Err(ProcfsError::IoError(io_err.to_string())))?;

            let tp = TokenParser::new(&stat_content);

            T::parse(&tp)
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
    }

    impl PidStat {
        pub fn running_time(&self) -> i64 {
            self.utime as i64 + self.stime as i64 + self.cutime as i64 + self.cstime as i64
        }
    }

    impl ProcfsData for PidStat {
        fn parse(token_parser: &TokenParser) -> Result<Self, ProcfsError> {
            Ok(PidStat {
                utime: token_parser.token(0, 12)?,
                stime: token_parser.token(0, 13)?,
                cutime: token_parser.token(0, 14)?,
                cstime: token_parser.token(0, 15)?,
            })
        }
    }

    #[cfg(test)]
    mod test_pid_stat {
        use super::*;

        #[test]
        fn test_parse_stat_file() {
            let content = "1905 (python3) S 1877 1905 1877 34822 1905 4194304 1096 0 0 \
13 42 11 10 0 20 0 1 0 487679 13963264 2541 18446744073709551615 4194304 7010805 \
140731882007344 0 0 0 0 16781312 134217730 1 0 0 17 0 0 0 0 0 0 9362864 9653016 \
10731520 140731882009319 140731882009327 140731882009327 140731882012647 0".to_string();

            let token_parser = TokenParser::new(&content);

            let pid_stat = PidStat::parse(&token_parser)
                .expect("Could not read PidStat");

            assert_eq!(pid_stat, PidStat {
                utime: 13,
                stime: 42,
                cutime: 11,
                cstime: 10,
            });
        }

        #[test]
        fn test_running_time() {
            let pid_stat = PidStat {
                utime: 1,
                stime: 2,
                cutime: 4,
                cstime: 8,
            };

            assert_eq!(15, pid_stat.running_time())
        }
    }

    /// Represents data and additional computed data from `/proc/stat`
    #[derive(Eq, PartialEq, Debug)]
    pub struct Stat {
        user: u64,
        // Time spent in user mode
        nice: u64,
        // Time spent in user mode with low priority (nice)
        system: u64,
        // Time spent in the idle task
        idle: u64,
        // Time spent in system mode
        // Time spent running a virtual CPU for guest operatin system under the control of the Linux
        // kernel
        guest: u64,
        // Time spent running a niced guest (virtual CPU for guest operating systems under the
        // control of the Linux kernel)
        guest_nice: u64,
    }

    impl Stat {
        pub fn running_time(&self) -> u64 {
            self.user + self.nice + self.system + self.idle + self.guest + self.guest_nice
        }
    }

    impl ProcfsData for Stat {
        fn parse(token_parser: &TokenParser) -> Result<Self, ProcfsError> {
            Ok(Stat {
                user: token_parser.token(0, 1)?,
                nice: token_parser.token(0, 2)?,
                system: token_parser.token(0, 3)?,
                idle: token_parser.token(0, 4)?,
                guest: token_parser.token(0, 9)?,
                guest_nice: token_parser.token(0, 10)?,
            })
        }
    }

    #[cfg(test)]
    mod test_stat {
        use super::*;

        #[test]
        fn test_parse_stat_file() {
            let content = "cpu 10132153 290696 3084719 46828483 16683 0 25195 0 175628 0
cpu0 1393280 32966 572056 13343292 6130 0 17875 0 23933 0".to_string();

            let token_parser = TokenParser::new(&content);

            let pid_stat = Stat::parse(&token_parser)
                .expect("Could not read Stat");

            assert_eq!(pid_stat, Stat {
                user: 10132153,
                nice: 290696,
                system: 3084719,
                idle: 46828483,
                guest: 175628,
                guest_nice: 0,
            });
        }

        #[test]
        fn test_running_time() {
            let stat = Stat {
                user: 1,
                nice: 2,
                system: 4,
                idle: 8,
                guest: 16,
                guest_nice: 32,
            };

            assert_eq!(63, stat.running_time())
        }
    }
}