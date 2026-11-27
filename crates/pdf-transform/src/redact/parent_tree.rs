//! A form's placement copy given an entry of its own in the structural parent tree (ADR 3846).
//!
//! A form drawn more than once that a region meets differently is written as the producer's form
//! and a copy per distinct edit (ADR 1363). A form whose sequences are structure content items
//! states the key of its entry in the parent tree, ISO 32000-2 §14.7.5.4 Table 359:
//!
//! > The integer key of this object's entry in the structural parent tree.
//!
//! A copy carrying the form's key shares the form's entry, so the tree cannot say which sequence
//! of which stream is still shown: a sequence the region took whole from one placement would be
//! found, through the copy, as the element's content still. Each copy is therefore keyed anew from
//! `/ParentTreeNextKey`, as §14.7.5.4 asks of an added entry —
//!
//! > Whenever a new entry is added to the parent tree, the current value of ParentTreeNextKey
//! > shall be used as its key.
//!
//! — and its entry is the form's array with a null where the copy no longer holds the sequence:
//! every reference in it is one the producer wrote, so nothing is invented. The form's own entry
//! is not touched, and neither is any other placement's.
//!
//! The new pairs go after every key the tree holds: at the end of a root's own `/Nums`, or in one
//! new leaf after a root's `/Kids` (Table 37 asks `/Limits` of a leaf and not of the root), so no
//! producer leaf and no `/Limits` above one is rewritten.

use std::collections::{BTreeMap, HashSet};

use pdf_syntax::Document;
use pdf_syntax::object::{Dictionary, Name, Object, ObjectId};

use super::MAX_DEPTH;

/// How many nodes the walk of a parent tree reads before it stops reading: a tree past it is left
/// as the producer wrote it, and the copies keep the form's key.
const MOST_NODES: usize = 100_000;

/// One placement copy of a form that states `/StructParents`.
pub(super) struct Copy {
    /// The copy, as the redacted page's index and the page's form edit.
    pub(super) at: (usize, usize),
    /// The form's own key.
    pub(super) key: i64,
    /// The `/MCID`s of the sequences the region took whole from this copy.
    pub(super) taken: HashSet<i64>,
}

/// The parent tree's new entries, and where each copy's key goes.
pub(super) struct Extension {
    /// Each copy's own key.
    pub(super) keys: Vec<((usize, usize), i64)>,
    /// The new pairs, keys ascending past every key the tree holds, in the source's numbering.
    pub(super) pairs: Vec<(i64, Object)>,
    /// `/ParentTreeNextKey` after them.
    pub(super) next: i64,
}

/// What a walk of the tree found: the values of the keys asked for, and the greatest key.
struct Reading {
    wanted: BTreeMap<i64, Option<Object>>,
    greatest: Option<i64>,
    nodes: usize,
}

impl Reading {
    /// Reads `node` and its kids, `false` where the tree is past [`MOST_NODES`] or [`MAX_DEPTH`].
    fn read(
        &mut self,
        document: &Document,
        node: &Dictionary,
        depth: usize,
        seen: &mut HashSet<ObjectId>,
    ) -> bool {
        self.nodes = self.nodes.saturating_add(1);
        if self.nodes > MOST_NODES || depth > MAX_DEPTH {
            return false;
        }
        if let Some(nums) = document.get_key(node, "Nums").as_array() {
            for pair in nums.as_chunks::<2>().0 {
                let [key, value] = pair;
                let Some(key) = document.resolve(key).as_integer() else {
                    continue;
                };
                self.greatest = Some(self.greatest.map_or(key, |greatest| greatest.max(key)));
                if let Some(slot) = self.wanted.get_mut(&key) {
                    *slot = Some(value.clone());
                }
            }
        }
        if let Some(kids) = document.get_key(node, "Kids").as_array() {
            for kid in kids {
                if let Some(id) = kid.as_reference()
                    && !seen.insert(id)
                {
                    continue;
                }
                let kid = document.resolve(kid);
                let Some(kid) = kid.as_dict() else {
                    continue;
                };
                if !self.read(document, kid, depth.saturating_add(1), seen) {
                    return false;
                }
            }
        }
        true
    }
}

/// The entries `copies` take in the parent tree `tree` (the node `/ParentTree` states, `at` where
/// it is an object), keyed from `stated_next`, or `None` where no copy can be given one: the tree
/// is past what the walk reads, or no copy's form has an array entry under its key.
pub(super) fn extend(
    document: &Document,
    (tree, at): (&Dictionary, Option<ObjectId>),
    stated_next: Option<i64>,
    copies: &[Copy],
) -> Option<Extension> {
    let mut reading = Reading {
        wanted: copies.iter().map(|copy| (copy.key, None)).collect(),
        greatest: None,
        nodes: 0,
    };
    let mut seen: HashSet<ObjectId> = at.into_iter().collect();
    if !reading.read(document, tree, 0, &mut seen) {
        return None;
    }
    // §14.7.5.4: the next key "shall hold an integer value greater than any that is currently in
    // use"; a producer's value that does not is not used, since a key it gave would be shared.
    let past_greatest = reading
        .greatest
        .map_or(0, |greatest| greatest.saturating_add(1));
    let mut next = stated_next.map_or(past_greatest, |stated| stated.max(past_greatest));
    let mut keys = Vec::new();
    let mut pairs = Vec::new();
    for copy in copies {
        let Some(Some(entry)) = reading.wanted.get(&copy.key) else {
            continue;
        };
        let Some(sequences) = document.resolve(entry).as_array().map(<[Object]>::to_vec) else {
            continue;
        };
        let value: Vec<Object> = sequences
            .into_iter()
            .enumerate()
            .map(|(mcid, element)| {
                let taken = i64::try_from(mcid).is_ok_and(|mcid| copy.taken.contains(&mcid));
                if taken { Object::Null } else { element }
            })
            .collect();
        keys.push((copy.at, next));
        pairs.push((next, Object::Array(value)));
        next = next.saturating_add(1);
    }
    (!pairs.is_empty()).then_some(Extension { keys, pairs, next })
}

/// `tree` with `pairs` at the end of its own `/Nums`, or, for a root with `/Kids`, the leaf that
/// holds them: `tree` is then answered with its `/Kids` written in place, and the caller adds the
/// leaf's reference to them once the leaf has a number.
pub(super) fn with_pairs(
    document: &Document,
    tree: &Dictionary,
    pairs: &[(i64, Object)],
) -> (Dictionary, Option<Dictionary>) {
    let flat: Vec<Object> = pairs
        .iter()
        .flat_map(|(key, value)| [Object::Integer(*key), value.clone()])
        .collect();
    if let Some(kids) = document.get_key(tree, "Kids").as_array() {
        let least = pairs.first().map_or(0, |(key, _)| *key);
        let greatest = pairs.last().map_or(0, |(key, _)| *key);
        let mut leaf = Dictionary::new();
        leaf.insert(
            Name::new(&b"Limits"[..]),
            Object::Array(vec![Object::Integer(least), Object::Integer(greatest)]),
        );
        leaf.insert(Name::new(&b"Nums"[..]), Object::Array(flat));
        // Written into the root's own dictionary even where the producer stated the array
        // through a reference: the root is rewritten either way, and the shared array is not.
        let mut out = tree.clone();
        out.insert(Name::new(&b"Kids"[..]), Object::Array(kids.to_vec()));
        return (out, Some(leaf));
    }
    let mut out = tree.clone();
    let mut nums = document
        .get_key(tree, "Nums")
        .as_array()
        .map_or_default(<[Object]>::to_vec);
    nums.extend(flat);
    out.insert(Name::new(&b"Nums"[..]), Object::Array(nums));
    (out, None)
}
