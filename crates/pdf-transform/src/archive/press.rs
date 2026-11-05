//! The operator's press: a CMYK output profile a configuration names once, and the checks it is
//! held to before any document is read (`doc/questions/A784`, `doc/adr/2704`, `doc/adr/2705`).
//!
//! # Where it stands in
//!
//! Only where this program would otherwise state the CMYK press it ships,
//! `GRACoL2006_Coated1v2.icc` (`doc/adr/1153`): where the plan authorises the loss `press-assumed`
//! and a conversion adds a CMYK output intent, or writes part 4's `DefaultCMYK` over an assumed
//! press (`doc/adr/2518`, `doc/adr/2701`). The profile of this renderer's assumed inks, which the
//! other profiles state, is not replaced: it exists to draw as the source did, and a real press
//! would move the drawing (`doc/adr/2700`). A press given with `--output-intent-profile` outranks a
//! configured one, as it outranks the shipped one (`doc/adr/2130`).
//!
//! # What it is held to
//!
//! The profile becomes an output intent's destination profile or an `ICCBased` default, so it is
//! held to what ISO 19005-2 and ISO 19005-4 section 6.2.3 ask of a destination profile and what
//! section 6.2.4.2 asks of the profile forming an `ICCBased` stream, narrowed to a press:
//!
//! - an ICC profile at all, whose header this program reads;
//! - of the output device class (`prtr`) over `CMYK` — a monitor profile over four components
//!   describes no device a document is printed on, and a press of another space cannot stand in
//!   for a CMYK one;
//! - of an ICC edition the target part admits. ISO 19005-2 section 6.2.4.2 names four texts, which
//!   `data/icc/PROVENANCE.md` reads as profile versions 2, 4.0, 4.1 and 4.3, version 4.2 not among
//!   them; ISO 19005-4 section 6.2.4.2 sends the profile to ISO 32000-2 §8.6.5.5, whose Table 66
//!   names ICC.1:2010 (version 4.3) for PDF 2.0 and requires earlier versions to be supported too,
//!   so versions 2 to 4.3.
//!
//! A refusal names the reason in one sentence; the face that read the file names the file.

use std::fmt;
use std::sync::Arc;

use pdf_archive::{Part, Target};
use pdf_model::icc::Identification;

/// Why a configured press is not used.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PressRefused {
    /// The bytes are not an ICC profile this program can read the header of.
    NotAProfile,
    /// The profile is not an output profile over CMYK.
    NotACmykPress {
        /// The device class its header states.
        class: String,
        /// The colour space its header states.
        space: String,
    },
    /// The profile's version names an ICC edition the target part does not admit.
    Edition {
        /// The part the conversion targets.
        part: Part,
        /// The version its header states, as major, minor and bug-fix.
        version: (u8, u8, u8),
    },
}

impl fmt::Display for PressRefused {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotAProfile => f.write_str("this file is not an ICC profile"),
            Self::NotACmykPress { class, space } => write!(
                f,
                "a press is an output (prtr) profile over CMYK, and this one is a {class} profile \
                 over {space}"
            ),
            Self::Edition {
                part: Part::Two,
                version: (major, minor, bug_fix),
            } => write!(
                f,
                "ISO 19005-2 section 6.2.4.2 admits a profile of ICC.1:1998-09, ICC.1:2001-12, \
                 ICC.1:2003-09 or ISO 15076-1 (versions 2, 4.0, 4.1 and 4.3), and this one states \
                 version {major}.{minor}.{bug_fix}"
            ),
            Self::Edition {
                part: Part::Four,
                version: (major, minor, bug_fix),
            } => write!(
                f,
                "ISO 19005-4 section 6.2.4.2 sends a profile to ISO 32000-2 \u{a7}8.6.5.5, whose \
                 Table 66 names ICC.1:2010 (version 4.3) and the editions before it, and this one \
                 states version {major}.{minor}.{bug_fix}"
            ),
        }
    }
}

impl std::error::Error for PressRefused {}

/// Whether a profile version names an edition `part` admits.
const fn admitted(part: Part, version: (u8, u8, u8)) -> bool {
    match (part, version) {
        (_, (2, _, _)) => true,
        (Part::Two, (4, minor, _)) => matches!(minor, 0 | 1 | 3),
        (Part::Four, (4, minor, _)) => minor <= 3,
        _ => false,
    }
}

/// The operator's configured press, held to what [`target`](Target) asks of it.
///
/// # Errors
///
/// [`PressRefused`], saying which of the three checks the module comment names it failed.
pub fn configured_press(bytes: &[u8], target: Target) -> Result<Arc<[u8]>, PressRefused> {
    let stated = Identification::read(bytes).ok_or(PressRefused::NotAProfile)?;
    if stated.class != *b"prtr" || stated.space != *b"CMYK" {
        return Err(PressRefused::NotACmykPress {
            class: stated.class_name(),
            space: stated.space_name(),
        });
    }
    if !admitted(target.part(), stated.version) {
        return Err(PressRefused::Edition {
            part: target.part(),
            version: stated.version,
        });
    }
    Ok(bytes.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdf_archive::{Flavour, Level};

    /// The shipped press and the assumed inks are both version 2 output profiles over CMYK.
    #[test]
    fn the_two_cmyk_profiles_the_tree_holds_are_presses_at_both_parts() {
        let inks: &[u8] = include_bytes!("../../../../data/icc/quorra-assumed-inks.icc");
        for bytes in [super::super::prepare::CMYK, inks] {
            for target in [Target::Two(Level::B), Target::Four(Flavour::Plain)] {
                assert!(configured_press(bytes, target).is_ok());
            }
        }
    }

    /// 4.2 is out at part 2 and in at part 4; 4.4 and iccMAX are out at both.
    #[test]
    fn the_editions_follow_each_parts_list() {
        assert!(!admitted(Part::Two, (4, 2, 0)));
        assert!(admitted(Part::Four, (4, 2, 0)));
        assert!(admitted(Part::Two, (4, 3, 0)));
        for part in [Part::Two, Part::Four] {
            assert!(admitted(part, (2, 1, 0)));
            assert!(!admitted(part, (4, 4, 0)));
            assert!(!admitted(part, (5, 0, 0)));
        }
    }
}
