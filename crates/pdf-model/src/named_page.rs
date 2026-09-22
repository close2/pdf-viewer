//! ISO 32000-2 §12.7.7's named pages: the pages a document keeps *outside* the page tree.
//!
//! The clause is one paragraph and a rule, and the rule is what makes it checkable:
//!
//! > A named page that is intended to be visible to a user shall be left in the page tree (see
//! > 7.7.3, "Page tree" ), and there shall be a reference to it in the appropriate leaf node of
//! > the name dictionary's Pages tree. If the page is not intended to be displayed by the PDF
//! > processor, it shall be referenced from the name dictionary's Templates tree instead. Such
//! > invisible pages shall have an object type of Template rather than Page and shall have no
//! > Parent or B entry
//!
//! So there are two trees, they mean opposite things, and each states an invariant about the
//! objects it names. [`NamedPages::disagreements`] runs both, for §12.8.7's reason (ADR 0089): a
//! clause that states a rule about a document's own contents is a clause that can audit one.
//!
//! # Why a renderer reads this at all
//!
//! §12.7.7 gives naming a page exactly two purposes, and one of them is excluded:
//!
//! > - An import-data action can add the named page to the document into which FDF is being
//! >   imported, either as a page or as a button appearance.
//! > - A script executed by an ECMAScript action can add the named page to the current document
//! >   as a regular page.
//!
//! The second is `CLAUDE.md`'s closed exclusion list. The first is §12.7.8.3.3, which is read as
//! of the hundredth session (ADR 0090) and which this closes: an FDF page dictionary's
//! `/Templates` each carry a Table 253 named page reference, and resolving one means looking a
//! name up here. A template page added to the document is a page a viewer *shows*, so this is
//! not a data-interchange feature — it is the one route by which a document gains a page after
//! it was opened.
//!
//! # A template page is not in the page tree, so it is built without one
//!
//! §7.7.3.4's inheritance runs up `/Parent`, and this clause says a template "shall have no
//! Parent". A template page therefore inherits nothing and states everything it needs, which is
//! what [`crate::page::Pages::detached`] builds: the same `build_page` every other page goes
//! through, with an empty ancestry.
//!
//! # One corpus document names a page, and it is not the tree anybody expected
//!
//! Measured over all 964 openable pdf.js documents by running [`NamedPages::read`] on each:
//! **one** names a page — `issue19389.pdf`, one entry in the `/Pages` tree — and **none** states
//! a `/Templates` tree at all. That one document agrees with every rule
//! [`NamedPages::disagreements`] checks.
//!
//! The number matters more than it looks. This comment was about to say "no corpus document has
//! one", which is the shape of claim trap 8 exists for, and measuring it cost one throwaway test
//! over a corpus that was already on disk. What the measurement does *not* change is that the
//! tests here are synthetic: one document with one name exercises no rule.

use pdf_syntax::{Document, Object, ObjectId, tree};

/// Most names taken from either tree.
///
/// A document naming this many pages has one name per page of a large book, which is far past
/// any template library and is where a file stops describing a document and starts making a
/// reader work.
const MAX_NAMES: usize = 16384;

/// §12.7.7's two name trees, read.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NamedPages {
    /// `/Names /Pages`: names of pages that are "intended to be visible to a user".
    ///
    /// In the tree's own order, which §7.9.6 makes ascending by key over the key's own bytes —
    /// so this is a list a person could be shown, and a duplicate name is kept rather than
    /// resolved, because the clause states no tie-break and a file writing one has said
    /// something worth seeing.
    ///
    /// **This line said "which §7.9.6 makes lexical by key" until the
    /// seven-hundred-and-fiftieth session**, and *lexical* is the one word Errata Collection 3's
    /// Issue #214 takes out of that clause; what defines the order is the byte comparison the
    /// two sentences after it state, which `pdf_syntax::tree` now carries in full.
    pub pages: Vec<(String, ObjectId)>,
    /// `/Names /Templates`: names of pages "not intended to be displayed by the PDF processor".
    pub templates: Vec<(String, ObjectId)>,
}

/// One place a document contradicts §12.7.7's own rule about the pages it names.
///
/// A question rather than a verdict, exactly as [`pdf_signature::signature`]'s attestations are: the
/// clause states what a *writer* shall do, so a reader meeting a breach has found a malformed
/// file and not an instruction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Disagreement {
    /// The name the tree filed the page under.
    pub name: String,
    /// What the clause says about it and the document does not.
    pub detail: &'static str,
}

impl NamedPages {
    /// Reads both of §7.7.4's page-naming trees.
    ///
    /// Empty for a document with no name dictionary and for one whose name dictionary states
    /// neither tree, which is all 974 corpus documents — neither is an error and neither is
    /// distinguishable from the other here, because §12.7.7 makes both entries optional.
    #[must_use]
    pub fn read(document: &Document) -> Self {
        let Ok(catalog) = document.catalog() else {
            return Self::default();
        };
        let names = document.get_key(&catalog, "Names");
        let Some(names) = names.as_dict() else {
            return Self::default();
        };
        let read_tree = |key: &str| {
            let root = document.get_key(names, key);
            let Some(root) = root.as_dict() else {
                return Vec::new();
            };
            // `name_entries` rather than `name_pairs`: a page is identified by its *object*,
            // which is what `Pages::index_of` compares and what a template reference resolves
            // to, and resolving the leaf would throw that identity away. A leaf that is not a
            // reference names a page the file did not make an indirect object of, which no
            // destination and no `/Parent` could reach either, so it is not a named page.
            tree::name_entries(root, &|object| document.resolve(object))
                .into_iter()
                .take(MAX_NAMES)
                .filter_map(|(name, value)| {
                    Some((pdf_syntax::text_string(&name), value.as_reference()?))
                })
                .collect()
        };
        Self {
            pages: read_tree("Pages"),
            templates: read_tree("Templates"),
        }
    }

    /// Whether the document names no page at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.pages.is_empty() && self.templates.is_empty()
    }

    /// The page a Table 253 named page reference names.
    ///
    /// Both trees, `/Templates` first. The clause divides them by *intent* rather than by
    /// namespace — one holds pages a person may see and the other pages that exist to be
    /// copied — and §12.7.8.3.3 asks for "the location of the template", so a name in the
    /// template tree is the one it means. A name in both is a file contradicting itself, which
    /// [`Self::disagreements`] does not check because the clause states no such rule.
    #[must_use]
    pub fn lookup(&self, name: &str) -> Option<ObjectId> {
        let find = |list: &[(String, ObjectId)]| {
            list.iter()
                .find(|(candidate, _)| candidate == name)
                .map(|(_, id)| *id)
        };
        find(&self.templates).or_else(|| find(&self.pages))
    }

    /// Where this document breaks §12.7.7's own rule about the pages it names.
    ///
    /// Four checks, one per clause of the quoted sentence: a page in the `/Pages` tree shall be
    /// in the page tree; a page in the `/Templates` tree shall have `/Type /Template`, no
    /// `/Parent` and no `/B`. Empty for every well-formed document and for every document that
    /// names no page.
    ///
    /// `pages` is the document's own page index, which the `/Pages` half needs. One walk of the
    /// tree serves every name (`Pages::indices`) rather than one walk per name — the shape
    /// §12.3.3's outline was quadratic in until the hundred-and-forty-first session, and the
    /// same fix, applied here before anybody met it on a document with enough names to notice.
    #[must_use]
    pub fn disagreements(
        &self,
        document: &Document,
        pages: &crate::page::Pages<'_>,
    ) -> Vec<Disagreement> {
        let mut out = Vec::new();
        let indices = (!self.pages.is_empty()).then(|| pages.indices());
        for (name, id) in &self.pages {
            if !indices
                .as_ref()
                .is_some_and(|indices| indices.contains_key(id))
            {
                out.push(Disagreement {
                    name: name.clone(),
                    detail: "named in /Pages and not in the page tree, which §12.7.7 requires \
                             of a page intended to be visible",
                });
            }
        }
        for (name, id) in &self.templates {
            let object = document.get(*id);
            let Some(dict) = object.as_dict() else {
                out.push(Disagreement {
                    name: name.clone(),
                    detail: "named in /Templates and is not a dictionary",
                });
                continue;
            };
            let stated = document.get_key(dict, "Type");
            if stated
                .as_name()
                .is_some_and(|kind| kind.as_bytes() != b"Template")
            {
                out.push(Disagreement {
                    name: name.clone(),
                    detail: "named in /Templates with a /Type that is not Template",
                });
            }
            for (key, detail) in [
                (
                    "Parent",
                    "named in /Templates and has a /Parent, which §12.7.7 forbids",
                ),
                (
                    "B",
                    "named in /Templates and has a /B, which §12.7.7 forbids",
                ),
            ] {
                if dict.get(key).is_some() {
                    out.push(Disagreement {
                        name: name.clone(),
                        detail,
                    });
                }
            }
        }
        out
    }
}

/// A Table 253 named page reference: which page, and in which file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reference {
    /// `/Name`, "[t]he name of the referenced page". Required.
    pub name: String,
    /// `/F`, "[t]he file containing the named page", where the reference names one.
    ///
    /// The table states the default itself — "[i]f this entry is absent, it shall be assumed
    /// that the page resides in the associated PDF file" — so `None` means *this* document and is
    /// what [`page_as_form`] and `crate::view::ViewState::import` answer, while `Some` names a
    /// second PDF file.
    ///
    /// That second file is a file a **document** named, which is §12.7.6.4's hazard: its bytes
    /// may come only from a directory a person supplied, which `viewer_host::policy::read_import`
    /// answers and no part of this crate has (ADR 1155). It is a host question rather than an
    /// impossibility, and it is named rather than silently ignored. ADR 1235.
    pub file: Option<String>,
}

impl Reference {
    /// Reads Table 253's two entries.
    ///
    /// `None` where `/Name` is absent or is not a string: the entry is required, and a
    /// reference naming no page has stated nothing to resolve.
    #[must_use]
    pub fn read(document: &Document, dict: &pdf_syntax::Dictionary) -> Option<Self> {
        let name = pdf_syntax::text_string(document.get_key(dict, "Name").as_string()?);
        // §7.11.3's file specification, in both its forms, as §12.7.6.4's `/F` is read.
        let stated = document.get_key(dict, "F");
        let file = match &stated {
            Object::String(bytes) => Some(pdf_syntax::text_string(bytes)),
            Object::Dictionary(specification) => ["UF", "F"].iter().find_map(|key| {
                document
                    .get_key(specification, key)
                    .as_string()
                    .map(pdf_syntax::text_string)
            }),
            _ => None,
        };
        Some(Self { name, file })
    }
}

/// A page this document holds, as the form `XObject` §12.5.5 places for a widget's appearance.
///
/// §12.7.7 states the two things naming a page is for, and this is the first of them:
///
/// > - An import-data action can add the named page to the document into which FDF is being
/// >   imported, either as a page or as a button appearance.
///
/// *As a button appearance* is §12.7.8.3.2's `/APRef`, whose `/N`, `/R` and `/D` are Table 253
/// named page references rather than streams. A widget's appearance is a form `XObject`: §12.5.5's
/// algorithm maps from that form's own coordinate system, as its `/Matrix` defines it, onto the
/// annotation's rectangle in default user space. So the conversion this needs is page to form, and
/// Table 93 is what it is written into.
///
/// **The marks are the document's own producer's**, carried without reinterpretation: the content
/// stream is §7.8.2's concatenation of the page's `/Contents` and the resources are the ones
/// §7.7.3.4's inheritance puts in effect for it. Nothing is composed, which is `CLAUDE.md`'s
/// provenance test (ADR 1120). ADR 1235.
///
/// Three entries are decided here rather than copied, and each follows a clause:
///
/// - **`/BBox`** is the page's crop box, which §14.11.2.1 makes "the region to which the contents
///   of the page shall be clipped (cropped) when displayed or printed" — the same clipping Table
///   93 gives a form's bounding box, already defaulted to the media box and intersected with it.
/// - **`/Matrix`** carries Table 31's `/Rotate`, "[t]he number of degrees by which the page shall
///   be rotated clockwise when displayed or printed", as the rotation about the origin that turns
///   the page's space that way. §12.5.5's step 1 takes the bounding box *through* the matrix and
///   bounds it upright, and its steps 2 and 3 then map that onto `/Rect`, so no translation is
///   owed here: the algorithm supplies it.
/// - **`/Group`** is copied where the page states one, because §11.4.7's page group and Table
///   93's group attributes dictionary are the same dictionary — Table 31 points at §11.4.7 and
///   §11.6.6 together — and a page whose marks the producer composited inside a group would
///   composite differently without it.
///
/// `None` where the object is not a dictionary, or where a part of `/Contents` did not decode:
/// half a page is a button drawn wrong, and a caller that gets `None` names it rather than
/// showing marks the document did not state (trap 5).
///
/// **A page's `/Annots` are not part of this.** Table 31 keeps them beside `/Contents` rather than
/// in it, and §12.5.5 draws an annotation against the page it is on; composing them into a form
/// would be this program deciding a composition the producer did not write, which is the far side
/// of the same fence. A page carrying them is named by the caller.
#[must_use]
pub fn page_as_form(
    document: &Document,
    pages: &crate::page::Pages<'_>,
    id: ObjectId,
) -> Option<Object> {
    let object = document.get(id);
    let dict = object.as_dict()?;
    // A name in the `/Pages` tree is a page of the page tree, so §7.7.3.4's inheritance runs up
    // its `/Parent`; a `/Templates` name is outside the tree and "shall have no Parent", so it
    // inherits nothing. `Pages::get` is the first reading and `Pages::detached` the second.
    let page = pages
        .index_of(id)
        .and_then(|index| pages.get(index))
        .unwrap_or_else(|| pages.detached(dict));
    let (content, issues) = page.content_with_report(document);
    if !issues.is_empty() {
        return None;
    }
    let mut form = pdf_syntax::Dictionary::new();
    let name = |bytes: &[u8]| Object::Name(pdf_syntax::Name::new(bytes));
    form.insert(pdf_syntax::Name::new(&b"Type"[..]), name(b"XObject"));
    form.insert(pdf_syntax::Name::new(&b"Subtype"[..]), name(b"Form"));
    form.insert(
        pdf_syntax::Name::new(&b"BBox"[..]),
        Object::Array(
            page.crop_box
                .iter()
                .map(|edge| Object::Real(f64::from(*edge)))
                .collect(),
        ),
    );
    if let Some(matrix) = rotation(page.rotate) {
        form.insert(
            pdf_syntax::Name::new(&b"Matrix"[..]),
            Object::Array(matrix.iter().map(|term| Object::Real(*term)).collect()),
        );
    }
    form.insert(
        pdf_syntax::Name::new(&b"Resources"[..]),
        Object::Dictionary(page.resources.clone()),
    );
    if let Some(group) = dict.get("Group") {
        form.insert(pdf_syntax::Name::new(&b"Group"[..]), group.clone());
    }
    // The bytes are written as they came out of the filters, so the stream states no `/Filter`
    // and §7.3.8.2's `/Length` is their own count. `crate::view::ViewState` gives the stream an
    // object number when it is saved, which §7.3.8.1 requires of a stream in a file.
    form.insert(
        pdf_syntax::Name::new(&b"Length"[..]),
        Object::Integer(i64::try_from(content.len()).ok()?),
    );
    Some(Object::Stream(std::sync::Arc::new(pdf_syntax::Stream {
        dict: form,
        data: content.into(),
        decryption_failed: false,
    })))
}

/// Table 31's `/Rotate` as the form matrix that turns a page's space that way, or `None` for a
/// page stating no rotation.
///
/// The entry is "[t]he number of degrees by which the page shall be rotated clockwise", and
/// `crate::page::Page::rotate` has already normalised it to one of the four multiples of 90 the
/// table permits. A clockwise turn by `r` in a space whose y axis runs up is §8.3.3's rotation by
/// `-r`, whose terms at these four angles are exactly 0, 1 and -1 — so no trigonometry is
/// evaluated and the matrix carries no rounding.
fn rotation(degrees: u16) -> Option<[f64; 6]> {
    match degrees {
        90 => Some([0.0, -1.0, 1.0, 0.0, 0.0, 0.0]),
        180 => Some([-1.0, 0.0, 0.0, -1.0, 0.0, 0.0]),
        270 => Some([0.0, 1.0, -1.0, 0.0, 0.0, 0.0]),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt::Write as _;

    /// Builds a document from object bodies numbered from 1.
    fn document(objects: &[&str]) -> Document {
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
        Document::open(out.into_bytes()).expect("the fixture is a valid PDF")
    }

    /// The catalog, page tree and one page every fixture here shares, with `extra` written into
    /// the catalog's `/Names` dictionary.
    fn with_names(names: &str, extra: &[&str]) -> Document {
        let catalog = format!("<< /Type /Catalog /Pages 2 0 R /Names << {names} >> >>");
        let mut objects = vec![
            catalog.as_str(),
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] >>",
        ];
        objects.extend_from_slice(extra);
        document(&objects)
    }

    /// §12.7.7's two trees mean opposite things, and both are read.
    #[test]
    fn both_trees_are_read_and_a_template_is_looked_up_first() {
        let document = with_names(
            "/Pages << /Names [(cover) 3 0 R] >> /Templates << /Names [(blank) 4 0 R] >>",
            &["<< /Type /Template /MediaBox [0 0 200 100] >>"],
        );
        let named = NamedPages::read(&document);
        assert_eq!(named.pages, [("cover".to_owned(), ObjectId::new(3, 0))]);
        assert_eq!(named.templates, [("blank".to_owned(), ObjectId::new(4, 0))]);
        assert_eq!(named.lookup("blank"), Some(ObjectId::new(4, 0)));
        assert_eq!(named.lookup("cover"), Some(ObjectId::new(3, 0)));
        assert_eq!(named.lookup("neither"), None);
    }

    /// The clause states four rules about the objects its trees name, and a document that
    /// breaks them is asking a question rather than giving an instruction.
    #[test]
    fn a_document_that_breaks_the_clauses_own_rule_is_named() {
        let document = with_names(
            "/Pages << /Names [(loose) 4 0 R] >> /Templates << /Names [(wrong) 5 0 R] >>",
            &[
                "<< /Type /Page /MediaBox [0 0 200 100] >>",
                "<< /Type /Page /Parent 2 0 R /B [] /MediaBox [0 0 200 100] >>",
            ],
        );
        let named = NamedPages::read(&document);
        let pages = crate::page::Pages::new(&document);
        let details: Vec<&str> = named
            .disagreements(&document, &pages)
            .iter()
            .map(|item| item.detail)
            .collect();
        assert_eq!(details.len(), 4, "{details:?}");
        assert!(details[0].contains("not in the page tree"));
        assert!(details[1].contains("/Type that is not Template"));
        assert!(details[2].contains("has a /Parent"));
        assert!(details[3].contains("has a /B"));
    }

    /// A well-formed document contradicts nothing, and one that names no page has nothing to
    /// contradict.
    #[test]
    fn a_well_formed_document_and_an_empty_one_both_agree() {
        let good = with_names(
            "/Pages << /Names [(cover) 3 0 R] >> /Templates << /Names [(blank) 4 0 R] >>",
            &["<< /Type /Template /MediaBox [0 0 200 100] >>"],
        );
        let pages = crate::page::Pages::new(&good);
        assert!(
            NamedPages::read(&good)
                .disagreements(&good, &pages)
                .is_empty()
        );

        let bare = with_names("", &[]);
        assert!(NamedPages::read(&bare).is_empty());
    }

    /// Table 253: `/Name` is required and `/F` names another file, whose absence the table
    /// makes mean *this* one.
    #[test]
    fn a_named_page_reference_states_a_name_and_maybe_a_file() {
        let document = with_names("", &[]);
        let mut dict = pdf_syntax::Dictionary::new();
        assert_eq!(Reference::read(&document, &dict), None, "/Name is required");
        dict.insert(
            pdf_syntax::Name::new(b"Name".to_vec()),
            Object::String(b"blank".to_vec().into()),
        );
        assert_eq!(
            Reference::read(&document, &dict),
            Some(Reference {
                name: "blank".to_owned(),
                file: None
            })
        );
        dict.insert(
            pdf_syntax::Name::new(b"F".to_vec()),
            Object::String(b"library.pdf".to_vec().into()),
        );
        assert_eq!(
            Reference::read(&document, &dict).and_then(|reference| reference.file),
            Some("library.pdf".to_owned())
        );
    }
}
