//! The pair kerning of §9.6.2.2's fourteen, as Adobe published it.
//!
//! ISO 32000-2 §9.6.2.2 names fourteen fonts a document may use without embedding them, and a
//! document that does names a typeface whose metrics were published with it: Adobe's Core 14 AFM
//! files, which state each face's pairs as `KPX` lines — "\[n]ame of the first character in the
//! kerning pair followed by the name of the second character followed by the kerning amount in
//! the x direction" (Adobe Technical Note #5004, *Adobe Font Metrics File Format Specification*,
//! version 4.1, section 9.2, quoted from Adobe's text and not ISO's). Those pairs are the
//! typeface's, the same kind of fact as the widths `crate::standard_metrics` holds, so a run set in
//! one of the fourteen is kerned by them although its glyphs are drawn by a stand-in
//! (`doc/questions/A308`, ADR 1916).
//!
//! # Where the numbers come from
//!
//! `build.rs` reads `data/standard-fonts/afm/`, the fourteen files and `MustRead.html` as Adobe
//! published them (`data/standard-fonts/PROVENANCE.md`), and writes their `KPX` lines as plain
//! numbers — **a modified form of the files, marked as one at the head of what it writes**, as
//! `MustRead.html`'s conditions require; `/NOTICE` carries its paragraph.
//!
//! # Choices (ADR 1916)
//!
//! - **The fourteen by their own names only.** A `/BaseFont` that is one of §9.6.2.2's strings is
//!   kerned by its file; a clone the substitution accepts for the same metrics — `Arial`,
//!   `TimesNewRoman` — is another typeface whose pairs are its own, and is not.
//! - **A face whose file states no pairs is kerned by none, and that is the property carried out.**
//!   The four Couriers, `Symbol` and `ZapfDingbats` publish no `KernData`, so a run in them is laid
//!   out as the published metrics lay it out and nothing is said.
//! - **The amount is the first glyph's**: the kerning vector "is the amount by which to move the
//!   second character relative to the first" (the same section), so it is added to the left-hand
//!   glyph's advance, in thousandths of an em — the unit of every AFM metric of these files.

/// The table `build.rs` writes from `data/standard-fonts/afm/`.
mod tables {
    include!(concat!(env!("OUT_DIR"), "/published_pairs.rs"));
}

/// One of the fourteen's published pairs, as a range of the compiled-in table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Published {
    pairs: &'static [(u16, u16, i16)],
}

/// The published pairs of the face `base_font` names, where it is one of §9.6.2.2's fourteen
/// spelled as the clause spells it ([`crate::standard::is_standard_name`]).
///
/// `CourierBoldOblique` is the clause's spelling of the file Adobe names `Courier-BoldOblique`.
pub(crate) fn published(base_font: &str) -> Option<Published> {
    if !crate::standard::is_standard_name(base_font) {
        return None;
    }
    let name = if base_font == "CourierBoldOblique" {
        "Courier-BoldOblique"
    } else {
        base_font
    };
    let (start, count) = *tables::RANGES.get(tables::face_number(name)?)?;
    let start = usize::from(start);
    let pairs = tables::PAIRS.get(start..start.checked_add(usize::from(count))?)?;
    Some(Published { pairs })
}

/// The number the table gives a glyph name, where some pair of the fourteen names it.
pub(crate) fn glyph(name: &str) -> Option<u16> {
    let text = |&(start, length): &(u16, u8)| {
        let start = usize::from(start);
        tables::NAME_TEXT.get(start..start.saturating_add(usize::from(length)))
    };
    let at = tables::NAMES
        .binary_search_by(|entry| text(entry).unwrap_or_default().cmp(name.as_bytes()))
        .ok()?;
    u16::try_from(at).ok()
}

impl Published {
    /// The `KPX` amount between a left-hand and a right-hand glyph, in ems.
    pub(crate) fn amount(self, left: u16, right: u16) -> Option<f32> {
        let at = self
            .pairs
            .binary_search_by(|&(first, second, _)| (first, second).cmp(&(left, right)))
            .ok()?;
        let (_, _, amount) = self.pairs.get(at)?;
        Some(f32::from(*amount) / 1000.0)
    }

    /// How many pairs the face's file states.
    #[cfg(test)]
    fn len(self) -> usize {
        self.pairs.len()
    }
}

#[cfg(test)]
mod tests {
    use super::{glyph, published};

    /// The counts are the files' own `StartKernPairs` lines, which `build.rs` checks each file's
    /// `KPX` lines against: 2 705 for Helvetica and its oblique, 2 481 for the bold pair, 2 073,
    /// 2 242, 2 321 and 2 038 for the four Times faces, and none for the Couriers, Symbol and
    /// `ZapfDingbats`, which state no `KernData`.
    #[test]
    fn each_face_holds_the_pairs_its_file_states() {
        for (face, count) in [
            ("Helvetica", 2705),
            ("Helvetica-Oblique", 2705),
            ("Helvetica-Bold", 2481),
            ("Helvetica-BoldOblique", 2481),
            ("Times-Roman", 2073),
            ("Times-Bold", 2242),
            ("Times-Italic", 2321),
            ("Times-BoldItalic", 2038),
            ("Courier", 0),
            ("Courier-Bold", 0),
            ("Courier-Oblique", 0),
            ("CourierBoldOblique", 0),
            ("Courier-BoldOblique", 0),
            ("Symbol", 0),
            ("ZapfDingbats", 0),
        ] {
            let face = published(face).unwrap_or_else(|| panic!("{face} is one of the fourteen"));
            assert_eq!(face.len(), count, "{face:?}");
        }
    }

    /// Lines read out of the files themselves: `Helvetica.afm`'s `KPX A V -70` and `KPX V A -80`,
    /// which are not one pair stated twice, `Times-Roman.afm`'s `KPX A V -135` and `KPX T o -80`,
    /// and `Helvetica-Bold.afm`'s `KPX A V -80`. A pair a file does not state has no amount.
    #[test]
    fn a_pair_is_the_amount_its_kpx_line_states() {
        let amount = |face: &str, first: &str, second: &str| {
            let face = published(face).expect("one of the fourteen");
            face.amount(glyph(first)?, glyph(second)?)
        };
        assert_eq!(amount("Helvetica", "A", "V"), Some(-0.07));
        assert_eq!(amount("Helvetica", "V", "A"), Some(-0.08));
        assert_eq!(amount("Times-Roman", "A", "V"), Some(-0.135));
        assert_eq!(amount("Times-Roman", "T", "o"), Some(-0.08));
        assert_eq!(amount("Helvetica-Bold", "A", "V"), Some(-0.08));
        assert_eq!(amount("Helvetica", "A", "A"), None);
        assert_eq!(amount("Courier", "A", "V"), None);
    }

    /// A font the document names as `Helvetica` and does not embed is kerned by `Helvetica.afm`'s
    /// pairs, read by the glyph name each code selects (§9.6.5's `StandardEncoding` here): `A V`
    /// is −70 thousandths on the `A`, `V A` −80 on the `V`, and the last glyph pairs with nothing.
    /// Read right to left the left-hand glyph is the second of the logical pair, so `A V` takes
    /// `KPX V A`'s −80. `Arial` is a stand-in whose pairs would be another typeface's, and says so.
    #[test]
    fn a_standard_font_the_document_does_not_embed_is_kerned_by_its_published_pairs() {
        use crate::pairs::{Language, NoPairs, scripts};
        use crate::{Code, LoadedFont};

        let font = LoadedFont::standard("Helvetica").expect("one of the fourteen loads");
        let pairs = font.pairs().expect("Helvetica's published pairs");
        let codes = b"AVA".map(Code::single_byte);
        let latin = scripts(&['A', 'V', 'A']);
        let advances = |right_to_left: &[bool]| -> Vec<f32> {
            font.pair_adjustments(&pairs, &codes, &latin, Language::default(), right_to_left)
                .glyphs
                .iter()
                .map(|glyph| glyph.advance)
                .collect()
        };
        assert_eq!(advances(&[false; 3]), [-0.07, -0.08, 0.0]);
        assert_eq!(advances(&[true; 3]), [-0.08, -0.07, 0.0]);

        let stand_in = LoadedFont::standard("Arial").expect("a stand-in is found");
        assert_eq!(stand_in.pairs().err(), Some(NoPairs::NotTheDocuments));
    }

    /// Only the clause's own names reach a file: a clone the substitution accepts for the same
    /// metrics names another typeface, and a resource name is not a typeface's.
    #[test]
    fn only_the_fourteen_by_name_have_published_pairs() {
        for name in [
            "Arial",
            "ArialMT",
            "TimesNewRoman",
            "helvetica",
            "Helv",
            "Helvetica,Bold",
        ] {
            assert!(
                published(name).is_none(),
                "{name} is not one of the fourteen"
            );
        }
    }
}
