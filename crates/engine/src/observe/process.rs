//! CPU time and peak memory of the current process. The standard library exposes neither;
//! on Windows the values come from `GetProcessTimes` and `K32GetProcessMemoryInfo`
//! (source: windows-sys 0.61.2, `src/Windows/Win32/System/Threading/mod.rs` and
//! `src/Windows/Win32/System/ProcessStatus/mod.rs`).

/// Kernel plus user CPU time of this process in milliseconds, when the platform exposes it.
#[cfg(windows)]
pub fn cpu_time_ms() -> Option<u64> {
    use windows_sys::Win32::Foundation::FILETIME;
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, GetProcessTimes};
    let zero = FILETIME {
        dwLowDateTime: 0,
        dwHighDateTime: 0,
    };
    let (mut creation, mut exit, mut kernel, mut user) = (zero, zero, zero, zero);
    // SAFETY: the pseudo-handle from GetCurrentProcess is always valid, and every pointer
    // refers to a live local FILETIME for the duration of the call.
    // nosemgrep: rust.lang.security.unsafe-usage.unsafe-usage -- Windows process-counter API call; see SAFETY above
    let ok = unsafe {
        GetProcessTimes(
            GetCurrentProcess(),
            &mut creation,
            &mut exit,
            &mut kernel,
            &mut user,
        )
    };
    if ok == 0 {
        return None;
    }
    let to_100ns = |f: FILETIME| (u64::from(f.dwHighDateTime) << 32) | u64::from(f.dwLowDateTime);
    Some((to_100ns(kernel) + to_100ns(user)) / 10_000)
}

/// Peak working set of this process in mebibytes, when the platform exposes it.
#[cfg(windows)]
pub fn peak_memory_mb() -> Option<f64> {
    use windows_sys::Win32::System::ProcessStatus::{
        K32GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS,
    };
    use windows_sys::Win32::System::Threading::GetCurrentProcess;
    let mut counters = PROCESS_MEMORY_COUNTERS {
        cb: std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32,
        PageFaultCount: 0,
        PeakWorkingSetSize: 0,
        WorkingSetSize: 0,
        QuotaPeakPagedPoolUsage: 0,
        QuotaPagedPoolUsage: 0,
        QuotaPeakNonPagedPoolUsage: 0,
        QuotaNonPagedPoolUsage: 0,
        PagefileUsage: 0,
        PeakPagefileUsage: 0,
    };
    // SAFETY: the pseudo-handle is valid and `counters` is a live, correctly sized struct.
    // nosemgrep: rust.lang.security.unsafe-usage.unsafe-usage -- Windows process-counter API call; see SAFETY above
    let ok = unsafe { K32GetProcessMemoryInfo(GetCurrentProcess(), &mut counters, counters.cb) };
    if ok == 0 {
        return None;
    }
    Some(counters.PeakWorkingSetSize as f64 / (1024.0 * 1024.0))
}

#[cfg(not(windows))]
pub fn cpu_time_ms() -> Option<u64> {
    None
}

#[cfg(not(windows))]
pub fn peak_memory_mb() -> Option<f64> {
    None
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn measurements_are_available_on_windows() {
        assert!(cpu_time_ms().is_some());
        assert!(peak_memory_mb().unwrap() > 0.0);
    }
}
