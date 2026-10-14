//! What Annex K's two entries say a document is, so that a reader can be told what is not done.
//!
//! This program does not implement XFA, on the permission ISO 32000-2 §K.1 grants in as many
//! words:
//!
//! > The implementation of such a schema driven page generation involves considerable effort
//! > beyond that for a simple PDF viewer and therefore a PDF processor may choose to not
//! > implement this feature.
//!
//! Declining is safe for the form a reader fills because §K.2 binds the writer of a hybrid file:
//!
//! > The other entries in the interactive form dictionary shall be consistent with the information
//! > in the XFA resource.
//!
//! so the `AcroForm` this tree reads is the form. What declining costs is everything only the XFA
//! resource states, and on a document whose catalog asks for its pages to be regenerated, the
//! pages themselves. A reader who is not told that sees a notice page, or a form whose
//! calculations never run, with nothing saying why — trap 5's shape. So this module answers one
//! question, *does this document carry an XFA form, and of which kind*, and `viewer-core` says the
//! answer once when the document reports on itself. Nothing here parses XML or reads a packet:
//! the presence of the entries is the whole of what the sentence needs (RFC 0009 §4.4, ADR 1932).

use pdf_syntax::{Document, Object};

/// What a document's catalog and interactive form dictionary state of Annex K.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stated {
    /// Table 224's `/XFA` is present in the interactive form dictionary as one of the two forms
    /// §K.2 allows: "either a stream containing the entire XFA resource or an array specifying
    /// individual packets that together make up the XFA resource".
    ///
    /// An entry of any other type is not an XFA resource under that sentence and is not counted;
    /// nothing in this program reads the entry, so there is nothing for it to have failed at.
    pub resource: bool,
    /// Table 29's `/NeedsRendering` is `true` in the catalog: "It specifies whether the document
    /// shall be regenerated when the document is first opened."
    ///
    /// The table's default is `false`, and a value that is not a boolean states nothing.
    pub needs_rendering: bool,
}

/// Whether the document states either of Annex K's entries; `None` where it states neither.
///
/// A catalog lookup and, where the catalog names one, an interactive form dictionary lookup —
/// no field tree, no stream decoded — so asking costs a document with no form nothing it would
/// notice.
#[must_use]
pub fn stated(document: &Document) -> Option<Stated> {
    let catalog = document.catalog().ok()?;
    let needs_rendering = matches!(
        document.get_key(&catalog, "NeedsRendering"),
        Object::Boolean(true)
    );
    let form = document.get_key(&catalog, "AcroForm");
    let resource = form.as_dict().is_some_and(|form| {
        matches!(
            document.get_key(form, "XFA"),
            Object::Stream(_) | Object::Array(_)
        )
    });
    (resource || needs_rendering).then_some(Stated {
        resource,
        needs_rendering,
    })
}

#[cfg(test)]
mod tests {
    use super::{Stated, stated};
    use pdf_syntax::Document;

    /// Builds a document from object bodies numbered from 1; object 1 is the catalog.
    fn document(objects: &[&str]) -> Document {
        use std::fmt::Write as _;
        let mut out = String::from("%PDF-1.7\n");
        let mut offsets = Vec::new();
        for (index, body) in objects.iter().enumerate() {
            offsets.push(out.len());
            let _ = write!(out, "{} 0 obj\n{body}\nendobj\n", index.saturating_add(1));
        }
        let xref_at = out.len();
        let _ = write!(
            out,
            "xref\n0 {}\n0000000000 65535 f \n",
            objects.len().saturating_add(1)
        );
        for offset in &offsets {
            let _ = writeln!(out, "{offset:010} 00000 n ");
        }
        let _ = write!(
            out,
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n",
            objects.len().saturating_add(1)
        );
        Document::open(out.into_bytes()).expect("a valid file")
    }

    const PAGES: &str = "<< /Type /Pages /Kids [] /Count 0 >>";
    const PACKET: &str = "<< /Length 9 >>\nstream\n<xdp:xdp>\nendstream";

    /// §K.2's two forms of the entry — a stream, and an array of packets as its EXAMPLE 1 spells
    /// it — are each an XFA resource, and the catalog states nothing about regeneration.
    #[test]
    fn a_stream_or_an_array_of_packets_is_an_xfa_resource() {
        let as_stream = document(&[
            "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [] /XFA 3 0 R >> >>",
            PAGES,
            PACKET,
        ]);
        let as_packets = document(&[
            "<< /Type /Catalog /Pages 2 0 R /AcroForm 4 0 R >>",
            PAGES,
            PACKET,
            "<< /Fields [] /XFA [(xdp:xdp) 3 0 R (/xdp:xdp) 3 0 R] >>",
        ]);
        let hybrid = Some(Stated {
            resource: true,
            needs_rendering: false,
        });
        assert_eq!(stated(&as_stream), hybrid);
        assert_eq!(stated(&as_packets), hybrid);
    }

    /// Table 29's flag is read where the catalog states it `true`, with or without a resource
    /// beside it, and its default `false` states nothing.
    #[test]
    fn needs_rendering_is_read_only_where_it_is_true() {
        let dynamic = document(&[
            "<< /Type /Catalog /Pages 2 0 R /NeedsRendering true \
             /AcroForm << /Fields [] /XFA 3 0 R >> >>",
            PAGES,
            PACKET,
        ]);
        assert_eq!(
            stated(&dynamic),
            Some(Stated {
                resource: true,
                needs_rendering: true,
            })
        );
        let flag_alone = document(&[
            "<< /Type /Catalog /Pages 2 0 R /NeedsRendering true >>",
            PAGES,
        ]);
        assert_eq!(
            stated(&flag_alone),
            Some(Stated {
                resource: false,
                needs_rendering: true,
            })
        );
        let default = document(&[
            "<< /Type /Catalog /Pages 2 0 R /NeedsRendering false >>",
            PAGES,
        ]);
        assert_eq!(stated(&default), None);
    }

    /// A form with no `/XFA`, and an `/XFA` of a type §K.2 does not allow, state no XFA form;
    /// a document with no form at all states nothing.
    #[test]
    fn an_acroform_alone_or_a_malformed_entry_states_no_xfa_form() {
        let plain = document(&[
            "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [] >> >>",
            PAGES,
        ]);
        let malformed = document(&[
            "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [] /XFA (template) >> >>",
            PAGES,
        ]);
        let none = document(&["<< /Type /Catalog /Pages 2 0 R >>", PAGES]);
        assert_eq!(stated(&plain), None);
        assert_eq!(stated(&malformed), None);
        assert_eq!(stated(&none), None);
    }
}
