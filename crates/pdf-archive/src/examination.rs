//! What a predicate is handed: the document, the target, and one shared survey.
//!
//! # Why this exists, with the number that made it necessary
//!
//! A requirement's predicate used to take `(&Document, &mut Findings)`, so a rule that needed
//! shared work had to redo it. A full PDF/A-4 report over ISO 32000-2's own 1023-page
//! specification took **19 s**, which `CLAUDE.md` principle 2 makes a defect rather than a
//! detail.
//!
//! **Two guesses about where that time went were both wrong, and the third was measured** —
//! which is why `examples/cost.rs` exists and why these numbers are here rather than a belief.
//! Sharing the content survey took it to 10.5 s; sharing the object population moved it by
//! almost nothing (that walk is 346 ms once); and the actual cost was fifteen annotation, form
//! and action requirements each re-walking `/Annots` over a thousand pages, at 250 to 500 ms
//! apiece. Sharing that took the predicates from 8.8 s to 5.9 s and the report to about 7 s.
//!
//! So a predicate is handed this instead, and each shared thing is computed **at most once per
//! report and only where something asks for it**: a document whose target binds no
//! content-dependent rule never walks its content.
//!
//! What is left is not shared and is written down rather than hidden: several predicates build
//! their own page-level index — `Pages::new` and a pass over every page — and the two dearest
//! rules remaining are both of that shape. A `pages()` field here is the next step, and it is
//! not taken on a guess: `examples/cost.rs` will say whether it is worth it.
//!
//! # Why the survey is not cached across documents
//!
//! It is deliberately owned by the examination and dies with it. A cache keyed on a `&Document`
//! would be keyed on a pointer, and a validator that answered about the wrong file because an
//! allocator reused an address is worse than a slow one by a margin no benchmark measures.
//!
//! # Why the target rides here rather than in a second `Check` variant
//!
//! Because the rules that need it are not rare enough to be an exception: the file header's
//! version digit, the `ToUnicode` exemption lists, and every device-colour licence that part 4
//! states and part 2 does not. One signature that carries everything a predicate could want is
//! simpler than four variants naming the combinations, and it costs a predicate that wants
//! neither exactly one field access it never makes.

use std::cell::OnceCell;

use pdf_syntax::{Dictionary, Document, Object, ObjectId};

use crate::survey::Survey;
use crate::target::Target;

/// One document, held to one target, with the shared work computed on demand.
#[derive(Debug)]
pub struct Examination<'a> {
    /// The document being judged.
    pub document: &'a Document,
    /// The target it is being held to.
    ///
    /// Read by the rules whose *content* differs by part rather than only their citation.
    pub target: Target,
    /// The content survey, computed by the first predicate that asks and shared by the rest.
    survey: OnceCell<Survey>,
    /// Every object a cross-reference section names, fetched once.
    objects: OnceCell<Vec<(ObjectId, Object)>>,
    /// Every annotation of every page, in page order, resolved once.
    annotations: OnceCell<Vec<Annotation>>,
    /// Every page, with inheritance applied, walked once.
    pages: OnceCell<Vec<pdf_model::Page>>,
}

/// One annotation, with the page it is on and the object it was reached through.
///
/// The identity is kept because a report has to name where a fault is, and
/// `pdf_model::retrieval::annotations` throws it away — a viewer wants the dictionary, a
/// validator wants to be able to say *which* dictionary.
#[derive(Debug, Clone)]
pub struct Annotation {
    /// The zero-based page it is on.
    pub page: usize,
    /// The object it was reached through, where it is an indirect one.
    pub id: Option<ObjectId>,
    /// The annotation dictionary itself.
    pub dict: Dictionary,
}

impl<'a> Examination<'a> {
    /// Begins an examination. Nothing is walked until something asks for it.
    #[must_use]
    pub const fn new(document: &'a Document, target: Target) -> Self {
        Self {
            document,
            target,
            survey: OnceCell::new(),
            objects: OnceCell::new(),
            annotations: OnceCell::new(),
            pages: OnceCell::new(),
        }
    }

    /// The content survey, walked once however many requirements ask for it.
    pub fn survey(&self) -> &Survey {
        self.survey.get_or_init(|| Survey::of(self.document))
    }

    /// Every object a cross-reference section names, fetched once and lent to every rule.
    ///
    /// **The population the requirements bind**, and it is the cross-reference table's rather
    /// than a traversal's for the reason both parts give: ISO 19005-2 section 6.1.4 and ISO 19005-4
    /// Section 6.1.4 exempt an indirect object no cross-reference section names.
    ///
    /// Fetched once for the same reason the survey is. Twelve requirements walked this
    /// separately, at roughly 180 ms each on ISO 32000-2's 110 000 objects, and the repetition
    /// was most of what a report on that file cost after the survey was shared.
    pub fn objects(&self) -> &[(ObjectId, Object)] {
        self.objects.get_or_init(|| {
            self.document
                .xref()
                .object_numbers()
                .map(|number| {
                    let id = ObjectId::new(number, 0);
                    (id, self.document.get(id))
                })
                .collect()
        })
    }

    /// Every page of the document, with §7.7.3.4's inheritance applied, walked once.
    ///
    /// **Added on a guess that measurement then refuted, and kept for a different reason.**
    /// `examples/cost.rs` put `implementation-limits/page-boundary-sizes` second dearest at
    /// 153 ms and the page tree looked like the cause; sharing the walk moved it to 146 ms. The
    /// cost is elsewhere — that rule follows §7.7.3.4's `/Parent` chain for five boundaries on
    /// every page, which is the inheritance rather than the enumeration.
    ///
    /// It stays because it is right for the other reason: half a dozen rules and
    /// [`Self::annotations`] all wanted the same enumeration, and one of them having it twice
    /// is worse than all of them having it once. It is the third guess about this crate's cost
    /// to be wrong, which is why `cost.rs` exists and why this comment records the refutation
    /// rather than the hypothesis.
    pub fn pages(&self) -> &[pdf_model::Page] {
        self.pages.get_or_init(|| {
            let pages = pdf_model::Pages::new(self.document);
            (0..pages.len()).filter_map(|at| pages.get(at)).collect()
        })
    }

    /// Every annotation of every page, resolved once.
    ///
    /// **The measurement that put this here.** Fifteen requirements of the annotation, form and
    /// action tranches each walked `/Annots` over every page: on ISO 32000-2's 1023 pages that
    /// was 250 to 500 ms apiece and 8.8 s of a 10 s report, while the object population was
    /// 338 ms and the content survey 824 ms. `examples/cost.rs` prints the breakdown, and it is
    /// the reason this is a field rather than a helper — two earlier guesses about where the
    /// time went were both wrong, and the third was measured.
    ///
    /// A duplicate reference within one page's array is visited once: a producer that lists the
    /// same annotation twice has stated one annotation, and reporting it twice would count a
    /// fault against a document that has one of them.
    pub fn annotations(&self) -> &[Annotation] {
        self.annotations.get_or_init(|| {
            let mut out = Vec::new();
            for (page, sheet) in self.pages().iter().enumerate() {
                let listed = self.document.get_key(&sheet.dict, "Annots");
                let Some(items) = listed.as_array() else {
                    continue;
                };
                let mut seen = std::collections::BTreeSet::new();
                for item in items {
                    let id = item.as_reference();
                    if let Some(id) = id
                        && !seen.insert(id)
                    {
                        continue;
                    }
                    if let Some(dict) = self.document.resolve(item).as_dict() {
                        out.push(Annotation {
                            page,
                            id,
                            dict: dict.clone(),
                        });
                    }
                }
            }
            out
        })
    }
}
