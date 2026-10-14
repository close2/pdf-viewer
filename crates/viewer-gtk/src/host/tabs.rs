//! The strip of open documents: a tab chosen, a document opened beside or behind this one, the ones arriving, and a document closed.

use std::collections::VecDeque;
use std::path::{Path, PathBuf};

use gtk4::prelude::*;
use gtk4::{gio, glib};
use viewer_core::{Command, DocumentId, Purpose};

use super::{DOCUMENT, Host, Showing, with};

impl Host {
    /// What the tab in front says: §14.3.3's `/Title`, or the file's name
    /// (`viewer_host::documents::titled`).
    pub(super) fn name_the_tab(&mut self, information: &pdf_model::metadata::Information) {
        let label = viewer_host::documents::titled(information, &self.showing.path);
        self.documents
            .relabel(self.documents.focused(), label.clone());
        if let Some(page) = u32::try_from(self.documents.focused_index())
            .ok()
            .and_then(|index| self.ui.documents.nth_page(Some(index)))
        {
            self.ui.documents.set_tab_label_text(&page, &label);
        }
    }

    /// A person chose a tab in the strip of open documents.
    ///
    /// Reached from `switch-page` on the idle queue, so it runs after whatever caused the signal
    /// has let go of this host. A tab that is already in front is a switch that moves nothing,
    /// which is what makes this safe to reach from both a click and this host's own
    /// `set_current_page` — the second is a no-op that costs one idle callback (ADR 1264).
    pub(super) fn tab_chosen(&mut self, index: usize) {
        // A switch the idle queue carried past a later one is stale: the notebook's current page is
        // the one last chosen, and answering the older index would swap two tabs back and forth
        // for as long as the loop ran — which is what two documents opened in one pump did.
        if self.ui.documents.current_page() != u32::try_from(index).ok() {
            return;
        }
        let Some(id) = self.documents.id_at(index) else {
            return;
        };
        let mut queue = VecDeque::new();
        self.show_document(id, &mut queue);
        if !queue.is_empty() {
            self.pump(queue);
        }
    }

    /// Brings one of the open documents to the front.
    ///
    /// Three things move together and none of them may move without the others: what this window
    /// holds about the document ([`Showing`]), the document scope of the restriction menu — which
    /// `viewer_core` already keeps beside the document it is about (ADR 1145) — and the view
    /// itself, which is one widget tree moved into the notebook page that is now current.
    pub(super) fn show_document(&mut self, id: DocumentId, queue: &mut VecDeque<Command>) {
        self.showing.departures = self.restrictions.document();
        if !self.documents.focus(id, &mut self.showing) {
            return;
        }
        self.restrictions.depart(self.showing.departures);
        self.hold_the_view(self.documents.focused_index());
        // §12.7's controls and §12.5.6.14's windows are **not** taken down here. Each is rebuilt
        // from the answer that follows `Command::Focus`, and both rebuilds are guarded by a
        // comparison against what is on the screen — so a document whose fields and popups differ
        // gets new widgets and one whose do not keeps the widgets it would have been given.
        queue.push_back(Command::Focus(id));
        self.retitle();
        self.panels_due = true;
    }

    /// Moves the one view this window has into the notebook page that should be showing it.
    ///
    /// The strip is a real [`gtk4::Notebook`] with a page per document and one widget tree between
    /// them; see [`Ui::documents`](super::Ui::documents) for why. It is also the one place `split` may be taken out of a
    /// page, which is what makes a page safe to destroy afterwards.
    fn hold_the_view(&mut self, index: usize) {
        let Ok(index) = u32::try_from(index) else {
            return;
        };
        // This emits `switch-page` synchronously, and the handler defers to the idle queue and
        // comes back here — where the swap has already happened and both guards below answer
        // *nothing to do*. A flag would say the same thing twice.
        self.ui.documents.set_current_page(Some(index));
        let Some(page) = self
            .ui
            .documents
            .nth_page(Some(index))
            .and_downcast::<gtk4::Box>()
        else {
            return;
        };
        if let Some(holder) = self.ui.split.parent().and_downcast::<gtk4::Box>() {
            if holder == page {
                return;
            }
            holder.remove(&self.ui.split);
        }
        page.append(&self.ui.split);
    }

    /// A name held out for a document Table 203's `/NewWindow true` would open beside this one.
    ///
    /// **An offer rather than a request**, which is `viewer_core::Command::Beside`'s own shape: the
    /// name is used only where the action states the entry, and one this window offers and nothing
    /// opens under is simply skipped. It is offered per supplied file so that the reserve is never
    /// stale — a name held across two clicks would have the second document open under the first's
    /// identity, which `Command::Open`'s rule makes a *replacement* of a tab somebody is reading.
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
    /// The core has already made it the focused document — a person who followed a link asking for
    /// a new window asked to be reading the destination — so what is left here is the tab, the
    /// swap of this window's own per-document state, and the sentence.
    ///
    /// A document a *reader* named arrives here too, under the name `viewer_host::Arrivals` reserved
    /// for it; what comes back is the tab to put in front again, for one named behind the first.
    pub(super) fn opened_beside(
        &mut self,
        id: DocumentId,
        queue: &mut VecDeque<Command>,
    ) -> Option<DocumentId> {
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
        if let Some(index) = self.documents.index_of(id) {
            self.documents.relabel(id, label.clone());
            if let Some(page) = u32::try_from(index)
                .ok()
                .and_then(|index| self.ui.documents.nth_page(Some(index)))
            {
                self.ui.documents.set_tab_label_text(&page, &label);
            }
            if id == self.documents.focused() {
                self.showing = Showing::new(path, bytes, None);
            }
            return None;
        }
        let holder = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        self.ui
            .documents
            .append_page(&holder, Some(&gtk4::Label::new(Some(&label))));
        self.documents
            .add(id, label.clone(), Showing::new(path, bytes, None));
        self.show_the_tabs();
        // `Documents::focus` is what swaps this window's fields; the core has moved its own focus
        // already, and `Command::Focus` on a document that is already focused does nothing.
        self.show_document(id, queue);
        self.say(&viewer_host::documents::opened_beside(
            &label,
            self.documents.len(),
        ));
        behind
    }

    /// A person asked for a document: `gtk4::FileDialog`, and the file it answers opens beside the
    /// one showing.
    ///
    /// The dialogue decides nothing — it spells a path, which is ADR 1240's rule for the other
    /// chooser in this window — and what the path becomes is `viewer_host::open_chosen`'s, asked
    /// when its turn comes in `viewer_host::Arrivals` (ADR 1275).
    pub(super) fn choose_a_document(&self) {
        let dialog = gtk4::FileDialog::new();
        dialog.set_title("Open a document beside this one");
        dialog.set_modal(true);
        // Beside the document already showing, which is where a person reaching for a second one
        // most often looks; GTK's own default where this is not a directory.
        // A path named relative to the working directory has an empty parent, which is no folder
        // GIO can list, so that one is left to GTK too.
        if let Some(directory) = self
            .showing
            .directory
            .as_deref()
            .filter(|directory| !directory.as_os_str().is_empty())
        {
            dialog.set_initial_folder(Some(&gio::File::for_path(directory)));
        }
        let me = self.me.clone();
        dialog.open(
            Some(&self.ui.window),
            gio::Cancellable::NONE,
            move |chosen| {
                // A person who dismissed the chooser has asked for nothing: `Err` here is that
                // dismissal as much as it is a failure, and either way there is no path.
                if let Ok(file) = chosen
                    && let Some(path) = file.path()
                {
                    with(&me, |host| host.chose(path));
                }
            },
        );
    }

    /// A file a person chose, which goes to the front when it opens: they asked to read it.
    fn chose(&mut self, path: PathBuf) {
        self.arrivals.wait(viewer_host::Named::file(path), false);
        self.open_the_next();
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

    /// [`Self::open_the_next`], from the main loop rather than from inside the event being answered.
    ///
    /// Every place a document settles is inside a `pump`, and opening the next from there would be
    /// a second command loop inside the first.
    pub(super) fn later_open_the_next(&self) {
        let me = self.me.clone();
        glib::idle_add_local_once(move || with(&me, Self::open_the_next));
    }

    /// Closes the document in front, or the window where it is the only one.
    ///
    /// `viewer_host::documents::Close::Last` is where the two part: a window with one document
    /// left has nothing to show after the close, and closing the window is this toolkit's
    /// `GtkWindow::close` rather than anything on the boundary (ADR 1264).
    pub(super) fn close_document(&mut self) {
        let closing = self.documents.focused();
        let Some(index) = self.documents.index_of(closing) else {
            return;
        };
        // The menu's document scope goes into the state that is about to be swapped, exactly as
        // it does on a switch: the value the menu holds is this document's and the one that comes
        // back belongs to whichever tab takes the front.
        self.showing.departures = self.restrictions.document();
        match self.documents.close(closing, &mut self.showing) {
            viewer_host::Close::Last => self.ui.window.close(),
            viewer_host::Close::Unknown => {}
            viewer_host::Close::Done { state, focused } => {
                self.restrictions.depart(self.showing.departures);
                // **The view leaves the page before the page is destroyed.** A notebook page
                // owns its child, and the page being removed is the one holding the splitter —
                // so removing it first would take the panels and the page area down with it.
                if let Some(holder) = self.ui.split.parent().and_downcast::<gtk4::Box>() {
                    holder.remove(&self.ui.split);
                }
                if let Ok(index) = u32::try_from(index) {
                    self.ui.documents.remove_page(Some(index));
                }
                self.show_the_tabs();
                self.hold_the_view(self.documents.focused_index());
                let label = viewer_host::documents::label(&state.path);
                drop(state);
                self.pump(VecDeque::from([
                    Command::Close(closing),
                    Command::Focus(focused),
                ]));
                self.retitle();
                self.build_panels();
                self.say(&viewer_host::documents::closed(
                    &label,
                    self.documents.len(),
                ));
            }
        }
    }

    /// Shows the strip of document tabs where there is more than one document and Table 29's
    /// `FullScreen` is not asking for "no menu bar, window controls, or any other window visible".
    ///
    /// **A hidden strip also takes no keys**: a `GtkNotebook` whose tab holds the keyboard turns
    /// Left and Right into a change of document, and in a presentation those are the keys that
    /// turn the page (ADR 1303).
    pub(super) fn show_the_tabs(&self) {
        let shown = self.presenting.chrome().other_windows && !self.documents.is_alone();
        self.ui.documents.set_show_tabs(shown);
    }
}
