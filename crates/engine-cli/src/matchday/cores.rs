//! The machine's usable cores and the background threads' priority.
//!
//! `std::thread::available_parallelism` reads `GetSystemInfo` on Windows and so counts every
//! logical processor even when the process is limited to fewer by an affinity mask
//! (source: the installed std, `library/std/src/sys/thread/windows.rs:81-89`), so on Windows
//! the mask is read directly. On Linux std reads `sched_getaffinity` and is exact
//! (`library/std/src/sys/thread/unix.rs:154-175`). std has no call that sets a thread's
//! priority, so the platform calls are used.

/// The logical cores this process may run on, at least 1.
pub fn usable() -> usize {
    #[cfg(windows)]
    {
        if let Some(mask) = windows::process_mask() {
            return (mask.count_ones() as usize).max(1);
        }
    }
    std::thread::available_parallelism().map_or(1, std::num::NonZero::get)
}

/// Lowers the calling thread below normal priority, so the player's match and the page keep
/// the machine first. A refusal is logged once and play goes on.
pub fn lower_priority() {
    if let Err(code) = lower_this_thread() {
        static LOGGED: std::sync::Once = std::sync::Once::new();
        LOGGED.call_once(|| {
            tracing::warn!(signal = "matchday.priority_refused", error.code = code);
        });
    }
}

/// Limits this process to its first `n` usable logical cores. A test seam for the timing run.
pub fn limit_to(n: usize) -> Result<(), String> {
    if n == 0 {
        return Err("at least one core is needed".into());
    }
    #[cfg(windows)]
    {
        windows::limit_to(n)
    }
    #[cfg(target_os = "linux")]
    {
        linux::limit_to(n)
    }
    #[cfg(not(any(windows, target_os = "linux")))]
    {
        Err("limiting the cores is not supported on this system".into())
    }
}

#[cfg(windows)]
fn lower_this_thread() -> Result<(), i64> {
    windows::lower_this_thread()
}

#[cfg(target_os = "linux")]
fn lower_this_thread() -> Result<(), i64> {
    linux::lower_this_thread()
}

#[cfg(not(any(windows, target_os = "linux")))]
fn lower_this_thread() -> Result<(), i64> {
    Ok(())
}

#[cfg(windows)]
mod windows {
    use windows_sys::Win32::System::Threading::{
        GetCurrentProcess, GetCurrentThread, GetProcessAffinityMask, SetProcessAffinityMask,
        SetThreadPriority, THREAD_PRIORITY_BELOW_NORMAL,
    };

    /// The process affinity mask, or `None` when Windows refuses to say.
    pub fn process_mask() -> Option<usize> {
        let mut process = 0usize;
        let mut system = 0usize;
        // SAFETY: the pseudo handle of the current process is always valid, and both
        // out-pointers point at live locals.
        let ok = unsafe { GetProcessAffinityMask(GetCurrentProcess(), &mut process, &mut system) };
        (ok != 0 && process != 0).then_some(process)
    }

    pub fn lower_this_thread() -> Result<(), i64> {
        // SAFETY: the pseudo handle of the current thread is always valid.
        let ok = unsafe { SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_BELOW_NORMAL) };
        if ok == 0 {
            return Err(i64::from(
                std::io::Error::last_os_error().raw_os_error().unwrap_or(0),
            ));
        }
        Ok(())
    }

    pub fn limit_to(n: usize) -> Result<(), String> {
        let mask = process_mask().ok_or("the process affinity mask cannot be read")?;
        let mut kept = 0usize;
        let mut left = n;
        for bit in 0..usize::BITS {
            if left == 0 {
                break;
            }
            if mask & (1 << bit) != 0 {
                kept |= 1 << bit;
                left -= 1;
            }
        }
        if left > 0 {
            return Err(format!(
                "the process may run on {} logical cores, fewer than {n}",
                mask.count_ones()
            ));
        }
        // SAFETY: the pseudo handle of the current process is always valid, and `kept` is a
        // subset of the mask Windows reported.
        if unsafe { SetProcessAffinityMask(GetCurrentProcess(), kept) } == 0 {
            return Err(format!(
                "the affinity mask could not be set: {}",
                std::io::Error::last_os_error()
            ));
        }
        Ok(())
    }
}

#[cfg(target_os = "linux")]
mod linux {
    /// Nice value of a background thread: below normal, never above it.
    const NICE: libc::c_int = 10;

    pub fn lower_this_thread() -> Result<(), i64> {
        // SAFETY: gettid has no preconditions; setpriority on the calling thread's id
        // changes only that thread's nice value.
        let rc = unsafe {
            let tid = libc::gettid();
            libc::setpriority(libc::PRIO_PROCESS, tid as libc::id_t, NICE)
        };
        if rc != 0 {
            return Err(i64::from(
                std::io::Error::last_os_error().raw_os_error().unwrap_or(0),
            ));
        }
        Ok(())
    }

    pub fn limit_to(n: usize) -> Result<(), String> {
        // SAFETY: `set` is a zeroed cpu_set_t on the stack; the CPU_* helpers only touch it,
        // and pid 0 names the calling process.
        unsafe {
            let mut current: libc::cpu_set_t = std::mem::zeroed();
            if libc::sched_getaffinity(0, std::mem::size_of::<libc::cpu_set_t>(), &mut current) != 0
            {
                return Err(format!(
                    "the affinity cannot be read: {}",
                    std::io::Error::last_os_error()
                ));
            }
            let mut set: libc::cpu_set_t = std::mem::zeroed();
            let mut left = n;
            for cpu in 0..libc::CPU_SETSIZE as usize {
                if left == 0 {
                    break;
                }
                if libc::CPU_ISSET(cpu, &current) {
                    libc::CPU_SET(cpu, &mut set);
                    left -= 1;
                }
            }
            if left > 0 {
                return Err(format!(
                    "the process may run on {} logical cores, fewer than {n}",
                    libc::CPU_COUNT(&current)
                ));
            }
            if libc::sched_setaffinity(0, std::mem::size_of::<libc::cpu_set_t>(), &set) != 0 {
                return Err(format!(
                    "the affinity could not be set: {}",
                    std::io::Error::last_os_error()
                ));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_process_has_at_least_one_usable_core() {
        assert!(usable() >= 1);
    }

    #[test]
    fn a_worker_thread_can_lower_its_priority() {
        std::thread::spawn(|| {
            assert!(lower_this_thread().is_ok());
        })
        .join()
        .unwrap();
    }

    #[test]
    fn no_core_is_refused() {
        assert!(limit_to(0).is_err());
    }
}
