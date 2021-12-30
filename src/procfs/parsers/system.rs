use std::path::PathBuf;
use std::time::Duration;
#[cfg(not(test))]
use std::time::Instant;

#[cfg(test)]
use sn_fake_clock::FakeClock as Instant;

use crate::core::time::Timestamp;
use crate::procfs::parsers::{Parse, SystemData, TokenParser};
use crate::procfs::ProcfsError;
use crate::procfs::ProcfsError::InvalidFileContent;

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

    use sn_fake_clock::FakeClock;

    use crate::core::time::Timestamp;
    use crate::procfs::parsers::system::Uptime;
    use crate::procfs::parsers::{Parse, TokenParser};

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
