//! Two negatives whose witness is a *shape in a content stream* rather than a name anywhere.
//!
//! `doc/todo/01`'s sixteenth sweep splits the rows carrying a "no corpus document …" sentence by
//! what would settle them, and the hardest group is the one whose witness no dictionary states:
//! a sequence of operators. `examples/witness_census`'s third column reaches anything that is a
//! **token** — a marked-content tag, a `CMap` operator — and reaches none of these, because what
//! makes them witnesses is the *order* the tokens arrive in. This is that instrument, for the two
//! rows whose shape a path and a text object can be walked for:
//!
//! - **ISO 32000-2 §8.5.2.1.** "Most operators that add a segment to the current path start at
//!   the current point; if the current point is undefined, an error shall be generated." That row
//!   has said since the twenty-fourth session that such a path is passed to the rasteriser
//!   unclassified rather than refused, and that no corpus first page reaches it.
//! - **§9.4.2, as Errata Collection 3's Issue #368 adds to it.** "Within a text object, the
//!   graphics state stack operators q and Q (see 8.4.2, 'Graphics state stack') shall
//!   additionally push and pop Tm and Tlm as part of the graphics state stack." A `Q` inside a
//!   text object therefore restores the two matrices — which changes where the next glyph lands
//!   only if something moved them between the `q` and the `Q`. That row's count was taken over
//!   974 documents before `CC-MAIN-2021-31` was on this disk.
//!
//! **The second question has a sharp half and a broad half and this counts both**, because they
//! are different claims and the row states only one of them. `Td`, `TD`, `Tm` and `T*` move Tm
//! and Tlm outright, and so do `'` and `"`, which perform a `T*` before showing; that is the
//! sharp half, and it is the one the row's sentence is about. But a text-showing operator
//! *advances* Tm by what it drew, so a `q … Tj … Q` inside a text object is moved by the pop
//! exactly as a `q … Td … Q` is. Counting only the sharp half would report a rule no file
//! exercises while files exercise it.
//!
//! Three controls, printed beside the two questions rather than merged into them:
//!
//! - **§8.5.3.1's neighbouring sentence** — "[a]ttempting to execute a painting operator when
//!   the current path is undefined … shall generate an error", which Issue #549 turns into "shall
//!   be ignored". It is the same state machine read at the other end, so a run reporting a
//!   painting operator on an empty path is a run whose current-point tracking is finding real
//!   things rather than nothing.
//! - **a `q` or a `Q` inside a text object at all**, which is the figure §9.4.2's row already
//!   carries and which this census has to reproduce before its new columns mean anything.
//! - **a `q` inside a text object whose `Q` is outside it**, which §9.4.1 says is invalid — a
//!   population apart from the one the errata sentence governs.
//!
//! # What this census reaches, and what it does not
//!
//! The first page's `/Contents`, and every form `XObject` its resources reach, recursively,
//! because §8.10.1 makes a form's content a content stream of its own with its own path and text
//! objects. What it does **not** walk is stated as a count rather than left implicit: tiling
//! pattern cells, Type 3 glyph procedures and annotation appearance streams. A zero over a
//! population an instrument cannot see is `doc/todo/01`'s own standing warning, so the size of
//! what was not seen is printed under every run.
//!
//! **Inline images are skipped through `pdf_model::inline_image::scan` rather than lexed.** A
//! `BI … ID … EI` sequence's data is bytes, and bytes lex into keywords: an image whose samples
//! happen to spell `l` after a page's last `f` would be a witness this census invented. That is
//! trap 11 at the level of the lexer, and the interpreter's own reader is what avoids it.
//!
//! ```sh
//! cargo run --release -p pdf-model --example operator_shape_census              # curated
//! cargo run --release -p pdf-model --example operator_shape_census -- --pdfjs
//! cargo run --release -p pdf-model --example operator_shape_census -- --crawl   # CC-MAIN-2021-31
//! cargo run --release -p pdf-model --example operator_shape_census -- <file.pdf>...
//! ```
//!
//! The three scopes are `examples/long_mitre_census`'s, for ADR 0490's reason: a negative decays
//! when the population grows, and the control run is stated beside the crawl run rather than
//! merged with it.
#![expect(
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::arithmetic_side_effects,
    reason = "a measurement example: its output is the point, and every count below is bounded \
              by the number of tokens in content streams already held in memory"
)]

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use pdf_syntax::{Dictionary, Document, Lexer, Object, ObjectId, Token};
use rayon::prelude::*;

/// How many witnessing documents are named per finding before the list is truncated.
const MAX_NAMED: usize = 12;

/// The most pages `--pages=` will walk, matching `examples/variable_text_census`'s own bound.
const MAX_PAGES: usize = 100;

/// How deep a chain of form `XObject`s is followed.
///
/// A form may draw a form, and a malformed file may make that a cycle; the visited set answers
/// the cycle and this answers the chain, at the same depth `content::xobject` uses.
const MAX_FORM_DEPTH: usize = 16;

/// Which population a run is over.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Scope {
    /// The pdf.js corpus alone.
    PdfJs,
    /// That, the four `doc/corpora/` submodules, and this project's own fixtures — the
    /// population §8.5.2.1's and §9.4.2's sentences were measured over.
    Curated,
    /// The `SafeDocs` `CC-MAIN-2021-31` crawl under `corpus-cache/`, and nothing else.
    Crawl,
    /// Whatever files the command line named.
    Named,
}

/// Every PDF this census measures over, in the scope asked for.
fn corpus(scope: Scope, named: &[String]) -> Vec<PathBuf> {
    if scope == Scope::Named {
        return named.iter().map(PathBuf::from).collect();
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut files = Vec::new();
    let roots: &[&str] = match scope {
        Scope::PdfJs => &["doc/pdf.js/test/pdfs"],
        Scope::Curated => &["doc/pdf.js/test/pdfs", "doc/corpora", "doc/corpora-own"],
        Scope::Crawl => &["corpus-cache/safedocs/cc-main-2021-31"],
        Scope::Named => &[],
    };
    for relative in roots {
        collect(&root.join(relative), &mut files);
    }
    files.sort();
    files.dedup();
    files
}

/// Every `.pdf` under one directory, recursively.
///
/// `is_dir` and `read_dir` both follow symbolic links, which is not incidental: a parallel
/// worktree reaches the corpora through symlinks, and a walk that did not follow them would
/// report the emptiest possible false zero.
fn collect(dir: &Path, into: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
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

/// What one document's first page said, over every content stream this census reaches.
#[derive(Default)]
struct Counts {
    /// Pages walked.
    pages: usize,
    /// Content streams walked — the page's `/Contents` counts as one, plus each form.
    streams: usize,
    /// §8.5.2.1: `l`, `c`, `v` or `y` with the current point undefined.
    segments_with_no_point: usize,
    /// §8.5.2.1 as it reaches `h`, counted apart because the two cost different things. A
    /// segment with no current point reaches the rasteriser and is drawn from wherever its
    /// builder starts; a close on an empty path is declined by `content::path::close_subpath`,
    /// which pushes nothing where there is no command to close. Table 58 also gives `h` a
    /// sentence of its own — "[i]f the current subpath is already closed, h shall do nothing" —
    /// and a close with no subpath at all is the general sentence rather than that one.
    closes_with_no_point: usize,
    /// §8.5.3.1's control: a painting operator with the current path undefined.
    paints_with_no_path: usize,
    /// §9.4.2's own figure: a `q` or a `Q` between a `BT` and its `ET`.
    stack_ops_in_a_text_object: usize,
    /// A `q … Q` pair inside one text object with `Td`, `TD`, `Tm`, `T*`, `'` or `"` between.
    saves_spanning_a_move: usize,
    /// That, or a text-showing operator between — the broad half.
    saves_spanning_a_show: usize,
    /// Of the broad half, the pairs whose restore a *mark* can see: text is shown after the `Q`
    /// before anything sets Tm outright again and before the text object ends.
    ///
    /// **This is the column that says whether a page draws differently**, and it is a third
    /// claim rather than a sharpening of the second. A `Q` restores Tm, and the very next `Tm`
    /// replaces it — Table 106 says the operands "shall not be concatenated onto the current
    /// text matrix, but shall replace it" — so a file that closes every save and immediately
    /// positions afresh has the construction and none of its consequence.
    restores_a_mark_can_see: usize,
    /// Of those, the ones whose pair is in the *sharp* half.
    sharp_restores_a_mark_can_see: usize,
    /// §9.4.1's invalid nesting: a `q` inside a text object whose `Q` is not.
    saves_crossing_the_text_object: usize,
    /// Form `XObject` streams reached from the page's resources.
    forms: usize,
    /// Content streams this census does **not** walk, so that the bound has a size: tiling
    /// pattern cells, Type 3 glyph procedures and annotation appearance streams.
    unwalked: usize,
}

/// Whether a keyword moves Tm and Tlm, which is what Issue #368's pop restores.
///
/// `'` and `"` are here as well as among the showing operators because Table 107 states each of
/// them as a `T*` followed by a `Tj`, and a `T*` is a move.
fn moves_the_text_matrices(word: &[u8]) -> bool {
    matches!(word, b"Td" | b"TD" | b"Tm" | b"T*" | b"'" | b"\"")
}

/// Whether a keyword shows text, and so advances Tm by what it drew.
fn shows_text(word: &[u8]) -> bool {
    matches!(word, b"Tj" | b"TJ" | b"'" | b"\"")
}

/// Whether a keyword is one of Table 59's ten path-painting operators.
fn paints_a_path(word: &[u8]) -> bool {
    matches!(
        word,
        b"S" | b"s" | b"f" | b"F" | b"f*" | b"B" | b"B*" | b"b" | b"b*" | b"n"
    )
}

/// One `q` still open, and what has happened since it.
struct Save {
    /// Whether the `q` was itself inside a text object.
    in_a_text_object: bool,
    /// Whether Tm has been moved outright since.
    moved: bool,
    /// Whether text has been shown since.
    shown: bool,
}

/// A `Q` that restored Tm, waiting to find out whether anything is drawn from it.
struct Restore {
    /// Whether the pair it came from moved Tm outright rather than only advancing it.
    sharp: bool,
}

fn main() {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let named: Vec<String> = arguments
        .iter()
        .filter(|a| !a.starts_with("--"))
        .cloned()
        .collect();
    let scope = if named.is_empty() {
        if arguments.iter().any(|a| a == "--crawl") {
            Scope::Crawl
        } else if arguments.iter().any(|a| a == "--pdfjs") {
            Scope::PdfJs
        } else {
            Scope::Curated
        }
    } else {
        Scope::Named
    };

    // §8.5.2.1's sentence is about a first page and §9.4.2's about any page, so how many pages
    // are walked is the caller's rather than this program's, and every run prints what it took.
    let pages = arguments
        .iter()
        .find_map(|argument| argument.strip_prefix("--pages="))
        .map_or(1, |value| {
            value.parse::<usize>().unwrap_or(1).clamp(1, MAX_PAGES)
        });

    let files = corpus(scope, &named);
    eprintln!(
        "{} PDF(s) in the population, {pages} page(s) apiece",
        files.len()
    );

    let measured: Vec<(String, Option<Counts>)> = files
        .par_iter()
        .map(|path| {
            let label = path.to_string_lossy().into_owned();
            // A hostile file may panic under the object graph; over sixty-five thousand of them
            // an instrument that dies on one measures nothing. Counted, and printed below.
            let counts =
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| measure(path, pages)))
                    .unwrap_or(None);
            (label, counts)
        })
        .collect();

    report(&measured);
}

/// Which documents witnessed what, beside the totals.
#[derive(Default)]
struct Witnesses {
    /// A line apiece for the documents with a segment operator and no current point.
    segments: Vec<String>,
    /// Documents whose only witness is an `h`, which costs no mark.
    closes: Vec<String>,
    /// Documents with a painting operator on an undefined path.
    paints: Vec<String>,
    /// Documents with a sharp `q` … `Q` pair.
    sharp: Vec<String>,
    /// Documents with a broad one.
    broad: Vec<String>,
    /// Documents with a `q` or a `Q` inside a text object at all.
    stack: Vec<String>,
    /// Documents whose restore reaches a mark.
    visible: Vec<String>,
    /// Documents nesting a `q` across a `BT` or an `ET`.
    crossing: Vec<String>,
    /// How many documents this census opened.
    opened: usize,
    /// How many pages it walked.
    pages: usize,
}

impl Witnesses {
    /// Folds one document in.
    fn take(&mut self, file: &str, counts: &Counts) {
        self.opened += 1;
        self.pages += counts.pages;
        if counts.segments_with_no_point > 0 {
            self.segments.push(format!(
                "{file}: {} segment(s) and {} close(s) with no current point",
                counts.segments_with_no_point, counts.closes_with_no_point
            ));
        } else if counts.closes_with_no_point > 0 {
            self.closes.push(file.to_owned());
        }
        for (count, list) in [
            (counts.paints_with_no_path, &mut self.paints),
            (counts.saves_spanning_a_move, &mut self.sharp),
            (counts.saves_spanning_a_show, &mut self.broad),
            (counts.stack_ops_in_a_text_object, &mut self.stack),
            (counts.restores_a_mark_can_see, &mut self.visible),
            (counts.saves_crossing_the_text_object, &mut self.crossing),
        ] {
            if count > 0 {
                list.push(file.to_owned());
            }
        }
    }
}

impl Counts {
    /// Adds one document's counts into a running total.
    fn add(&mut self, other: &Self) {
        self.pages += other.pages;
        self.streams += other.streams;
        self.segments_with_no_point += other.segments_with_no_point;
        self.closes_with_no_point += other.closes_with_no_point;
        self.paints_with_no_path += other.paints_with_no_path;
        self.stack_ops_in_a_text_object += other.stack_ops_in_a_text_object;
        self.saves_spanning_a_move += other.saves_spanning_a_move;
        self.saves_spanning_a_show += other.saves_spanning_a_show;
        self.restores_a_mark_can_see += other.restores_a_mark_can_see;
        self.sharp_restores_a_mark_can_see += other.sharp_restores_a_mark_can_see;
        self.saves_crossing_the_text_object += other.saves_crossing_the_text_object;
        self.forms += other.forms;
        self.unwalked += other.unwalked;
    }
}

/// Prints the witnesses, then the totals.
fn report(measured: &[(String, Option<Counts>)]) {
    let mut totals = Counts::default();
    let mut witnesses = Witnesses::default();
    for (file, counts) in measured {
        if let Some(counts) = counts {
            totals.add(counts);
            witnesses.take(file, counts);
        }
    }
    report_paths(&totals, &witnesses);
    report_text(&totals, &witnesses);
    println!(
        "{} of {} file(s) opened, {} page(s) walked: {} content stream(s), of which {} are form \
         XObjects; {} stream(s) this census does not reach (pattern cells, Type 3 procedures, \
         annotation appearances)",
        witnesses.opened,
        measured.len(),
        witnesses.pages,
        totals.streams,
        totals.forms,
        totals.unwalked
    );
    println!(
        "  {} file(s) this census could not reach at all",
        measured.len() - witnesses.opened
    );
}

/// §8.5.2.1's answer, with §8.5.3.1's control under it.
fn report_paths(totals: &Counts, witnesses: &Witnesses) {
    println!("§8.5.2.1 — a segment operator with no current point");
    for line in witnesses.segments.iter().take(MAX_NAMED) {
        println!("  {line}");
    }
    if witnesses.segments.len() > MAX_NAMED {
        println!("  … and {} more", witnesses.segments.len() - MAX_NAMED);
    }
    println!(
        "  {} document(s) with a segment, {} with an h and no segment; {} l/c/v/y and {} h with \
         the current point undefined",
        witnesses.segments.len(),
        witnesses.closes.len(),
        totals.segments_with_no_point,
        totals.closes_with_no_point
    );
    println!("  the h-only ones: {}", names(&witnesses.closes));
    println!(
        "  control, §8.5.3.1: {} painting operator(s) on an undefined path, over {} document(s): \
         {}",
        totals.paints_with_no_path,
        witnesses.paints.len(),
        names(&witnesses.paints)
    );
}

/// §9.4.2's answer, in the three widths the row's sentence does not distinguish.
fn report_text(totals: &Counts, witnesses: &Witnesses) {
    println!("§9.4.2 — Issue #368's q/Q inside a text object");
    println!(
        "  control: {} q or Q between a BT and its ET, over {} document(s)",
        totals.stack_ops_in_a_text_object,
        witnesses.stack.len()
    );
    println!(
        "  sharp: {} q…Q pair(s) with Td/TD/Tm/T*/'/\" between, over {} document(s): {}",
        totals.saves_spanning_a_move,
        witnesses.sharp.len(),
        names(&witnesses.sharp)
    );
    println!(
        "  broad: {} pair(s) with any of those or a showing operator between, over {} \
         document(s): {}",
        totals.saves_spanning_a_show,
        witnesses.broad.len(),
        names(&witnesses.broad)
    );
    println!(
        "  visible: {} of the broad pairs ({} of the sharp ones) show text after the Q before \
         anything sets Tm outright again, over {} document(s): {}",
        totals.restores_a_mark_can_see,
        totals.sharp_restores_a_mark_can_see,
        witnesses.visible.len(),
        names(&witnesses.visible)
    );
    println!(
        "  §9.4.1's invalid nesting: {} q inside a text object whose Q is outside it, over {} \
         document(s): {}",
        totals.saves_crossing_the_text_object,
        witnesses.crossing.len(),
        names(&witnesses.crossing)
    );
}

/// A truncated list of document names.
fn names(pages: &[String]) -> String {
    if pages.len() > MAX_NAMED {
        format!(
            "{}, … and {} more",
            pages[..MAX_NAMED].join(", "),
            pages.len() - MAX_NAMED
        )
    } else if pages.is_empty() {
        "none".to_owned()
    } else {
        pages.join(", ")
    }
}

/// Walks the first `wanted` pages of one document, and every form their resources reach.
fn measure(path: &Path, wanted: usize) -> Option<Counts> {
    let bytes = std::fs::read(path).ok()?;
    let document = Document::open(bytes).ok()?;
    let pages = pdf_model::Pages::new(&document);

    let mut counts = Counts::default();
    // One visited set for the whole document: a form drawn by two pages is one content stream
    // and counting it twice would inflate every column below it.
    let mut visited = BTreeSet::new();
    for index in 0..wanted.min(pages.len()) {
        let Some(page) = pages.get(index) else {
            continue;
        };
        counts.pages += 1;
        counts.unwalked += unwalked(&document, &page.dict, &page.resources);
        let content = page.content(&document);
        walk(&document, &content, &page.resources, &mut counts);
        counts.streams += 1;
        forms(&document, &page.resources, 0, &mut visited, &mut counts);
    }
    (counts.pages > 0).then_some(counts)
}

/// How many content streams this page states that this census does not walk.
///
/// Named rather than followed: a tiling cell, a Type 3 glyph procedure and an annotation's
/// appearance are each a content stream with a path and text objects of its own, and a zero
/// measured without them is a zero over a smaller population than the sentence claims.
fn unwalked(document: &Document, page: &Dictionary, resources: &Dictionary) -> usize {
    let mut count = 0;
    if let Object::Dictionary(patterns) = entry(document, resources, "Pattern") {
        count += patterns.len();
    }
    if let Object::Dictionary(fonts) = entry(document, resources, "Font") {
        for (_, value) in fonts.iter() {
            if let Object::Dictionary(font) = document.resolve(value)
                && is_name(font.get("Subtype"), b"Type3")
            {
                count += 1;
            }
        }
    }
    if let Object::Array(annotations) = entry(document, page, "Annots") {
        for annotation in &annotations {
            if let Object::Dictionary(dict) = document.resolve(annotation)
                && dict.get("AP").is_some()
            {
                count += 1;
            }
        }
    }
    count
}

/// One dictionary entry, resolved — the null of §7.3.9 where the dictionary states none.
fn entry(document: &Document, dict: &Dictionary, key: &str) -> Object {
    dict.get(key)
        .map_or(Object::Null, |object| document.resolve(object))
}

/// Whether an entry states one particular name.
fn is_name(object: Option<&Object>, wanted: &[u8]) -> bool {
    object
        .and_then(Object::as_name)
        .is_some_and(|name| name.0.as_ref() == wanted)
}

/// Walks every form `XObject` one resource dictionary names, recursively.
fn forms(
    document: &Document,
    resources: &Dictionary,
    depth: usize,
    visited: &mut BTreeSet<ObjectId>,
    counts: &mut Counts,
) {
    if depth >= MAX_FORM_DEPTH {
        return;
    }
    let Object::Dictionary(xobjects) = entry(document, resources, "XObject") else {
        return;
    };
    for (_, value) in xobjects.iter() {
        if let Object::Reference(id) = value
            && !visited.insert(*id)
        {
            continue;
        }
        let resolved = document.resolve(value);
        let Some(stream) = resolved.as_stream() else {
            continue;
        };
        if !is_name(stream.dict.get("Subtype"), b"Form") {
            continue;
        }
        let Some(data) = document.decoded_stream_data(stream) else {
            continue;
        };
        // §8.10.1: a form's own resources where it states them, the parent's where it does not
        // — the same fallback `content::xobject` makes.
        let own = match entry(document, &stream.dict, "Resources") {
            Object::Dictionary(dict) => dict,
            _ => resources.clone(),
        };
        counts.unwalked += unwalked(document, &Dictionary::new(), &own);
        walk(document, &data, &own, counts);
        counts.streams += 1;
        counts.forms += 1;
        forms(document, &own, depth + 1, visited, counts);
    }
}

/// Counts both questions' shapes over one content stream's tokens.
fn walk(document: &Document, content: &[u8], resources: &Dictionary, counts: &mut Counts) {
    let mut lexer = Lexer::new(content);
    let mut machines = Machines::default();

    while let Some(token) = lexer.next_token() {
        let Token::Keyword(word) = token else {
            continue;
        };
        if word == b"BI" {
            // Image data is bytes rather than a program: hand it to the same reader the
            // interpreter uses and resume past the `EI` it found.
            let scan =
                pdf_model::inline_image::scan(document, content, lexer.position(), resources, true);
            lexer.seek(scan.resume);
            continue;
        }
        machines.path(word, counts);
        machines.text(word, counts);
    }
}

/// The two state machines, run side by side over one content stream.
///
/// They are kept apart because they answer different rows and share nothing but the token
/// stream: no path operator touches a text object and no text operator touches the path.
#[derive(Default)]
struct Machines {
    /// §8.5.2.1: "If the current path is empty, the current point shall be undefined."
    current_point: bool,
    /// How many `BT`s are open. A second one before an `ET` is malformed and counted as nesting
    /// rather than guessed at.
    text_depth: usize,
    /// Every `q` still open, innermost last.
    saves: Vec<Save>,
    /// The most recent `Q` that restored Tm, if nothing has read it yet.
    restored: Option<Restore>,
}

impl Machines {
    /// Table 58's construction operators and Table 59's painting ones.
    fn path(&mut self, word: &[u8], counts: &mut Counts) {
        match word {
            b"m" | b"re" => self.current_point = true,
            b"l" | b"c" | b"v" | b"y" => {
                if !self.current_point {
                    counts.segments_with_no_point += 1;
                }
                self.current_point = true;
            }
            b"h" => {
                if !self.current_point {
                    counts.closes_with_no_point += 1;
                }
            }
            painting if paints_a_path(painting) => {
                if !self.current_point {
                    counts.paints_with_no_path += 1;
                }
                self.current_point = false;
            }
            _ => {}
        }
    }

    /// Table 105's text object, Table 106's positioning and §8.4.2's stack inside them.
    fn text(&mut self, word: &[u8], counts: &mut Counts) {
        match word {
            b"BT" => {
                self.text_depth += 1;
                for save in &mut self.saves {
                    save.in_a_text_object = false;
                }
            }
            b"ET" => self.end_text_object(counts),
            b"q" => {
                if self.text_depth > 0 {
                    counts.stack_ops_in_a_text_object += 1;
                }
                self.saves.push(Save {
                    in_a_text_object: self.text_depth > 0,
                    moved: false,
                    shown: false,
                });
            }
            b"Q" => self.restore(counts),
            other if self.text_depth > 0 => self.inside_a_text_object(other, counts),
            _ => {}
        }
    }

    /// An `ET`, which ends both the text object and any restore nothing has yet read.
    fn end_text_object(&mut self, counts: &mut Counts) {
        self.text_depth = self.text_depth.saturating_sub(1);
        // Table 105: an `ET` discards the text matrix, so a restore nothing has drawn from by
        // now is a restore nothing can see.
        self.restored = None;
        for save in &mut self.saves {
            if save.in_a_text_object {
                save.in_a_text_object = false;
                counts.saves_crossing_the_text_object += 1;
            }
        }
    }

    /// A `Q`, and whether the pair it closes is one Issue #368's rule reaches.
    fn restore(&mut self, counts: &mut Counts) {
        if self.text_depth > 0 {
            counts.stack_ops_in_a_text_object += 1;
        }
        if let Some(save) = self.saves.pop()
            && save.in_a_text_object
            && self.text_depth > 0
        {
            if save.moved {
                counts.saves_spanning_a_move += 1;
            }
            if save.moved || save.shown {
                counts.saves_spanning_a_show += 1;
                self.restored = Some(Restore { sharp: save.moved });
            }
        }
    }

    /// Every other operator, met between a `BT` and its `ET`.
    fn inside_a_text_object(&mut self, word: &[u8], counts: &mut Counts) {
        let moved = moves_the_text_matrices(word);
        let shown = shows_text(word);
        if moved || shown {
            for save in &mut self.saves {
                if save.in_a_text_object {
                    save.moved |= moved;
                    save.shown |= shown;
                }
            }
        }
        if shown {
            // A glyph drawn from the restored matrix is what makes the pop visible. `Td`, `TD`
            // and `T*` are deliberately not here: each is relative to Tlm, so a restore still
            // reaches whatever is shown after them.
            if let Some(restore) = self.restored.take() {
                counts.restores_a_mark_can_see += 1;
                if restore.sharp {
                    counts.sharp_restores_a_mark_can_see += 1;
                }
            }
        } else if word == b"Tm" {
            // Table 106: the operands "shall not be concatenated onto the current text matrix,
            // but shall replace it" — so the restored value is gone.
            self.restored = None;
        }
    }
}
