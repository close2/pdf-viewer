//! §7.8.3's named resources: the lookups every operator family shares.
//!
//! Three shapes of the same question — resolved, unresolved (identity matters for §8.11's
//! optional content groups), and by way of a memoised category table — plus the report a
//! name nobody defined earns.
//!
//! # A resource name is bytes, and that decides the signature
//!
//! Every lookup here takes a [`Name`] rather than a `&str`, because §7.3.5 states both what a
//! name is and what makes two of them the same one:
//!
//! > Beginning with PDF 1.2 a name object is an atomic symbol uniquely defined by a sequence of
//! > any characters (8-bit values) except null (character code 0). Uniquely defined means that
//! > any two name objects that, after all escaping is expanded (see below), and the resulting
//! > sequences of bytes are not an exact binary match denote different objects.
//!
//! and the same clause says what the bytes are not: "Ordinarily, the bytes making up the name are
//! never treated as text to be presented to a human user or to an application external to a PDF
//! processor." A producer that writes `/Contr#F4le` states a name whose sixth byte is 0xF4, which
//! is not valid UTF-8 on its own. Converting such a name to a `String` before the lookup — which
//! this module's callers did until the six-hundred-and-third session — replaces that byte with
//! U+FFFD, so the probe's bytes are not the key's bytes and a resource the file *does* define
//! cannot be found. ADR 0438: the witness draws a full-page scan in `poppler`, `mupdf` and
//! `ghostscript` and drew a blank sheet here, reported as a resource nobody defined.
//!
//! Text is still what a *report* wants, and that conversion belongs at the report rather than at
//! the lookup — [`Interpreter::note_missing_resource`] is where it happens, once.

use pdf_syntax::{Dictionary, Name, Object};

use super::Interpreter;
use super::report::Unsupported;

impl Interpreter<'_> {
    /// Reports a name §7.8.3's current resource dictionary does not define, if it costs a mark.
    ///
    /// **The condition is where the honesty is** (trap 11). Content inside a marked-content
    /// section this configuration hides marks nothing whatever the name resolves to, so a
    /// report there would cost the oracle a judged page for a difference no raster can hold —
    /// and `paint_shading` already skips a hidden `sh` "including the report a shading we
    /// cannot build would otherwise make". §8.11.3.1 is the clause: an invisible object "shall
    /// be skipped, as if there were no `Do` operator to invoke it", and a `Do` that was never
    /// invoked cannot have failed.
    ///
    /// Two neighbouring cases reach nothing here and are worth naming, because a report that
    /// fires where nothing was lost is worse than the silence it replaces. An `XObject` a
    /// resource dictionary *defines* and no content stream draws never reaches `Do` at all; and
    /// a name a form uses that only the page defines is not this — §7.8.3 hands such a form the
    /// page's dictionary whole, so the lookup that happens is the one the clause describes and
    /// its failure is a failure of both dictionaries at once.
    pub(super) fn note_missing_resource(
        &mut self,
        category: &'static str,
        name: &Name,
        issue: &str,
    ) {
        if self.is_hidden() {
            return;
        }
        self.note(Unsupported::MissingResource {
            category,
            detail: format!("/{} {issue}", String::from_utf8_lossy(name.as_bytes())),
        });
    }

    /// Looks up a named resource of a given category.
    pub(super) fn resource(
        &self,
        resources: &Dictionary,
        category: &str,
        name: &Name,
    ) -> Option<Object> {
        Some(
            self.document
                .resolve(&self.resource_entry(resources, category, name)?),
        )
    }

    /// The same lookup, *unresolved*: the reference a resource dictionary states.
    ///
    /// Almost every caller wants the object; a cache keyed by identity wants the name of it,
    /// and resolving first throws that away. `shading::Cache` is the one caller — a page may
    /// paint one shading object thousands of times, and only the reference says they are the
    /// same one.
    pub(super) fn resource_entry(
        &self,
        resources: &Dictionary,
        category: &str,
        name: &Name,
    ) -> Option<Object> {
        match resources.get(category)? {
            // Already in hand: read the entry out of the table rather than copying the table
            // to do it. This is the common shape and it costs nothing.
            Object::Dictionary(table) => table.get_by_name(name).cloned(),
            // The expensive shape, and [`Interpreter::resource_tables`] is why: a reference is
            // resolved by *copying* the object out of the document's cache, so a page with one
            // entry per operator pays the whole table per operator.
            Object::Reference(id) => {
                let id = *id;
                if let Some(table) = self.resource_tables.borrow().get(&id) {
                    return table.get_by_name(name).cloned();
                }
                let resolved = self.document.get(id);
                let Some(table) = resolved.as_dict() else {
                    // A reference to a reference, or to something that is not a dictionary at
                    // all: the general path answers both and neither is worth remembering.
                    return self
                        .document
                        .resolve(&resolved)
                        .as_dict()?
                        .get_by_name(name)
                        .cloned();
                };
                let entry = table.get_by_name(name).cloned();
                self.resource_tables.borrow_mut().insert(id, table.clone());
                entry
            }
            other => self
                .document
                .resolve(other)
                .as_dict()?
                .get_by_name(name)
                .cloned(),
        }
    }

    /// A resource entry exactly as the file writes it, reference and all.
    ///
    /// Optional content is the one place where a resource's *identity* matters rather than
    /// its value. §8.11.2.2 requires an optional content group to be an indirect object, and
    /// `/OCProperties /OCGs` lists the document's groups by reference, so a group is
    /// recognised by which object it is. Resolving it first throws that away and leaves two
    /// identical-looking dictionaries indistinguishable.
    pub(super) fn unresolved_resource(
        &self,
        resources: &Dictionary,
        category: &str,
        name: &Name,
    ) -> Option<Object> {
        let table = self.document.get_key(resources, category);
        Some(table.as_dict()?.get_by_name(name)?.clone())
    }
}
