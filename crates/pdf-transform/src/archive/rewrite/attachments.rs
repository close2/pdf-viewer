//! Embedded files: the file specification a requirement completes, the names it states and the
//! attachments filed under the name dictionary.
use std::collections::BTreeMap;

use pdf_syntax::object::{Dictionary, Name, Object, ObjectId};

use super::{Rewrite, Rewriter, count};

impl Rewriter<'_> {
    /// An embedded file's specification: §7.11.3's `/F` and `/UF`, and its `/AFRelationship`.
    ///
    /// **The population is the `/EF` key**, which is what makes a file specification one that
    /// *carries* an embedded file — the population both ISO 19005-2 section 6.8 and ISO 19005-4
    /// section 6.9 state their rules over. A specification the document holds as a direct
    /// dictionary inside its `/EmbeddedFiles` name tree is not reached, because this walk
    /// rewrites objects rather than values inside them; the requirement then stays failed and
    /// `doc/adr/0947`'s third stage refuses the file rather than writing a half-corrected one.
    pub(super) fn complete_file_specification(
        &self,
        out: &mut Dictionary,
        applied: &mut BTreeMap<Rewrite, usize>,
    ) -> bool {
        if out.get("EF").is_none() {
            return false;
        }
        let mut changed = false;
        if self.wants(Rewrite::AssociatedFileNames)
            && let Some(pair) = self.file_names(out)
        {
            out.insert(Name::new(&b"F"[..]), Object::String(pair.clone().into()));
            out.insert(Name::new(&b"UF"[..]), Object::String(pair.into()));
            count(applied, Rewrite::AssociatedFileNames);
            changed = true;
        }
        if self.wants(Rewrite::AssociatedFileRelationship)
            && self.document.get_key(out, "AFRelationship").is_null()
        {
            out.insert(
                Name::new(&b"AFRelationship"[..]),
                Object::Name(Name::new(&b"Unspecified"[..])),
            );
            count(applied, Rewrite::AssociatedFileRelationship);
            changed = true;
        }
        changed
    }

    /// The one ASCII file name a specification missing `/F` or `/UF` states in the other key.
    ///
    /// `None` where both keys are already there, where neither is, or where the one that is
    /// holds a byte outside ASCII — the three cases [`Rewrite::AssociatedFileNames`] does not
    /// write, and the last of them is the one that matters: outside ASCII the byte string's own
    /// character set is not something this file states.
    fn file_names(&self, out: &Dictionary) -> Option<Vec<u8>> {
        let stated = |key: &str| {
            self.document
                .get_key(out, key)
                .as_string()
                .map(<[u8]>::to_vec)
        };
        let (present, absent) = (stated("F"), stated("UF"));
        if present.is_some() == absent.is_some() {
            return None;
        }
        let name = present.or(absent)?;
        name.is_ascii().then_some(name)
    }

    /// Points the name dictionary's `/EmbeddedFiles` at the tree an attachment rewrote.
    ///
    /// §7.7.4 puts the tree in the name dictionary, and where this document states that
    /// dictionary as an object of its own the entry is rewritten *there* rather than in the
    /// catalog — which leaves every other name tree the producer wrote exactly where it was.
    /// [`Self::file_the_attachments`] is the other shape.
    pub(super) fn file_in_the_name_dictionary(&self, id: ObjectId, out: &mut Dictionary) -> bool {
        if Some(id) != self.sites.names {
            return false;
        }
        let Some(attached) = self.attachments() else {
            return false;
        };
        out.insert(
            Name::new(&b"EmbeddedFiles"[..]),
            Object::Reference(attached.tree),
        );
        true
    }

    /// The attachments this conversion filed, where it filed any.
    ///
    /// Two rewrites file into the tree: the `preserve` that attaches the producer's bytes, and the
    /// removal of a `FileAttachment` annotation whose file stays (`doc/adr/2067` section 4).
    fn attachments(&self) -> Option<&super::super::attach::Attached> {
        self.attached.filter(|attached| {
            self.wants(Rewrite::PreservedAsAttachment)
                || (self.wants(Rewrite::ForbiddenAnnotationRemoved) && !attached.kept.is_empty())
        })
    }

    /// Files the attachments in the catalog: §7.7.4's name tree and §14.13.3's `/AF`.
    ///
    /// Two entries, and each is the clause's own. §7.7.4's Table 32 makes `/EmbeddedFiles` "[a]
    /// name tree mapping name strings to file specifications for embedded file streams", which is
    /// the tree [`super::super::attach`] wrote; where the document states the name dictionary as an
    /// object of its own the entry goes there instead, and [`Self::rewrite_dictionary`] is where.
    /// §14.13.3 is the second, and the catalog is where it puts a file associated with the
    /// document as a whole:
    ///
    /// > One or more files may be associated with the PDF document as a whole by including a file
    /// > specification dictionary (7.11.3, "File specification dictionaries") for each file as one
    /// > of the members of the array value of the AF key in the document catalog (7.7.2, "Document
    /// > catalog dictionary").
    ///
    /// **The producer's own array is added to rather than replaced**: a document already
    /// associating files with itself goes on associating them, and `doc/adr/0947`'s rule is that
    /// nothing changes which no failed requirement asked to change.
    pub(super) fn file_the_attachments(
        &self,
        catalog: &mut Dictionary,
        applied: &mut BTreeMap<Rewrite, usize>,
    ) -> bool {
        let Some(attached) = self.attachments() else {
            return false;
        };
        if self.sites.names.is_none() {
            let mut names = self
                .document
                .get_key(catalog, "Names")
                .as_dict()
                .cloned()
                .unwrap_or_default();
            names.insert(
                Name::new(&b"EmbeddedFiles"[..]),
                Object::Reference(attached.tree),
            );
            catalog.insert(Name::new(&b"Names"[..]), Object::Dictionary(names));
        }
        // A file kept from a removed annotation is associated with nothing new, so a tree holding
        // only those leaves the catalog's `/AF` as the producer wrote it.
        if attached.associated.is_empty() {
            return true;
        }
        let mut associated = self
            .document
            .get_key(catalog, "AF")
            .as_array()
            .map(<[Object]>::to_vec)
            .unwrap_or_default();
        for specification in &attached.associated {
            associated.push(Object::Reference(*specification));
            count(applied, Rewrite::PreservedAsAttachment);
        }
        catalog.insert(Name::new(&b"AF"[..]), Object::Array(associated));
        true
    }

    /// ISO 19005-2 section 6.8 and ISO 19005-4 section 6.9, at whichever entry names the file.
    ///
    /// `at` is the object being rewritten, or `None` for the catalog's own dictionary: the
    /// catalog may state its `/AF` array directly, and a removal that only edited objects would
    /// leave that one standing.
    pub(super) fn forget_embedded_files(
        &self,
        at: Option<ObjectId>,
        out: &mut Dictionary,
        applied: &mut BTreeMap<Rewrite, usize>,
    ) -> bool {
        if !self.wants(Rewrite::EmbeddedFileRemoved) {
            return false;
        }
        let Some(removed) = self.embedded_files else {
            return false;
        };
        let mut changed = false;
        let holds_a_name = match at {
            Some(id) => removed.named_by.contains(&id),
            None => removed.named_in_catalog,
        };
        if holds_a_name {
            changed |= Self::prune_name_tree(out, &removed.at, 0, applied);
        }
        let associated = match at {
            Some(id) => removed.associated_by.contains(&id),
            None => removed.associated_in_catalog,
        };
        if associated && let Some(files) = self.document.get_key(out, "AF").as_array() {
            let kept: Vec<Object> = files
                .iter()
                .filter(|entry| {
                    entry
                        .as_reference()
                        .is_none_or(|id| !removed.at.contains(&id))
                })
                .cloned()
                .collect();
            // §14.13.2 makes `/AF` an array of associated files; one with nothing left in it
            // says the object has none, which is what an absent entry says, so the entry goes
            // rather than staying as an empty array.
            if kept.is_empty() {
                out.remove("AF");
            } else {
                out.insert(Name::new(&b"AF"[..]), Object::Array(kept));
            }
            count(applied, Rewrite::EmbeddedFileRemoved);
            changed = true;
        }
        changed
    }
}
