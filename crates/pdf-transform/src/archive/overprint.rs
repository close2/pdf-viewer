//! The overprint mode ISO 19005 forbids under an `ICCBased` CMYK mark, set to 0 in the graphics
//! states that set it.
//!
//! # What the mode does, and to which marks
//!
//! ISO 19005-2 section 6.2.4.2 and ISO 19005-4 section 6.2.4.2 forbid overprint mode 1 while an
//! `ICCBased` CMYK space is used with overprinting on. ISO 32000-2 §8.6.7 says which marks the
//! mode acts on:
//!
//! > Non-zero overprint mode shall apply only to painting operations that use the current colour
//! > in the graphics state when the current colour space is DeviceCMYK (or is implicitly
//! > converted to DeviceCMYK ; see (8.6.5.7, "Implicit conversion of CIE-Based colour spaces").
//!
//! and §8.6.5.7 says when an `ICCBased` mark is one of them: "When this type of implicit
//! conversion is done, all of the semantics of the device colour space shall also apply" — the
//! conversion being one a processor may make where a source space "accurately represents the
//! particular output device being used", whose conditions its NOTE 3 says "cannot be specified
//! in PDF". That dependence on the processor is what the archive rule removes. This reader makes
//! the conversion only on a page or group compositing in a press, and such a mark is refused
//! below; everywhere else it converts each colour to the screen and the mode reaches no
//! `ICCBased` mark (`pdf_model`'s `ProcessTints::on` gives one tints only on a press whose
//! profile it states, ADR 1996).
//!
//! The graphics state that set the mode is the one lever: no operator sets it (§8.4.4's Table 56
//! has none), so writing `/OPM 0` there settles every mark it governed, and a mode can only fall,
//! so no new failure follows.
//!
//! # Two answers, decided by what else the state governed
//!
//! - **No `DeviceCMYK` mark painted under the mode with overprinting on.** Then the marks the mode
//!   reached are `ICCBased` ones on a page this reader draws in the screen's components, which it
//!   drew the same under either mode — rendered at 72 and 720 dpi, `102952.pdf`'s 77 pages are
//!   byte-identical with the mode at 0. A statement.
//! - **One was** — `0 0 0 1 k` text over a background is the common one — and §8.6.7 then has a
//!   CMYK device leave the backdrop's cyan, magenta and yellow under it where mode 0 paints them
//!   out. That is [`super::Loss::OverprintMode`], which the caller authorises.
//!
//! **A mark of either kind inside a page or group blending in four components is refused by
//! name**: there this reader composites by the mode — §11.7.4.3's special blend mode for a
//! `DeviceCMYK` mark, §8.6.5.7's paragraph for an `ICCBased` one — so the loss would be one the
//! window showed, and the profile that answers for the window cannot tell it from the one only a
//! press sees (`doc/adr/1978`). So is a mode set by a dictionary written into a resource
//! dictionary rather than as an object, since this conversion rewrites objects.

use std::collections::BTreeSet;

use pdf_archive::survey::Survey;
use pdf_syntax::Document;
use pdf_syntax::object::{Object, ObjectId};

use super::decision::Because;

/// Why the mode is not changed at all.
const STATE_NOT_REACHED: &str = "this file sets overprint mode 1 for an ICCBased CMYK mark from \
     a graphics state this conversion cannot change: the dictionary is written inside a resource \
     dictionary rather than as an object of its own, or the content walk stopped at one of its \
     bounds before it had read every mark, so the states it found may not be all of them. \
     Changing the ones it can reach would leave the requirement failing at the others, so none is \
     changed (doc/adr/1978)";

/// Why a mode whose effect this reader's window shows is not offered as a loss.
const SHOWN_IN_THE_WINDOW: &str = "a graphics state that sets overprint mode 1 for an ICCBased \
     CMYK mark governs a CMYK mark painted with overprinting on inside a page or group whose \
     blending space has four components, where this reader composites by the mode - ISO 32000-2 \
     11.7.4.3's special blend mode for a DeviceCMYK mark, 8.6.5.7's implicit conversion for an \
     ICCBased one - and its window draws the result. Setting the mode to 0 would move pixels the \
     window showed, so it is not offered; the archive profiles cannot tell that loss from the one \
     only a press sees (doc/adr/1978)";

/// The graphics states whose `OPM` becomes 0, and what else they governed.
#[derive(Debug, Default)]
pub(super) struct OverprintModes {
    /// Every graphics state parameter dictionary whose mode is set to 0.
    pub(super) at: BTreeSet<ObjectId>,
    /// The pages on which one of them governed a `DeviceCMYK` mark with overprinting on.
    pub(super) process_pages: BTreeSet<usize>,
}

impl OverprintModes {
    /// Whether changing the mode moves nothing the standard defines: no `DeviceCMYK` mark was
    /// painted under any of these states with overprinting on.
    pub(super) fn costs_nothing(&self) -> bool {
        self.process_pages.is_empty()
    }
}

/// The graphics states that set the forbidden mode, from the one content walk.
///
/// The survey is the population rather than the findings, because a finding names the page and
/// not the graphics state — and a survey a bound stopped is refused, because a state it never
/// reached would keep the mode.
pub(super) fn zeroing(document: &Document, survey: &Survey) -> Result<OverprintModes, Because> {
    let refused = Because::NotBuiltYet(STATE_NOT_REACHED);
    if survey.stopped_short() {
        return Err(refused);
    }
    let mut out = OverprintModes::default();
    for source in survey.icc_overprint_mode_sources() {
        let id = source.ok_or(refused)?;
        let Object::Dictionary(state) = document.get(id) else {
            return Err(refused);
        };
        if state.get("OPM").is_none() {
            return Err(refused);
        }
        out.at.insert(id);
    }
    if out.at.is_empty() {
        return Err(refused);
    }
    if survey.icc_overprint_in_four_components() {
        return Err(Because::NotBuiltYet(SHOWN_IN_THE_WINDOW));
    }
    for mark in survey.process_cmyk_overprints() {
        let governed = mark.mode_set_by.is_some_and(|id| out.at.contains(&id));
        if !governed {
            continue;
        }
        if mark.in_four_components {
            return Err(Because::NotBuiltYet(SHOWN_IN_THE_WINDOW));
        }
        out.process_pages.insert(mark.page);
    }
    Ok(out)
}
