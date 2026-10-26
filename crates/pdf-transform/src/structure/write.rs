//! The rebuilt structure tree, element by element, and the parent tree that points back at the
//! content (ISO 32000-2 §14.7.2, §14.7.5.4).

use pdf_syntax::object::{Dictionary, Name, Object};
use pdf_syntax::{Document, tree};

use super::{Carry, Child, Element, Host, Wrapper, structure_root};
use crate::Warning;

impl Carry {
    /// Builds every kept element, §14.7.5.4's parent tree and the structure tree root.
    ///
    /// Run after the closure walk, because the object keys above are assigned by it.
    pub(crate) fn finish(mut self, host: &mut dyn Host, warnings: &mut Vec<Warning>) {
        let elements = std::mem::take(&mut self.elements);
        for element in &elements {
            let Some(placed) = self.placed.get(&(element.at, element.source)).copied() else {
                continue;
            };
            let built = self.element(host, element);
            host.place_object(placed, built);
        }

        let parent_tree = self.parent_tree(&*host);
        let mut root = Dictionary::new();
        root.insert(
            Name::new(&b"Type"[..]),
            Object::Name(Name::new(&b"StructTreeRoot"[..])),
        );
        let mut tops: Vec<Object> = Vec::with_capacity(self.tops.len());
        let mut wrapper_placed = false;
        for key in &self.tops {
            let Some(placed) = self.placed.get(key).copied() else {
                continue;
            };
            match &self.wrapper {
                Some(wrapper) if wrapper.children.contains(key) => {
                    if !wrapper_placed {
                        tops.push(Object::Reference(wrapper.id));
                        wrapper_placed = true;
                    }
                }
                _ => tops.push(Object::Reference(placed)),
            }
        }
        if let Some(wrapper) = &self.wrapper {
            host.place_object(wrapper.id, self.wrapper_element(wrapper));
        }
        if !tops.is_empty() {
            root.insert(Name::new(&b"K"[..]), Object::Array(tops));
        }
        if let Some(tree) = parent_tree {
            root.insert(Name::new(&b"ParentTree"[..]), tree);
            // §14.7.5.4: "The ParentTreeNextKey entry in the structure tree root shall hold an
            // integer value greater than any that is currently in use as a key in the structural
            // parent tree."
            root.insert(
                Name::new(&b"ParentTreeNextKey"[..]),
                Object::Integer(self.next_key),
            );
        }
        for (entry, held) in [("RoleMap", &self.role_map), ("ClassMap", &self.class_map)] {
            if held.is_empty() {
                continue;
            }
            let mut dict = Dictionary::new();
            for (key, value) in held {
                dict.insert(Name::new(key.as_slice()), value.clone());
            }
            root.insert(Name::new(entry.as_bytes()), Object::Dictionary(dict));
        }
        let ids: Vec<Object> = self
            .ids
            .iter()
            .filter_map(|(bytes, key)| {
                self.placed
                    .get(key)
                    .map(|placed| (bytes.clone(), Object::Reference(*placed)))
            })
            .flat_map(|(bytes, value)| [Object::String(bytes.into()), value])
            .collect();
        if !ids.is_empty() {
            // §7.9.6 permits the whole tree in one node — "[i]f the root node has a Names entry,
            // it shall be the only node in the tree" — and the keys go in sorted, which is what
            // the clause requires of each node's `/Names` array.
            let mut node = Dictionary::new();
            node.insert(Name::new(&b"Names"[..]), Object::Array(ids));
            root.insert(Name::new(&b"IDTree"[..]), Object::Dictionary(node));
        }
        for (entry, items) in &self.lists {
            if items.is_empty() {
                continue;
            }
            root.insert(Name::new(entry.as_bytes()), Object::Array(items.clone()));
        }
        host.place_object(self.root, Object::Dictionary(root));
        self.report(warnings);
    }

    /// One rebuilt structure element: the source's entries, with the pruned hierarchy's own
    /// `/K`, `/P` and `/Pg`.
    fn element(&self, host: &mut dyn Host, element: &Element) -> Object {
        let source = {
            let Some(document) = host.source(element.at) else {
                return Object::Null;
            };
            match document.get(element.source) {
                Object::Dictionary(dict) => dict,
                _ => return Object::Null,
            }
        };
        let mut out = Dictionary::new();
        for (key, value) in source.iter() {
            match key.as_bytes() {
                // Rebuilt from the pruned hierarchy rather than carried.
                b"K" | b"P" | b"Pg" => {}
                // Table 355's `/Ref` is "zero, one or more indirect references to structure
                // elements", so an entry naming one the output does not hold is dropped rather
                // than written as §7.3.10's null: the array is a list of elements, and a null is
                // not one.
                // §14.7.6.2: "[t]he C entry in a structure element dictionary … shall contain
                // a class name or an array of class names (typically accompanied by revision
                // numbers as well)". The revision numbers are integers between the names and are
                // carried untouched; only a renamed class's name changes.
                b"C" => {
                    let renames = self.class_renames.get(&element.at);
                    let rename = |value: &Object| match (value.as_name(), renames) {
                        (Some(name), Some(renames)) => renames
                            .get(name.as_bytes())
                            .map_or_else(|| value.clone(), |to| Object::Name(Name::new(&to[..]))),
                        _ => value.clone(),
                    };
                    let mapped = match value {
                        Object::Array(items) => Object::Array(items.iter().map(rename).collect()),
                        other => rename(other),
                    };
                    out.insert(key.clone(), mapped);
                }
                b"Ref" => {
                    let items = host
                        .source(element.at)
                        .map(|document| document.resolve(value));
                    let kept: Vec<Object> = match items {
                        Some(Object::Array(items)) => items
                            .iter()
                            .filter_map(Object::as_reference)
                            .filter_map(|id| self.placed.get(&(element.at, id)).copied())
                            .map(Object::Reference)
                            .collect(),
                        _ => Vec::new(),
                    };
                    if !kept.is_empty() {
                        out.insert(key.clone(), Object::Array(kept));
                    }
                }
                _ => {
                    let carried = host.carry_value(element.at, value);
                    out.insert(key.clone(), carried);
                }
            }
        }
        let parent = match element.parent {
            Some(id) => self
                .placed
                .get(&(element.at, id))
                .copied()
                .unwrap_or(self.root),
            None => self
                .wrapper
                .as_ref()
                .filter(|wrapper| wrapper.children.contains(&(element.at, element.source)))
                .map_or(self.root, |wrapper| wrapper.id),
        };
        // Table 355: "( Required; shall be an indirect reference ) The structure element or the
        // structure tree root that is the immediate parent of this structure element."
        out.insert(Name::new(&b"P"[..]), Object::Reference(parent));
        if let Some(page) = element.page {
            out.insert(Name::new(&b"Pg"[..]), Object::Reference(page));
        }
        let mut children: Vec<Object> = Vec::with_capacity(element.children.len());
        for child in &element.children {
            match child {
                Child::Element(id) => {
                    if let Some(placed) = self.placed.get(&(element.at, *id)).copied() {
                        children.push(Object::Reference(placed));
                    }
                }
                Child::Item(object) => children.push(object.clone()),
                Child::Reference { dictionary, page } => {
                    let mut item = Dictionary::new();
                    for (key, value) in dictionary.iter() {
                        // Written explicitly below, so that a pruned hierarchy cannot leave the
                        // reference depending on an entry a dropped ancestor used to state.
                        if key.as_bytes() == b"Pg" {
                            continue;
                        }
                        let carried = host.carry_value(element.at, value);
                        item.insert(key.clone(), carried);
                    }
                    item.insert(Name::new(&b"Pg"[..]), Object::Reference(*page));
                    children.push(Object::Dictionary(item));
                }
            }
        }
        out.insert(Name::new(&b"K"[..]), Object::Array(children));
        Object::Dictionary(out)
    }

    /// The Annex L `Document`: Table 355's required `/S` and `/P`, the namespace its children are
    /// in, and the children in the order their sources contributed them.
    ///
    /// Nothing else is stated, because nothing else is known: a `/Lang`, an `/Alt` or a `/T` would
    /// be a claim about the merged document this program has no source for.
    fn wrapper_element(&self, wrapper: &Wrapper) -> Object {
        let mut out = Dictionary::new();
        out.insert(
            Name::new(&b"Type"[..]),
            Object::Name(Name::new(&b"StructElem"[..])),
        );
        out.insert(
            Name::new(&b"S"[..]),
            Object::Name(Name::new(&b"Document"[..])),
        );
        out.insert(Name::new(&b"NS"[..]), wrapper.namespace.clone());
        out.insert(Name::new(&b"P"[..]), Object::Reference(self.root));
        let children: Vec<Object> = wrapper
            .children
            .iter()
            .filter_map(|key| self.placed.get(key).copied())
            .map(Object::Reference)
            .collect();
        out.insert(Name::new(&b"K"[..]), Object::Array(children));
        Object::Dictionary(out)
    }

    /// §14.7.5.4's parent tree, as one number-tree node.
    ///
    /// > The tree shall contain an entry for each object that is a content item of at least one
    /// > structure element and for each content stream containing at least one marked-content
    /// > sequence that is a content item.
    ///
    /// A page's value is the source's array at its own length and in its own order, because the
    /// array is indexed by the marked-content identifiers in a content stream this suite carries
    /// byte for byte. An object's value is one reference, which is the clause's other bullet.
    ///
    /// The distinction the code below turns on: a page's *array* is resolved and its *members*
    /// are not. Resolving a member would throw away the identity this module maps; leaving the
    /// array unresolved wrote a one-long `[null]` for every source that states it out of line,
    /// which is most of them (ADR 0838).
    fn parent_tree(&mut self, host: &dyn Host) -> Option<Object> {
        let mut nums: Vec<(i64, Object)> = Vec::new();
        let mut orphaned = 0_u64;
        for (at, old, new) in &self.page_sources {
            let Some(value) = host
                .source(*at)
                .and_then(|document| Self::source_entry(document, *old))
            else {
                continue;
            };
            // The array itself may be an indirect object — §7.3.10 makes a reference equivalent
            // to what it names, and most producers write this one out of line — so the *array*
            // is resolved while its members are not. `resolve` follows one object rather than
            // the tree under it, so every member arrives as the reference this module maps.
            let array = host
                .source(*at)
                .map_or(Object::Null, |document| document.resolve(&value));
            let mapped = match array {
                Object::Array(items) => Object::Array(
                    items
                        .iter()
                        .map(|item| {
                            let mapped = self.element_reference(*at, item);
                            // A member the source filled and this carry could not: the element
                            // it named is not in the hierarchy the output holds, so the slot is
                            // §7.3.9's null and an assistive reader finds nothing at that
                            // marked-content identifier. Counted so that it is said out loud.
                            if item.as_reference().is_some() && mapped == Object::Null {
                                orphaned = orphaned.saturating_add(1);
                            }
                            mapped
                        })
                        .collect(),
                ),
                _ => Object::Array(vec![self.element_reference(*at, &value)]),
            };
            nums.push((*new, mapped));
        }
        for (at, old, new) in &self.object_keys {
            let Some(value) = host
                .source(*at)
                .and_then(|document| Self::source_entry(document, *old))
            else {
                continue;
            };
            nums.push((*new, self.element_reference(*at, &value)));
        }
        if nums.is_empty() {
            return None;
        }
        nums.sort_by_key(|(key, _)| *key);
        let mut array: Vec<Object> = Vec::new();
        for (key, value) in nums {
            array.push(Object::Integer(key));
            array.push(value);
        }
        let mut node = Dictionary::new();
        node.insert(Name::new(&b"Nums"[..]), Object::Array(array));
        self.orphaned_items = self.orphaned_items.saturating_add(orphaned);
        Some(Object::Dictionary(node))
    }

    /// One source's parent-tree value for a key, left as the tree states it.
    ///
    /// Unresolved, because §14.7.5.4's object-key value *is* a reference — "the value shall be an
    /// indirect reference to the parent structure element" — and resolving one throws away the
    /// identity this module maps. A page key's value is an array, and [`Self::parent_tree`]
    /// resolves that one level itself, for the reason stated there.
    pub(super) fn source_entry(document: &Document, key: i64) -> Option<Object> {
        let root = structure_root(Some(document))?;
        let Object::Dictionary(tree) = root
            .get("ParentTree")
            .map(|value| document.resolve(value))?
        else {
            return None;
        };
        tree::lookup_unresolved(&tree, &tree::TreeKey::Number(key), &|value| {
            document.resolve(value)
        })
    }

    /// One parent-tree entry's element: the kept one it names, or §7.3.10's null.
    fn element_reference(&self, at: usize, value: &Object) -> Object {
        value
            .as_reference()
            .and_then(|id| self.placed.get(&(at, id)).copied())
            .map_or(Object::Null, Object::Reference)
    }
}
