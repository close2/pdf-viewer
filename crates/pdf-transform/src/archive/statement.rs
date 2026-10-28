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

use std::collections::BTreeSet;
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

/// The files one row's step removed, with what the row says was tried before it.
pub(super) type Files<'a> = (&'a [&'a AttachmentOutcome], &'a Statement);

/// The statement's text, or the bound it would pass.
///
/// `files` is every embedded file an embedded-file row's step took the loss for, with what that
/// row says was tried before it and what the target passed over; `entries` is every entry a
/// halftone row's step took, each with its own row's [`Statement`]. The files are listed first and
/// the entries after them, each in the order given.
pub(super) fn text(
    document: &Document,
    target: Target,
    files: Option<Files<'_>>,
    entries: &[GraphicsEntry<'_>],
) -> Result<String, &'static str> {
    let (removed, asked) = files.unwrap_or((&[], &NOTHING_TRIED));
    if removed.len().saturating_add(entries.len()) > MOST_STATED_OBJECTS {
        return Err(TOO_MANY_STATED);
    }
    let mut out = format!("{TITLE} {target}\n");
    for (index, outcome) in removed.iter().enumerate() {
        if out.len() > MOST_STATEMENT_BYTES || outcome.name.len() > MOST_STATEMENT_BYTES {
            return Err(STATEMENT_TOO_LONG);
        }
        entry(&mut out, document, target, index, outcome, asked);
    }
    let pages = pages_naming(
        document,
        &entries.iter().map(|entry| entry.object).collect::<Vec<_>>(),
    )?;
    for (offset, (stated, on)) in entries.iter().zip(&pages).enumerate() {
        if out.len() > MOST_STATEMENT_BYTES {
            return Err(STATEMENT_TOO_LONG);
        }
        graphics_entry(
            &mut out,
            document,
            target,
            removed.len().saturating_add(offset),
            stated,
            on,
        )?;
    }
    if out.len() > MOST_STATEMENT_BYTES {
        return Err(STATEMENT_TOO_LONG);
    }
    Ok(out)
}

/// What a row whose statement is its first step says was tried: nothing.
static NOTHING_TRIED: Statement = Statement {
    before: None,
    skipped: Vec::new(),
};

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

/// One entry a halftone row's step took from one object, to be stated (`doc/adr/2562`).
pub(super) struct GraphicsEntry<'a> {
    /// The requirement, one of [`super::config::STATED_GRAPHICS`].
    pub(super) site: &'static str,
    /// The graphics state or halftone the entry is taken from, in the source's numbering.
    pub(super) object: ObjectId,
    /// What the row says was tried before the loss.
    pub(super) asked: &'a Statement,
}

/// What the page says of one halftone row's entry: the object the entry is in, the key, what is
/// written in its place, the loss word and its one line — all fixed here, so the page states
/// nothing a reader could not check against the source and this table.
struct Taken {
    site: &'static str,
    holder: &'static str,
    key: &'static str,
    written: Option<&'static str>,
    loss: &'static str,
    line: &'static str,
}

/// The three halftone rows' entries, in [`super::config::STATED_GRAPHICS`]'s order.
///
/// `super::halftones` is what each population is: a halftone stating `HalftoneName` (ISO 32000-2
/// §10.6.5.1's pointer into a device's own halftones), a graphics state whose Table 57 `HT` names
/// a halftone of a type other than 1 or 5 and is written `Default`, and a graphics state stating
/// Table 57's `HTO`. The loss words are [`super::Loss::HalftoneName`]'s and
/// [`super::Loss::Halftone`]'s.
const TAKEN: [Taken; 3] = [
    Taken {
        site: "graphics/no-halftone-name",
        holder: "halftone",
        key: "HalftoneName",
        written: None,
        loss: "halftone-name",
        line: "the halftone's name is no longer in this document, so a press holding a halftone of \
               that name screens with the entries this halftone states instead",
    },
    Taken {
        site: "graphics/halftone-type-is-one-or-five",
        holder: "graphics state",
        key: "HT",
        written: Some("Default"),
        loss: "halftone",
        line: "the graphics state names the halftone in effect at the start of the page, and the \
               halftone it named left the file with every object only it reached",
    },
    Taken {
        site: "graphics/no-halftone-origin-in-a-graphics-state",
        holder: "graphics state",
        key: "HTO",
        written: None,
        loss: "halftone",
        line: "the graphics state's halftone origin is no longer in this document, so a press \
               places its screen at its own origin",
    },
];

/// Why an entry whose row the table above does not hold is not stated.
const NOT_A_STATED_ROW: &str = "the statement of what this conversion removed was asked to list \
     an entry of a row whose loss it does not state, so the loss is not taken";

/// Why an entry the source does not state is not listed.
const NOT_IN_THE_SOURCE: &str = "the statement of what this conversion removed was asked to list \
     an entry the source's object does not state, so the loss is not taken";

/// One halftone row's lines, every value read from the source here rather than handed in.
fn graphics_entry(
    out: &mut String,
    document: &Document,
    target: Target,
    index: usize,
    stated: &GraphicsEntry<'_>,
    pages: &[usize],
) -> Result<(), &'static str> {
    let taken = TAKEN
        .iter()
        .find(|taken| taken.site == stated.site)
        .ok_or(NOT_A_STATED_ROW)?;
    let object = document.get(stated.object);
    let dict = match &object {
        Object::Dictionary(dict) => dict,
        Object::Stream(stream) => &stream.dict,
        _ => return Err(NOT_IN_THE_SOURCE),
    };
    let value = dict.get(taken.key).ok_or(NOT_IN_THE_SOURCE)?;
    // The value is the source's own object, already held whole, so writing it costs what the
    // file already cost; the bound is checked before it joins the text.
    let mut written = Vec::new();
    pdf_syntax::write::object(value, &mut written);
    if written.len() > MOST_STATEMENT_BYTES {
        return Err(STATEMENT_TOO_LONG);
    }
    let requirement = table::requirements().find(|requirement| requirement.id == taken.site);
    let citation = requirement
        .and_then(|requirement| requirement.clauses.citation(target))
        .unwrap_or_default();
    let asks = requirement.map_or("", |requirement| requirement.asks);
    let _ = write!(
        out,
        "\n{}. Requirement: {} ({citation})\n   Asks: {asks}\n   Loss: {}, {}\n   Object: {} {} {}\n",
        index.saturating_add(1),
        taken.site,
        taken.loss,
        taken.line,
        taken.holder,
        stated.object.number,
        stated.object.generation,
    );
    match taken.written {
        Some(now) => {
            let _ = writeln!(out, "   Entry written {now}: {}", taken.key);
        }
        None => {
            let _ = writeln!(out, "   Entry removed: {}", taken.key);
        }
    }
    // `pdf_syntax::write::object` writes a string in hexadecimal and a name with its `#xx`
    // escapes, so the value is printable ASCII as written; `escaped` holds that if it changes.
    let _ = writeln!(
        out,
        "   Value in the source: {}",
        escaped(&String::from_utf8_lossy(&written))
    );
    if pages.is_empty() {
        out.push_str("   In the resources of: no page\n");
    } else {
        let _ = writeln!(
            out,
            "   In the resources of page(s): {}",
            pages
                .iter()
                .map(|page| page.saturating_add(1).to_string())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    out.push_str("   Tried before: ");
    match &stated.asked.before {
        Some(step) => {
            let _ = writeln!(out, "{step}");
        }
        None => out.push_str("no step\n"),
    }
    for (step, why) in &stated.asked.skipped {
        let _ = writeln!(out, "   Not admitted at this target: {step}, because {why}");
    }
    Ok(())
}

/// The most objects [`pages_naming`] visits over every page of the document, each page and each
/// value inside a dictionary or array counted as one.
///
/// `CLAUDE.md` principle 3's explicit budget: the walk is per page, so a document whose pages
/// share one resource graph pays for it once a page, and a page count the tree only claims costs
/// a visit per page asked for. A walk past it refuses the loss by name rather than stating pages
/// it did not finish looking at.
pub(super) const MOST_VISITS: usize = 1 << 20;

/// Why the pages using an entry were not worked out.
pub(super) const PAGES_NOT_WALKED: &str = "the statement of what this conversion removed would \
     visit more than the 1 048 576 objects and values (MOST_VISITS) it may visit to name the pages \
     whose resources reach each object it lists, so the loss is not taken";

/// The zero-based pages whose resources reach each of `objects`, one list per object.
///
/// A page reaches an object where it is reachable from §7.8.3's resource dictionary in force for
/// the page (Table 31's `Resources`, inherited where the page states none) or from the appearance
/// dictionary of one of the page's annotations (§12.5.5), through any chain of references — a
/// graphics state named by a form `XObject`'s own resources, or a halftone named by a graphics
/// state, among them. Nothing else on a page is followed, so a page naming another page's
/// annotation or destination does not reach that page's resources.
fn pages_naming(
    document: &Document,
    objects: &[ObjectId],
) -> Result<Vec<Vec<usize>>, &'static str> {
    pages_naming_within(document, objects, MOST_VISITS)
}

/// [`pages_naming`] under a budget of `most` visits, so that a test can stand at the bound.
fn pages_naming_within(
    document: &Document,
    objects: &[ObjectId],
    most: usize,
) -> Result<Vec<Vec<usize>>, &'static str> {
    let mut out = vec![Vec::new(); objects.len()];
    if objects.is_empty() {
        return Ok(out);
    }
    let mut visits = Visits { done: 0, most };
    // One walk of the tree for every page: `Pages::get` per index reads a flat `/Kids` array from
    // its start each time, n²/2 node reads for a root holding n leaves (ADR 1953), and those reads
    // are not visits this budget counts.
    for (index, page) in pdf_model::Pages::new(document)
        .every()
        .into_iter()
        .enumerate()
    {
        visit(&mut visits)?;
        let Some(page) = page else {
            continue;
        };
        let mut pending = Vec::new();
        for (_, value) in page.resources.iter() {
            references_in(value, &mut pending, &mut visits, 0)?;
        }
        if let Object::Array(annotations) = document.get_key(&page.dict, "Annots") {
            for annotation in &annotations {
                let annotation = match annotation {
                    Object::Reference(id) => document.get(*id),
                    direct => direct.clone(),
                };
                if let Some(appearance) = annotation.as_dict().and_then(|dict| dict.get("AP")) {
                    references_in(appearance, &mut pending, &mut visits, 0)?;
                }
            }
        }
        let mut seen = BTreeSet::new();
        while let Some(id) = pending.pop() {
            if !seen.insert(id) {
                continue;
            }
            if let Some(at) = objects.iter().position(|object| *object == id) {
                out[at].push(index);
            }
            references_in(&document.get(id), &mut pending, &mut visits, 0)?;
        }
    }
    Ok(out)
}

/// The deepest a direct value is followed into, `pdf_syntax::Limits::DEFAULT`'s `max_depth`: the
/// parser admitted nothing deeper, so a value past it is not one this document holds.
const MOST_DEPTH: usize = 256;

/// The visits [`pages_naming_within`] has made, and the most it may.
struct Visits {
    done: usize,
    most: usize,
}

/// One more visit, or [`PAGES_NOT_WALKED`] past the budget.
fn visit(visits: &mut Visits) -> Result<(), &'static str> {
    visits.done = visits.done.saturating_add(1);
    if visits.done > visits.most {
        return Err(PAGES_NOT_WALKED);
    }
    Ok(())
}

/// Every reference inside one value, its direct dictionaries, arrays and stream dictionary
/// included, each value counted as a visit.
fn references_in(
    value: &Object,
    into: &mut Vec<ObjectId>,
    visits: &mut Visits,
    depth: usize,
) -> Result<(), &'static str> {
    visit(visits)?;
    if depth > MOST_DEPTH {
        return Err(PAGES_NOT_WALKED);
    }
    let below = depth.saturating_add(1);
    match value {
        Object::Reference(id) => into.push(*id),
        Object::Dictionary(dict) => {
            for (_, value) in dict.iter() {
                references_in(value, into, visits, below)?;
            }
        }
        Object::Stream(stream) => {
            for (_, value) in stream.dict.iter() {
                references_in(value, into, visits, below)?;
            }
        }
        Object::Array(items) => {
            for item in items {
                references_in(item, into, visits, below)?;
            }
        }
        _ => {}
    }
    Ok(())
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
    use super::{
        MOST_STATED_OBJECTS, MOST_VISITS, PAGES_NOT_WALKED, STATEMENT_TOO_LONG, TOO_MANY_STATED,
        escaped, pages_naming_within, text,
    };

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
            text(
                &document,
                Target::Two(Level::B),
                Some((&listed, &NOTHING_TRIED)),
                &[]
            ),
            Err(TOO_MANY_STATED)
        );
        assert!(
            text(
                &document,
                Target::Two(Level::B),
                Some((&listed[..MOST_STATED_OBJECTS], &NOTHING_TRIED)),
                &[]
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
            text(
                &document,
                Target::Two(Level::B),
                Some((&[&long], &NOTHING_TRIED)),
                &[]
            ),
            Err(STATEMENT_TOO_LONG)
        );
    }

    /// One page whose graphics state is named through a form `XObject`'s own resources, and whose
    /// resources reach a cycle back to themselves.
    fn a_page_reaching_a_graphics_state() -> Document {
        Document::open(
            b"%PDF-1.7\n1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n2 0 obj\n<< /Type \
              /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n3 0 obj\n<< /Type /Page /Parent 2 0 R \
              /MediaBox [0 0 10 10] /Resources 4 0 R >>\nendobj\n4 0 obj\n<< /XObject << /X0 \
              5 0 R >> /Again 4 0 R >>\nendobj\n5 0 obj\n<< /Type /XObject /Subtype /Form \
              /BBox [0 0 1 1] /Resources << /ExtGState << /G0 6 0 R >> >> /Length 0 >>\nstream\n\
              \nendstream\nendobj\n6 0 obj\n<< /Type /ExtGState /HTO [0 0] >>\nendobj\n\
              7 0 obj\n<< /Type /ExtGState /HTO [1 1] >>\nendobj\ntrailer\n<< /Root 1 0 R >>\n\
              %%EOF\n"
                .to_vec(),
        )
        .expect("the fixture opens")
    }

    #[test]
    fn the_pages_naming_an_object_are_walked_under_the_budget_and_refused_past_it() {
        let document = a_page_reaching_a_graphics_state();
        let objects = [ObjectId::new(6, 0), ObjectId::new(7, 0)];
        let named = pages_naming_within(&document, &objects, MOST_VISITS);
        assert_eq!(
            named,
            Ok(vec![vec![0], Vec::new()]),
            "6 through the form, 7 by nothing"
        );
        // The smallest budget the walk fits in, found from below: one fewer is refused by name.
        let fits = (1..MOST_VISITS)
            .find(|most| pages_naming_within(&document, &objects, *most).is_ok())
            .expect("the walk fits in the budget");
        assert_eq!(pages_naming_within(&document, &objects, fits), named);
        assert_eq!(
            pages_naming_within(&document, &objects, fits - 1),
            Err(PAGES_NOT_WALKED)
        );
    }

    #[test]
    fn a_character_outside_printable_ascii_is_written_as_its_code_point() {
        assert_eq!(escaped("Grün\tdatei.txt"), "Gr<U+00FC>n<U+0009>datei.txt");
    }
}
