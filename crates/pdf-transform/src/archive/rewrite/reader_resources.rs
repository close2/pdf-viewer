//! The resources the reader used, and the empty resources for names it found nothing for, written
//! into the objects that hold them (`super::super::reader_resources` decides which, `doc/adr/2500`).
use std::collections::BTreeMap;

use pdf_syntax::object::{Dictionary, Name, Object, ObjectId};

use super::super::reader_resources::{Addition, Edit, Place};
use super::{Rewrite, Rewriter, count};

impl Rewriter<'_> {
    /// A form, pattern or appearance that states no `/Resources` is given the dictionary the
    /// reader read it against.
    ///
    /// ISO 19005-2 section 6.2.2 and ISO 19005-4 section 6.2.2 ask for an explicit association;
    /// ISO 32000-2 §7.8.3 ends the reader's search at the page's resources, so the value written
    /// is the one every name already resolved through. **Never over an entry that is there.**
    pub(super) fn give_reader_resources(
        &self,
        id: ObjectId,
        out: &mut Dictionary,
        applied: &mut BTreeMap<Rewrite, usize>,
    ) -> bool {
        if !self.wants(Rewrite::PageResources) {
            return false;
        }
        let Some(edits) = self.form_resources.and_then(|found| found.at.get(&id)) else {
            return false;
        };
        let mut changed = false;
        for edit in edits {
            if let Edit::Assign(value) = edit
                && out.get("Resources").is_none()
            {
                out.insert(Name::new(&b"Resources"[..]), value.clone());
                count(applied, Rewrite::PageResources);
                changed = true;
            }
        }
        changed
    }

    /// An object this conversion added is the dictionary `assign` wrote for a page's direct
    /// resources, and the names the forms that share it used and nothing defined are defined in it.
    ///
    /// An added object does not pass through [`Rewriter::rewrite`], so the definitions planned
    /// against it are made here.
    pub(super) fn define_in_added(
        &self,
        id: ObjectId,
        added: Object,
        applied: &mut BTreeMap<Rewrite, usize>,
    ) -> Object {
        let Object::Dictionary(mut dict) = added else {
            return added;
        };
        self.define_empty_resources(id, &mut dict, applied);
        Object::Dictionary(dict)
    }

    /// A name no dictionary defined is defined as an empty resource of its category.
    ///
    /// **Never over an entry that is there**, so a shared dictionary reached by two edits is
    /// written once.
    pub(super) fn define_empty_resources(
        &self,
        id: ObjectId,
        out: &mut Dictionary,
        applied: &mut BTreeMap<Rewrite, usize>,
    ) -> bool {
        if !self.wants(Rewrite::EmptyResourceDefined) {
            return false;
        }
        let Some(edits) = self.defined_resources.and_then(|found| found.at.get(&id)) else {
            return false;
        };
        let mut changed = false;
        for edit in edits {
            match edit {
                // A page that inherits a dictionary written in an ancestor is given its own copy
                // first, so that the name is defined in the page's and in no other page's.
                Edit::Assign(value) if out.get("Resources").is_none() => {
                    out.insert(Name::new(&b"Resources"[..]), value.clone());
                    changed = true;
                }
                Edit::Define(place, addition) if define(place, addition, out) => {
                    count(applied, Rewrite::EmptyResourceDefined);
                    changed = true;
                }
                Edit::Assign(_) | Edit::Define(..) => {}
            }
        }
        changed
    }
}

/// Defines one name in the place the plan chose, and says whether anything was written.
///
/// The tables are taken out of their holder and put back rather than copied, because a stream that
/// names tens of thousands of missing resources reaches this once for each, and a copy of the
/// table per name would make the rewrite quadratic.
fn define(place: &Place, addition: &Addition, out: &mut Dictionary) -> bool {
    match place {
        Place::OwnResources => match out.remove("Resources") {
            Some(Object::Dictionary(mut resources)) => {
                let written = define_in(&mut resources, addition);
                out.insert(Name::new(&b"Resources"[..]), Object::Dictionary(resources));
                written
            }
            Some(other) => {
                out.insert(Name::new(&b"Resources"[..]), other);
                false
            }
            None => false,
        },
        Place::Itself => define_in(out, addition),
        Place::Table => insert_name(out, addition),
    }
}

/// Defines the name in the category table of a resources dictionary.
///
/// A table that is not a direct dictionary was planned as a [`Place::Table`] edit on its own
/// object, so any other value here is left as it is.
fn define_in(resources: &mut Dictionary, addition: &Addition) -> bool {
    let key = Name::new(addition.category.as_bytes());
    match resources.remove(addition.category) {
        Some(Object::Dictionary(mut table)) => {
            let written = insert_name(&mut table, addition);
            resources.insert(key, Object::Dictionary(table));
            written
        }
        None => {
            let mut table = Dictionary::new();
            insert_name(&mut table, addition);
            resources.insert(key, Object::Dictionary(table));
            true
        }
        Some(other) => {
            resources.insert(key, other);
            false
        }
    }
}

/// Adds the name to a table that does not hold it.
fn insert_name(table: &mut Dictionary, addition: &Addition) -> bool {
    let name = Name::new(addition.name.clone());
    if table.get_by_name(&name).is_some() {
        return false;
    }
    table.insert(name, addition.value.clone());
    true
}
