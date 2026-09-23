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
    /// What the tab says — [`titled`]'s answer, a document's own title or its file's name, and
    /// never a path, so that a strip of them fits.
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

    /// What the host keeps about a document that is not in front, to add to while it waits.
    ///
    /// `None` for the focused document — whose state is the host's own fields — and for a name
    /// that is not open here. What a host puts here is something the core sent *about* a tab
    /// behind the front: a page drawn for a document opened beside the one showing that was given
    /// the front back before the drawing arrived (ADR 1275).
    pub fn parked_mut(&mut self, id: DocumentId) -> Option<&mut T> {
        let index = self.index_of(id)?;
        self.tabs[index].parked.as_mut()
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

/// What a tab says: ISO 32000-2 §14.3.3's `/Title`, where the document states one, and otherwise
/// the file's name.
///
/// Table 349's row is "[t]he document's title", a text string, and the clause states what makes a
/// value one:
///
/// > Where a document information dictionary contains keys other than CreationDate and ModDate ,
/// > the value associated with any such key shall be a text string.
///
/// So a `/Title` that is not a string has stated no title, and [`pdf_model::metadata::Information`]
/// has already answered `None` for it; the §7.9.2.2 decoding — `PDFDocEncoding`, or UTF-16 behind a
/// byte order mark — is `pdf_syntax::text_string`, which that type reads through, so there is one
/// decoder in the tree and this is not a second.
///
/// **What no clause states, and is a choice**: that a tab shows this at all. §12.2's
/// `/DisplayDocTitle` is about the window's *title bar* and names XMP's `dc:title`; a strip of
/// tabs is a user interface the standard does not describe. The title bar keeps obeying
/// `/DisplayDocTitle` where a window has one, and the tab is the cheaper of the two tables —
/// Table 349 is a dictionary lookup where §14.3.2's stream is a decode and a parse — so a name on
/// the strip costs no document anything to show. A title that is empty or only white space names
/// nothing a person can read on a tab, and gets the file's name, which is also what every document
/// stating no `/Info` gets (ADR 1275).
#[must_use]
pub fn titled(information: &pdf_model::metadata::Information, path: &std::path::Path) -> String {
    information
        .title
        .as_deref()
        .map(str::trim)
        .filter(|title| !title.is_empty())
        .map_or_else(|| label(path), str::to_owned)
}

/// A document named on a command line or chosen by a person: the file, and Annex O's fragment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Named {
    /// The file.
    pub path: std::path::PathBuf,
    /// Annex O's fragment — the text after `#`, undecoded, because splitting a URI is the host's
    /// and percent-decoding belongs to whoever knows which component it is decoding (ADR 0209).
    pub fragment: Option<String>,
}

impl Named {
    /// A file with no fragment, which is what a file dialogue answers.
    #[must_use]
    pub fn file(path: std::path::PathBuf) -> Self {
        Self {
            path,
            fragment: None,
        }
    }

    /// One document word from a command line, with Annex O's fragment split off.
    ///
    /// Annex O's fragment is the text after `#` in the URI the bytes came from. A path is not a
    /// URI, but a path with a `#` in it is how a person types one on a command line.
    ///
    /// **The filesystem decides, not the punctuation**, and that is this program's choice rather
    /// than anything the annex says. A `#` is an ordinary character in a file name on every system
    /// this program runs on, so a word that names an existing file is taken whole; only when it
    /// does not is it split at its first `#`, which is where RFC 3986 puts the boundary. The cost
    /// is one `stat` per document named and a file called `a#b.pdf` that still opens; splitting
    /// first would make that file unopenable and say nothing. A `#` with nothing before it is
    /// handed on whole, so the read fails by name: a path that does not exist is a better message
    /// than a fragment nobody asked for. One reading for three windows, which is the argument
    /// every function in this crate is here on.
    #[must_use]
    pub fn from_argument(argument: &std::ffi::OsStr) -> Self {
        let whole = std::path::PathBuf::from(argument);
        if whole.exists() {
            return Self::file(whole);
        }
        let text = argument.to_string_lossy();
        match text.split_once('#') {
            Some((path, fragment)) if !path.is_empty() => Self {
                path: std::path::PathBuf::from(path),
                fragment: Some(fragment.to_owned()),
            },
            _ => Self::file(whole),
        }
    }

    /// A document that came out of another one, named where its tab and its sentences can say so.
    ///
    /// ISO 32000-2 §O.2.1's `ef` opens an embedded file out of the document's `/EmbeddedFiles` name
    /// tree, and that file has no path on this machine: its bytes are the holding document's. What it is given is a name beside the holding document — the directory
    /// §12.7.6.4's import and a save resolve against — made of the last component of the name the
    /// document filed it under, because that name is a string the *document* wrote (§7.11.4's
    /// `/F` is "a platform-dependent encoding") and a name that is a path is not followed. A name
    /// with no last component — empty, or `..` — is given the key's own words, which is a choice:
    /// no clause says what an embedded file with no usable name is called.
    #[must_use]
    pub fn embedded(
        directory: Option<&std::path::Path>,
        name: &str,
        fragment: Option<String>,
    ) -> Self {
        let single = std::path::Path::new(name).file_name().map_or_else(
            || std::path::PathBuf::from("embedded file"),
            std::path::PathBuf::from,
        );
        Self {
            path: directory.map_or_else(|| single.clone(), |directory| directory.join(&single)),
            fragment,
        }
    }
}

/// A document a reader named, on its way to a tab of its own.
///
/// Everything a window needs to answer the events that come back about a document that is not yet
/// in its strip: which name it was opened under, the file (for the sentence, and for §7.6.4.1's
/// second attempt), and its own count of password prompts — the one in the window's fields belongs
/// to the document in front.
#[derive(Debug)]
pub struct Arriving {
    /// The name it is being opened under, handed out by [`Documents::reserve`].
    pub id: DocumentId,
    /// The file and its fragment.
    pub named: Named,
    /// The document that goes back in front once this one has opened, where it was named *behind*
    /// the one showing — a second path on a command line, whose first path is the one the launch
    /// was for.
    pub behind: Option<DocumentId>,
    /// §7.6.4.1's prompts, counted for this document.
    pub asking: crate::Asking,
    /// The bytes, where the document is held in memory rather than named on disk — an embedded
    /// file §O.2.1's `ef` opened, whose [`Named::path`] is a name and not a file.
    held: Option<pdf_syntax::FileBytes>,
}

impl Arriving {
    /// The bytes to open it from: the ones held, or the file its path names.
    ///
    /// One function for the first open and §7.6.4.1's second attempt, so that a document held in
    /// memory is never looked for on disk under the name it was given.
    ///
    /// # Errors
    ///
    /// [`crate::open_chosen`]'s sentence, where the path names nothing that can be opened.
    pub fn bytes(&self) -> Result<pdf_syntax::FileBytes, String> {
        match &self.held {
            Some(bytes) => Ok(bytes.clone()),
            None => crate::open_chosen(&self.named.path),
        }
    }
}

/// The documents a window has been asked to open beside the one showing, one at a time.
///
/// **One at a time because a document may ask a question on its way in** — §7.6.4.1's password,
/// §12.11.6's requirements at the *ask* level — and a window asking about two documents at once
/// would be asking a person to know which prompt is which. So each waits until the one before it
/// has opened, failed or been declined, and [`Self::start`] is what a window calls at each of those
/// three moments and once after its first frame.
#[derive(Debug, Default)]
pub struct Arrivals {
    /// What is still to be opened, in the order it was named, whether each goes behind, and the
    /// bytes of one held in memory.
    waiting: std::collections::VecDeque<(Named, bool, Option<pdf_syntax::FileBytes>)>,
    /// The one being opened.
    now: Option<Arriving>,
}

impl Arrivals {
    /// A window with nothing to open.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a document to the end of the queue.
    ///
    /// `behind` is whether the tab in front stays in front once it has opened: `true` for the
    /// command line's later paths, `false` for a file a person has just chosen, who asked to read it.
    pub fn wait(&mut self, named: Named, behind: bool) {
        self.waiting.push_back((named, behind, None));
    }

    /// Adds a document held in memory to the end of the queue.
    ///
    /// §O.2.1's `ef`: the fragment asked for the embedded file to be opened, and the document
    /// holding it keeps its own tab rather than being replaced — ISO 32000-2 states no window rule,
    /// and replacing the document a person had open is the one choice that loses something.
    /// `behind` is the holding document's own: an embedded file named by the fragment of a document
    /// in front comes to the front, because a reader following that URI asked to read it, and one
    /// named by a document opened behind stays behind with it.
    pub fn wait_held(&mut self, named: Named, bytes: pdf_syntax::FileBytes, behind: bool) {
        self.waiting.push_back((named, behind, Some(bytes)));
    }

    /// The next document to open, where nothing is being opened and something is waiting.
    ///
    /// Reserves its name from the window's strip, which is what makes the name one no other tab
    /// has or will have.
    pub fn start<T>(&mut self, documents: &mut Documents<T>) -> Option<&Arriving> {
        if self.now.is_some() {
            return None;
        }
        let (named, behind, held) = self.waiting.pop_front()?;
        let arriving = Arriving {
            id: documents.reserve(),
            named,
            behind: behind.then(|| documents.focused()),
            asking: crate::Asking::new(),
            held,
        };
        Some(self.now.insert(arriving))
    }

    /// The document being opened, where one is.
    #[must_use]
    pub fn current(&self) -> Option<&Arriving> {
        self.now.as_ref()
    }

    /// Whether this is the name of the document being opened.
    #[must_use]
    pub fn is(&self, id: DocumentId) -> bool {
        self.now.as_ref().is_some_and(|now| now.id == id)
    }

    /// The document being opened, where the name is its name.
    pub fn named_mut(&mut self, id: DocumentId) -> Option<&mut Arriving> {
        self.now.as_mut().filter(|now| now.id == id)
    }

    /// Takes the document being opened out of the queue, where the name is its name.
    ///
    /// Called when it has opened, when it failed, and when a person declined it; the next one
    /// waiting is then [`Self::start`]'s.
    pub fn settle(&mut self, id: DocumentId) -> Option<Arriving> {
        if self.is(id) { self.now.take() } else { None }
    }
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

    /// Table 349's `/Title` is what a tab says where the document states one it can show.
    ///
    /// §14.3.3's own EXAMPLE is the second case: its document information dictionary holds "just
    /// the creation and last modification date", and its title is in the metadata stream only —
    /// so the tab is the file's name, and a reader that read `dc:title` for the strip would have
    /// written a different function.
    #[test]
    fn a_tab_says_the_documents_title_and_otherwise_its_file_name() {
        use pdf_model::metadata::Information;
        let path = std::path::Path::new("/tmp/a/report.pdf");
        let stated = Information {
            title: Some("Annual report 2014".to_owned()),
            ..Information::default()
        };
        assert_eq!(super::titled(&stated, path), "Annual report 2014");

        let example = Information {
            created: Some("D:20140314124211+01'00".to_owned()),
            modified: Some("D:20140924212303+02'00".to_owned()),
            ..Information::default()
        };
        assert_eq!(super::titled(&example, path), "report.pdf");

        for nothing in ["", "   "] {
            let blank = Information {
                title: Some(nothing.to_owned()),
                ..Information::default()
            };
            assert_eq!(
                super::titled(&blank, path),
                "report.pdf",
                "a title with nothing to read on it is not a name for a tab"
            );
        }
    }

    /// A fragment is split off a command-line word, undecoded, unless the whole word is a file.
    #[test]
    fn a_fragment_is_split_off_a_path_that_does_not_exist_whole() {
        use super::Named;
        let read = Named::from_argument(std::ffi::OsStr::new("doc/x.pdf#nameddest=A%26B"));
        assert_eq!(read.path, std::path::PathBuf::from("doc/x.pdf"));
        assert_eq!(read.fragment.as_deref(), Some("nameddest=A%26B"));
        let bare = Named::from_argument(std::ffi::OsStr::new("#page=2"));
        assert_eq!(bare, Named::file(std::path::PathBuf::from("#page=2")));

        let directory = std::env::temp_dir().join(format!("named-{}", std::process::id()));
        std::fs::create_dir_all(&directory).expect("a scratch directory");
        let odd = directory.join("a#b.pdf");
        std::fs::write(&odd, b"%PDF-2.0").expect("a scratch file");
        assert_eq!(
            Named::from_argument(odd.as_os_str()),
            Named::file(odd.clone()),
            "a file whose name has a # in it is that file"
        );
        std::fs::remove_dir_all(&directory).expect("scratch removed");
    }

    /// Documents are opened one at a time, each under a name nobody else has.
    #[test]
    fn documents_named_beside_arrive_one_at_a_time() {
        use super::{Arrivals, Named};
        let mut documents: Documents<()> = Documents::new(DocumentId(1), "first.pdf".to_owned());
        let mut arrivals = Arrivals::new();
        assert!(arrivals.start(&mut documents).is_none(), "nothing waits");
        arrivals.wait(Named::file("b.pdf".into()), true);
        arrivals.wait(Named::file("c.pdf".into()), false);

        let first = arrivals.start(&mut documents).map(|a| (a.id, a.behind));
        assert_eq!(first, Some((DocumentId(2), Some(DocumentId(1)))));
        assert!(
            arrivals.start(&mut documents).is_none(),
            "a second does not start while the first is on its way"
        );
        assert!(arrivals.is(DocumentId(2)));
        assert!(arrivals.settle(DocumentId(9)).is_none(), "not its name");
        assert!(arrivals.settle(DocumentId(2)).is_some());

        let second = arrivals.start(&mut documents).map(|a| (a.id, a.behind));
        assert_eq!(
            second,
            Some((DocumentId(3), None)),
            "a chosen file is not put behind: the person asked to read it"
        );
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

    /// §O.2.1's `ef` opens an embedded file out of the `/EmbeddedFiles` name tree, and that file
    /// has no path: it is opened from the bytes it came out
    /// in, the first time and on §7.6.4.1's second attempt, and never looked for on disk under the
    /// name it is given.
    #[test]
    fn an_embedded_document_opens_from_its_bytes_under_a_name_beside_its_holder() {
        let mut documents: Documents<()> = Documents::new(DocumentId(1), "holder.pdf".to_owned());
        let mut arrivals = super::Arrivals::new();
        let named = super::Named::embedded(
            Some(std::path::Path::new("/nowhere")),
            "../../etc/destination-doc.pdf",
            Some("page=3".to_owned()),
        );
        assert_eq!(
            named.path,
            std::path::Path::new("/nowhere/destination-doc.pdf"),
            "the document's own name, its last component only, beside the holder"
        );
        assert_eq!(label(&named.path), "destination-doc.pdf");
        arrivals.wait_held(
            named,
            pdf_syntax::FileBytes::from(b"%PDF-2.0\n".to_vec()),
            false,
        );

        let Some(arriving) = arrivals.start(&mut documents) else {
            panic!("the held document starts");
        };
        assert_eq!(arriving.id, DocumentId(2));
        assert_eq!(arriving.behind, None, "it comes to the front");
        assert_eq!(arriving.named.fragment.as_deref(), Some("page=3"));
        for attempt in ["first", "second"] {
            let Ok(bytes) = arriving.bytes() else {
                panic!("{attempt}: the held bytes, and no file under /nowhere is asked for");
            };
            assert_eq!(bytes.read(0..5).as_ref(), b"%PDF-", "{attempt}");
        }

        let unnamed = super::Named::embedded(None, "..", None);
        assert_eq!(
            unnamed.path,
            std::path::Path::new("embedded file"),
            "a name with no last component is given one"
        );
    }
}
