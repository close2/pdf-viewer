//! What the device keeps of a page between frames, charged against one figure the device states
//! (viewer ADR 2030).
//!
//! A retained encode's instance streams stay on the device after the frame that made them
//! (viewer ADR 1993), so that a replay sends none of them again. That is device memory held while
//! the page is shown rather than for one frame, and a frame's own budget does not see it: the frame
//! that made the streams was admitted, and every frame after it allocates beside them. So the
//! streams are charged here, against
//! [`Limits::max_resident_bytes`](crate::device::Limits::max_resident_bytes), and a charge the
//! budget will not take is a refusal that names its three numbers — after which the encode's
//! streams are made per frame, as they were before they stayed, and the page is drawn exactly the
//! same.
//!
//! **The figure is the frame's ceiling unless the host asks for less**, because the adapter states
//! no memory size (`wgpu` reports none, ADR 1823 section 3) and the one figure it does state that
//! bounds a held stream is the largest buffer it makes and binds. Held to that, what the device
//! keeps of the pages it has been asked to retain is never more than what it admits one frame to
//! allocate, and the host that lowers either figure lowers what is kept.
//!
//! **A charge is a value that gives its bytes back when it is dropped**, so the budget is right by
//! construction rather than by every caller remembering a refund: the charge lives in the retained
//! encode's cell beside the buffers it pays for, and the encode's replacement, its `forget`, a new
//! scene or the handle's own drop each return it.

use std::fmt;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use super::KeptInstances;

/// The device's residency budget: the bytes of retained instance streams held now, and the
/// ceiling they are held to.
#[derive(Debug)]
pub(crate) struct ResidencyBudget {
    /// Shared with every [`ResidentCharge`], which is how a charge dropped with a handle the
    /// device never sees again still gives its bytes back to this device.
    held: Arc<AtomicU64>,
    limit: u64,
}

impl ResidencyBudget {
    /// A budget holding nothing, at the device's ceiling.
    pub(crate) fn new(limit: u64) -> Self {
        Self {
            held: Arc::new(AtomicU64::new(0)),
            limit,
        }
    }

    /// Bytes held between frames now.
    pub(crate) fn in_use(&self) -> u64 {
        self.held.load(Ordering::Relaxed)
    }

    /// Count, then admit: `bytes` are charged only if the total stays within the ceiling.
    ///
    /// The compare-exchange loop is `resources::budget`'s, for its reason: a `fetch_add` and a
    /// check after it would transiently over-count and could refuse a charge that fits.
    pub(crate) fn charge(&self, bytes: u64) -> Result<ResidentCharge, ResidencyRefused> {
        let mut in_use = self.held.load(Ordering::Relaxed);
        loop {
            let needed = in_use.saturating_add(bytes);
            if needed > self.limit {
                return Err(ResidencyRefused {
                    needed,
                    in_use,
                    budget: self.limit,
                });
            }
            match self.held.compare_exchange_weak(
                in_use,
                needed,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => {
                    return Ok(ResidentCharge {
                        held: Arc::clone(&self.held),
                        bytes,
                    });
                }
                Err(actual) => in_use = actual,
            }
        }
    }
}

/// Bytes charged against a device's residency budget, given back when this is dropped.
///
/// Neither `Clone` nor `Copy`: a second copy would give the same bytes back twice.
#[derive(Debug)]
pub(crate) struct ResidentCharge {
    held: Arc<AtomicU64>,
    bytes: u64,
}

impl Drop for ResidentCharge {
    fn drop(&mut self) {
        // Saturating, as the resource budget's refund is: a refund past what is held is
        // arithmetic already gone wrong, and wrapping would make every later charge refuse.
        let bytes = self.bytes;
        // `fetch_update`'s closure always answers `Some`, so the update cannot be declined and
        // there is no `Err` to handle.
        let _ = self
            .held
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |held| {
                Some(held.saturating_sub(bytes))
            });
    }
}

/// Holding a retained encode's instance streams between frames would have taken the device past
/// its residency budget, so they are made per frame instead (viewer ADR 2030).
///
/// Not an error: the frame is drawn, byte for byte the frame a resident encode draws, and what it
/// costs is the transfer a replay would have saved. A host reads it from
/// [`RetainedScene::residency_refused`](crate::retained::RetainedScene::residency_refused) and
/// says it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResidencyRefused {
    /// What the bytes held between frames would have come to.
    pub needed: u64,
    /// What was held when the charge was refused.
    pub in_use: u64,
    /// The device's ceiling,
    /// [`Limits::max_resident_bytes`](crate::device::Limits::max_resident_bytes).
    pub budget: u64,
}

impl fmt::Display for ResidencyRefused {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "the page's instance streams are sent every frame rather than kept on the device: \
             keeping them would hold {} bytes against the device's residency budget of {} \
             ({} held already)",
            self.needed, self.budget, self.in_use
        )
    }
}

/// What a retained encode's cell holds once its first frame has asked to keep the streams.
#[derive(Debug)]
pub(crate) enum Residency {
    /// The streams, resident, and the charge that pays for them.
    Kept {
        streams: KeptInstances,
        /// Held for its `Drop`, which gives the bytes back with the streams.
        #[expect(dead_code, reason = "held for its drop, which is the refund")]
        charge: ResidentCharge,
    },
    /// The budget would not take them: every frame of this encode makes its own.
    Refused(ResidencyRefused),
}

impl Residency {
    /// Device bytes held between frames for this encode.
    pub(crate) fn bytes(&self) -> u64 {
        match self {
            Self::Kept { streams, .. } => streams.bytes(),
            Self::Refused(_) => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ResidencyBudget;

    /// A charge past the ceiling names its numbers and charges nothing; a charge dropped gives
    /// its bytes back, so the refused one then fits.
    #[test]
    fn a_charge_past_the_ceiling_is_refused_by_name_and_a_dropped_one_is_given_back() {
        let budget = ResidencyBudget::new(100);
        let first = budget.charge(60);
        assert!(first.is_ok());
        assert_eq!(budget.in_use(), 60);
        let refused = budget.charge(50).map(|_| ());
        assert_eq!(
            refused,
            Err(super::ResidencyRefused {
                needed: 110,
                in_use: 60,
                budget: 100
            })
        );
        assert_eq!(budget.in_use(), 60, "a refusal charges nothing");
        drop(first);
        assert_eq!(budget.in_use(), 0);
        let second = budget.charge(50);
        assert!(second.is_ok());
        assert_eq!(budget.in_use(), 50);
    }
}
