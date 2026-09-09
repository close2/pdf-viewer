//! Asks a document a question and prints the answer as JSON.
//!
//! ```sh
//! pdf-retrieve document doc/ISO_32000-2_sponsored_EC3.pdf
//! pdf-retrieve outline  doc/ISO_32000-2_sponsored_EC3.pdf
//! pdf-retrieve sections doc/ISO_32000-2_sponsored_EC3.pdf
//! pdf-retrieve structure doc/ISO_32000-2_sponsored_EC3.pdf
//! pdf-retrieve archive-check somefile.pdf 4
//! pdf-retrieve page     doc/ISO_32000-2_sponsored_EC3.pdf 339 --annotations
//! pdf-retrieve text     doc/ISO_32000-2_sponsored_EC3.pdf --logical
//! pdf-retrieve section  doc/ISO_32000-2_sponsored_EC3.pdf 9.6.5.4 --annotations --no-artifacts
//! ```
//!
//! **The answer is JSON on stdout and nothing else**, so a caller may pipe it; a diagnostic goes
//! to stderr as an object of its own and the exit status is non-zero. That is the shape
//! `tools/conformance` and `tools/spec-errata` already have, and it is what `doc/todo/36`
//! chose over a network service: no listener, no socket, nothing added to the surface principle
//! 3 defends.
//!
//! **Its output is derived from whatever document it is pointed at.** Run against the
//! specifications under `doc/`, that output is the standard's text and ADR 0187's licence
//! discipline applies to it exactly as it does to `doc/md/` — it may be read, and it may not be
//! committed.

#![expect(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "a command-line tool whose entire output is a report"
)]

use std::path::PathBuf;

use pdf_retrieve::json::Value;
use pdf_retrieve::{Item, Note, PageText, Retrieval, SectionText, Wanted};

/// What went wrong, which is either the document's fault or the caller's.
#[derive(Debug, thiserror::Error)]
enum Refused {
    /// The retrieval itself could not be done.
    #[error("{0}")]
    Retrieval(#[from] pdf_retrieve::Error),
    /// A word in the first position that names no question.
    #[error("no such question: {0:?}")]
    Question(String),
    /// A word that names no PDF/A target.
    #[error("no such PDF/A target: {0:?} — try 2b, 2u, 2a, 4, 4f or 4e")]
    Target(String),
}

/// What the tool was asked for.
enum Question {
    /// What the file is: its pages, and how much of it is addressable.
    Document,
    /// §12.3.3's outline, as the tree it is.
    Outline,
    /// Every addressable section, flat, with the pages each occupies.
    Sections,
    /// §14.7's structure tree, flat, with the text each element marked.
    Structure,
    /// One page's text.
    Page(usize),
    /// Every page's text, in one answer.
    Text,
    /// One section's text.
    Section(String),
    /// Whether the document conforms to a stated part and level of ISO 19005.
    Archive(String),
}

fn main() -> std::process::ExitCode {
    match run() {
        Ok(report) => {
            print!("{}", report.render());
            std::process::ExitCode::SUCCESS
        }
        Err(error) => {
            eprint!(
                "{}",
                Value::Object(vec![("error".to_owned(), Value::text(error.to_string()))]).render()
            );
            std::process::ExitCode::FAILURE
        }
    }
}

/// Reads the arguments, opens the document and answers.
fn run() -> Result<Value, Refused> {
    let mut arguments = std::env::args().skip(1);
    let command = arguments.next().unwrap_or_default();
    let path = PathBuf::from(arguments.next().unwrap_or_default());
    let rest: Vec<String> = arguments.collect();
    let flag = |name: &str| rest.iter().any(|argument| argument == name);
    let value = |name: &str| {
        rest.iter()
            .position(|argument| argument == name)
            .map_or_else(
                || {
                    rest.iter()
                        .find_map(|argument| argument.strip_prefix(&format!("{name}=")))
                        .map(str::to_owned)
                },
                |at| rest.get(at.saturating_add(1)).cloned(),
            )
    };
    // The one positional argument after the path, which is a page number or an address.
    // `--subtype` is the only flag that takes a separate value, so a word right after it is
    // that flag's and not this one's — stated as the one exception rather than as a rule about
    // flags in general, because every other flag here is a bare switch.
    let subject = rest
        .iter()
        .enumerate()
        .filter(|(at, _)| {
            at.checked_sub(1)
                .and_then(|before| rest.get(before))
                .is_none_or(|before| before != VALUED)
        })
        .map(|(_, argument)| argument)
        .find(|argument| !argument.starts_with("--"))
        .cloned()
        .unwrap_or_default();
    let question = match command.as_str() {
        "document" => Question::Document,
        "outline" => Question::Outline,
        "sections" => Question::Sections,
        "structure" => Question::Structure,
        "archive-check" => Question::Archive(subject.clone()),
        "page" => Question::Page(subject.parse().unwrap_or_default()),
        "text" => Question::Text,
        "section" => Question::Section(subject),
        _ => {
            usage();
            return Err(Refused::Question(command));
        }
    };
    // `--subtype` implies `--annotations`, which `Wanted::wants_annotations` states once.
    let subtypes: Vec<String> = value(VALUED)
        .map(|list| list.split(',').map(str::to_owned).collect())
        .unwrap_or_default();
    let wanted = Wanted {
        annotations: flag("--annotations"),
        subtypes,
        drop_artifacts: flag("--no-artifacts"),
        logical: flag("--logical"),
    };
    let retrieval = Retrieval::open(&path)?;
    match question {
        Question::Document => Ok(document(&retrieval)),
        Question::Outline => Ok(outline(&retrieval)),
        Question::Sections => Ok(sections(&retrieval)),
        Question::Structure => Ok(structure(&retrieval)),
        Question::Archive(target) => archive(&retrieval, &target),
        Question::Page(index) => Ok(page(&retrieval.page(index, &wanted)?)),
        Question::Text => Ok(Value::Array(
            (0..retrieval.page_count())
                .map(|index| {
                    retrieval
                        .page(index, &wanted)
                        .map_or(Value::Null, |read| page(&read))
                })
                .collect(),
        )),
        Question::Section(address) => Ok(section(&retrieval.section(&address, &wanted)?)),
    }
}

/// The one flag that takes a value of its own rather than being a switch.
const VALUED: &str = "--subtype";

/// What the tool takes, printed where a caller got it wrong.
fn usage() {
    eprintln!(
        "usage: pdf-retrieve <document|outline|sections|structure|archive-check|text|page|section> \
         <file.pdf> \
         [<n>|<address>] \
         [--annotations] [--subtype <Name,Name>] [--no-artifacts] [--logical]"
    );
}

/// What the file is, and how much of it can be addressed.
fn document(retrieval: &Retrieval) -> Value {
    let sections = retrieval.sections();
    Value::Object(vec![
        ("pages".to_owned(), Value::count(retrieval.page_count())),
        (
            "outline_items".to_owned(),
            Value::count(retrieval.outline().visible_count()),
        ),
        ("sections".to_owned(), Value::count(sections.len())),
        (
            "numbered_sections".to_owned(),
            Value::count(
                sections
                    .iter()
                    .filter(|section| section.number.is_some())
                    .count(),
            ),
        ),
        (
            "first_page_label".to_owned(),
            Value::optional(retrieval.label(0)),
        ),
    ])
}

/// §12.3.3's outline as a tree, each item with the page it goes to.
fn outline(retrieval: &Retrieval) -> Value {
    // The flat sections carry the resolved page, so the tree is drawn from them by depth: an
    // item deeper than the one before it is that item's child. Rebuilding the nesting here
    // rather than re-resolving every destination is the same arithmetic `Outline::section_at`
    // avoids, one level up.
    Value::Array(
        retrieval
            .sections()
            .iter()
            .map(|section| {
                Value::Object(vec![
                    ("depth".to_owned(), Value::count(section.depth)),
                    ("number".to_owned(), Value::optional(section.number.clone())),
                    ("title".to_owned(), Value::text(section.title.clone())),
                    ("page".to_owned(), Value::count(section.first_page)),
                ])
            })
            .collect(),
    )
}

/// Every addressable section, with the pages it occupies.
fn sections(retrieval: &Retrieval) -> Value {
    Value::Array(
        retrieval
            .sections()
            .iter()
            .map(|section| {
                Value::Object(vec![
                    ("number".to_owned(), Value::optional(section.number.clone())),
                    ("title".to_owned(), Value::text(section.title.clone())),
                    ("depth".to_owned(), Value::count(section.depth)),
                    ("first_page".to_owned(), Value::count(section.first_page)),
                    ("last_page".to_owned(), Value::count(section.last_page)),
                    (
                        "ends_at".to_owned(),
                        Value::optional(section.ends_at.clone()),
                    ),
                ])
            })
            .collect(),
    )
}

/// §14.7's structure tree, with the text under each element.
///
/// A document that states no tree answers with `null` rather than with an empty array: §14.7.1
/// makes tagging optional, and "this document says nothing about its own structure" and "this
/// document's structure is empty" are different facts about a file.
fn structure(retrieval: &Retrieval) -> Value {
    let Some(structure) = retrieval.structure() else {
        return Value::Null;
    };
    Value::Object(vec![
        ("truncated".to_owned(), Value::Bool(structure.truncated)),
        (
            "items".to_owned(),
            Value::Array(
                structure
                    .items
                    .iter()
                    .map(|item| match item {
                        Item::Element {
                            depth,
                            kind,
                            stated,
                            page,
                        } => Value::Object(vec![
                            ("depth".to_owned(), Value::count(*depth)),
                            ("type".to_owned(), Value::optional(kind.clone())),
                            ("stated".to_owned(), Value::optional(stated.clone())),
                            ("page".to_owned(), page.map_or(Value::Null, Value::count)),
                        ]),
                        Item::Text { depth, page, text } => Value::Object(vec![
                            ("depth".to_owned(), Value::count(*depth)),
                            ("page".to_owned(), Value::count(*page)),
                            ("text".to_owned(), Value::text(text.clone())),
                        ]),
                    })
                    .collect(),
            ),
        ),
    ])
}

/// Where one failed requirement was not met, as JSON, and `null` for any other outcome.
///
/// A free function rather than a closure inside `archive`, which `clippy::pedantic`'s line
/// limit made the occasion for and which reads better anyway: the shape of a finding is a fact
/// about the report rather than about this one call.
fn archive_places(judgement: &pdf_archive::Judgement) -> Value {
    match &judgement.outcome {
        pdf_archive::Outcome::Failed { places, total } => Value::Object(vec![
            ("total".to_owned(), Value::count(*total)),
            (
                "places".to_owned(),
                Value::Array(
                    places
                        .iter()
                        .map(|finding| {
                            Value::Object(vec![
                                (
                                    "page".to_owned(),
                                    finding.place.page.map_or(Value::Null, Value::count),
                                ),
                                (
                                    "object".to_owned(),
                                    finding
                                        .place
                                        .object
                                        .map_or(Value::Null, |id| Value::count(id.number as usize)),
                                ),
                                (
                                    "name".to_owned(),
                                    Value::optional(finding.place.name.clone()),
                                ),
                                ("what".to_owned(), Value::text(finding.what.clone())),
                            ])
                        })
                        .collect(),
                ),
            ),
        ]),
        _ => Value::Null,
    }
}

/// One requirement's row: what it asks, which record changes it, and what became of it.
fn archive_row(judgement: &pdf_archive::Judgement) -> Value {
    let mut fields = vec![
        ("id".to_owned(), Value::text(judgement.id)),
        ("clause".to_owned(), Value::text(judgement.citation.clone())),
        ("asks".to_owned(), Value::text(judgement.asks)),
    ];
    if let Some(erratum) = judgement.amended_by {
        fields.push((
            "corrected_by".to_owned(),
            Value::Object(vec![
                ("standard".to_owned(), Value::text(erratum.standard)),
                ("issue".to_owned(), Value::count(erratum.issue as usize)),
                ("change".to_owned(), Value::text(erratum.change)),
            ]),
        ));
    }
    if let Some(clarification) = judgement.clarified_by {
        fields.push((
            "clarified_by".to_owned(),
            Value::Object(vec![
                ("note".to_owned(), Value::text(clarification.note)),
                ("item".to_owned(), Value::text(clarification.item)),
                ("parts".to_owned(), Value::text(clarification.parts)),
                ("change".to_owned(), Value::text(clarification.change)),
            ]),
        ));
    }
    match judgement.outcome {
        pdf_archive::Outcome::Unchecked(why) => {
            fields.push(("not_checked_because".to_owned(), Value::text(why)));
        }
        // A different key, because it is a different fact: `not_checked_because` names a
        // debt this crate owes, and nothing is owed here.
        pdf_archive::Outcome::OutsideValidation(why) => {
            fields.push(("outside_validation_because".to_owned(), Value::text(why)));
        }
        _ => fields.push(("found".to_owned(), archive_places(judgement))),
    }
    Value::Object(fields)
}

/// Whether a document conforms to a part and level of ISO 19005, and what the answer covers.
///
/// # Why the answer is not a boolean
///
/// ISO 19005-2 section 6.6.4 and ISO 19005-4 section 6.7.3 both close by saying that the `pdfaid`
/// properties do not themselves determine conformance — the determination is made against clause 5.
/// So this reports what was *checked*, what failed and where, and **which requirements were not
/// checked at all**, which `doc/questions/Q20` makes the discipline this crate lives by: a clean
/// verdict that did not say what it was clean over would be indistinguishable from a complete one.
///
/// A non-conforming document is an answer rather than an error, so the exit status stays zero
/// and the verdict is in the JSON. A caller wanting a shell test reads `conforms`.
fn archive(retrieval: &Retrieval, target: &str) -> Result<Value, Refused> {
    let target = if target.is_empty() {
        pdf_archive::Target::Four(pdf_archive::Flavour::Plain)
    } else {
        pdf_archive::Target::parse(target).ok_or_else(|| Refused::Target(target.to_owned()))?
    };
    let report = pdf_archive::check(retrieval.document(), target);
    Ok(Value::Object(vec![
        ("target".to_owned(), Value::text(target.to_string())),
        (
            "conforms".to_owned(),
            Value::Bool(report.verdict() == pdf_archive::Verdict::Conforms),
        ),
        (
            "requirements".to_owned(),
            Value::count(report.judgements.len()),
        ),
        ("checked".to_owned(), Value::count(report.checked())),
        (
            "failed".to_owned(),
            Value::Array(report.failures().map(archive_row).collect()),
        ),
        (
            "not_checked".to_owned(),
            Value::Array(report.unchecked().map(archive_row).collect()),
        ),
        // Separate from `not_checked`, because they are not a debt: the clause is published and
        // a clarification its own committee resolved puts it outside a validator's remit. See
        // `pdf_archive::Check::OutsideValidation`.
        (
            "outside_validation".to_owned(),
            Value::Array(report.outside_validation().map(archive_row).collect()),
        ),
    ]))
}

/// One page.
fn page(read: &PageText) -> Value {
    Value::Object(vec![
        ("page".to_owned(), Value::count(read.index)),
        ("label".to_owned(), Value::optional(read.label.clone())),
        ("order".to_owned(), Value::text(read.order.as_str())),
        ("complete".to_owned(), Value::Bool(read.complete)),
        (
            "unsupported".to_owned(),
            Value::Array(read.unsupported.iter().cloned().map(Value::Text).collect()),
        ),
        ("readback".to_owned(), shortfall(read.shortfall)),
        ("text".to_owned(), Value::text(read.text.clone())),
        ("annotations".to_owned(), notes(&read.annotations)),
    ])
}

/// What a page's or a section's codes cost the text beside it.
///
/// A count and never a refusal: ISO 32000-2 §9.10.2's own closing sentence is that where its
/// methods fail "there is no way to determine what the character code represents", so a code in
/// `unnamed` is an answer the standard gives rather than something this program failed to do — it
/// may not join `unsupported`, and a caller quoting the text still has to be able to see it.
/// `missing_glyphs` is the other direction and *is* a loss on the page; `blank_glyphs` is not one
/// at all, because a glyph the program describes as empty is how every font stores a space
/// (ADR 0270). ADR 0422.
///
/// `upright_vertical_forms` is a fourth thing again and is about the *shape* rather than about the
/// text: the character is there and reads back correctly, and the substituted face drew it
/// standing up where §9.7.5.1's NOTE says the producer's CID asked for a vertical form. A caller
/// extracting text is unaffected by it, which is exactly why it is reported here as its own
/// number rather than folded into either of the two above (ADR 0764).
fn shortfall(counts: pdf_model::content::Shortfall) -> Value {
    let pdf_model::content::Shortfall {
        unnamed,
        without_a_glyph,
        reaching_a_blank_glyph,
        without_a_vertical_form,
    } = counts;
    Value::Object(vec![
        ("unnamed".to_owned(), Value::count(unnamed.total())),
        (
            "unnamed_by_method".to_owned(),
            Value::Object(vec![
                (
                    "empty_mapping".to_owned(),
                    Value::count(unnamed.empty_mapping),
                ),
                (
                    "incomplete_to_unicode".to_owned(),
                    Value::count(unnamed.incomplete_to_unicode),
                ),
                (
                    "unlisted_name".to_owned(),
                    Value::count(unnamed.unlisted_name),
                ),
                ("unnamed_cid".to_owned(), Value::count(unnamed.unnamed_cid)),
                (
                    "unaddressable_cid".to_owned(),
                    Value::count(unnamed.unaddressable_cid),
                ),
                (
                    "unnamed_glyph".to_owned(),
                    Value::count(unnamed.unnamed_glyph),
                ),
            ]),
        ),
        ("missing_glyphs".to_owned(), Value::count(without_a_glyph)),
        (
            "blank_glyphs".to_owned(),
            Value::count(reaching_a_blank_glyph),
        ),
        (
            "upright_vertical_forms".to_owned(),
            Value::count(without_a_vertical_form),
        ),
    ])
}

/// One section.
fn section(read: &SectionText) -> Value {
    Value::Object(vec![
        (
            "number".to_owned(),
            Value::optional(read.section.number.clone()),
        ),
        ("title".to_owned(), Value::text(read.section.title.clone())),
        (
            "pages".to_owned(),
            Value::Array(read.pages.iter().copied().map(Value::count).collect()),
        ),
        (
            "ends_at".to_owned(),
            Value::optional(read.section.ends_at.clone()),
        ),
        ("order".to_owned(), Value::text(read.order.as_str())),
        ("trimmed_start".to_owned(), Value::Bool(read.trimmed_start)),
        ("trimmed_end".to_owned(), Value::Bool(read.trimmed_end)),
        ("complete".to_owned(), Value::Bool(read.complete)),
        (
            "unsupported".to_owned(),
            Value::Array(read.unsupported.iter().cloned().map(Value::Text).collect()),
        ),
        ("readback".to_owned(), shortfall(read.shortfall)),
        (
            "words".to_owned(),
            Value::count(read.text.split_whitespace().count()),
        ),
        ("text".to_owned(), Value::text(read.text.clone())),
        ("annotations".to_owned(), notes(&read.annotations)),
    ])
}

/// A list of annotations.
fn notes(notes: &[Note]) -> Value {
    Value::Array(
        notes
            .iter()
            .map(|note| {
                Value::Object(vec![
                    ("page".to_owned(), Value::count(note.page)),
                    ("subtype".to_owned(), Value::text(note.subtype.clone())),
                    ("title".to_owned(), Value::optional(note.title.clone())),
                    ("subject".to_owned(), Value::optional(note.subject.clone())),
                    ("created".to_owned(), Value::optional(note.created.clone())),
                    ("covers".to_owned(), Value::optional(note.covers.clone())),
                    (
                        "contents".to_owned(),
                        Value::optional(note.contents.clone()),
                    ),
                ])
            })
            .collect(),
    )
}
