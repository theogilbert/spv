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
    use std::io::{Seek, SeekFrom};
    use std::path::Path;

    #[derive(Eq, PartialEq, Debug)]
    enum ProcfsError {
        InvalidFileContent(String),
        InvalidFileFormat(String),
        IoError(String),
    }

    trait ProcfsData: Sized {
        fn read<T>(file: &mut T) -> Result<Self, ProcfsError>
            where T: std::io::Read;
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

            T::read(&mut self.file)
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

    /// Given a collection of String tokens, parse one as type `T`
    /// # Arguments
    ///  * `tokens`: A collection of tokens
    ///  * `pos`: The position in `tokens` of the token to parse
    fn parse_token<T>(tokens: &[&str], pos: usize) -> Result<T, ProcfsError>
        where T: std::str::FromStr {
        Ok(tokens.get(pos)
            .ok_or({
                let err_msg = format!("Could not get token at position {}", pos);
                ProcfsError::InvalidFileContent(err_msg)
            })?
            .parse::<T>()
            .or({
                let err_msg = format!("The token at position {} could not be parsed", pos);
                Err(ProcfsError::InvalidFileFormat(err_msg))
            })?)
    }

    impl ProcfsData for PidStat {
        // TODO take T where T: Read as parameter, makes it easier to test, no need to write files
        fn read<T>(file: &mut T) -> Result<Self, ProcfsError>
            where T: std::io::Read {
            // Might be optimized, by not reallocating at each call
            let mut stat_content = String::new();

            file.read_to_string(&mut stat_content)
                .or_else(|io_err| Err(ProcfsError::IoError(io_err.to_string())))?;

            let tokens: Vec<&str> = stat_content.split(' ').collect();

            Ok(PidStat {
                utime: parse_token(&tokens, 12)?,
                stime: parse_token(&tokens, 13)?,
                cutime: parse_token(&tokens, 14)?,
                cstime: parse_token(&tokens, 15)?,
            })
        }
    }

    #[cfg(test)]
    mod test_pid_stat {
        use super::*;
        use std::io::Cursor;

        #[test]
        fn test_parse_stat_file() {
            let content = "1905 (python3) S 1877 1905 1877 34822 1905 4194304 1096 0 0 \
13 42 11 10 0 20 0 1 0 487679 13963264 2541 18446744073709551615 4194304 7010805 \
140731882007344 0 0 0 0 16781312 134217730 1 0 0 17 0 0 0 0 0 0 9362864 9653016 \
10731520 140731882009319 140731882009327 140731882009327 140731882012647 0".to_string();

            let mut data_cursor = Cursor::new(content);

            let pid_stat = PidStat::read(&mut data_cursor)
                .expect("Could not create PidStat from file");

            assert_eq!(pid_stat, PidStat {
                utime: 13,
                stime: 42,
                cutime: 11,
                cstime: 10,
            });
        }
    }

    /// Represents data and additional computed data from `/proc/stat`
    struct Stat {
        /// Sum of all time spent by the process, as indicated by the cpu line
        cumul_cpu_time: u64,
    }
}