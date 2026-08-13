//! §7.8.3's named resources: the lookups every operator family shares.
//!
//! Three shapes of the same question — resolved, unresolved (identity matters for §8.11's
//! optional content groups), and by way of a memoised category table — plus the report a
//! name nobody defined earns.

use pdf_syntax::{Dictionary, Object};

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
        name: &str,
        issue: &str,
    ) {
        if self.is_hidden() {
            return;
        }
        self.note(Unsupported::MissingResource {
            category,
            detail: format!("/{name} {issue}"),
        });
    }

    /// Looks up a named resource of a given category.
    pub(super) fn resource(
        &self,
        resources: &Dictionary,
        category: &str,
        name: &str,
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
        name: &str,
    ) -> Option<Object> {
        match resources.get(category)? {
            // Already in hand: read the entry out of the table rather than copying the table
            // to do it. This is the common shape and it costs nothing.
            Object::Dictionary(table) => table.get(name).cloned(),
            // The expensive shape, and [`Interpreter::resource_tables`] is why: a reference is
            // resolved by *copying* the object out of the document's cache, so a page with one
            // entry per operator pays the whole table per operator.
            Object::Reference(id) => {
                let id = *id;
                if let Some(table) = self.resource_tables.borrow().get(&id) {
                    return table.get(name).cloned();
                }
                let resolved = self.document.get(id);
                let Some(table) = resolved.as_dict() else {
                    // A reference to a reference, or to something that is not a dictionary at
                    // all: the general path answers both and neither is worth remembering.
                    return self
                        .document
                        .resolve(&resolved)
                        .as_dict()?
                        .get(name)
                        .cloned();
                };
                let entry = table.get(name).cloned();
                self.resource_tables.borrow_mut().insert(id, table.clone());
                entry
            }
            other => self.document.resolve(other).as_dict()?.get(name).cloned(),
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
        name: &str,
    ) -> Option<Object> {
        let table = self.document.get_key(resources, category);
        Some(table.as_dict()?.get(name)?.clone())
    }
}
