//! The standard 14 font programs, compiled into the binary.
//!
//! ISO 32000-2 §9.6.2.2 names fourteen fonts a document may use without carrying them, and
//! Table 109 is where that permission binds: `/FirstChar`, `/LastChar`, `/Widths` and
//! `/FontDescriptor` are "(Required; optional in PDF 1.0-1.7 for the standard 14 fonts)". A file
//! may therefore state a Type 1 font by name alone, and a processor with no metrics of its own
//! cannot lay one line of it out.
//!
//! **The clause used to say so in a `shall` and no longer does.** Errata Collection 3 (Issue #47
//! and #48, `/State` `Review` `Completed`) strikes §9.6.2.2's "These fonts, or their font
//! metrics and suitable substitution fonts, shall be available to the PDF processor." outright,
//! turns the paragraph above it into an informative NOTE, and softens that paragraph's own
//! "shall have" to "are required to have"; §9.6.2.1's "For compatibility reasons PDF processors
//! shall provide glyph widths and font descriptor data for those standard fonts" goes the same
//! way, replaced by a cross-reference. `doc/md/` carries none of that, because the sponsored
//! copy records EC3 as review markup and the conversion dropped it (ADR 0252, ADR 0253).
//!
//! So what this module answers to is now §6.3.2.2's requirement on a rendering processor — the
//! page contents "as defined in this document" — by way of a Table 109 permission that has not
//! moved. That is a *better* justification than the one it replaced, because it does not depend
//! on a sentence about what a processor happens to have.
//!
//! [`crate::standard_metrics`] is the "font metrics" half and has been here since the thirtieth
//! session. This is the other half — the programs themselves — and until the
//! hundred-and-forty-eighth session this tree did not have it, so a "suitable substitution font"
//! meant whatever happened to be installed. That made a rendered page a property of the machine.
//!
//! # Why these fourteen and nothing else
//!
//! They are the only faces a *document* names without supplying, so they are the only ones where
//! the file's intent is known and a substitute is not a guess. A document naming `/Garamond`
//! without embedding it is asking for something this program cannot have; a document naming
//! `/Helvetica` is asking for something the standard says a processor has.
//!
//! So [`crate::substitute::find`] consults this set *first* for a request whose `/BaseFont` is
//! one of the fourteen and *last* otherwise — see [`crate::substitute::Request::standard`]. The
//! machine's own fonts keep serving everything else, where their broader coverage is worth more
//! than reproducibility.
//!
//! **And "first" is not "only", which the four-hundred-and-thirty-fourth session had to find in a
//! 65 944-document survey.** Ten of these fourteen are bare CFF programs whose charsets hold the
//! standard Latin character set and nothing else, so a document naming one of the fourteen and
//! then stating an `/Encoding` whose `/Differences` name Cyrillic or Greek asked for characters
//! the compiled-in face has never had — and lost them in silence, because the Latin codes of the
//! same font drew and the "this font drew nothing" report never fired.
//! `pdf_font::substitute_face` is the answer and it keeps this module's trade intact: the
//! compiled-in face is replaced only by one of the same family whose code table over the codes
//! the document declares is a **strict superset**, so a page that this set can draw is still
//! drawn from the binary and identically on every machine. ADR 0270.
//!
//! # What is here, and what it costs
//!
//! Ten bare CFF programs from `PDFium`'s Foxit set (BSD-3-Clause) and four Liberation Sans faces
//! (SIL OFL 1.1), 804 KB in total, `include_bytes!`d as `static` data. That is zero parse time at
//! launch, which is the rule `CLAUDE.md` states for compiled-in data and the same rule
//! `pdf-spec`'s Arlington tables follow. `data/standard-fonts/PROVENANCE.md` records where each
//! byte came from and `/NOTICE` carries the attribution both licences require.

use crate::substitute::{Family, Format, Request};

/// One compiled-in face: its bytes and which reader parses them.
type Face = (&'static [u8], Format);

/// Liberation Sans, which is metric-compatible with Helvetica.
const SANS: [Face; 4] = [
    (
        include_bytes!("../../../data/standard-fonts/LiberationSans-Regular.ttf"),
        Format::Sfnt,
    ),
    (
        include_bytes!("../../../data/standard-fonts/LiberationSans-Bold.ttf"),
        Format::Sfnt,
    ),
    (
        include_bytes!("../../../data/standard-fonts/LiberationSans-Italic.ttf"),
        Format::Sfnt,
    ),
    (
        include_bytes!("../../../data/standard-fonts/LiberationSans-BoldItalic.ttf"),
        Format::Sfnt,
    ),
];

/// Foxit's Times-metric serif faces, which are CFF programs under a `.pfb` name.
const SERIF: [Face; 4] = [
    (
        include_bytes!("../../../data/standard-fonts/FoxitSerif.pfb"),
        Format::BareCff,
    ),
    (
        include_bytes!("../../../data/standard-fonts/FoxitSerifBold.pfb"),
        Format::BareCff,
    ),
    (
        include_bytes!("../../../data/standard-fonts/FoxitSerifItalic.pfb"),
        Format::BareCff,
    ),
    (
        include_bytes!("../../../data/standard-fonts/FoxitSerifBoldItalic.pfb"),
        Format::BareCff,
    ),
];

/// Foxit's Courier-metric fixed-pitch faces.
const FIXED: [Face; 4] = [
    (
        include_bytes!("../../../data/standard-fonts/FoxitFixed.pfb"),
        Format::BareCff,
    ),
    (
        include_bytes!("../../../data/standard-fonts/FoxitFixedBold.pfb"),
        Format::BareCff,
    ),
    (
        include_bytes!("../../../data/standard-fonts/FoxitFixedItalic.pfb"),
        Format::BareCff,
    ),
    (
        include_bytes!("../../../data/standard-fonts/FoxitFixedBoldItalic.pfb"),
        Format::BareCff,
    ),
];

/// Symbol, whose character set is its own (§9.6.2.2, Annex D.5).
const SYMBOL: Face = (
    include_bytes!("../../../data/standard-fonts/FoxitSymbol.pfb"),
    Format::BareCff,
);

/// `ZapfDingbats`, likewise (Annex D.6).
const DINGBATS: Face = (
    include_bytes!("../../../data/standard-fonts/FoxitDingbats.pfb"),
    Format::BareCff,
);

/// The compiled-in face answering a request, which exists for every request.
///
/// Never `None`: every [`Family`] has a face here, which is what makes a page render the same on
/// a machine with no fonts installed at all as on one with a thousand.
#[must_use]
pub fn face(request: Request) -> Face {
    // Bold and italic index a four-element table in the order regular, bold, italic, bold-italic,
    // which is the order the four files are listed in above.
    let style = usize::from(request.bold) | (usize::from(request.italic) << 1);
    match request.family {
        Family::SansSerif => SANS[style & 3],
        Family::Serif => SERIF[style & 3],
        Family::Monospace => FIXED[style & 3],
        Family::Symbol => SYMBOL,
        Family::ZapfDingbats => DINGBATS,
    }
}

/// §9.6.2.2's fourteen names, exactly as the clause spells them.
///
/// > The PostScript language names of 14 Type 1 fonts, known as the standard 14 fonts, are as
/// > follows: Times-Roman, Helvetica, Courier, Symbol, Times-Bold, Helvetica-Bold, Courier-Bold,
/// > `ZapfDingbats`, Times-Italic, Helvetica-Oblique, Courier-Oblique, Times-BoldItalic,
/// > Helvetica-BoldOblique, `CourierBoldOblique`.
///
/// **This is a different question from [`crate::substitute`]'s**, and the difference is what
/// makes the narrowness necessary. That module asks whether a `/BaseFont` — a *typeface's* name,
/// written by whoever made the font — is one of the fourteen, so it folds case, matches on the
/// family and accepts the metric-compatible clones a producer means by `Arial`. This asks
/// whether a **resource name** is one of the fourteen, and a resource name is arbitrary: a file
/// may call its resource `/Arial` and mean anything at all. Only the clause's own fourteen
/// strings are a name whose meaning the standard states rather than a producer's.
///
/// `Courier-BoldOblique` is accepted beside the clause's unhyphenated `CourierBoldOblique`: the
/// list spells thirteen of the fourteen with a hyphen and this one without, which reads as the
/// standard's own typography rather than as a distinct name, and producers write the hyphen.
const STANDARD_NAMES: [&str; 15] = [
    "Times-Roman",
    "Times-Bold",
    "Times-Italic",
    "Times-BoldItalic",
    "Helvetica",
    "Helvetica-Bold",
    "Helvetica-Oblique",
    "Helvetica-BoldOblique",
    "Courier",
    "Courier-Bold",
    "Courier-Oblique",
    "CourierBoldOblique",
    "Courier-BoldOblique",
    "Symbol",
    "ZapfDingbats",
];

/// Whether `name` is one of §9.6.2.2's fourteen, spelled exactly as the clause spells it.
///
/// Case-sensitive and whole-string, for the reason [`STANDARD_NAMES`] gives: this is asked of a
/// *resource* name, where anything looser would start claiming that a file's `/helvetica` or
/// `/Helvetica2` resource means the standard font.
#[must_use]
pub fn is_standard_name(name: &str) -> bool {
    STANDARD_NAMES.contains(&name)
}

#[cfg(test)]
mod tests {
    use super::{DINGBATS, FIXED, SANS, SERIF, SYMBOL, face, is_standard_name};

    /// The fourteen are matched exactly, and nothing near them is.
    ///
    /// The negative half is the point: `/Arial` is a resource name a file may use for anything,
    /// and §9.6.2.2 does not name it. `crate::substitute` *does* accept it, one question over,
    /// because there it is a typeface's own name rather than a label in a resource dictionary.
    #[test]
    fn only_the_clauses_own_fourteen_names_are_standard() {
        for name in [
            "Helvetica",
            "Helvetica-BoldOblique",
            "Times-Roman",
            "Courier",
            "Courier-BoldOblique",
            "CourierBoldOblique",
            "Symbol",
            "ZapfDingbats",
        ] {
            assert!(is_standard_name(name), "{name} is one of the fourteen");
        }
        for name in [
            "helvetica",
            "HELVETICA",
            "Arial",
            "Helvetica2",
            "F1",
            "Helv",
            "TimesNewRoman",
            "",
        ] {
            assert!(!is_standard_name(name), "{name} is not one of the fourteen");
        }
    }

    use crate::substitute::{Family, Format, Request};

    /// Every compiled-in face is present, non-empty and in the format claimed for it.
    ///
    /// The point is not the byte count. It is that `include_bytes!` of a missing file is a
    /// compile error while `include_bytes!` of a *truncated* one is not, and a font this
    /// program cannot parse would be reported per document rather than once, here.
    #[test]
    fn every_compiled_in_face_parses() {
        let mut checked = 0;
        for (bytes, format) in SANS
            .iter()
            .chain(SERIF.iter())
            .chain(FIXED.iter())
            .chain([&SYMBOL, &DINGBATS])
        {
            assert!(bytes.len() > 4096, "a face of {} bytes", bytes.len());
            match format {
                Format::Sfnt => {
                    skrifa::raw::FontRef::new(bytes).expect("a compiled-in sfnt face parses");
                }
                Format::BareCff => {
                    crate::cff::units_per_em(bytes).expect("a compiled-in CFF face parses");
                }
            }
            checked += 1;
        }
        assert_eq!(checked, 14, "§9.6.2.2's fourteen");
    }

    /// The fourteen compiled-in faces agree about which way a contour runs.
    ///
    /// **This is the property `OverlappingGlyphClipping.pdf` needs and it is checkable with no
    /// reference at all.** §9.3.6 combines a text object's glyph outlines into one path under
    /// the non-zero winding number rule and its NOTE 2 says that "the direction of the paths
    /// comprising each glyph can cause different output for overlapping glyphs", so a document
    /// that draws two of §9.6.2.2's fourteen into one clip sees the difference: where two
    /// glyphs wound opposite ways overlap, the overlap cancels instead of uniting. §9.6.2.2
    /// calls the fourteen one set of Type 1 fonts, so the set standing in for them may not
    /// disagree with itself.
    ///
    /// Ten of these faces are Foxit's bare CFF and four are Liberation Sans `sfnt`s, and the
    /// two formats carry **opposite** conventions — measured, in the
    /// five-hundred-and-sixty-first session, at −0.186 against +0.165 for a capital `B` in the
    /// em square. [`crate::substituted::wound_counter_clockwise`] is what makes this pass, so
    /// deleting it fails here rather than on a page nobody looks at.
    #[test]
    fn every_compiled_in_face_winds_its_contours_the_same_way() {
        // A letter every one of the fourteen draws, including the two symbolic faces, whose
        // encodings answer this code with something of their own (Annex D.5, D.6).
        let code = crate::Code::single_byte(b'B');
        let mut checked = 0;
        for name in [
            "Helvetica",
            "Helvetica-Bold",
            "Helvetica-Oblique",
            "Helvetica-BoldOblique",
            "Times-Roman",
            "Times-Bold",
            "Times-Italic",
            "Times-BoldItalic",
            "Courier",
            "Courier-Bold",
            "Courier-Oblique",
            "Courier-BoldOblique",
            "Symbol",
            "ZapfDingbats",
        ] {
            let font = crate::LoadedFont::standard(name).expect("one of the fourteen");
            let outline = font.outline(code).expect("a glyph with contours");
            assert!(
                outline.signed_area() > 0.0,
                "{name} draws {:+} where every other face draws counter-clockwise",
                outline.signed_area()
            );
            checked += 1;
        }
        assert_eq!(checked, 14, "§9.6.2.2's fourteen");
    }

    /// Every request this crate can derive has a face, and the styles are not transposed.
    #[test]
    fn every_family_and_style_answers() {
        for family in [
            Family::SansSerif,
            Family::Serif,
            Family::Monospace,
            Family::Symbol,
            Family::ZapfDingbats,
        ] {
            let mut seen = Vec::new();
            for (bold, italic) in [(false, false), (true, false), (false, true), (true, true)] {
                let (bytes, _) = face(Request {
                    family,
                    bold,
                    italic,
                    standard: true,
                });
                seen.push(bytes.as_ptr());
            }
            if family.is_symbolic() {
                // Symbol and ZapfDingbats have one face each, and the standard names no bold or
                // italic variant of either (§9.6.2.2's list).
                assert!(seen.windows(2).all(|pair| pair[0] == pair[1]), "{family:?}");
            } else {
                // Four distinct programs, so a table written in the wrong order shows up as a
                // repeat rather than as a subtly wrong weight on a page nobody looks at.
                seen.sort_unstable();
                seen.dedup();
                assert_eq!(seen.len(), 4, "{family:?}");
            }
        }
    }
}
