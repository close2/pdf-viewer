//! The standard 14 font programs, compiled into the binary.
//!
//! ISO 32000-2 §9.6.2.2 names fourteen fonts a document may use without carrying them:
//!
//! > These fonts, or their font metrics and suitable substitution fonts, shall be available to
//! > the PDF processor.
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

#[cfg(test)]
mod tests {
    use super::{DINGBATS, FIXED, SANS, SERIF, SYMBOL, face};
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
