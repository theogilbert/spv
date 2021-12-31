use libc::{getrlimit64, rlimit64, sysconf, RLIMIT_NOFILE, _SC_CLK_TCK};

use crate::procfs::ProcfsError;

/// Returns the clock ticks value of the system
pub(crate) fn clock_ticks() -> Result<u64, ProcfsError> {
    let clock_ticks_value;

    unsafe {
        clock_ticks_value = sysconf(_SC_CLK_TCK);
    }

    match clock_ticks_value {
        -1 => Err(ProcfsError::SysconfError),
        _ => Ok(clock_ticks_value as u64),
    }
}

#[cfg(test)]
mod test_clock_ticks {
    use crate::procfs::libc::clock_ticks;

    #[test]
    fn test_should_get_clock_ticks() {
        assert!(clock_ticks().is_ok());
    }
}

/// Returns a value one greater than the maximum amount of files that this process can open at once
pub fn open_file_limit() -> Result<u64, ProcfsError> {
    let return_value;
    let mut rlimit = rlimit64 {
        rlim_cur: 0,
        rlim_max: 0,
    };

    unsafe {
        return_value = getrlimit64(RLIMIT_NOFILE, &mut rlimit);
    }

    match return_value {
        0 => Ok(rlimit.rlim_cur),
        -1 => Err(ProcfsError::RLimitError),
        _ => unreachable!(),
    }
}

#[cfg(test)]
mod test_rlimit_nofile {
    use crate::procfs::libc::open_file_limit;

    #[test]
    fn test_should_get_rlimit_nofile() {
        assert!(open_file_limit().is_ok());
    }
}
