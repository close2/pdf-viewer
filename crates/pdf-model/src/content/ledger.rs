//! The named-resource lookups one interpretation made, kept only where a caller asked for them.
//!
//! # What this is, and what it is not
//!
//! ISO 32000-2 §7.8.3 makes a content stream's named resources a question with one answer per
//! operator: the operand names an entry in one subdictionary of the resource dictionary in force,
//! and that entry — or its absence — is what the operator selects. This module records those
//! answers as the interpreter gives them, one [`Selection`] per distinct (stream, category, name,
//! outcome), so that another reader of the same document can be held to the same answers.
//!
//! **It is a ledger of lookups and not an observer of the interpreter's state.** `doc/questions/A61`
//! declined an observer — a surface exposing the resolved graphics state and resources dictionary
//! at every operator, which every future operator would have to feed — and asked for a test that
//! holds `pdf-archive`'s survey and this interpreter to each other over the corpus instead. That
//! test needs one fact from this side, and it is a fact the interpreter already produces at one
//! place: the entry a lookup found. So the recording sits in `resources.rs`'s three lookup
//! functions and nowhere an operator has to remember, and a caller that does not ask pays one
//! `Option` test per lookup. ADR 1055 is the decision, and it names the constructs the comparison
//! is scoped to.
//!
//! # Where a selection is made
//!
//! Every selection carries the run of [`Frame`]s it was made under — the page's content, the form
//! `XObject` invoked from it, the glyph description that form's text ran — so that the same form
//! reached along two routes with two different resource dictionaries in force is two different
//! places rather than one. A frame names the [`Route`] by which its stream was entered because the
//! comparison A61 asked for is scoped by route: a soft mask's group is one the survey deliberately
//! does not walk, and a selection made under it is not one the survey failed to judge.

use std::collections::BTreeMap;

use pdf_syntax::{Object, ObjectId};

/// How many distinct selections one ledger keeps before it stops and says so.
///
/// A ledger is asked for by a test over a corpus, and every entry it keeps is one a content
/// stream produced by naming a distinct resource — which a hostile stream can do once per
/// operator. `MAX_OPERATIONS` bounds those at four million per interpretation; this is a quarter
/// of that, and reaching it is reported by [`Ledger::truncated`] rather than silently dropped.
pub const MAX_SELECTIONS: usize = 1 << 20;

/// By which of §7.8.2's routes a content stream was entered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Route {
    /// The page's own `/Contents` (§7.7.3.3).
    Page,
    /// A form `XObject` invoked by `Do` (§8.10).
    Form,
    /// A tiling pattern's cell, painted where the pattern was selected (§8.7.3).
    TilingPattern,
    /// A Type 3 font's glyph description, run by a text-showing operator (§9.6.4).
    Type3Glyph,
    /// An annotation's appearance stream, drawn by §12.5.3's pass (§12.5.5).
    Appearance,
    /// The transparency group of a soft mask, evaluated at the `gs` that set it (§11.6.5.1).
    SoftMask,
}

/// One content stream a selection was made under: how it was entered, and which object it is.
///
/// `stream` is `None` for the page's own content, which is a sequence of streams rather than one
/// object, and for a stream this program composed (§12.7.4.3's constructed appearance).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Frame {
    /// By which route the stream was entered.
    pub route: Route,
    /// The stream's own object, where the file names it by reference.
    pub stream: Option<ObjectId>,
}

/// What a lookup found.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Outcome {
    /// The entry is an indirect reference, and this is the object it names.
    Reference(ObjectId),
    /// The entry is a direct object, printed as `Debug` prints it.
    ///
    /// A direct entry has no identity but its value, and two readers that found the same value
    /// selected the same thing. The value is the entry as the resource dictionary states it — an
    /// array naming a colour space family, an inline dictionary — and never a stream, which
    /// §7.3.8.1 requires to be indirect.
    Direct(String),
    /// The subdictionary defines no such name (§7.8.3).
    Missing,
}

/// One distinct named-resource selection.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Selection {
    /// The streams the selection was made under, outermost first.
    pub frames: Vec<Frame>,
    /// Which of Table 34's subdictionaries was asked: `Font`, `XObject`, `ExtGState`, and so on.
    pub category: &'static str,
    /// The name the operator gave, as §7.3.5's bytes.
    pub name: Vec<u8>,
    /// What the lookup found.
    pub outcome: Outcome,
}

/// Where a selection was first made, for a report that has to point at an operator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Occurrence {
    /// The operator that made it, as the stream wrote it.
    pub operator: Vec<u8>,
    /// How many operators of the innermost frame had run before it, counting this one.
    pub ordinal: usize,
}

/// The selections one interpretation made, deduplicated, with where each was first made.
#[derive(Debug, Clone, Default)]
pub struct Ledger {
    /// The streams being run, outermost first.
    frames: Vec<Frame>,
    /// How many operators each frame has run, parallel to `frames`.
    ordinals: Vec<usize>,
    /// The operator running now, in the innermost frame.
    operator: Vec<u8>,
    /// Every distinct selection, with its first occurrence.
    selections: BTreeMap<Selection, Occurrence>,
    /// Whether [`MAX_SELECTIONS`] stopped the ledger keeping more.
    truncated: bool,
}

impl Ledger {
    /// An empty ledger, with no stream running.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// A content stream has started running.
    pub fn enter(&mut self, route: Route, stream: Option<ObjectId>) {
        self.frames.push(Frame { route, stream });
        self.ordinals.push(0);
    }

    /// The innermost content stream has finished.
    pub fn leave(&mut self) {
        self.frames.pop();
        self.ordinals.pop();
    }

    /// One more operator has begun in the innermost stream.
    pub fn operator(&mut self, word: &[u8]) {
        if let Some(count) = self.ordinals.last_mut() {
            *count = count.saturating_add(1);
        }
        self.operator.clear();
        self.operator.extend_from_slice(word);
    }

    /// A named-resource lookup was made, and this is what it found.
    pub fn select(&mut self, category: &'static str, name: &[u8], entry: Option<&Object>) {
        let outcome = match entry {
            None => Outcome::Missing,
            Some(Object::Reference(id)) => Outcome::Reference(*id),
            Some(other) => Outcome::Direct(format!("{other:?}")),
        };
        let selection = Selection {
            frames: self.frames.clone(),
            category,
            name: name.to_vec(),
            outcome,
        };
        if self.selections.contains_key(&selection) {
            return;
        }
        if self.selections.len() >= MAX_SELECTIONS {
            self.truncated = true;
            return;
        }
        let occurrence = Occurrence {
            operator: self.operator.clone(),
            ordinal: self.ordinals.last().copied().unwrap_or(0),
        };
        self.selections.insert(selection, occurrence);
    }

    /// Every distinct selection, in a stable order, with where each was first made.
    pub fn selections(&self) -> impl Iterator<Item = (&Selection, &Occurrence)> {
        self.selections.iter()
    }

    /// Whether [`MAX_SELECTIONS`] stopped this ledger keeping more.
    #[must_use]
    pub const fn truncated(&self) -> bool {
        self.truncated
    }

    /// How deep the stream being run is nested — zero when nothing is running.
    #[must_use]
    pub fn depth(&self) -> usize {
        self.frames.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_selection_is_kept_once_with_its_first_occurrence() {
        let mut ledger = Ledger::new();
        ledger.enter(Route::Page, None);
        ledger.operator(b"q");
        ledger.operator(b"Tf");
        let entry = Object::Reference(ObjectId::new(7, 0));
        ledger.select("Font", b"F1", Some(&entry));
        ledger.operator(b"Tf");
        ledger.select("Font", b"F1", Some(&entry));
        ledger.select("Font", b"F2", None);
        ledger.leave();
        assert_eq!(ledger.depth(), 0);

        let kept: Vec<_> = ledger.selections().collect();
        assert_eq!(kept.len(), 2);
        let (first, at) = kept[0];
        assert_eq!(first.name, b"F1");
        assert_eq!(first.outcome, Outcome::Reference(ObjectId::new(7, 0)));
        assert_eq!(
            at.ordinal, 2,
            "the first `Tf` is the second operator of the page"
        );
        assert_eq!(at.operator, b"Tf");
        assert_eq!(kept[1].0.outcome, Outcome::Missing);
    }

    #[test]
    fn a_nested_stream_is_its_own_place() {
        let mut ledger = Ledger::new();
        ledger.enter(Route::Page, None);
        ledger.operator(b"Do");
        ledger.enter(Route::Form, Some(ObjectId::new(3, 0)));
        ledger.operator(b"gs");
        ledger.select("ExtGState", b"GS0", None);
        ledger.leave();
        ledger.operator(b"gs");
        ledger.select("ExtGState", b"GS0", None);
        let kept: Vec<_> = ledger.selections().collect();
        assert_eq!(
            kept.len(),
            2,
            "the same name under two streams is two selections"
        );
        assert_eq!(kept[0].0.frames.len(), 1);
        assert_eq!(kept[1].0.frames.len(), 2);
        assert_eq!(kept[1].0.frames[1].route, Route::Form);
        assert_eq!(kept[1].1.ordinal, 1, "the form's own count, not the page's");
    }
}
