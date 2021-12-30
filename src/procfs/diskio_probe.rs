//! Disk usage probing

use std::time::Duration;

use crate::core::metrics::IOMetric;
use crate::core::probe::Probe;
use crate::core::process::Pid;
use crate::core::Error;
use crate::procfs::parsers::process::PidIO;
use crate::procfs::parsers::{ProcessDataReader, ReadProcessData};
use crate::procfs::rates::{ProcessesRates, PushMode};

const IO_RATE_RETENTION: Duration = Duration::from_secs(1);

/// Probe implementation to measure and calculate the I/O usage of the disk
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
            .map_err(|e| Error::ProbingError("Could not read process IO stats".to_string(), e.into()))?;

        self.input_rate_calculator.push(pid, pid_io.read_bytes());
        let input_rate = self
            .input_rate_calculator
            .rate(pid)
            .map_err(|e| Error::ProbingError("Could not calculate disk input rate".to_string(), e.into()))?;

        self.output_rate_calculator.push(pid, pid_io.written_bytes());
        let output_rate = self
            .output_rate_calculator
            .rate(pid)
            .map_err(|e| Error::ProbingError("Could not calculate disk output rate".to_string(), e.into()))?;

        Ok(IOMetric::new(input_rate as usize, output_rate as usize))
    }
}

#[cfg(test)]
mod test_disk_io_probe {
    use rstest::*;
    use sn_fake_clock::FakeClock;

    use crate::core::metrics::IOMetric;
    use crate::core::probe::Probe;
    use crate::procfs::diskio_probe::DiskIOProbe;
    use crate::procfs::parsers::fakes::FakeProcessDataReader;
    use crate::procfs::parsers::process::PidIO;

    #[rstest]
    #[case(0, 0, 0, 0, 0)]
    #[case(10, 15, 5, 10, 10)]
    #[case(10, 15, 0, 10, 15)]
    fn test_should_calculate_correct_input_rate(
        #[case] read_bytes: usize,
        #[case] write_bytes: usize,
        #[case] cancelled_write_bytes: usize,
        #[case] expected_input: usize,
        #[case] expected_output: usize,
    ) {
        let sequence = vec![
            PidIO::new(0, 0, 0),
            PidIO::new(read_bytes, write_bytes, cancelled_write_bytes),
        ];

        let mut reader = FakeProcessDataReader::new();
        reader.set_pid_sequence(1, sequence);

        let mut io_probe = DiskIOProbe::from_reader(Box::new(reader));

        let _ = io_probe.probe(1).unwrap();
        FakeClock::advance_time(1000);
        let io_2 = io_probe.probe(1).unwrap();

        assert_eq!(io_2, IOMetric::new(expected_input, expected_output));
    }
}
