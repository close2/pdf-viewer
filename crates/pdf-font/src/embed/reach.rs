//! Which subroutines a kept charstring reaches, so that a `CFF ` subset keeps them (ADR 1449).
//!
//! A Type 2 charstring calls a subroutine by pushing its number and then `callsubr` (local) or
//! `callgsubr` (global); the operand is the subroutine's index less a bias that Adobe Technical
//! Note #5177 section 4.7 derives from how many subroutines the INDEX holds, and a subroutine may
//! call further ones up to ten deep. So the closure is found by running each kept charstring the
//! way an interpreter does — one operand stack shared across the calls, the stem hints counted so
//! that a `hintmask`'s data bytes are stepped over rather than read as operators — and noting
//! every subroutine entered.
//!
//! The walk follows only what it can know exactly. The arithmetic and storage operators of the
//! note's section 4.5 can compute a subroutine number rather than push it, and a reserved
//! operator has no meaning at all, so either answers [`Walk::Unfollowable`] and the writer keeps
//! every subroutine: a larger subset, never a glyph that draws wrong. An `endchar` with the
//! accented-character arguments of the note's Appendix C names two more glyphs by standard
//! code, which a CID-keyed program — what the subset is written as — cannot carry, so it answers
//! [`Walk::Seac`] and the face is written whole.

use std::collections::BTreeSet;

use crate::cff::{bias, small_operand};

/// How deep subroutine calls nest before the walk stops following: Adobe Technical Note #5177's
/// Appendix B limit.
const MAX_DEPTH: usize = 10;

/// What walking one charstring found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Walk {
    /// Every subroutine the charstring reaches was noted.
    Followed,
    /// The charstring does something whose subroutine numbers the walk cannot know.
    Unfollowable,
    /// The charstring ends in the accented-character form of `endchar`.
    Seac,
}

/// The subroutines one Font DICT's charstrings may call, and those reached so far.
pub(super) struct Reached<'a> {
    /// The Global Subr INDEX.
    pub(super) global: &'a [&'a [u8]],
    /// The Local Subr INDEX of the Font DICT the charstring is read against.
    pub(super) local: &'a [&'a [u8]],
    /// The global subroutines entered, by index.
    pub(super) global_entered: BTreeSet<usize>,
    /// The local subroutines entered, by index.
    pub(super) local_entered: BTreeSet<usize>,
}

/// Why a subroutine's run ended.
enum Flow {
    /// `return`, or the end of its bytes.
    Returned,
    /// `endchar`: the glyph is finished, whatever called this.
    Ended,
}

/// The interpreter state one charstring's walk carries through its calls.
struct Machine {
    /// The operand stack, whose values are shared between a caller and the subroutines it calls.
    stack: Vec<f64>,
    /// Stem hints declared so far, which fix how many data bytes a `hintmask` has.
    stems: usize,
}

impl Reached<'_> {
    /// Walks one charstring, noting what it reaches.
    pub(super) fn walk(&mut self, charstring: &[u8]) -> Walk {
        let mut machine = Machine {
            stack: Vec::new(),
            stems: 0,
        };
        match self.run(&mut machine, charstring, 0) {
            Ok(_) => Walk::Followed,
            Err(stop) => stop,
        }
    }

    /// Runs `code` from its first byte, following calls.
    fn run(&mut self, machine: &mut Machine, code: &[u8], depth: usize) -> Result<Flow, Walk> {
        let mut at = 0usize;
        while let Some(&b0) = code.get(at) {
            at = at.saturating_add(1);
            match b0 {
                // Operands (Adobe Technical Note #5177, section 3.2).
                32..=254 => {
                    let b1 = code.get(at).copied().unwrap_or(0);
                    let (value, width) = small_operand(b0, b1);
                    #[expect(
                        clippy::cast_precision_loss,
                        reason = "a two-byte operand is at most 1131 in magnitude"
                    )]
                    machine.stack.push(value as f64);
                    at = at.saturating_add(width.saturating_sub(1));
                }
                28 => {
                    let bytes = code
                        .get(at..at.saturating_add(2))
                        .ok_or(Walk::Unfollowable)?;
                    machine
                        .stack
                        .push(f64::from(i16::from_be_bytes([bytes[0], bytes[1]])));
                    at = at.saturating_add(2);
                }
                255 => {
                    let bytes = code
                        .get(at..at.saturating_add(4))
                        .ok_or(Walk::Unfollowable)?;
                    let fixed = i32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
                    machine.stack.push(f64::from(fixed) / 65536.0);
                    at = at.saturating_add(4);
                }
                // callsubr, callgsubr.
                10 | 29 => {
                    let global = b0 == 29;
                    let operand = machine.stack.pop().ok_or(Walk::Unfollowable)?;
                    let index = self.index(operand, global).ok_or(Walk::Unfollowable)?;
                    if depth >= MAX_DEPTH {
                        return Err(Walk::Unfollowable);
                    }
                    let subr = if global {
                        self.global_entered.insert(index);
                        self.global.get(index)
                    } else {
                        self.local_entered.insert(index);
                        self.local.get(index)
                    }
                    .copied()
                    .ok_or(Walk::Unfollowable)?;
                    if let Flow::Ended = self.run(machine, subr, depth.saturating_add(1))? {
                        return Ok(Flow::Ended);
                    }
                }
                // return.
                11 => return Ok(Flow::Returned),
                // endchar: four operands beyond a width are Appendix C's accented character.
                14 => {
                    return if machine.stack.len() >= 4 {
                        Err(Walk::Seac)
                    } else {
                        Ok(Flow::Ended)
                    };
                }
                // hstem, vstem, hstemhm, vstemhm: pairs of operands, each a stem; an odd one out
                // is the width.
                1 | 3 | 18 | 23 => {
                    machine.stems = machine.stems.saturating_add(machine.stack.len() / 2);
                    machine.stack.clear();
                }
                // hintmask, cntrmask: operands still on the stack are an implied vstem, and a
                // bit per stem follows the operator, rounded up to whole bytes.
                19 | 20 => {
                    machine.stems = machine.stems.saturating_add(machine.stack.len() / 2);
                    machine.stack.clear();
                    at = at.saturating_add(machine.stems.saturating_add(7) / 8);
                }
                // The two-byte operators: the four flex forms and the deprecated dotsection
                // clear the stack; every other one is arithmetic, storage or reserved.
                12 => {
                    let b1 = code.get(at).copied().ok_or(Walk::Unfollowable)?;
                    at = at.saturating_add(1);
                    if matches!(b1, 0 | 34..=37) {
                        machine.stack.clear();
                    } else {
                        return Err(Walk::Unfollowable);
                    }
                }
                // The path construction operators, each of which clears the stack.
                4..=8 | 21 | 22 | 24..=27 | 30 | 31 => machine.stack.clear(),
                // Reserved: 0, 2, 9, 13, 15, 16, 17.
                _ => return Err(Walk::Unfollowable),
            }
        }
        Ok(Flow::Returned)
    }

    /// The INDEX position a call's operand names, the bias added back.
    fn index(&self, operand: f64, global: bool) -> Option<usize> {
        if operand.fract() != 0.0 {
            return None;
        }
        let count = if global {
            self.global.len()
        } else {
            self.local.len()
        };
        #[expect(
            clippy::cast_possible_truncation,
            reason = "an integral operand, which a charstring states in at most 32 bits"
        )]
        let operand = operand as i64;
        usize::try_from(operand.checked_add(bias(count))?).ok()
    }
}
