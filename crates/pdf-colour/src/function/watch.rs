//! Which type 4 functions a span of work parsed, so that whoever ran the span can ask them
//! afterwards whether they met one of ISO 32000-2 §7.10.5.3's errors.
//!
//! # Why a scope, and not a walk over the resources
//!
//! §7.10.5.3 states three errors a program meets only for some inputs:
//!
//! > It shall be an error for the number of remaining operands to differ from the number of
//! > output variables specified by Range or for any of them to be objects other than numbers.
//!
//! > … it shall be an error to overflow the stack.
//!
//! The evaluator records each on the function ([`super::EvaluationFault`]), and a function reaches
//! a page through a dozen kinds of resource — a `Separation` or `DeviceN` tint transform, the base
//! of an `Indexed` space, an image's space, a `/TR`, a `/BG` or a `/UCR`, a shading's group, a
//! mesh's parametric colours, a soft mask's group space. ADR 2833 proposed giving each kind a walk
//! and having the interpreter ask every resource it resolved; a resource resolved and not kept is
//! one the interpreter would have to remember at each of the places that resolve one, and a place
//! missed is a fault that is never said (trap 5). What every route shares is
//! [`super::Function::parse`], so the cell is registered there, with the scope a caller opened
//! around its work, and the caller asks the scope once at the end. ADR 2848 has the comparison.
//!
//! # What it does not see
//!
//! A function parsed on **another thread** than the one that opened the scope, and a function
//! parsed **before** the scope opened and evaluated inside it. Every parse an interpretation makes
//! runs on the thread that called it — its parallel sections convert colours, they do not parse
//! spaces — and no cache that outlives an interpretation holds a parsed function. A scope is
//! cheap to nest: an inner scope's cells are handed to the one around it when it closes, so both
//! see them.
//!
//! # What it costs
//!
//! One shared cell per type 4 function parsed, which is the only type that can fault; nothing per
//! evaluation. A cell whose function and every clone of it have been dropped can never change
//! again, so when the list doubles its bits are folded into one byte and the cell is let go — the
//! list is bounded by the type 4 functions alive at once, not by how many times a page parsed one.

use std::cell::RefCell;
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering::Relaxed};

use super::EvaluationFault;

/// The fewest cells a scope holds before it folds the settled ones away.
const FIRST_FOLD: usize = 64;

thread_local! {
    /// The scopes open on this thread, innermost last.
    static OPEN: RefCell<Vec<Scope>> = const { RefCell::new(Vec::new()) };
}

/// One open scope: the cells still alive, and the bits of the ones already let go.
#[derive(Debug, Default)]
struct Scope {
    cells: Vec<Arc<AtomicU8>>,
    settled: u8,
    /// How long [`Self::cells`] may grow before the next fold.
    fold_at: usize,
}

impl Scope {
    fn push(&mut self, cell: Arc<AtomicU8>) {
        self.cells.push(cell);
        if self.cells.len() >= self.fold_at.max(FIRST_FOLD) {
            self.fold();
        }
    }

    /// Lets go of every cell only this scope still holds, keeping its bits.
    fn fold(&mut self) {
        let mut settled = self.settled;
        self.cells.retain(|cell| {
            // One strong count is this scope's own: the function and its clones are gone, so no
            // evaluation can set another bit and the byte read now is the cell's last word.
            if Arc::strong_count(cell) == 1 {
                settled |= cell.load(Relaxed);
                false
            } else {
                true
            }
        });
        self.settled = settled;
        self.fold_at = self.cells.len().saturating_mul(2);
    }
}

/// Adds a type 4 function's fault cell to the innermost scope open on this thread, if any.
pub(super) fn register(cell: &Arc<AtomicU8>) {
    OPEN.with(|open| {
        if let Some(scope) = open.borrow_mut().last_mut() {
            scope.push(Arc::clone(cell));
        }
    });
}

/// Pops the scope [`watching`] pushed, even where its work panics.
struct Close;

impl Drop for Close {
    fn drop(&mut self) {
        OPEN.with(|open| {
            open.borrow_mut().pop();
        });
    }
}

/// Runs `work` and hands back, beside its answer, every type 4 function it parsed on this thread.
///
/// The [`Watched`] keeps reading the functions' cells, so a function evaluated after `work`
/// returned — a §8.7.4.5.2 type 1 shading's grid, which a backend asks for at its device's
/// resolution — is seen by asking it again later.
pub fn watching<R>(work: impl FnOnce() -> R) -> (R, Watched) {
    OPEN.with(|open| open.borrow_mut().push(Scope::default()));
    let close = Close;
    let answer = work();
    let scope = OPEN.with(|open| {
        open.borrow_mut()
            .last_mut()
            .map(std::mem::take)
            .unwrap_or_default()
    });
    drop(close);
    OPEN.with(|open| {
        if let Some(outer) = open.borrow_mut().last_mut() {
            outer.settled |= scope.settled;
            for cell in &scope.cells {
                outer.push(Arc::clone(cell));
            }
        }
    });
    (
        answer,
        Watched {
            cells: scope.cells.into(),
            settled: scope.settled,
        },
    )
}

/// The type 4 functions a [`watching`] scope parsed, asked for the §7.10.5.3 errors they met.
#[derive(Clone, Default)]
pub struct Watched {
    cells: Arc<[Arc<AtomicU8>]>,
    settled: u8,
}

/// What a scope states is the faults its functions met, not how many cells held them: a page read
/// in pieces parses its functions in more places than the same page read whole, and the two state
/// the same page when they met the same faults.
impl std::fmt::Debug for Watched {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Watched")
            .field("faults", &self.faults())
            .finish()
    }
}

impl Watched {
    /// Every distinct fault any of the functions has met so far, in a fixed order.
    #[must_use]
    pub fn faults(&self) -> Vec<EvaluationFault> {
        let bits = self
            .cells
            .iter()
            .fold(self.settled, |bits, cell| bits | cell.load(Relaxed));
        EvaluationFault::ALL
            .into_iter()
            .filter(|fault| bits & fault.bit() != 0)
            .collect()
    }

    /// The functions both `self` and `other` watch, as one.
    #[must_use]
    pub fn joined(&self, other: &Self) -> Self {
        Self {
            cells: self
                .cells
                .iter()
                .chain(other.cells.iter())
                .cloned()
                .collect(),
            settled: self.settled | other.settled,
        }
    }

    /// How many functions are watched still alive, for a test that a scope saw what it parsed.
    #[must_use]
    pub fn len(&self) -> usize {
        self.cells.len()
    }

    /// Whether no function is watched still alive.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }
}
