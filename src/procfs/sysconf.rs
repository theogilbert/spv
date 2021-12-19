use libc::{sysconf, _SC_CLK_TCK};

use crate::procfs::ProcfsError;

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
    use crate::procfs::sysconf::clock_ticks;

    #[test]
    fn test_should_get_clock_ticks() {
        assert!(clock_ticks().is_ok());
    }
}
