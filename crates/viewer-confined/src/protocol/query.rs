//! Questions, host to viewer: their discriminants, encode, and the owned form the worker reads.
//!
//! The bytes are unchanged by the split that made this module (ADR 2256).

use viewer_core::Query;

use super::error::{ProtocolError, Uncarried};
use super::wire::{Reader, Writer};

/// Query discriminants. One per variant of [`viewer_core::Query`] that crosses.
mod query_kind {
    pub(in crate::protocol) const PAGE_COUNT: u8 = 1;
    pub(in crate::protocol) const CURRENT_PAGE: u8 = 2;
    pub(in crate::protocol) const PAGE_GEOMETRY: u8 = 3;
    pub(in crate::protocol) const PAGE_LABEL: u8 = 4;
    pub(in crate::protocol) const LINK_AT: u8 = 5;
    pub(in crate::protocol) const FIELD_AT: u8 = 6;
    pub(in crate::protocol) const CARET: u8 = 7;
    pub(in crate::protocol) const DIRTY: u8 = 8;
    pub(in crate::protocol) const FIND: u8 = 9;
    pub(in crate::protocol) const SELECTION: u8 = 10;
    pub(in crate::protocol) const LOGICAL_SELECTION: u8 = 11;
    pub(in crate::protocol) const FOCUS: u8 = 12;
    pub(in crate::protocol) const FRAME: u8 = 13;
    pub(in crate::protocol) const REPORTS: u8 = 14;
    // The eleven a panel is made of. Each answers with a `pdf-model` type: `protocol::panels` is
    // the encoding of those types and ADR 0223 is the argument.
    pub(in crate::protocol) const OUTLINE: u8 = 15;
    pub(in crate::protocol) const LAYERS: u8 = 16;
    pub(in crate::protocol) const ATTACHMENTS: u8 = 17;
    pub(in crate::protocol) const COLLECTION: u8 = 18;
    pub(in crate::protocol) const ARTICLES: u8 = 19;
    pub(in crate::protocol) const THUMBNAIL: u8 = 20;
    pub(in crate::protocol) const PROPERTIES: u8 = 21;
    pub(in crate::protocol) const OPENING: u8 = 22;
    pub(in crate::protocol) const PREFERENCES: u8 = 23;
    pub(in crate::protocol) const POPUPS: u8 = 24;
    pub(in crate::protocol) const ACCESSIBILITY_TREE: u8 = 25;
    // The caret's inverse and the shapes over a selected range of a field's value. ADR 0225.
    pub(in crate::protocol) const OFFSET: u8 = 26;
    pub(in crate::protocol) const FIELD_SELECTION: u8 = 27;
    // §12.7's form fields: the sixth of ADR 0235's six chrome populations.
    pub(in crate::protocol) const FIELDS: u8 = 28;
    // §12.5.6.6's annotation at a point: the way in to typing on the one markup subtype whose text
    // is the annotation. ADR 0238.
    pub(in crate::protocol) const FREE_TEXT_AT: u8 = 29;
    // Annex O's highlighted rectangle: the fragment identifier's own shape, which no host can
    // derive because no host sees the fragment. ADR 0357.
    pub(in crate::protocol) const HIGHLIGHT: u8 = 30;
    // What the page's codes cost the readback: §9.10.2's own "there is no way", counted rather than
    // reported. ADR 0422.
    pub(in crate::protocol) const READBACK: u8 = 31;
    // Where the reader is looking: the question a host holds per frame so that a worker's death
    // costs the reader nothing but the wait. ADR 0737.
    pub(in crate::protocol) const VIEW: u8 = 32;
    // One page of a print operation, at the printer's resolution. It crosses because the worker
    // holds the document and therefore the interpretation; the window process holds the printer
    // (ADR 1180).
    pub(in crate::protocol) const PRINT_PAGE: u8 = 33;
    // §12.9's measurement of a traced path. It crosses because the worker holds the document and
    // therefore the page's `/VP`, while the window process holds the pointer (ADR 1191).
    pub(in crate::protocol) const MEASURE: u8 = 34;
    // §12.3.6's preview picture for one attachment, which is that attachment's own first page's
    // §12.3.4 `/Thumb`. It crosses because only the worker holds the bytes to open (ADR 1251).
    pub(in crate::protocol) const ATTACHMENT_PREVIEW: u8 = 35;
    // When a script's timer is next owed. It crosses because the worker holds the view the timers
    // are in and the window holds the clock (ADR 1702).
    pub(in crate::protocol) const TIMER_DUE: u8 = 36;
}

/// Encodes one question.
///
/// **Every question crosses** (ADR 0223), so this function returns [`Uncarried`] for nothing. It
/// still returns a `Result` because the answers do — see [`encode_answer`](super::encode_answer), where a collection
/// value outside Table 47 and an `#[non_exhaustive]` metadata failure are the two refusals left in
/// this module besides the render round trip.
#[expect(
    clippy::unnecessary_wraps,
    reason = "the symmetry with `encode_command` and `encode_answer` is the shape a caller reads, \
              and a question this transport cannot carry is a thing that has existed twice"
)]
#[expect(
    clippy::too_many_lines,
    reason = "one arm per variant of a `viewer-core` enum, and the count is that enum's — the same \
              reason `encode_answer` carries this, and splitting it would lose the property this \
              module rests on: the compiler naming the variant nobody handled"
)]
pub(crate) fn encode_query(query: Query<'_>) -> Result<Vec<u8>, Uncarried> {
    use query_kind as k;

    let mut writer = Writer::new();
    match query {
        Query::PageCount => {
            writer.u8(k::PAGE_COUNT);
        }
        Query::CurrentPage => {
            writer.u8(k::CURRENT_PAGE);
        }
        Query::View => {
            writer.u8(k::VIEW);
        }
        Query::PageGeometry(index) => {
            writer.u8(k::PAGE_GEOMETRY).usize(index);
        }
        Query::PageLabel(index) => {
            writer.u8(k::PAGE_LABEL).usize(index);
        }
        Query::LinkAt(at) => {
            writer.u8(k::LINK_AT).point(at);
        }
        Query::FieldAt(at) => {
            writer.u8(k::FIELD_AT).point(at);
        }
        Query::Caret { at, offset } => {
            writer.u8(k::CARET).point(at).usize(offset);
        }
        Query::Offset { at, point } => {
            writer.u8(k::OFFSET).point(at).point(point);
        }
        Query::FieldSelection { at, from, to } => {
            writer
                .u8(k::FIELD_SELECTION)
                .point(at)
                .usize(from)
                .usize(to);
        }
        Query::FreeTextAt { at } => {
            writer.u8(k::FREE_TEXT_AT).point(at);
        }
        Query::Dirty => {
            writer.u8(k::DIRTY);
        }
        Query::TimerDue => {
            writer.u8(k::TIMER_DUE);
        }
        Query::Find(needle) => {
            writer.u8(k::FIND).str(needle);
        }
        Query::Selection => {
            writer.u8(k::SELECTION);
        }
        Query::LogicalSelection => {
            writer.u8(k::LOGICAL_SELECTION);
        }
        Query::Focus => {
            writer.u8(k::FOCUS);
        }
        Query::Highlight => {
            writer.u8(k::HIGHLIGHT);
        }
        Query::Frame => {
            writer.u8(k::FRAME);
        }
        Query::Reports => {
            writer.u8(k::REPORTS);
        }
        Query::Readback => {
            writer.u8(k::READBACK);
        }
        Query::Outline => {
            writer.u8(k::OUTLINE);
        }
        Query::Layers => {
            writer.u8(k::LAYERS);
        }
        Query::Attachments => {
            writer.u8(k::ATTACHMENTS);
        }
        Query::Collection => {
            writer.u8(k::COLLECTION);
        }
        Query::Articles => {
            writer.u8(k::ARTICLES);
        }
        Query::Thumbnail(index) => {
            writer.u8(k::THUMBNAIL).usize(index);
        }
        Query::AttachmentPreview(name) => {
            writer.u8(k::ATTACHMENT_PREVIEW).str(name);
        }
        Query::PrintPage(index) => {
            writer.u8(k::PRINT_PAGE).usize(index);
        }
        Query::Properties => {
            writer.u8(k::PROPERTIES);
        }
        Query::Opening => {
            writer.u8(k::OPENING);
        }
        Query::Preferences => {
            writer.u8(k::PREFERENCES);
        }
        Query::Popups => {
            writer.u8(k::POPUPS);
        }
        Query::Fields => {
            writer.u8(k::FIELDS);
        }
        Query::AccessibilityTree => {
            writer.u8(k::ACCESSIBILITY_TREE);
        }
        Query::Measure(points) => {
            writer.u8(k::MEASURE).usize(points.len());
            for [x, y] in points {
                writer.point((*x, *y));
            }
        }
    }
    Ok(writer.finish())
}

/// A question read on the confined side.
///
/// Owned, because [`Query::Find`] borrows the string a host already has and there is no such
/// string on this side of the pipe — it arrived in the message and has to outlive the reader.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum OwnedQuery {
    /// A question carrying nothing a caller has to keep alive.
    Plain(PlainQuery),
    /// [`Query::Find`], with the string it searches for.
    Find(String),
    /// [`Query::AttachmentPreview`], with the `/EmbeddedFiles` key it names.
    ///
    /// Owned for [`Self::Find`]'s reason: the key belongs to the panel that asked, and nothing on
    /// this side of the transport is holding it.
    AttachmentPreview(String),
    /// [`Query::Measure`], with the path it measures.
    ///
    /// Owned for [`Self::Find`]'s reason: the slice a host passes is the host's own gesture, and
    /// on this side of the pipe the points arrived in the message.
    Measure(Vec<[f32; 2]>),
}

/// Every carried question except [`Query::Find`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum PlainQuery {
    PageCount,
    CurrentPage,
    View,
    PageGeometry(usize),
    PageLabel(usize),
    LinkAt((f32, f32)),
    FieldAt((f32, f32)),
    Caret {
        at: (f32, f32),
        offset: usize,
    },
    Offset {
        at: (f32, f32),
        point: (f32, f32),
    },
    FieldSelection {
        at: (f32, f32),
        from: usize,
        to: usize,
    },
    FreeTextAt {
        at: (f32, f32),
    },
    Dirty,
    TimerDue,
    Selection,
    LogicalSelection,
    Focus,
    Highlight,
    Frame,
    Reports,
    Readback,
    Outline,
    Layers,
    Attachments,
    Collection,
    Articles,
    Thumbnail(usize),
    PrintPage(usize),
    Properties,
    Opening,
    Preferences,
    Popups,
    Fields,
    AccessibilityTree,
}

impl OwnedQuery {
    /// The question, borrowed for as long as this value lives.
    pub(crate) fn as_query(&self) -> Query<'_> {
        match self {
            Self::Find(needle) => Query::Find(needle),
            Self::AttachmentPreview(name) => Query::AttachmentPreview(name),
            Self::Plain(plain) => match *plain {
                PlainQuery::PageCount => Query::PageCount,
                PlainQuery::CurrentPage => Query::CurrentPage,
                PlainQuery::View => Query::View,
                PlainQuery::PageGeometry(index) => Query::PageGeometry(index),
                PlainQuery::PageLabel(index) => Query::PageLabel(index),
                PlainQuery::LinkAt(at) => Query::LinkAt(at),
                PlainQuery::FieldAt(at) => Query::FieldAt(at),
                PlainQuery::Caret { at, offset } => Query::Caret { at, offset },
                PlainQuery::Offset { at, point } => Query::Offset { at, point },
                PlainQuery::FieldSelection { at, from, to } => {
                    Query::FieldSelection { at, from, to }
                }
                PlainQuery::FreeTextAt { at } => Query::FreeTextAt { at },
                PlainQuery::Dirty => Query::Dirty,
                PlainQuery::TimerDue => Query::TimerDue,
                PlainQuery::Selection => Query::Selection,
                PlainQuery::LogicalSelection => Query::LogicalSelection,
                PlainQuery::Focus => Query::Focus,
                PlainQuery::Highlight => Query::Highlight,
                PlainQuery::Frame => Query::Frame,
                PlainQuery::Reports => Query::Reports,
                PlainQuery::Readback => Query::Readback,
                PlainQuery::Outline => Query::Outline,
                PlainQuery::Layers => Query::Layers,
                PlainQuery::Attachments => Query::Attachments,
                PlainQuery::Collection => Query::Collection,
                PlainQuery::Articles => Query::Articles,
                PlainQuery::Thumbnail(index) => Query::Thumbnail(index),
                PlainQuery::PrintPage(index) => Query::PrintPage(index),
                PlainQuery::Properties => Query::Properties,
                PlainQuery::Opening => Query::Opening,
                PlainQuery::Preferences => Query::Preferences,
                PlainQuery::Popups => Query::Popups,
                PlainQuery::Fields => Query::Fields,
                PlainQuery::AccessibilityTree => Query::AccessibilityTree,
            },
            Self::Measure(points) => Query::Measure(points),
        }
    }
}

/// Reads one question.
///
/// # Errors
///
/// [`ProtocolError`] where a field is truncated, a discriminant is not one this build defines,
/// or bytes are left over.
pub(crate) fn decode_query(bytes: &[u8]) -> Result<OwnedQuery, ProtocolError> {
    use query_kind as k;

    let mut reader = Reader::new(bytes);
    let what = "a query";
    let query = match reader.u8(what)? {
        k::PAGE_COUNT => OwnedQuery::Plain(PlainQuery::PageCount),
        k::CURRENT_PAGE => OwnedQuery::Plain(PlainQuery::CurrentPage),
        k::VIEW => OwnedQuery::Plain(PlainQuery::View),
        k::PAGE_GEOMETRY => {
            OwnedQuery::Plain(PlainQuery::PageGeometry(reader.usize("a page index")?))
        }
        k::PAGE_LABEL => OwnedQuery::Plain(PlainQuery::PageLabel(reader.usize("a page index")?)),
        k::LINK_AT => OwnedQuery::Plain(PlainQuery::LinkAt(reader.point("a point")?)),
        k::FIELD_AT => OwnedQuery::Plain(PlainQuery::FieldAt(reader.point("a point")?)),
        k::CARET => OwnedQuery::Plain(PlainQuery::Caret {
            at: reader.point("a point")?,
            offset: reader.usize("a caret offset")?,
        }),
        k::OFFSET => OwnedQuery::Plain(PlainQuery::Offset {
            at: reader.point("a point")?,
            point: reader.point("a point")?,
        }),
        k::FIELD_SELECTION => OwnedQuery::Plain(PlainQuery::FieldSelection {
            at: reader.point("a point")?,
            from: reader.usize("a selection offset")?,
            to: reader.usize("a selection offset")?,
        }),
        k::FREE_TEXT_AT => OwnedQuery::Plain(PlainQuery::FreeTextAt {
            at: reader.point("a point")?,
        }),
        k::DIRTY => OwnedQuery::Plain(PlainQuery::Dirty),
        k::TIMER_DUE => OwnedQuery::Plain(PlainQuery::TimerDue),
        k::FIND => OwnedQuery::Find(reader.string("a search string")?),
        k::ATTACHMENT_PREVIEW => {
            OwnedQuery::AttachmentPreview(reader.string("an attachment's name")?)
        }
        k::SELECTION => OwnedQuery::Plain(PlainQuery::Selection),
        k::LOGICAL_SELECTION => OwnedQuery::Plain(PlainQuery::LogicalSelection),
        k::FOCUS => OwnedQuery::Plain(PlainQuery::Focus),
        k::HIGHLIGHT => OwnedQuery::Plain(PlainQuery::Highlight),
        k::FRAME => OwnedQuery::Plain(PlainQuery::Frame),
        k::REPORTS => OwnedQuery::Plain(PlainQuery::Reports),
        k::READBACK => OwnedQuery::Plain(PlainQuery::Readback),
        k::OUTLINE => OwnedQuery::Plain(PlainQuery::Outline),
        k::LAYERS => OwnedQuery::Plain(PlainQuery::Layers),
        k::ATTACHMENTS => OwnedQuery::Plain(PlainQuery::Attachments),
        k::COLLECTION => OwnedQuery::Plain(PlainQuery::Collection),
        k::ARTICLES => OwnedQuery::Plain(PlainQuery::Articles),
        k::THUMBNAIL => OwnedQuery::Plain(PlainQuery::Thumbnail(reader.usize("a page index")?)),
        k::PRINT_PAGE => OwnedQuery::Plain(PlainQuery::PrintPage(reader.usize("a page index")?)),
        k::PROPERTIES => OwnedQuery::Plain(PlainQuery::Properties),
        k::OPENING => OwnedQuery::Plain(PlainQuery::Opening),
        k::PREFERENCES => OwnedQuery::Plain(PlainQuery::Preferences),
        k::POPUPS => OwnedQuery::Plain(PlainQuery::Popups),
        k::FIELDS => OwnedQuery::Plain(PlainQuery::Fields),
        k::ACCESSIBILITY_TREE => OwnedQuery::Plain(PlainQuery::AccessibilityTree),
        k::MEASURE => OwnedQuery::Measure(reader.list("a measured path", |reader| {
            let (x, y) = reader.point("a measured point")?;
            Ok([x, y])
        })?),
        value => {
            return Err(ProtocolError::Unrecognised {
                what,
                value: u32::from(value),
            });
        }
    };
    reader.end(what)?;
    Ok(query)
}
