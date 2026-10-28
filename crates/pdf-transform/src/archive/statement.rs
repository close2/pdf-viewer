//! The page `append-statement` appends: what a conversion removed, in facts computed from the file.
//!
//! # The permission
//!
//! `CLAUDE.md`'s first amendment to the authoring line, with the sentence the owner added in
//! `A706`: *a page may also state what an archival conversion removed, in facts computed from the
//! file and a fixed vocabulary naming the requirement and the loss; it carries no other text.* So
//! the page is ADR 1014's appended page ([`super::preserve`], which sets it, extends the page
//! labels and gives a tagged document its structure element) and its text is of two kinds only:
//!
//! - **facts computed from the source**: the object's number, its `/UF` or `/F` name, its decoded
//!   length, its `/Subtype` media type, the SHA-256 of its decoded bytes, the target, and the
//!   clause the requirement is stated in for that target;
//! - **a closed vocabulary fixed in this file and in the requirement table**: the title, the
//!   field labels, the requirement's identifier and `asks` sentence, the loss word with its one
//!   line, and the sentences the steps tried before were refused with — every one of them a
//!   constant of this program, so the page can be checked against the input exactly as
//!   `doc/adr/1014` section 3 asks of every appended page.
//!
//! The profile says whether the page is wanted and nothing about what it says (`doc/rfc/0014`
//! section 4); there is no key a configuration could put text on it with.
//!
//! # Characters
//!
//! A name and a media type are the file's bytes, and a page sets what a face holds. Every
//! character outside printable ASCII is written `<U+XXXX>`, which is a rule rather than a choice of
//! glyph, so the statement is set in any face and still names the object exactly.
//!
//! # Bounds
//!
//! `CLAUDE.md` principle 3's explicit budgets, each checked before the work it bounds and refused
//! by name past it: [`MOST_STATED_OBJECTS`] before any stream is decoded or hashed,
//! [`MOST_STATEMENT_BYTES`] before the text is laid out, and [`MOST_STATEMENT_PAGES`] on the
//! pages the layout took.

use std::fmt::Write as _;

use pdf_archive::{Target, table};
use pdf_syntax::Document;
use pdf_syntax::object::{Object, ObjectId};
use sha2::{Digest as _, Sha256};

use super::config::Statement;
use super::report::{AttachmentAnswer, AttachmentOutcome};

/// The most objects one statement lists.
///
/// Each costs a decode and a hash of the embedded file, so the count is bounded before either.
pub(super) const MOST_STATED_OBJECTS: usize = 32;

/// The longest statement, in bytes of text, laid out on pages.
pub(super) const MOST_STATEMENT_BYTES: usize = 64 * 1024;

/// The most pages one statement takes.
pub(super) const MOST_STATEMENT_PAGES: usize = 16;

/// Why a statement listing more objects than the bound is refused.
pub(super) const TOO_MANY_STATED: &str = "the statement of what this conversion removed would \
     list more than the thirty-two objects one statement lists (MOST_STATED_OBJECTS), so \
     the loss is not taken";

/// Why a statement longer than the bound is refused.
pub(super) const STATEMENT_TOO_LONG: &str = "the statement of what this conversion removed would \
     be longer than the 64 KiB of text one statement is laid out from (MOST_STATEMENT_BYTES), so \
     the loss is not taken";

/// Why a statement taking more pages than the bound is refused.
pub(super) const STATEMENT_TOO_MANY_PAGES: &str = "the statement of what this conversion removed \
     would take more than the sixteen pages one statement may append (MOST_STATEMENT_PAGES), so the \
     loss is not taken";

/// Why a row whose loss is stated is refused where no statement was composed for it.
pub(super) const NOTHING_STATED: &str = "the row's step is append-statement, and no statement \
     page was composed for the objects its loss took, so the loss is not taken unstated";

/// The page's title, before the target.
const TITLE: &str = "Removed in the conversion to";

/// The one line the `embedded-file` loss is described by on the page.
const EMBEDDED_FILE_LOSS: &str = "the embedded file is no longer in this document, and its bytes \
     went with it";

/// The subject the report and `xmpMM:History` name the page by.
pub(super) const SUBJECT: &str = "the statement of what this conversion removed";

/// The statement's text, or the bound it would pass.
///
/// `removed` is every outcome the step took the loss for; `asked` is what the row says was tried
/// before it and what the target passed over.
pub(super) fn text(
    document: &Document,
    target: Target,
    removed: &[&AttachmentOutcome],
    asked: &Statement,
) -> Result<String, &'static str> {
    if removed.len() > MOST_STATED_OBJECTS {
        return Err(TOO_MANY_STATED);
    }
    let mut out = format!("{TITLE} {target}\n");
    for (index, outcome) in removed.iter().enumerate() {
        if out.len() > MOST_STATEMENT_BYTES || outcome.name.len() > MOST_STATEMENT_BYTES {
            return Err(STATEMENT_TOO_LONG);
        }
        entry(&mut out, document, target, index, outcome, asked);
    }
    if out.len() > MOST_STATEMENT_BYTES {
        return Err(STATEMENT_TOO_LONG);
    }
    Ok(out)
}

/// One object's lines.
fn entry(
    out: &mut String,
    document: &Document,
    target: Target,
    index: usize,
    outcome: &AttachmentOutcome,
    asked: &Statement,
) {
    let requirement = table::requirements().find(|requirement| requirement.id == outcome.site);
    let citation = requirement
        .and_then(|requirement| requirement.clauses.citation(target))
        .unwrap_or_default();
    let asks = requirement.map_or("", |requirement| requirement.asks);
    let (media_type, bytes) = embedded(document, outcome.object);
    // Writing into a `String` cannot fail, so each `write!` result is discarded deliberately.
    let _ = write!(
        out,
        "\n{}. Requirement: {} ({citation})\n   Asks: {asks}\n   Loss: embedded-file, {EMBEDDED_FILE_LOSS}\n   \
         Object: file specification {} {}\n   Name: {}\n",
        index.saturating_add(1),
        outcome.site,
        outcome.object.number,
        outcome.object.generation,
        escaped(&outcome.name),
    );
    match &bytes {
        Some(bytes) => {
            let _ = write!(
                out,
                "   Decoded length: {} bytes\n   SHA-256: {}\n",
                bytes.len(),
                hex(&Sha256::digest(bytes))
            );
        }
        None => out.push_str("   Decoded length: the stream does not decode\n"),
    }
    let _ = writeln!(
        out,
        "   Media type: {}",
        media_type
            .as_deref()
            .map_or_else(|| "not stated".to_owned(), escaped)
    );
    match (&asked.before, &outcome.answer) {
        (Some(step), AttachmentAnswer::Removed(because)) => {
            let _ = writeln!(
                out,
                "   Tried before: {step}, which did not hold: {because}"
            );
        }
        _ => out.push_str("   Tried before: no step\n"),
    }
    for (step, why) in &asked.skipped {
        let _ = writeln!(out, "   Not admitted at this target: {step}, because {why}");
    }
}

/// §7.11.4's embedded file stream a specification names, `/UF` before `/F`: its `/Subtype` and
/// its decoded bytes, decoded under the document's own limits.
fn embedded(
    document: &Document,
    specification: ObjectId,
) -> (Option<String>, Option<std::sync::Arc<[u8]>>) {
    let object = document.get(specification);
    let Some(files) = object.as_dict().map(|dict| document.get_key(dict, "EF")) else {
        return (None, None);
    };
    let Some(files) = files.as_dict() else {
        return (None, None);
    };
    for key in ["UF", "F"] {
        if let Object::Stream(stream) = document.get_key(files, key) {
            let media_type = match document.get_key(&stream.dict, "Subtype") {
                Object::Name(name) => Some(String::from_utf8_lossy(name.as_bytes()).into_owned()),
                _ => None,
            };
            return (media_type, document.decoded_stream_data(&stream));
        }
    }
    (None, None)
}

/// A file's characters with everything outside printable ASCII written `<U+XXXX>`.
fn escaped(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for character in text.chars() {
        if character.is_ascii_graphic() || character == ' ' {
            out.push(character);
        } else {
            let _ = write!(out, "<U+{:04X}>", u32::from(character));
        }
    }
    out
}

/// Lower-case hexadecimal.
fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().saturating_mul(2));
    for byte in bytes {
        let _ = write!(out, "{byte:02x}");
    }
    out
}

#[cfg(test)]
mod tests {
    use pdf_archive::{Level, Target};
    use pdf_syntax::Document;
    use pdf_syntax::object::ObjectId;

    use super::super::config::Statement;
    use super::super::report::{AttachmentAnswer, AttachmentOutcome};
    use super::{MOST_STATED_OBJECTS, STATEMENT_TOO_LONG, TOO_MANY_STATED, escaped, text};

    fn a_document() -> Document {
        Document::open(
            b"%PDF-1.7\n1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n2 0 obj\n<< /Type \
              /Pages /Kids [] /Count 0 >>\nendobj\ntrailer\n<< /Root 1 0 R >>\n%%EOF\n"
                .to_vec(),
        )
        .expect("the fixture opens")
    }

    fn removed(number: u32, name: String) -> AttachmentOutcome {
        AttachmentOutcome {
            site: "embedded-files/embedded-file-is-itself-pdfa",
            object: ObjectId::new(number, 0),
            name,
            answer: AttachmentAnswer::Removed("the row discards it".to_owned()),
            annotations: Vec::new(),
            stated_on: None,
        }
    }

    const NOTHING_TRIED: Statement = Statement {
        before: None,
        skipped: Vec::new(),
    };

    #[test]
    fn one_object_past_the_count_is_refused_before_any_is_read() {
        let document = a_document();
        let outcomes: Vec<AttachmentOutcome> = (0..=MOST_STATED_OBJECTS)
            .map(|index| removed(u32::try_from(index).expect("small"), format!("f{index}")))
            .collect();
        let listed: Vec<&AttachmentOutcome> = outcomes.iter().collect();
        assert_eq!(
            text(&document, Target::Two(Level::B), &listed, &NOTHING_TRIED),
            Err(TOO_MANY_STATED)
        );
        assert!(
            text(
                &document,
                Target::Two(Level::B),
                &listed[..MOST_STATED_OBJECTS],
                &NOTHING_TRIED
            )
            .is_ok(),
            "the bound itself is stated"
        );
    }

    #[test]
    fn a_name_longer_than_the_statement_is_refused_before_it_is_written() {
        let document = a_document();
        let long = removed(9, "n".repeat(64 * 1024 + 1));
        assert_eq!(
            text(&document, Target::Two(Level::B), &[&long], &NOTHING_TRIED),
            Err(STATEMENT_TOO_LONG)
        );
    }

    #[test]
    fn a_character_outside_printable_ascii_is_written_as_its_code_point() {
        assert_eq!(escaped("Grün\tdatei.txt"), "Gr<U+00FC>n<U+0009>datei.txt");
    }
}
