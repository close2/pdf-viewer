//! Which of Table 192's push-button icon and caption entries the corpus actually states.
//!
//! ISO 32000-2 §12.5.6.19 gives a widget's appearance characteristics dictionary eleven entries.
//! Four of them — `/R`, `/BC`, `/BG` and `/CA` — apply to any widget; the other seven (`/RC`,
//! `/AC`, `/I`, `/RI`, `/IX`, `/IF`, `/TP`) apply to push-buttons alone. Every one is read by
//! `appearance.rs` except `/TP`'s four codes that name a side and no proportion, which is what is
//! left of that clause's ledger row. A count comes before the work, which is what `doc/todo/01`
//! means by "the round that takes it owes the count first": an entry no file states is a clause
//! with no witness, and one with a witness is work with a page behind it.
//!
//! **§12.5.5's `/AP` sub-dictionary is counted beside them**, over every widget rather than the
//! push-buttons alone: `/N`, `/R` and `/D` are Table 170's and are the other half of the question
//! *which appearance does a pointer state select*. A widget stating a `/R` or `/D` with no `/N` is
//! the only population for which this crate would construct one of the two interactive states
//! itself, which is why that count is printed on its own.
//!
//! ```sh
//! cargo run --release -p pdf-model --example push_button_census -- doc/pdf.js/test/pdfs/*.pdf
//! ```

#![expect(
    clippy::print_stdout,
    reason = "an example whose entire output is a measurement"
)]

use std::collections::BTreeMap;

use pdf_syntax::{Dictionary, Document, Object};

/// Table 192's entries, in the order the standard prints them.
const ENTRIES: &[&str] = &[
    "R", "BC", "BG", "CA", "RC", "AC", "I", "RI", "IX", "IF", "TP",
];

/// How far §12.7.4.1's `/Parent` chain is followed, matching `appearance.rs`'s own bound.
const MAX_ANCESTRY: usize = 32;

fn main() {
    let mut documents = 0_usize;
    let mut widgets = 0_usize;
    let mut push_buttons = 0_usize;
    let mut with_mk = 0_usize;
    let mut stated: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut files: BTreeMap<&'static str, Vec<String>> = BTreeMap::new();
    let mut tp_values: BTreeMap<i64, usize> = BTreeMap::new();
    let mut with_appearance = 0_usize;
    let mut constructs: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut appearances: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut state_without_normal = 0_usize;

    for path in std::env::args().skip(1) {
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        let Ok(document) = Document::open(bytes) else {
            continue;
        };
        documents = documents.saturating_add(1);
        let name = path.rsplit('/').next().unwrap_or(&path).to_owned();
        let table = pdf_model::view::widgets_by_field_name(&document);
        for identifiers in table.values() {
            for identifier in identifiers {
                let object = document.get(*identifier);
                let Some(widget) = object.as_dict() else {
                    continue;
                };
                widgets = widgets.saturating_add(1);
                // §12.5.5's three appearances, over *every* widget rather than the push-buttons
                // alone: `/N`, `/R` and `/D` are Table 170's and apply to any annotation, while
                // the rows below are Table 192's and are the push-button's.
                for key in stored_states(&document, widget) {
                    let counter = appearances.entry(key).or_default();
                    *counter = counter.saturating_add(1);
                }
                let stored = stored_states(&document, widget);
                if (stored.contains(&"R") || stored.contains(&"D")) && !stored.contains(&"N") {
                    state_without_normal = state_without_normal.saturating_add(1);
                }
                if !is_push_button(&document, widget) {
                    continue;
                }
                push_buttons = push_buttons.saturating_add(1);
                // §12.5.6.19's Table 191 makes Table 192 the input to *constructing* an
                // appearance: the `/MK` dictionary "shall be used in constructing a dynamic
                // appearance stream". A widget that states its own `/AP` is drawn from that
                // stream by §12.5.5 and never reaches the construction, so the entries below
                // only decide a mark on a widget with no normal appearance.
                let drawn_from_its_own_stream = matches!(
                    document.get_key(widget, "AP").as_dict(),
                    Some(appearance) if !matches!(appearance.get("N"), None | Some(Object::Null))
                );
                if drawn_from_its_own_stream {
                    with_appearance = with_appearance.saturating_add(1);
                }
                let characteristics = document.get_key(widget, "MK");
                let Some(characteristics) = characteristics.as_dict() else {
                    continue;
                };
                with_mk = with_mk.saturating_add(1);
                for entry in ENTRIES {
                    if matches!(characteristics.get(entry), None | Some(Object::Null)) {
                        continue;
                    }
                    let key = *entry;
                    let counter = stated.entry(key).or_default();
                    *counter = counter.saturating_add(1);
                    let names = files.entry(key).or_default();
                    if names.last() != Some(&name) {
                        names.push(name.clone());
                    }
                    if !drawn_from_its_own_stream {
                        let counter = constructs.entry(key).or_default();
                        *counter = counter.saturating_add(1);
                    }
                    if *entry == "TP"
                        && let Object::Integer(value) = document.get_key(characteristics, "TP")
                    {
                        let counter = tp_values.entry(value).or_default();
                        *counter = counter.saturating_add(1);
                    }
                }
            }
        }
    }

    println!(
        "{documents} document(s) opened, {widgets} widget(s), {push_buttons} push-button(s), \
         {with_mk} with an /MK, {with_appearance} with an /AP /N"
    );
    println!(
        "  /AP /N {:>5}, /AP /R {:>5}, /AP /D {:>5} widget(s); {state_without_normal} \
         stating /R or /D with no /N",
        appearances.get("N").copied().unwrap_or_default(),
        appearances.get("R").copied().unwrap_or_default(),
        appearances.get("D").copied().unwrap_or_default(),
    );
    print_entries(&stated, &files, &constructs);
    if !tp_values.is_empty() {
        println!("  /TP values: {tp_values:?}");
    }
}

/// One line per Table 192 entry: how many widgets state it, how many of those construct, and where.
fn print_entries(
    stated: &BTreeMap<&'static str, usize>,
    files: &BTreeMap<&'static str, Vec<String>>,
    constructs: &BTreeMap<&'static str, usize>,
) {
    for entry in ENTRIES {
        let count = stated.get(*entry).copied().unwrap_or_default();
        let names = files.get(*entry).map(Vec::as_slice).unwrap_or_default();
        let constructed = constructs.get(*entry).copied().unwrap_or_default();
        println!(
            "  /{entry:<3} {count:>5} widget(s) ({constructed} of them constructing) in \
             {:>3} document(s): {}",
            names.len(),
            names
                .iter()
                .take(6)
                .cloned()
                .collect::<Vec<String>>()
                .join(", ")
        );
    }
}

/// Which of Table 170's three appearances this widget's `/AP` states.
///
/// `/N` is the only one §12.5.5 requires; `/R` and `/D` are the two a pointer state selects, and
/// a widget stating one of those with no `/N` is the only population for which this crate would
/// construct an interactive appearance of its own.
fn stored_states(document: &Document, widget: &Dictionary) -> Vec<&'static str> {
    let entry = document.get_key(widget, "AP");
    let Some(appearances) = entry.as_dict() else {
        return Vec::new();
    };
    ["N", "R", "D"]
        .into_iter()
        .filter(|key| !matches!(appearances.get(key), None | Some(Object::Null)))
        .collect()
}

/// Whether a widget's field is §12.7.5.2.2's push-button: `/FT /Btn` with Table 229's bit 17 set.
///
/// Both the field type and the flags are inheritable through `/Parent` (§12.7.4.1), which is why
/// this walks the chain rather than reading the widget alone.
fn is_push_button(document: &Document, widget: &Dictionary) -> bool {
    let mut node = widget.clone();
    let mut kind = None;
    let mut flags = 0_i64;
    for _ in 0..MAX_ANCESTRY {
        if kind.is_none()
            && let Object::Name(name) = document.get_key(&node, "FT")
        {
            kind = name.as_str().map(str::to_owned);
        }
        if flags == 0
            && let Object::Integer(value) = document.get_key(&node, "Ff")
        {
            flags = value;
        }
        let parent = document.get_key(&node, "Parent");
        let Some(parent) = parent.as_dict() else {
            break;
        };
        node = parent.clone();
    }
    kind.as_deref() == Some("Btn") && flags & (1_i64 << 16) != 0
}
