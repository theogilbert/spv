use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::marker::PhantomData;
use std::path::{Path, PathBuf};
use std::time::Duration;
#[cfg(not(test))]
use std::time::Instant;

#[cfg(test)]
use sn_fake_clock::FakeClock as Instant;

use crate::core::process::Pid;
use crate::core::time::Timestamp;
use crate::procfs::ProcfsError;
use crate::procfs::ProcfsError::InvalidFileContent;

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
}

/// Reads data from procfs system files that are not associated to processes (directly in `/proc`)
pub struct SystemDataReader<D>
where
    D: SystemData + Sized,
{
    reader: ProcfsReader<D>,
}

impl<D> SystemDataReader<D>
where
    D: SystemData + Sized,
{
    pub fn new() -> Result<Self, ProcfsError> {
        let reader = ProcfsReader::new(D::filepath().as_path())?;
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

/// Reads data from procfs files bound to a PID (in `/proc/[pid]/`)
pub struct ProcessDataReader<D>
where
    D: ProcessData + Sized,
{
    readers: HashMap<Pid, ProcfsReader<D>>,
}

impl<D> ProcessDataReader<D>
where
    D: ProcessData + Sized,
{
    pub fn new() -> Self {
        ProcessDataReader {
            readers: HashMap::new(),
        }
    }

    fn process_reader(&mut self, pid: Pid) -> Result<&mut ProcfsReader<D>, ProcfsError> {
        Ok(match self.readers.entry(pid) {
            Entry::Occupied(o) => o.into_mut(),
            Entry::Vacant(v) => v.insert(ProcfsReader::new(D::filepath(pid).as_path())?),
        })
    }
}

impl<D> ReadProcessData<D> for ProcessDataReader<D>
where
    D: ProcessData + Sized,
{
    fn read(&mut self, pid: u32) -> Result<D, ProcfsError> {
        let data_ret = self.process_reader(pid)?.read();

        if data_ret.is_err() {
            // if reading files for this PID fails, we stop tracking the file
            self.readers.remove(&pid);
        }

        data_ret
    }
}

struct ProcfsReader<D>
where
    D: Parse + Sized,
{
    reader: DataReader<File, D>,
}

impl<D> ProcfsReader<D>
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

/// --------------------
/// Data implementations
/// --------------------

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
    pub fn new(user: u64, nice: u64, system: u64, idle: u64, guest: u64, guest_nice: u64) -> Self {
        Stat {
            user,
            nice,
            system,
            idle,
            guest,
            guest_nice,
        }
    }

    pub fn running_time(&self) -> u64 {
        self.user + self.nice + self.system + self.idle + self.guest + self.guest_nice
    }
}

impl Parse for Stat {
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

impl SystemData for Stat {
    fn filepath() -> PathBuf {
        ["/proc", "stat"].iter().collect()
    }
}

#[cfg(test)]
mod test_stat {
    use std::string::ToString;

    use super::*;

    #[test]
    fn test_parse_stat_file() {
        let content = "cpu 10132153 290696 3084719 46828483 16683 0 25195 0 175628 0
cpu0 1393280 32966 572056 13343292 6130 0 17875 0 23933 0"
            .to_string();

        let token_parser = TokenParser::new(&content);

        let pid_stat = Stat::parse(&token_parser).expect("Could not read Stat");

        assert_eq!(
            pid_stat,
            Stat {
                user: 10132153,
                nice: 290696,
                system: 3084719,
                idle: 46828483,
                guest: 175628,
                guest_nice: 0,
            }
        );
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

    use crate::procfs::parsers::{Comm, Parse, TokenParser};

    #[rstest]
    #[case("bash")]
    #[case("bash\n")]
    fn test_should_correctly_parse_command(#[case] comm_content: String) {
        let parser = TokenParser::new(&comm_content);
        let comm = Comm::parse(&parser).expect("Cannot parse comm");

        assert_eq!(comm.into_command(), "bash")
    }
}

/// Represents data from `/proc/uptime`
#[derive(Eq, PartialEq, Debug, Copy, Clone)]
pub struct Uptime {
    /// Represents the amount of seconds elapsed since the system booted
    // scanf format: unspecified
    uptime: u64,
    /// Represents the actual timestamp at which the system was booted
    boot_time: Timestamp,
}

impl Parse for Uptime {
    fn parse(token_parser: &TokenParser) -> Result<Self, ProcfsError> {
        let uptime_repr: String = token_parser.token(0, 0)?;
        let uptime: u64 = uptime_repr
            .parse::<f64>()
            .map_err(|_| InvalidFileContent("Could not parse uptime".to_string()))? as u64;

        let boot_time = Instant::now()
            .checked_sub(Duration::from_secs(uptime))
            .ok_or_else(|| InvalidFileContent("Uptime is greater than current time".to_string()))?;
        let boot_time = Timestamp::from_instant(boot_time);

        Ok(Self { uptime, boot_time })
    }
}

impl Uptime {
    pub fn boot_time(&self) -> Timestamp {
        self.boot_time
    }
}

impl SystemData for Uptime {
    fn filepath() -> PathBuf {
        ["/proc", "uptime"].iter().collect()
    }
}

#[cfg(test)]
mod test_uptime {
    use std::ops::Sub;
    use std::time::Duration;

    use crate::core::time::Timestamp;
    use sn_fake_clock::FakeClock;

    use crate::procfs::parsers::{Parse, TokenParser, Uptime};

    #[test]
    fn test_parse_uptime() {
        FakeClock::set_time(1000000000);
        let content = "10281.87 123230.54".to_string();

        let token_parser = TokenParser::new(&content);

        let uptime = Uptime::parse(&token_parser).expect("Could not read Uptime");

        assert_eq!(uptime.uptime, 10281);
    }

    #[test]
    fn test_should_calculate_boottime_from_now_instant() {
        FakeClock::set_time(1000000000);
        let content = "2000.87 123230.54".to_string();

        let token_parser = TokenParser::new(&content);
        let uptime = Uptime::parse(&token_parser).expect("Could not read Uptime");

        let expected_boot_time = Timestamp::from_instant(FakeClock::now().sub(Duration::from_secs(2000)));

        assert_eq!(uptime.boot_time(), expected_boot_time);
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

    use crate::procfs::parsers::{Parse, PidIO, ProcessData, TokenParser};

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
