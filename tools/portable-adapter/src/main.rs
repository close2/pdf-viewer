//! Answers the portable conformance suite's `text` and `query` requests for this program.
//!
//! ```sh
//! portable-adapter text  fixture.pdf 0
//! portable-adapter query fixture.pdf destination Chap6.begin
//! portable-adapter query fixture.pdf page-label 7
//! portable-adapter query fixture.pdf field-value form.name
//! portable-adapter query fixture.pdf page-count
//! ```
//!
//! The protocol is the suite's and not this tree's (`portable-conformance/README.md`, RFC 0016
//! section 5): an answer is one line on stdout, a request this program cannot be asked exits 3,
//! and anything else that goes wrong exits 1 with the reason on stderr. A page is zero-based in
//! every request, as §12.4.2's page index is. Pages are drawn by `quorra-transform render`
//! rather than here, so that the pixels the suite judges are the ones the shipped command writes.
//!
//! **What it answers is the model's reading and nothing of its own.** Each request is one call
//! into `pdf_model`, the same call the viewer makes, so a verdict the suite reaches is a verdict
//! about the program a person runs rather than about a second implementation written for a test.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use std::fmt;
use std::io::Write as _;
use std::process::ExitCode;

use pdf_model::Pages;
use pdf_model::destination::Destination;
use pdf_model::page_label::PageLabels;
use pdf_model::view::ViewState;
use pdf_syntax::{Document, Name, Object};

/// Why a request was not answered, each with the exit status the protocol gives it.
#[derive(Debug)]
enum Failure {
    /// The arguments do not form a request.
    Usage(String),
    /// A request the protocol allows and this program cannot be asked.
    Cannot(String),
    /// The file could not be read or opened.
    Open(String),
    /// The question was understood, and the document holds no answer to it.
    NoAnswer(String),
}

impl fmt::Display for Failure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usage(what) => write!(formatter, "usage: {what}"),
            Self::Cannot(what) => write!(formatter, "not a request this program answers: {what}"),
            Self::Open(what) => write!(formatter, "cannot open the document: {what}"),
            Self::NoAnswer(what) => write!(formatter, "no answer: {what}"),
        }
    }
}

impl Failure {
    /// The protocol's exit status: 3 is "cannot be asked", which the runner shows as `n/a`.
    fn status(&self) -> u8 {
        match self {
            Self::Usage(_) => 2,
            Self::Cannot(_) => 3,
            Self::Open(_) | Self::NoAnswer(_) => 1,
        }
    }
}

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    match answer(&arguments) {
        Ok(line) => {
            let mut out = std::io::stdout().lock();
            match writeln!(out, "{line}") {
                Ok(()) => ExitCode::SUCCESS,
                Err(error) => {
                    eprintln!("portable-adapter: stdout: {error}");
                    ExitCode::FAILURE
                }
            }
        }
        Err(failure) => {
            eprintln!("portable-adapter: {failure}");
            ExitCode::from(failure.status())
        }
    }
}

/// One request's answer.
fn answer(arguments: &[String]) -> Result<String, Failure> {
    let [request, path, rest @ ..] = arguments else {
        return Err(Failure::Usage(
            "portable-adapter <text|query> <pdf> <arguments...>".to_owned(),
        ));
    };
    let bytes = std::fs::read(path).map_err(|error| Failure::Open(format!("{path}: {error}")))?;
    let document =
        Document::open(bytes).map_err(|error| Failure::Open(format!("{path}: {error}")))?;
    match request.as_str() {
        "text" => text(&document, rest),
        "query" => query(&document, rest),
        other => Err(Failure::Cannot(other.to_owned())),
    }
}

/// A zero-based page index from an argument.
fn index(word: Option<&String>) -> Result<usize, Failure> {
    let word = word.ok_or_else(|| Failure::Usage("a page index is missing".to_owned()))?;
    word.parse()
        .map_err(|_| Failure::Usage(format!("{word:?} is not a page index")))
}

/// ISO 32000-2 §9.10's text of one page, as the pass that draws its glyphs maps each code.
fn text(document: &Document, rest: &[String]) -> Result<String, Failure> {
    let page = index(rest.first())?;
    let page = Pages::new(document)
        .get(page)
        .ok_or_else(|| Failure::NoAnswer(format!("no page {page}")))?;
    Ok(pdf_model::content::interpret(document, &page).text)
}

/// The answer to one structure question.
fn query(document: &Document, rest: &[String]) -> Result<String, Failure> {
    let pages = Pages::new(document);
    match rest {
        [question] if question == "page-count" => Ok(pages.len().to_string()),
        [question, page] if question == "page-label" => {
            let page = index(Some(page))?;
            // §12.4.2: a document "may optionally define page labels", so a page without one has
            // no answer rather than its index printed as though it were a label.
            PageLabels::read(document)
                .label(page)
                .ok_or_else(|| Failure::NoAnswer(format!("page {page} has no label")))
        }
        [question, name] if question == "destination" => {
            // §12.3.2.4 keys the catalog's `/Dests` by name objects and the name tree by
            // strings, so the name is asked in both forms, the name object first.
            let as_name = Object::Name(Name::new(name.as_bytes()));
            let as_string = Object::String(name.as_bytes().into());
            let destination = Destination::read(document, &as_name)
                .or_else(|| Destination::read(document, &as_string))
                .ok_or_else(|| Failure::NoAnswer(format!("no destination named {name}")))?;
            let page = destination
                .page_index(document, &pages)
                .ok_or_else(|| Failure::NoAnswer(format!("{name} names no page")))?;
            Ok(format!("page {page}"))
        }
        [question, name] if question == "field-value" => ViewState::of(document)
            .field_value(document, name)
            .map(|shown| shown.text)
            .ok_or_else(|| Failure::NoAnswer(format!("no field named {name}"))),
        _ => Err(Failure::Cannot(rest.join(" "))),
    }
}
