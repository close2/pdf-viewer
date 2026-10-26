//! The one `Document` Annex L's Table L.2 puts under the structure tree root, planned where two
//! sources each hold a PDF 2.0 `Document`.

use std::collections::BTreeSet;

use pdf_model::structure::{STANDARD_NAMESPACE_2_0, StandardType, Tree};
use pdf_syntax::object::{Dictionary, Name, Object, ObjectId};

use super::{Carry, Host, Wrapper, structure_root};
use crate::Refusal;

impl Carry {
    /// Plans the one `Document` Annex L requires, where the sources' top-level elements include
    /// PDF 2.0 `Document`s from more than one source.
    ///
    /// The population is Annex L's: an element whose type, after §14.8.6.2's role mapping, is
    /// `Document` in the standard structure namespace for PDF 2.0 —
    /// [`Tree::in_pdf_2_0_namespace`] — including one role mapped into it, which the annex binds
    /// in as many words.
    ///
    /// # Errors
    ///
    /// [`Refusal::Assembly`] where the numbering is spent.
    pub(super) fn wrap_documents(&mut self, host: &mut dyn Host) -> Result<(), Refusal> {
        let mut documents: Vec<(usize, ObjectId)> = Vec::new();
        for &(at, id) in &self.tops {
            let Some(document) = host.source(at) else {
                continue;
            };
            let Some(tree) = Tree::of(document) else {
                continue;
            };
            let Object::Dictionary(element) = document.get(id) else {
                continue;
            };
            if tree.standard_role(document, &element) == Some(StandardType::Document)
                && tree.in_pdf_2_0_namespace(document, &element)
            {
                documents.push((at, id));
            }
        }
        let sources: BTreeSet<usize> = documents.iter().map(|(at, _)| *at).collect();
        if sources.len() < 2 {
            return Ok(());
        }
        let namespace = self.pdf_2_0_namespace(host, &sources)?;
        let id = host
            .reserve_slot()
            .map_err(|why| Refusal::Assembly(why.to_string()))?;
        self.wrapper = Some(Wrapper {
            id,
            namespace,
            children: documents,
        });
        Ok(())
    }

    /// A reference to a PDF 2.0 namespace dictionary the output's root lists.
    ///
    /// Table 355 makes an element's `/NS` "\[a]n indirect reference to a namespace dictionary", and
    /// §14.8.6.2 that an element in an explicit namespace has "that namespace ... identified in
    /// the structure tree root dictionary's Namespaces array entry". A source that holds a PDF 2.0
    /// element in conformance with that already lists one, carried into the output's array by
    /// [`Carry::read_root`], so the first such entry is reused; only where no source lists one —
    /// every element reaching the namespace by a role map whose target dictionary the root does
    /// not list — is a dictionary written and added to the array.
    ///
    /// # Errors
    ///
    /// [`Refusal::Assembly`] where the numbering is spent.
    fn pdf_2_0_namespace(
        &mut self,
        host: &mut dyn Host,
        sources: &BTreeSet<usize>,
    ) -> Result<Object, Refusal> {
        for &at in sources {
            let listed = {
                let Some(document) = host.source(at) else {
                    continue;
                };
                let Some(root) = structure_root(Some(document)) else {
                    continue;
                };
                let Object::Array(items) = document.get_key(&root, "Namespaces") else {
                    continue;
                };
                items.into_iter().find(|item| {
                    item.as_reference().is_some()
                        && document.resolve(item).as_dict().is_some_and(|space| {
                            pdf_model::structure::Namespace::read(document, space)
                                .is_some_and(|space| space.name == STANDARD_NAMESPACE_2_0)
                        })
                })
            };
            if let Some(item) = listed {
                return Ok(host.carry_value(at, &item));
            }
        }
        let id = host
            .reserve_slot()
            .map_err(|why| Refusal::Assembly(why.to_string()))?;
        let mut space = Dictionary::new();
        space.insert(
            Name::new(&b"Type"[..]),
            Object::Name(Name::new(&b"Namespace"[..])),
        );
        space.insert(
            Name::new(&b"NS"[..]),
            Object::String(STANDARD_NAMESPACE_2_0.as_bytes().to_vec().into()),
        );
        host.place_object(id, Object::Dictionary(space));
        self.lists
            .entry("Namespaces")
            .or_default()
            .push(Object::Reference(id));
        Ok(Object::Reference(id))
    }
}
