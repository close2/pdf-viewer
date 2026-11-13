//! The two calls, on the one platform that has them.

#![expect(
    unsafe_code,
    reason = "the two foreign calls this crate exists to wrap, each with its contract beside it"
)]

use std::ffi::{CStr, CString, c_char};

// `sandbox_init` and `sandbox_free_error` are declared by `<sandbox.h>` and defined in
// `libsystem_sandbox`, which `libSystem` re-exports; `libc` does not bind them.
unsafe extern "C" {
    /// `int sandbox_init(const char *profile, uint64_t flags, char **errorbuf)`.
    ///
    /// With `flags` zero, `profile` is the text of a profile in the Sandbox Profile Language
    /// rather than the name of one of the built-in ones. On failure it returns -1 and points
    /// `errorbuf` at a message the caller frees with `sandbox_free_error`.
    fn sandbox_init(profile: *const c_char, flags: u64, errorbuf: *mut *mut c_char) -> i32;
    /// `void sandbox_free_error(char *errorbuf)`.
    fn sandbox_free_error(errorbuf: *mut c_char);
}

/// Why [`enter`] did not confine the calling process.
#[derive(Debug)]
pub enum EnterError {
    /// The profile text contains a NUL byte, so it cannot be handed to a C function whole.
    NulInProfile,
    /// The kernel refused the profile, with its own sentence — usually a syntax error in it.
    Refused(String),
}

impl std::fmt::Display for EnterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NulInProfile => f.write_str("the Seatbelt profile contains a NUL byte"),
            Self::Refused(why) => write!(f, "the Seatbelt profile was refused: {why}"),
        }
    }
}

impl std::error::Error for EnterError {}

/// Confines the calling process to `profile`, a profile in the Sandbox Profile Language.
///
/// **There is no way back**: every thread of the process, and every process it starts, is held to
/// the profile from the moment this returns `Ok`. A profile that begins `(deny default)` denies
/// every operation it does not name, the filesystem and the network included; what an already-open
/// descriptor reads and writes is not an operation the profile is asked about.
///
/// # Errors
///
/// [`EnterError::NulInProfile`] where the text cannot cross to C, and [`EnterError::Refused`] with
/// the kernel's sentence where the profile was not accepted. A caller that gets either is not
/// confined and must not go on to the work it meant to confine.
pub fn enter(profile: &str) -> Result<(), EnterError> {
    let text = CString::new(profile).map_err(|_| EnterError::NulInProfile)?;
    let mut error: *mut c_char = std::ptr::null_mut();
    // SAFETY: `text` is a NUL-terminated string that outlives the call, `error` is a valid place
    // for the function to write one pointer, and flags zero asks for nothing but the profile text
    // to be read. The function keeps neither pointer.
    let status = unsafe { sandbox_init(text.as_ptr(), 0, &raw mut error) };
    if status == 0 {
        return Ok(());
    }
    let sentence = if error.is_null() {
        String::from("no reason was given")
    } else {
        // SAFETY: on failure `sandbox_init` points `error` at a NUL-terminated string it
        // allocated, which stays valid until it is handed to `sandbox_free_error` just below.
        let said = unsafe { CStr::from_ptr(error) }
            .to_string_lossy()
            .into_owned();
        // SAFETY: `error` is the buffer `sandbox_init` allocated, freed once and never read again.
        unsafe { sandbox_free_error(error) };
        said
    };
    Err(EnterError::Refused(sentence))
}

/// What one process is charged for, read from outside it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Usage {
    /// The physical footprint in bytes: what macOS charges the process for memory, compressed and
    /// swapped pages included, and the number Activity Monitor shows as its memory.
    pub footprint: u64,
    /// When the process started, in Mach absolute time units. Constant for the life of a process
    /// and different for the next one to be given the same identifier, which is what lets a
    /// caller holding only an identifier tell that the process is still the one it meant.
    pub started: u64,
}

/// Reads the [`Usage`] of the process `pid`.
///
/// # Errors
///
/// The operating system's error: `ESRCH` once the process is gone (a zombie that has not been
/// reaped still answers), `EPERM` for a process of another user.
pub fn usage(pid: u32) -> std::io::Result<Usage> {
    let pid = i32::try_from(pid).map_err(|_| std::io::Error::from_raw_os_error(libc::ESRCH))?;
    let mut record = std::mem::MaybeUninit::<libc::rusage_info_v2>::zeroed();
    // SAFETY: `RUSAGE_INFO_V2` asks for exactly a `rusage_info_v2`, and `record` is one, zeroed
    // and writable for the length of the call. The function's `rusage_info_t *` parameter is a
    // `void **` by declaration and a pointer to the record by contract, which is how `<libproc.h>`'s
    // own callers pass it.
    let status = unsafe {
        libc::proc_pid_rusage(
            pid,
            libc::RUSAGE_INFO_V2,
            record.as_mut_ptr().cast::<libc::rusage_info_t>(),
        )
    };
    if status != 0 {
        return Err(std::io::Error::last_os_error());
    }
    // SAFETY: the call returned zero, so it filled the record; and every field of it is an integer
    // or an array of them, for which the zeroed bytes were already a valid value.
    let record = unsafe { record.assume_init() };
    Ok(Usage {
        footprint: record.ri_phys_footprint,
        started: record.ri_proc_start_abstime,
    })
}

#[cfg(test)]
mod tests {
    use super::usage;

    /// This process has a footprint, and two readings of it agree about when it started.
    #[test]
    fn this_process_reads_as_itself() {
        let me = std::process::id();
        let first = usage(me).unwrap();
        // `black_box`, or an optimised build removes a buffer nothing reads, and its pages with it.
        let held = std::hint::black_box(vec![1u8; 64 << 20]);
        let second = usage(me).unwrap();
        assert!(first.footprint > 0);
        assert_eq!(first.started, second.started);
        assert!(
            second.footprint >= first.footprint.saturating_add(32 << 20),
            "64 MiB written did not show in the footprint: {first:?} then {second:?}"
        );
        drop(held);
    }

    /// A process that is gone is an error rather than a reading.
    #[test]
    fn a_process_that_is_gone_is_an_error() {
        let mut child = std::process::Command::new("/usr/bin/true").spawn().unwrap();
        let pid = child.id();
        child.wait().unwrap();
        assert!(usage(pid).is_err());
    }
}
