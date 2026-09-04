//! ISO 32000-2 §7.3.3's reader-side answer, in one place.
//!
//! The clause states the rule and states it to the *writer*:
//!
//! > A real number shall not be present when an integer is expected.
//!
//! Nothing anywhere addresses the reader, so what a reader does with such a file is this
//! project's choice in the sense `CLAUDE.md`'s principle 5 means — a documented decision, never
//! a derivation. This module is where that decision lives, so that the tree cannot make it twice
//! and differently; ADR 0912 is the sweep that found it had made it five times, and ADR 0913 is
//! this module.
//!
//! # There is not one answer, and the families are why
//!
//! A real arriving where an integer is typed carries one of two defects, and a reader cannot
//! tell them apart from the value alone: an integer *value* written in real *syntax*
//! (`1062.00`), which loses nothing; or a genuinely fractional value (`1062.5`), whose intended
//! integer is not recoverable. One rule has to answer both, and which rule is right depends on
//! what the integer is *for*. Four families, each with its own argument:
//!
//! 1. **A magnitude on a grid the rest of the file corroborates** — Table 87's `/Width` and
//!    `/Height`. The integers are amounts, the neighbouring integer is nearly the stated value,
//!    and the sample data, the codestream's own frame (§7.4.8) and the length check are all
//!    second opinions. **Answer: read it, as the integer nearest the value.** This module.
//! 2. **An identifier wearing a number** — a bit set (`/F`, `/Ff`, `/P`, `/SigFlags`), an
//!    enumeration (`/PatternType`, `/ShadingType`, `/LC`, `/LJ`), a key into a number tree
//!    (`/StructParent`, `/StructParents`, `/MCID`), an object or generation number. Here the
//!    integers are *names*: a value between two of them is not nearly either, and the nearest
//!    one is a different name, not a weaker version of the stated one. A wrong name is silent
//!    and unbounded — the wrong structure element, the wrong permission set. **Answer: refuse,
//!    and take the entry's stated default**, which is the reading a reader already has for an
//!    entry that is absent.
//! 3. **A claim about the file that another reading settles** — §7.7.3.2's `/Count`,
//!    §7.5.8's `/Size`, a stream's `/Length`. The standard itself ranks these below the reading
//!    they summarise; of `/Count`, Table 30 says the `/Kids` arrays are what "definitively
//!    determines the number of descendant pages" and calls the entry "redundant". Accepting a
//!    malformed one would make a damaged file agree with itself and would *suppress* the
//!    disagreement that is the finding. **Answer: refuse, and derive it** — which is what this
//!    tree already does when the entry is absent.
//! 4. **A restriction the document asserts over its reader** — Table 22's `/P`, §12.8.2.2's
//!    `/DocMDP` `/P`, §12.8.6's usage rights. `CLAUDE.md` principle 3 makes these the reader's
//!    to set, so a malformed value must never restrict *harder* than the default.
//!    **Answer: refuse, in the permissive direction.** `crate::signature` already does, and its
//!    test pins it.
//!
//! A fifth shape needs no answer at all: where the value is only ever *compared* as a number —
//! `pdf_font`'s `/FontWeight` against a threshold — nothing has to become an integer, so
//! `Object::as_number` reads the file as written and the question does not arise.
//!
//! # Which entries this module answers for, and why it is not all of them
//!
//! `/Width` and `/Height`, and nothing else. That scope is a *measurement* rather than caution:
//! `crates/pdf-model/examples/integer_entry_census.rs` derives every key name the Arlington
//! model types `integer` or `bitmask` and never `number`, and counts what stands at each over
//! every corpus on the disk. A tolerance no document exercises is untested code rather than
//! robustness, and how far the tolerance *should* travel is
//! [`Q31`](../../../doc/questions/Q31-how-far-a-readers-tolerance-of-7-3-3-travels.md), open with
//! the owner. What is settled meanwhile is the shape: wherever it goes, it goes into one
//! function for that entry rather than into each call site.
//!
//! # The nearest integer, not the truncated one
//!
//! ADR 0904 chose truncation, on ADR 0371's word for §7.10.5's calculator, and pinned it with a
//! `/W 2.9` fixture. ADR 0912 measured the world and found the rule wrong on the only document
//! that exercises it: `GHOSTSCRIPT-695872-0.pdf` writes `/W 737.999999999715 /H 49.999999999`
//! over a JPEG whose own frame is 738 × 50, so truncation reads the grid one short in both axes
//! and makes this reader accuse the file of the §7.4.8 disagreement that is ours. The nearest
//! integer agrees with the codestream in both axes, and agrees with truncation on every value
//! the rest of the population writes.

use pdf_syntax::{Dictionary, Document, Object};

/// A dimension Table 87 types as an integer, read where the file wrote a real instead.
///
/// Reads an integer as itself and a real as the integer nearest its value, ties away from zero.
/// A value that names no grid at all — a NaN, an infinity, a negative, or one no `u32` holds —
/// is [`None`], exactly as it was before any tolerance existed; the callers' own `> 0` filters
/// and `crate::image::MAX_SAMPLES` are unchanged and still decide what is drawable.
///
/// This is the only place in this crate where either entry is read, which is the half of ADR
/// 0904 that was most of its value: five call sites in `crate::image` had each written their own
/// `as_integer().and_then(u32::try_from)`, so a real `/Width` on a `/Mask` would have read as the
/// number in one function and as zero in the next. `crate::inline_image::unfiltered_length` was
/// the sixth and was missed, which is the same defect one module over — and the one that matters
/// most, because both documents in the measured population write their dimensions inside a `BI`.
pub(crate) fn dimension(document: &Document, dict: &Dictionary, key: &str) -> Option<u32> {
    nearest_u32(&document.get_key(dict, key))
}

/// [`dimension`] for a value already in hand.
fn nearest_u32(value: &Object) -> Option<u32> {
    match value {
        Object::Integer(value) => u32::try_from(*value).ok(),
        // `f64::round` is ties-away-from-zero and the guard above the cast is what keeps the
        // cast's own saturation from standing in for a dimension: a value outside `u32` is
        // refused rather than clamped to its edge, because an edge is not what the file said.
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "the guard admits only a finite non-negative value inside u32's range, \
                      where the cast of an already-rounded double is exact"
        )]
        Object::Real(value) => {
            let rounded = value.round();
            (rounded.is_finite() && rounded >= 0.0 && rounded <= f64::from(u32::MAX))
                .then_some(rounded as u32)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::nearest_u32;
    use pdf_syntax::Object;

    /// `1062.00` is the value `qpdf-278-0.pdf` writes, and it names one grid.
    #[test]
    fn a_real_with_no_fractional_part_is_the_integer_it_spells() {
        assert_eq!(nearest_u32(&Object::Real(1062.00)), Some(1062));
        assert_eq!(nearest_u32(&Object::Real(1425.00)), Some(1425));
    }

    /// `GHOSTSCRIPT-695872-0.pdf`'s two dimensions, whose JPEG frame is 738 × 50.
    ///
    /// Truncation answers 737 and 49 here, which is what made this reader report a §7.4.8
    /// disagreement the file does not contain (ADR 0912).
    #[test]
    fn a_real_just_under_an_integer_is_that_integer() {
        assert_eq!(nearest_u32(&Object::Real(737.999_999_999_715)), Some(738));
        assert_eq!(nearest_u32(&Object::Real(49.999_999_999)), Some(50));
    }

    /// Nothing that is not a grid becomes one.
    #[test]
    fn a_value_that_names_no_grid_is_refused() {
        assert_eq!(nearest_u32(&Object::Real(f64::NAN)), None);
        assert_eq!(nearest_u32(&Object::Real(f64::INFINITY)), None);
        assert_eq!(nearest_u32(&Object::Real(-1.0)), None);
        assert_eq!(nearest_u32(&Object::Real(5e9)), None);
        assert_eq!(nearest_u32(&Object::Integer(-1)), None);
        assert_eq!(
            nearest_u32(&Object::Name(pdf_syntax::Name::new(&b"W"[..]))),
            None
        );
    }
}
