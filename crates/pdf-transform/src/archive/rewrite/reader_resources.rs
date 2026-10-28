//! The resources the reader used, and the empty resources for names it found nothing for, written
//! into the objects that hold them (`super::super::reader_resources` decides which, `doc/adr/2500`).
use std::collections::BTreeMap;

use pdf_syntax::Document;
use pdf_syntax::object::{Dictionary, Name, Object, ObjectId, Stream};

use super::super::reader_resources::{Addition, Edit, FormCopy, Place};
use super::{Rewrite, Rewriter, Rewritten, count};

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
            match edit {
                Edit::Assign(value) if out.get("Resources").is_none() => {
                    out.insert(Name::new(&b"Resources"[..]), value.clone());
                    count(applied, Rewrite::PageResources);
                    changed = true;
                }
                Edit::Retarget { name, from, to } if retarget(out, name, *from, *to) => {
                    count(applied, Rewrite::PageResources);
                    changed = true;
                }
                Edit::Assign(_) | Edit::Retarget { .. } | Edit::Define(..) => {}
            }
        }
        changed
    }

    /// A form written again for the pages whose resources it states: the form's own rewrites,
    /// then the resources of those pages in place of the ones it was assigned.
    ///
    /// The copy is built from the source form, not from a rewritten value, because the rewrites
    /// are decided per object and this object has no number of its own in the source.
    pub(super) fn form_copy(
        &self,
        copy: &FormCopy,
        applied: &mut BTreeMap<Rewrite, usize>,
    ) -> Object {
        let source = self.document.get(copy.of);
        let rewritten = match self.rewrite(copy.of, &source, applied) {
            Rewritten::Changed(changed) => changed,
            Rewritten::Carried => source,
        };
        let Object::Stream(stream) = rewritten else {
            return rewritten;
        };
        let mut dict = stream.dict.clone();
        dict.insert(
            Name::new(&b"Resources"[..]),
            Object::Reference(copy.resources),
        );
        Object::Stream(std::sync::Arc::new(Stream {
            dict,
            data: std::sync::Arc::clone(&stream.data),
            decryption_failed: stream.decryption_failed,
        }))
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
        if let Some(copy) = self.form_resources.and_then(|found| found.copies.get(&id)) {
            return self.form_copy(copy, applied);
        }
        let Object::Dictionary(mut dict) = added else {
            return added;
        };
        self.define_empty_resources(id, &mut dict, applied);
        Object::Dictionary(dict)
    }

    /// A name no dictionary defined is defined as an empty resource of its category.
    ///
    /// **Never over an entry that is there**, so a shared dictionary reached by two edits is
    /// written once. An entry whose value resolves to null is not there: ISO 32000-2 §7.3.9 makes
    /// it equivalent to omitting the entry, and a reference to an object the file does not define
    /// the same as null, so the reader drew nothing for the name and the survey reports it
    /// undefined (`doc/adr/2550`).
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
                Edit::Define(place, addition) if define(self.document, place, addition, out) => {
                    count(applied, Rewrite::EmptyResourceDefined);
                    changed = true;
                }
                Edit::Assign(_) | Edit::Define(..) | Edit::Retarget { .. } => {}
            }
        }
        changed
    }
}

/// Makes the entries of the object's own `XObject` table that are named `name` and name `from`
/// name `to`, and says whether one was.
fn retarget(resources: &mut Dictionary, name: &[u8], from: ObjectId, to: ObjectId) -> bool {
    let Some(Object::Dictionary(table)) = resources.get("XObject") else {
        return false;
    };
    let name = Name::new(name.to_vec());
    if table.get_by_name(&name).and_then(Object::as_reference) != Some(from) {
        return false;
    }
    let mut table = table.clone();
    table.insert(name, Object::Reference(to));
    resources.insert(Name::new(&b"XObject"[..]), Object::Dictionary(table));
    true
}

/// Defines one name in the place the plan chose, and says whether anything was written.
///
/// The tables are taken out of their holder and put back rather than copied, because a stream that
/// names tens of thousands of missing resources reaches this once for each, and a copy of the
/// table per name would make the rewrite quadratic.
fn define(document: &Document, place: &Place, addition: &Addition, out: &mut Dictionary) -> bool {
    match place {
        Place::OwnResources => match out.remove("Resources") {
            Some(Object::Dictionary(mut resources)) => {
                let written = define_in(document, &mut resources, addition);
                out.insert(Name::new(&b"Resources"[..]), Object::Dictionary(resources));
                written
            }
            Some(other) => {
                out.insert(Name::new(&b"Resources"[..]), other);
                false
            }
            None => false,
        },
        Place::Itself => define_in(document, out, addition),
        Place::Table => insert_name(document, out, addition),
    }
}

/// Defines the name in the category table of a resources dictionary.
///
/// A table that is not a direct dictionary was planned as a [`Place::Table`] edit on its own
/// object, so any other value here is left as it is.
fn define_in(document: &Document, resources: &mut Dictionary, addition: &Addition) -> bool {
    let key = Name::new(addition.category.as_bytes());
    match resources.remove(addition.category) {
        Some(Object::Dictionary(mut table)) => {
            let written = insert_name(document, &mut table, addition);
            resources.insert(key, Object::Dictionary(table));
            written
        }
        None => {
            let mut table = Dictionary::new();
            insert_name(document, &mut table, addition);
            resources.insert(key, Object::Dictionary(table));
            true
        }
        Some(other) => {
            resources.insert(key, other);
            false
        }
    }
}

/// Adds the name to a table that does not hold it, or holds it with a value that resolves to null.
///
/// The value this conversion writes is a reference to an object of its own, which the source does
/// not define either, so an entry already holding it is the one a second edit of a shared table
/// finds, and is left as it is.
fn insert_name(document: &Document, table: &mut Dictionary, addition: &Addition) -> bool {
    let name = Name::new(addition.name.clone());
    if let Some(entry) = table.get_by_name(&name)
        && (*entry == addition.value || !document.resolve(entry).is_null())
    {
        return false;
    }
    table.insert(name, addition.value.clone());
    true
}
