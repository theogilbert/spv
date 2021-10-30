//! CPU Usage probing

use std::time::Duration;

use crate::core::metrics::IOMetric;
use crate::core::probe::Probe;
use crate::core::process::Pid;
use crate::core::Error;
use crate::procfs::parsers::{PidIO, ProcessDataReader, ReadProcessData};
use crate::procfs::rates::{ProcessesRates, PushMode};

const IO_RATE_RETENTION: Duration = Duration::from_secs(1);

/// Probe implementation to measure the CPU usage (in percent) of processes
pub struct DiskIOProbe {
    reader: Box<dyn ReadProcessData<PidIO>>,
    input_rate_calculator: ProcessesRates,
    output_rate_calculator: ProcessesRates,
}

impl Default for DiskIOProbe {
    fn default() -> Self {
        Self::from_reader(Box::new(ProcessDataReader::new()))
    }
}

impl DiskIOProbe {
    fn from_reader(reader: Box<dyn ReadProcessData<PidIO>>) -> Self {
        DiskIOProbe {
            reader,
            input_rate_calculator: ProcessesRates::new(PushMode::Accumulative, IO_RATE_RETENTION),
            output_rate_calculator: ProcessesRates::new(PushMode::Accumulative, IO_RATE_RETENTION),
        }
    }
}

impl Probe<IOMetric> for DiskIOProbe {
    fn name(&self) -> &'static str {
        "Disk I/O"
    }

    fn probe(&mut self, pid: Pid) -> Result<IOMetric, Error> {
        let pid_io = self
            .reader
            .read(pid)
            .map_err(|e| Error::ProbingError(format!("Error probing disk IO stats for PID {}", pid), e.into()))?;

        self.input_rate_calculator.push(pid, pid_io.read_bytes());
        let input_rate = self
            .input_rate_calculator
            .rate(pid)
            .map_err(|e| Error::ProbingError(format!("Error calculating input rate for PID {}", pid), e.into()))?;

        self.output_rate_calculator.push(pid, pid_io.written_bytes());
        let output_rate = self
            .output_rate_calculator
            .rate(pid)
            .map_err(|e| Error::ProbingError(format!("Error calculating output rate for PID {}", pid), e.into()))?;

        Ok(IOMetric::new(input_rate as usize, output_rate as usize))
    }
}

#[cfg(test)]
mod test_disk_io_probe {
    use crate::core::metrics::IOMetric;
    use crate::core::probe::Probe;
    use crate::core::process::Pid;
    use crate::procfs::diskio_probe::DiskIOProbe;
    use crate::procfs::parsers::{PidIO, ReadProcessData};
    use crate::procfs::ProcfsError;
    use sn_fake_clock::FakeClock;

    struct PidIOReaderStub {
        reverted_sequence: Vec<PidIO>,
    }

    impl PidIOReaderStub {
        fn new(mut sequence: Vec<PidIO>) -> Self {
            sequence.reverse();

            PidIOReaderStub {
                reverted_sequence: sequence,
            }
        }
    }

    impl ReadProcessData<PidIO> for PidIOReaderStub {
        fn read(&mut self, _pid: Pid) -> Result<PidIO, ProcfsError> {
            Ok(self
                .reverted_sequence
                .pop()
                .expect("Index error while reading with PidIOReaderStub"))
        }
    }

    #[test]
    fn test_should_calculate_correct_input_rate() {
        let sequence = vec![PidIO::new(0, 0, 0), PidIO::new(10, 15, 5)];
        let reader = PidIOReaderStub::new(sequence);

        let mut io_probe = DiskIOProbe::from_reader(Box::new(reader));

        FakeClock::set_time(1000);
        let io_1 = io_probe.probe(0).unwrap();
        FakeClock::advance_time(1000);
        let io_2 = io_probe.probe(0).unwrap();

        assert_eq!(io_1, IOMetric::new(0, 0));
        assert_eq!(io_2, IOMetric::new(10, 10));
    }
}
