//! Wording what a page could not draw.
//!
//! Trap 5 says every layer reports what it could not handle. Those reports are structured
//! values, one per clause that could not be honoured, and somewhere they have to become a
//! sentence a person reads. That happens here rather than in a host, for the reason every
//! shared decision in this tree lives one level down from its consumers: three hosts wording
//! [`Unsupported`] three ways would be three answers to one question, and two of them would be
//! out of date.
//!
//! It happens here rather than in `pdf-model` for the opposite reason. A sentence for a person
//! is a presentation decision — it chooses a length, a tone, and what to leave out — and
//! `pdf-model` is where PDF semantics live, not where a status bar is written.

use pdf_model::Unsupported;

/// One sentence naming what could not be drawn.
///
/// Deliberately says what is *missing from the page* rather than what the interpreter did,
/// because the question a person has is whether what they are looking at is the document.
pub(crate) fn describe(item: &Unsupported) -> String {
    match item {
        Unsupported::Text { operations } => {
            format!("{operations} text operation(s) were not drawn")
        }
        Unsupported::Image { name } => format!("an image ({name}) was not drawn"),
        Unsupported::Shading { name } => format!("a shading or pattern ({name}) was not drawn"),
        // Drawn, and worded so: the gradient is on the page and the wash the file asked for
        // around it is not (§8.7.4.3).
        Unsupported::ShadingBackground { detail } => format!(
            "a shading pattern's background colour was not painted around it, so less was \
             painted than asked for: {detail}"
        ),
        Unsupported::Operator { operator } => {
            format!("the operator {operator} is not implemented")
        }
        Unsupported::Font { detail } => format!("a font was not drawn: {detail}"),
        Unsupported::Content { issue } => {
            format!("part of the page's content is missing: {issue:?}")
        }
        Unsupported::Annotation { detail } => format!("an annotation was not drawn: {detail}"),
        Unsupported::LimitReached { limit } => {
            format!("the page reached this program's {limit} bound and was not drawn to the end")
        }
        // Both of these are drawn — they say the picture is *nearly* right, which is a
        // different sentence from the ones above and has to read like one.
        Unsupported::TextKnockout { glyphs } => format!(
            "{glyphs} glyph(s) were composited against the page rather than knocked out of it \
             (§9.3.8)"
        ),
        Unsupported::CompositedInParts { detail } => {
            format!("{detail} was composited in parts, which §11.6.2 makes one object")
        }
        Unsupported::SoftMask { detail } => {
            format!("a soft mask was not applied, so more was painted than asked for: {detail}")
        }
        Unsupported::TransparencyGroup { detail } => {
            format!("a transparency group was drawn as an isolated one: {detail}")
        }
        // The one report whose subject is the *file* rather than this program, which is why it
        // is worded as a statement about the document: §7.8.3 obliges a content stream's
        // resource dictionary to define every name its operators use, and this one did not.
        Unsupported::MissingResource { category, detail } => {
            format!("the document names a /{category} resource it does not define: {detail}")
        }
        // Drawn as well, and worded so: the bytes on the page are the producer's own and what
        // is missing is everything past the damage (§7.8.2, ADR 0359).
        Unsupported::DamagedContentStream { stream } => format!(
            "{} decoded only {} byte(s) before it was {:?}, and the rest of it is not on the page",
            stream.detail, stream.kept, stream.damage
        ),
        Unsupported::OptionalContent { detail } => {
            format!(
                "optional content was drawn because its visibility could not be decided: {detail}"
            )
        }
        // Drawn too, and the sentence has to say what kind of wrong the colours are: every mark
        // is where the producer put it and some of them carry a transfer function the standard
        // does not put there, or miss one it does (§10.5, §11.7.5.2).
        Unsupported::TransferFunction { detail } => {
            format!("a transfer function reached the wrong colours on this page: {detail}")
        }
        // The second report whose subject is the file, and the first about the page as a whole:
        // everything above says a mark is missing or wrong, and this says the *sheet* the marks
        // were placed on is not the producer's (§7.7.3.3, §7.7.3.4; ADR 0389). The
        // `PageDictionary` sentence below is the other one, and can be the *reason* for this one.
        Unsupported::MediaBox { detail } => {
            format!("the document states no page size, so one was chosen for it: {detail}")
        }
        // The fifth about the file, and the widest: every other one names a mark or the sheet,
        // and this says the *page dictionary* is only as much of itself as the file states
        // readably, so any of Table 31's entries may be a default this reader chose (§7.3.7).
        Unsupported::PageDictionary { detail } => {
            format!("this page's dictionary is damaged and was read only in part: {detail}")
        }
        // The third report whose subject is the file. §8.3.4 NOTE 3 says a noninvertible matrix
        // "can result in unpredictable behaviour" and nothing else; what it means on a screen is
        // that the mark's path lands on a line or a point, so this reads as a statement about
        // what the document asked for rather than about something this program declined.
        Unsupported::NoninvertibleMatrix { commands } => format!(
            "{commands} mark(s) are placed by a matrix with no inverse, which collapses them to \
             no area, so nothing was painted where they stand (§8.3.4)"
        ),
        // The fourth about the file, and the sentence has to name what the alternative would
        // have been: §8.5.2.1 gives such a segment no starting point, so drawing one means
        // choosing a place the document never wrote — which is what every library here does.
        Unsupported::UndefinedCurrentPoint { segments } => format!(
            "{segments} path segment(s) were written with no point to start from, which §8.5.2.1 \
             makes an error, so they were left off the page rather than drawn from a corner of it"
        ),
    }
}
