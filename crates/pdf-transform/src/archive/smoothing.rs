//! The images whose `/Interpolate` is written false, as the file's own `xmpMM:History` records them.
//!
//! ISO 19005-2 section 6.2.8.1 and ISO 19005-4 section 6.2.7.1 require an image dictionary's
//! `/Interpolate` to be false where it is stated. ISO 32000-2 §8.9.5.3 calls the entry
//! "only a hint", so a processor that ignores it is conforming, and a print carried none of the
//! smoothing; but this reader honours the hint (`doc/pdf-a-conversion-limits.md` section 4.7), so
//! under a window's profile the loss is real. Either way it is the loss `image-smoothing`
//! (`doc/adr/2498`), and the audit trail ISO 19005-2 section 6.6.6 and ISO 19005-4 section 6.7.5
//! ask a converter to keep names it: one entry in the output's own history, saying how many
//! images stopped asking for smoothing and which.

use std::fmt::Write as _;

use pdf_archive::Outcome;

use super::decision::Loss;

/// The requirement this module records the answer to.
const SITE: &str = "graphics/image-interpolation-is-off";

/// How many images the history entry names before it counts the rest, so a document with a
/// thousand smoothed images does not write a thousand references into its packet.
const MOST_NAMED: usize = 20;

/// The `xmpMM:History` parameters recording the images set to `/Interpolate false`.
///
/// `None` where the document states no such image. The count is the validator's own total, which
/// is exact even where its list of places was capped, and the references are the places it kept.
pub(super) fn history(input: &pdf_archive::Report) -> Option<String> {
    let judgement = input.failures().find(|judgement| judgement.id == SITE)?;
    let Outcome::Failed { places, total } = &judgement.outcome else {
        return None;
    };
    let mut out = format!(
        "{}: {} image(s) had /Interpolate set to false; loss {}",
        judgement.citation,
        total,
        Loss::ImageSmoothing.word()
    );
    let named: Vec<_> = places
        .iter()
        .filter_map(|finding| finding.place.object)
        .take(MOST_NAMED)
        .collect();
    for (index, id) in named.iter().enumerate() {
        let lead = if index == 0 { ": image" } else { ", image" };
        let _ = write!(out, "{lead} {} {} R", id.number, id.generation);
    }
    if *total > named.len() {
        let _ = write!(out, ", and {} more", total.saturating_sub(named.len()));
    }
    Some(out)
}
