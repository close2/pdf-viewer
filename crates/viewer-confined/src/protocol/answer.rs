//! Answers, viewer to host: their discriminants, encode, and decode with the held lists.
//!
//! The bytes are unchanged by the split that made this module (ADR 2256).

use pdf_render::{RasterFormat, Size};
use viewer_core::{Answer, PageGeometry};

use crate::Reply;

use super::error::{ProtocolError, Uncarried};
use super::marks::{Marks, Placed};
use super::measurement::{decode_located, decode_traced, encode_located, encode_traced};
use super::payload::{
    HeldLists, decode_list_payload, decode_raster_payload, decode_window_payload,
};
use super::view_fields::{decode_viewing, encode_viewing, tagging_code, tagging_of};
use super::wire::{Reader, Writer, read_quads};
use super::{PAYLOAD_LIST, PAYLOAD_RASTER, PAYLOAD_WINDOW};
use super::{display_list, panels};

/// Answer discriminants. One per variant of [`viewer_core::Answer`] that crosses.
pub(super) mod answer_kind {
    pub(in crate::protocol) const NONE: u8 = 1;
    pub(in crate::protocol) const COUNT: u8 = 2;
    pub(in crate::protocol) const PAGE: u8 = 3;
    pub(in crate::protocol) const GEOMETRY: u8 = 4;
    pub(in crate::protocol) const LABEL: u8 = 5;
    pub(in crate::protocol) const LINK: u8 = 6;
    pub(in crate::protocol) const SELECTED: u8 = 7;
    pub(in crate::protocol) const FIELD: u8 = 8;
    pub(in crate::protocol) const CARET: u8 = 9;
    pub(in crate::protocol) const FOUND: u8 = 10;
    pub(in crate::protocol) const DIRTY: u8 = 11;
    pub(in crate::protocol) const FOCUS: u8 = 12;
    pub(in crate::protocol) const LOGICAL_SELECTION: u8 = 13;
    pub(in crate::protocol) const FRAME: u8 = 14;
    pub(in crate::protocol) const REPORTS: u8 = 15;
    // The eleven a panel is made of. `protocol::panels` is what each of them costs.
    pub(in crate::protocol) const OUTLINE: u8 = 16;
    pub(in crate::protocol) const LAYERS: u8 = 17;
    pub(in crate::protocol) const ATTACHMENTS: u8 = 18;
    pub(in crate::protocol) const COLLECTION: u8 = 19;
    pub(in crate::protocol) const ARTICLES: u8 = 20;
    pub(in crate::protocol) const THUMBNAIL: u8 = 21;
    pub(in crate::protocol) const PROPERTIES: u8 = 22;
    pub(in crate::protocol) const OPENING: u8 = 23;
    pub(in crate::protocol) const PREFERENCES: u8 = 24;
    pub(in crate::protocol) const POPUPS: u8 = 25;
    pub(in crate::protocol) const ACCESSIBILITY: u8 = 26;
    // The caret's inverse and a field selection's shapes. ADR 0225.
    pub(in crate::protocol) const OFFSET: u8 = 27;
    pub(in crate::protocol) const FIELD_SELECTION: u8 = 28;
    // §12.7's form fields, the twelfth answer a panel — or a native form — is made of. ADR 0235.
    pub(in crate::protocol) const FIELDS: u8 = 29;
    // §12.5.6.6's annotation and its `/Contents`. ADR 0238.
    pub(in crate::protocol) const FREE_TEXT: u8 = 30;
    // Annex O's highlighted rectangles. ADR 0357.
    pub(in crate::protocol) const HIGHLIGHTED: u8 = 31;
    // The per-code counts (ADR 0422), four of them (ADR 0764).
    pub(in crate::protocol) const READBACK: u8 = 32;
    // Where the reader is looking. ADR 0737.
    pub(in crate::protocol) const VIEW: u8 = 33;
    // One page of a print operation, as marks and a target — `query_kind::PRINT_PAGE`'s answer.
    pub(in crate::protocol) const PRINT_PAGE: u8 = 34;
    // §12.9's formatted measurement: strings Table 267's arrays produced and §12.10's reading
    // beside them (ADR 1191).
    pub(in crate::protocol) const MEASURED: u8 = 35;
    // `query_kind::TIMER_DUE`'s answer: a flag, and the milliseconds where it is set (ADR 1702).
    pub(in crate::protocol) const TIMER_DUE: u8 = 36;
}

/// Encodes one answer.
///
/// `marks` is what the confined process decided each page crosses as, and it is a parameter
/// rather than something this function works out because the decision is made where the display
/// list is — see [`Marks`]. An empty one is a viewer whose every page crosses as pixels, which is
/// what the boundary carried before ADR 0607.
///
/// # Errors
///
/// [`Uncarried`] in three places, and each names what it refused: a raster in a pixel layout this
/// build cannot spell, a §7.11.6 collection value outside Table 47's three kinds, and a metadata
/// failure `pdf_model::xmp` added after this build. The eleven panel answers all cross (ADR 0223).
#[expect(
    clippy::too_many_lines,
    reason = "one arm per variant of a `viewer-core` enum, and the count is that enum's. Splitting it would put half the vocabulary in another function and lose the property the whole module rests on: the compiler naming the variant nobody handled"
)]
pub(crate) fn encode_answer(answer: &Answer<'_>, marks: &Marks) -> Result<Vec<u8>, Uncarried> {
    use answer_kind as k;

    let mut writer = Writer::new();
    match answer {
        Answer::None => {
            writer.u8(k::NONE);
        }
        Answer::Count(count) => {
            writer.u8(k::COUNT).usize(*count);
        }
        Answer::View(view) => {
            writer.u8(k::VIEW);
            encode_viewing(&mut writer, *view);
        }
        Answer::Page {
            document,
            index,
            label,
            of,
        } => {
            writer
                .u8(k::PAGE)
                .document(*document)
                .usize(*index)
                .option_str(label.as_deref())
                .usize(*of);
        }
        Answer::Geometry(geometry) => {
            writer
                .u8(k::GEOMETRY)
                .f32(geometry.page.width)
                .f32(geometry.page.height)
                .f32(geometry.scale)
                .u32(geometry.width)
                .u32(geometry.height)
                .point(geometry.origin);
        }
        Answer::Label(label) => {
            writer.u8(k::LABEL).str(label);
        }
        Answer::Link(there) => {
            writer.u8(k::LINK).bool(*there);
        }
        Answer::Selected(selected) => {
            writer.u8(k::SELECTED).str(&selected.text);
            writer.usize(selected.quads.len());
            for quad in &selected.quads {
                writer.quad(*quad);
            }
        }
        Answer::Field { name, value } => {
            writer
                .u8(k::FIELD)
                .str(&name.qualified)
                .option_str(name.alternative.as_deref());
            panels::encode_shown(&mut writer, value.as_ref());
        }
        Answer::Caret { from, to } => {
            writer.u8(k::CARET).point(*from).point(*to);
        }
        Answer::Offset(offset) => {
            writer.u8(k::OFFSET).usize(*offset);
        }
        Answer::FieldSelection(quads) => {
            writer.u8(k::FIELD_SELECTION).usize(quads.len());
            for quad in quads {
                writer.quad(*quad);
            }
        }
        Answer::Found(occurrences) => {
            writer.u8(k::FOUND).usize(occurrences.len());
            for occurrence in occurrences {
                writer.usize(occurrence.len());
                for quad in occurrence {
                    writer.quad(*quad);
                }
            }
        }
        Answer::Highlighted(quads) => {
            writer.u8(k::HIGHLIGHTED).usize(quads.len());
            for quad in quads {
                writer.quad(*quad);
            }
        }
        Answer::FreeText { annotation, text } => {
            writer.u8(k::FREE_TEXT).object(*annotation).str(text);
        }
        Answer::Dirty(dirty) => {
            writer.u8(k::DIRTY).bool(*dirty);
        }
        Answer::TimerDue(due) => {
            writer
                .u8(k::TIMER_DUE)
                .bool(due.is_some())
                .u32(due.unwrap_or(0));
        }
        Answer::Focus { object, quad } => {
            writer.u8(k::FOCUS).object(*object).quad(*quad);
        }
        Answer::LogicalSelection(text) => {
            writer.u8(k::LOGICAL_SELECTION).str(text);
        }
        Answer::Frame(frames) => {
            // **A payload choice per page since ADR 0607, made where the two sizes are known.**
            // The marks are what a host holding a graphics device needs, because a process
            // holding one cannot be confined; the pixels stay for the pages where the pixels are
            // smaller, which is a scan, whose decoded samples are its display list.
            //
            // **A list since Table 29's `/PageLayout` was obeyed**: `OneColumn` puts several
            // pages in one window, and a wire that carried the first of them would show the host
            // a continuous view with a hole in it.
            //
            // **The two halves are held in two places since ADR 0640, and this is where they
            // meet.** A page crossing as marks answers `viewer_core::Rendered::Listed`, which is
            // how the worker gets out of drawing a page whose pixels it does not send — so the
            // viewer holds nothing for it and `Answer::Frame` is silent about it, by design.
            // Keyed by page number, so the reply is in the page order this answer already
            // arrives in whichever half a page came from.
            //
            // The pixels are inserted second, so that a page both halves claim crosses as the
            // pixels the viewer is holding *now*. That case is unreachable as the two crates
            // stand — the store forgets a page it did not record marks for — and the tie-break
            // is stated rather than assumed, because it is the older `Marks::of`'s rule and it
            // rested on a size comparison this arm no longer has a second side for.
            let mut placed: std::collections::BTreeMap<usize, Placed<'_>> = marks
                .by_page
                .iter()
                .map(|(page, crossed)| (*page, Placed::Marks(crossed)))
                .collect();
            // A page drawn here as the window shows it crosses once a window of it is drawn;
            // before that the viewport shows none of it (ADR 1986).
            for (page, held) in &marks.windows {
                if let Some((window, raster)) = &held.drawn {
                    placed.insert(*page, Placed::Window(held, *window, raster));
                }
            }
            for frame in frames {
                placed.insert(frame.page, Placed::Pixels(frame));
            }
            writer.u8(k::FRAME).usize(placed.len());
            for (page, entry) in placed {
                writer.usize(page);
                match entry {
                    // The target is written out rather than left to be rebuilt from the list's
                    // page size and a scale: it carries the y flip and any tile offset, and
                    // `doc/traps/the-interactive-loop.md`'s trap 12a is what happens when the two
                    // spaces are assumed to be one.
                    Placed::Marks(crossed) => {
                        writer
                            .u8(PAYLOAD_LIST)
                            .u32(crossed.target.width)
                            .u32(crossed.target.height);
                        display_list::write_transform(&mut writer, crossed.target.transform);
                        writer.bytes(&crossed.bytes);
                        writer.point(crossed.origin);
                    }
                    // `RasterFormat` is written out rather than assumed, because a second format
                    // would otherwise be read as the first. Exhaustive since ADR 0247: it is no
                    // longer `#[non_exhaustive]`, so a second pixel layout fails to compile here
                    // and has to be given a wire byte deliberately. The *reading* side keeps its
                    // refusal, because a byte arriving from the confined process is a claim
                    // rather than a variant.
                    Placed::Pixels(frame) => {
                        let format = match frame.raster.format {
                            RasterFormat::Rgba8 => 0,
                        };
                        writer
                            .u8(PAYLOAD_RASTER)
                            .u32(frame.raster.width)
                            .u32(frame.raster.height)
                            .u8(format)
                            .bytes(&frame.raster.data);
                        writer.point(frame.origin);
                    }
                    // The whole target, then the window's corner in it and its pixels, so the
                    // host places the window where this process drew it (ADR 1986).
                    Placed::Window(held, window, raster) => {
                        let format = match raster.format {
                            RasterFormat::Rgba8 => 0,
                        };
                        writer
                            .u8(PAYLOAD_WINDOW)
                            .u32(held.target.width)
                            .u32(held.target.height);
                        display_list::write_transform(&mut writer, held.target.transform);
                        writer
                            .u32(window.x)
                            .u32(window.y)
                            .u32(raster.width)
                            .u32(raster.height)
                            .u8(format)
                            .bytes(&raster.data);
                        writer.point(held.origin);
                    }
                }
            }
        }
        // One entry per page the arrangement shows, each carrying its page: the same shape
        // `Answer::Frame` takes and for the same reason (ADR 0445).
        Answer::Reports(pages) => {
            writer.u8(k::REPORTS).usize(pages.len());
            for page in pages {
                writer.usize(page.page).usize(page.notes.len());
                for note in page.notes {
                    writer.str(note);
                }
            }
        }
        Answer::Readback(pages) => {
            writer.u8(k::READBACK).usize(pages.len());
            for page in pages {
                // Destructured rather than written field by field off `shortfall`, so that a
                // field added to either struct fails to compile here instead of crossing as a
                // zero.
                let pdf_model::content::Shortfall {
                    unnamed:
                        pdf_model::content::UnnamedCodes {
                            empty_mapping,
                            incomplete_to_unicode,
                            unlisted_name,
                            unnamed_cid,
                            unaddressable_cid,
                            unnamed_glyph,
                        },
                    without_a_glyph,
                    reaching_a_blank_glyph,
                    without_a_vertical_form,
                } = page.shortfall;
                writer
                    .usize(page.page)
                    .usize(empty_mapping)
                    .usize(incomplete_to_unicode)
                    .usize(unlisted_name)
                    .usize(unnamed_cid)
                    .usize(unaddressable_cid)
                    .usize(unnamed_glyph)
                    .usize(without_a_glyph)
                    .usize(reaching_a_blank_glyph)
                    .usize(without_a_vertical_form);
            }
        }
        Answer::Accessibility(pages) => {
            writer.u8(k::ACCESSIBILITY).usize(pages.len());
            for page in pages {
                writer.usize(page.page);
                panels::encode_accessibility(&mut writer, &page.nodes);
                panels::encode_accessibility(&mut writer, &page.widgets);
                writer.u8(tagging_code(page.tagging));
            }
        }
        Answer::Outline(outline) => {
            writer.u8(k::OUTLINE);
            panels::encode_outline(&mut writer, outline);
        }
        Answer::Layers(layers) => {
            writer.u8(k::LAYERS);
            panels::encode_layers(&mut writer, layers);
        }
        Answer::Attachments(attachments) => {
            writer.u8(k::ATTACHMENTS);
            panels::encode_attachments(&mut writer, attachments);
        }
        Answer::Articles(threads) => {
            writer.u8(k::ARTICLES);
            panels::encode_articles(&mut writer, threads);
        }
        Answer::Collection {
            collection,
            initial,
            order,
        } => {
            writer.u8(k::COLLECTION);
            panels::encode_collection(&mut writer, collection)?;
            panels::encode_initial(&mut writer, initial);
            // Table 153's `/Sort` applied, as `/EmbeddedFiles` keys: the *answer* crosses rather
            // than the values it was computed from, which is the same division §12.3.5.1's `/D`
            // takes one field up (ADR 1168).
            writer.strings(order);
        }
        Answer::Thumbnail(thumbnail) => {
            writer.u8(k::THUMBNAIL);
            panels::encode_thumbnail(&mut writer, thumbnail);
        }
        // The list is encoded whole rather than through `Marks`: that store exists to keep a
        // *frame* from crossing twice while a reader scrolls, and a print job asks for each page
        // once. A refused target crosses as its absence, with the worker's own sentence among
        // the reports — see `viewer_core::PrintPage`.
        Answer::PrintPage(page) => {
            let bytes = display_list::encode(&page.list).map_err(|_| Uncarried {
                message: "Answer::PrintPage",
                reason: "the page's marks are one of the four `display_list::Uncodable` names \
                         this format does not carry, and a print page has no pixel arm to fall \
                         back to: a frame that cannot cross as marks crosses as the raster the \
                         confined process already drew, and a page at the printer's resolution \
                         is not drawn there at all",
            })?;
            writer.u8(k::PRINT_PAGE).usize(page.page);
            match page.target {
                Some(target) => {
                    writer.u8(1).u32(target.width).u32(target.height);
                    display_list::write_transform(&mut writer, target.transform);
                }
                None => {
                    writer.u8(0);
                }
            }
            writer.bytes(&bytes).strings(&page.reports);
        }
        Answer::Measured { traced, located } => {
            writer.u8(k::MEASURED);
            encode_traced(&mut writer, traced);
            encode_located(&mut writer, located.as_ref());
        }
        Answer::Popups(popups) => {
            writer.u8(k::POPUPS);
            panels::encode_popups(&mut writer, popups);
        }
        Answer::Fields(fields) => {
            writer.u8(k::FIELDS);
            panels::encode_fields(&mut writer, fields);
        }
        Answer::Properties {
            information,
            metadata,
        } => {
            writer.u8(k::PROPERTIES);
            panels::encode_properties(&mut writer, information, metadata.as_ref())?;
        }
        Answer::Opening(opening) => {
            writer.u8(k::OPENING);
            panels::encode_opening(&mut writer, *opening);
        }
        Answer::Preferences(preferences) => {
            writer.u8(k::PREFERENCES);
            panels::encode_preferences(&mut writer, preferences);
        }
    }
    Ok(writer.finish())
}

pub(crate) fn decode_answer(bytes: &[u8]) -> Result<Reply, ProtocolError> {
    decode_answer_reusing(bytes, &mut HeldLists::default())
}

/// [`decode_answer`], handing back the same `Arc` for a list whose bytes are unchanged.
///
/// `held` is the caller's across calls — [`crate::Confined`] keeps one per worker — and it is
/// what makes an unchanged page's identity survive two [`Query::Frame`](viewer_core::Query::Frame)s (see [`HeldLists`]).
/// [`decode_answer`] is this with a throwaway, for the callers that decode one message and stop.
#[expect(
    clippy::too_many_lines,
    reason = "one arm per variant of a `viewer-core` enum, and the count is that enum's. Splitting it would put half the vocabulary in another function and lose the property the whole module rests on: the compiler naming the variant nobody handled"
)]
pub(crate) fn decode_answer_reusing(
    bytes: &[u8],
    held: &mut HeldLists,
) -> Result<Reply, ProtocolError> {
    use answer_kind as k;

    let mut reader = Reader::new(bytes);
    let what = "an answer";
    let answer = match reader.u8(what)? {
        k::NONE => Reply::None,
        k::MEASURED => Reply::Measured {
            traced: Box::new(decode_traced(&mut reader)?),
            located: decode_located(&mut reader)?,
        },
        k::COUNT => Reply::Count(reader.usize("a page count")?),
        k::VIEW => Reply::View(decode_viewing(&mut reader)?),
        k::PAGE => Reply::Page {
            document: reader.document(what)?,
            index: reader.usize("a page index")?,
            label: reader.option_string("a page label")?,
            of: reader.usize("a page count")?,
        },
        k::GEOMETRY => Reply::Geometry(PageGeometry {
            page: Size {
                width: reader.f32("a page size")?,
                height: reader.f32("a page size")?,
            },
            scale: reader.f32("a scale")?,
            width: reader.u32("a raster width")?,
            height: reader.u32("a raster height")?,
            origin: reader.point("an origin")?,
        }),
        k::LABEL => Reply::Label(reader.string("a page label")?),
        k::LINK => Reply::Link(reader.bool("a link")?),
        k::SELECTED => Reply::Selected {
            text: reader.string("the selected text")?,
            quads: read_quads(&mut reader, "a selection's shapes")?,
        },
        k::FIELD => Reply::Field {
            qualified: reader.string("a field's qualified name")?,
            alternative: reader.option_string("a field's alternative name")?,
            value: panels::decode_shown(&mut reader)?,
        },
        k::CARET => Reply::Caret {
            from: reader.point("a caret")?,
            to: reader.point("a caret")?,
        },
        k::OFFSET => Reply::Offset(reader.usize("a caret offset")?),
        k::FIELD_SELECTION => {
            Reply::FieldSelection(read_quads(&mut reader, "a field selection's shapes")?)
        }
        k::FOUND => {
            let what = "a search result";
            Reply::Found(reader.list(what, |reader| read_quads(reader, what))?)
        }
        k::HIGHLIGHTED => {
            Reply::Highlighted(read_quads(&mut reader, "Annex O's highlighted rectangles")?)
        }
        k::FREE_TEXT => Reply::FreeText {
            annotation: reader.object("a free text annotation")?,
            text: reader.string("a free text annotation's contents")?,
        },
        k::DIRTY => Reply::Dirty(reader.bool("a dirty flag")?),
        k::TIMER_DUE => {
            let held = reader.bool("whether a timer is held")?;
            let due = reader.u32("a timer's milliseconds")?;
            Reply::TimerDue(held.then_some(due))
        }
        k::FOCUS => Reply::Focus {
            object: reader.object("an annotation")?,
            quad: reader.quad("a focus ring")?,
        },
        k::LOGICAL_SELECTION => Reply::LogicalSelection(reader.string("the selected text")?),
        k::FRAME => {
            let count = reader.usize("a frame count")?;
            let mut frames = Vec::new();
            // The pages whose marks this frame carries: what `held` keeps is bounded by them,
            // and a page that crossed as pixels — or left the frame — is forgotten with the
            // frame that says so.
            let mut listed = Vec::new();
            for _ in 0..count {
                let page = reader.usize("a page index")?;
                let before = reader.remaining();
                let payload = match reader.u8("a frame payload")? {
                    PAYLOAD_RASTER => crate::Payload::Raster(decode_raster_payload(&mut reader)?),
                    PAYLOAD_LIST => {
                        listed.push(page);
                        decode_list_payload(&mut reader, page, held)?
                    }
                    PAYLOAD_WINDOW => decode_window_payload(&mut reader, held)?,
                    value => {
                        return Err(ProtocolError::Unrecognised {
                            what: "a frame payload",
                            value: u32::from(value),
                        });
                    }
                };
                let bytes = before.saturating_sub(reader.remaining());
                frames.push(crate::Framed {
                    page,
                    payload,
                    bytes,
                    origin: reader.point("an origin")?,
                });
            }
            held.retain(&listed);
            Reply::Frame(frames)
        }
        k::REPORTS => Reply::Reports(reader.list("a page's reports", |reader| {
            Ok(crate::Reported {
                page: reader.usize("a report's page")?,
                notes: reader.strings("a report's notes")?,
            })
        })?),
        k::READBACK => Reply::Readback(reader.list("a page's readback", |reader| {
            Ok(crate::ReadShort {
                page: reader.usize("a readback's page")?,
                shortfall: pdf_model::content::Shortfall {
                    unnamed: pdf_model::content::UnnamedCodes {
                        empty_mapping: reader.usize("an unnamed-code count")?,
                        incomplete_to_unicode: reader.usize("an unnamed-code count")?,
                        unlisted_name: reader.usize("an unnamed-code count")?,
                        unnamed_cid: reader.usize("an unnamed-code count")?,
                        unaddressable_cid: reader.usize("an unnamed-code count")?,
                        unnamed_glyph: reader.usize("an unnamed-code count")?,
                    },
                    without_a_glyph: reader.usize("a missing-glyph count")?,
                    reaching_a_blank_glyph: reader.usize("a blank-glyph count")?,
                    without_a_vertical_form: reader.usize("an upright vertical-form count")?,
                },
            })
        })?),
        k::OUTLINE => Reply::Outline(panels::decode_outline(&mut reader)?),
        k::LAYERS => Reply::Layers(panels::decode_layers(&mut reader)?),
        k::ATTACHMENTS => Reply::Attachments(panels::decode_attachments(&mut reader)?),
        k::COLLECTION => Reply::Collection {
            collection: Box::new(panels::decode_collection(&mut reader)?),
            initial: panels::decode_initial(&mut reader)?,
            order: reader.strings("a collection's /Sort order")?,
        },
        k::ARTICLES => Reply::Articles(panels::decode_articles(&mut reader)?),
        k::THUMBNAIL => Reply::Thumbnail(panels::decode_thumbnail(&mut reader)?),
        k::PRINT_PAGE => {
            let page = reader.usize("a print page's index")?;
            let target = match reader.u8("whether a print page has a target")? {
                0 => None,
                1 => {
                    let width = reader.u32("a print target's width")?;
                    let height = reader.u32("a print target's height")?;
                    // The same bounds every render target crosses under, and for the same
                    // reason: the worker is the untrusted side, so a dimension it states is a
                    // claim rather than a number.
                    if width > pdf_render::MAX_EXTENT || height > pdf_render::MAX_EXTENT {
                        return Err(ProtocolError::Unbuildable {
                            what: "a print target",
                            why: "a dimension past what an f32 resolves to a fraction of a pixel",
                        });
                    }
                    if u64::from(width).saturating_mul(u64::from(height)) > viewer_core::MAX_PIXELS
                    {
                        return Err(ProtocolError::Unbuildable {
                            what: "a print target",
                            why: "more pixels than a render request is held to",
                        });
                    }
                    let transform = display_list::read_transform(&mut reader, "a print target")?;
                    Some(pdf_render::TargetSpec {
                        width,
                        height,
                        transform,
                    })
                }
                value => {
                    return Err(ProtocolError::Unrecognised {
                        what: "whether a print page has a target",
                        value: u32::from(value),
                    });
                }
            };
            let list = std::sync::Arc::new(display_list::decode(
                reader.bytes("a print page's display list")?,
            )?);
            Reply::PrintPage {
                page,
                list,
                target,
                reports: reader.strings("a print page's reports")?,
            }
        }
        k::PROPERTIES => {
            let (information, metadata) = panels::decode_properties(&mut reader)?;
            Reply::Properties {
                information: Box::new(information),
                metadata,
            }
        }
        k::OPENING => Reply::Opening(panels::decode_opening(&mut reader)?),
        k::PREFERENCES => Reply::Preferences(Box::new(panels::decode_preferences(&mut reader)?)),
        k::POPUPS => Reply::Popups(panels::decode_popups(&mut reader)?),
        k::FIELDS => Reply::Fields(panels::decode_fields(&mut reader)?),
        k::ACCESSIBILITY => Reply::Accessibility(reader.list("a page's structure", |reader| {
            Ok(crate::Structured {
                page: reader.usize("a structure tree's page")?,
                nodes: panels::decode_accessibility(reader)?,
                widgets: panels::decode_accessibility(reader)?,
                tagging: tagging_of(reader.u8("which of four answers a page's structure is")?)?,
            })
        })?),
        value => {
            return Err(ProtocolError::Unrecognised {
                what,
                value: u32::from(value),
            });
        }
    };
    reader.end(what)?;
    Ok(answer)
}
