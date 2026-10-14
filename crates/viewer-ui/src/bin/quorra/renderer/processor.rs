//! The processor's half of a frame: the half that can be taken back (ADR 1821) and shown as it is
//! built (ADR 1831), and page one finished from its drawing ahead (ADR 1855).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::time::{Duration, Instant};

use pdf_render::{DisplayList, Interrupt, TargetSpec};
use render_raster::{PresentFrame, QuorraWindowRenderer, WindowTextures};

use super::{Finished, Pair, Partial};

/// The processor's half of a frame the device refused: `None` where the interrupt took it back,
/// and how long the processor drew, where it was started.
///
/// **The one half of a refused frame that can be taken back** (ADR 1821): the event thread raises
/// the interrupt where the view has left this magnification or the person has pressed the key, and
/// a raise that came while the device was being asked is honoured here, before anything starts.
///
/// **And the half that can be shown as it is built** (ADR 1831): the draw runs under
/// [`viewer_ui::software::compose_pages_watched`], and every look is uploaded into a pair of its
/// own by `shown` and sent to the window, which puts it up where it has no truer picture.
fn draw_on_the_processor(
    pages: &[crate::stale::Placed],
    interrupt: &Interrupt,
    on_the_processor: &AtomicBool,
    mut shown: impl FnMut(&viewer_ui::software::Assembly),
) -> (Option<Result<pdf_render::Raster, String>>, Option<Duration>) {
    if interrupt.raised() {
        return (None, None);
    }
    if pages.is_empty() {
        return (
            Some(Err("there is no page to draw on the processor".to_owned())),
            None,
        );
    }
    let borrowed: Vec<(&DisplayList, TargetSpec)> = pages
        .iter()
        .map(|placed| (placed.list.as_ref(), placed.target))
        .collect();
    on_the_processor.store(true, Ordering::Release);
    let began = Instant::now();
    let drawn = viewer_ui::software::compose_pages_watched(
        &borrowed,
        Some(interrupt),
        viewer_ui::software::Looks::SHIPPED,
        |assembly| {
            if !interrupt.raised() {
                shown(assembly);
            }
        },
    );
    let drew = Some(began.elapsed());
    match drawn {
        // Asked of the error rather than of the flag, for `crate::composer`'s reason: a frame
        // finished before the raise is a frame this window can present.
        Err(problem) if viewer_ui::software::was_interrupted(&problem) => (None, drew),
        drawn => (Some(drawn.map_err(|problem| problem.to_string())), drew),
    }
}

/// What the processor's half of a frame came to.
pub(super) struct ProcessorHalf {
    pub(super) drew: Option<Duration>,
    /// Whether the frame was finished from the page drawn while it was read.
    pub(super) finished_ahead: bool,
    pub(super) abandoned: bool,
    pub(super) refused: Option<String>,
}

/// The processor's half of a frame: drawn under its looks, each uploaded and sent to the window,
/// and the finished raster uploaded into the job's pair (ADR 1821, ADR 1831).
pub(super) fn processor_half(
    renderer: &mut QuorraWindowRenderer,
    (width, height): (u32, u32),
    (pages, ahead): (&[crate::stale::Placed], Option<render_cpu::Underway>),
    borrowed: &[&DisplayList],
    (interrupt, on_the_processor): (&Interrupt, &AtomicBool),
    (done, waker): (
        &Sender<Finished>,
        Option<&winit::event_loop::EventLoopProxy<()>>,
    ),
    into: WindowTextures<'_>,
) -> ProcessorHalf {
    // **One of the two jobs `CLAUDE.md` keeps the CPU backend for**: a page the device refuses is
    // a page this program can still show, more slowly, which is a cost a person can see past where
    // a page that never appears is not. The raster goes back through the device as one image,
    // because a window's pixels have one path and this is it — and it is the whole arrangement's
    // raster since ADR 0442, because a column the device refused is still a column.
    let looked_at = |assembly: &viewer_ui::software::Assembly| {
        let pair = Pair::new(renderer, width, height);
        let uploaded = renderer.render(
            PresentFrame {
                width,
                height,
                pages: &[],
                raster: Some(assembly.picture()),
                overlays: borrowed,
            },
            WindowTextures {
                page: &pair.page,
                chrome: &pair.chrome,
            },
        );
        // A look the device would not take is a look the window goes without: the frame itself
        // is still being drawn, and its own upload says what the device refused.
        if uploaded.is_ok() {
            let partial = Partial {
                pages: pages.to_vec(),
                textures: pair,
                fraction: assembly.fraction(),
            };
            if done.send(Finished::Partial(Box::new(partial))).is_ok()
                && let Some(waker) = waker
            {
                let _ = waker.send_event(());
            }
        }
    };
    // Page one drawn while it was read is finished here from the list it was read into, and a page
    // whose drawing ahead cannot be its picture is drawn whole as every other is (ADR 1855).
    let began = Instant::now();
    let finished = match (ahead, pages) {
        (Some(ahead), [placed]) => ahead.finish(&placed.list).ok(),
        _ => None,
    };
    let finished_ahead = finished.is_some();
    let (drawn, drew) = match finished {
        Some(raster) => {
            on_the_processor.store(true, Ordering::Release);
            (Some(Ok(raster)), Some(began.elapsed()))
        }
        None => draw_on_the_processor(pages, interrupt, on_the_processor, looked_at),
    };
    let (abandoned, refused) = match drawn {
        None => (true, None),
        Some(Ok(raster)) => {
            let second = renderer.render(
                PresentFrame {
                    width,
                    height,
                    pages: &[],
                    raster: Some(&raster),
                    overlays: borrowed,
                },
                into,
            );
            (
                false,
                second
                    .err()
                    .map(|second| format!("and the processor's page {second}")),
            )
        }
        Some(Err(second)) => (false, Some(format!("and {second}"))),
    };
    ProcessorHalf {
        drew,
        finished_ahead,
        abandoned,
        refused,
    }
}
