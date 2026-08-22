//! How many documents state §10.5's transfer function, how many state a real one, and how many
//! paint a **shading** under it.
//!
//! Table 57's `/TR` and `/TR2` were on this tree's "describes a marking device" list until the
//! three-hundred-and-fifty-seventh session, when `issue6931_reduced.pdf` turned out to decide what
//! a *screen* shows with one. `doc/todo/13` said the number a round taking the clause owes first is
//! how many of the corpus state one at all, and how many state anything but `/Identity` — because
//! that decides whether it is one page or a population.
//!
//! The third figure is the six-hundred-and-fiftieth session's, which made §10.5 reach a shading's
//! colours: it is the population that round could move. A document counts when a page states a
//! transfer function that is not `/Identity` or `/Default` **and** that page's display list holds a
//! `Paint::Shading` — an over-approximation of "a shading painted *under* one", since nothing here
//! knows which graphics state was in force at which mark, and a tight one because the first
//! condition is already rare.
//!
//! Walks every page's `/Resources /ExtGState` and every form `XObject`'s, since §8.4.5's parameters
//! are set wherever a `gs` operator can name one.
//!
//! ```sh
//! cargo run --release -p pdf-model --example transfer_function_census -- doc/pdf.js/test/pdfs/*.pdf
//! ```
//!
//! Documents are walked in parallel, one to a thread, because a `Document` is not `Sync` and this
//! is run over the `SafeDocs` crawl as well as over `doc/pdf.js`.

#![expect(
    clippy::print_stdout,
    reason = "an example whose entire output is a measurement"
)]

use std::collections::{BTreeMap, BTreeSet};

use rayon::iter::{IntoParallelRefIterator as _, ParallelIterator as _};

use pdf_syntax::{Dictionary, Document, Object};

/// How far a form `XObject`'s own resources are followed.
const MAX_DEPTH: usize = 8;

/// How many pages of one document are walked.
const MAX_PAGES: usize = 100;

/// What one document said about §10.5.
#[derive(Default)]
struct Says {
    /// The document opened.
    opened: bool,
    /// Some page states a `/TR` or `/TR2` at all.
    any: bool,
    /// Some page states one that is not `/Identity` or `/Default`.
    real: bool,
    /// The first page that states a real one *and* paints a shading, if any.
    ///
    /// The page index is carried rather than a flag because that page is the witness a round has
    /// to look at, and finding it again over sixty thousand documents is the expensive part.
    shading: Option<usize>,
    /// Every entry shape seen, `"/TR array of 4"` and the like, with its count.
    states: BTreeMap<String, usize>,
}

fn main() {
    let paths: Vec<String> = std::env::args().skip(1).collect();
    let said: Vec<(String, Says)> = paths
        .par_iter()
        .map(|path| {
            let name = path.rsplit('/').next().unwrap_or(path).to_owned();
            (name, read(path))
        })
        .collect();

    let mut documents = 0_usize;
    let mut stating = 0_usize;
    let mut stating_real = 0_usize;
    let mut with_shading = 0_usize;
    let mut states: BTreeMap<String, usize> = BTreeMap::new();
    let mut named: Vec<String> = Vec::new();
    let mut real: Vec<String> = Vec::new();
    let mut shading: Vec<String> = Vec::new();

    for (name, says) in said {
        if !says.opened {
            continue;
        }
        documents = documents.saturating_add(1);
        for (shape, count) in says.states {
            let counter = states.entry(shape).or_default();
            *counter = counter.saturating_add(count);
        }
        if says.any {
            stating = stating.saturating_add(1);
            named.push(name.clone());
        }
        if says.real {
            stating_real = stating_real.saturating_add(1);
            real.push(name.clone());
        }
        if let Some(page) = says.shading {
            with_shading = with_shading.saturating_add(1);
            shading.push(format!("{name}#{page}"));
        }
    }

    println!("{documents} document(s) opened");
    println!("  {stating} state a Table 57 /TR or /TR2 anywhere in a page's graphics states");
    println!("  **{stating_real} state one that is not /Identity or /Default**");
    println!("  **{with_shading} of those also paint a shading on the page that states it**");
    println!("  values: {states:?}");
    if named.len() <= 20 {
        println!("  stating one: {}", named.join(" "));
    }
    if real.len() <= 40 {
        println!("  stating a real one: {}", real.join(" "));
    }
    println!(
        "  stating a real one and painting a shading: {}",
        shading.join(" ")
    );
}

/// One document read: its `/ExtGState` entries, and whether a page that states a real one draws a
/// shading.
fn read(path: &str) -> Says {
    let Ok(bytes) = std::fs::read(path) else {
        return Says::default();
    };
    let Ok(document) = Document::open(bytes) else {
        return Says::default();
    };
    let mut says = Says {
        opened: true,
        ..Says::default()
    };
    let pages = pdf_model::Pages::new(&document);
    for index in 0..pages.len().min(MAX_PAGES) {
        let Some(page) = pages.get(index) else {
            continue;
        };
        let mut seen = BTreeSet::new();
        let mut any = false;
        let mut real = false;
        walk(
            &document,
            &page.resources,
            0,
            &mut seen,
            &mut says.states,
            &mut any,
            &mut real,
        );
        says.any |= any;
        says.real |= real;
        // Interpreting is the expensive half, so it is asked only of a page that states a real
        // transfer function — which is what makes this affordable over sixty thousand documents.
        if real && says.shading.is_none() {
            let interpretation = pdf_model::interpret(&document, &page);
            let paints = interpretation
                .display_list
                .commands()
                .iter()
                .any(|command| {
                    matches!(
                        command,
                        pdf_render::Command::Fill {
                            paint: pdf_render::Paint::Shading(_),
                            ..
                        } | pdf_render::Command::Stroke {
                            paint: pdf_render::Paint::Shading(_),
                            ..
                        }
                    )
                });
            if paints {
                says.shading = Some(index);
            }
        }
    }
    says
}

/// Every `/ExtGState` reachable from one resource dictionary, including through form `XObject`s.
fn walk(
    document: &Document,
    resources: &Dictionary,
    depth: usize,
    seen: &mut BTreeSet<pdf_syntax::ObjectId>,
    states: &mut BTreeMap<String, usize>,
    any: &mut bool,
    any_real: &mut bool,
) {
    if depth > MAX_DEPTH {
        return;
    }
    let graphics = document.get_key(resources, "ExtGState");
    if let Some(dict) = graphics.as_dict() {
        for (_, value) in dict.iter() {
            let resolved = document.resolve(value);
            let Some(state) = resolved.as_dict() else {
                continue;
            };
            for key in ["TR", "TR2"] {
                let entry = document.get_key(state, key);
                if entry.is_null() {
                    continue;
                }
                *any = true;
                let shape = match &entry {
                    Object::Name(name) => String::from_utf8_lossy(name.as_bytes()).into_owned(),
                    Object::Array(items) => format!("array of {}", items.len()),
                    Object::Dictionary(_) | Object::Stream(_) => "one function".to_owned(),
                    other => format!("{other:?}"),
                };
                if !matches!(shape.as_str(), "Identity" | "Default") {
                    *any_real = true;
                }
                let counter = states.entry(format!("/{key} {shape}")).or_default();
                *counter = counter.saturating_add(1);
            }
        }
    }
    let objects = document.get_key(resources, "XObject");
    let Some(dict) = objects.as_dict() else {
        return;
    };
    for (_, value) in dict.iter() {
        if let Some(id) = value.as_reference()
            && !seen.insert(id)
        {
            continue;
        }
        let resolved = document.resolve(value);
        let Object::Stream(stream) = &resolved else {
            continue;
        };
        let inner = document.get_key(&stream.dict, "Resources");
        if let Some(inner) = inner.as_dict() {
            walk(
                document,
                inner,
                depth.saturating_add(1),
                seen,
                states,
                any,
                any_real,
            );
        }
    }
}
