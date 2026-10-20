//! What a zoom gesture's frames are, measured on the real render thread and the real adapter.
//!
//! **A measurement rather than a gate**, and ignored for the reason every measurement on a real
//! device is: it needs one, and it prints durations rather than asserting them. It drives the
//! render thread this window uses — [`spawn`], the sharp pass and the retained pages included —
//! from a clock that ticks at the owner's 120 Hz, asks [`crate::stale::Stale::plan`] the question
//! `quorra::surface` asks at every tick, and counts what each tick would have put up: a rendering
//! of the view asked for, a stand-in, or the picture already there held for one more refresh.
//!
//! ```sh
//! cargo test --release -p viewer-ui --bin quorra -- --ignored --nocapture a_zoom_gesture
//! GESTURE_PDF=tmp/Entwurf.pdf GESTURE_EVERY=12 cargo test … a_zoom_gesture
//! ```
//!
//! `GESTURE_EVERY` is how many refreshes lie between two wheel notches (4 by default, which is a
//! hand turning a wheel briskly), `GESTURE_SUPERSAMPLE` is `--supersample`'s factor.

use super::lanes::Prediction;
use super::thread::{Idle, spawn};
use super::*;
use crate::stale::{Placed, Plan, Stale, Standing};
use pdf_render::{TargetSpec, Transform};

/// What one tick put up, in the words the frame line uses.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Put {
    Fresh,
    StoodIn,
    Held,
    Refused,
}

fn variable(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|value| !value.is_empty())
}

/// The arrangement `quorra::surface`'s `arrangement` builds, for a window with no panel.
fn arrangement(
    viewer: &viewer_core::Viewer,
    requests: &mut Vec<viewer_core::RenderRequest>,
    (width, height): (u32, u32),
) -> Vec<Placed> {
    let mut placed = Vec::new();
    for request in requests.iter() {
        let viewer_core::Answer::Geometry(geometry) =
            viewer.query(viewer_core::Query::PageGeometry(request.page))
        else {
            continue;
        };
        placed.push(Placed {
            of: crate::stale::Picture::new(request.document, request.page, request.ink),
            list: Arc::clone(&request.list),
            target: TargetSpec {
                width,
                height,
                transform: request
                    .target
                    .transform
                    .then(Transform::translate(geometry.origin.0, geometry.origin.1)),
            },
            depicts: crate::stale::Depicts::Page,
        });
    }
    requests.retain(|request| placed.iter().any(|one| one.of.page() == request.page));
    placed
}

fn fold(
    events: impl Iterator<Item = viewer_core::Event>,
    into: &mut Vec<viewer_core::RenderRequest>,
) {
    for event in events {
        if let viewer_core::Event::NeedsRender(request) = event {
            into.retain(|held| held.page != request.page);
            into.push(request);
        }
    }
}

/// The committed document every headless test opens, laid out in a window of `size`.
fn opened(size: (u32, u32)) -> (viewer_core::Viewer, Vec<viewer_core::RenderRequest>) {
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc/PDF20_AN001-BPC.pdf");
    let bytes = std::fs::read(&path)
        .unwrap_or_else(|error| panic!("{} is committed: {error}", path.display()));
    let mut viewer = viewer_core::Viewer::new(size.0, size.1, 1.0);
    let mut requests = Vec::new();
    fold(
        viewer.handle(viewer_core::Command::Open {
            id: viewer_core::DocumentId(1),
            bytes: bytes.into(),
            password: None,
            fragment: None,
        }),
        &mut requests,
    );
    (viewer, requests)
}

/// What the render thread does between jobs, for a test: no flag left anywhere it is read.
fn idle(proxy_pages: usize, supersample: u32) -> Idle {
    Idle {
        proxy_pages,
        supersample,
        coverage: crate::lane::CoverageChoice::Auto,
        interrupt: Arc::default(),
    }
}

fn job(pages: &[Placed], size: (u32, u32)) -> Job {
    Job {
        width: size.0,
        height: size.1,
        pages: pages.to_vec(),
        overlays: Vec::new(),
        coverage: raster_gpu::Coverage::Compute,
        reuse: None,
        known_refusal: None,
        cheaper: None,
        interrupt: Interrupt::new(),
        on_the_processor: Arc::new(AtomicBool::new(false)),
        ahead: None,
        look: None,
        build_ahead: None,
    }
}

/// **A zoom step is drawn before the sharp pass of the view it replaced, and the sharp pass
/// still comes once the view is still** (ADR 1289).
///
/// The owner's report was a zoom gesture that showed a stand-in throughout on a device quick
/// enough to draw every step: each step's render waited behind the previous view's sharp
/// pass, which the render thread began the moment it was idle and which nothing can take
/// back. The shape asserted is the order of what comes back, never a duration: a view change
/// sent the instant the first frame lands must be answered by its own frame, and the
/// sharpened picture that follows must be of the *new* view.
#[test]
fn a_zoom_step_is_drawn_before_the_sharp_pass_of_the_view_it_replaced() {
    let Ok(renderer) = QuorraWindowRenderer::new_headless() else {
        println!("skipped: no adapter on this machine");
        return;
    };
    let size = (800_u32, 1000_u32);
    let (mut viewer, mut requests) = opened(size);
    let first = arrangement(&viewer, &mut requests, size);
    fold(
        viewer.handle(viewer_core::Command::Zoom {
            zoom: viewer_core::Zoom::Out,
            at: None,
        }),
        &mut requests,
    );
    let second = arrangement(&viewer, &mut requests, size);
    assert!(
        !first.is_empty() && !second.is_empty(),
        "the document shows a page"
    );

    let link = spawn(renderer, idle(0, 2), None);
    let patience = Duration::from_mins(2);
    let next = || {
        link.done
            .recv_timeout(patience)
            .expect("the render thread answers")
    };
    link.jobs
        .send(job(&first, size))
        .expect("the thread is running");
    let Finished::Frame(landed) = next() else {
        panic!("the first answer is the frame asked for");
    };
    assert!(landed.refused.is_none(), "{:?}", landed.refused);
    // The step, sent the moment the first frame is in hand — the tightest a gesture can be.
    link.jobs
        .send(job(&second, size))
        .expect("the thread is running");
    match next() {
        Finished::Frame(done) => assert!(
            same_pages(&done.pages, &second),
            "the frame that answered is of the view asked for"
        ),
        Finished::Sharp(_) => panic!(
            "the previous view's sharp pass was drawn in front of the zoom step that \
             replaced it"
        ),
        Finished::Page(_) => panic!("no retained page was asked for"),
        Finished::Partial(_) | Finished::Read(_) => {
            panic!("a device frame is not drawn on the processor")
        }
        Finished::Scene(_) => panic!("no scene was asked to be built ahead"),
        Finished::Lane { said, .. } => panic!("no lane was refused here: {said}"),
    }
    let Finished::Sharp(sharp) = next() else {
        panic!("a still view is sharpened");
    };
    assert!(
        same_pages(&sharp.pages, &second),
        "the sharp pass is of the view that stayed"
    );
}

/// **A frame the device refused before is drawn on the processor without asking it, and is not
/// sharpened by it** (ADR 1821).
///
/// The job carries the refusal the window remembered; what comes back says the device was not
/// asked, the processor drew, the pixels landed — and nothing follows, because the 2× pass is
/// the same view at four times the pixels and would only hear the same answer.
#[test]
fn a_frame_the_device_refused_before_is_drawn_on_the_processor_without_asking_it() {
    let Ok(renderer) = QuorraWindowRenderer::new_headless() else {
        println!("skipped: no adapter on this machine");
        return;
    };
    let size = (400_u32, 500_u32);
    let (viewer, mut requests) = opened(size);
    let pages = arrangement(&viewer, &mut requests, size);
    assert!(!pages.is_empty(), "the document shows a page");
    let link = spawn(renderer, idle(0, 2), None);
    let mut asked = job(&pages, size);
    asked.known_refusal = Some(Known {
        said: "frame refused: remembered".to_owned(),
        of: Kept::Frame,
    });
    link.jobs.send(asked).expect("the thread is running");
    let Ok(Finished::Frame(done)) = link.done.recv_timeout(Duration::from_mins(2)) else {
        panic!("the first answer is the frame asked for");
    };
    assert_eq!(done.fell_back.as_deref(), Some("frame refused: remembered"));
    assert_eq!(
        done.known,
        Some(Kept::Frame),
        "and says why it was not asked"
    );
    assert!(done.device_tried.is_none(), "the device was not asked");
    assert!(done.processor_drew.is_some(), "the processor drew it");
    assert!(
        !done.abandoned && done.refused.is_none(),
        "{:?}",
        done.refused
    );
    assert!(done.refused_now.is_none(), "nothing new was learned");
    match link.done.recv_timeout(Duration::from_secs(3)) {
        Err(_) => {}
        Ok(Finished::Sharp(_)) => panic!("a view the device refused was sharpened by it"),
        Ok(_) => panic!("nothing else was asked for"),
    }
}

/// **A frame the processor drew is followed by the device's scene of it, built while nobody
/// waits, and the device's next frame of the page keeps it** (ADR 1898): the window asked for
/// the build with the frame, the thread sent the frame and then the scene, and the frame after
/// it, on the device, drew the scene it held rather than building one.
#[test]
fn a_scene_built_ahead_after_a_processor_frame_is_kept_by_the_device_s_next_frame() {
    let Ok(renderer) = QuorraWindowRenderer::new_headless() else {
        println!("skipped: no adapter on this machine");
        return;
    };
    let size = (400_u32, 500_u32);
    let (viewer, mut requests) = opened(size);
    let pages = arrangement(&viewer, &mut requests, size);
    assert!(!pages.is_empty(), "the document shows a page");
    let link = spawn(renderer, idle(0, 2), None);
    let commands = pages[0].list.commands().len();
    let mut asked = job(&pages, size);
    asked.cheaper = Some(Prediction {
        commands,
        device: Duration::from_secs(2),
        device_measured: false,
        scene_kept: false,
        processor: Duration::from_secs(1),
        processor_measured: false,
    });
    asked.build_ahead = Some(Duration::ZERO);
    link.jobs.send(asked).expect("the thread is running");
    let Ok(Finished::Frame(done)) = link.done.recv_timeout(Duration::from_mins(2)) else {
        panic!("the first answer is the frame asked for");
    };
    assert!(done.processor_drew.is_some(), "the processor drew it");
    let Ok(Finished::Scene(prepared)) = link.done.recv_timeout(Duration::from_mins(2)) else {
        panic!("the device's scene of the page follows the processor's frame");
    };
    assert!(same_pages(&prepared.pages, &pages), "of the pages on hand");
    assert!(!prepared.cost.scene_kept, "and it was built");
    link.jobs
        .send(job(&pages, size))
        .expect("the thread is running");
    let Ok(Finished::Frame(done)) = link.done.recv_timeout(Duration::from_mins(2)) else {
        panic!("the next answer is the frame asked for");
    };
    assert!(done.device_drew.is_some() && done.fell_back.is_none());
    assert!(
        done.cost.scene_kept,
        "the device drew the scene built ahead"
    );
}

/// **An interrupt raised before the processor starts abandons the frame**, and nothing is drawn
/// for it (ADR 1821) — the shape a notch of zoom takes while the device is still being asked.
#[test]
fn an_interrupt_raised_before_the_processor_starts_abandons_the_frame() {
    let Ok(renderer) = QuorraWindowRenderer::new_headless() else {
        println!("skipped: no adapter on this machine");
        return;
    };
    let size = (400_u32, 500_u32);
    let (viewer, mut requests) = opened(size);
    let pages = arrangement(&viewer, &mut requests, size);
    let link = spawn(renderer, idle(0, 1), None);
    let mut asked = job(&pages, size);
    // The lane's refusal, which the job's own coverage was refused under (ADR 1968): the frame
    // is answered as one of the frame's own is.
    asked.known_refusal = Some(Known {
        said: "frame refused: remembered".to_owned(),
        of: Kept::Lane(raster_gpu::Coverage::Compute),
    });
    asked.interrupt.raise();
    link.jobs.send(asked).expect("the thread is running");
    let Ok(Finished::Frame(done)) = link.done.recv_timeout(Duration::from_mins(2)) else {
        panic!("the first answer is the frame asked for");
    };
    assert!(done.abandoned, "the frame was taken back");
    assert!(
        done.processor_drew.is_none(),
        "the processor was not started"
    );
    assert!(done.device_tried.is_none(), "nor was the device asked");
    assert_eq!(done.known, Some(Kept::Lane(raster_gpu::Coverage::Compute)));
}

/// **A frame that waited behind other work is charged only its own drawing** (ADR 1289).
///
/// Rule 5 predicts the next render from this one, and time spent queued behind a pass
/// nobody asked for is not something the next render will cost. Two jobs sent together are
/// drawn one after the other, so the second waits for the whole of the first — and the span
/// [`Landed::drew`] is measured over has to begin after the first has finished.
#[test]
fn a_frame_that_waited_behind_another_is_charged_only_its_own_drawing() {
    let Ok(renderer) = QuorraWindowRenderer::new_headless() else {
        println!("skipped: no adapter on this machine");
        return;
    };
    let size = (800_u32, 1000_u32);
    let (mut viewer, mut requests) = opened(size);
    let first = arrangement(&viewer, &mut requests, size);
    fold(
        viewer.handle(viewer_core::Command::Zoom {
            zoom: viewer_core::Zoom::Out,
            at: None,
        }),
        &mut requests,
    );
    let second = arrangement(&viewer, &mut requests, size);
    let link = spawn(renderer, idle(0, 1), None);
    let asked = Instant::now();
    link.jobs
        .send(job(&first, size))
        .expect("the thread is running");
    link.jobs
        .send(job(&second, size))
        .expect("the thread is running");
    let mut frames = Vec::new();
    while frames.len() < 2 {
        if let Finished::Frame(done) = link
            .done
            .recv_timeout(Duration::from_mins(2))
            .expect("the render thread answers")
        {
            frames.push(done);
        }
    }
    assert!(
        frames[1].started >= frames[0].finished,
        "the second began after the first"
    );
    assert!(
        frames[1]
            .finished
            .saturating_duration_since(frames[1].started)
            < frames[1].finished.saturating_duration_since(asked),
        "what the second is charged excludes the first it waited behind"
    );
}

#[test]
#[ignore = "a measurement on the real adapter; prints what a gesture's refreshes put up"]
#[expect(
    clippy::too_many_lines,
    reason = "one simulated event loop, read top to bottom as the tick it models"
)]
fn a_zoom_gesture_out_counts_what_each_refresh_put_up() {
    let Ok(renderer) = QuorraWindowRenderer::new_headless() else {
        eprintln!("no adapter: nothing to measure");
        return;
    };
    let adapter = renderer.adapter_description().to_owned();
    let path = variable("GESTURE_PDF").map_or_else(
        || std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc/PDF20_AN001-BPC.pdf"),
        |named| {
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../..")
                .join(named)
        },
    );
    let every: u32 = variable("GESTURE_EVERY")
        .and_then(|v| v.parse().ok())
        .unwrap_or(4);
    let notches: u32 = variable("GESTURE_NOTCHES")
        .and_then(|v| v.parse().ok())
        .unwrap_or(6);
    let supersample: u32 = variable("GESTURE_SUPERSAMPLE")
        .and_then(|v| v.parse().ok())
        .unwrap_or(2);
    let size = (1200_u32, 1500_u32);
    let period = Duration::from_micros(8_333);

    let bytes = std::fs::read(&path).expect("the document named is readable");
    let mut viewer = viewer_core::Viewer::new(size.0, size.1, 1.0);
    let mut requests = Vec::new();
    fold(
        viewer.handle(viewer_core::Command::Open {
            id: viewer_core::DocumentId(1),
            bytes: bytes.into(),
            password: None,
            fragment: None,
        }),
        &mut requests,
    );

    let link = spawn(renderer, idle(2, supersample), None);
    let mut stale = Stale::default();
    let mut proxies: crate::stale::Proxies<Arc<wgpu::Texture>> = crate::stale::Proxies::new(2);
    let mut shown: Option<Vec<Placed>> = None;
    let mut spare: Option<Pair> = None;
    let mut in_flight: Option<Instant> = None;
    let mut lane: Option<raster_gpu::Coverage> = None;
    let mut puts = Vec::new();
    let mut first_notch_at: Option<u32> = None;
    let settle_ticks = 90;
    let ticks = every * notches + settle_ticks;
    let began = Instant::now();
    for tick in 0..ticks {
        let due = began + period * tick;
        if let Some(wait) = due.checked_duration_since(Instant::now()) {
            std::thread::sleep(wait);
        }
        let now = Instant::now();
        let mut landed = String::new();
        while let Ok(finished) = link.done.try_recv() {
            match finished {
                Finished::Frame(done) => {
                    // Exactly what `Window::collect` hands `quorra::surface`'s `adopt`.
                    let waited = in_flight.take().map_or(Duration::ZERO, |asked| {
                        done.finished.saturating_duration_since(asked)
                    });
                    let drew = done.finished.saturating_duration_since(done.started);
                    let built = !matches!(
                        done.cost.encode_source,
                        Some(raster_gpu::EncodeSource::Replayed)
                    );
                    stale.settled(&done.pages, drew, built);
                    landed = format!(
                        "landed: waited {:.1} ms, drawn {:.1} ms",
                        waited.as_secs_f64() * 1e3,
                        done.cost.total.as_secs_f64() * 1e3
                    );
                    spare = Some(done.textures);
                    shown = Some(done.pages);
                }
                Finished::Page(page) => proxies.keep(page),
                Finished::Sharp(sharp) => {
                    landed = format!("{landed} sharp {:.1} ms", sharp.took.as_secs_f64() * 1e3);
                }
                // A look at a frame the processor is drawing changes nothing this test
                // counts: what it reads is when each true frame landed.
                Finished::Partial(_)
                | Finished::Read(_)
                | Finished::Scene(_)
                | Finished::Lane { .. } => {}
            }
        }
        // The gesture: a notch every `every` refreshes, after a still start that lets the
        // launch frame, its sharp pass and the retained pages land.
        let warm = 60;
        let notch = tick >= warm && (tick - warm) % every == 0 && (tick - warm) / every < notches;
        if notch {
            first_notch_at.get_or_insert(tick);
            fold(
                viewer.handle(viewer_core::Command::Zoom {
                    zoom: viewer_core::Zoom::Out,
                    at: None,
                }),
                &mut requests,
            );
        }
        let pages = arrangement(&viewer, &mut requests, size);
        let was_out = in_flight.map(|asked| asked.elapsed());
        let of_this_view = shown.as_ref().is_some_and(|drawn| {
            drawn.len() == pages.len()
                && drawn.iter().zip(&pages).all(|(drawn, asked)| {
                    Arc::ptr_eq(&drawn.list, &asked.list) && drawn.target == asked.target
                })
        });
        if !of_this_view && in_flight.is_none() && !pages.is_empty() {
            let coverage = crate::lane::lane_for(
                pages[0].target.transform,
                shown
                    .as_ref()
                    .and_then(|drawn| Some(drawn.first()?.target.transform)),
                lane,
                &[],
                false,
                crate::lane::CoverageChoice::Auto,
            );
            lane = Some(coverage);
            let job = Job {
                width: size.0,
                height: size.1,
                pages: pages.clone(),
                overlays: Vec::new(),
                coverage,
                reuse: spare.take(),
                known_refusal: None,
                cheaper: None,
                interrupt: Interrupt::new(),
                on_the_processor: Arc::new(AtomicBool::new(false)),
                ahead: None,
                look: None,
                build_ahead: None,
            };
            if link.jobs.send(job).is_ok() {
                in_flight = Some(now);
            }
        }
        if !stale.has_rendering() || first_notch_at.is_none() {
            continue;
        }
        let under = proxies.placements(&pages).len();
        let put = match stale.plan(&pages, under, period, was_out, Standing::Quads) {
            Plan::Render => Put::Fresh,
            Plan::Approximate(_) => Put::StoodIn,
            Plan::Refused(crate::stale::Refusal::InsideTheRefresh { .. }) => Put::Held,
            Plan::Refused(_) => Put::Refused,
        };
        puts.push((tick, notch, put));
        if notch || put != Put::Fresh || !landed.is_empty() {
            println!(
                "tick {tick:>3}{} {put:?} {landed}",
                if notch { " notch" } else { "      " }
            );
        }
    }
    let count = |what: Put| puts.iter().filter(|(.., put)| *put == what).count();
    println!(
        "{adapter}; {notches} notches every {every} refresh(es) at 120 Hz, supersample \
         {supersample}"
    );
    println!(
        "refreshes from the first notch on: {} a rendering of the view asked for, {} stood in, \
         {} held the picture already up, {} refused",
        count(Put::Fresh),
        count(Put::StoodIn),
        count(Put::Held),
        count(Put::Refused)
    );
    // How long after each notch the first rendering of *its* view was put up.
    let mut after = Vec::new();
    let notch_ticks: Vec<u32> = puts
        .iter()
        .filter(|(_, notch, _)| *notch)
        .map(|(t, ..)| *t)
        .collect();
    for (at, next) in notch_ticks
        .iter()
        .zip(notch_ticks.iter().skip(1).map(Some).chain([None]))
    {
        let fresh = puts
            .iter()
            .find(|(t, _, put)| t >= at && next.is_none_or(|n| t < n) && *put == Put::Fresh);
        after.push(fresh.map(|(t, ..)| t - at));
    }
    println!(
        "refreshes from each notch to a rendering of its view (None: never before the next notch): {after:?}"
    );
}
