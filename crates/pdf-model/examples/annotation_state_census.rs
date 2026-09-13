//! §12.5.6.3's `/State` and `/StateModel`, counted: how many annotations a reviewer has ruled on.
//!
//! The clause puts the state somewhere a reader would not look for it:
//!
//! > Beginning with PDF 1.5, annotations may have an author-specific state associated with them.
//! > The state is not specified in the annotation itself but in a separate text annotation that
//! > refers to the original annotation by means of its IRT ("in reply to") entry
//!
//! So the population that ranks `pdf_model::annotation_state` is not "annotations" but the
//! narrower one where the walk has something to find: a `Text` annotation stating an `/IRT` **and**
//! one of Table 175's two entries. This counts that, the state names the files actually use, and
//! the two cases where reading the entries naively gives a different answer — a chain more than one
//! reply deep, where the clause's "in reply to the previous reply" decides which state is current,
//! and a state annotation that is a group's subordinate, whose `/T` §12.5.6.2 says to read from the
//! primary.
//!
//! ```sh
//! cargo run --release -p pdf-model --example annotation_state_census -- doc/pdf.js/test/pdfs/*.pdf
//! ```

#![expect(
    clippy::print_stdout,
    reason = "an example whose entire output is a measurement"
)]

use std::collections::BTreeMap;

use pdf_syntax::{Document, Object, ObjectId};

/// What one document contributes to the census.
#[derive(Default)]
struct Counts {
    /// Annotations seen at all.
    seen: usize,
    /// `Text` annotations stating an `/IRT`: the only shape §12.5.6.3 reads a state from.
    text_replies: usize,
    /// Of those, the ones stating `/State` or `/StateModel`.
    state_changes: usize,
    /// Of those, the ones stating `/State` with no `/StateModel`, against Table 175's *required*.
    state_without_model: usize,
    /// Of those, the ones stating `/RT /Group`, whose `/T` is §12.5.6.2's primary's.
    subordinate: usize,
    /// State changes whose `/IRT` names another state change: a chain more than one reply deep.
    chained: usize,
    /// Annotations that end up carrying at least one state.
    annotations_with_a_state: usize,
    /// Of those, the ones carrying a state for more than one user.
    annotations_with_two_users: usize,
    /// How often each `/State` string is stated, whatever Table 174 has.
    by_state: BTreeMap<String, usize>,
}

impl Counts {
    /// Adds `other`'s totals to this one's.
    fn absorb(&mut self, other: &Self) {
        self.seen = self.seen.saturating_add(other.seen);
        self.text_replies = self.text_replies.saturating_add(other.text_replies);
        self.state_changes = self.state_changes.saturating_add(other.state_changes);
        self.state_without_model = self
            .state_without_model
            .saturating_add(other.state_without_model);
        self.subordinate = self.subordinate.saturating_add(other.subordinate);
        self.chained = self.chained.saturating_add(other.chained);
        self.annotations_with_a_state = self
            .annotations_with_a_state
            .saturating_add(other.annotations_with_a_state);
        self.annotations_with_two_users = self
            .annotations_with_two_users
            .saturating_add(other.annotations_with_two_users);
        for (name, count) in &other.by_state {
            let held = self.by_state.entry(name.clone()).or_default();
            *held = held.saturating_add(*count);
        }
    }
}

fn main() {
    let mut total = Counts::default();
    let mut opened = 0_usize;
    let mut lines: Vec<String> = Vec::new();

    for path in std::env::args().skip(1) {
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        let Ok(document) = Document::open(bytes) else {
            continue;
        };
        opened = opened.saturating_add(1);
        let counts = document_counts(&document);
        if counts.state_changes > 0 {
            let name = path.rsplit('/').next().unwrap_or(&path).to_owned();
            lines.push(format!(
                "  {name}: {} state change(s) over {} text repl(ies), {} chained, {} subordinate, \
                 {} annotation(s) left with a state",
                counts.state_changes,
                counts.text_replies,
                counts.chained,
                counts.subordinate,
                counts.annotations_with_a_state
            ));
        }
        total.absorb(&counts);
    }

    println!("{opened} document(s) opened, {} annotation(s)", total.seen);
    println!(
        "  {} Text annotation(s) state an /IRT; {} of those state /State or /StateModel",
        total.text_replies, total.state_changes
    );
    println!(
        "  {} state a /State with no /StateModel; {} are a group's subordinate",
        total.state_without_model, total.subordinate
    );
    println!(
        "  {} state change(s) reply to another state change, which is where depth decides",
        total.chained
    );
    println!(
        "  {} annotation(s) end up with a state, {} of them for more than one user",
        total.annotations_with_a_state, total.annotations_with_two_users
    );
    let names: Vec<String> = total
        .by_state
        .iter()
        .map(|(name, count)| format!("{name} {count}"))
        .collect();
    println!("    /State strings: {}", names.join(", "));
    for line in &lines {
        println!("{line}");
    }
}

/// Walks every annotation on every page of one document.
fn document_counts(document: &Document) -> Counts {
    let mut counts = Counts::default();
    let pages = pdf_model::Pages::new(document);
    for index in 0..pages.len() {
        let Some(page) = pages.get(index) else {
            continue;
        };
        let entry = document.get_key(&page.dict, "Annots");
        let Some(list) = entry.as_array() else {
            continue;
        };
        let ids: Vec<ObjectId> = list.iter().filter_map(Object::as_reference).collect();
        // Which of the page's annotations are themselves state changes, so that a reply to one can
        // be told from a reply to the original.
        let mut setters: Vec<ObjectId> = Vec::new();
        for item in list {
            let object = document.resolve(item);
            let Some(annotation) = object.as_dict() else {
                continue;
            };
            counts.seen = counts.seen.saturating_add(1);
            if !is_text(document, annotation) || annotation.get("IRT").is_none() {
                continue;
            }
            counts.text_replies = counts.text_replies.saturating_add(1);
            let state = string(document, annotation, "State");
            let model = string(document, annotation, "StateModel");
            if state.is_none() && model.is_none() {
                continue;
            }
            counts.state_changes = counts.state_changes.saturating_add(1);
            if state.is_some() && model.is_none() {
                counts.state_without_model = counts.state_without_model.saturating_add(1);
            }
            if document
                .get_key(annotation, "RT")
                .as_name()
                .is_some_and(|name| name.as_bytes() == b"Group")
            {
                counts.subordinate = counts.subordinate.saturating_add(1);
            }
            if let Some(name) = state {
                let held = counts.by_state.entry(name).or_default();
                *held = held.saturating_add(1);
            }
            if let Some(id) = item.as_reference() {
                setters.push(id);
            }
        }
        for setter in &setters {
            let object = document.get(*setter);
            let Some(dict) = object.as_dict() else {
                continue;
            };
            if dict
                .get("IRT")
                .and_then(Object::as_reference)
                .is_some_and(|target| setters.contains(&target))
            {
                counts.chained = counts.chained.saturating_add(1);
            }
        }
        // What the reader actually answers. Asked only of the annotations something replies to:
        // `states` builds its index per call, so asking it of every annotation on a page would be
        // quadratic, and one stress-test document in this corpus states 32 768 on one page.
        let mut targets: Vec<ObjectId> = list
            .iter()
            .filter_map(|item| document.resolve(item).as_dict().cloned())
            .filter_map(|dict| dict.get("IRT").and_then(Object::as_reference))
            .filter(|target| ids.contains(target))
            .collect();
        targets.sort();
        targets.dedup();
        for id in &targets {
            let states = pdf_model::annotation_state::states(document, &page, *id);
            if states.is_empty() {
                continue;
            }
            counts.annotations_with_a_state = counts.annotations_with_a_state.saturating_add(1);
            let mut users: Vec<Option<&String>> = states.iter().map(|s| s.user.as_ref()).collect();
            users.sort();
            users.dedup();
            if users.len() > 1 {
                counts.annotations_with_two_users =
                    counts.annotations_with_two_users.saturating_add(1);
            }
        }
    }
    counts
}

/// Whether this annotation is §12.5.6.4's `Text`, which is the only subtype a state is stated on.
fn is_text(document: &Document, annotation: &pdf_syntax::Dictionary) -> bool {
    document
        .get_key(annotation, "Subtype")
        .as_name()
        .is_some_and(|name| name.as_bytes() == b"Text")
}

/// A `text string` entry, or `None` where there is none.
fn string(document: &Document, annotation: &pdf_syntax::Dictionary, key: &str) -> Option<String> {
    let value = document.get_key(annotation, key);
    let bytes = value.as_string()?;
    Some(pdf_syntax::text_string(bytes))
}
