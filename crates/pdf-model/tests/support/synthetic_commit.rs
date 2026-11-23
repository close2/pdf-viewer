//! The synthetic commit both `script_corpus` walks make: a field's own value entered again, the way
//! a reader of that field could enter it.
//!
//! One definition for the two walks — Tier 0's in `pdf-model`'s `tests/script_corpus.rs` and Tier
//! 1's in `pdf-script`'s, reached through a `#[path]` — so that the engine and the library are
//! handed the same commits.
//!
//! A text field and an editable combo box take characters, so their value is entered as the text
//! they show. A choice field without Table 233's `Edit` flag takes a selection and nothing else —
//! bit 19 clear "shall include only a drop-down list", and a list box is a list — so it is entered
//! as [`Entered::Chosen`] of the options §12.7.5.4 says its value names, which is what a reader
//! choosing in its list sends, and what runs its `/K` and `/V` (ADR 3470).

use std::collections::BTreeMap;
use std::time::Duration;

use pdf_model::Pages;
use pdf_model::form::{self, ChoiceControl, Control};
use pdf_model::view::{Entered, ViewState};
use pdf_syntax::Document;

/// What both walks hold a sequence of scripts to, in place of the one second a person waiting on
/// a commit is owed.
///
/// One second is a measure of the machine: a document whose `/CO` walk costs half of it on an idle
/// machine is stopped at an entry the load chose on a busy one, and the walk's figures moved with
/// the load alone (ADR 3454). A minute is sixty times the default, so only a load that slows a
/// walk sixtyfold stops one the default lets finish; every script is still held to its own budget,
/// and every stop is still counted and named (ADR 3469).
pub(crate) const WALK_SEQUENCE_BUDGET: Duration = Duration::from_secs(60);

/// Every choice field that takes only a selection, by its fully qualified name, with the index of
/// the first page a widget of it is on.
pub(crate) fn selection_fields(document: &Document, view: &ViewState) -> BTreeMap<String, usize> {
    let pages = Pages::new(document);
    let mut found = BTreeMap::new();
    for index in 0..pages.len() {
        let Some(page) = pages.get(index) else {
            continue;
        };
        for field in form::fields(document, &page, view) {
            if let Control::Choice(choice) = &field.control
                && !takes_text(choice)
            {
                found.entry(field.name.qualified).or_insert(index);
            }
        }
    }
    found
}

/// Whether a choice field takes characters: Table 233 bit 19, which "shall be used only if the
/// Combo flag is set".
fn takes_text(choice: &ChoiceControl) -> bool {
    choice.combo && choice.editable
}

/// What a reader enters to commit `name`'s value again, or `None` where no reader of the field
/// could enter it.
///
/// `shown` is the text [`ViewState::field_value`] answers with; `selection` is
/// [`selection_fields`]'s answer for the document. A selection-only field whose value names no
/// option — §12.7.5.4: "V is a text string representing the selected item, as given in the field
/// dictionary's Opt array" — holds a value no choice in its list makes, so there is nothing to
/// enter, and the walk counts its value untaken. An empty value is the clause's "no item is
/// currently selected", which an empty selection enters.
pub(crate) fn re_entered(
    document: &Document,
    view: &ViewState,
    name: &str,
    shown: String,
    selection: &BTreeMap<String, usize>,
) -> Option<Entered> {
    let Some(page) = selection.get(name) else {
        return Some(Entered::Text(shown));
    };
    let page = Pages::new(document).get(*page)?;
    let choice = form::fields(document, &page, view)
        .into_iter()
        .find(|field| field.name.qualified == name)
        .and_then(|field| match field.control {
            Control::Choice(choice) => Some(choice),
            _ => None,
        })?;
    if choice.selected.is_empty() && !shown.is_empty() {
        return None;
    }
    Some(Entered::Chosen(choice.selected))
}
