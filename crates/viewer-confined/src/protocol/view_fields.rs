//! The small fields a command shares: a page target, a zoom, a view and its tagging.
//!
//! The bytes are unchanged by the split that made this module (ADR 2256).

use viewer_core::{PageTarget, Zoom};

use super::error::ProtocolError;
use super::wire::{Reader, Writer};

pub(super) fn encode_page_target(writer: &mut Writer, target: PageTarget) {
    match target {
        PageTarget::Index(index) => {
            writer.u8(0).usize(index);
        }
        PageTarget::First => {
            writer.u8(1);
        }
        PageTarget::Last => {
            writer.u8(2);
        }
        PageTarget::Next => {
            writer.u8(3);
        }
        PageTarget::Previous => {
            writer.u8(4);
        }
        PageTarget::Relative(by) => {
            // `try_from` for `as_u64`'s reason: `isize` is at most 64 bits everywhere this
            // builds, and a saturated value is one the reader refuses rather than a wrong page.
            writer.u8(5).i64(i64::try_from(by).unwrap_or(i64::MAX));
        }
    }
}

pub(super) fn decode_page_target(reader: &mut Reader<'_>) -> Result<PageTarget, ProtocolError> {
    let what = "a page target";
    Ok(match reader.u8(what)? {
        0 => PageTarget::Index(reader.usize("a page index")?),
        1 => PageTarget::First,
        2 => PageTarget::Last,
        3 => PageTarget::Next,
        4 => PageTarget::Previous,
        5 => PageTarget::Relative(isize::try_from(reader.i64("a relative page")?).map_err(
            |_| ProtocolError::Unrecognised {
                what: "a relative page",
                value: u32::MAX,
            },
        )?),
        value => {
            return Err(ProtocolError::Unrecognised {
                what,
                value: u32::from(value),
            });
        }
    })
}

pub(super) fn encode_zoom(writer: &mut Writer, zoom: Zoom) {
    match zoom {
        Zoom::FitPage => {
            writer.u8(0);
        }
        Zoom::FitWidth => {
            writer.u8(1);
        }
        Zoom::FitHeight => {
            writer.u8(2);
        }
        Zoom::Scale(scale) => {
            writer.u8(3).f32(scale);
        }
        Zoom::In => {
            writer.u8(4);
        }
        Zoom::Out => {
            writer.u8(5);
        }
    }
}

pub(super) fn decode_zoom(reader: &mut Reader<'_>) -> Result<Zoom, ProtocolError> {
    let what = "a zoom";
    Ok(match reader.u8(what)? {
        0 => Zoom::FitPage,
        1 => Zoom::FitWidth,
        2 => Zoom::FitHeight,
        3 => Zoom::Scale(reader.f32("a magnification")?),
        4 => Zoom::In,
        5 => Zoom::Out,
        value => {
            return Err(ProtocolError::Unrecognised {
                what,
                value: u32::from(value),
            });
        }
    })
}

/// Where the reader is looking, in both directions: [`Command::View`](viewer_core::Command::View) and [`Answer::View`](viewer_core::Answer::View).
///
/// One pair of functions rather than two, because the value crossing back is the value crossing
/// out — which is what makes the restore exact on this side of the pipe as well as on the other.
pub(super) fn encode_viewing(writer: &mut Writer, view: viewer_core::Viewing) {
    writer.usize(view.page);
    encode_zoom(writer, view.zoom);
    writer.f32(view.scroll.0).f32(view.scroll.1);
}

pub(super) fn decode_viewing(
    reader: &mut Reader<'_>,
) -> Result<viewer_core::Viewing, ProtocolError> {
    Ok(viewer_core::Viewing {
        page: reader.usize("a page index")?,
        zoom: decode_zoom(reader)?,
        scroll: (reader.f32("a scroll")?, reader.f32("a scroll")?),
    })
}

/// Which of four answers a page's structure list is, as one byte (ADR 1393).
///
/// Its own field on the wire rather than read off the list's length, because an empty list is
/// three different sentences on the other side and the length says only that it is empty.
pub(super) fn tagging_code(tagging: viewer_core::Tagging) -> u8 {
    match tagging {
        viewer_core::Tagging::Untagged => 0,
        viewer_core::Tagging::Unread => 1,
        viewer_core::Tagging::Unreached { marked: false } => 2,
        viewer_core::Tagging::Unreached { marked: true } => 3,
        viewer_core::Tagging::Reached => 4,
    }
}

/// [`tagging_code`]'s inverse, refusing a byte it does not write.
pub(super) fn tagging_of(code: u8) -> Result<viewer_core::Tagging, ProtocolError> {
    match code {
        0 => Ok(viewer_core::Tagging::Untagged),
        1 => Ok(viewer_core::Tagging::Unread),
        2 => Ok(viewer_core::Tagging::Unreached { marked: false }),
        3 => Ok(viewer_core::Tagging::Unreached { marked: true }),
        4 => Ok(viewer_core::Tagging::Reached),
        value => Err(ProtocolError::Unrecognised {
            what: "which of four answers a page's structure is",
            value: u32::from(value),
        }),
    }
}
