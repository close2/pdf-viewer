//! A non-standard structure type mapped to a standard one, on the operator's word.
//!
//! ISO 19005-2 section 6.7.3.4 requires every non-standard structure type to map, through the
//! role map and further non-standard types if need be, to a standard one; ISO 32000-2 §14.7.3
//! makes the structure tree root's `/RoleMap` that mapping. Which standard type a producer's
//! `Chapter` or `Sidebar` *means* is not in the file, so it is a `supply`: the operator's
//! `role-map` table names it once for a producer's vocabulary (`doc/pdf-a-mitigations.md`,
//! `doc/adr/1990` section 6).
//!
//! **What is written is one entry per type at the end of a chain**: an element's `/S` is followed
//! through the producer's own map as `pdf_model::structure::Tree::role` follows it, and where the
//! chain stops at a type that is neither standard nor mapped, that type gains the entry. Every
//! entry the producer wrote stays. Refused by name: an element stating a namespace (§14.8.6.2's
//! `/RoleMapNS` is that namespace's map, not the root's), a chain whose last type already has an
//! entry that does not resolve (a cycle, or an array target), a type the operator's table does not
//! name, and a root or map this conversion cannot write as an object.

use std::collections::BTreeMap;

use pdf_model::structure::{Child, Tree};
use pdf_syntax::Document;
use pdf_syntax::object::{Dictionary, Name, Object, ObjectId};

/// The requirement this file answers.
pub(super) const SITE: &str = "logical-structure/role-map-terminates-at-a-standard-type";

/// The entries the role map gains, and the object they are written into.
#[derive(Debug)]
pub(super) struct RoleEntries {
    /// The object whose dictionary holds the role map: the structure tree root where the map is
    /// written inside it, or the map itself where the root names it by reference.
    pub(super) at: ObjectId,
    /// Whether `at` is the root, whose `/RoleMap` entry is the dictionary to extend.
    pub(super) in_the_root: bool,
    /// Each non-standard type and the standard type it is mapped to.
    pub(super) entries: BTreeMap<String, String>,
}

/// The entries the operator's table gives this document's unmapped types, or `None` where any
/// unmapped type cannot be answered that way.
pub(super) fn entries(document: &Document, table: &[(String, String)]) -> Option<RoleEntries> {
    let tree = Tree::of(document)?;
    let catalog = document.catalog().ok()?;
    let root = catalog.get("StructTreeRoot")?.as_reference()?;
    let root_dict = document.get(root);
    let root_dict = root_dict.as_dict()?;
    let (at, in_the_root) = match root_dict.get("RoleMap") {
        None | Some(Object::Dictionary(_)) => (root, true),
        Some(Object::Reference(map)) => (*map, false),
        Some(_) => return None,
    };
    let map = document.get_key(root_dict, "RoleMap");
    let walk = tree.walk(document);
    if walk.truncated {
        return None;
    }
    let mut entries = BTreeMap::new();
    for (_, child) in walk.items {
        let Child::Element(element) = child else {
            continue;
        };
        if pdf_archive::standard_structure_element(&tree, document, &element) {
            continue;
        }
        if element.get("NS").is_some() {
            return None;
        }
        let last = tree.role(document, &element)?;
        let already = map.as_dict().is_some_and(|map| map.get(&last).is_some());
        if already {
            return None;
        }
        let (_, standard) = table.iter().find(|(from, _)| *from == last)?;
        if !pdf_archive::standard_structure_type(standard) {
            return None;
        }
        entries.insert(last, standard.clone());
    }
    if entries.is_empty() {
        return None;
    }
    Some(RoleEntries {
        at,
        in_the_root,
        entries,
    })
}

/// The role map dictionary with the entries written in, every entry it held kept.
pub(super) fn extended(map: &Dictionary, entries: &BTreeMap<String, String>) -> Dictionary {
    let mut out = map.clone();
    for (from, to) in entries {
        out.insert(
            Name::new(from.as_bytes()),
            Object::Name(Name::new(to.as_bytes())),
        );
    }
    out
}
