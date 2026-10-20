//! The render thread: spawned by the first job, and its one loop over what it is asked and what it
//! does while nothing is.

use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::Duration;

use pdf_render::Interrupt;
use raster_gpu::wgpu;
use render_raster::QuorraWindowRenderer;

use super::as_read::sent_read;
use super::device::{draw, prepare};
use super::lanes::RefusedLanes;
use super::retained::{Turn, retain_one};
use super::sharp::{SHARP_STALL_BUDGET, draw_sharp, sharp_pass_affordable, sharp_pass_cost};
use super::{Finished, Job, Known, Link, same_pages};

/// What the render thread does between jobs, as the window configured it.
#[derive(Debug)]
pub(super) struct Idle {
    /// How many whole pages it retains a low-resolution picture of.
    pub(super) proxy_pages: usize,
    /// `--supersample`'s factor.
    pub(super) supersample: u32,
    /// `--coverage`'s choice.
    pub(super) coverage: crate::lane::CoverageChoice,
    /// Where it leaves the flag that takes a whole page drawn on the processor back
    /// ([`Window::idle_interrupt`]).
    ///
    /// [`Window::idle_interrupt`]: super::Window::idle_interrupt
    pub(super) interrupt: Arc<std::sync::Mutex<Option<Interrupt>>>,
}

/// Starts the render thread around a renderer, and hands back the channels to it.
pub(super) fn spawn(
    renderer: QuorraWindowRenderer,
    idle: Idle,
    waker: Option<crate::Wake>,
) -> Link {
    let (jobs, incoming) = channel::<Job>();
    let (outgoing, done) = channel::<Finished>();
    // A spawn this machine refuses is a machine with no thread to spare, and the window then
    // presents whatever it has and asks for frames nobody answers — which is what
    // `Link::jobs.send` failing already means, so there is one path rather than two.
    let thread = std::thread::Builder::new()
        .name("page renderer".to_owned())
        .spawn(move || {
            draw_until_told_to_stop(renderer, &idle, waker.as_ref(), &incoming, &outgoing);
        })
        .ok();
    Link { jobs, done, thread }
}

/// The render thread's whole life: draw what is asked for, send it back, and draw a whole page at
/// a fraction of the size while nothing else is asked of it.
///
/// Ends when the event thread drops its sender, which is when the window is closing.
///
/// **The order is the whole of `CLAUDE.md`'s startup rule here.** A job always wins: the channel
/// is asked first, a proxy is drawn only when it is empty, and one page is drawn per idle turn so
/// that a view change arriving mid-way waits for one low-resolution frame rather than for a set of
/// them. Nothing about this runs before the first job, because the thread does not exist until
/// there is one.
#[expect(
    clippy::too_many_lines,
    reason = "the render thread's one loop, its kinds of work in the order they win: a look, a \
              frame, the sharp pass, a retained page. Each kind's work is a function already; \
              what is left here is the order, which reads best in one place"
)]
fn draw_until_told_to_stop(
    mut renderer: QuorraWindowRenderer,
    idle: &Idle,
    waker: Option<&crate::Wake>,
    jobs: &Receiver<Job>,
    done: &Sender<Finished>,
) {
    use std::sync::mpsc::TryRecvError;

    let (supersample, coverage) = (idle.supersample, idle.coverage);
    let mut proxies: crate::stale::Proxies<Arc<wgpu::Texture>> =
        crate::stale::Proxies::new(idle.proxy_pages);
    // What the last frame drew, which is the only set of pages this thread has display lists for.
    let mut showing: Vec<crate::stale::Placed> = Vec::new();
    // The window size the last frame was drawn for — what the sharp pass doubles.
    let mut shown_size = (0_u32, 0_u32);
    // Whether the view on hand has had its sharp pass. True after one attempt either way:
    // a refusal (a device limit, most likely) must not spin, exactly as `draw_whole_page`'s
    // silence must not — the next view change resets it.
    let mut sharpened = false;
    // What the last frame this thread had to *build* cost — [`sharp_pass_affordable`]'s
    // prediction. A replay says what a replay costs and nothing about what a 4× render
    // will, which is the same reading the event thread makes for rule 5's prediction
    // (`quorra::surface`'s `adopt`, ADR 0384).
    let mut last_built: Option<Duration> = None;
    // The chrome lane needs a target of its own even where there is no chrome, so one scratch
    // texture is kept for whatever size the proxies are and remade when a page of another shape
    // arrives. One allocation per page shape rather than one per proxy.
    let mut scratch: Option<(u32, u32, wgpu::Texture)> = None;
    // The pages whose whole-page picture the device would not draw, which are not asked for again
    // (ADR 1821). `draw_whole_page`'s silence stays: a proxy is a picture nobody asked for. What
    // changes is that its refusal stops being paid once per frame — on a page the device refuses
    // that was the scene built and priced again after every frame, in front of the next one.
    let mut no_proxy: Vec<crate::stale::Picture> = Vec::new();
    // The pages whose last frame the processor drew — refused by the device, or predicted cheaper
    // there (ADR 1831). Their whole-page picture is the processor's too: the device would refuse
    // it or take as long over it as over the frame, and a device draw cannot be taken back when
    // the next job arrives, where the processor's can.
    let mut on_the_processor: Vec<crate::stale::Picture> = Vec::new();
    // The coverage lanes the adapter refused, learned from the frames and from the retained
    // layer's own ask, so that a retained picture of a refused lane is not asked of the device
    // either (ADR 1984), nor a frame of one asked before the window was told (ADR 2000).
    let mut refused_lanes = RefusedLanes::default();
    // What building the device's scene of the pages on hand is predicted to cost, where the last
    // frame was the processor's and the window asked for the build ([`Job::build_ahead`]).
    let mut build_ahead: Option<Duration> = None;
    let mut waiting: Option<Job> = None;
    loop {
        let job = match waiting.take() {
            Some(job) => Some(job),
            None => match jobs.try_recv() {
                Ok(job) => Some(job),
                Err(TryRecvError::Empty) => None,
                Err(TryRecvError::Disconnected) => return,
            },
        };
        if let Some(mut job) = job {
            if let Some(look) = job.look.take() {
                if !sent_read(&mut renderer, (job.width, job.height), &look, (done, waker)) {
                    return;
                }
                continue;
            }
            // A job of the same pages at the same size — a chrome-only change, a selection,
            // the find bar — leaves the sharp picture true, so it is not paid for again.
            if !(same_pages(&showing, &job.pages) && shown_size == (job.width, job.height)) {
                sharpened = false;
            }
            showing.clone_from(&job.pages);
            shown_size = (job.width, job.height);
            // **A lane this thread heard refused answers a job the window asked before it was
            // told** (ADR 2000): the retained layer's ask can hear the refusal while the job is on
            // its way, and the device would be asked the lane again for nothing. Only a job the
            // device would have been asked for: one predicted the processor's, or finished from the
            // page drawn while it was read, is drawn as it was asked.
            if job.known_refusal.is_none()
                && job.cheaper.is_none()
                && job.ahead.is_none()
                && let Some((said, of)) = refused_lanes.said(&job.pages, job.coverage)
            {
                job.known_refusal = Some(Known {
                    said: said.to_owned(),
                    of,
                });
            }
            let asked_ahead = job.build_ahead.take();
            let finished = draw(&mut renderer, job, done, waker);
            if let Some(said) = finished.fell_back.as_deref() {
                refused_lanes.learn(finished.known.or(finished.refused_now), said);
            }
            // Only after a frame the processor drew and the device was not asked: a refusal is the
            // device's answer for these pages, and a frame taken back is of a view nobody wants.
            build_ahead = asked_ahead.filter(|_| {
                finished.on_the_processor()
                    && finished.fell_back.is_none()
                    && !finished.abandoned
                    && finished.refused.is_none()
            });
            // Where the frame was sent, not whether it began: a frame taken back before the
            // processor started was still the processor's, and its page's picture is too.
            let drawn_there = finished.on_the_processor();
            for placed in &finished.pages {
                on_the_processor.retain(|picture| *picture != placed.of);
                if drawn_there {
                    on_the_processor.push(placed.of);
                }
            }
            // **A view the device refused is not sharpened by it** (ADR 1821): the pass is the
            // same view at four times the pixels, which raster prices higher rather than lower,
            // so asking would build and price a scene to hear the answer this frame already gave.
            // Nor one the processor drew because it was the faster lane (ADR 1831): the device
            // was predicted to take seconds over the view at one times the pixels.
            let processor_frame = finished.on_the_processor();
            if processor_frame {
                sharpened = true;
            }
            // A processor's frame is an upload to the device, whose cost says nothing about what
            // the device would take to build the page (trap 143), so it predicts no pass.
            if !processor_frame
                && !matches!(
                    finished.cost.encode_source,
                    Some(raster_gpu::EncodeSource::Replayed)
                )
            {
                last_built = Some(finished.cost.total);
            }
            // A send that fails is an event thread that has gone, and there is nobody left to draw
            // for. The loop would end at the next `recv` anyway; leaving now saves a frame nobody
            // would see.
            if done.send(Finished::Frame(Box::new(finished))).is_err() {
                return;
            }
            // Woken, as for the sharp pass, a look and a scene, so that the window takes the frame
            // when it lands rather than at whatever wakes the loop next (ADR 1926).
            if let Some(waker) = waker {
                waker.wake();
            }
            continue;
        }
        // The sharp pass goes before the proxies: it is what the person is looking at now,
        // where a proxy is for a view change that has not happened yet. Declined without a
        // word where the prediction says the machine cannot afford it — `draw_sharp`'s own
        // silence rule: this is a picture nobody asked for, and what a decline costs is a
        // seam drawn at full width rather than half (ADR 0699), not a wrong pixel. The flag
        // is set either way, so a declined view is not re-asked until the view changes.
        if supersample >= 2 && !sharpened && !showing.is_empty() {
            // **The view has to have been still for as long as the pass will take before the
            // pass may begin** (ADR 1289). It cannot be taken back once begun, so a view change
            // arriving during it waits for all of it, and on the owner's 890M the worst page's
            // pass costs what a zoom step does (57–63 ms against 55–62): begun the moment the
            // thread is idle, it puts a whole step in front of every step of a gesture, and the
            // window shows a stand-in from the first notch to the last. Waiting out the pass's
            // own predicted length first is the rent-or-buy bound: a view change is delayed by
            // the pass only after the person has already been still for at least as long as the
            // delay, and never by a pass begun in a pause shorter than itself.
            if let Some(stillness) =
                sharp_pass_cost(last_built).filter(|pass| *pass <= SHARP_STALL_BUDGET)
            {
                match jobs.recv_timeout(stillness) {
                    Ok(job) => {
                        waiting = Some(job);
                        continue;
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
                    Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => return,
                }
            }
            sharpened = true;
            if sharp_pass_affordable(last_built)
                && let Some(sharp) = draw_sharp(&mut renderer, &showing, shown_size, coverage)
            {
                if done.send(Finished::Sharp(Box::new(sharp))).is_err() {
                    return;
                }
                // The loop this picture is for may be at rest — a settled view is exactly
                // when it is — so it is woken. A send to a loop that has exited is nothing.
                if let Some(waker) = waker {
                    waker.wake();
                }
                continue;
            }
        }
        // **The device's scene of the pages on hand, built while nobody waits** (ADR 1898), before
        // the retained pages: it is what the person's next view of these pages is drawn from,
        // where a retained page stands in for a view nobody has asked for. It cannot be taken back
        // once begun, so it waits out its own predicted length first — the sharp pass's
        // rent-or-buy bound (ADR 1289) — and a job arriving in that time is drawn instead.
        if let Some(stillness) = build_ahead.take() {
            match jobs.recv_timeout(stillness) {
                Ok(job) => {
                    waiting = Some(job);
                    continue;
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => return,
            }
            if let Some(prepared) = prepare(&mut renderer, &showing, shown_size) {
                if done.send(Finished::Scene(Box::new(prepared))).is_err() {
                    return;
                }
                // Woken so that the window's record of what the device holds is true before the
                // next view is priced, rather than at whatever input comes next.
                if let Some(waker) = waker {
                    waker.wake();
                }
            }
            continue;
        }
        match retain_one(
            &mut renderer,
            (&showing, &on_the_processor),
            (
                &mut proxies,
                &mut no_proxy,
                &mut scratch,
                &mut refused_lanes,
            ),
            idle,
            jobs,
            done,
        ) {
            Turn::Drew => continue,
            Turn::Job(job) => {
                waiting = Some(*job);
                continue;
            }
            Turn::Stop => return,
            Turn::Nothing => {}
        }
        match jobs.recv() {
            Ok(job) => waiting = Some(job),
            Err(_) => return,
        }
    }
}
