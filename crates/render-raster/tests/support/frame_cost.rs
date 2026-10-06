//! What one page's frame costs, stage by stage, measured the one way two instruments share:
//! `examples/frame_budget`, which prints it, and `tests/turn_path.rs`, which holds
//! `doc/performance.md`'s table of it to bands (ADR 1513).
//!
//! **One copy of the method, because the gate's figures are claims about the table's.** A row of
//! that table is the minimum of rounds of exactly what [`round`] does — a page interpreted against
//! the font cache a page turn has, drawn on a device warmed by another document on the lane a page
//! turn takes, and placed again at twice the magnification on the lane a moved view takes — and a
//! gate that measured anything else would be holding a number nobody recorded. The module comment
//! of `examples/frame_budget.rs` has the argument for each of those choices.

use std::sync::Arc;
use std::time::{Duration, Instant};

use pdf_render::{DisplayList, TargetSpec, Transform};
use pdf_syntax::Document;
use render_raster::{FrameCost, PresentFrame, QuorraRasterizer};

/// The document every round's device is warmed with, and the page of it.
///
/// Committed to this repository, small, and none of the measured pages — so it fills the
/// device's pipelines and its first-use allocations without filling any cache the measured page
/// would hit. **A page of *this* document is the one input the method cannot answer for**: its
/// outlines are in the device's caches before the timed frame, so its `turn` row would be a warm
/// row wearing a cold row's name.
pub(crate) const WARM_UP: (&str, usize) = ("doc/PDF20_AN001-BPC.pdf", 1);

/// The window every figure is measured in, in device pixels.
///
/// The same viewport `launch_path.rs` states, and for its reason: a frame's cost is a function of
/// how many pixels are drawn, so a figure taken at the machine's own screen size would be a
/// different question on a different machine.
pub(crate) const WINDOW: (u32, u32) = (1600, 1000);

/// How long every core a round may run on is kept busy just before the round (ADR 1577).
///
/// **A round is a claim about the tree, and the clock an idle processor wakes at is not part of
/// the tree.** This machine's governor (`amd-pstate-epp`, `balance_performance`) gives a core that
/// has idled for a second a clock a quarter lower than one that has been working, and a child of
/// a light page is short enough to live its whole run at the clock it started at: the mesh page's
/// turn read 11.2 to 11.7 ms that way and 9.3 to 9.8 after this spin, the calibration probe beside
/// it declined 24 of 90 children against 2, and the heavy rows moved by under 5% (ADRs 1519,
/// 1556, 1577). Thirty milliseconds is what ADRs 1519 and 1556 measured with.
pub(crate) const WARM_SPIN: Duration = Duration::from_millis(30);

/// Keeps every core this process may run on busy for [`WARM_SPIN`], one thread a core, and
/// returns when all of them have stopped.
///
/// The figure that follows is then the tree's at the clock a working processor runs at, which is
/// the state the bands were taken in; what an idle machine adds to a first frame is the launch
/// gate's subject, measured there as a person meets it.
pub(crate) fn warm_the_cores() {
    let cores = std::thread::available_parallelism().map_or(1, std::num::NonZero::get);
    let began = Instant::now();
    std::thread::scope(|scope| {
        for _ in 0..cores {
            scope.spawn(|| {
                let mut state = 1_u64;
                while began.elapsed() < WARM_SPIN {
                    for _ in 0..1_000 {
                        state = std::hint::black_box(
                            state
                                .wrapping_mul(6_364_136_223_846_793_005)
                                .wrapping_add(1),
                        );
                    }
                }
            });
        }
    });
}

/// Milliseconds, the unit every frame line in this project is in.
pub(crate) fn ms(span: Duration) -> f64 {
    span.as_secs_f64() * 1e3
}

/// One frame's stages, in milliseconds, with the host's walk separated from the library's.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct Stages {
    /// `pdf_model::content::interpret_with_fonts`, zero for a frame that interprets nothing.
    pub(crate) interpret: f64,
    /// The scene walk.
    pub(crate) scene: f64,
    /// raster's encode.
    pub(crate) encode: f64,
    /// The upload, which on a photograph is mostly the host's reduction (ADR 1433).
    pub(crate) transfer: f64,
    /// The device's own passes.
    pub(crate) execute: f64,
    /// Host time inside raster's `render` that no phase names.
    pub(crate) elsewhere: f64,
    /// The copy back off the device, which only an offscreen frame pays.
    pub(crate) readback: f64,
    /// Bytes uploaded.
    pub(crate) bytes: u64,
    /// Uploads made.
    pub(crate) uploads: u32,
}

impl Stages {
    /// The stages of `cost`, with `interpret` beside them.
    pub(crate) fn of(interpret: f64, cost: FrameCost) -> Self {
        // `device` less the phases raster names, which is host time inside its own `render` —
        // the same arithmetic `zoom_frame` does, and a bound rather than a duration because
        // `execute` is usually the adapter's clock and the rest are this thread's.
        let elsewhere = cost
            .device
            .saturating_sub(cost.encode)
            .saturating_sub(cost.upload)
            .saturating_sub(cost.execute)
            .saturating_sub(cost.readback);
        Self {
            interpret,
            scene: ms(cost.scene),
            encode: ms(cost.encode),
            transfer: ms(cost.upload),
            execute: ms(cost.execute),
            elsewhere: ms(elsewhere),
            readback: ms(cost.readback),
            bytes: cost.bytes_uploaded,
            uploads: cost.uploads,
        }
    }

    /// What a window would wait for: everything but the readback an offscreen frame alone pays.
    pub(crate) fn budget(self) -> f64 {
        self.interpret + self.scene + self.encode + self.transfer + self.execute + self.elsewhere
    }
}

/// Keeps whichever of two samples of the same row is the quicker.
pub(crate) fn keep(slot: &mut Option<Stages>, sample: Stages) {
    if slot.is_none_or(|kept| sample.budget() < kept.budget()) {
        *slot = Some(sample);
    }
}

/// The page placed in the window at this magnification, centred — what the keyboard zoom does.
fn placed(list: &DisplayList, zoom: f32, window: (u32, u32)) -> TargetSpec {
    let page = TargetSpec::for_page(list, zoom, 1 << 30).expect("a target");
    let size = list.page_size;
    let centre = Transform::translate(
        (window.0 as f32).mul_add(0.5, -(size.width * zoom * 0.5)),
        (window.1 as f32).mul_add(0.5, -(size.height * zoom * 0.5)),
    );
    TargetSpec {
        width: window.0,
        height: window.1,
        transform: page.transform.then(centre),
    }
}

/// Draws one placed list into the window-sized target and hands back what it cost.
fn draw(backend: &mut QuorraRasterizer, list: &Arc<DisplayList>, target: TargetSpec) -> FrameCost {
    let frame = PresentFrame {
        width: target.width,
        height: target.height,
        pages: &[(list, target)],
        raster: None,
        overlays: &[],
    };
    backend
        .rasterize_frame(&frame)
        .unwrap_or_else(|error| panic!("refused: {error}"));
    backend.last_frame()
}

/// One page interpreted, as a page turn interprets it.
pub(crate) struct Read {
    /// The display list.
    pub(crate) list: Arc<DisplayList>,
    /// What the interpretation took, in milliseconds.
    pub(crate) interpret: f64,
    /// How many commands the list holds — the witness that the page drew what it draws.
    pub(crate) commands: usize,
    /// Whether the font cache was the one a predecessor left, which is what a page turn has —
    /// false for a one-page document, where a viewer arriving at the page has an empty cache too.
    pub(crate) preceded: bool,
}

/// Opens a document and interprets one of its pages, reporting what the interpretation cost.
///
/// **The font cache is the one a page turn has, and getting that wrong halves the figure.**
/// `viewer-core` keeps a [`FontCache`](pdf_model::content::FontCache) for as long as a document
/// is open and hands it to every interpretation, so §9.6's font programs are loaded once per
/// document rather than once per page — a page arrived at by `Command::GoTo(Next)` pays for the
/// faces its predecessor did not use and for no others. So the page *before* this one is
/// interpreted first, against the same cache, and only the second interpretation is timed.
pub(crate) fn read(path: &str, index: usize) -> Read {
    let document = Document::open(std::fs::read(path).unwrap_or_else(|error| {
        panic!("{path} is readable: {error}");
    }))
    .expect("it opens");
    let pages = pdf_model::Pages::new(&document);
    let state = pdf_model::view::ViewState::of(&document);
    let fonts = pdf_model::content::FontCache::new();
    let preceded = index > 1;
    if preceded && let Some(before) = pages.get(index.saturating_sub(2)) {
        drop(pdf_model::content::interpret_with_fonts(
            &document, &before, &state, &fonts,
        ));
    }
    let page = pages
        .get(index.saturating_sub(1))
        .unwrap_or_else(|| panic!("{path} has a page {index}"));
    let began = Instant::now();
    let interpretation = pdf_model::content::interpret_with_fonts(&document, &page, &state, &fonts);
    let interpret = ms(began.elapsed());
    let list = interpretation.display_list;
    let commands = list.command_count();
    Read {
        list: Arc::new(list),
        interpret,
        commands,
        preceded,
    }
}

/// A device on `lane`, warmed by [`WARM_UP`] and by nothing else, and its adapter's name.
fn warmed_device(lane: raster_gpu::Coverage, window: (u32, u32)) -> (QuorraRasterizer, String) {
    let mut backend =
        QuorraRasterizer::with_options(&render_raster::options()).expect("an adapter");
    backend.set_coverage(lane);
    let description = backend.adapter_description().to_owned();
    let (path, page) = WARM_UP;
    let warm = read(path, page);
    let _warmed = draw(&mut backend, &warm.list, placed(&warm.list, 1.0, window));
    (backend, description)
}

/// One round of one page: its three rows, each on a device of its own where the row needs one.
pub(crate) struct Round {
    /// The page interpreted and drawn on a device that has drawn another document's page.
    pub(crate) turn: Stages,
    /// The same frame asked for again, nothing changed.
    pub(crate) warm: Stages,
    /// The page placed at twice the magnification against the caches the first placement filled.
    pub(crate) step: Stages,
    /// [`Read::commands`].
    pub(crate) commands: usize,
    /// [`Read::preceded`].
    pub(crate) preceded: bool,
    /// The adapter the round drew on.
    pub(crate) adapter: String,
}

/// One round of `path`'s page `index` in a window of `window` device pixels, on cores
/// [`warm_the_cores`] has just kept busy.
pub(crate) fn round(path: &str, index: usize, window: (u32, u32)) -> Round {
    warm_the_cores();
    let page = read(path, index);
    // The page turn and the repaint after it, on the lane `coverage_for` gives every
    // magnification below 10× — which is the lane the window turns a page on.
    let (mut backend, adapter) = warmed_device(raster_gpu::Coverage::Cpu, window);
    let target = placed(&page.list, 1.0, window);
    let turn = Stages::of(page.interpret, draw(&mut backend, &page.list, target));
    let warm = Stages::of(0.0, draw(&mut backend, &page.list, target));
    drop(backend);
    // The zoom step, on the moved-view lane, against the caches the first placement
    // filled — one notch of a gesture rather than a first sight of the page.
    let (mut backend, _) = warmed_device(raster_gpu::Coverage::Compute, window);
    let _first = draw(&mut backend, &page.list, target);
    let stepped = placed(&page.list, 2.0, window);
    let step = Stages::of(0.0, draw(&mut backend, &page.list, stepped));
    Round {
        turn,
        warm,
        step,
        commands: page.commands,
        preceded: page.preceded,
        adapter,
    }
}
