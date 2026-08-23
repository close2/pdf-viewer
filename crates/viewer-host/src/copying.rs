//! §14.8.2.5's two orders, and which of them the text leaving the program is in.
//!
//! **The decision, not the clipboard.** Putting a selection where another application can take it
//! is a *platform* surface and there are four of them in this tree —
//! `gdk::Clipboard`, `QClipboard`, `arboard` for the host that has no toolkit to ask, and, for a C
//! caller, whatever that caller's own program uses. None of those is shared and none of them
//! belongs in [`viewer_core`], which has no opinion about chrome by construction. What *is* shared
//! is the question they all have to answer first: a selection is measured in page content order,
//! and the text a person meant to copy is often in the other one.
//!
//! ISO 32000-2 §14.8.2.5.1 states both orders:
//!
//! > Page content order shall be defined by the sequencing of graphics objects within a page's
//! > content stream.
//!
//! > Logical content order -the ordering for semantic purposes -shall be defined by a depth-first
//! > traversal of the document's logical structure
//!
//! and says the two "should coincide" without requiring it, which is exactly why a copy has to
//! choose. [`viewer_core::Query::Selection`] answers in the first — it is the order the shapes are
//! in, so it is the right answer for *drawing* — and
//! [`viewer_core::Query::LogicalSelection`] answers in the second where the document's structure
//! tree reaches every byte of what is selected.
//!
//! **This was `viewer-ui`'s private function until the six-hundred-and-eighty-third session**
//! (ADR 0519), which is the same sentence [`crate::clock`] and [`crate::presentation`] carry: the
//! third copy of a decision is where two hosts stop agreeing. Here it is one function with one
//! test, and each host supplies only the platform call.

use core::fmt;

/// Which of §14.8.2.5's two orders a copy's text is in.
///
/// A value rather than a sentence, because three hosts and a C caller say it in three languages
/// and one of them cannot read a `&str` this crate owns. [`ContentOrder::stated`] is the wording
/// the two hosts that print a note use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentOrder {
    /// §14.8.2.5.1's logical content order — the document's structure tree, depth first.
    Logical,
    /// §14.8.2.5.1's page content order — the sequence of the content stream's graphics objects.
    PageContent,
}

impl ContentOrder {
    /// How a host says which order it got, for a note a person reads.
    #[must_use]
    pub const fn stated(self) -> &'static str {
        match self {
            Self::Logical => "§14.8.2.5's logical content order",
            Self::PageContent => "page content order",
        }
    }
}

impl fmt::Display for ContentOrder {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.stated())
    }
}

/// What a copy carries, and which of §14.8.2.5's orders it is in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Copied {
    /// The characters, in [`Copied::order`].
    pub text: String,
    /// Which order they are in, which is a thing to say rather than a thing to hide.
    pub order: ContentOrder,
}

/// The text a copy should carry, from the two answers, or nothing to copy at all.
///
/// `logical` is [`viewer_core::Query::LogicalSelection`]'s answer and `page_order` is
/// [`viewer_core::Query::Selection`]'s text. Three cases and each is a decision rather than a
/// fallback:
///
/// - **Nothing selected is nothing copied**, and whatever is already on the platform's clipboard
///   is left there. A copy that quietly emptied it would lose a person's paste.
/// - **The logical order, where the core could give one**, which is the point of the second
///   question: §14.8.2.5 only *recommends* that the two coincide, and they differ on 5 of the
///   corpus's 77 tagged first pages.
/// - **Page content order otherwise**, said out loud rather than passed off as the other one. The
///   core answers nothing both for a document with no structure tree and for a tree that does not
///   reach every byte of the selection, and no host can tell those apart — what a host can do is
///   not claim an order it did not get.
#[must_use]
pub fn copied(logical: Option<String>, page_order: &str) -> Option<Copied> {
    if page_order.is_empty() {
        return None;
    }
    Some(match logical {
        Some(text) => Copied {
            text,
            order: ContentOrder::Logical,
        },
        None => Copied {
            text: page_order.to_owned(),
            order: ContentOrder::PageContent,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::{ContentOrder, Copied, copied};

    /// §14.8.2.5's two orders, and which one a copy takes.
    ///
    /// The clause gives a page a page content order and a logical content order and says they
    /// "should" coincide; `viewer_core::Query::LogicalSelection` answers with the second where the
    /// structure tree reaches every byte of the selection and with nothing otherwise. What a host
    /// owns is the platform call and the sentence it prints, because a person who copied a
    /// two-column page and got the columns interleaved has been told nothing at all.
    #[test]
    fn a_copy_takes_the_logical_order_where_the_core_could_give_one() {
        assert_eq!(
            copied(None, ""),
            None,
            "nothing selected leaves the clipboard alone"
        );
        assert_eq!(
            copied(Some("a tree that reaches it".to_owned()), ""),
            None,
            "and does so even if the tree would have answered"
        );
        assert_eq!(
            copied(Some("logical".to_owned()), "page"),
            Some(Copied {
                text: "logical".to_owned(),
                order: ContentOrder::Logical,
            }),
            "a rearrangement the core could make is the one a copy wants"
        );
        assert_eq!(
            copied(None, "page"),
            Some(Copied {
                text: "page".to_owned(),
                order: ContentOrder::PageContent,
            }),
            "and the order it did not get is not claimed"
        );
    }

    /// The order a copy got is a value, and every consumer says it the same way.
    #[test]
    fn each_order_states_itself() {
        assert_eq!(
            ContentOrder::Logical.stated(),
            "§14.8.2.5's logical content order"
        );
        assert_eq!(ContentOrder::PageContent.stated(), "page content order");
        assert_eq!(
            ContentOrder::PageContent.to_string(),
            ContentOrder::PageContent.stated(),
            "the display is the same sentence rather than a second wording"
        );
    }
}
