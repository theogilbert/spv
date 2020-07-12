use crate::metrics::Value;
use crate::process::PID;

trait Probe<T>
    where T: Value {
    /// Probe a specific metric value for a given process
    /// # Arguments
    ///  * `pid`: The ID of the process to probe
    ///
    fn probe(&mut self, pid: PID) -> T;
}


/// Set of objects to parse and interpret files from `/proc` FS
mod procfs {
    use std::fs::File;
    use std::io::{Read, Seek, SeekFrom};
    use std::path::Path;

    #[derive(Eq, PartialEq, Debug)]
    enum ProcfsError {
        InvalidFileContent(String),
        InvalidFileFormat(String),
        IoError(String),
    }

    trait ProcfsData: Sized {
        fn parse(token_parser: &TokenParser) -> Result<Self, ProcfsError>;
    }


    /// Parses space-separated token from a given multi-line string reference
    struct TokenParser<'a> {
        lines: Vec<Vec<&'a str>>
    }

    impl<'a> TokenParser<'a> {
        /// Builds a token parser from a string slice
        /// # Arguments
        ///  * `content` The string slice from which to parse tokens
        fn new(content: &'a str) -> TokenParser<'a> {
            let mut lines = Vec::<Vec<&'a str>>::new();

            for line in content.split('\n') {
                let tokens: Vec<&str> = line.split(' ').collect();
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
                    let err_msg = format!("Could not get token at position {}", pos);
                    ProcfsError::InvalidFileFormat(err_msg)
                })?
                .parse::<T>()
                .or({
                    let err_msg = format!("The token at position {} could not be parsed", pos);
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
    struct ProcFile {
        file: File,
    }

    impl ProcFile {
        fn new(file_path: &Path) -> std::io::Result<Self> {
            Ok(ProcFile { file: File::open(file_path)? })
        }

        /// Returns the data parsed from the file
        fn parse<T>(&mut self) -> Result<T, ProcfsError>
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
    #[derive(Eq, PartialEq, Debug)]
    struct PidStat {
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
        use std::io::Cursor;

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
    }

    /// Represents data and additional computed data from `/proc/stat`
    #[derive(Eq, PartialEq, Debug)]
    struct Stat {
        user: u64,
        // Time spent in user mode
        nice: u64,
        // Time spent in user mode with low priority (nice)
        system: u64,
        // Time spent in system mode
        // Time spent running a virtual CPU for guest operatin system under the control of the Linux
        // kernel
        guest: u64,
        // Time spent running a niced guest (virtual CPU for guest operating systems under the
        // control of the Linux kernel)
        guest_nice: u64,
    }

    impl ProcfsData for Stat {
        fn parse(token_parser: &TokenParser) -> Result<Self, ProcfsError> {
            Ok(Stat {
                user: token_parser.token(0, 1)?,
                nice: token_parser.token(0, 2)?,
                system: token_parser.token(0, 3)?,
                guest: token_parser.token(0, 9)?,
                guest_nice: token_parser.token(0, 10)?,
            })
        }
    }

    #[cfg(test)]
    mod test_stat {
        use std::io::Cursor;

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
                guest: 175628,
                guest_nice: 0,
            });
        }
    }
}