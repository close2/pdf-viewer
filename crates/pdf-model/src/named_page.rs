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
    /// In the tree's own order, which §7.9.6 makes lexical by key — so this is a list a person
    /// could be shown, and a duplicate name is kept rather than resolved, because the clause
    /// states no tie-break and a file writing one has said something worth seeing.
    pub pages: Vec<(String, ObjectId)>,
    /// `/Names /Templates`: names of pages "not intended to be displayed by the PDF processor".
    pub templates: Vec<(String, ObjectId)>,
}

/// One place a document contradicts §12.7.7's own rule about the pages it names.
///
/// A question rather than a verdict, exactly as [`crate::signature`]'s attestations are: the
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
    /// that the page resides in the associated PDF file" — so `None` means *this* document, and
    /// `Some` is a second file this program has no filesystem to open, named rather than
    /// silently ignored.
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
