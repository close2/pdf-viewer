//! Page one as it is drawn while it is read, uploaded for the window to put up (ADR 1855).

use std::sync::mpsc::Sender;

use render_raster::{PresentFrame, QuorraWindowRenderer, WindowTextures};

use super::{Finished, Pair, Partial};

/// A look at page one being read, uploaded and sent to the window, which is woken for it — and
/// `false` only where the window has gone. A look the device would not take is one the window
/// goes without. An upload and nothing else: no frame, no pages, nothing for the render thread's
/// stores to learn (ADR 1855).
pub(super) fn sent_read(
    renderer: &mut QuorraWindowRenderer,
    size: (u32, u32),
    look: &pdf_render::Raster,
    (done, waker): (&Sender<Finished>, Option<&crate::Wake>),
) -> bool {
    let Some(read) = upload_read(renderer, size, look) else {
        return true;
    };
    if done.send(Finished::Read(Box::new(read))).is_err() {
        return false;
    }
    if let Some(waker) = waker {
        waker.wake();
    }
    true
}

/// A look at page one being read, uploaded into a pair of its own for the window to put up
/// (ADR 1855) — `None` where the device would not take it, and the window goes without.
fn upload_read(
    renderer: &mut QuorraWindowRenderer,
    (width, height): (u32, u32),
    look: &pdf_render::Raster,
) -> Option<Partial> {
    let pair = Pair::new(renderer, width, height);
    renderer
        .render(
            PresentFrame {
                width,
                height,
                pages: &[],
                raster: Some(look),
                overlays: &[],
            },
            WindowTextures {
                page: &pair.page,
                chrome: &pair.chrome,
            },
        )
        .ok()?;
    Some(Partial {
        pages: Vec::new(),
        textures: pair,
        fraction: 0.0,
    })
}
