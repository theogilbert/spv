use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::marker::PhantomData;
use std::path::PathBuf;

#[derive(Eq, PartialEq, Debug)]
pub enum ProcfsError {
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

pub trait ProcfsData: Sized {
    fn parse(token_parser: &TokenParser) -> Result<Self, ProcfsError>;
}


/// Parses space-separated token from a given multi-line string slice
pub struct TokenParser<'a> {
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
                .filter(|t| !t.is_empty())
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
pub struct ProcfsReader<T> where T: ProcfsData + Sized {
    file: File,
    phantom: PhantomData<T>,  // Here to be able to templetize ProcfsReader
}

impl<T> ProcfsReader<T> where T: ProcfsData + Sized {
    pub fn new(filename: &str) -> Result<Self, ProcfsError> {
        let mut path = PathBuf::from("/proc");
        path.push(filename);

        let file = File::open(path.as_path())
            .map_err(|e| ProcfsError::IoError(e.to_string()))?;

        Ok(ProcfsReader::<T> { file, phantom: PhantomData })
    }

    pub fn new_for_pid(pid: u32, filename: &str) -> Result<Self, ProcfsError> {
        let mut path = PathBuf::from("/proc");
        path.push(pid.to_string());
        path.push(filename);

        let file = File::open(path.as_path())
            .map_err(|e| ProcfsError::IoError(e.to_string()))?;

        Ok(ProcfsReader::<T> { file, phantom: PhantomData })
    }

    /// Returns the data parsed from the file
    pub fn read(&mut self) -> Result<T, ProcfsError> {
        // rather than re-opening the file at each read, we just seek back the start of the file
        self.file
            .seek(SeekFrom::Start(0))
            .map_err(|e| ProcfsError::IoError(e.to_string()))?;

        // Might be optimized, by not reallocating at each call
        let mut stat_content = String::new();
        self.file.read_to_string(&mut stat_content)
            .map_err(|io_err| ProcfsError::IoError(io_err.to_string()))?;

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
    pub fn new(utime: u32, stime: u32, cutime: i32, cstime: i32) -> Self {
        PidStat { utime, stime, cutime, cstime }
    }

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
    use std::string::ToString;

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
    pub fn new(user: u64, nice: u64, system: u64, idle: u64, guest: u64,
               guest_nice: u64) -> Self {
        Stat { user, nice, system, idle, guest, guest_nice }
    }

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
    use std::string::ToString;

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