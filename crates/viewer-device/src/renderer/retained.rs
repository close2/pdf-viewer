//! The stand-in's layer: a whole page drawn small while the render thread is idle, on the lane its
//! frame is drawn on (ADRs 0443, 1831).

use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender};

use pdf_render::Interrupt;
use raster_gpu::wgpu;
use render_raster::{PresentFrame, QuorraRasterError, QuorraWindowRenderer, WindowTextures};

use super::lanes::{RefusedLanes, remembered};
use super::thread::Idle;
use super::{Finished, Job};

/// The render thread's own record of the whole pages it holds pictures of.
type Store = crate::stale::Proxies<Arc<wgpu::Texture>>;

/// The chrome lane's target for a whole page drawn on the device, kept per page shape.
type Scratch = Option<(u32, u32, wgpu::Texture)>;

/// The coverage lane a retained picture is drawn under on the device.
///
/// A whole page at a few hundred pixels is far below the magnification at which raster's GPU
/// coverage lane is the cheaper one, whatever the window is showing (`quorra::surface`), so the lane
/// is stated rather than inherited from whatever the last frame asked for. `draw` sets it per job,
/// so the next window frame is unaffected.
const LANE: raster_gpu::Coverage = raster_gpu::Coverage::Cpu;

/// What one idle turn spent on the retained layer came to.
pub(super) enum Turn {
    /// A page was drawn, or given up on, or taken back: the loop looks for work again.
    Drew,
    /// A job arrived first.
    Job(Box<Job>),
    /// The event thread has gone.
    Stop,
    /// No page wants a picture.
    Nothing,
}

/// One idle turn's page for the retained layer, where one wants a picture (ADR 0443, ADR 1831).
pub(super) fn retain_one(
    renderer: &mut QuorraWindowRenderer,
    (showing, on_the_processor): (&[crate::stale::Placed], &[crate::stale::Picture]),
    (proxies, no_proxy, scratch, lanes): (
        &mut Store,
        &mut Vec<crate::stale::Picture>,
        &mut Scratch,
        &mut RefusedLanes,
    ),
    idle: &Idle,
    jobs: &Receiver<Job>,
    done: &Sender<Finished>,
) -> Turn {
    let drawable: Vec<crate::stale::Placed> = showing
        .iter()
        .filter(|placed| !no_proxy.contains(&placed.of))
        .cloned()
        .collect();
    let Some(page) = proxies.wanted(&drawable) else {
        return Turn::Nothing;
    };
    let frame_on_the_processor = on_the_processor.contains(&page.of);
    let retained = match retain(
        renderer,
        &page,
        frame_on_the_processor,
        (scratch, lanes),
        idle,
        (jobs, done),
    ) {
        Drawn::Picture(retained) => retained,
        Drawn::Behind(job) => return Turn::Job(job),
        Drawn::Taken => return Turn::Drew,
        Drawn::Refused => {
            // The next page that wants one is asked for at the next idle turn, and this one never
            // again for this ink; the store stays a document's pages long, as `Proxies` is.
            no_proxy.retain(|picture| picture.same_ink(page.of));
            no_proxy.push(page.of);
            return Turn::Drew;
        }
    };
    // Kept here as well as sent, because this thread is the one that decides what to draw next
    // and a store that forgot would draw the same page for ever.
    proxies.keep(crate::stale::Retained {
        of: retained.of,
        target: retained.target,
        pixels: Arc::clone(&retained.pixels),
    });
    if done.send(Finished::Page(retained)).is_err() {
        return Turn::Stop;
    }
    Turn::Drew
}

/// Draws one whole page into a raster of its own, for the layer under the base.
///
/// `None` where the page states no usable extent, and the device's refusal where it refused —
/// **which is not said**, because this is a picture nobody asked for: a refusal costs a stand-in
/// that will not be as good and nothing else, and reporting it would be a note about work the
/// person did not request. The caller sorts it by the frames' rule and draws on the processor. It
/// cannot spin, and it is not paid twice: the caller keeps the page out of every later idle turn
/// for as long as its ink stands (ADR 1821), and a refused lane out of every later picture (ADR
/// 1984). The lane is [`LANE`], which [`retain`] has set.
fn draw_whole_page(
    renderer: &mut QuorraWindowRenderer,
    page: &crate::stale::Placed,
    scratch: &mut Option<(u32, u32, wgpu::Texture)>,
) -> Option<Result<crate::stale::Retained<Arc<wgpu::Texture>>, QuorraRasterError>> {
    let target = crate::stale::proxy_target(&page.list)?;
    let (width, height) = (target.width, target.height);
    let chrome = match scratch.take() {
        Some((held_width, held_height, texture))
            if (held_width, held_height) == (width, height) =>
        {
            texture
        }
        // The chrome lane needs a target even where there is no chrome to draw, and
        // `Target::Texture` wants one sized exactly to the frame — so a page of another shape
        // costs one texture and every page of one document costs one between them.
        _ => renderer.layer_texture("a retained page's unused chrome", width, height),
    };
    let texture = renderer.layer_texture("a retained page", width, height);
    let placed = [(&page.list, target)];
    let drawn = renderer.render(
        PresentFrame {
            width,
            height,
            pages: &placed,
            raster: None,
            overlays: &[],
        },
        WindowTextures {
            page: &texture,
            chrome: &chrome,
        },
    );
    *scratch = Some((width, height, chrome));
    Some(drawn.map(|()| crate::stale::Retained {
        of: page.of,
        target,
        pixels: Arc::new(texture),
    }))
}

/// What a whole page drawn for the retained layer came to.
enum Drawn {
    /// The picture, uploaded.
    Picture(crate::stale::Retained<Arc<wgpu::Texture>>),
    /// A job had arrived before the processor began, and goes first.
    Behind(Box<Job>),
    /// A job arrived and took the thread back; the page is asked for again at the next idle turn.
    Taken,
    /// Neither lane would draw it.
    Refused,
}

/// One whole page for the retained layer: on the device where its frame is drawn there, and on the
/// processor where the frame is, where the device refused this picture (ADR 1831), or where the
/// adapter refused [`LANE`]'s pipelines before — and then the device is not asked (ADR 1984).
///
/// **A refused lane is routed as a frame of it is** (ADR 1968): raster answers it in the same words
/// for the device's life, but only after the picture's scene is encoded, so asking again is that
/// encode paid once per page for nothing. The lane is known from a frame that carried or heard its
/// refusal, or from this layer's own first ask, and the device is asked once per window.
///
/// **A lane this layer heard first is told to the window** (`Finished::Lane`, ADR 2000), which
/// keeps it as it keeps one a frame heard, so the next frame of the lane does not ask either.
fn retain(
    renderer: &mut QuorraWindowRenderer,
    page: &crate::stale::Placed,
    frame_on_the_processor: bool,
    (scratch, lanes): (&mut Scratch, &mut RefusedLanes),
    idle: &Idle,
    (jobs, done): (&Receiver<Job>, &Sender<Finished>),
) -> Drawn {
    if !frame_on_the_processor {
        // Set where the device would have been asked, refused or not, so that the processor's
        // picture goes back under the lane it went back under after a refusal: one image and no
        // chrome, which the compute assist does not take (ADR 1984).
        renderer.set_coverage(LANE);
        if !lanes.refused(LANE) {
            match draw_whole_page(renderer, page, scratch) {
                Some(Ok(retained)) => return Drawn::Picture(retained),
                Some(Err(problem)) => {
                    let said = problem.to_string();
                    if lanes.learn(remembered(&problem, LANE), &said) {
                        // A send that fails is an event thread that has gone; the picture's own
                        // send, or the loop's next wait, ends this thread on the same answer.
                        drop(done.send(Finished::Lane { lane: LANE, said }));
                    }
                }
                None => {}
            }
        }
    }
    let interrupt = Interrupt::new();
    if let Ok(mut held) = idle.interrupt.lock() {
        *held = Some(interrupt.clone());
    }
    // A job that arrived before the flag was left is a job this draw would stand in front of:
    // asked first, and the page is drawn at the next idle turn.
    if let Ok(job) = jobs.try_recv() {
        return Drawn::Behind(Box::new(job));
    }
    draw_whole_page_on_the_processor(renderer, page, &interrupt)
}

/// [`draw_whole_page`] on the processor, for a page whose frame the processor draws (ADR 1831).
///
/// Silent for that function's reason — a picture nobody asked for — and interruptible because the
/// processor's draw is: [`Window::ask`] raises `interrupt` the moment a job is sent, so a view
/// change waits for the command in progress rather than for a whole page drawn small.
///
/// [`Window::ask`]: super::Window::ask
fn draw_whole_page_on_the_processor(
    renderer: &mut QuorraWindowRenderer,
    page: &crate::stale::Placed,
    interrupt: &Interrupt,
) -> Drawn {
    let Some(target) = crate::stale::proxy_target(&page.list) else {
        return Drawn::Refused;
    };
    let drawn = crate::compose::compose_pages_interruptibly(
        &[(page.list.as_ref(), target)],
        Some(interrupt),
    );
    let raster = match drawn {
        Ok(raster) => raster,
        Err(problem) if crate::compose::was_interrupted(&problem) => return Drawn::Taken,
        Err(_) => return Drawn::Refused,
    };
    let texture = renderer.layer_texture("a retained page", target.width, target.height);
    let uploaded = renderer.render_page_only(
        PresentFrame {
            width: target.width,
            height: target.height,
            pages: &[],
            raster: Some(&raster),
            overlays: &[],
        },
        &texture,
    );
    if uploaded.is_err() {
        return Drawn::Refused;
    }
    Drawn::Picture(crate::stale::Retained {
        of: page.of,
        target,
        pixels: Arc::new(texture),
    })
}
