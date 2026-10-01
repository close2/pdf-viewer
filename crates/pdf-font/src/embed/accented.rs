//! An accented character composed into one outline, so that a `CFF ` subset can keep it
//! (ADR 1486).
//!
//! Adobe Technical Note #5177's Appendix C gives `endchar` a second form, `adx ady bchar achar
//! endchar`, which draws two other glyphs of the face: the base and the accent, each found by
//! taking its code through `StandardEncoding` to a glyph name and the name through the charset
//! to a charstring, the accent's origin placed at `(adx, ady)` from the base's. A subset is
//! written CID-keyed (ADR 1449), and a CID-keyed program's charset names no glyphs, so the form
//! cannot be carried into it as it stands. What can be carried is what it draws: the two
//! components' paths, the accent's moved by `(adx, ady)`, written out as one charstring of
//! `rmoveto`, `rlineto` and `rrcurveto` under the glyph's own width.
//!
//! **The composed charstring carries no stem hints**, which is the cost. A component's hints are
//! stated against its own outline, and the note's hint operators must precede the first path
//! operator, so the two components' hints cannot simply be laid end to end; hints change how a
//! glyph is fitted to a coarse pixel grid and never the outline itself. Every other glyph keeps
//! its own bytes.
//!
//! A face that cannot be read this way — a CID-keyed face, whose charset has no names for the
//! form to reach; a code that `StandardEncoding` leaves empty or the charset does not name; a
//! component that is itself accented, which the note forbids in so many words; a charstring that
//! drew a path before its `endchar` — is answered `None`, and the face is written whole as
//! before.

use skrifa::GlyphId;
use skrifa::raw::ps::cff::CffFontRef;
use skrifa::raw::ps::cs::CommandSink;
use skrifa::raw::ps::encoding::PredefinedEncoding;
use skrifa::raw::types::Fixed;

use super::reach::{Accented, Reached, Walk};
use crate::cff::Parts;

/// `rmoveto`, `rlineto`, `rrcurveto` and `endchar` (Adobe Technical Note #5177, Appendix A).
const RMOVETO: u8 = 21;
/// See [`RMOVETO`].
const RLINETO: u8 = 5;
/// See [`RMOVETO`].
const RRCURVETO: u8 = 8;
/// See [`RMOVETO`].
const ENDCHAR: u8 = 14;

/// The charstring that draws what `accented` draws in the name-keyed face `cff`, or `None` where
/// the module documentation says the face is written whole instead.
pub(super) fn composed(cff: &[u8], parts: &Parts<'_>, accented: &Accented) -> Option<Vec<u8>> {
    if parts.cid_keyed || accented.drew {
        return None;
    }
    let font = CffFontRef::new_cff(cff, 0, None).ok()?;
    let charset = font.charset()?;
    let glyph = |code: f64| -> Option<GlyphId> {
        let code = u8::try_from(integral(code)?).ok()?;
        let sid = PredefinedEncoding::Standard.sid(code)?;
        // SID 0 is `.notdef`, which is where `StandardEncoding` leaves a code unassigned.
        if sid.to_u16() == 0 {
            return None;
        }
        charset.glyph_id(sid).ok()
    };
    let base = glyph(accented.bchar)?;
    let accent = glyph(accented.achar)?;
    let dx = fixed(accented.adx)?;
    let dy = fixed(accented.ady)?;

    let subfont = font.subfont(0, &[]).ok()?;
    let mut path = Path::default();
    for (component, offset) in [(accent, (dx, dy)), (base, (0, 0))] {
        let charstring = parts.charstrings.get(component.to_u32() as usize)?;
        let local = &parts.font_dicts.first()?.subrs;
        let mut reached = Reached {
            global: &parts.global_subrs,
            local,
            global_entered: std::collections::BTreeSet::new(),
            local_entered: std::collections::BTreeSet::new(),
        };
        if let Walk::Seac(_) = reached.walk(charstring) {
            return None;
        }
        path.offset = offset;
        font.evaluate_charstring(&subfont, component, &[], &mut path)
            .ok()?;
        if path.overflowed {
            return None;
        }
    }

    let mut out = Vec::with_capacity(path.code.len().saturating_add(8));
    if let Some(width) = accented.width {
        push_operand(&mut out, fixed(width)?);
    }
    out.extend_from_slice(&path.code);
    out.push(ENDCHAR);
    Some(out)
}

/// `value` where it is a whole number a charstring operand can state, as the note's operands are
/// for a code.
fn integral(value: f64) -> Option<i64> {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "checked integral and within a charstring operand's 32 bits first"
    )]
    (value.fract() == 0.0 && value.abs() <= f64::from(i32::MAX)).then_some(value as i64)
}

/// `value` in the note's 16.16 fixed-point form, the precision of every charstring operand.
fn fixed(value: f64) -> Option<i32> {
    let bits = (value * 65536.0).round();
    #[expect(
        clippy::cast_possible_truncation,
        reason = "checked within i32 first; a charstring operand is a 16.16 number"
    )]
    (bits.is_finite() && bits.abs() <= f64::from(i32::MAX)).then_some(bits as i32)
}

/// One operand in the shortest form the note's section 3.2 gives it: an integer in one, two or
/// three bytes, and anything with a fraction as `255` and a 16.16 number.
fn push_operand(out: &mut Vec<u8>, bits: i32) {
    if bits & 0xFFFF != 0 {
        out.push(255);
        out.extend_from_slice(&bits.to_be_bytes());
        return;
    }
    let value = bits >> 16;
    match value {
        -107..=107 => out.push(byte(value.saturating_add(139))),
        108..=1131 => {
            let rest = value.saturating_sub(108);
            out.extend_from_slice(&[byte((rest >> 8).saturating_add(247)), byte(rest & 0xFF)]);
        }
        -1131..=-108 => {
            let rest = value.saturating_neg().saturating_sub(108);
            out.extend_from_slice(&[byte((rest >> 8).saturating_add(251)), byte(rest & 0xFF)]);
        }
        _ => {
            // `bits >> 16` of an i32 is always an i16.
            out.push(28);
            out.extend_from_slice(&i16::try_from(value).unwrap_or_default().to_be_bytes());
        }
    }
}

/// The low byte of a value each arm of [`push_operand`] has already put in `0..=255`.
fn byte(value: i32) -> u8 {
    u8::try_from(value).unwrap_or_default()
}

/// The components' paths as relative charstring operators, built as a component is evaluated.
#[derive(Default)]
struct Path {
    /// The charstring bytes so far, without a width and without `endchar`.
    code: Vec<u8>,
    /// The current point, in 16.16 units: the last point written.
    at: (i32, i32),
    /// Where the component being evaluated has its origin.
    offset: (i32, i32),
    /// Whether a point fell outside what 16.16 can state once moved.
    overflowed: bool,
}

impl Path {
    /// Writes `points` as the operands of `operator`, each relative to the one before it.
    fn push(&mut self, points: &[(Fixed, Fixed)], operator: u8) {
        for (x, y) in points {
            let target = (
                x.to_bits().checked_add(self.offset.0),
                y.to_bits().checked_add(self.offset.1),
            );
            let (Some(tx), Some(ty)) = target else {
                self.overflowed = true;
                return;
            };
            let (Some(dx), Some(dy)) = (tx.checked_sub(self.at.0), ty.checked_sub(self.at.1))
            else {
                self.overflowed = true;
                return;
            };
            push_operand(&mut self.code, dx);
            push_operand(&mut self.code, dy);
            self.at = (tx, ty);
        }
        self.code.push(operator);
    }
}

impl CommandSink for Path {
    fn move_to(&mut self, x: Fixed, y: Fixed) {
        self.push(&[(x, y)], RMOVETO);
    }

    fn line_to(&mut self, x: Fixed, y: Fixed) {
        self.push(&[(x, y)], RLINETO);
    }

    fn curve_to(&mut self, cx0: Fixed, cy0: Fixed, cx1: Fixed, cy1: Fixed, x: Fixed, y: Fixed) {
        self.push(&[(cx0, cy0), (cx1, cy1), (x, y)], RRCURVETO);
    }

    // A Type 2 charstring closes a subpath where the next `rmoveto` or the `endchar` comes.
    fn close(&mut self) {}
}
