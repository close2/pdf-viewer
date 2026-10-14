//! The six panels: their rows, built when a panel is first shown, and what a press on a row does.

use std::rc::Rc;

use gtk4::glib;
use gtk4::prelude::*;
use viewer_core::{Answer, Command, PageTarget, Query};

use viewer_host::panel::{self, RowAction, Tab};
use viewer_host::trace::Topic;

use crate::{page, pages, tree};

use super::{DOCUMENT, Host, PANEL_WIDTH, Panel, with};

impl Host {
    /// What one of [`Tab`]'s panels holds, once its answer has been asked for.
    ///
    /// **The match below is exhaustive over [`Tab`] on purpose**, and it is this host's half of
    /// `doc/todo/30`'s "all three hosts stay level": a panel added to `viewer_host::Tab` fails to
    /// compile here, in `viewer-qt` and in `viewer-ui` until each supplies a widget for it. It is
    /// [`viewer_host::Key`]'s mechanism applied to the other thing a window shows (ADR 0526).
    ///
    /// [`viewer_host::Key`]: viewer_host::Key
    fn panel_of(&mut self, tab: Tab) -> Panel {
        match tab {
            // Not while the launch document's outline is being read off this thread: asking here
            // would read it on GTK's, in front of page one. Its arrival builds this panel again.
            Tab::Contents
                if self.preparing.is_some() && self.state.documents.focused() == DOCUMENT =>
            {
                Panel::Rows(vec![panel::PanelRow::saying(READING_THE_OUTLINE)])
            }
            Tab::Contents => Panel::Rows(match self.state.viewer.query(Query::Outline) {
                Answer::Outline(outline) if !outline.items.is_empty() => {
                    panel::outline_rows(&outline)
                }
                _ => vec![panel::PanelRow::saying("This document states no outline.")],
            }),
            // §12.3.4 is a page count and a picture per row rather than a tree — see `crate::pages`
            // for why the count is all that crosses here.
            Tab::Pages => Panel::Pages(match self.state.viewer.query(Query::PageCount) {
                Answer::Count(count) => count,
                _ => 0,
            }),
            Tab::Layers => Panel::Rows(match self.state.viewer.query(Query::Layers) {
                Answer::Layers(layers) if !layers.is_empty() => panel::layer_rows(&layers),
                _ => vec![panel::PanelRow::saying(
                    "This document states no optional content.",
                )],
            }),
            Tab::Files => Panel::Rows(self.file_rows()),
            Tab::Articles => Panel::Rows(match self.state.viewer.query(Query::Articles) {
                Answer::Articles(threads) => panel::article_rows(&threads),
                _ => panel::article_rows(&[]),
            }),
            // The one moment this window reads Table 349, so the tab's name is taken from the same
            // answer rather than from a second decode of §14.3.2's stream (ADR 1275).
            Tab::Document => Panel::Rows(match self.state.viewer.query(Query::Properties) {
                Answer::Properties {
                    information,
                    metadata,
                } => {
                    self.name_the_tab(&information);
                    panel::property_rows(&information, metadata.as_ref())
                }
                _ => panel::property_rows(&pdf_model::metadata::Information::default(), None),
            }),
        }
    }

    /// §7.11.4's embedded files, arranged as §12.3.5 says to where the document states a
    /// collection.
    ///
    /// > If this dictionary is present in a PDF document, the interactive PDF processor shall
    /// > present the document as a portable collection.
    ///
    /// The collection is asked for *first* because it decides how this same list is shown: a
    /// document stating one gets §12.3.5.2's folder tree and Table 155's columns, and every other
    /// document gets the `/EmbeddedFiles` tree's own order. Both mappings are
    /// `viewer_host::panel`'s, so this window and `viewer-qt` present a collection identically.
    fn file_rows(&self) -> Vec<panel::PanelRow> {
        let files = match self.state.viewer.query(Query::Attachments) {
            Answer::Attachments(files) => files,
            _ => Vec::new(),
        };
        if let Answer::Collection {
            collection,
            initial,
            order,
        } = self.state.viewer.query(Query::Collection)
        {
            return panel::collection_rows(&collection, &initial, &order, &files);
        }
        if files.is_empty() {
            return vec![panel::PanelRow::saying("This document embeds no files.")];
        }
        panel::attachment_rows(&files)
    }

    /// Builds every panel from its own answer.
    pub(super) fn build_panels(&mut self) {
        self.miniatures.borrow_mut().clear();
        self.previews.borrow_mut().clear();
        self.dedicate_to_the_collection();
        for tab in Tab::ALL {
            self.rebuild_panel(*tab);
        }
    }

    /// Builds one panel from its own answer, replacing whatever its slot held.
    ///
    /// Every panel at launch, and §7.11.4's files tab again whenever
    /// `Event::AttachmentsChanged` says the list it was built from has moved.
    pub(super) fn rebuild_panel(&mut self, tab: Tab) {
        let act = self.row_sink();
        let row = self.page_sink();
        let show = self.show_page_sink();
        let filled = self.panel_of(tab);
        let Some(slot) = self.ui.slots.get(tab.index()) else {
            return;
        };
        while let Some(child) = slot.first_child() {
            slot.remove(&child);
        }
        match &filled {
            Panel::Rows(rows) => {
                self.state.trace.say(
                    Topic::Panel,
                    format_args!("{}: {} row(s)", tab.label(), rows.len()),
                );
                let pictures = self.preview_sink();
                // §12.3.6's `FreeForm` is the one named layout that is not a list of rows —
                // "thumbnails … displayed at a random location on the view" — so it gets a
                // surface with places on it rather than a `GtkListView` (ADR 1251).
                if tab == Tab::Files && self.scatters_the_files() {
                    slot.append(&tree::scatter(rows, &act, &pictures));
                } else {
                    slot.append(&tree::tree(rows, &act, &pictures));
                }
            }
            Panel::Pages(count) => {
                self.state.trace.say(
                    Topic::Panel,
                    format_args!("{}: {count} page(s), miniatures on demand", tab.label()),
                );
                // **On the idle queue, not here**, and the screen is what said so.
                // `GtkListView` binds a row from GTK's layout, and putting the list into a
                // realised window starts one *synchronously* — inside this method, which runs
                // with the host's `RefCell` borrowed. The first run printed "the host was busy,
                // so page 1's row was drawn without its /Thumb" and drew a page list with no
                // pictures in it. Deferring lets the command that got here unwind first; it is
                // the same move `find.connect_search_mode_enabled_notify` makes below, and
                // `viewer-qt` makes with `Busy`.
                let (slot, count, row, show) = (slot.clone(), *count, row.clone(), show.clone());
                glib::idle_add_local_once(move || {
                    // Emptied here as well as above: two builds inside one pump queue two of
                    // these, and the slot holds one list rather than one per build.
                    while let Some(child) = slot.first_child() {
                        slot.remove(&child);
                    }
                    slot.append(&pages::page_list(count, &row, &show));
                });
            }
        }
    }

    /// §12.3.4: one page's label and miniature, asked for when the list is about to draw that row.
    ///
    /// The decode is here rather than in `build_panels` because that is the whole of `CLAUDE.md`
    /// principle 2's rule reaching this panel — a loop over the page count would have moved the eager work
    /// out of the launch path rather than out of the program. [`viewer_host::Miniatures`] bounds
    /// what is kept afterwards.
    ///
    /// The cache is beside the host rather than in it so that a `GtkListView` binding a row while
    /// the host is borrowed still gets its picture: `bind` fires from GTK's layout, which a
    /// `set_start_child` or a `set_current_page` inside a command can start.
    fn page_sink(&self) -> Rc<dyn Fn(usize) -> Option<pages::Row>> {
        let me = self.me.clone();
        let held = Rc::clone(&self.miniatures);
        let trace = self.state.trace;
        Rc::new(move |index| {
            let host = me.upgrade()?;
            // A row bound while the host is held — a tab arriving moves the view between notebook
            // pages from inside a command — is answered `None`, and `pages::bind` asks for it
            // again from the idle queue once the command has unwound. Traced rather than printed:
            // the row is drawn with its `/Thumb` a moment later, so there is nothing to tell a
            // reader.
            let host = host
                .try_borrow()
                .inspect_err(|_| {
                    trace.say(
                        Topic::Panel,
                        format_args!(
                            "the host was busy, so page {}'s row is asked for again once it is free",
                            index.saturating_add(1)
                        ),
                    );
                })
                .ok()?;
            Some(held.borrow_mut().row(index, || {
                let entry = viewer_host::page_entry(&host.state.viewer, index);
                viewer_host::Held {
                    label: entry.label,
                    picture: entry.thumbnail.as_ref().and_then(|image| {
                        page::thumbnail(image)
                            .inspect_err(|error| {
                                eprintln!("note: cannot show page {index}'s /Thumb: {error}");
                            })
                            .ok()
                    }),
                }
            }))
        })
    }

    /// §12.3.4: "allowing the user to navigate to a page by clicking its thumbnail image".
    ///
    /// A page index rather than a destination, because a thumbnail *is* the page and there is
    /// nothing to resolve.
    fn show_page_sink(&self) -> Rc<dyn Fn(usize)> {
        let me = self.me.clone();
        Rc::new(move |index| {
            with(&me, |host| {
                host.dispatch(Command::GoTo(PageTarget::Index(index)));
            });
        })
    }

    /// What a tree row does when a person acts on it.
    /// Table 158's `/Direction` `N`: the whole window region to the file navigation view.
    ///
    /// §12.3.5.1, Table 158:
    ///
    /// > N indicates that the window is not split. The entire window region shall be dedicated to
    /// > the file navigation view.
    ///
    /// The page's half of the splitter is **taken out rather than hidden**, which is the move
    /// `apply_chrome` already makes in the other direction and for the same reason: a `GtkPaned`
    /// carries a `position` that was set, and a hidden child would leave a hole where the page
    /// was. Put back for any document that does not ask for this, so nothing survives a second
    /// file. The panel is opened and the files tab selected, because a window dedicated to a view
    /// a person cannot see would be obeying the letter of the sentence and none of it (ADR 1252).
    fn dedicate_to_the_collection(&mut self) {
        let whole = match self.state.viewer.query(Query::Collection) {
            Answer::Collection { collection, .. } => panel::whole_window(&collection),
            _ => false,
        };
        if whole {
            self.panel_wanted = true;
            self.ui
                .tabs
                .set_current_page(Some(notebook_page(Tab::Files)));
            if self.ui.split.end_child().is_some() {
                self.ui.split.set_end_child(None::<&gtk4::Widget>);
            }
        } else if self.ui.split.end_child().is_none() {
            let page_area = self.ui.page_area.clone();
            self.ui.split.set_end_child(Some(&page_area));
            self.ui.split.set_position(PANEL_WIDTH);
        }
        self.apply_chrome();
    }

    /// Whether the files panel is drawing §12.3.6's `FreeForm` scatter rather than a list.
    ///
    /// Asked of the same answer the rows were built from, because a surface and a list are two
    /// widgets and only one of them is built: "thumbnails … displayed at a random location on the
    /// view" is not a `GtkListView` (ADR 1251).
    fn scatters_the_files(&self) -> bool {
        match self.state.viewer.query(Query::Collection) {
            Answer::Collection { collection, .. } => {
                panel::presentation(&collection) == panel::Mode::FreeForm
            }
            _ => false,
        }
    }

    /// §12.3.6's preview picture for one attachment, decoded when a row is bound.
    ///
    /// `page_sink`'s shape and its reasoning: GTK binds the rows it lays out, so a collection of
    /// a hundred files opens the embedded documents of the rows on the screen and no others.
    /// A file that states no §12.3.4 `/Thumb` on its first page is held as one, so the same
    /// document is not opened again on the next bind (ADR 1251).
    fn preview_sink(&self) -> tree::Previews {
        let me = self.me.clone();
        let held = Rc::clone(&self.previews);
        Rc::new(move |name: &str| {
            let host = me.upgrade()?;
            // Trap 5 rather than a blank picture, exactly as `page_sink` says: a toolkit's
            // scheduling is a claim, and one that failed silently would be a thumbnail nobody
            // could account for.
            let host = host
                .try_borrow()
                .inspect_err(|_| {
                    eprintln!("note: the host was busy, so {name:?} was drawn without its preview");
                })
                .ok()?;
            held.borrow_mut()
                .picture(name, || {
                    panel::attachment_preview(&host.state.viewer, name).and_then(|image| {
                        page::thumbnail(&image)
                            .inspect_err(|error| {
                                eprintln!("note: cannot show {name:?}'s preview: {error}");
                            })
                            .ok()
                    })
                })
                .cloned()
        })
    }

    fn row_sink(&self) -> Rc<dyn Fn(&RowAction)> {
        let me = self.me.clone();
        Rc::new(move |action| {
            let action = action.clone();
            with(&me, |host| match action {
                RowAction::Activate(object) => host.dispatch(Command::Activate(object)),
                RowAction::Toggle { group, on, .. } => {
                    host.dispatch(Command::SetGroup { group, on });
                }
                RowAction::Extract { name } => host.dispatch(Command::Extract { name }),
                RowAction::Inert => {}
            });
        })
    }
}

/// Which of this notebook's pages one of [`Tab`]'s panels is.
///
/// **Exhaustive over [`Tab`] on purpose, and it is a second statement rather than a wrapper around
/// [`Tab::index`].** The notebook's pages are appended in [`Tab::ALL`]'s order and a panel is
/// addressed by its index, so the two ways of naming a panel are two things that can disagree —
/// which is exactly what `every_panel_the_list_states_has_a_page_of_this_notebook` checks. A panel
/// added to `viewer_host::Tab` fails to compile here until this host says where it goes.
pub(super) const fn notebook_page(tab: Tab) -> u32 {
    match tab {
        Tab::Contents => 0,
        Tab::Pages => 1,
        Tab::Layers => 2,
        Tab::Files => 3,
        Tab::Articles => 4,
        Tab::Document => 5,
    }
}

/// What the contents panel says while the launch document's outline is read beside page one
/// (ADR 1553): a sentence about this window, since the document's answer is not in yet.
const READING_THE_OUTLINE: &str = "Reading the outline.";
