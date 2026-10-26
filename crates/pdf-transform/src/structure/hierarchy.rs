//! The walk that decides which structure elements reach a carried page, and the slots it gives
//! them (ISO 32000-2 §14.7.2).

use std::collections::{BTreeMap, BTreeSet};

use pdf_syntax::object::{Dictionary, Object, ObjectId};

use super::{Carry, Child, Element, Host, MAX_DEPTH, Outcome, children_of};
use crate::Refusal;

impl Carry {
    /// Walks one source's hierarchy from the structure tree root, keeping what reaches a page.
    ///
    /// # Errors
    ///
    /// [`Refusal::Assembly`] where the numbering is spent.
    pub(super) fn decide(
        &mut self,
        host: &mut dyn Host,
        at: usize,
        root: &Dictionary,
        carried: &BTreeMap<ObjectId, ObjectId>,
    ) -> Result<(), Refusal> {
        let children = {
            let Some(document) = host.source(at) else {
                return Ok(());
            };
            children_of(document, root)
        };
        let mut seen = BTreeSet::new();
        for child in &children {
            let Some(id) = child.as_reference() else {
                // Table 355 makes `/P` "( Required; shall be an indirect reference )", so an
                // element written directly into its parent cannot be named by its own children,
                // and a hierarchy this module rebuilds by reference cannot hold it.
                self.dropped = self.dropped.saturating_add(1);
                continue;
            };
            if self.keep(host, at, id, None, carried, &mut seen, 0)? {
                self.tops.push((at, id));
            }
        }
        Ok(())
    }

    /// One element: kept where it reaches a carried page, dropped otherwise.
    ///
    /// Depth-first and bottom-up, because an element is kept for its descendants as much as for
    /// its own content: §14.7.2 makes `/K` "\[t]he children of this structure element", and an
    /// ancestor of a kept child is what holds it in the hierarchy.
    ///
    /// # Errors
    ///
    /// [`Refusal::Assembly`] where the numbering is spent.
    #[expect(
        clippy::too_many_arguments,
        reason = "the walk's state, threaded rather than held in a struct that would outlive \
                  the one element it is about"
    )]
    fn keep(
        &mut self,
        host: &mut dyn Host,
        at: usize,
        id: ObjectId,
        parent: Option<ObjectId>,
        carried: &BTreeMap<ObjectId, ObjectId>,
        seen: &mut BTreeSet<ObjectId>,
        depth: usize,
    ) -> Result<bool, Refusal> {
        if depth >= MAX_DEPTH || !seen.insert(id) {
            return Ok(false);
        }
        let (children, own_page, stated_page, identifier) = {
            let Some(document) = host.source(at) else {
                return Ok(false);
            };
            let Object::Dictionary(element) = document.get(id) else {
                return Ok(false);
            };
            // Table 355's `/Pg`: "( Optional; required if K is an integer object or an array
            // containing integer objects; shall be an indirect reference ) A page object
            // representing a page on which some or all of the content items designated by the K
            // entry shall be rendered."
            let stated = element.get("Pg").and_then(Object::as_reference);
            let own = stated.filter(|page| carried.contains_key(page));
            let children = children_of(document, &element);
            let identifier = match element.get("ID") {
                Some(Object::String(bytes)) => Some(Object::String(bytes.clone())),
                _ => None,
            };
            (children, own, stated, identifier)
        };

        let mut kept: Vec<Child> = Vec::new();
        for child in &children {
            match self.child(
                host,
                at,
                child,
                id,
                own_page,
                stated_page,
                carried,
                seen,
                depth,
            )? {
                Outcome::Kept(child) => kept.push(child),
                Outcome::DroppedItem => {
                    self.dropped_items = self.dropped_items.saturating_add(1);
                }
                Outcome::Unplaceable => {
                    self.unplaceable = self.unplaceable.saturating_add(1);
                }
                Outcome::Dropped => {}
            }
        }

        if kept.is_empty() {
            // The element reaches no page the output holds. It is *blocked* rather than merely
            // skipped, so that a `/Ref` or an attribute naming it cannot drag its subtree in.
            host.block_object(at, id);
            self.dropped = self.dropped.saturating_add(1);
            return Ok(false);
        }
        // Table 355's `/ID`: "The string shall be unique among all elements in the document's
        // structure hierarchy." Recorded here, checked once the whole hierarchy is decided, so
        // that a collision names both sources rather than whichever was walked second.
        if let Some(Object::String(bytes)) = identifier {
            self.ids.entry(bytes.to_vec()).or_insert((at, id));
        }
        self.elements.push(Element {
            at,
            source: id,
            parent,
            children: kept,
            page: own_page.and_then(|page| carried.get(&page).copied()),
        });
        Ok(true)
    }

    /// What one child of an element turned out to be.
    #[expect(
        clippy::too_many_arguments,
        reason = "the walk's state, threaded rather than held in a struct that would outlive \
                  the one element it is about"
    )]
    fn child(
        &mut self,
        host: &mut dyn Host,
        at: usize,
        child: &Object,
        parent: ObjectId,
        own_page: Option<ObjectId>,
        stated_page: Option<ObjectId>,
        carried: &BTreeMap<ObjectId, ObjectId>,
        seen: &mut BTreeSet<ObjectId>,
        depth: usize,
    ) -> Result<Outcome, Refusal> {
        let resolved = {
            let Some(document) = host.source(at) else {
                return Ok(Outcome::Dropped);
            };
            document.resolve(child)
        };
        match &resolved {
            // §14.7.5.2's second form: "[a]n integer that specifies the marked-content
            // identifier", read against Table 355's `/Pg` on this element.
            Object::Integer(_) => Ok(match (own_page, stated_page) {
                (Some(_), _) => Outcome::Kept(Child::Item(resolved.clone())),
                (None, Some(_)) => Outcome::DroppedItem,
                (None, None) => Outcome::Unplaceable,
            }),
            Object::Dictionary(dict) => {
                let kind = dict
                    .get("Type")
                    .and_then(Object::as_name)
                    .map(|name| name.as_bytes().to_vec());
                // Table 357's marked-content reference and Table 358's object reference are the
                // two content items that are dictionaries. Each may state a `/Pg` of its own,
                // which "overrides any Pg entry in the structure element containing" it. Anything
                // else is Table 355's own default: "[i]f the value of K is a dictionary
                // containing no Type entry, it shall be assumed to be a structure element
                // dictionary."
                if let Some(b"MCR" | b"OBJR") = kind.as_deref() {
                    {
                        let here = dict.get("Pg").and_then(Object::as_reference);
                        let Some(page) = here.or(stated_page) else {
                            return Ok(Outcome::Unplaceable);
                        };
                        let Some(placed) = carried.get(&page).copied() else {
                            return Ok(Outcome::DroppedItem);
                        };
                        Ok(Outcome::Kept(Child::Reference {
                            dictionary: dict.clone(),
                            page: placed,
                        }))
                    }
                } else {
                    {
                        let Some(id) = child.as_reference() else {
                            self.dropped = self.dropped.saturating_add(1);
                            return Ok(Outcome::Dropped);
                        };
                        if self.keep(
                            host,
                            at,
                            id,
                            Some(parent),
                            carried,
                            seen,
                            depth.saturating_add(1),
                        )? {
                            Ok(Outcome::Kept(Child::Element(id)))
                        } else {
                            Ok(Outcome::Dropped)
                        }
                    }
                }
            }
            _ => Ok(Outcome::Dropped),
        }
    }

    /// Gives every kept element the slot that stands in for it.
    ///
    /// # Errors
    ///
    /// [`Refusal::Assembly`] where the numbering is spent.
    pub(super) fn number(&mut self, host: &mut dyn Host) -> Result<(), Refusal> {
        for element in &self.elements {
            let placed = host
                .replace_object(element.at, element.source)
                .map_err(|why| {
                    Refusal::Assembly(format!(
                        "§14.7's structure element {}: {why}",
                        element.source.number
                    ))
                })?;
            self.placed.insert((element.at, element.source), placed);
        }
        Ok(())
    }
}
