//! A document that holds none of the pages it states, refused by name (`doc/questions/A775`,
//! `doc/adr/2670`).
//!
//! ISO 32000-2 §7.7.3.2's Table 30 makes a page tree node's `/Count` "( Required ) The number of
//! leaf nodes (page objects) that are descendants of this node within the page tree", and Annex
//! F.3.3's Table F.1 makes a linearization parameter dictionary's `/N` "The number of pages in
//! the document". A file whose root states one page or more and from which this reader draws
//! none has broken the count, and it holds nothing a reader saw: the owner's answer A775 refuses
//! it under every profile, by the name of the loss `page-not-held` and with both counts, rather
//! than writing an archive of no page or of a statement alone.
//!
//! **One sentence, two readers.** The checker records it as the finding of
//! `file-structure/page-tree-holds-a-page-it-counts`, and the converter refuses the source with it
//! before it rebuilds or decides anything, so a person reading either reads the same words.
//! Neither judges a count that disagrees with a tree holding *some* pages: that is the loss
//! `page-not-held` a profile may take (`doc/adr/2596`), and the `/Count` row's narrowing to
//! presence and type (ADR 2456) stands.

use std::fmt;

use pdf_syntax::Document;
use pdf_syntax::object::{Object, ObjectId};

/// Where the count is read from when it is the page tree root's (`doc/adr/2596`'s wording).
pub const STATED_BY_THE_ROOT: &str = "its page tree root's /Count";

/// Where the count is read from when it is a linearization parameter dictionary's.
pub const STATED_BY_LINEARIZATION: &str = "its linearization parameter dictionary's /N";

/// A document stating one page or more and holding none of them: the count and where it is
/// stated. Its [`fmt::Display`] is the sentence the checker and the converter both say.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NoPageHeld {
    /// The statement the count is read from: [`STATED_BY_THE_ROOT`] or
    /// [`STATED_BY_LINEARIZATION`].
    pub stated_by: &'static str,
    /// The number of pages it states, one or more.
    pub stated: usize,
}

impl NoPageHeld {
    /// Whether `document` states a page and holds none, by this reader's own pages.
    ///
    /// The pages are `pdf_model::Pages`'s, the ones the viewer draws, asked one at a time until
    /// the first is found, so a document holding its first page costs one page's reading.
    #[must_use]
    pub fn of(document: &Document) -> Option<Self> {
        let pages = pdf_model::Pages::new(document);
        let holds_a_page = (0..pages.len()).any(|index| pages.get(index).is_some());
        Self::given(document, holds_a_page)
    }

    /// [`Self::of`], where the caller has already read whether the document holds a page.
    ///
    /// The page tree root's `/Count`, where the catalog names a root, is the statement read; a
    /// linearization parameter dictionary's `/N` only where the root states none. The search for
    /// that dictionary reads every object the cross-reference information names, which is why it
    /// runs only for a document already found to hold no page.
    #[must_use]
    pub fn given(document: &Document, holds_a_page: bool) -> Option<Self> {
        if holds_a_page {
            return None;
        }
        let as_count = |value: &Object| value.as_integer().and_then(|n| usize::try_from(n).ok());
        let root_count = document
            .catalog()
            .ok()
            .and_then(|catalog| catalog.get("Pages").cloned())
            .map(|root| document.resolve(&root))
            .and_then(|root| as_count(&document.get_key(root.as_dict()?, "Count")));
        let stated = match root_count {
            Some(count) => Some((STATED_BY_THE_ROOT, count)),
            None => document.xref().object_numbers().find_map(|number| {
                let object = document.get(ObjectId::new(number, 0));
                let dict = object.as_dict()?;
                dict.get("Linearized")?;
                Some((
                    STATED_BY_LINEARIZATION,
                    as_count(&document.get_key(dict, "N"))?,
                ))
            }),
        };
        let (stated_by, stated) = stated.filter(|(_, stated)| *stated > 0)?;
        Some(Self { stated_by, stated })
    }

    /// The clause the count is stated in.
    #[must_use]
    pub fn clause(&self) -> &'static str {
        if self.stated_by == STATED_BY_LINEARIZATION {
            "ISO 32000-2 F.3.3, Table F.1"
        } else {
            "ISO 32000-2 7.7.3.2, Table 30"
        }
    }
}

impl fmt::Display for NoPageHeld {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "page-not-held: {} states {} {} and the file holds 0 of them ({}), so it holds \
             nothing a reader sees to archive (doc/questions/A775)",
            self.stated_by,
            self.stated,
            if self.stated == 1 { "page" } else { "pages" },
            self.clause(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sentence_names_the_loss_both_counts_and_the_clause() {
        let one = NoPageHeld {
            stated_by: STATED_BY_THE_ROOT,
            stated: 1,
        };
        assert_eq!(
            one.to_string(),
            "page-not-held: its page tree root's /Count states 1 page and the file holds 0 of \
             them (ISO 32000-2 7.7.3.2, Table 30), so it holds nothing a reader sees to archive \
             (doc/questions/A775)"
        );
        let many = NoPageHeld {
            stated_by: STATED_BY_LINEARIZATION,
            stated: 429,
        };
        assert!(
            many.to_string()
                .contains("/N states 429 pages and the file holds 0")
                && many.to_string().contains("F.3.3, Table F.1"),
            "{many}"
        );
    }
}
