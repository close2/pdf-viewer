//! A bare name-keyed CFF program cut to the glyphs a simple font shows, for §9.9.1's Table 124
//! `/FontFile3` with `/Subtype /Type1C` under a `Type1` font dictionary (`doc/adr/2939`).
//!
//! [`super::cff`] writes every subset CID-keyed, because a `CIDFontType0` reaches its glyphs by
//! CID. A simple font reaches them by **name**: §9.6.5.2 takes a code through the font's encoding
//! to a glyph name, and the name through the program's charset to a glyph. So this subset keeps
//! what that route reads and nothing else:
//!
//! - **The charstrings of the glyphs asked for and of `.notdef`**, glyph 0 — §9.9.2's "\[t]he
//!   glyph name .notdef shall be defined in the font subset" — renumbered densely in the source's
//!   order, with every subroutine they reach ([`super::cff::Closure`], which also composes an
//!   accented `endchar`'s outline in its place).
//! - **A charset in format 0 giving each kept glyph its own name** (Adobe Technical Note #5176,
//!   section 13): the name, not the index, is what a code reaches, so renumbering moves nothing a
//!   reader looks up. A custom name is carried into the subset's String INDEX.
//! - **The Top DICT's descriptive entries and the Private DICT**, as the CID-keyed subset keeps
//!   them, with the three offsets — `charset`, `CharStrings`, `Private` — written fresh.
//!
//! **The built-in encoding must survive the cut, and only the standard one does by name.** A
//! program that states no `Encoding` has the Standard Encoding (Technical Note #5176, section
//! 12), which maps codes to names and so reaches the same glyphs in the subset. A custom or
//! Expert encoding is a table of codes to glyph indices or one this does not restate, so such a
//! program is answered `None` and the caller writes it whole.
//!
//! The name is §9.9.2's: "a tag followed by a plus sign (+) followed by the PostScript name of the
//! font from which the subset was created", and it is written into the Name INDEX as well, since
//! Table 109's `BaseFont` row makes a `Type1` font's `/BaseFont` the value of the program's own
//! `FontName` entry.

use std::collections::{BTreeMap, BTreeSet};

use super::Refusal;
use super::cff::{CARRIED_STRINGS, Closure, RESTATED, Strings, index, private_with_subrs, stubbed};
use crate::cff::{
    OP_CHARSET, OP_CHARSTRINGS, OP_ENCODING, OP_PRIVATE, Parts, dict_ints, int5, push_operator,
};

/// A bare name-keyed CFF subset, and how its glyphs were renumbered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompactSubset {
    /// The program: the stream's decoded data under `/FontFile3 /Type1C`.
    pub program: Vec<u8>,
    /// Each kept glyph of the source, to its index in [`Self::program`].
    pub glyphs: BTreeMap<u16, u16>,
    /// §9.9.2's name: six uppercase letters, a plus sign and the source's own `FontName`, which
    /// the font dictionary's `/BaseFont` and the descriptor's `/FontName` state.
    pub name: String,
}

const MALFORMED: Refusal = Refusal::Malformed("CFF ");

/// `cff`, a bare name-keyed CFF program, cut to `used` and `.notdef` as the module documentation
/// describes; `None` where it is to be written whole — its built-in encoding is not the Standard
/// one, or a kept accented charstring cannot be composed.
///
/// A bare program states no licence of its own (an sfnt's `OS/2` `fsType` is where one is), so
/// whether the face may be subset is the caller's to know before it asks.
///
/// # Errors
///
/// [`Refusal::Malformed`] where the program does not take apart, is CID-keyed (which names no
/// glyph), states no charset for a kept glyph, or `used` names a glyph past its last.
pub fn compact_subset(cff: &[u8], used: &BTreeSet<u16>) -> Result<Option<CompactSubset>, Refusal> {
    let parts = Parts::read(cff).ok_or(MALFORMED)?;
    if parts.cid_keyed {
        return Err(MALFORMED);
    }
    let kept: BTreeSet<u16> = used.iter().copied().chain([0]).collect();
    if kept
        .iter()
        .any(|glyph| usize::from(*glyph) >= parts.charstrings.len())
    {
        return Err(MALFORMED);
    }
    let standard_encoding = parts
        .top
        .iter()
        .filter(|(_, op)| *op == OP_ENCODING)
        .all(|(operands, _)| dict_ints(operands).first() == Some(&0));
    if !standard_encoding {
        return Ok(None);
    }
    let Some(closure) = Closure::of(cff, &parts, &kept)? else {
        return Ok(None);
    };
    let source_name: String = String::from_utf8_lossy(parts.name)
        .chars()
        .filter(|c| c.is_ascii_graphic() && !"()<>[]{}/%#".contains(*c))
        .collect();
    if source_name.is_empty() {
        return Err(MALFORMED);
    }
    let name = format!("{}+{source_name}", super::tag(cff, &kept));
    let program = written(&parts, &closure, name.as_bytes())?;
    Ok(Some(CompactSubset {
        program,
        glyphs: kept
            .iter()
            .zip(0u16..)
            .map(|(old, new)| (*old, new))
            .collect(),
        name,
    }))
}

/// Where the three tables the Top DICT points at were placed, and the Private DICT's length.
#[derive(Clone, Copy, Default)]
struct Placed {
    charset: i64,
    charstrings: i64,
    private_size: i64,
    private: i64,
}

/// The subset's bytes, in Technical Note #5176 section 2's order: header, Name INDEX, Top DICT
/// INDEX, String INDEX, Global Subr INDEX, charset, `CharStrings`, then the Private DICT with its
/// Local Subr INDEX directly after it. Every offset is written in the five-byte form, so the
/// layout is measured once with placeholders and written again at the same lengths, as
/// [`super::cff`]'s writer does.
fn written(parts: &Parts<'_>, closure: &Closure, name: &[u8]) -> Result<Vec<u8>, Refusal> {
    let mut strings = Strings::default();
    let mut head = Vec::new();
    for (operands, op) in &parts.top {
        if RESTATED.contains(op) {
            continue;
        }
        if CARRIED_STRINGS.contains(op) {
            head.extend_from_slice(&strings.carried(parts, operands)?);
        } else {
            head.extend_from_slice(operands);
        }
        push_operator(&mut head, *op);
    }
    // Format 0: a format byte, then one SID for every glyph after `.notdef`, which has none.
    let mut charset = vec![0u8];
    for (glyph, _) in closure.glyphs.iter().skip(1) {
        let sid = parts.glyph_sid(*glyph).ok_or(MALFORMED)?;
        let sid = u16::try_from(strings.carried_sid(parts, sid)?).map_err(|_| MALFORMED)?;
        charset.extend_from_slice(&sid.to_be_bytes());
    }
    let charstrings = closure.charstrings(parts).ok_or(MALFORMED)?;
    let global = stubbed(&parts.global_subrs, &closure.global);
    let dict = parts.font_dicts.first().ok_or(MALFORMED)?;
    let reached = closure.local.first().ok_or(MALFORMED)?;
    let (private, subrs) = private_with_subrs(dict, reached)?;

    let top_dict = |placed: Placed| {
        let mut out = head.clone();
        out.extend_from_slice(&int5(placed.charset));
        push_operator(&mut out, OP_CHARSET);
        out.extend_from_slice(&int5(placed.charstrings));
        push_operator(&mut out, OP_CHARSTRINGS);
        out.extend_from_slice(&int5(placed.private_size));
        out.extend_from_slice(&int5(placed.private));
        push_operator(&mut out, OP_PRIVATE);
        out
    };
    let header = [1u8, 0, 4, 4];
    let name_index = index(&[name]).ok_or(MALFORMED)?;
    let top_len = index(&[top_dict(Placed::default())])
        .ok_or(MALFORMED)?
        .len();
    let string_index = index(&strings.0).ok_or(MALFORMED)?;
    let global_index = index(&global).ok_or(MALFORMED)?;
    let charstring_index = index(&charstrings).ok_or(MALFORMED)?;

    let offset = |length: usize| i64::try_from(length).map_err(|_| MALFORMED);
    let charset_at = header
        .len()
        .saturating_add(name_index.len())
        .saturating_add(top_len)
        .saturating_add(string_index.len())
        .saturating_add(global_index.len());
    let charstrings_at = charset_at.saturating_add(charset.len());
    let private_at = charstrings_at.saturating_add(charstring_index.len());
    let top = top_dict(Placed {
        charset: offset(charset_at)?,
        charstrings: offset(charstrings_at)?,
        private_size: offset(private.len())?,
        private: offset(private_at)?,
    });
    let mut out = Vec::with_capacity(
        private_at
            .saturating_add(private.len())
            .saturating_add(subrs.len()),
    );
    out.extend_from_slice(&header);
    out.extend_from_slice(&name_index);
    out.extend_from_slice(&index(&[top.as_slice()]).ok_or(MALFORMED)?);
    out.extend_from_slice(&string_index);
    out.extend_from_slice(&global_index);
    out.extend_from_slice(&charset);
    out.extend_from_slice(&charstring_index);
    out.extend_from_slice(&private);
    out.extend_from_slice(&subrs);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::compact_subset;
    use crate::cff::{charstring_widths, draw};
    use skrifa::outline::OutlinePen;
    use std::collections::BTreeSet;

    /// The serif face this program compiles in: a bare name-keyed CFF with the Standard Encoding.
    const SERIF: &[u8] = include_bytes!("../../../../data/standard-fonts/FoxitSerif.pfb");

    /// Every pen call, as text, so that two outlines compare as the calls that drew them.
    #[derive(Default)]
    struct Calls(Vec<String>);

    impl OutlinePen for Calls {
        fn move_to(&mut self, x: f32, y: f32) {
            self.0.push(format!("M {x} {y}"));
        }
        fn line_to(&mut self, x: f32, y: f32) {
            self.0.push(format!("L {x} {y}"));
        }
        fn quad_to(&mut self, cx: f32, cy: f32, x: f32, y: f32) {
            self.0.push(format!("Q {cx} {cy} {x} {y}"));
        }
        fn curve_to(&mut self, cx0: f32, cy0: f32, cx1: f32, cy1: f32, x: f32, y: f32) {
            self.0.push(format!("C {cx0} {cy0} {cx1} {cy1} {x} {y}"));
        }
        fn close(&mut self) {
            self.0.push("Z".to_owned());
        }
    }

    fn outline(program: &[u8], glyph: u16) -> Vec<String> {
        let mut calls = Calls::default();
        draw(program, glyph, &mut calls).expect("the glyph draws");
        calls.0
    }

    /// The glyph a name reaches through the program's own charset, which is how §9.6.5.2 reaches
    /// a simple font's glyph from the name its encoding gives a code.
    fn by_name(program: &[u8], name: &str) -> Option<u16> {
        crate::cff::CodeToGlyph::read(program)
            .ok()
            .and_then(|table| match table {
                crate::cff::CodeToGlyph::Named(named) => named.by_name.get(name).copied(),
                crate::cff::CodeToGlyph::Keyed { .. } => None,
            })
    }

    #[test]
    fn a_subset_keeps_each_glyph_under_its_own_name_with_its_own_outline_and_width() {
        let names = ["A", "g", "eacute", "quotedblleft"];
        let used: BTreeSet<u16> = names
            .iter()
            .map(|name| by_name(SERIF, name).expect("the serif face names it"))
            .collect();
        let subset = compact_subset(SERIF, &used)
            .expect("the face takes apart")
            .expect("the Standard Encoding survives the cut");
        assert!(
            subset.program.len() * 10 < SERIF.len(),
            "{}",
            subset.program.len()
        );
        assert_eq!(crate::cff::glyph_count(&subset.program).ok(), Some(5));
        for name in names {
            let (old, new) = (
                by_name(SERIF, name).expect("named in the face"),
                by_name(&subset.program, name).expect("named in the subset"),
            );
            assert_eq!(subset.glyphs.get(&old), Some(&new), "{name}");
            assert_eq!(outline(SERIF, old), outline(&subset.program, new), "{name}");
            assert_eq!(
                charstring_widths(SERIF, &[old]).ok(),
                charstring_widths(&subset.program, &[new]).ok(),
                "{name}"
            );
        }
        // §9.9.2: `.notdef` is defined, and the name is six capitals, a plus and the face's own.
        assert_eq!(subset.glyphs.get(&0), Some(&0));
        let (tag, face) = subset.name.split_once('+').expect("a tagged name");
        assert!(tag.len() == 6 && tag.bytes().all(|byte| byte.is_ascii_uppercase()));
        assert!(!face.is_empty() && !face.contains('+'));
        assert!(by_name(&subset.program, "B").is_none());
    }

    #[test]
    fn two_subsets_of_one_face_are_tagged_apart() {
        let one = compact_subset(SERIF, &BTreeSet::from([36]))
            .expect("takes apart")
            .expect("subset");
        let two = compact_subset(SERIF, &BTreeSet::from([37]))
            .expect("takes apart")
            .expect("subset");
        assert_ne!(one.name, two.name);
    }
}
