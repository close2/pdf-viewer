//! What the store and its warm-up promise, asked through them rather than of them.
//!
//! Two properties, and both are about a refusal rather than a picture: **a pipeline this
//! adapter cannot build is an `Err` that names it** (brief section 5, ADR 0042), and **a warm-up
//! that ends always says so**, including by panicking — the defect that used to hang the whole
//! suite on a `Condvar` with no notifier left alive, which is why every wait here is
//! bounded and runs on a thread of its own.
//!
//! One file for both halves of a module that is two files, deliberately: `--list` names a
//! test by its module path, so dividing these along the source's seam would rename every
//! one of them, and a rename is a change to the gate rather than to what is gated. It is
//! worth doing as its own round, with the renames as the only visible change.
#![allow(clippy::expect_used, clippy::panic)] // test-file policy: a fixture that cannot run must fail loudly

use std::sync::Arc;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use raster_scene::{Affine, BlendMode, Color, ImageFilter, ImageSpec, Point, Rect, SceneBuilder};

use super::warm::WarmUpGuard;
use super::{Kind, PipelineStore, WARM_FORMAT};
use crate::device::Device;
use crate::error::{PipelineProblem, RenderError};
use crate::startup::{Options, WarmUp};
use crate::target::Target;
use crate::viewport::Viewport;

/// The software adapter, as everywhere in this crate's tests.
fn device() -> Device {
    Device::headless(&Options {
        adapter: Some("llvmpipe".into()),
        ..Options::default()
    })
    .expect("llvmpipe is present wherever this suite runs")
}

/// **How long a wait is allowed to be before it counts as never ending.** Every
/// test here that could hang runs its wait on a thread and gives up after this, so
/// a regression fails the suite instead of wedging it — which is the trap the
/// defect these tests guard against actually sprang.
const PATIENCE: Duration = Duration::from_secs(30);

/// Call `wait` on a thread and insist it returns. Panics with `what` if it does
/// not, leaving the waiting thread behind: it is blocked on a `Condvar` and holds
/// nothing, so the process still exits.
fn within_patience<T: Send + 'static>(what: &str, wait: impl FnOnce() -> T + Send + 'static) {
    let (done, waited) = mpsc::channel();
    thread::spawn(move || {
        drop(wait());
        let _ = done.send(());
    });
    assert!(
        waited.recv_timeout(PATIENCE).is_ok(),
        "{what} did not return within {PATIENCE:?}"
    );
}

/// The defect ADR 0042 closes, stated as the property that failed: a warm-up that
/// ends by panicking must still release everything waiting on it. Before the guard,
/// this test hung — which is the whole reason the wait is bounded.
#[test]
fn a_warm_up_that_panics_still_releases_its_waiters() {
    let device = device();
    let (gpu, _) = device.wgpu();
    let store = PipelineStore::new(gpu.clone());
    let panicking = {
        let store = Arc::clone(&store);
        thread::spawn(move || {
            let _guard = WarmUpGuard::new(&store);
            panic!("a driver error wgpu reports by panicking rather than by an error scope");
        })
    };
    assert!(panicking.join().is_err(), "the thread panicked as staged");
    within_patience("wait_until_warm after a panicking warm-up", {
        let store = Arc::clone(&store);
        move || store.wait_until_warm()
    });
    assert_eq!(store.warm_up(), WarmUp::Abandoned);
    assert!(store.warm_duration().is_none());
}

/// A store whose warm-up has not run yet keeps `wait_until_warm` waiting — the
/// other half of the property above, so that "it returns" is not passing by
/// returning always.
#[test]
fn a_running_warm_up_is_reported_as_running() {
    let device = device();
    let (gpu, _) = device.wgpu();
    let store = PipelineStore::new(gpu.clone());
    assert_eq!(store.warm_up(), WarmUp::Running);
}

/// Asking where the warm-up has got to does not wait for the compile lock, which the
/// warm-up thread holds for as long as each pipeline compiles: with that lock held here,
/// both readers still answer, and answer `Running` (the caller's ADR 1558, where a
/// host's read of `Device::startup` at launch waited 4.1 to 4.3 ms on RADV for the warm set).
#[test]
fn the_warm_up_is_read_while_a_compile_holds_the_store() {
    let device = device();
    let (gpu, _) = device.wgpu();
    let store = PipelineStore::new(gpu.clone());
    let compiling = store.lock();
    within_patience("warm_up and warm_duration while the store is held", {
        let store = Arc::clone(&store);
        move || {
            assert_eq!(store.warm_up(), WarmUp::Running);
            assert!(store.warm_duration().is_none());
        }
    });
    drop(compiling);
}

/// A pipeline this adapter cannot build is an `Err` naming it, not a panic on
/// whichever thread asked (brief section 5: refused, never survived).
///
/// `Rgba8Snorm` is the instrument: WebGPU gives it no `RENDER_ATTACHMENT` usage, so
/// a colour target in that format is a validation error every backend agrees on,
/// reached without shipping a shader that does not parse.
#[test]
fn a_pipeline_the_adapter_refuses_is_an_error_and_not_a_panic() {
    let device = device();
    let (gpu, _) = device.wgpu();
    let store = PipelineStore::new(gpu.clone());
    let refused = store.get(Kind::Blit, wgpu::TextureFormat::Rgba8Snorm);
    match refused {
        Err(PipelineProblem::Pipeline {
            pipeline, format, ..
        }) => {
            assert_eq!(pipeline, "raster blit");
            assert_eq!(format, wgpu::TextureFormat::Rgba8Snorm);
        }
        other => panic!("expected a named pipeline refusal, got {other:?}"),
    }
    // Nothing was cached for the format that failed, and the store still works:
    // one refused format must not take the device with it.
    store
        .get(Kind::Blit, WARM_FORMAT)
        .expect("the blit pipeline builds for the format the frame actually uses");
}

/// A device made for a surface warms the presenting lanes in the surface's own
/// format (ADR 0043), so its first frame does not compile them inside itself —
/// asserted at the store, because this account has no window to present to.
/// `Composite` deliberately stays out of the second set: a composite's target is
/// always an internal accumulator (ADR 0038's hand-off), so warming it for the
/// surface's format would compile a pipeline no frame can reach.
#[test]
fn the_warm_set_includes_the_present_format_when_given_one() {
    let device = device();
    let (gpu, _) = device.wgpu();
    let store = PipelineStore::new(gpu.clone());
    let present = wgpu::TextureFormat::Bgra8Unorm;
    store.warm_up_now(Some(present));
    assert!(matches!(store.warm_up(), WarmUp::Warm(_)));
    let state = store.lock();
    for kind in [Kind::RectOver, Kind::CoverOver, Kind::Blit, Kind::Present] {
        assert!(
            state.pipelines.contains_key(&(kind, present)),
            "{kind:?} missing for the present format"
        );
    }
    assert!(
        !state.pipelines.contains_key(&(Kind::Composite, present)),
        "a composite never targets the surface, so warming it there is waste"
    );
    assert!(
        !state.pipelines.contains_key(&(Kind::Present, WARM_FORMAT)),
        "a presenter draws onto the surface and onto nothing else"
    );
}

/// [`Kind::Present`] is the one member of the second set that is compiled even when
/// the surface negotiated [`WARM_FORMAT`] itself — the other three are already
/// built, and this one has no first set to be in (ADR 0056). This is what makes
/// `Device::detach_presenter` a move rather than a compile: the pipeline a presenter
/// needs exists before it is asked for, built by the thread nobody blocks on.
#[test]
fn the_presenting_pass_is_warmed_for_the_surfaces_format_whatever_it_is() {
    let device = device();
    let (gpu, _) = device.wgpu();
    for present in [WARM_FORMAT, wgpu::TextureFormat::Bgra8Unorm] {
        let store = PipelineStore::new(gpu.clone());
        store.warm_up_now(Some(present));
        assert!(matches!(store.warm_up(), WarmUp::Warm(_)));
        assert!(
            store
                .lock()
                .pipelines
                .contains_key(&(Kind::Present, present)),
            "the present pass is missing for {present:?}"
        );
    }
    // And a headless device warms none of it: there is no surface to present to.
    let store = PipelineStore::new(gpu.clone());
    store.warm_up_now(None);
    assert!(
        !store
            .lock()
            .pipelines
            .keys()
            .any(|(kind, _)| *kind == Kind::Present),
        "a headless device has no surface, so the presenting pass is waste"
    );
}

/// The other half of ADR 0043's rule, which the presenter relies on: a pipeline the
/// warm set did not build is built by the first ask, **inline**, and that ask says
/// what it cost — which is how `PresentCost::compiled` can be truthful about a
/// presenter detached before its device was warm.
#[test]
fn a_pipeline_the_warm_set_missed_is_compiled_by_the_first_ask_and_says_so() {
    let device = device();
    let (gpu, _) = device.wgpu();
    let store = PipelineStore::new(gpu.clone());
    let (_, compiled) = store
        .get(Kind::Present, WARM_FORMAT)
        .expect("the present pipeline builds for a format the adapter renders to");
    assert!(
        compiled.is_some(),
        "the ask that compiled a pipeline has to be able to say so"
    );
    let (_, again) = store
        .get(Kind::Present, WARM_FORMAT)
        .expect("and the second ask finds it cached");
    assert!(
        again.is_none(),
        "a cached pipeline costs nothing and must not claim a compile"
    );
}

/// The other half: a headless device (no present format) and a surface whose
/// format is already [`WARM_FORMAT`] compile no second set at all.
#[test]
fn the_warm_set_compiles_one_format_when_no_second_is_needed() {
    let device = device();
    let (gpu, _) = device.wgpu();
    for present in [None, Some(WARM_FORMAT)] {
        let store = PipelineStore::new(gpu.clone());
        store.warm_up_now(present);
        assert!(matches!(store.warm_up(), WarmUp::Warm(_)));
        let state = store.lock();
        assert!(
            state
                .pipelines
                .keys()
                .all(|(_, format)| *format == WARM_FORMAT),
            "no pipeline outside {WARM_FORMAT:?} belongs in this warm set"
        );
    }
}

/// The refusal above must reach a caller in the words of the error, not only as a
/// variant — a person reading a log has to be able to attribute it.
#[test]
fn a_refusal_names_the_pipeline_and_the_format() {
    let device = device();
    let (gpu, _) = device.wgpu();
    let store = PipelineStore::new(gpu.clone());
    let Err(reason) = store.get(Kind::Composite, wgpu::TextureFormat::Rgba8Snorm) else {
        panic!("Rgba8Snorm is not a renderable format");
    };
    let message = reason.to_string();
    assert!(message.contains("raster composite"), "{message}");
    assert!(message.contains("Rgba8Snorm"), "{message}");
}

/// A pipeline `create_render_pipeline` refuses takes the frame that needs it with it, by name,
/// and nothing else (ADR 1929): the frame is a [`RenderError`] a window answers by drawing on
/// the processor, the next frame that needs the pipeline is refused in the same words without a
/// second compile, and a frame that does not need it still draws on the device.
///
/// The plant is the image lane built against an entry point its module does not have — a
/// refusal every backend agrees on, as the owner's DX12 compiler refused the lane's real one.
#[test]
fn a_frame_whose_pipeline_will_not_build_is_refused_by_name_and_only_that_frame() {
    let mut device = device();
    device.pipelines().break_pipeline(Kind::ImageOver);
    let image = device
        .upload_image(&ImageSpec {
            width: 2,
            height: 2,
            data: Arc::from(vec![255_u8; 16]),
        })
        .expect("a two-by-two image uploads");
    let mut builder = SceneBuilder::new();
    builder
        .image(
            image,
            Affine {
                a: 8.0,
                d: 8.0,
                ..Affine::IDENTITY
            },
            1.0,
            ImageFilter::Auto { interpolate: false },
            None,
            BlendMode::Normal,
            None,
        )
        .expect("an image command is a valid scene");
    let with_image = builder.finish();
    let mut refusal = || match device.render(
        &with_image,
        &Viewport::full(8, 8, Affine::IDENTITY),
        Target::Readback,
    ) {
        Err(RenderError::PipelineUnavailable {
            reason: PipelineProblem::Pipeline {
                pipeline, detail, ..
            },
        }) => (pipeline, detail),
        other => panic!("expected the image lane refused by name, got {other:?}"),
    };
    let first = refusal();
    assert_eq!(first.0, "raster image");
    assert_eq!(
        refusal(),
        first,
        "the second frame hears the first refusal's words"
    );
    assert!(
        device
            .pipelines()
            .lock()
            .refused
            .contains_key(&(Kind::ImageOver, WARM_FORMAT)),
        "the refusal is kept, so the second frame compiled nothing"
    );
    let mut builder = SceneBuilder::new();
    builder
        .rect(
            Rect::new(Point { x: 0.0, y: 0.0 }, Point { x: 8.0, y: 8.0 }),
            Affine::IDENTITY,
            Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 1.0,
            },
            None,
            None,
        )
        .expect("a rectangle is a valid scene");
    device
        .render(
            &builder.finish(),
            &Viewport::full(8, 8, Affine::IDENTITY),
            Target::Readback,
        )
        .expect("a frame with no image does not need the refused pipeline");
}
