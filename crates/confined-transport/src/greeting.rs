//! The first thing a worker says: its magic, and what the kernel actually granted it.
//!
//! **The greeting is a report, never a promise.** A kernel can refuse what a build offers, so
//! what crosses here is [`pdf_sandbox::lockdown::Confinement`] as
//! [`pdf_sandbox::lockdown::apply_for`] returned it — and a host that believed itself confined
//! and was not is the failure the whole arrangement exists to prevent.
//!
//! The magic is the caller's, eight bytes, and it is what stops a host and a worker from
//! different builds — or from different *protocols*, now that two of them share this wire — from
//! talking to each other. The cheapest place to find that out is the first thing either says.

use pdf_sandbox::lockdown::{Confinement, LandlockLevel, SystemCalls};

/// Length of a greeting: the magic, the Landlock level, the memory ceiling, and which mechanism
/// holds the worker's operations — none, seccomp or Seatbelt.
pub const LEN: usize = 8 + 1 + 8 + 1;

/// Encodes a worker's greeting under `magic`.
#[must_use]
pub fn encode(magic: &[u8; 8], confinement: Confinement) -> [u8; LEN] {
    let mut greeting = [0u8; LEN];
    let (stamp, rest) = greeting.split_at_mut(8);
    stamp.copy_from_slice(magic);
    let (level, rest) = rest.split_at_mut(1);
    level[0] = match confinement.landlock {
        LandlockLevel::Enforced => 2,
        LandlockLevel::Partial => 1,
        LandlockLevel::Unavailable => 0,
    };
    let (limit, filtered) = rest.split_at_mut(8);
    limit.copy_from_slice(&confinement.address_space_limit.to_be_bytes());
    filtered[0] = match confinement.system_calls {
        SystemCalls::Unfiltered => 0,
        SystemCalls::Filtered => 1,
        SystemCalls::Seatbelt => 2,
    };
    greeting
}

/// Reads a greeting written under `magic`, or `None` where it is not one.
#[must_use]
pub fn parse(magic: &[u8; 8], greeting: &[u8; LEN]) -> Option<Confinement> {
    let (stamp, rest) = greeting.split_at(8);
    if stamp != magic {
        return None;
    }
    let (level, rest) = rest.split_at(1);
    let landlock = match level.first()? {
        2 => LandlockLevel::Enforced,
        1 => LandlockLevel::Partial,
        0 => LandlockLevel::Unavailable,
        _ => return None,
    };
    let (limit, filtered) = rest.split_at(8);
    let bytes: [u8; 8] = limit.try_into().ok()?;
    let system_calls = match filtered.first()? {
        2 => SystemCalls::Seatbelt,
        1 => SystemCalls::Filtered,
        0 => SystemCalls::Unfiltered,
        _ => return None,
    };
    Some(Confinement {
        landlock,
        address_space_limit: u64::from_be_bytes(bytes),
        system_calls,
    })
}

#[cfg(test)]
mod tests {
    use pdf_sandbox::lockdown::{Confinement, LandlockLevel, SystemCalls};

    use super::{LEN, encode, parse};

    const MINE: &[u8; 8] = b"TESTMGC1";
    const THEIRS: &[u8; 8] = b"TESTMGC2";

    #[test]
    fn a_greeting_carries_all_three_facts() {
        let confinement = Confinement {
            landlock: LandlockLevel::Partial,
            address_space_limit: 4 << 30,
            system_calls: SystemCalls::Filtered,
        };
        assert_eq!(parse(MINE, &encode(MINE, confinement)), Some(confinement));
    }

    /// A worker in a Seatbelt profile is read back as one, and not as a filtered or an unconfined
    /// one: the three mechanisms are three bytes (ADR 2864).
    #[test]
    fn a_seatbelt_greeting_is_read_as_seatbelt() {
        let confinement = Confinement {
            landlock: LandlockLevel::Unavailable,
            address_space_limit: 96 << 20,
            system_calls: SystemCalls::Seatbelt,
        };
        let parsed = parse(MINE, &encode(MINE, confinement));
        assert_eq!(parsed, Some(confinement));
        assert!(parsed.is_some_and(|parsed| parsed.is_enforced()));
    }

    /// **The property two protocols on one wire depend on**: a worker speaking the other one is
    /// refused at its first nine bytes rather than answering a question it will misread.
    #[test]
    fn a_greeting_under_another_magic_is_not_a_greeting() {
        let confinement = Confinement {
            landlock: LandlockLevel::Enforced,
            address_space_limit: 0,
            system_calls: SystemCalls::Unfiltered,
        };
        assert_eq!(parse(MINE, &encode(THEIRS, confinement)), None);
    }

    #[test]
    fn noise_is_not_a_greeting() {
        let mut noise = [0u8; LEN];
        noise[0] = b'X';
        assert_eq!(parse(MINE, &noise), None);
    }
}
