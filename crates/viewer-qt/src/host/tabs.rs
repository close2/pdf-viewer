//! The strip of open documents (ADR 1264): the order and the state are this host's, the `QTabBar`
//! is the C++ side's, and nothing here calls a Qt object. A document on its way to a tab arrives
//! through `viewer_host::Arrivals` (filed here by ADR 1908).

use std::collections::VecDeque;
use std::path::{Path, PathBuf};

use viewer_core::{Command, DocumentId, Purpose};

use super::{DOCUMENT, Host, Showing};

impl Host {
    /// What each tab says, in the order the strip shows them.
    ///
    /// **And the machine is asked for a face for any character of them the compiled-in face lacks**
    /// (ADR 1406) — the question `quorra`'s chrome asks of the same strings (ADR 1382), answered on
    /// `viewer_host::machine_faces`' thread and taken by [`Host::take_face`] when it lands.
    pub(crate) fn documents(&self) -> Vec<String> {
        let labels: Vec<String> = self
            .documents
            .iter()
            .map(|(_, label)| label.to_owned())
            .collect();
        for label in &labels {
            self.ask_machine(label);
        }
        labels
    }

    /// Which of them is in front.
    pub(crate) fn focused_document(&self) -> usize {
        self.documents.focused_index()
    }

    /// A person chose a tab, by its place in the strip.
    pub(crate) fn choose_document(&mut self, index: usize) {
        let Some(id) = self.documents.id_at(index) else {
            return;
        };
        self.show_document(id);
    }

    /// Brings one of the open documents to the front.
    ///
    /// Two things move together and neither may move without the other: what this window holds
    /// about the document ([`Showing`]), and the document scope of the restriction menu — which
    /// `viewer_core` already keeps beside the document it is about (ADR 1145).
    pub(super) fn show_document(&mut self, id: DocumentId) {
        if self.take_the_front(id) {
            self.dispatch(Command::Focus(id));
            self.build_panels();
        }
    }

    /// The half of [`Host::show_document`] that sends nothing.
    ///
    /// Its own function because a document opened *beside* the one showing arrives while the core
    /// is already settling the command that opened it: the core has moved its own focus by then,
    /// so what is left is this window's own swap and a nested `pump` would be a second command
    /// loop inside the first. Answers whether anything moved.
    pub(super) fn take_the_front(&mut self, id: DocumentId) -> bool {
        self.showing.departures = self.restrictions.document();
        if !self.documents.focus(id, &mut self.showing) {
            return false;
        }
        self.restrictions.depart(self.showing.departures);
        // §12.7's controls and §12.5.6.14's windows are **not** taken down here. Each is rebuilt
        // from the answer that follows `Command::Focus`, and both rebuilds are guarded by a
        // comparison against what is on the screen.
        self.documents_moved = true;
        self.update.title = true;
        self.update.controls = true;
        self.update.popups = true;
        true
    }

    /// A name held out for a document Table 203's `/NewWindow true` would open beside this one.
    ///
    /// **An offer rather than a request**, which is `viewer_core::Command::Beside`'s own shape:
    /// the name is used only where the action states the entry, and one this window offers and
    /// nothing opens under is simply skipped (ADR 1263).
    ///
    /// The file waits beside the name in `viewer_host::Arrivals`, so that a document which opens
    /// under it gets its tab and one that asks for §7.6.4.1's password is asked about as itself
    /// (ADR 1332).
    pub(super) fn offer_a_name(
        &mut self,
        path: &Path,
        read: &[u8],
        purpose: Purpose,
        queue: &mut VecDeque<Command>,
    ) {
        let name = self
            .arrivals
            .offer(&mut self.documents, path.to_owned(), read, purpose);
        queue.push_back(Command::Beside(Some(name)));
    }

    /// A document opened beside the one that was showing, under the name this window offered.
    ///
    /// A document a *reader* named arrives here too, under the name `viewer_host::Arrivals` reserved
    /// for it; what comes back is the tab to put in front again, for one named behind the first.
    pub(super) fn opened_beside(&mut self, id: DocumentId) -> Option<DocumentId> {
        let arriving = self.arrivals.settle(id)?;
        let (path, behind, bytes) = (
            arriving.named.path.clone(),
            arriving.behind,
            arriving.bytes(),
        );
        let bytes = match bytes {
            Ok(bytes) => bytes,
            Err(error) => {
                self.say(&format!(
                    "cannot keep {} open for a second attempt: {error}",
                    path.display()
                ));
                return None;
            }
        };
        let label = viewer_host::documents::label(&path);
        // §12.6.4.3's `/NewWindow false`, reached after §7.6.4.1's prompt: the document it was
        // reached from is replaced in its own tab, so the tab and this window's fields become the
        // new file's rather than a second tab being added (ADR 1332).
        if self.documents.index_of(id).is_some() {
            self.documents.relabel(id, label);
            if id == self.documents.focused() {
                self.showing = Showing::new(path, bytes, None);
            }
            self.documents_moved = true;
            self.update.title = true;
            return None;
        }
        self.documents
            .add(id, label.clone(), Showing::new(path, bytes, None));
        // `Documents::focus` is what swaps this window's fields; the core has moved its own focus
        // already, so nothing is sent from inside the settling of the command that opened it.
        if self.take_the_front(id) {
            self.build_panels();
        }
        self.say(&viewer_host::documents::opened_beside(
            &label,
            self.documents.len(),
        ));
        behind
    }

    /// The file a person chose in the `QFileDialog` Ctrl + O put on the screen, which goes to the
    /// front when it opens: they asked to read it.
    ///
    /// The dialogue decides nothing — it spells a path, which is ADR 1240's rule for the other
    /// chooser in this window — and what the path becomes is `viewer_host::open_chosen`'s, asked
    /// when its turn comes in `viewer_host::Arrivals` (ADR 1275).
    pub(crate) fn open_chosen(&mut self, path: &str) {
        if path.is_empty() {
            return;
        }
        self.arrivals
            .wait(viewer_host::Named::file(PathBuf::from(path)), false);
        self.open_the_next();
    }

    /// Where the chooser opens: the directory of the document in front, or nothing for Qt's own —
    /// which is also what a path named relative to the working directory has for a parent.
    pub(crate) fn chooser_directory(&self) -> String {
        self.showing
            .path
            .parent()
            .map(|directory| directory.to_string_lossy().into_owned())
            .unwrap_or_default()
    }

    /// How many pages this window keeps a small picture of, per
    /// [`viewer_host::stand_in::RETAINED_PAGES_WORD`]; zero turns the whole of it off (ADR 1866).
    pub fn retain_pages(&mut self, extent: usize) {
        self.retained = viewer_host::stand_in::Retained::new(extent);
    }

    /// The command line's later paths, each to open as a tab behind the first once page one is up.
    pub fn open_behind(&mut self, named: Vec<viewer_host::Named>) {
        for one in named {
            self.arrivals.wait(one, true);
        }
    }

    /// The C++ side's answer to [`Host::arrival_due`]: the next document may start now.
    pub(crate) fn arrive(&mut self) {
        self.open_the_next();
    }

    /// Starts the next document waiting to open, where none is on its way.
    ///
    /// `viewer_core::Command::Open` under the name `viewer_host::Arrivals` reserved, so the core
    /// hands the document every answer this reader has given exactly as it did the first
    /// (`Viewer::adopt`, ADR 1263); the tab is added when `Event::Opened` names it.
    pub(super) fn open_the_next(&mut self) {
        if let Some(declined) = self.arrivals.declined() {
            self.dispatch(declined);
        }
        loop {
            let Some(arriving) = self.arrivals.start(&mut self.documents) else {
                return;
            };
            let id = arriving.id;
            let fragment = arriving.named.fragment.clone();
            match arriving.bytes() {
                Ok(bytes) => {
                    self.dispatch(Command::Open {
                        id,
                        bytes,
                        password: None,
                        fragment,
                    });
                    return;
                }
                Err(sentence) => {
                    self.say(&sentence);
                    self.arrivals.settle(id);
                }
            }
        }
    }

    /// §7.6.4.1's second attempt, for the document on its way to a tab rather than the one in front.
    pub(super) fn open_arriving(&mut self, password: Option<viewer_core::Secret>) {
        let Some((id, bytes, fragment)) = self.arrivals.current().map(|arriving| {
            (
                arriving.id,
                arriving.bytes(),
                arriving.named.fragment.clone(),
            )
        }) else {
            return;
        };
        match bytes {
            Ok(bytes) => self.dispatch(Command::Open {
                id,
                bytes,
                password,
                fragment,
            }),
            Err(sentence) => {
                self.say(&sentence);
                self.given_up(id);
            }
        }
    }

    /// A document on its way to a tab that will not arrive — cancelled, refused, declined — and the
    /// next one waiting, which may now start. Nothing for any other name.
    pub(super) fn given_up(&mut self, id: DocumentId) {
        if self.arrivals.settle(id).is_some() || id == DOCUMENT {
            self.later_open_the_next();
        }
    }

    /// [`Self::given_up`], for §7.6.4.1's prompt cancelled or out of attempts: a file a document
    /// named is declined to the core as well, where the next arrival starts (ADR 1345).
    pub(super) fn cancelled(&mut self, id: DocumentId) {
        if self.arrivals.cancel(id) || id == DOCUMENT {
            self.later_open_the_next();
        }
    }

    /// [`Self::open_the_next`], from Qt's loop rather than from inside the event being answered.
    ///
    /// Every place a document settles is inside a `pump`, and opening the next from there would be
    /// a second command loop inside the first; so this is a flag the C++ side reads at the end of
    /// every update and answers with a zero-length `QTimer` and a call to [`Self::arrive`], which is
    /// `update.password`'s shape. Not a field of `QtUpdate`, because the first frame's `painted`
    /// raises it from inside the update that has already been taken.
    pub(super) fn later_open_the_next(&mut self) {
        self.arrival_due = true;
    }

    /// Whether [`Self::arrive`] is owed a call, taken so that it is answered once.
    pub(crate) fn arrival_due(&mut self) -> bool {
        std::mem::take(&mut self.arrival_due)
    }

    /// Closes the document in front, or the window where it is the only one.
    ///
    /// `viewer_host::documents::Close::Last` is where the two part, and the second half is the
    /// C++ side's: closing a window is `QWidget::close` and Rust never calls a Qt object, so it
    /// goes out as [`Host::closing_the_window`] on the next update (ADR 1264).
    pub(super) fn close_document(&mut self) {
        let closing = self.documents.focused();
        // The menu's document scope goes into the state that is about to be swapped, exactly as
        // it does on a switch.
        self.showing.departures = self.restrictions.document();
        match self.documents.close(closing, &mut self.showing) {
            viewer_host::Close::Last => self.closing = true,
            viewer_host::Close::Unknown => {}
            viewer_host::Close::Done { state, focused } => {
                self.restrictions.depart(self.showing.departures);
                let label = viewer_host::documents::label(&state.path);
                drop(state);
                self.documents_moved = true;
                self.pump(vec![Command::Close(closing), Command::Focus(focused)]);
                self.update.title = true;
                self.update.controls = true;
                self.update.popups = true;
                self.build_panels();
                self.say(&viewer_host::documents::closed(
                    &label,
                    self.documents.len(),
                ));
            }
        }
    }

    /// Whether the window should close, because the last document in it was closed.
    ///
    /// A flag read by C++ rather than a call into it, which is this bridge's own rule:
    /// `QWidget::close` is a Qt object's method and Rust never calls one.
    pub(crate) fn closing_the_window(&self) -> bool {
        self.closing
    }
}
