//! Which XFA forms carry scripts, at which of the template's sites, in which language, and what a
//! `FormCalc` script among them uses — the census ADR 2482 chooses `FormCalc`'s first subset on.
//!
//! RFC 0011 section 5 owes §K.2's calculations and events in two steps, JavaScript through
//! `pdf-script` and `FormCalc` in this crate, and which comes first, and how much of `FormCalc`,
//! has to rest on what the world's forms carry. So this walk asks, of every document whose
//! interactive form dictionary states an `/XFA` resource (§K.2's two forms, the one stream or the
//! packet array):
//!
//! 1. **Is it a hybrid or a dynamic form?** `/NeedsRendering` and the template's own occurrence
//!    and `subformSet` test, as `pdf_model::xfa::Template::dynamic` answers it (XFA 3.3 page
//!    333, ADR 2380) — the static half is what this program runs.
//! 2. **Where is each script, and in which language?** A `script` element's parent — a
//!    `calculate`, a `validate`, an `event` (with its `activity`), a `variables` — and its
//!    `contentType`, absent meaning `FormCalc` (XFA 3.3 page 412).
//! 3. **What does a `FormCalc` script use?** This crate's own lexer over its text: the keywords and
//!    operators, every function call by name (case folded, as page 1076 makes function names), and
//!    the accessor shapes of chapter 3's SOM — a `$`- or `!`-rooted name, `[*]`, `..`, `.#`. A
//!    script the lexer refuses is counted as such, so the lexer is measured as well as used.
//!
//! ```sh
//! find -L doc/pdf.js/test/pdfs -maxdepth 1 -name '*.pdf' > /tmp/paths
//! find -L doc/corpora corpus-cache -name '*.pdf' >> /tmp/paths
//! cargo run --release -p pdf-formcalc --example xfa_script_census -- @/tmp/paths
//! ```
//!
//! An argument beginning with `@` names a file of paths, one to a line; a directory is walked
//! recursively. Every document is read in parallel, with a panic counted rather than fatal, and a
//! file over 128 MiB counted rather than read.

#![expect(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "a measurement whose output is its purpose"
)]
#![expect(
    clippy::arithmetic_side_effects,
    reason = "tallies of documents, scripts and bytes of a walk bounded by the file system, \
              nowhere near a 64-bit counter's end"
)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use pdf_formcalc::Stop;
use pdf_formcalc::lex::{self, Kind};
use pdf_model::xfa::{Change, Language, Resource};
use pdf_syntax::{Document, Object};
use rayon::prelude::*;

/// The largest file read; a crawl holds artefacts of several gigabytes.
const MAX_FILE_BYTES: u64 = 128 << 20;

/// How many distinct scripts of one population and site are printed whole.
const PRINTED: usize = 60;

/// The adapter between `pdf-model`'s form and this crate's host, the file the hybrid-form test
/// includes too, so that what is counted here is what is tested there (ADR 2482).
#[path = "../tests/common/hosted.rs"]
mod hosted;

/// The tallies of one document or of the whole walk.
#[derive(Default)]
struct Counts {
    opened: u64,
    unopenable: u64,
    too_large: u64,
    panicked: u64,
    /// Documents by population: `hybrid`, `dynamic`, or `unread` where the template did not read.
    forms: BTreeMap<String, Vec<String>>,
    /// Scripts by `population site language`.
    scripts: BTreeMap<String, u64>,
    /// `FormCalc` tokens by `population site use`.
    uses: BTreeMap<String, u64>,
    /// Every distinct script text by `population site language`, with its count.
    texts: BTreeMap<String, BTreeMap<String, u64>>,
    /// What became of each hybrid form's `FormCalc` `calculate` script run against its own form,
    /// by outcome.
    outcomes: BTreeMap<String, u64>,
    /// One line per document that carries a `calculate` script, by population.
    calculating: BTreeMap<String, Vec<String>>,
}

impl Counts {
    fn absorb(&mut self, other: Self) {
        self.opened += other.opened;
        self.unopenable += other.unopenable;
        self.too_large += other.too_large;
        self.panicked += other.panicked;
        for (key, mut paths) in other.forms {
            self.forms.entry(key).or_default().append(&mut paths);
        }
        for (key, n) in other.scripts {
            *self.scripts.entry(key).or_default() += n;
        }
        for (key, n) in other.outcomes {
            *self.outcomes.entry(key).or_default() += n;
        }
        for (key, n) in other.uses {
            *self.uses.entry(key).or_default() += n;
        }
        for (key, mut lines) in other.calculating {
            self.calculating.entry(key).or_default().append(&mut lines);
        }
        for (key, texts) in other.texts {
            let into = self.texts.entry(key).or_default();
            for (text, n) in texts {
                *into.entry(text).or_default() += n;
            }
        }
    }
}

/// One `script` element of a template.
struct Script {
    /// `calculate`, `validate`, `event:<activity>`, `variables`, or the parent's own name.
    site: String,
    /// `FormCalc`, `JavaScript`, or the `contentType` as written.
    language: String,
    text: String,
}

/// The template's text: the `template` packet of the array form, the whole XDP of the stream.
fn template_text(document: &Document) -> Option<String> {
    let catalog = document.catalog().ok()?;
    let form = document.get_key(&catalog, "AcroForm");
    let form = form.as_dict()?;
    let id = match form.get("XFA")? {
        Object::Reference(id) => match document.get(*id) {
            Object::Array(packets) => named(&packets, "template")?,
            Object::Stream(_) => *id,
            _ => return None,
        },
        Object::Array(packets) => named(packets, "template")?,
        _ => return None,
    };
    let object = document.get(id);
    let bytes = document.decoded_stream_data(object.as_stream()?)?;
    Some(String::from_utf8_lossy(&bytes).into_owned())
}

fn named(packets: &[Object], wanted: &str) -> Option<pdf_syntax::ObjectId> {
    packets.chunks_exact(2).find_map(|pair| match pair {
        [Object::String(name), Object::Reference(id)]
            if pdf_syntax::text_string(name) == wanted =>
        {
            Some(*id)
        }
        _ => None,
    })
}

/// XML's five predefined entities and its character references.
fn unescape(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut rest = raw;
    while let Some(amp) = rest.find('&') {
        out.push_str(&rest[..amp]);
        rest = &rest[amp..];
        let Some(semi) = rest.find(';') else { break };
        let entity = &rest[1..semi];
        let resolved = match entity {
            "lt" => Some('<'),
            "gt" => Some('>'),
            "amp" => Some('&'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            _ => entity
                .strip_prefix("#x")
                .and_then(|hex| u32::from_str_radix(hex, 16).ok())
                .or_else(|| entity.strip_prefix('#').and_then(|dec| dec.parse().ok()))
                .and_then(char::from_u32),
        };
        if let Some(c) = resolved {
            out.push(c);
            rest = &rest[semi + 1..];
        } else {
            out.push('&');
            rest = &rest[1..];
        }
    }
    out.push_str(rest);
    out
}

/// Every `script` element of the template, with its parent and language.
fn scripts(template: &str) -> Vec<Script> {
    let mut stack: Vec<(String, Option<String>)> = Vec::new();
    let mut found = Vec::new();
    let mut current: Option<Script> = None;
    let mut pending: Option<(String, Option<String>)> = None;
    for token in xmlparser::Tokenizer::from(template) {
        let Ok(token) = token else { break };
        match token {
            xmlparser::Token::ElementStart { local, .. } => {
                pending = Some((local.as_str().to_owned(), None));
                if local.as_str() == "script" {
                    let site = match stack.last() {
                        Some((parent, Some(activity))) if parent == "event" => {
                            format!("event:{activity}")
                        }
                        Some((parent, _)) => parent.clone(),
                        None => "root".to_owned(),
                    };
                    current = Some(Script {
                        site,
                        language: "FormCalc".to_owned(),
                        text: String::new(),
                    });
                }
            }
            xmlparser::Token::Attribute { local, value, .. } => {
                if let Some((name, activity)) = pending.as_mut()
                    && name == "event"
                    && local.as_str() == "activity"
                {
                    *activity = Some(value.as_str().to_owned());
                }
                if local.as_str() == "contentType"
                    && let Some(script) = current.as_mut()
                {
                    script.language = match value.as_str() {
                        "application/x-javascript" => "JavaScript".to_owned(),
                        "application/x-formcalc" => "FormCalc".to_owned(),
                        other => other.to_owned(),
                    };
                }
            }
            xmlparser::Token::ElementEnd { end, .. } => match end {
                xmlparser::ElementEnd::Open => {
                    if let Some(open) = pending.take() {
                        stack.push(open);
                    }
                }
                xmlparser::ElementEnd::Empty => {
                    if pending.take().is_some_and(|(name, _)| name == "script")
                        && let Some(script) = current.take()
                    {
                        found.push(script);
                    }
                }
                xmlparser::ElementEnd::Close(_, local) => {
                    stack.pop();
                    if local.as_str() == "script"
                        && let Some(script) = current.take()
                    {
                        found.push(script);
                    }
                }
            },
            xmlparser::Token::Text { text } => {
                if let Some(script) = current.as_mut() {
                    script.text.push_str(&unescape(text.as_str()));
                }
            }
            xmlparser::Token::Cdata { text, .. } => {
                if let Some(script) = current.as_mut() {
                    script.text.push_str(text.as_str());
                }
            }
            _ => {}
        }
    }
    found
}

/// Whether a JavaScript text is nothing but comments and white space: a producer that comments a
/// calculation out leaves a `script` element that runs nothing.
fn inert(text: &str) -> bool {
    let mut rest = text.trim_start();
    loop {
        if let Some(after) = rest.strip_prefix("//") {
            rest = after
                .find('\n')
                .map_or("", |end| &after[end..])
                .trim_start();
        } else if let Some(after) = rest.strip_prefix("/*") {
            match after.find("*/") {
                Some(end) => rest = after[end + 2..].trim_start(),
                None => return true,
            }
        } else {
            return rest.is_empty();
        }
    }
}

/// The lexical uses of one `FormCalc` script, each a key of [`Counts::uses`].
fn uses(text: &str) -> Vec<String> {
    let tokens = match lex::tokens(text) {
        Ok(tokens) => tokens,
        Err(error) => return vec![format!("lexer refuses: {error}")],
    };
    let mut out = Vec::new();
    for (i, token) in tokens.iter().enumerate() {
        let next = tokens.get(i + 1).map(|t| &t.kind);
        let previous = i.checked_sub(1).map(|p| &tokens[p].kind);
        let called = matches!(next, Some(Kind::Punct(lex::Punct::OpenParen)));
        let after_dot = matches!(
            previous,
            Some(Kind::Punct(
                lex::Punct::Dot | lex::Punct::DotDot | lex::Punct::DotHash
            ))
        );
        match &token.kind {
            Kind::Identifier(name) if called && after_dot => {
                out.push(format!("method {name}()"));
            }
            Kind::Identifier(name) if called => {
                out.push(format!("call {}()", name.to_ascii_lowercase()));
            }
            Kind::Identifier(name) if name.starts_with('$') || name.starts_with('!') => {
                out.push(format!("root {name}"));
            }
            Kind::Identifier(_) => {}
            Kind::Keyword(keyword) => out.push(format!("keyword {keyword:?}")),
            Kind::Punct(punct) => out.push(format!("operator {}", punct.spelling())),
            Kind::Number(_) => out.push("literal number".to_owned()),
            Kind::String(_) => out.push("literal string".to_owned()),
        }
    }
    out
}

/// Runs every `FormCalc` `calculate` script of a hybrid form once, in document order, against the
/// values its `datasets` packet holds, and tallies what became of each.
fn evaluate(document: &Document, resource: &Resource, counts: &mut Counts) {
    let fields: Vec<(String, Change)> = pdf_model::view::widgets_by_field_name(document)
        .into_keys()
        .map(|name| {
            let held = resource.datasets.value(&resource.template, &name);
            let change = held.map_or(Change::Cleared, Change::Text);
            (name, change)
        })
        .collect();
    let mut form = resource.form(&fields);
    for calculation in form.calculations() {
        if calculation.language != Language::FormCalc {
            continue;
        }
        let before = form.value(calculation.node);
        let outcome = match hosted::calculate(&mut form, &calculation) {
            Ok(_) if form.value(calculation.node) == before => {
                "evaluated, value unchanged".to_owned()
            }
            Ok(_) => "evaluated, value changed".to_owned(),
            Err(Stop::Parse(_)) => "does not parse".to_owned(),
            Err(Stop::Unsupported(what)) => format!("refused: {what}"),
            Err(Stop::Exception(why)) => {
                format!(
                    "raised an error: {}",
                    why.chars().take(70).collect::<String>()
                )
            }
            Err(Stop::Exceeded(what)) => format!("exceeded the {what} budget"),
        };
        *counts.outcomes.entry(outcome).or_default() += 1;
    }
}

fn measure(path: &Path) -> Counts {
    let mut counts = Counts::default();
    if std::fs::metadata(path).is_ok_and(|meta| meta.len() > MAX_FILE_BYTES) {
        counts.too_large = 1;
        return counts;
    }
    let Some(document) = std::fs::read(path)
        .ok()
        .and_then(|bytes| Document::open(bytes).ok())
    else {
        counts.unopenable = 1;
        return counts;
    };
    counts.opened = 1;
    let Some(stated) = pdf_model::xfa::stated(&document) else {
        return counts;
    };
    if !stated.resource {
        return counts;
    }
    let resource = Resource::read(&document);
    let population = match &resource {
        _ if stated.needs_rendering => "dynamic",
        Some(Ok(resource)) if resource.template.dynamic().is_some() => "dynamic",
        Some(Ok(_)) => "hybrid",
        _ => "unread",
    };
    if let (true, Some(Ok(resource))) = (population == "hybrid", &resource) {
        evaluate(&document, resource, &mut counts);
    }
    counts
        .forms
        .insert(population.to_owned(), vec![path.display().to_string()]);
    let Some(template) = template_text(&document) else {
        return counts;
    };
    let (mut formcalc, mut live, mut dead) = (0, 0, 0);
    for mut script in scripts(&template) {
        if script.language == "JavaScript" && inert(&script.text) {
            "JavaScript-inert".clone_into(&mut script.language);
        }
        if script.site == "calculate" {
            match script.language.as_str() {
                "FormCalc" => formcalc += 1,
                "JavaScript-inert" => dead += 1,
                _ => live += 1,
            }
        }
        let key = format!("{population} {} {}", script.site, script.language);
        *counts.scripts.entry(key.clone()).or_default() += 1;
        if script.language == "FormCalc" {
            for used in uses(&script.text) {
                *counts
                    .uses
                    .entry(format!("{population} {} {used}", script.site))
                    .or_default() += 1;
            }
        }
        *counts
            .texts
            .entry(key)
            .or_default()
            .entry(script.text.trim().to_owned())
            .or_default() += 1;
    }
    if formcalc + live + dead > 0 {
        counts.calculating.insert(
            population.to_owned(),
            vec![format!(
                "{}: FormCalc {formcalc}, JavaScript {live}, JavaScript commented out or empty {dead}",
                path.display()
            )],
        );
    }
    counts
}

fn collect(path: &Path, into: &mut Vec<PathBuf>) {
    if path.is_file() {
        into.push(path.to_path_buf());
        return;
    }
    let Ok(entries) = std::fs::read_dir(path) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, into);
        } else if path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("pdf"))
        {
            into.push(path);
        }
    }
}

fn population() -> Vec<PathBuf> {
    let mut files = Vec::new();
    for argument in std::env::args().skip(1) {
        if let Some(list) = argument.strip_prefix('@') {
            match std::fs::read_to_string(list) {
                Ok(text) => files.extend(
                    text.lines()
                        .map(str::trim)
                        .filter(|l| !l.is_empty())
                        .map(PathBuf::from),
                ),
                Err(error) => eprintln!("{list}: {error}"),
            }
        } else {
            collect(Path::new(&argument), &mut files);
        }
    }
    files.sort();
    files.dedup();
    files
}

fn main() {
    let files = population();
    eprintln!("{} PDF(s) in the population", files.len());
    let total = files
        .par_iter()
        .map(|path| {
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| measure(path))).unwrap_or_else(
                |_| Counts {
                    panicked: 1,
                    ..Counts::default()
                },
            )
        })
        .reduce(Counts::default, |mut left, right| {
            left.absorb(right);
            left
        });
    println!(
        "opened {}, unopenable {}, too large {}, panicked {}",
        total.opened, total.unopenable, total.too_large, total.panicked
    );
    for (population, mut paths) in total.forms {
        paths.sort();
        println!("\n{population}: {} document(s)", paths.len());
        for path in paths {
            println!("  {path}");
        }
    }
    for (population, mut lines) in total.calculating {
        lines.sort();
        println!(
            "\n{population} with a calculate script: {} document(s)",
            lines.len()
        );
        for line in lines {
            println!("  {line}");
        }
    }
    println!("\nscripts by population, site and language");
    for (key, n) in &total.scripts {
        println!("  {n:>7}  {key}");
    }
    println!("\nhybrid FormCalc calculations run against their own forms");
    for (key, n) in &total.outcomes {
        println!("  {n:>7}  {key}");
    }
    println!("\nFormCalc uses by population and site");
    for (key, n) in &total.uses {
        println!("  {n:>7}  {key}");
    }
    for (key, texts) in &total.texts {
        let mut texts: Vec<_> = texts.iter().collect();
        texts.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
        println!("\n{key}: {} distinct", texts.len());
        for (text, n) in texts.into_iter().take(PRINTED) {
            println!("  [{n}] {}", text.replace('\n', "\n      "));
        }
    }
}
