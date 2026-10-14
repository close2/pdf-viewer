//! The six panels' rows: §12.3.3's outline, §12.3.4's miniatures, §8.11.4.3's `/Order`, §7.11.4's
//! files, §12.4.3's threads and §14.3.3's information, flattened because Qt's item model wants
//! them flat (ADR 0246; filed here by ADR 1908).

use viewer_core::{Answer, Command, PageTarget, Query};
use viewer_host::panel::{Icon, PanelRow, RowAction, Tab};
use viewer_host::trace::Topic;

use crate::bridge::ffi::{QtPage, QtRow, QtScatter};

use super::{DOCUMENT, Host, READING_THE_OUTLINE};

/// One row of a tree as the C++ side sees it, beside what acting on it does.
///
/// The flattening is Qt's requirement rather than a convenience: `QAbstractItemModel` must answer
/// `rowCount` and `parent` for every node at any moment, so a lazily-built tree is not available
/// to it the way `GtkTreeListModel`'s is. ADR 0246.
#[derive(Debug, Clone)]
pub(super) struct Flat {
    /// What the row shows and how deep it is.
    pub(super) row: QtRow,
    /// What acting on it does, which stays on this side of the bridge.
    action: RowAction,
}

impl Host {
    /// A row of one of the three trees was activated.
    pub(crate) fn activate_row(&mut self, tree: u8, index: usize) {
        let Some(flat) = self.flat(tree, index) else {
            return;
        };
        match flat.action.clone() {
            RowAction::Activate(object) => self.dispatch(Command::Activate(object)),
            RowAction::Extract { name } => self.dispatch(Command::Extract { name }),
            // A switch is moved by `toggle_row` and a heading does nothing: §8.11.4.3's leading
            // text string is "a collection of related groups" under a heading, and the heading is
            // not a layer.
            RowAction::Toggle { .. } | RowAction::Inert => {}
        }
    }

    /// §8.11.4.3's switch on a row was moved.
    pub(crate) fn toggle_row(&mut self, tree: u8, index: usize, on: bool) {
        let Some(flat) = self.flat(tree, index) else {
            return;
        };
        if let RowAction::Toggle { group, locked, .. } = flat.action {
            // Table 99: "[t]he state of a locked group cannot be changed through the user
            // interface of an interactive PDF processor." The model refuses the click as well —
            // this is the second half of the same clause, said where the command is sent.
            if locked {
                self.say("that group is locked (Table 99) and its state cannot be changed");
                return;
            }
            self.dispatch(Command::SetGroup { group, on });
        }
    }

    /// One panel's rows, depth first.
    ///
    /// **And the machine is asked for a face for any character of them the compiled-in face
    /// lacks**, as the strip's labels are (ADR 1418): Qt falls back by family in a tree view exactly
    /// as it does in a tab bar, and drew a Chinese outline title as boxes on a machine whose
    /// covering face shares its family name with Latin ones.
    pub(crate) fn rows(&self, tree: u8) -> Vec<QtRow> {
        let rows: Vec<QtRow> = self
            .tree(tree)
            .map(|rows| rows.iter().map(|flat| flat.row.clone()).collect())
            .unwrap_or_default();
        for row in &rows {
            self.ask_machine(&row.label);
            self.ask_machine(&row.detail);
        }
        rows
    }

    /// What one of [`viewer_host::Tab`]'s panels is called.
    ///
    /// Asked rather than written into the C++ so that the six words are one set of six words: the
    /// same argument `notices` and `password_prompt` are here for.
    #[expect(
        clippy::unused_self,
        reason = "the bridge's Rust side is a set of methods on Host, so a constant answer is \
                  still a method — an associated function would need a second declaration"
    )]
    pub(crate) fn panel_label(&self, tab: u8) -> String {
        Tab::at(usize::from(tab)).map_or_else(String::new, |tab| tab.label().to_owned())
    }

    /// Which of the panels is §12.3.4's, which is the one that is a list of pictures.
    ///
    /// Asked rather than written into the C++ so that the *order* of the panels is stated once:
    /// a panel inserted before this one in `viewer_host::Tab::ALL` moves this number, and a window
    /// that had the position written down would put a `QListView` where a `QTreeView` belongs.
    #[expect(
        clippy::unused_self,
        reason = "the bridge's Rust side is a set of methods on Host, so a constant answer is \
                  still a method — an associated function would need a second declaration"
    )]
    pub(crate) fn pages_panel(&self) -> u8 {
        pages_panel_index()
    }

    /// Which of the panels is §12.3.5's, whose presentation Table 153 and §12.3.6 decide.
    ///
    /// Asked across the bridge for `pages_panel`'s reason: the list of panels is
    /// `viewer_host::Tab` and a second one in C++ would be a second thing to keep level.
    #[expect(
        clippy::unused_self,
        reason = "the bridge's Rust side is a set of methods on Host, so a constant answer is \
                  still a method"
    )]
    pub(crate) fn files_panel(&self) -> u8 {
        u8::try_from(Tab::Files.index()).unwrap_or(u8::MAX)
    }

    /// How many rows §12.3.4's panel has, which is how many pages the document has.
    pub(crate) fn page_count(&self) -> usize {
        match self.state.viewer.query(Query::PageCount) {
            Answer::Count(count) => count,
            _ => 0,
        }
    }

    /// One row of §12.3.4's panel: the page's label, and its miniature where it states one.
    ///
    /// **Asked for one page at a time, when the model is about to draw that row**, which is the
    /// whole of `CLAUDE.md` section 2's rule reaching this panel — a call per page at build time would
    /// have moved the eager decode out of the launch path rather than out of the program.
    ///
    /// The picture is copied out rather than borrowed, unlike a frame's: a miniature is tens of
    /// kilobytes against a frame's megabytes, it is asked for once per row rather than once per
    /// present, and the C++ side keeps a `QPixmap` of it afterwards — so a borrow would buy one
    /// avoided copy at the price of a second cache on this side of the bridge.
    pub(crate) fn page_row(&self, index: usize) -> QtPage {
        let entry = viewer_host::page_entry(&self.state.viewer, index);
        self.ask_machine(&entry.label);
        match entry.thumbnail {
            Some(image) => QtPage {
                label: entry.label,
                width: image.width,
                height: image.height,
                pixels: image.data.to_vec(),
            },
            None => QtPage {
                label: entry.label,
                width: 0,
                height: 0,
                pixels: Vec::new(),
            },
        }
    }

    /// How many of §12.3.4's miniatures the panel may keep, from the one place that decides it.
    ///
    /// The `QPixmap` cache is C++'s, because a `QPixmap` is, and the *bound* is
    /// [`viewer_host::KEPT_MINIATURES`] — asked for across the bridge rather than written down a
    /// second time, which is the same reason the panel labels are asked for.
    #[expect(
        clippy::unused_self,
        reason = "the bridge's Rust side is a set of methods on Host, so a constant answer is \
                  still a method — an associated function would need a second declaration"
    )]
    pub(crate) fn kept_miniatures(&self) -> usize {
        viewer_host::KEPT_MINIATURES
    }

    /// §12.3.6's preview picture for one attachment, asked for when a row is about to be drawn.
    ///
    /// [`Self::page_row`]'s shape and every one of its reasons: one file at a time, the pixels
    /// copied out because the C++ side keeps a `QPixmap` of them, and a zero width for a file
    /// that states no §12.3.4 `/Thumb` on its first page — which is most files, and is a document
    /// being quiet rather than a failure (ADR 1251).
    pub(crate) fn attachment_preview(&self, name: &str) -> QtPage {
        match viewer_host::panel::attachment_preview(&self.state.viewer, name) {
            Some(image) => QtPage {
                label: String::new(),
                width: image.width,
                height: image.height,
                pixels: image.data.to_vec(),
            },
            None => QtPage {
                label: String::new(),
                width: 0,
                height: 0,
                pixels: Vec::new(),
            },
        }
    }

    /// Where §12.3.6's `FreeForm` puts one file's thumbnail, from the one place all three windows
    /// ask.
    #[expect(
        clippy::unused_self,
        reason = "the bridge's Rust side is a set of methods on Host, so an answer that depends                   only on its argument is still a method"
    )]
    pub(crate) fn scatter_place(&self, name: &str) -> QtScatter {
        let (across, down) = viewer_host::panel::scattered(name);
        QtScatter { across, down }
    }

    /// Whether the files panel is drawing §12.3.6's `FreeForm` scatter rather than a list of rows.
    pub(crate) fn collection_scatters(&self) -> bool {
        match self.state.viewer.query(Query::Collection) {
            Answer::Collection { collection, .. } => {
                viewer_host::panel::presentation(&collection) == viewer_host::panel::Mode::FreeForm
            }
            _ => false,
        }
    }

    /// Table 158's `/Direction` `N`: whether the whole window region is the file navigation view's.
    pub(crate) fn collection_takes_the_window(&self) -> bool {
        match self.state.viewer.query(Query::Collection) {
            Answer::Collection { collection, .. } => viewer_host::panel::whole_window(&collection),
            _ => false,
        }
    }

    /// §12.3.4: "allowing the user to navigate to a page by clicking its thumbnail image".
    ///
    /// A page index rather than a destination, because a thumbnail *is* the page.
    pub(crate) fn show_page(&mut self, index: usize) {
        self.dispatch(Command::GoTo(PageTarget::Index(index)));
    }

    /// The rows of one panel, or none at all for the one panel that has no rows.
    ///
    /// **The match is exhaustive over [`viewer_host::Tab`] on purpose**, and it is this host's half
    /// of `doc/todo/30`'s "all three hosts stay level": a panel added to `viewer_host::Tab` fails
    /// to compile here, in `viewer-gtk` and in `viewer-ui` until each supplies a widget for it. It
    /// is `viewer_host::Key`'s mechanism applied to the other thing a window shows (ADR 0526).
    pub(super) fn panel_of(&mut self, tab: Tab) -> Vec<PanelRow> {
        use viewer_host::panel::{
            article_rows, attachment_rows, collection_rows, layer_rows, outline_rows, property_rows,
        };
        match tab {
            // Not while the launch document's outline is being read off this thread: asking here
            // would read it on Qt's, in front of page one. Its arrival builds this panel again.
            Tab::Contents
                if self.preparing.is_some() && self.state.documents.focused() == DOCUMENT =>
            {
                vec![PanelRow::saying(READING_THE_OUTLINE)]
            }
            Tab::Contents => match self.state.viewer.query(Query::Outline) {
                Answer::Outline(outline) if !outline.items.is_empty() => outline_rows(&outline),
                _ => vec![PanelRow::saying("This document states no outline.")],
            },
            // §12.3.4 is a page count and a picture per row rather than a tree; the C++ side asks
            // `page_count` and `page_row` for it, one row at a time.
            Tab::Pages => Vec::new(),
            Tab::Layers => match self.state.viewer.query(Query::Layers) {
                Answer::Layers(layers) if !layers.is_empty() => layer_rows(&layers),
                _ => vec![PanelRow::saying(
                    "This document states no optional content.",
                )],
            },
            // §12.3.5: "[i]f this dictionary is present in a PDF document, the interactive PDF
            // processor shall present the document as a portable collection." The collection is
            // asked for first because it decides how this same list is shown — §12.3.5.2's folder
            // tree and Table 155's columns where a document states one, the `/EmbeddedFiles`
            // tree's own order where it does not. Both mappings are `viewer_host::panel`'s, so
            // this window and `viewer-gtk` present a collection identically.
            Tab::Files => {
                let files = match self.state.viewer.query(Query::Attachments) {
                    Answer::Attachments(files) => files,
                    _ => Vec::new(),
                };
                match self.state.viewer.query(Query::Collection) {
                    Answer::Collection {
                        collection,
                        initial,
                        order,
                    } => collection_rows(&collection, &initial, &order, &files),
                    _ if files.is_empty() => {
                        vec![PanelRow::saying("This document embeds no files.")]
                    }
                    _ => attachment_rows(&files),
                }
            }
            Tab::Articles => match self.state.viewer.query(Query::Articles) {
                Answer::Articles(threads) => article_rows(&threads),
                _ => article_rows(&[]),
            },
            // The one moment this window reads Table 349, so the tab's name is taken from the same
            // answer rather than from a second decode of §14.3.2's stream (ADR 1275).
            Tab::Document => match self.state.viewer.query(Query::Properties) {
                Answer::Properties {
                    information,
                    metadata,
                } => {
                    let label =
                        viewer_host::documents::titled(&information, &self.state.showing.path);
                    self.state
                        .documents
                        .relabel(self.state.documents.focused(), label);
                    self.documents_moved = true;
                    property_rows(&information, metadata.as_ref())
                }
                _ => property_rows(&pdf_model::metadata::Information::default(), None),
            },
        }
    }

    /// Builds every panel that has rows from its own answer.
    pub(super) fn build_panels(&mut self) {
        let trees = std::array::from_fn(|index| {
            Tab::at(index)
                .map(|tab| flatten(&self.panel_of(tab)))
                .unwrap_or_default()
        });
        self.trees = trees;
        for (tab, rows) in Tab::ALL.iter().zip(&self.trees) {
            self.state.trace.say(
                Topic::Panel,
                format_args!("{}: {} row(s)", tab.label(), rows.len()),
            );
        }
        self.update.panels = true;
    }

    /// One tree's rows, where the number names one.
    fn tree(&self, tree: u8) -> Option<&Vec<Flat>> {
        self.trees.get(usize::from(tree))
    }

    /// One row of one tree, where both indices name one.
    fn flat(&self, tree: u8, index: usize) -> Option<&Flat> {
        self.tree(tree).and_then(|rows| rows.get(index))
    }
}

/// A tree of rows, depth first, with each row's depth beside it.
pub(super) fn flatten(rows: &[PanelRow]) -> Vec<Flat> {
    let mut flat = Vec::new();
    push_rows(rows, 0, &mut flat);
    flat
}

/// Which place in [`Tab::ALL`] §12.3.4's panel is, which is the one the C++ builds a list at.
///
/// A function rather than a literal in `window.cpp` so that the order of the panels is stated once:
/// a panel inserted before §12.3.4's moves this number, and a window with it written down would put
/// a `QListView` where a `QTreeView` belongs.
pub(super) fn pages_panel_index() -> u8 {
    u8::try_from(Tab::Pages.index()).unwrap_or(u8::MAX)
}

/// One level of the tree and everything under it.
fn push_rows(rows: &[PanelRow], depth: u32, into: &mut Vec<Flat>) {
    for row in rows {
        let (action, on, locked) = match &row.action {
            RowAction::Inert => (0, false, false),
            RowAction::Activate(_) => (1, false, false),
            RowAction::Toggle { on, locked, .. } => (2, *on, *locked),
            RowAction::Extract { .. } => (3, false, false),
        };
        into.push(Flat {
            row: QtRow {
                label: row.label.clone(),
                detail: row.detail.clone().unwrap_or_default(),
                depth,
                expanded: row.expanded,
                action,
                on,
                locked,
                note: row.note,
                emphasis: row.emphasis,
                headings: row.cells.iter().map(|cell| cell.heading.clone()).collect(),
                values: row.cells.iter().map(|cell| cell.value.clone()).collect(),
                icon: row
                    .picture
                    .map(|picture| Icon::theme_name(picture.kind()))
                    .unwrap_or_default()
                    .to_owned(),
                picture: match row.picture {
                    None => 0,
                    Some(viewer_host::panel::Picture::Icon(_)) => 1,
                    Some(viewer_host::panel::Picture::Thumbnail(_)) => 2,
                    Some(viewer_host::panel::Picture::Preview(_)) => 3,
                },
                key: match &row.action {
                    RowAction::Extract { name } => name.clone(),
                    _ => String::new(),
                },
            },
            action: row.action.clone(),
        });
        push_rows(&row.children, depth.saturating_add(1), into);
    }
}
