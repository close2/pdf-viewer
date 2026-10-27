//! ISO 32000-2 §12.7.4.1's field inheritance, resolved once per field for the document's life.
//!
//! > Many field attributes are inheritable , meaning that if they are not explicitly specified for
//! > a given field, their values are taken from those of its parent in the field hierarchy.
//!
//! > An interactive PDF processor shall not limit the range of inheritance for field dictionaries.
//!
//! Two things a hostile file can do to a `/Parent` chain decide this module's shape. **It can be a
//! cycle**, which [`Ancestors`] finds with two positions on the chain (Brent's method) rather than
//! a set of every dictionary passed, and reports as [`Ending::Cycle`]: a chain that returns to a
//! field it has passed is no hierarchy, so there is no "range" for the clause to protect. **It can
//! be deep and acyclic, with a widget at every link**, which walked once per widget costs widgets ×
//! depth; [`Lineages`] keeps each field's resolved entries beside the document, so every field
//! dictionary is read once whatever number of widgets sit below it. Neither needs a bound on the
//! number of links, which is how the clause's sentence is met (ADR 2448).
//!
//! What a field resolves is [`READ_UP_THE_CHAIN`]: the entries Tables 226, 228, 229 and 230 mark
//! inheritable, and the five this crate's readers also take from the nearest dictionary of the
//! chain stating them. A value is *stated* where it is present and not null, which is §7.3.9's
//! rule for a null entry —
//!
//! > Specifying the null object as the value of a dictionary entry (7.3.7, "Dictionary objects")
//! > shall be equivalent to omitting the entry entirely.
//!
//! — and a stated value of the wrong type is what the field states, for the reader that asked to
//! judge.

use std::collections::HashMap;
use std::sync::{Arc, PoisonError, RwLock};

use pdf_syntax::{Dictionary, Document, Name, Object, ObjectId};

/// The entries a field takes from the nearest dictionary of its `/Parent` chain that states them.
///
/// The first eight are the standard's: Table 226's `/FT`, `/Ff`, `/V` and `/DV`, Table 228's `/DA`
/// and `/Q`, Table 229's `/MaxLen` and Table 230's `/Opt`, each "(… inheritable)" in its table.
/// The last five are not designated inheritable, and are read up the chain because a widget that
/// is not merged with its field holds none of them itself: Table 231's `/TI` and `/I`, which a
/// choice field states, the `/Kids` §12.7.5.2.4 reads a radio button's set from, and Table 228's
/// `/RV` and `/DS`, which the field's variable text states. A reader that asks [`Inherited`] for an
/// entry missing here is told only what the widget itself states.
pub(crate) const READ_UP_THE_CHAIN: [&str; 13] = [
    "FT", "Ff", "V", "DV", "DA", "Q", "MaxLen", "Opt", "TI", "I", "Kids", "RV", "DS",
];

/// How a walk up a `/Parent` chain ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Ending {
    /// A dictionary with no `/Parent` that resolves to a dictionary: the top of the hierarchy.
    Root,
    /// The chain came back to a field it had already passed, so it is no tree.
    Cycle,
}

/// A field dictionary's `/Parent` chain, nearest first, starting with the dictionary it was given.
///
/// Each item is the dictionary with the object it was read from, `None` for the starting one
/// when it was handed over as a dictionary and for a `/Parent` stated directly rather than by
/// reference (§12.7.4.1 makes every field "an indirect object", so that is a damaged file, and
/// one a reader still follows).
///
/// **The cycle is found without a set** (Brent's method): one position, the *tortoise*, stays on a
/// field while the walk moves on, and is moved forward to wherever the walk is each time the walk
/// has taken a power of two steps from it. A chain that ends is walked once; a chain that loops
/// meets the tortoise within twice its loop's length after the tortoise first lands inside the
/// loop, which is where the walk stops and [`Ancestors::ending`] says [`Ending::Cycle`]. Only a
/// dictionary read by reference can be on a loop — a direct dictionary is written inside its
/// parent and cannot contain itself — so only those positions are compared. Some dictionaries of
/// a loop are yielded twice before it is found, which a reader taking the nearest dictionary that
/// states an entry cannot tell apart from yielding them once.
pub(crate) struct Ancestors<F> {
    /// Reads a field's dictionary by its object, so that a save can walk the chain through what
    /// it has changed (`view::writing`) and everything else through the document.
    fetch: F,
    /// The dictionary the next call yields.
    next: Option<(Option<ObjectId>, Dictionary)>,
    /// Where the tortoise is, once the walk has passed an object.
    tortoise: Option<ObjectId>,
    /// The length of the stretch the tortoise is waiting out.
    stretch: usize,
    /// How many steps of that stretch the walk has taken.
    taken: usize,
    /// How the walk ended, once it has.
    ending: Option<Ending>,
}

impl<F> Ancestors<F>
where
    F: Fn(ObjectId) -> Option<Dictionary>,
{
    /// The chain from `start` upward, each parent read through `fetch`.
    pub(crate) fn from(start: (Option<ObjectId>, Dictionary), fetch: F) -> Self {
        Self {
            fetch,
            next: Some(start),
            tortoise: None,
            stretch: 1,
            taken: 0,
            ending: None,
        }
    }

    /// How the walk ended: `None` while it has not.
    pub(crate) fn ending(&self) -> Option<Ending> {
        self.ending
    }

    /// Whether the walk, now at `id`, has met the tortoise; moves the tortoise on where its
    /// stretch is over.
    fn met(&mut self, id: ObjectId) -> bool {
        if self.tortoise == Some(id) {
            return true;
        }
        self.taken = self.taken.saturating_add(1);
        if self.taken == self.stretch {
            self.tortoise = Some(id);
            self.stretch = self.stretch.saturating_mul(2);
            self.taken = 0;
        }
        false
    }
}

impl<F> Iterator for Ancestors<F>
where
    F: Fn(ObjectId) -> Option<Dictionary>,
{
    type Item = (Option<ObjectId>, Dictionary);

    fn next(&mut self) -> Option<Self::Item> {
        let (id, dict) = self.next.take()?;
        if let Some(id) = id
            && self.met(id)
        {
            self.ending = Some(Ending::Cycle);
            return None;
        }
        self.next = parent_of(&dict, &self.fetch);
        if self.next.is_none() {
            self.ending = Some(Ending::Root);
        }
        Some((id, dict))
    }
}

/// The parent `dict` names, with the object it was read from where it names one by reference.
fn parent_of<F: Fn(ObjectId) -> Option<Dictionary>>(
    dict: &Dictionary,
    fetch: &F,
) -> Option<(Option<ObjectId>, Dictionary)> {
    match dict.get("Parent")? {
        Object::Reference(id) => Some((Some(*id), fetch(*id)?)),
        Object::Dictionary(parent) => Some((None, parent.clone())),
        _ => None,
    }
}

/// The chain above `start` in `document`, starting with `start`, whose object is not known.
pub(crate) fn ancestors(
    document: &Document,
    start: Dictionary,
) -> Ancestors<impl Fn(ObjectId) -> Option<Dictionary> + '_> {
    Ancestors::from((None, start), |id| document.get(id).as_dict().cloned())
}

/// The chain above the object `start` in `document`, starting with it; empty where `start` is not
/// a dictionary.
pub(crate) fn ancestors_of(
    document: &Document,
    start: ObjectId,
) -> Ancestors<impl Fn(ObjectId) -> Option<Dictionary> + '_> {
    let fetch = |id: ObjectId| document.get(id).as_dict().cloned();
    let mut walk = Ancestors::from((Some(start), Dictionary::new()), fetch);
    walk.next = (walk.fetch)(start).map(|dict| (Some(start), dict));
    walk
}

/// One field's entries of [`READ_UP_THE_CHAIN`], each the nearest stated one of its chain.
#[derive(Debug, Default)]
pub(crate) struct Resolved {
    /// The entries, as written in the dictionary that stated each: a reference stays one.
    ///
    /// Shared with the parent's where the field states none of them, which is most fields of a
    /// hierarchy: a resolution then costs a reference count, not a copy (ADR 2448 section 5).
    entries: Arc<Dictionary>,
    /// Table 194's and Table 199's triggers, each from the nearest dictionary of the chain whose
    /// `/AA` states it, as written there.
    ///
    /// By trigger rather than by `/AA` whole, because a widget's `/AA` and its field's hold
    /// different triggers — Table 194's for the annotation, Table 199's `/K`, `/F`, `/V` and `/C`
    /// for the field — and the field's script is the one a widget that states only its own
    /// triggers runs. Shared with the parent's where the field states no `/AA`.
    actions: Arc<Dictionary>,
    /// Whether the field's chain is a cycle, which makes it no hierarchy.
    cyclic: bool,
}

/// Every field's [`Resolved`] entries, kept beside the document by [`Document::derived`].
///
/// Keyed by the field's object, which §12.7.4.1 makes indirect. A field is resolved the first
/// time a widget below it is read and never again, so the cost of a hierarchy is its number of
/// dictionaries, not widgets × depth (ADR 2448). Everything kept is a function of the bytes —
/// what a person entered or an import replaced is applied by [`crate::appearance::Field`] over
/// it — so the memo cannot make the document mutable.
#[derive(Debug, Default)]
pub(crate) struct Lineages {
    resolved: RwLock<HashMap<ObjectId, Arc<Resolved>>>,
    /// What a field with no parent inherits, which is nothing: one value for every top of every
    /// hierarchy, so that a widget merged with a top-level field allocates nothing.
    root: Arc<Resolved>,
}

impl Lineages {
    /// The memo's entry for `id`, if it has one. A poisoned lock is read past for the reason
    /// `pdf_syntax::Document`'s own caches are: every write is one insert of a value the bytes
    /// determine, so a panic between two leaves a memo merely smaller.
    fn get(&self, id: ObjectId) -> Option<Arc<Resolved>> {
        self.resolved
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .get(&id)
            .cloned()
    }
}

/// What a widget's field states and inherits: the widget's own dictionary, then what the nearest
/// dictionary above it states of [`READ_UP_THE_CHAIN`].
#[derive(Debug)]
pub(crate) struct Inherited {
    /// The dictionary the reader was handed, which states its own entries first.
    widget: Dictionary,
    /// What its chain above it resolves to.
    above: Arc<Resolved>,
}

impl Inherited {
    /// Resolves `widget`'s chain, reading each field above it at most once per document.
    pub(crate) fn of(document: &Document, widget: &Dictionary) -> Self {
        let memo = document.derived::<Lineages>();
        let fetch = |id: ObjectId| document.get(id).as_dict().cloned();
        // The walk starts above the widget, which the reader already holds.
        let Some(start) = parent_of(widget, &fetch) else {
            return Self {
                widget: widget.clone(),
                above: Arc::clone(&memo.root),
            };
        };
        let mut walk = Ancestors::from(start, fetch);
        let mut path = Vec::new();
        let mut above: Option<Arc<Resolved>> = None;
        for (id, dict) in walk.by_ref() {
            if let Some(found) = id.and_then(|id| memo.get(id)) {
                above = Some(found);
                break;
            }
            path.push((id, dict));
        }
        let cyclic = walk.ending() == Some(Ending::Cycle);
        let mut above = match above {
            Some(found) => found,
            None if cyclic => Arc::new(Resolved {
                cyclic,
                ..Resolved::default()
            }),
            None => Arc::clone(&memo.root),
        };
        let mut written = Vec::new();
        // Filled from the top down, so that each field's entries are its own over its parent's:
        // the work is one merge per dictionary the walk had not resolved before.
        for (id, dict) in path.into_iter().rev() {
            let resolved = Arc::new(Resolved {
                entries: stated_over(document, &dict, &above.entries),
                actions: actions_over(document, &dict, &above.actions),
                cyclic: cyclic || above.cyclic,
            });
            if let Some(id) = id {
                written.push((id, Arc::clone(&resolved)));
            }
            above = resolved;
        }
        if !written.is_empty() {
            let mut resolved = memo
                .resolved
                .write()
                .unwrap_or_else(PoisonError::into_inner);
            // Top first and nearest last, so that a field a cycle passed twice keeps what the
            // walk found from its first passing; either is refused by its `cyclic`.
            for (id, entry) in written {
                resolved.insert(id, entry);
            }
        }
        Self {
            widget: widget.clone(),
            above,
        }
    }

    /// The two dictionaries a reader looks through, nearest first: the widget's own, then what
    /// its chain resolves to. Only [`READ_UP_THE_CHAIN`]'s entries are in the second.
    pub(crate) fn chain(&self) -> [&Dictionary; 2] {
        [&self.widget, &self.above.entries]
    }

    /// `key`'s nearest stated value, references resolved, or `Null` where nothing states it.
    pub(crate) fn get(&self, document: &Document, key: &str) -> Object {
        self.chain()
            .into_iter()
            .map(|dict| document.get_key(dict, key))
            .find(|value| !value.is_null())
            .unwrap_or(Object::Null)
    }

    /// `key`'s value where the nearest dictionary of [`Inherited::chain`] that states an integer
    /// for it states one.
    pub(crate) fn integer(&self, document: &Document, key: &str) -> Option<i64> {
        self.chain()
            .into_iter()
            .find_map(|dict| document.get_key(dict, key).as_integer())
    }

    /// `key`'s value where the nearest dictionary of [`Inherited::chain`] that states a name for
    /// it states one.
    pub(crate) fn name(&self, document: &Document, key: &str) -> Option<Name> {
        self.chain()
            .into_iter()
            .find_map(|dict| document.get_key(dict, key).as_name().cloned())
    }

    /// The action the nearest dictionary of the chain whose `/AA` states `trigger` states for it,
    /// as written there.
    pub(crate) fn action(&self, document: &Document, trigger: &str) -> Option<Object> {
        document
            .get_key(&self.widget, "AA")
            .as_dict()
            .and_then(|additional| additional.get(trigger).cloned())
            .or_else(|| self.above.actions.get(trigger).cloned())
    }

    /// Whether the widget's `/Parent` chain is a cycle rather than a hierarchy.
    pub(crate) fn cyclic(&self) -> bool {
        self.above.cyclic
    }
}

/// The triggers `dict`'s `/AA` states over `above`'s. An `/AA` that is not a dictionary states
/// none.
fn actions_over(
    document: &Document,
    dict: &Dictionary,
    above: &Arc<Dictionary>,
) -> Arc<Dictionary> {
    let stated = document.get_key(dict, "AA");
    let Some(additional) = stated.as_dict() else {
        return Arc::clone(above);
    };
    let mut actions = Dictionary::clone(above);
    for (trigger, action) in additional.iter() {
        actions.insert(trigger.clone(), action.clone());
    }
    Arc::new(actions)
}

/// Whether `value`, as written in a dictionary, states something: a null does not, and neither
/// does a reference to one (§7.3.9: a reference "to a nonexistent object shall be treated the
/// same as a null object").
fn states(document: &Document, value: &Object) -> bool {
    match value {
        Object::Null => false,
        Object::Reference(_) => !document.resolve(value).is_null(),
        _ => true,
    }
}

/// `dict`'s stated entries of [`READ_UP_THE_CHAIN`] over `above`'s.
fn stated_over(document: &Document, dict: &Dictionary, above: &Arc<Dictionary>) -> Arc<Dictionary> {
    let own: Vec<(&str, &Object)> = READ_UP_THE_CHAIN
        .into_iter()
        .filter_map(|key| Some((key, dict.get(key)?)))
        .filter(|(_, value)| states(document, value))
        .collect();
    if own.is_empty() {
        return Arc::clone(above);
    }
    let mut entries = Dictionary::clone(above);
    for (key, value) in own {
        entries.insert(Name::new(key.as_bytes().to_vec()), value.clone());
    }
    Arc::new(entries)
}

#[cfg(test)]
mod tests {
    use std::fmt::Write as _;

    use pdf_syntax::{Document, ObjectId};

    use super::{Ending, Inherited, ancestors_of};

    fn document(objects: &str) -> Document {
        let body = format!(
            "%PDF-1.7\n\
             1 0 obj << /Type /Catalog /Pages 2 0 R >> endobj\n\
             2 0 obj << /Type /Pages /Kids [] /Count 0 >> endobj\n\
             {objects}\
             trailer << /Root 1 0 R >>\n"
        );
        Document::open(body.into_bytes()).expect("the fixture parses")
    }

    fn id(number: u32) -> ObjectId {
        ObjectId::new(number, 0)
    }

    /// A field that is its own parent, two that are each other's, and a tail leading into a loop
    /// of three are each found to be a cycle, and the walk ends; a chain with a top ends at it.
    #[test]
    fn a_chain_that_returns_to_a_field_it_passed_ends_as_a_cycle() {
        let document = document(
            "4 0 obj << /Parent 4 0 R >> endobj\n\
             5 0 obj << /Parent 6 0 R >> endobj\n\
             6 0 obj << /Parent 5 0 R >> endobj\n\
             7 0 obj << /Parent 8 0 R >> endobj\n\
             8 0 obj << /Parent 9 0 R >> endobj\n\
             9 0 obj << /Parent 10 0 R >> endobj\n\
             10 0 obj << /Parent 11 0 R >> endobj\n\
             11 0 obj << /Parent 9 0 R >> endobj\n\
             12 0 obj << /Parent 13 0 R >> endobj\n\
             13 0 obj << /T (top) >> endobj\n",
        );
        for start in [4, 5, 7] {
            let mut walk = ancestors_of(&document, id(start));
            let passed = walk.by_ref().count();
            assert_eq!(walk.ending(), Some(Ending::Cycle), "from {start}");
            assert!(passed <= 12, "from {start}, {passed} dictionaries");
        }
        let mut walk = ancestors_of(&document, id(12));
        assert_eq!(walk.by_ref().count(), 2);
        assert_eq!(walk.ending(), Some(Ending::Root));
    }

    /// The clause's own sentence, held where it acts: a chain of four thousand links, a widget at
    /// every one, each reads the value the top states — and the whole is one merge per field,
    /// which a walk per widget would make eight million dictionary reads.
    #[test]
    fn every_widget_of_a_chain_past_any_bound_reads_what_its_top_states() {
        let links = 4000;
        let mut objects =
            String::from("4 0 obj << /FT /Tx /V (top) /DA (/Helv 9 Tf 0 g) >> endobj\n");
        for link in 0..links {
            let _ = writeln!(
                objects,
                "{} 0 obj << /Parent {} 0 R >> endobj",
                5 + link,
                4 + link
            );
        }
        let document = document(&objects);
        for link in (0..links).rev() {
            let widget = document.get(id(5 + link));
            let widget = widget.as_dict().expect("the fixture's dictionary");
            let inherited = Inherited::of(&document, widget);
            assert!(!inherited.cyclic());
            assert_eq!(
                inherited.get(&document, "V").as_string(),
                Some(&b"top"[..]),
                "link {link}"
            );
        }
    }

    /// A null is no statement (§7.3.9), so a field stating `/V null` inherits its parent's.
    #[test]
    fn a_null_entry_is_inherited_past() {
        let document = document(
            "4 0 obj << /V (top) >> endobj\n\
             5 0 obj << /V null /Parent 4 0 R >> endobj\n\
             6 0 obj << /Parent 5 0 R >> endobj\n",
        );
        let widget = document.get(id(6));
        let inherited = Inherited::of(&document, widget.as_dict().expect("a dictionary"));
        assert_eq!(inherited.get(&document, "V").as_string(), Some(&b"top"[..]));
    }
}
