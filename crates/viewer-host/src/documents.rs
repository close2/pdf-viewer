//! The documents one window has open, in the order it shows them, and which of them is focused.
//!
//! # Why a host needs anything at all here
//!
//! [`viewer_core::Viewer`] has held a map of [`viewer_core::DocumentId`] to open documents since
//! it existed, and [`viewer_core::Command::Focus`] has said which of them commands apply to for
//! just as long. What was missing was on this side: every window put one name in the map and
//! never a second, so a remote go-to asking for a new window, a `GoToE` into an embedded file and
//! a person naming two files on the command line all came to the same sentence — *this view has
//! one*.
//!
//! What a window needs in order to hold two is a very small amount of state and a very exact set
//! of rules about it: which names are open, what each tab is called, which one is in front, what
//! closing one does to that, and where the state a window keeps *per document* goes while the
//! person is reading the other. Those rules are identical in a `gtk4::Notebook`, a `QTabWidget`
//! and a strip of rectangles a program draws for itself — and the third copy of them is where two
//! hosts stop agreeing, which is the test this crate applies to everything in it.
//!
//! # What is a host's, and what is not
//!
//! The *widget* is the host's: a notebook page, a tab bar, a row of labels over a surface. So is
//! everything in `T` — each host keeps a different set of facts about the document it is showing,
//! and pretending otherwise would be an invention rather than a finding (ADR 1264 lists them side
//! by side).
//!
//! What is here is the bookkeeping neither toolkit supplies and both need to agree about:
//!
//! - **One `T` per document, and exactly one of them is not here.** The focused document's state
//!   lives in the host's own fields, because that is what every line of a window already reads;
//!   the rest are parked beside their names. [`Documents::focus`] swaps the two in one move, so a
//!   field that must not leak between tabs cannot be left behind by forgetting it at a call site —
//!   the whole of `T` travels or none of it does.
//! - **Names increase and are never reused.** [`Documents::reserve`] answers the next one, which
//!   is what [`viewer_core::Command::Beside`] holds out for a document an action opens. A name
//!   reserved and not used is simply skipped: [`viewer_core::DocumentId`] is a `u64` and this
//!   program will not run out of them.
//! - **The last document is not closed here.** [`Close::Last`] says so and leaves the act to the
//!   host, because closing the last tab is closing the *window*, and what that means is a
//!   `GtkWindow::close` against a `QWidget::close` against a loop that stops — which is what a
//!   toolkit is.
//!
//! ADR 1264.

use viewer_core::DocumentId;

/// One document a window is holding, as the tab strip sees it.
#[derive(Debug)]
struct Tab<T> {
    /// What this window calls the document, and what every message about it names.
    id: DocumentId,
    /// What the tab says — a file's name rather than its path, so that a strip of them fits.
    label: String,
    /// What the host keeps about this document while somebody else is in front.
    ///
    /// `None` for exactly one tab, the focused one, whose state is in the host's own fields.
    /// [`Documents::focus`] is the only thing that moves it, and the invariant is what makes
    /// "the window's fields are the focused document's" a statement a reader can rely on rather
    /// than a convention.
    parked: Option<T>,
}

/// What closing a document came to.
#[derive(Debug)]
pub enum Close<T> {
    /// No document of that name is open in this window.
    Unknown,
    /// The only document open. Closing it is closing the window, which is the host's to do.
    Last,
    /// Closed.
    Done {
        /// What the closed document's host state was.
        ///
        /// Handed back rather than dropped here, because a host may have a thread, a texture
        /// cache or a file handle in it that has to be given up in a particular order.
        state: T,
        /// The document now in front, which the host shows.
        ///
        /// Where the closed document was the focused one, the state handed to [`Documents::close`]
        /// now holds *this* document's and the caller draws it; where it was not, that state was
        /// not touched and this names the document already in front.
        focused: DocumentId,
    },
}

/// The documents one window has open, in tab order, and which of them is in front.
///
/// Never empty: a window with nothing open has no [`Documents`] rather than an empty one, which is
/// why [`Documents::new`] takes the first document and [`Documents::close`] refuses the last.
#[derive(Debug)]
pub struct Documents<T> {
    /// In the order the tabs are shown, which is the order they were opened in.
    tabs: Vec<Tab<T>>,
    /// Which of them is in front. Always a valid index into `tabs`.
    focused: usize,
    /// The next name this window will hand out, which only ever increases.
    next: u64,
}

impl<T> Documents<T> {
    /// A window showing one document, which is how every window on this boundary starts.
    ///
    /// The identity is the caller's rather than this type's: two of the four windows in this tree
    /// call their first document `DocumentId(1)` and two call it `DocumentId(0)`, and changing
    /// that would change the first message on a launch path `CLAUDE.md` measures.
    #[must_use]
    pub fn new(first: DocumentId, label: String) -> Self {
        Self {
            tabs: vec![Tab {
                id: first,
                label,
                parked: None,
            }],
            focused: 0,
            next: first.0.saturating_add(1),
        }
    }

    /// The next name this window has free, without opening anything under it.
    ///
    /// What [`viewer_core::Command::Beside`] holds out for a document §12.6.4.3's or §12.6.4.4's
    /// `/NewWindow true` asks for. A name this answers and nothing is opened under is skipped
    /// rather than handed out twice, which is the property that makes an unused offer harmless.
    pub fn reserve(&mut self) -> DocumentId {
        let id = DocumentId(self.next);
        self.next = self.next.saturating_add(1);
        id
    }

    /// Puts a document this window has just opened at the end of the strip, parked.
    ///
    /// The caller has the state because it has just built it; focus does not move, so a host that
    /// wants the new tab in front says so with [`Documents::focus`] and sends
    /// [`viewer_core::Command::Focus`] with it.
    ///
    /// A name already open is **not** added twice: the tab is relabelled and the state handed in
    /// is given back, which is [`viewer_core::Command::Open`]'s own rule about opening twice under
    /// one identity, said in this window's vocabulary.
    pub fn add(&mut self, id: DocumentId, label: String, state: T) -> Option<T> {
        if let Some(index) = self.index_of(id) {
            self.tabs[index].label = label;
            return Some(state);
        }
        // A name a host opened for itself may be above anything this type has handed out, and the
        // next reservation must not collide with it.
        self.next = self.next.max(id.0.saturating_add(1));
        self.tabs.push(Tab {
            id,
            label,
            parked: Some(state),
        });
        None
    }

    /// Brings a document to the front, swapping the host's fields for that document's.
    ///
    /// `current` holds the focused document's state on the way in and the named document's on the
    /// way out. Answers whether anything moved: a name that is not open here, or one that is
    /// already in front, leaves `current` exactly as it was.
    pub fn focus(&mut self, id: DocumentId, current: &mut T) -> bool {
        let Some(to) = self.index_of(id) else {
            return false;
        };
        if to == self.focused {
            return false;
        }
        // `parked` is `Some` for every tab that is not the focused one, and `to` is not the
        // focused one — so this cannot be `None`. It is written as a pattern rather than an
        // `expect` because the invariant is this type's to keep, and a broken one leaving the
        // window on the document it was already showing is a better answer than a panic in a
        // toolkit's event handler.
        let Some(taken) = self.tabs[to].parked.take() else {
            return false;
        };
        let from = self.focused;
        self.tabs[from].parked = Some(std::mem::replace(current, taken));
        self.focused = to;
        true
    }

    /// The document after the focused one, wrapping round to the first.
    ///
    /// A choice and written down as one: the standard says nothing whatever about moving between
    /// documents, because moving between them is a user interface.
    #[must_use]
    pub fn next_id(&self) -> DocumentId {
        // `tabs` is never empty — `new` takes the first document and `close` refuses the last —
        // and the wrap is written as a comparison rather than a remainder so that nothing here
        // divides at all.
        let after = self.focused.saturating_add(1);
        let index = if after < self.tabs.len() { after } else { 0 };
        self.tabs
            .get(index)
            .map_or_else(|| self.focused(), |tab| tab.id)
    }

    /// Closes a document, and says what the window shows afterwards.
    ///
    /// `current` holds the focused document's state on the way in; where the document closed *was*
    /// the focused one it holds the newly focused document's on the way out, and the closed one's
    /// comes back in [`Close::Done`]. Where it was not, `current` is untouched.
    ///
    /// The tab to the right takes the front, or the one to the left where there is nothing to the
    /// right — which is what every strip of tabs does and what no clause states.
    pub fn close(&mut self, id: DocumentId, current: &mut T) -> Close<T> {
        let Some(index) = self.index_of(id) else {
            return Close::Unknown;
        };
        if self.tabs.len() == 1 {
            return Close::Last;
        }
        if index == self.focused {
            // The neighbour that takes the front, named before the removal shifts the indices.
            let neighbour = if index.saturating_add(1) < self.tabs.len() {
                index.saturating_add(1)
            } else {
                index.saturating_sub(1)
            };
            let Some(taken) = self.tabs[neighbour].parked.take() else {
                return Close::Unknown;
            };
            let state = std::mem::replace(current, taken);
            self.tabs.remove(index);
            self.focused = if neighbour > index {
                neighbour.saturating_sub(1)
            } else {
                neighbour
            };
            return Close::Done {
                state,
                focused: self.tabs[self.focused].id,
            };
        }
        let Some(state) = self.tabs.remove(index).parked else {
            return Close::Unknown;
        };
        if index < self.focused {
            self.focused = self.focused.saturating_sub(1);
        }
        Close::Done {
            state,
            focused: self.tabs[self.focused].id,
        }
    }

    /// The document in front, which is the one every message this window sends is about.
    #[must_use]
    pub fn focused(&self) -> DocumentId {
        self.tabs[self.focused].id
    }

    /// Where the document in front sits in the strip.
    #[must_use]
    pub const fn focused_index(&self) -> usize {
        self.focused
    }

    /// How many documents this window is holding.
    ///
    /// There is no `is_empty` beside it and there never will be: this type cannot be empty, since
    /// [`Self::new`] takes the first document and [`Self::close`] refuses the last. What a caller
    /// wants instead is [`Self::is_alone`].
    #[must_use]
    #[expect(
        clippy::len_without_is_empty,
        reason = "a window with nothing open holds no `Documents` at all, so the question \
                  `is_empty` asks has no answer here; `is_alone` is the one a strip of tabs asks"
    )]
    pub fn len(&self) -> usize {
        self.tabs.len()
    }

    /// Whether this window is showing one document, which is what a window without tabs is.
    #[must_use]
    pub fn is_alone(&self) -> bool {
        self.tabs.len() == 1
    }

    /// Where a name sits in the strip, or nothing where it is not open here.
    #[must_use]
    pub fn index_of(&self, id: DocumentId) -> Option<usize> {
        self.tabs.iter().position(|tab| tab.id == id)
    }

    /// The name at a position in the strip — what a click on the nth tab is about.
    #[must_use]
    pub fn id_at(&self, index: usize) -> Option<DocumentId> {
        self.tabs.get(index).map(|tab| tab.id)
    }

    /// Every document, in tab order: what it is called here and what its tab says.
    pub fn iter(&self) -> impl Iterator<Item = (DocumentId, &str)> {
        self.tabs.iter().map(|tab| (tab.id, tab.label.as_str()))
    }

    /// Changes what one tab says, for a host that has learnt the document's own title.
    pub fn relabel(&mut self, id: DocumentId, label: String) {
        if let Some(index) = self.index_of(id) {
            self.tabs[index].label = label;
        }
    }
}

/// What a tab says, from the path the file came from.
///
/// The file's name rather than its path, because a strip of tabs is measured in characters and a
/// path is measured in directories; the whole path is what the window's title says, which is the
/// division every program with tabs makes. A path with no final component — which is a directory,
/// and not something this program opens — falls back to the whole of it rather than to nothing.
#[must_use]
pub fn label(path: &std::path::Path) -> String {
    path.file_name().map_or_else(
        || path.display().to_string(),
        |name| name.to_string_lossy().into_owned(),
    )
}

/// What a window says when a document opens beside the one that was showing.
///
/// Worded once for three windows for [`crate::status`]'s reason: the third copy of a sentence is
/// where two hosts stop agreeing about what they are saying. The count is in it because a person
/// who has just acquired a second tab is owed the fact that they have two.
#[must_use]
pub fn opened_beside(label: &str, open: usize) -> String {
    format!("{label} opened beside — {open} document(s) open")
}

/// What a window says when a tab is closed.
#[must_use]
pub fn closed(label: &str, open: usize) -> String {
    format!("closed {label} — {open} document(s) open")
}

#[cfg(test)]
mod tests {
    use super::{Close, Documents, closed, label, opened_beside};
    use viewer_core::DocumentId;

    /// The focused document's state is the host's and every other document's is parked, which is
    /// the invariant every other method rests on.
    #[test]
    fn exactly_one_document_has_its_state_out_in_the_window() {
        let mut documents = Documents::new(DocumentId(1), "first.pdf".to_owned());
        assert!(documents.is_alone());
        assert_eq!(documents.focused(), DocumentId(1));

        let second = documents.reserve();
        assert_eq!(second, DocumentId(2));
        assert!(documents.add(second, "second.pdf".to_owned(), 22).is_none());
        assert_eq!(documents.len(), 2);
        assert!(!documents.is_alone());
        assert_eq!(
            documents.focused(),
            DocumentId(1),
            "adding a tab does not move the front"
        );

        let mut current = 11;
        assert!(documents.focus(second, &mut current));
        assert_eq!(current, 22, "the window is now holding the second's state");
        assert_eq!(documents.focused(), second);

        assert!(
            !documents.focus(second, &mut current),
            "focusing what is already in front moves nothing"
        );
        assert_eq!(current, 22);
        assert!(!documents.focus(DocumentId(9), &mut current));
        assert_eq!(current, 22, "and a name that is not open leaves it alone");

        assert!(documents.focus(DocumentId(1), &mut current));
        assert_eq!(current, 11, "the first's state came back whole");
    }

    /// A name is handed out once, and a name a host opened for itself is not handed out again.
    #[test]
    fn names_only_ever_increase() {
        let mut documents = Documents::new(DocumentId(0), "a".to_owned());
        assert_eq!(documents.reserve(), DocumentId(1));
        assert_eq!(documents.reserve(), DocumentId(2));
        // A reservation nothing was opened under is skipped rather than reused.
        assert!(documents.add(DocumentId(7), "b".to_owned(), ()).is_none());
        assert_eq!(
            documents.reserve(),
            DocumentId(8),
            "a name opened above the counter pushes it past"
        );
    }

    /// Opening twice under one name relabels rather than adding a second tab, which is
    /// `Command::Open`'s own rule said in a window's vocabulary.
    #[test]
    fn one_name_is_one_tab() {
        let mut documents = Documents::new(DocumentId(1), "a".to_owned());
        assert!(documents.add(DocumentId(2), "b".to_owned(), 2).is_none());
        assert_eq!(documents.add(DocumentId(2), "c".to_owned(), 3), Some(3));
        assert_eq!(documents.len(), 2);
        assert_eq!(documents.iter().last(), Some((DocumentId(2), "c")));
    }

    /// Closing the focused tab hands the window the neighbour's state in the same move.
    #[test]
    fn closing_the_front_tab_puts_its_neighbour_there() {
        let mut documents = Documents::new(DocumentId(1), "a".to_owned());
        documents.add(DocumentId(2), "b".to_owned(), 2);
        documents.add(DocumentId(3), "c".to_owned(), 3);
        let mut current = 1;

        // The tab to the right takes the front.
        match documents.close(DocumentId(1), &mut current) {
            Close::Done { state, focused } => {
                assert_eq!(state, 1, "the closed document's state came back");
                assert_eq!(focused, DocumentId(2));
                assert_eq!(current, 2, "and the window is holding the new front's");
            }
            other => panic!("{other:?}"),
        }

        // A tab that is not in front closes without touching what the window is holding.
        match documents.close(DocumentId(3), &mut current) {
            Close::Done { state, focused } => {
                assert_eq!(state, 3);
                assert_eq!(focused, DocumentId(2));
                assert_eq!(current, 2);
            }
            other => panic!("{other:?}"),
        }

        assert!(matches!(
            documents.close(DocumentId(2), &mut current),
            Close::Last
        ));
        assert!(matches!(
            documents.close(DocumentId(9), &mut current),
            Close::Unknown
        ));
    }

    /// Closing the last tab in the strip falls back to the one on its left.
    #[test]
    fn closing_the_rightmost_tab_falls_back_leftwards() {
        let mut documents = Documents::new(DocumentId(1), "a".to_owned());
        documents.add(DocumentId(2), "b".to_owned(), 2);
        let mut current = 1;
        assert!(documents.focus(DocumentId(2), &mut current));
        match documents.close(DocumentId(2), &mut current) {
            Close::Done { state, focused } => {
                assert_eq!(state, 2);
                assert_eq!(focused, DocumentId(1));
                assert_eq!(current, 1);
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(documents.focused_index(), 0);
    }

    /// The next document wraps, because a strip of tabs has two ends and no clause states one.
    #[test]
    fn the_next_document_wraps_round() {
        let mut documents = Documents::new(DocumentId(1), "a".to_owned());
        assert_eq!(
            documents.next_id(),
            DocumentId(1),
            "one document is its own next"
        );
        documents.add(DocumentId(2), "b".to_owned(), ());
        assert_eq!(documents.next_id(), DocumentId(2));
        let mut current = ();
        documents.focus(DocumentId(2), &mut current);
        assert_eq!(documents.next_id(), DocumentId(1));
    }

    /// A tab says the file's name and the sentences name the count.
    #[test]
    fn the_wording_names_the_file_and_how_many_are_open() {
        assert_eq!(
            label(std::path::Path::new("/tmp/a/report.pdf")),
            "report.pdf"
        );
        assert_eq!(label(std::path::Path::new("report.pdf")), "report.pdf");
        assert!(opened_beside("b.pdf", 2).contains("b.pdf"));
        assert!(opened_beside("b.pdf", 2).contains('2'));
        assert!(closed("b.pdf", 1).contains("closed b.pdf"));
    }
}
