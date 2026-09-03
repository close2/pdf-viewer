//! Turning an address-space ceiling into a message budget.
//!
//! **Every term is read from somewhere or measured, and none is picked.** The confinement
//! installs `RLIMIT_AS`; what a worker needs to know is how large a message it can afford to
//! *hold*, which is a different number and is derived from that one.

use std::num::NonZeroU64;

/// What a confined process still has to grow by after its baseline is taken, in bytes.
///
/// **The baseline is measured at the one moment it can be**, which is before the confinement —
/// and a worker is not finished growing then. Between that moment and the first page it builds
/// `rayon`'s pool, and on `glibc` a thread costs its stack plus a 64 MiB arena of address space.
/// Measured on this machine (ADR 0597): `VmSize` is 14.1 MB when the baseline is taken and
/// 82.0 MB once a page has been drawn, so what the baseline misses is 68 MB.
///
/// A hundred and twenty-eight mebibytes is that measurement rounded up to the next power of two,
/// which is 3% of the interpreter's ceiling — and rounding *up* is the direction to be wrong in,
/// because being wrong the other way is the abort this whole arithmetic exists to prevent.
pub const SETTLING_ALLOWANCE: u64 = 128 << 20;

/// The largest message a ceiling leaves room for, in bytes.
///
/// - `ceiling` is what the kernel installed and what the greeting reports. Zero means no ceiling,
///   and then there is no budget either.
/// - `already` is the process's own address space *before* the confinement, from
///   [`address_space_in_use`]. It has to be read there because a confined process has no
///   filesystem: `openat` is not on the allow-list, so that is the only moment the question can
///   be asked.
/// - [`SETTLING_ALLOWANCE`] is what it grows by after that.
/// - `reserved` is what the work itself will claim beside the message — a page's pixels for an
///   interpreter, an answer's own buffer for a generator. It is subtracted because the document
///   is still held when that allocation happens.
/// - `copies` is how many copies of the message live at once at the moment the peak is reached.
///   A count rather than a number, because a message that lives in no copies is not a message and
///   dividing by it is the one arithmetic here that could go wrong.
///
/// The result is capped at [`crate::frame::MAX_MESSAGE`], which is what the wire would carry
/// anyway.
#[must_use]
pub fn message_budget(ceiling: u64, already: u64, reserved: u64, copies: NonZeroU64) -> u64 {
    if ceiling == 0 {
        return u64::MAX;
    }
    ceiling
        .saturating_sub(already)
        .saturating_sub(SETTLING_ALLOWANCE)
        .saturating_sub(reserved)
        .checked_div(copies.get())
        .unwrap_or(0)
        .min(crate::frame::MAX_MESSAGE)
}

/// This process's address space in bytes, or 0 where it cannot be read.
///
/// `VmSize` rather than `VmRSS`, because `RLIMIT_AS` is compared against the address space and a
/// budget derived from the resident set would be derived from the wrong counter. **Call it before
/// the confinement and never after it**: `/proc/self/status` is a file, and a confined process has
/// none.
#[must_use]
pub fn address_space_in_use() -> u64 {
    let Ok(status) = std::fs::read_to_string("/proc/self/status") else {
        return 0;
    };
    status
        .lines()
        .find(|line| line.starts_with("VmSize:"))
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|value| value.parse::<u64>().ok())
        .map_or(0, |kilobytes| kilobytes.saturating_mul(1024))
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU64;

    use super::{SETTLING_ALLOWANCE, message_budget};
    use crate::frame::MAX_MESSAGE;

    /// Two copies of a message live at once at the peak (ADR 0597), which is what the tests use.
    const TWO: NonZeroU64 = NonZeroU64::new(2).unwrap();

    /// A budget is what is left after everything that is not the message.
    #[test]
    fn a_budget_is_the_ceiling_less_what_is_not_the_message() {
        const CEILING: u64 = 4 << 30;
        const ALREADY: u64 = 16 << 20;
        const RESERVED: u64 = 1 << 30;
        let budget = message_budget(CEILING, ALREADY, RESERVED, TWO);
        assert_eq!(
            budget,
            (CEILING - ALREADY - SETTLING_ALLOWANCE - RESERVED) / 2
        );
        assert!(budget < MAX_MESSAGE);
    }

    /// No ceiling is no budget, and the wire's own bound is what is left.
    #[test]
    fn no_ceiling_leaves_only_the_wires_own_bound() {
        assert_eq!(message_budget(0, 0, 0, TWO), u64::MAX);
        assert_eq!(message_budget(u64::MAX, 0, 0, TWO), MAX_MESSAGE);
    }

    /// A ceiling smaller than what is subtracted from it leaves nothing, rather than wrapping.
    #[test]
    fn a_ceiling_too_small_to_hold_anything_leaves_nothing() {
        assert_eq!(message_budget(1 << 20, 0, 0, TWO), 0);
    }
}
