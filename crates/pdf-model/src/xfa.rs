//! What Annex K's two entries say a document is, so that a reader can be told what is not done.
//!
//! This program does not implement XFA's dynamic half, its schema-driven page generation, on the
//! permission ISO 32000-2 §K.1 grants in as many words:
//!
//! > The implementation of such a schema driven page generation involves considerable effort
//! > beyond that for a simple PDF viewer and therefore a PDF processor may choose to not
//! > implement this feature.
//!
//! Its static half, a form whose pages the producer wrote, is in scope (`CLAUDE.md`'s XFA entry,
//! `doc/questions/A639`, RFC 0011), and declining the dynamic half is safe for the form a reader
//! fills because §K.2 binds the writer of a hybrid file:
//!
//! > The other entries in the interactive form dictionary shall be consistent with the information
//! > in the XFA resource.
//!
//! so the `AcroForm` this tree reads is the form. What declining costs is everything only the XFA
//! resource states, and on a document whose catalog asks for its pages to be regenerated, the
//! pages themselves. A reader who is not told that sees a notice page, or a form whose
//! calculations never run, with nothing saying why — trap 5's shape. So this module answers one
//! question, *does this document carry an XFA form, and of which kind*, and `viewer-core` says the
//! answer once when the document reports on itself; the presence of the entries is the whole of
//! what that sentence needs (RFC 0009 §4.4, ADR 1932).
//!
//! [`Resource`] is the other thing it answers, and it is a writer's question. §K.2 binds a writer
//! of a hybrid file:
//!
//! > The XFA field values shall be consistent with the corresponding V entries of the PDF field
//! > objects.
//!
//! so a save that writes a `/V` reads the template and the `datasets` packet far enough to write
//! the data value that field binds: in its canonical form, written out by its `bind` picture and
//! passed by its `validate` picture's format test (RFC 0011 section 5's steps 1 to 3 and step 4's
//! format test; ADRs 2396, 2414, 2434).

use pdf_syntax::{Document, Object, ObjectId};

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

mod calculate;
mod canonical;
mod config;
mod datasets;
mod locale;
mod picture;
mod template;
mod validate;
mod xml;

pub use calculate::{
    Activation, Calculation, Ended, Form, Limits, Occurrence, Ran, Runner, Scalar, Step, Stimulus,
};
pub use config::Config;
pub use datasets::{Change, Datasets, Kept};
pub use locale::LocaleSet;
pub use template::{Language, Override, Template};
pub use xml::XmlError;

/// The largest XFA packet this module decodes.
///
/// §K.2 states no limit and a packet is a stream, so this is a decompression-bomb bound rather
/// than a reading (principle 3). The largest packet of RFC 0011 section 4's census is a
/// government form's template of 1.6 MB, so 32 MiB is twenty times past it.
pub const MAX_PACKET_BYTES: usize = 32 << 20;

/// Why an XFA resource could not be read into its template and its data.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ResourceError {
    /// §K.2 lists the packets a resource holds, and this one lacks one a reader needs.
    #[error("the XFA resource has no {0} packet")]
    Missing(&'static str),
    /// A packet's stream would not decode: a filter this tree refuses, or a missing key.
    #[error("the XFA {0} packet would not decode")]
    Undecodable(&'static str),
    /// A packet decodes past [`MAX_PACKET_BYTES`].
    #[error(
        "the XFA {packet} packet is {bytes} bytes, past this reader's {MAX_PACKET_BYTES}-byte bound"
    )]
    TooLarge {
        /// Which packet.
        packet: &'static str,
        /// Its decoded length.
        bytes: usize,
    },
    /// A packet is not XML this module reads.
    #[error("the XFA {packet} packet: {error}")]
    Xml {
        /// Which packet.
        packet: &'static str,
        /// What the reader said.
        error: XmlError,
    },
}

/// §K.2's XFA resource, read into the template and the data — RFC 0011 section 5's first step,
/// as far as its second needs (ADR 2396).
///
/// §K.2 allows the entry two forms, "either a stream containing the entire XFA resource or an
/// array specifying individual packets that together make up the XFA resource". Both are read:
/// the stream as one `xdp:xdp` document holding a `template` and a `datasets` element, the array
/// by its packet names, each packet parsed alone.
#[derive(Debug, Clone)]
pub struct Resource {
    /// The template's containers.
    pub template: Template,
    /// The `datasets` packet.
    pub datasets: Datasets,
    /// The `localeSet` packet's locales and the `config` packet's default locale, which a
    /// field's picture clauses are read in (XFA 3.3 pages 153 and 158; ADR 2414).
    pub locales: LocaleSet,
    /// The `config` packet's extended mapping rules and validation options (XFA 3.3 chapter 15,
    /// pages 386 and 1340; ADR 2450); its defaults where the resource holds no `config`.
    pub config: Config,
    /// The stream whose text [`Datasets::keep`] splices: the `datasets` packet's in the array
    /// form, the whole resource's in the stream form. A save replaces this one object, which is
    /// §7.5.6's "objects that have been changed, replaced, or deleted".
    pub datasets_stream: ObjectId,
}

impl Resource {
    /// The messages of the validations a save runs, where the form's `config` asks for them before
    /// one (XFA 3.3 pages 396, 506 and 1340): one `Vec` per message the person is shown, as page
    /// 386's `validationMessaging` groups them, each holding one sentence per field one of its
    /// tests fails — the `validate` element's null test and the datatype test its `value`
    /// implies. `fields` are every field of the form by its PDF name, with the value the save
    /// leaves in its `/V` (ADRs 2450, 2474).
    #[must_use]
    pub fn before_save(&self, fields: &[(String, Change)]) -> Vec<Vec<String>> {
        self.validations(validate::Occasion::Save, fields)
    }

    /// The messages of the validations a submit runs, where the form's `config` asks for them
    /// before one (page 1340's `preSubmit`, the default; page 447's standard submit processing):
    /// the null test, the datatype test and the format test, over every field of the form
    /// whatever the submit carries. `fields` are every field of the form by its PDF name, with the
    /// value the submit carries from its `/V` (ADR 2475).
    #[must_use]
    pub fn before_submit(&self, fields: &[(String, Change)]) -> Vec<Vec<String>> {
        self.validations(validate::Occasion::Submit, fields)
    }

    fn validations(
        &self,
        occasion: validate::Occasion,
        fields: &[(String, Change)],
    ) -> Vec<Vec<String>> {
        validate::before(
            occasion,
            (&self.template, &self.locales, &self.config),
            fields,
        )
    }

    /// The resource of `document`'s interactive form dictionary; `None` where it states none in
    /// either of §K.2's forms.
    ///
    /// # Errors
    ///
    /// [`ResourceError`], whose variants are the reasons.
    pub fn read(document: &Document) -> Option<Result<Self, ResourceError>> {
        let catalog = document.catalog().ok()?;
        let form = document.get_key(&catalog, "AcroForm");
        let form = form.as_dict()?;
        Some(match form.get("XFA")?.clone() {
            Object::Reference(id) => match document.get(id) {
                Object::Stream(_) => Self::from_stream(document, id),
                Object::Array(packets) => Self::from_packets(document, &packets),
                _ => return None,
            },
            Object::Array(packets) => Self::from_packets(document, &packets),
            _ => return None,
        })
    }

    fn from_stream(document: &Document, id: ObjectId) -> Result<Self, ResourceError> {
        let (text, tree) = packet(document, id, "XDP")?;
        // The `template` packet is a child of the `xdp:xdp` root. A `config` packet before it
        // holds a `template` element of its own, the configuration's (XFA 3.3 page 1335), which
        // is not the form's (ADR 2482).
        let template = tree
            .child(0, "template")
            // Only a stream whose root has no `template` child falls back to any depth: it is
            // not the shape §K.2 gives, and a configuration's `template` can be picked there.
            .or_else(|| tree.first("template"))
            .ok_or(ResourceError::Missing("template"))?;
        let datasets = tree
            .first("datasets")
            .ok_or(ResourceError::Missing("datasets"))?;
        let locale_set = tree.first("localeSet").map(|at| (&tree, at));
        let config = tree.first("config").is_some().then_some(&tree);
        let locales = LocaleSet::read(locale_set, config);
        Ok(Self {
            template: Template::read(&tree, template),
            locales,
            config: Config::read(&tree),
            datasets: Datasets::new(text, tree, datasets),
            datasets_stream: id,
        })
    }

    /// §K.2's array: "an array specifying individual packets", each a name and a stream, which
    /// its EXAMPLE 1 writes as a text string followed by a reference.
    fn from_packets(document: &Document, packets: &[Object]) -> Result<Self, ResourceError> {
        let named = |wanted: &str| {
            packets.chunks_exact(2).find_map(|pair| match pair {
                [Object::String(name), Object::Reference(id)]
                    if pdf_syntax::text_string(name) == wanted =>
                {
                    Some(*id)
                }
                _ => None,
            })
        };
        let template_id = named("template").ok_or(ResourceError::Missing("template"))?;
        let datasets_id = named("datasets").ok_or(ResourceError::Missing("datasets"))?;
        let (_, template_tree) = packet(document, template_id, "template")?;
        let (text, tree) = packet(document, datasets_id, "datasets")?;
        let template = template_tree
            .first("template")
            .ok_or(ResourceError::Missing("template"))?;
        let datasets = tree
            .first("datasets")
            .ok_or(ResourceError::Missing("datasets"))?;
        // A `config` packet that is stated and does not read leaves the form without its
        // configured locale and with page 509's and page 1340's defaults, rather than refusing
        // the fields that need neither.
        let config = match named("config").map(|id| packet(document, id, "config")) {
            Some(Ok((_, tree))) => Some(tree),
            _ => None,
        };
        Ok(Self {
            template: Template::read(&template_tree, template),
            datasets: Datasets::new(text, tree, datasets),
            datasets_stream: datasets_id,
            locales: Self::locales(document, named("localeSet"), config.as_ref()),
            config: config.as_ref().map(Config::read).unwrap_or_default(),
        })
    }

    /// The array form's `localeSet` and `config` packets, each optional. A packet that is stated
    /// and does not read leaves the form without locales and says why, rather than refusing the
    /// fields that need none.
    fn locales(
        document: &Document,
        locale_set: Option<ObjectId>,
        config: Option<&xml::Tree>,
    ) -> LocaleSet {
        match locale_set.map(|id| packet(document, id, "localeSet")) {
            None => LocaleSet::read(None, config),
            Some(Ok((_, tree))) => match tree.first("localeSet") {
                Some(at) => LocaleSet::read(Some((&tree, at)), config),
                None => LocaleSet::unreadable(
                    "its localeSet packet holds no localeSet element".to_owned(),
                ),
            },
            Some(Err(error)) => LocaleSet::unreadable(error.to_string()),
        }
    }
}

/// One packet's stream, decoded under [`MAX_PACKET_BYTES`] and parsed.
fn packet(
    document: &Document,
    id: ObjectId,
    name: &'static str,
) -> Result<(String, xml::Tree), ResourceError> {
    let object = document.get(id);
    let stream = object.as_stream().ok_or(ResourceError::Undecodable(name))?;
    let bytes = document
        .decoded_stream_data(stream)
        .ok_or(ResourceError::Undecodable(name))?;
    if bytes.len() > MAX_PACKET_BYTES {
        return Err(ResourceError::TooLarge {
            packet: name,
            bytes: bytes.len(),
        });
    }
    let tree = xml::parse(&bytes).map_err(|error| ResourceError::Xml {
        packet: name,
        error,
    })?;
    // `parse` has already refused bytes that are not UTF-8.
    let text = String::from_utf8_lossy(&bytes).into_owned();
    Ok((text, tree))
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
