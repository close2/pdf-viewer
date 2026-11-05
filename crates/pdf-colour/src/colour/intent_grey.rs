//! `DeviceGray` in a PDF/A file whose output intent is a CMYK press: the press's black ink alone.
//!
//! ISO 32000-2 leaves a grey painted beside a four-component output intent to the processor —
//! §14.11.5 makes the intent informational, and §11.7.2 converts a device colour of another
//! component count "in an implementationdependent fashion" — so on any other file this tree
//! draws `DeviceGray` as its own neutral grey and leaves the intent to the families it has the
//! components of ([`super::ColourSpace::device_family`]).
//!
//! **A PDF/A file is read by a second standard as well, and that one decides.** ISO 19005-2 and
//! ISO 19005-4, section 6.2.4.3, bind a conforming reader rendering a `DeviceGray` colour in a
//! file that has set no device-independent `DefaultGray`: where the PDF/A output intent carries a
//! CMYK destination profile, the reader converts the grey to CMYK by ISO 32000's classic method
//! and renders the result with that profile as its source colour space. (ISO 19005 is cited by
//! section and paraphrased, never quoted: `doc/third-party-data.md`.) The classic method is
//! ISO 32000-2 §10.4.2.3:
//!
//! > Nominally, a gray level is the complement of the black component of CMYK . Therefore, the
//! > CMYK colour value equivalent to a specific gray level shall be
//!
//! followed by a formula set in mathematical italics that no transcription survives: cyan,
//! magenta and yellow `0.0`, and black `1.0 − gray`. The owner adopted the rule for this reader
//! (`doc/questions/A788`, ADR 2712), so `0 g` in such a file draws as the press's black ink and
//! not as this reader's black.
//!
//! **The construction is a space the rest of the tree already reads**: a one-component
//! `Separation` whose alternate is the intent's own profile and whose tints are the clause's
//! formula ([`super::Tints::GreyAsBlack`]). Every route a colour takes — a fill, a stroke, an
//! image's samples, a shading's ramp, a mask group's backdrop — already evaluates a separation's
//! tints into its alternate and converts from there, so the rule reaches all of them from the one
//! place a device family is resolved, and a grey painted into a group composited in the intent's
//! own press keeps its `(0, 0, 0, 1 − gray)` exactly rather than being separated again by the
//! press's `B2A` table (ADR 2708 measured what that separation did to six soft masks).

use std::sync::{Arc, LazyLock};

use pdf_syntax::Name;

use super::{ColourSpace, Tints};

/// How `DeviceGray` is read on a page with an output intent.
///
/// The answer is the document's and the page's, decided where `pdf_model` selects the intent:
/// this crate cannot read a file's XMP identification, and should not — what makes a file PDF/A
/// is a question about the file, and the colour space parser is asked about one object.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GreyReading {
    /// `DeviceGray` is this reader's neutral grey, whatever the intent is: ISO 32000-2 alone,
    /// which leaves the conversion to a four-component intent implementation-dependent.
    #[default]
    Device,
    /// `DeviceGray` is the four-component output intent's black ink alone: ISO 19005-2 and -4
    /// section 6.2.4.3, in a file that identifies as PDF/A and whose PDF/A output intent is a
    /// CMYK profile. A `/DefaultGray` the resources state still outranks it, by the clause's own
    /// condition and by §8.6.5.6.
    IntentBlack,
}

/// The one component name the space carries, which no device has a colourant of.
///
/// A `Separation` names its colourant, and a colourant the simulated device of §10.8.3 has is
/// painted directly rather than through the alternate (`super::spots`). `DeviceGray` names no
/// colourant at all, and the clause routes it through the intent's profile on every device, so
/// the name is the family's own: it is none of §8.6.6.4's process names and no spot colourant a
/// page can name collides with a space family's name in this tree's reading.
static NAMES: LazyLock<Arc<[Name]>> =
    LazyLock::new(|| Arc::from([Name::new(b"DeviceGray".to_vec())]));

impl ColourSpace {
    /// `DeviceGray` read as `intent`'s black ink alone, where `intent` is a four-component
    /// profile; `None` for any other intent, whose grey the clause routes elsewhere and this
    /// tree does not yet read (a 'GRAY' intent already stands in for the family by its component
    /// count, and an 'RGB' one is section 6.2.4.3's second bullet, not taken here).
    #[must_use]
    pub fn grey_as_intent_black(intent: &Self) -> Option<Self> {
        match intent {
            Self::Icc { profile } if profile.channels() == 4 => Some(Self::Separation {
                inputs: 1,
                names: Arc::clone(&NAMES),
                alternate: Box::new(intent.clone()),
                tints: Tints::GreyAsBlack,
            }),
            _ => None,
        }
    }

    /// Whether this is [`Self::grey_as_intent_black`]'s space, whose single component is a grey
    /// level and not a tint.
    #[must_use]
    pub fn is_grey_as_intent_black(&self) -> bool {
        matches!(
            self,
            Self::Separation {
                tints: Tints::GreyAsBlack,
                ..
            }
        )
    }
}

/// §10.4.2.3's grey-to-CMYK formula: `(0, 0, 0, 1 − gray)`, the grey clamped to its range first.
pub(super) fn black_of(values: &[f32]) -> Vec<f32> {
    let grey = super::device_cmyk::channel(values.first().copied().unwrap_or(0.0));
    vec![0.0, 0.0, 0.0, 1.0 - grey]
}
