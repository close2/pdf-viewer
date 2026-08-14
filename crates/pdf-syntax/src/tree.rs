//! ISO 32000-2 §7.9.6's name trees and §7.9.7's number trees.
//!
//! Both are the same shape — a balanced tree of dictionaries whose leaves hold sorted
//! key-value pairs, with intermediate nodes carrying a `/Limits` pair so that a lookup can
//! descend without reading the whole structure. §7.9.6 states why they exist rather than
//! dictionaries:
//!
//! > The data structure can represent an arbitrarily large collection of key-value pairs,
//! > which can be looked up efficiently without requiring the entire data structure to be
//! > read from the PDF file.
//!
//! That sentence is `CLAUDE.md` principle 2 written by ISO, and it is the reason this module
//! *descends* rather than flattening: a document with ten thousand named destinations should
//! cost one path from root to leaf, not ten thousand dictionary reads.
//!
//! # One structure, four clauses waiting on it
//!
//! The conformance ledger found this before any document did. §12.3.2.4's named destinations,
//! §12.4.2's page labels, §12.7.7's named pages and §14.7.5.4's `/ParentTree` all need a name
//! or number tree and nothing else in this tree read one, so four families in two clauses were
//! blocked on one small piece of clause 7.

use crate::object::{Dictionary, Object};

/// How deep a tree will be followed.
///
/// A balanced tree over any collection a PDF can hold is far shallower than this; a file
/// claiming otherwise has a cycle or is trying to make the reader walk forever. The visited
/// set below catches a cycle directly, so this is the second line of defence rather than the
/// only one — the same arrangement `xref.rs` uses for the `/Prev` chain.
const MAX_DEPTH: usize = 64;

/// A key in either kind of tree.
///
/// The two clauses differ in exactly this: §7.9.6's keys are strings and §7.9.7's are
/// integers. Everything else — `/Kids`, `/Limits`, the ordering, the descent — is shared,
/// which is why one module answers both rather than two answering one each.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TreeKey<'a> {
    /// §7.9.6: a name tree's key, compared as bytes.
    ///
    /// Bytes rather than text because the clause makes them *strings*, which §7.9.2 says may
    /// be any of three encodings — and the ordering a producer sorted them by is over the
    /// bytes it wrote, whatever they mean.
    Name(&'a [u8]),
    /// §7.9.7: a number tree's key.
    Number(i64),
}

impl TreeKey<'_> {
    /// The entry naming this kind of tree's key-value pairs: `/Names` or `/Nums`.
    fn entries(&self) -> &'static str {
        match self {
            Self::Name(_) => "Names",
            Self::Number(_) => "Nums",
        }
    }

    /// Orders this key against a value from a `/Limits` array or a pairs list.
    ///
    /// Returns `None` when the object is not a key of this tree's kind, which a malformed
    /// file can produce and which the caller treats as "this node cannot answer".
    fn compare(&self, other: &Object) -> Option<std::cmp::Ordering> {
        match (self, other) {
            (Self::Name(key), Object::String(bytes)) => Some(key.cmp(&bytes.as_ref())),
            (Self::Number(key), Object::Integer(value)) => Some(key.cmp(value)),
            _ => None,
        }
    }
}

/// Looks a key up in a name or number tree, descending from `root`.
///
/// `resolve` is how an indirect reference is followed; in practice it is
/// `|object| document.resolve(object)`, and it is a parameter rather than a `&Document`
/// because this module is pure structure and has no business knowing what a document is.
///
/// Returns the value the tree associates with the key, already resolved, or `None` when no
/// leaf holds it.
///
/// # The `/Limits` are a hint, not a gate
///
/// The clause requires an intermediate or leaf node to carry `/Limits` and requires the keys
/// to be sorted, and a file that gets either wrong is common enough that trusting them blindly
/// loses entries that are present. So a node whose `/Limits` exclude the key is skipped and a
/// node with *no* `/Limits` is searched — which costs a wrong-but-not-fatal file a wider walk
/// and never costs a well-formed one anything, since a well-formed file's limits exclude at
/// most one subtree per level.
pub fn lookup(
    root: &Dictionary,
    key: &TreeKey<'_>,
    resolve: &dyn Fn(&Object) -> Object,
) -> Option<Object> {
    lookup_unresolved(root, key, resolve).map(|found| resolve(&found))
}

/// [`lookup`], with the value left **as the tree states it**: a reference stays a reference.
///
/// Almost every consumer of a name or number tree wants the value itself, which is why
/// [`lookup`] resolves before it answers. ISO 32000-2 §14.7.5.4's structural parent tree is the
/// exception, and it is the reason this exists: its values *are* references — "the value shall be
/// an indirect reference to the parent structure element" — so a caller asking which structure
/// element a content item belongs to is asking for an identity, and resolving throws exactly that
/// away. Two elements of one document can be equal dictionaries; they are never the same object.
///
/// `resolve` is still needed, because descending the tree follows references whatever the value
/// turns out to be.
pub fn lookup_unresolved(
    root: &Dictionary,
    key: &TreeKey<'_>,
    resolve: &dyn Fn(&Object) -> Object,
) -> Option<Object> {
    let mut visited = std::collections::BTreeSet::new();
    descend(root, key, resolve, 0, &mut visited)
}

/// One level of the walk; see [`lookup`]. Answers the value unresolved.
fn descend(
    node: &Dictionary,
    key: &TreeKey<'_>,
    resolve: &dyn Fn(&Object) -> Object,
    depth: usize,
    visited: &mut std::collections::BTreeSet<u32>,
) -> Option<Object> {
    if depth > MAX_DEPTH {
        return None;
    }

    // A leaf's or the root's own pairs. The clause says a node has `/Kids` or the pairs entry
    // "but not both"; a file writing both is answered by whichever holds the key, which is
    // more than the clause promises and costs nothing.
    if let Object::Array(pairs) = resolve(node.get(key.entries()).unwrap_or(&Object::Null)) {
        // `[key1 value1 key2 value2 …]`, so the keys are the even positions.
        for pair in pairs.chunks_exact(2) {
            let (Some(candidate), Some(value)) = (pair.first(), pair.get(1)) else {
                continue;
            };
            if key.compare(&resolve(candidate)) == Some(std::cmp::Ordering::Equal) {
                return Some(value.clone());
            }
        }
    }

    let Object::Array(kids) = resolve(node.get("Kids").unwrap_or(&Object::Null)) else {
        return None;
    };
    for kid in &kids {
        // A `/Kids` entry "shall be an array of indirect references", so a cycle is possible
        // and is bounded here by object number rather than by depth alone.
        if let Object::Reference(id) = kid
            && !visited.insert(id.number)
        {
            continue;
        }
        let child = resolve(kid);
        let Some(child) = child.as_dict() else {
            continue;
        };
        if !admits(child, key, resolve) {
            continue;
        }
        if let Some(found) = descend(child, key, resolve, depth.saturating_add(1), visited) {
            return Some(found);
        }
    }
    None
}

/// Whether a node's `/Limits` allow the key to be inside it.
///
/// §7.9.7's Table 37 makes it two keys, the least and the greatest held by this node or by any
/// leaf below it — so a node whose limits exclude the key holds nothing worth descending into.
///
/// A node with no `/Limits`, or with limits this key cannot be compared against, is admitted:
/// see [`lookup`] for why a missing hint widens the search rather than ending it.
fn admits(node: &Dictionary, key: &TreeKey<'_>, resolve: &dyn Fn(&Object) -> Object) -> bool {
    let Object::Array(limits) = resolve(node.get("Limits").unwrap_or(&Object::Null)) else {
        return true;
    };
    let (Some(low), Some(high)) = (limits.first(), limits.get(1)) else {
        return true;
    };
    let below = key.compare(&resolve(low)) == Some(std::cmp::Ordering::Less);
    let above = key.compare(&resolve(high)) == Some(std::cmp::Ordering::Greater);
    !below && !above
}

/// Every key-value pair in a number tree, in ascending key order.
///
/// §12.4.2's page labels need this rather than a lookup: a labelling range covers every page
/// from its own key up to the next one, so the *neighbouring* keys are the answer and no single
/// lookup produces them. Walking the whole tree is right here for the reason the clause's own
/// efficiency argument is not: a document has as many labelling ranges as it has numbering
/// styles, which is a handful, and never as many as it has pages.
///
/// Returns pairs in the order the tree holds them, which the clause requires to be ascending;
/// a file that sorted them wrongly gets them back in its own order rather than corrected,
/// because correcting it would hide the defect from anything that checks.
#[must_use]
pub fn number_pairs(root: &Dictionary, resolve: &dyn Fn(&Object) -> Object) -> Vec<(i64, Object)> {
    let mut out = Vec::new();
    let mut visited = std::collections::BTreeSet::new();
    collect(
        root,
        "Nums",
        resolve,
        0,
        &mut visited,
        &mut |key, value, out: &mut Vec<(i64, Object)>| {
            if let Some(key) = key.as_integer() {
                out.push((key, resolve(&value)));
            }
        },
        &mut out,
    );
    out
}

/// Every key-value pair in a *name* tree, in the order the tree holds them.
///
/// The same walk as [`number_pairs`] over §7.9.6's `/Names` array rather than §7.9.7's `/Nums`,
/// because the two clauses define one structure twice — "[a] name tree serves a similar purpose
/// to a dictionary … but by different means", and §7.9.7 opens by saying a number tree "shall
/// serve a similar purpose to a name tree … except that its keys shall be integers".
///
/// A whole-tree walk rather than a lookup for the reason §7.11.4's attachments need: a caller
/// listing a document's embedded files has no key to look up. Keys come back as the bytes the
/// file wrote, since §7.9.6 sorts them "by unsigned character code" and what they *mean* is
/// §7.9.2's question.
#[must_use]
pub fn name_pairs(
    root: &Dictionary,
    resolve: &dyn Fn(&Object) -> Object,
) -> Vec<(Vec<u8>, Object)> {
    let mut out = Vec::new();
    let mut visited = std::collections::BTreeSet::new();
    collect(
        root,
        "Names",
        resolve,
        0,
        &mut visited,
        &mut |key, value, out: &mut Vec<(Vec<u8>, Object)>| {
            if let Object::String(bytes) = key {
                out.push((bytes.to_vec(), resolve(&value)));
            }
        },
        &mut out,
    );
    out
}

/// The same walk as [`name_pairs`], keeping each value **as the leaf states it**.
///
/// §12.7.7's two page-naming trees are why this exists: what identifies a page is its object
/// identity — it is what §12.3.2's destinations carry and what `Pages::index_of` compares — and
/// [`name_pairs`] resolves a leaf's value, which throws the identity away before a caller can
/// ask for it. A tree whose leaves are direct objects yields those objects unchanged, which is
/// a file naming a page it did not make an indirect object of.
#[must_use]
pub fn name_entries(
    root: &Dictionary,
    resolve: &dyn Fn(&Object) -> Object,
) -> Vec<(Vec<u8>, Object)> {
    let mut out = Vec::new();
    let mut visited = std::collections::BTreeSet::new();
    collect(
        root,
        "Names",
        resolve,
        0,
        &mut visited,
        &mut |key, value, out: &mut Vec<(Vec<u8>, Object)>| {
            if let Object::String(bytes) = key {
                out.push((bytes.to_vec(), value));
            }
        },
        &mut out,
    );
    out
}

/// One level of the walk [`number_pairs`] and [`name_pairs`] share.
///
/// `entry` is the leaf array's key — `/Nums` or `/Names` — and `push` is what a pair means to
/// the caller. The two trees differ in exactly those two places, which is why this is one
/// function: §7.9.7 defines a number tree as a name tree with integer keys, and a second copy
/// of the descent would be a second place for the cycle guard to be got wrong.
fn collect<T>(
    node: &Dictionary,
    entry: &str,
    resolve: &dyn Fn(&Object) -> Object,
    depth: usize,
    visited: &mut std::collections::BTreeSet<u32>,
    push: &mut dyn FnMut(Object, Object, &mut Vec<T>),
    out: &mut Vec<T>,
) {
    if depth > MAX_DEPTH {
        return;
    }
    if let Object::Array(pairs) = resolve(node.get(entry).unwrap_or(&Object::Null)) {
        for pair in pairs.chunks_exact(2) {
            let (Some(key), Some(value)) = (pair.first(), pair.get(1)) else {
                continue;
            };
            // The key is resolved and the value is not. A caller almost always wants the
            // object a leaf names, and resolves it — but §12.7.7's named pages want the
            // *reference*, because what identifies a page is its object identity, and a
            // resolution here would throw that away before anybody could ask for it.
            push(resolve(key), value.clone(), out);
        }
    }
    let Object::Array(kids) = resolve(node.get("Kids").unwrap_or(&Object::Null)) else {
        return;
    };
    for kid in &kids {
        if let Object::Reference(id) = kid
            && !visited.insert(id.number)
        {
            continue;
        }
        if let Some(child) = resolve(kid).as_dict() {
            collect(
                child,
                entry,
                resolve,
                depth.saturating_add(1),
                visited,
                push,
                out,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{TreeKey, lookup, number_pairs};
    use crate::object::{Dictionary, Name, Object};

    /// Builds a node dictionary from entries, keeping the test fixtures readable.
    fn node(entries: &[(&str, Object)]) -> Dictionary {
        let mut dict = Dictionary::new();
        for (key, value) in entries {
            dict.insert(Name::new(key.as_bytes().to_vec()), value.clone());
        }
        dict
    }

    fn string(text: &str) -> Object {
        Object::String(text.as_bytes().into())
    }

    /// Everything here is direct, so resolution is the identity.
    fn direct(object: &Object) -> Object {
        object.clone()
    }

    /// A three-level tree: root, two intermediate nodes, four leaves.
    ///
    /// Deliberately not one leaf: §7.9.6's whole point is that a lookup descends, and a
    /// single-node fixture would pass with the descent deleted.
    fn names() -> Dictionary {
        let leaf = |low: &str, high: &str, pairs: Vec<Object>| {
            Object::Dictionary(node(&[
                ("Limits", Object::Array(vec![string(low), string(high)])),
                ("Names", Object::Array(pairs)),
            ]))
        };
        let branch = |low: &str, high: &str, kids: Vec<Object>| {
            Object::Dictionary(node(&[
                ("Limits", Object::Array(vec![string(low), string(high)])),
                ("Kids", Object::Array(kids)),
            ]))
        };
        let pair = |key: &str, value: i64| vec![string(key), Object::Integer(value)];
        let two = |a: &str, av: i64, b: &str, bv: i64| {
            let mut out = pair(a, av);
            out.extend(pair(b, bv));
            out
        };
        node(&[(
            "Kids",
            Object::Array(vec![
                branch(
                    "aa",
                    "ad",
                    vec![
                        leaf("aa", "ab", two("aa", 1, "ab", 2)),
                        leaf("ac", "ad", two("ac", 3, "ad", 4)),
                    ],
                ),
                branch(
                    "ba",
                    "bd",
                    vec![
                        leaf("ba", "bb", two("ba", 5, "bb", 6)),
                        leaf("bc", "bd", two("bc", 7, "bd", 8)),
                    ],
                ),
            ]),
        )])
    }

    /// Every key in the tree is found, and one that is not in it is not invented.
    ///
    /// Three levels and four leaves on purpose: §7.9.6's whole point is that a lookup
    /// *descends*, and every key here sits two `/Kids` below the root, so the test fails if
    /// the descent is deleted. `bd`, the last key under the second branch, is asserted
    /// alongside `aa` so that stopping after the first subtree fails too.
    #[test]
    fn a_key_is_found_through_intermediate_nodes() {
        let root = names();
        for (key, value) in [
            ("aa", 1),
            ("ab", 2),
            ("ac", 3),
            ("ad", 4),
            ("ba", 5),
            ("bb", 6),
            ("bc", 7),
            ("bd", 8),
        ] {
            assert_eq!(
                lookup(&root, &TreeKey::Name(key.as_bytes()), &direct),
                Some(Object::Integer(value)),
                "{key}"
            );
        }
        assert_eq!(lookup(&root, &TreeKey::Name(b"az"), &direct), None);
        // A number key cannot address a name tree, and asking must answer nothing rather
        // than falling through to the first pair.
        assert_eq!(lookup(&root, &TreeKey::Number(1), &direct), None);
    }

    /// A node whose `/Limits` exclude the key is skipped, and one with no `/Limits` is not.
    ///
    /// The clause requires intermediate and leaf nodes to carry `/Limits`; real files omit
    /// them, and a reader that treated a missing hint as "no keys here" would lose the
    /// entries. Both halves are asserted because they pull in opposite directions.
    #[test]
    fn limits_narrow_the_search_and_their_absence_widens_it() {
        let unlimited = node(&[(
            "Kids",
            Object::Array(vec![Object::Dictionary(node(&[(
                "Names",
                Object::Array(vec![string("only"), Object::Integer(9)]),
            )]))]),
        )]);
        assert_eq!(
            lookup(&unlimited, &TreeKey::Name(b"only"), &direct),
            Some(Object::Integer(9)),
            "a leaf with no /Limits still holds its keys"
        );

        let lying = node(&[(
            "Kids",
            Object::Array(vec![Object::Dictionary(node(&[
                ("Limits", Object::Array(vec![string("a"), string("b")])),
                (
                    "Names",
                    Object::Array(vec![string("zzz"), Object::Integer(9)]),
                ),
            ]))]),
        )]);
        assert_eq!(
            lookup(&lying, &TreeKey::Name(b"zzz"), &direct),
            None,
            "a key outside a node's stated limits is not searched for inside it"
        );
    }

    /// A number tree's pairs come back in the order the tree holds them, across nodes.
    ///
    /// §12.4.2 needs the *sequence* rather than one key, because a labelling range runs to
    /// the start of the next one — so this asserts the order and not only the membership.
    #[test]
    fn a_number_trees_pairs_come_back_in_order() {
        let leaf = |pairs: Vec<Object>| Object::Dictionary(node(&[("Nums", Object::Array(pairs))]));
        let root = node(&[(
            "Kids",
            Object::Array(vec![
                leaf(vec![
                    Object::Integer(0),
                    string("i"),
                    Object::Integer(4),
                    string("1"),
                ]),
                leaf(vec![Object::Integer(7), string("A-8")]),
            ]),
        )]);

        let pairs = number_pairs(&root, &direct);
        assert_eq!(
            pairs.iter().map(|(key, _)| *key).collect::<Vec<_>>(),
            [0, 4, 7]
        );
        assert_eq!(
            pairs.first().map(|(_, value)| value.clone()),
            Some(string("i"))
        );
    }

    /// A tree whose `/Kids` point back at an ancestor terminates.
    ///
    /// Not hypothetical: `/Kids` "shall be an array of indirect references", so a file can
    /// name any object, and a reader that followed them without a visited set would walk
    /// forever on one that names its own parent.
    #[test]
    fn a_cycle_in_the_kids_terminates() {
        use crate::object::ObjectId;

        let root = node(&[(
            "Kids",
            Object::Array(vec![Object::Reference(ObjectId::new(1, 0))]),
        )]);
        let cyclic = |object: &Object| match object {
            Object::Reference(_) => Object::Dictionary(node(&[(
                "Kids",
                Object::Array(vec![Object::Reference(ObjectId::new(1, 0))]),
            )])),
            other => other.clone(),
        };

        assert_eq!(lookup(&root, &TreeKey::Name(b"anything"), &cyclic), None);
        assert!(number_pairs(&root, &cyclic).is_empty());
    }
}
