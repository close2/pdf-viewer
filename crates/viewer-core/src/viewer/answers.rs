//! What the viewer answers when asked: [`Viewer::query`] and the printed-page and layer answers it
//! assembles from the open documents. Nothing here changes the state.

use std::collections::BTreeSet;
use std::sync::Arc;

use pdf_model::optional_content::{ListMode, OptionalContent, Presented};
use pdf_render::TargetSpec;
use pdf_syntax::ObjectId;

#[cfg(doc)]
use super::pointer::attached_files;
use super::{DocumentId, MAX_PIXELS, Viewer};
use crate::command::Viewing;
use crate::interact;
use crate::open::Open;
use crate::query::{Answer, FrameView, Layer, PageReadback, PageReports, Query};
use crate::readback::ReadbackCache;

impl Viewer {
    /// What one open document's readback cache is holding, or `None` for a document that is not
    /// open.
    ///
    /// An instrument rather than a [`Query`], and the difference is what each is for: a `Query`
    /// is a question a host asks in order to *draw* something, and six consumers plus a wire
    /// protocol match that enum exhaustively. This answers a question about the program's own
    /// memory, which no interface displays and which `--trace=search` and
    /// `viewer-core/examples/find_cost` print. `CLAUDE.md`'s rule about a budget is that it be
    /// legible as well as bounded, and this is the legible half.
    #[must_use]
    pub fn readback_cache(&self, document: DocumentId) -> Option<ReadbackCache> {
        self.documents
            .get(&document)
            .map(|open| open.readbacks.report())
    }

    /// Answers a question about the viewer's state without changing any of it.
    #[must_use]
    #[expect(
        clippy::too_many_lines,
        reason = "one arm per variant of this crate's own question enum, and the count is that \
                  enum's. Splitting it would put half the vocabulary in another function and lose \
                  what the exhaustive match is here for: the compiler naming the question nobody \
                  answered"
    )]
    pub fn query(&self, query: Query<'_>) -> Answer<'_> {
        // Every document's, before the focused one is looked for: a timer runs in a document
        // behind another, and a window with none in front still owes it its ticks (ADR 1702).
        if matches!(query, Query::TimerDue) {
            return Answer::TimerDue(
                self.documents
                    .values()
                    .filter_map(|open| open.view.timer_due())
                    .min(),
            );
        }
        let (Some(id), Some(open)) = (self.focused, self.focused()) else {
            return Answer::None;
        };
        match query {
            Query::PageCount => Answer::Count(open.page_count),
            Query::CurrentPage => Answer::Page {
                document: id,
                index: open.page_index,
                label: label(open, open.page_index),
                of: open.page_count,
            },
            Query::PageGeometry(index) => self
                .geometry(open, index)
                .map_or(Answer::None, Answer::Geometry),
            // The three as this crate holds them, settled: `Viewer::handle` clamps the scroll at
            // the end of every command, so what a host reads here is where the reader is rather
            // than where the last message asked for them to be.
            Query::View => Answer::View(Viewing {
                page: open.page_index,
                zoom: open.zoom,
                scroll: open.scroll,
            }),
            // Cloned rather than lent, since ADR 0247: every consumer cloned it anyway, because a
            // panel outlives the query that filled it. Measured by
            // `viewer-host --example outline_census`, median of five: ISO 32000-2's own outline
            // is 988 items and the whole answer takes **80.7 µs**; the five-page application
            // note's fourteen take **481 ns**. Against ADR 0246's 3.66 ms to build the three
            // panel models those 988 rows go into, the clone is 2% of what a host does with it.
            Query::Outline => Answer::Outline(open.outline().clone()),
            Query::Layers => Answer::Layers(layers(open)),
            // The log's view rather than the file's (ADR 0814): a file attached this sitting is
            // in the list before anything is saved, and
            // one detached is out of it. `ViewState::attachments` says which home the list is.
            Query::Attachments => Answer::Attachments(attachments_in_view(open)),
            // Read here rather than held on `Open`: see `Query::Articles`. Two of the 974 corpus
            // documents state a `/Threads` entry at all, one of them an empty array and one a
            // null, so a list built at launch would be empty 999 times in a thousand.
            Query::Articles => {
                Answer::Articles(pdf_model::article::Articles::read(&open.document).threads)
            }
            // Read on demand for the same reason: not one of the 974 corpus documents states a
            // `/Collection`, and a read at launch would walk a folder tree that is never there.
            Query::Collection => collection(open),
            Query::PageLabel(index) => label(open, index).map_or(Answer::None, Answer::Label),
            Query::Thumbnail(index) => pdf_model::Pages::new(&open.document)
                .get(index)
                .and_then(|page| pdf_model::thumbnail::read(&open.document, &page.dict))
                .and_then(Result::ok)
                .map_or(Answer::None, Answer::Thumbnail),
            Query::AttachmentPreview(name) => {
                attachment_preview(open, name).map_or(Answer::None, Answer::Thumbnail)
            }
            // §7.6.4.2's bit 3 was asked by `Command::Print`, so this answers only inside the
            // grant that command made: no job, no page. The interpretation is fresh and not kept
            // — a print job asks for each page once, and a thousand-page document's lists would
            // otherwise sit in this crate for as long as the document is open.
            Query::PrintPage(index) => open
                .printing
                .and_then(|sheet| print_page(open, index, sheet))
                .map_or(Answer::None, |page| Answer::PrintPage(Box::new(page))),
            Query::LinkAt(at) => Answer::Link(
                self.user_space(open, at)
                    .and_then(|(page, (x, y))| interact::link_at(open, page, x, y))
                    .is_some(),
            ),
            Query::FieldAt(at) => self
                .user_space(open, at)
                .and_then(|(page, (x, y))| {
                    let object = open.placed_page(page)?;
                    pdf_model::view::field_at(&open.document, object, x, y)
                })
                .map_or(Answer::None, |name| {
                    // The value is the *view*'s and the names are the document's, which is why
                    // they are gathered here rather than inside `field_at`: `pdf_model::view`'s
                    // walk knows the widget and this knows what has been typed into it.
                    // The characters and whether they *are* the characters, from one reading:
                    // a host that asked twice could be told the string is the field's own after
                    // being handed the bullets. ADR 0247.
                    let value = open.view.field_value(&open.document, &name.qualified);
                    Answer::Field { name, value }
                }),
            Query::Fields => Answer::Fields(self.form_fields(open)),
            Query::Caret { at, offset } => self
                .caret(open, at, offset)
                .map_or(Answer::None, |(from, to)| Answer::Caret { from, to }),
            Query::Offset { at, point } => self
                .offset(open, at, point)
                .map_or(Answer::None, Answer::Offset),
            Query::FieldSelection { at, from, to } => self
                .field_selection(open, at, (from, to))
                .map_or(Answer::None, Answer::FieldSelection),
            Query::FreeTextAt { at } => self
                .user_space(open, at)
                .and_then(|(page, (x, y))| {
                    let object = open.placed_page(page)?;
                    open.view.free_text_at(&open.document, object, x, y)
                })
                .map_or(Answer::None, |(annotation, text)| Answer::FreeText {
                    annotation,
                    text,
                }),
            Query::Measure(points) => {
                self.measured(open, points)
                    .map_or(Answer::None, |(traced, located)| Answer::Measured {
                        traced,
                        located,
                    })
            }
            Query::Dirty => Answer::Dirty(open.dirty()),
            Query::TimerDue => Answer::None,
            Query::Properties => Answer::Properties {
                information: pdf_model::metadata::Information::read(&open.document),
                metadata: pdf_model::xmp::Xmp::document(&open.document),
            },
            Query::Opening => {
                Answer::Opening(pdf_model::viewer_preferences::Opening::read(&open.document))
            }
            Query::Preferences => Answer::Preferences(
                pdf_model::viewer_preferences::ViewerPreferences::read(&open.document),
            ),
            Query::Find(needle) => Answer::Found(self.found(open, needle)),
            Query::Focus => self
                .focus_quad(open)
                .map_or(Answer::None, |(object, quad)| Answer::Focus {
                    object,
                    quad,
                }),
            Query::Highlight => Answer::Highlighted(self.highlight_quads(open)),
            Query::Popups => Answer::Popups(self.popup_windows(open)),
            Query::Selection => self.selected(open).map_or(Answer::None, Answer::Selected),
            Query::LogicalSelection => {
                Self::logical_selection(open).map_or(Answer::None, Answer::LogicalSelection)
            }
            // `Answer::None` for a tier-2 host, which hands back no pixels at all, and a list —
            // possibly empty — for a tier-1 host. The two are told apart by the same flag the
            // pixel budget is: a host that has answered `Rendered::Presented` holds its own.
            //
            // **A page the host answered `Rendered::Listed` for is simply not in the list**, and
            // that is the answer rather than an omission: this crate is holding no pixels of it
            // because the host took its display list, and the host knows which pages those are —
            // it said so. What matters is that the question goes on being answered for the
            // page's neighbours, which is what `Rendered::Presented` could not have left true.
            Query::Frame if !self.holds_rasters => Answer::None,
            Query::Frame => Answer::Frame(
                open.on_screen
                    .iter()
                    .filter_map(|on_screen| {
                        Some(FrameView {
                            page: on_screen.page,
                            raster: on_screen.frame.as_ref()?,
                            origin: on_screen.origin,
                        })
                    })
                    .collect(),
            ),
            Query::AccessibilityTree => Answer::Accessibility(self.accessibility(open)),
            // One entry per page the arrangement is showing and this crate has read — the same
            // population `Answer::Frame` and `Query::PageGeometry` answer over, and the same
            // reason: a host given the current page's sentences for a screen holding four would
            // be reassuring a person about three pages nothing looked at.
            Query::Reports => Answer::Reports(
                open.on_screen
                    .iter()
                    .filter_map(|on_screen| {
                        Some(PageReports {
                            page: on_screen.page,
                            notes: &on_screen.interpreted.as_ref()?.reports,
                        })
                    })
                    .collect(),
            ),
            // An **empty list** rather than a tally of zeroes where no page has been interpreted:
            // "nothing was lost" and "nothing has been read yet" are different answers, and a
            // host that showed the first for the second would be reassuring a person about a
            // page it has not looked at. `Answer::None` stays what it is for every question here
            // — no document is focused.
            Query::Readback => Answer::Readback(
                open.on_screen
                    .iter()
                    .filter_map(|on_screen| {
                        Some(PageReadback {
                            page: on_screen.page,
                            shortfall: on_screen.interpreted.as_ref()?.shortfall,
                        })
                    })
                    .collect(),
            ),
        }
    }
}

fn print_page(open: &Open, index: usize, sheet: crate::Sheet) -> Option<crate::PrintPage> {
    let read = crate::open::interpret(open, index)?;
    let list = Arc::new(read.interpretation.display_list);
    let mut reports = read.reports;
    let target = match TargetSpec::for_page(&list, sheet.scale, MAX_PIXELS) {
        Ok(target) => Some(target),
        // Trap 5: a page this crate will not rasterise at the printer's resolution is said out
        // loud rather than handed back as an empty sheet. The sentence is the backend's own,
        // which already names the budget and what was asked of it.
        Err(refusal) => {
            reports.push(format!(
                "page {} will not print: {refusal}",
                index.saturating_add(1)
            ));
            None
        }
    };
    Some(crate::PrintPage {
        page: index,
        list,
        target,
        reports,
    })
}

/// §12.3.5's collection, and the document §12.3.5.1's `/D` says to present first.
///
/// The resolution happens here rather than in a host because Table 153's `/D` "identifies an
/// entry in the `EmbeddedFiles` name tree" and that tree is the document's — a panel holding the
/// collection dictionary has no way to tell a `/D` naming a file from a `/D` naming nothing.
/// [`Answer::Collection`] carries both halves for that reason.
fn collection(open: &Open) -> Answer<'static> {
    pdf_model::collection::Collection::read(&open.document).map_or(Answer::None, |collection| {
        let initial = collection.initial_document(&open.document);
        // Table 153's `/Sort` is a `shall` about the order rows stand in, and the values it
        // orders by need both the document and the attachments — so it is resolved once here
        // rather than by each panel that draws the list (ADR 1168).
        let order = pdf_model::collection::sorted_keys(
            &open.document,
            &collection,
            &open.view.attachments(&open.document),
        )
        .unwrap_or_default();
        Answer::Collection {
            collection,
            initial,
            order,
        }
    })
}

/// §12.4.2's label for a page, where the document states a non-empty one.
pub(super) fn label(open: &Open, index: usize) -> Option<String> {
    open.labels.label(index).filter(|label| !label.is_empty())
}

/// §8.11.4.3's `/Order`, turned into what a layer panel shows.
///
/// **Table 99's `/ListMode` is applied here and nowhere else**, because it is the one entry of
/// that table whose answer depends on the window rather than on the file:
///
/// > A name specifying which optional content groups in the Order array shall be displayed to
/// > the user. Valid values shall be: AllPages Display all groups in the Order array.
/// > VisiblePages Display only those groups in the Order array that are referenced by one or
/// > more visible pages.
///
/// This window shows one page at a time, so "one or more visible pages" is the page it is
/// showing — the same derivation §12.6.3's `/PV` and `/PO` take (ADR 0164). `pdf_model` supplies
/// the half
/// that is about the file, `groups_referenced_by`, and this supplies the half that is about the
/// window. One corpus document states the entry (`visibility_expressions.pdf`, on a scan of every
/// uncompressed `/ListMode` in all 974), and it states `VisiblePages`.
///
/// A collection whose children all disappear disappears with them: §8.11.4.3 makes a label
/// "non-selectable" and a heading over nothing is not what the clause asked to be displayed.
fn layers(open: &Open) -> Vec<Layer> {
    let Some(content) = open.view.optional_content() else {
        return Vec::new();
    };
    let shown = match content.list_mode() {
        ListMode::AllPages => None,
        ListMode::VisiblePages => open
            .shown_page()
            .map(|page| pdf_model::optional_content::groups_referenced_by(&open.document, page)),
    };
    build_layers(open, content, content.presentation(), shown.as_ref())
}

/// One level of `/Order`, and its children.
///
/// `shown` is `None` where every group is displayed, and the set a page references where
/// `/ListMode` is `VisiblePages`.
fn build_layers(
    open: &Open,
    content: &OptionalContent,
    entries: &[Presented],
    shown: Option<&BTreeSet<ObjectId>>,
) -> Vec<Layer> {
    entries
        .iter()
        .filter_map(|entry| match entry {
            Presented::Group(group) => {
                if shown.is_some_and(|shown| !shown.contains(group)) {
                    return None;
                }
                Some(Layer::Group {
                    group: *group,
                    name: content.name(&open.document, *group),
                    // A group `/Order` names but `/OCGs` does not is not a group this document
                    // configured, and Table 99 does not say what it is. It reads as on, which is
                    // what content governed by no group is.
                    on: content.state(*group).unwrap_or(true),
                    locked: content.is_locked(*group),
                })
            }
            Presented::Collection { label, children } => {
                let children = build_layers(open, content, children, shown);
                if shown.is_some() && children.is_empty() {
                    return None;
                }
                Some(Layer::Collection {
                    label: label.clone(),
                    children,
                })
            }
        })
        .collect()
}

/// What the attachments panel lists: the document's files, and §14.13.4's files of the page in
/// view.
///
/// §14.13.4 states the second population and states it of a *page*:
///
/// > One or more files may be associated with any PDF page by including a file specification
/// > dictionary (7.11.3, "File specification dictionaries") for each file as one of the members
/// > of the array value of the AF key in the appropriate page dictionary (7.7.3.3, "Page
/// > objects"). The relationship that the associated files have to the page is supplied by the
/// > AFRelationship key in each file specification dictionary.
///
/// **The page in view rather than every page, and that is principle 2 rather than a shortcut.**
/// A panel listing every page's associated files has to walk the whole page tree, which is what
/// `CLAUDE.md` forbids on the launch path and what every host would pay for, since each of them
/// asks [`Query::Attachments`] when a document opens — the same measurement ADR 0295 took for
/// §12.5.6.15's annotations, 78 to 123 ms cold over ISO 32000-2's 1023 pages. The page the reader
/// is on is a page this crate has already built, so the entry costs one dictionary lookup, and
/// the panel follows the reader exactly as `attached_files` follows the pointer.
///
/// `pdf_model::attachment::associated` had no caller anywhere in this tree that handed it a page
/// dictionary, which is the shape §14.13.3's catalog `/AF` and §12.5.6.15's `/FS` were both in
/// before a consumer reached them: a reading that exists, a row that calls it implemented, and a
/// payload no panel could list. ADR 1186.
///
/// One payload named twice is one file, deduplicated by the stream's identity as
/// `attachment::attachments` and [`attached_files`] both do it.
fn attachments_in_view(open: &Open) -> Vec<pdf_model::attachment::Attachment> {
    let mut out = open.view.attachments(&open.document);
    let Some(page) = open.page(open.page_index) else {
        return out;
    };
    for file in pdf_model::attachment::associated(&open.document, &page.dict) {
        if out
            .iter()
            .any(|seen| Arc::ptr_eq(&seen.stream, &file.stream))
        {
            continue;
        }
        out.push(file);
    }
    out
}

/// §12.3.6's preview picture for one attachment: its own first page's §12.3.4 `/Thumb`.
///
/// Three of Table 160's layouts are made of pictures of the attachments — a strip of thumbnails,
/// thumbnails scattered over the view, a large preview beside the metadata — and §12.3.4 is the
/// one place the standard says what a picture of a document's page is:
///
/// > The thumbnail image for a page shall be an image XObject specified by the Thumb entry in the
/// > page object
///
/// So the preview is the attachment's producer's own miniature of its first page, and this
/// program invents nothing where a file states none. `None` covers every such file: one that is
/// not a PDF, one whose first page states no `/Thumb`, one whose bytes this reader cannot decode.
/// A panel says which of its files had no picture rather than drawing a substitute (ADR 1251).
///
/// **Bounded by what already bounds a stream.** The attachment's bytes come out under
/// `pdf_syntax::Limits::max_stream_len` and the nested document parses under the same limits, so
/// a decompression bomb attached to a collection is refused by the bound that refuses one in the
/// containing file — no second number is chosen here (trap 38).
///
/// **One level.** The nested document is asked for a page's `/Thumb` and never for *its*
/// attachments, so a document embedding itself is one open rather than a walk.
fn attachment_preview(open: &Open, name: &str) -> Option<pdf_model::thumbnail::Thumbnail> {
    let file = open
        .view
        .attachments(&open.document)
        .into_iter()
        .find(|file| file.name == name)?;
    let bytes = open.document.decoded_stream_data(&file.stream)?;
    let embedded = pdf_syntax::Document::open(bytes.to_vec()).ok()?;
    let page = pdf_model::Pages::new(&embedded).get(0)?;
    pdf_model::thumbnail::read(&embedded, &page.dict)?.ok()
}
