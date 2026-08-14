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
    }
}
