//! Parsers to read structured data from the /proc directory

use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::marker::PhantomData;
use std::path::{Path, PathBuf};

use crate::core::process::Pid;
use crate::procfs::ProcfsError;

pub mod process;
pub mod system;

/// Type which can be parsed from a `TokenParser`
pub trait Parse: Sized {
    fn parse(token_parser: &TokenParser) -> Result<Self, ProcfsError>;
}

/// Specialization of a `Data` type which is not associated to a process
pub trait SystemData: Parse {
    fn filepath() -> PathBuf;
}

/// Specialization of a `Data` type which is associated to a process
pub trait ProcessData: Parse {
    fn filepath(pid: Pid) -> PathBuf;
}

/// Type which can read a `SystemData`
pub trait ReadSystemData<D>
where
    D: SystemData + Sized,
{
    fn read(&mut self) -> Result<D, ProcfsError>;
}

/// Type which can read a `ProcessData`
pub trait ReadProcessData<D>
where
    D: ProcessData + Sized,
{
    fn read(&mut self, pid: Pid) -> Result<D, ProcfsError>;

    fn cleanup(&mut self, pid: Pid);
}

/// Reads data from procfs system files that are not associated to processes (directly in `/proc`)
pub struct SystemDataReader<D>
where
    D: SystemData + Sized,
{
    reader: ProcfsFileReader<D>,
}

impl<D> SystemDataReader<D>
where
    D: SystemData + Sized,
{
    pub fn new() -> Result<Self, ProcfsError> {
        let reader = ProcfsFileReader::new(D::filepath().as_path())?;
        Ok(SystemDataReader { reader })
    }
}

impl<D> ReadSystemData<D> for SystemDataReader<D>
where
    D: SystemData + Sized,
{
    fn read(&mut self) -> Result<D, ProcfsError> {
        self.reader.read()
    }
}

/// Reads data from procfs files bound to a PID
///
/// This reader does not keep open the files it reads
#[derive(Default)]
pub struct TransientProcessDataReader;

impl<D> ReadProcessData<D> for TransientProcessDataReader
where
    D: ProcessData + Sized,
{
    fn read(&mut self, pid: Pid) -> Result<D, ProcfsError> {
        ProcfsFileReader::new(D::filepath(pid).as_path())?.read()
    }

    fn cleanup(&mut self, _pid: Pid) {
        // Nothing to do
    }
}

/// Reads data from procfs files bound to a PID (in `/proc/[pid]/`)
///
/// This reader keeps the latest files it read open, to be more efficient.
pub struct ProcessDataReader<D>
where
    D: ProcessData + Sized,
{
    readers: HashMap<Pid, ProcfsFileReader<D>>,
    limiter: TailedProcessLimiter,
}

impl<D> ProcessDataReader<D>
where
    D: ProcessData + Sized,
{
    pub fn with_capacity(capacity: usize) -> Self {
        ProcessDataReader {
            readers: HashMap::new(),
            limiter: TailedProcessLimiter::with_capacity(capacity),
        }
    }

    fn process_reader(&mut self, pid: Pid) -> Result<&mut ProcfsFileReader<D>, ProcfsError> {
        Ok(match self.readers.entry(pid) {
            Entry::Occupied(o) => o.into_mut(),
            Entry::Vacant(v) => {
                self.limiter.push_pid(pid);
                v.insert(ProcfsFileReader::new(D::filepath(pid).as_path())?)
            }
        })
    }

    fn close_process_file(&mut self, pid: Pid) {
        self.readers.remove(&pid);
        self.limiter.delete_pid(pid);
    }
}

impl<D> ReadProcessData<D> for ProcessDataReader<D>
where
    D: ProcessData + Sized,
{
    fn read(&mut self, pid: u32) -> Result<D, ProcfsError> {
        let data_ret = self.process_reader(pid)?.read();

        if data_ret.is_err() || !self.limiter.should_keep_process_file_opened(pid) {
            self.close_process_file(pid);
        }

        data_ret
    }

    fn cleanup(&mut self, pid: Pid) {
        self.close_process_file(pid);
    }
}

/// This structure is there to help `ProcessDataReader` limit the amount of opened files.
///
/// As opening a file is an expensive operation, we want to keep as many files open as possible.
/// But on Linux, there is a limit to how many files a single process can hold open.
///
/// The issue is that if we do not limit the amount of opened files, the application will panic when too many processes
/// will be probed, as it will want to keep all of their procfs files opened.
///
/// The current strategy of this limiter is to only keep open the files of the oldest processes. The idea behind it is
/// that the oldest running processes are less likely to die in the near future, reducing the amount of open files
/// turnover when the application reaches the limit of files to keep open.
struct TailedProcessLimiter {
    opened_processes: Vec<Pid>,
    capacity: usize,
}

impl TailedProcessLimiter {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            opened_processes: Vec::with_capacity(capacity),
            capacity,
        }
    }

    pub fn push_pid(&mut self, pid: Pid) {
        if self.opened_processes.len() >= self.capacity {
            return;
        }

        self.opened_processes.push(pid);
    }

    pub fn delete_pid(&mut self, pid: Pid) {
        let pid_index = self.opened_processes.iter().position(|pid_in_vec| *pid_in_vec == pid);

        if let Some(pid_index) = pid_index {
            self.opened_processes.remove(pid_index);
        }
    }

    pub fn should_keep_process_file_opened(&self, pid: Pid) -> bool {
        self.opened_processes.contains(&pid)
    }
}

#[cfg(test)]
mod test_tailed_process_limiter {
    use crate::procfs::parsers::TailedProcessLimiter;

    #[test]
    fn should_keep_file_opened_when_under_capacity() {
        let mut limiter = TailedProcessLimiter::with_capacity(2);

        limiter.push_pid(1);
        limiter.push_pid(2);

        assert!(limiter.should_keep_process_file_opened(2));
    }

    #[test]
    fn should_not_keep_file_opened_when_above_capacity() {
        let mut limiter = TailedProcessLimiter::with_capacity(2);

        limiter.push_pid(1);
        limiter.push_pid(2);
        limiter.push_pid(3);

        assert!(!limiter.should_keep_process_file_opened(3));
    }

    #[test]
    fn should_keep_file_opened_when_below_capacity_after_cleanup() {
        let mut limiter = TailedProcessLimiter::with_capacity(2);

        limiter.push_pid(1);
        limiter.push_pid(2);
        limiter.delete_pid(2);
        limiter.push_pid(3);

        assert!(limiter.should_keep_process_file_opened(3));
    }

    #[test]
    fn should_keep_already_pushed_file_open_even_if_pushed_again_above_capacity() {
        let mut limiter = TailedProcessLimiter::with_capacity(2);

        limiter.push_pid(1);
        limiter.push_pid(2);
        limiter.push_pid(1);

        assert!(limiter.should_keep_process_file_opened(1));
    }

    #[test]
    fn should_not_keep_file_open_if_pushed_and_destroyed_and_repushed_above_capacity() {
        let mut limiter = TailedProcessLimiter::with_capacity(2);

        limiter.push_pid(1);
        limiter.push_pid(2);
        limiter.delete_pid(2);
        limiter.push_pid(3);
        limiter.push_pid(2);

        assert!(!limiter.should_keep_process_file_opened(2));
    }
}

/// This reader can repeatedly read and parse data from a given procfs file.
///
/// It keeps the file it reads open.
struct ProcfsFileReader<D>
where
    D: Parse + Sized,
{
    reader: DataReader<File, D>,
}

impl<D> ProcfsFileReader<D>
where
    D: Parse + Sized,
{
    pub fn new(filepath: &Path) -> Result<Self, ProcfsError> {
        File::open(filepath).map_err(ProcfsError::from).map(|file| Self {
            reader: DataReader::new(file),
        })
    }

    pub fn read(&mut self) -> Result<D, ProcfsError> {
        self.reader.read()
    }
}

/// This reader parses a struct implementing `Parse` from any structure which implements the `Read + Seek` traits
struct DataReader<R, D>
where
    R: Read + Seek,
    D: Parse + Sized,
{
    src: R,
    phantom: PhantomData<D>,
}

impl<R, D> DataReader<R, D>
where
    R: Read + Seek,
    D: Parse + Sized,
{
    pub fn new(src: R) -> Self {
        DataReader {
            src,
            phantom: PhantomData,
        }
    }

    pub fn read(&mut self) -> Result<D, ProcfsError> {
        self.src.seek(SeekFrom::Start(0))?;

        // Might be optimized, by not reallocating at each call
        let mut stat_content = String::new();
        self.src.read_to_string(&mut stat_content)?;

        let tp = TokenParser::new(&stat_content);

        D::parse(&tp)
    }
}

/// Parses space-separated token from a given multi-line string slice
pub struct TokenParser<'a> {
    lines: Vec<Vec<&'a str>>,
}

impl<'a> TokenParser<'a> {
    /// Builds a token parser from a string slice
    /// # Arguments
    ///  * `content` The string slice from which to parse tokens
    fn new(content: &'a str) -> TokenParser<'a> {
        let mut lines = Vec::<Vec<&'a str>>::new();

        for line in content.split('\n') {
            let tokens: Vec<&str> = line.split(' ').filter(|t| !t.is_empty()).collect();
            lines.push(tokens);
        }

        TokenParser { lines }
    }

    /// Get the value of a token from the parser
    /// # Arguments
    ///  * `line_no`: The line number from which to retrieve the token
    ///  * `pos`: The position of the token in the line (e.g. 1 for token 'b' in line 'a b c')
    fn token<T>(&self, line_no: usize, pos: usize) -> Result<T, ProcfsError>
    where
        T: std::str::FromStr,
    {
        self.lines
            .get(line_no)
            .ok_or({
                let err_msg = format!("Could not get data at line {} and position {}", line_no, pos);
                ProcfsError::InvalidFileFormat(err_msg)
            })?
            .get(pos)
            .ok_or({
                let err_msg = format!("Could not get token at line {} and position {}", line_no, pos);
                ProcfsError::InvalidFileFormat(err_msg)
            })?
            .parse::<T>()
            .or({
                let err_msg = format!(
                    "The token at line {} and position {} \
                                                could not be parsed",
                    line_no, pos
                );
                Err(ProcfsError::InvalidFileContent(err_msg))
            })
    }
}

#[cfg(test)]
mod test_data_reader {
    use std::io::Cursor;

    use crate::procfs::parsers::{DataReader, Parse, ProcfsError, TokenParser};

    #[derive(PartialEq, Debug)]
    struct TestSystemData {
        field_1: u8,
        field_2: i16,
    }

    impl Parse for TestSystemData {
        fn parse(token_parser: &TokenParser) -> Result<Self, ProcfsError> {
            Ok(TestSystemData {
                field_1: token_parser.token(0, 0)?,
                field_2: token_parser.token(0, 1)?,
            })
        }
    }

    #[test]
    fn test_load_correctly_data() {
        let data_src = Cursor::new(b"12 -92 abc");

        let mut data_reader = DataReader::new(data_src);

        assert!(matches!(
            data_reader.read(),
            Ok(TestSystemData {
                field_1: 12,
                field_2: -92
            })
        ));
    }
}

#[cfg(test)]
mod test_token_parser {
    use super::*;

    #[test]
    fn test_extract_data_from_content() {
        let tp = TokenParser::new("1 2 3\n4 5 6");

        assert!(matches!(tp.token::<u8>(1, 1), Ok(5)));
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

/// Modules containing fake readers to be used in tests
#[cfg(test)]
pub mod fakes {
    use std::collections::{HashMap, VecDeque};
    use std::io;

    use crate::core::process::Pid;
    use crate::procfs::parsers::{ProcessData, ReadProcessData, ReadSystemData, SystemData};
    use crate::procfs::ProcfsError;

    /// A fake structure implementing `ReadSystemData` for any implementation of `SystemData`.
    /// The `SystemData` returned by this reader can be customized through the [`Self::from_sequence()`] function.
    pub struct FakeSystemDataReader<D>
    where
        D: SystemData + Sized,
    {
        data_sequence: VecDeque<D>,
    }

    impl<D> FakeSystemDataReader<D>
    where
        D: SystemData + Sized,
    {
        pub fn from_sequence(sequence: Vec<D>) -> Self {
            Self {
                data_sequence: sequence.into(),
            }
        }
    }

    impl<D> ReadSystemData<D> for FakeSystemDataReader<D>
    where
        D: SystemData + Sized,
    {
        fn read(&mut self) -> Result<D, ProcfsError> {
            Ok(self
                .data_sequence
                .pop_front()
                .expect("The system data reader has nothing to return"))
        }
    }

    /// A fake structure implementing `ReadProcessData` for any implementation of `ProcessData`.
    pub struct FakeProcessDataReader<D>
    where
        D: ProcessData + Sized,
    {
        process_data_sequences: HashMap<Pid, VecDeque<Result<D, ProcfsError>>>,
    }

    impl<D> FakeProcessDataReader<D>
    where
        D: ProcessData + Sized,
    {
        pub fn new() -> Self {
            Self {
                process_data_sequences: hashmap!(),
            }
        }

        pub fn set_pid_sequence(&mut self, pid: Pid, sequence: Vec<D>) {
            let result_sequence = sequence.into_iter().map(|d| Ok(d)).collect();
            self.process_data_sequences.insert(pid, result_sequence);
        }

        pub fn make_pid_fail(&mut self, pid: Pid) {
            let err = Err(ProcfsError::IOError(io::Error::new(io::ErrorKind::Other, "oh no!")));
            self.process_data_sequences.insert(pid, vecdeque!(err));
        }
    }

    impl<D> ReadProcessData<D> for FakeProcessDataReader<D>
    where
        D: ProcessData + Sized,
    {
        fn read(&mut self, pid: Pid) -> Result<D, ProcfsError> {
            self.process_data_sequences
                .get_mut(&pid)
                .expect("No data is configured for this process")
                .pop_front()
                .expect("This process has no data left to return")
        }

        fn cleanup(&mut self, _pid: Pid) {
            // Nothing to cleanup
        }
    }
}
