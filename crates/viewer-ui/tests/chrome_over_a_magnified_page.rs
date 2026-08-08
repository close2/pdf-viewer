//! Chrome drawn over a page does not depend on the page's magnification.
//!
//! # Why this gate exists
//!
//! Every other gate in this tree rasterises **one** display list: the corpus and the oracle a
//! page, `render-quorra/tests/corpus.rs` a page at 1×, 2× and 4×, `viewer-ui/tests/panel.rs` the
//! panel alone. A window draws several into one scene — the page under its target transform and
//! the overlays at identity over it — and for four sessions that combination lost the sidebar
//! above about 2000% magnification with nothing able to see it (ADR 0198). The defect was
//! the rendering library's and is fixed; the hole in the instruments was this tree's.
//!
//! # What it checks, and why it needs no reference
//!
//! The panel is the **same display list at the same target** on every rung of a zoom ladder, so
//! its pixels may not depend on the page's magnification. That is a closed form: no second
//! renderer is asked what a sidebar should look like, and a frame whose chrome moved fails
//! against a frame of this test's own.
//!
//! One reference per **coverage lane**, because `viewer-ui` switches quorra's lane above 10×
//! (`GPU_COVERAGE_MAGNIFICATION`) and the two lanes are two rasterisers: a rung that switches
//! lanes differs for a reason that is not a defect. Within a lane the frames must agree.
//!
//! `examples/chrome_ladder` is the same walk with the pictures and the whole table, for when
//! this fails.

#![expect(
    clippy::expect_used,
    reason = "test code: a fixture that cannot be set up must fail loudly rather than pass by \
              doing nothing"
)]

use pdf_render::{TargetSpec, Transform};
use render_quorra::{PresentFrame, QuorraRasterizer};
use viewer_core::{Answer, Command, DocumentId, Query, Rendered, Viewer, Zoom};
use viewer_ui::chrome::{Chrome, Content, Sidebar, Tab};

/// The window this pretends to be, which is ADR 0198's own.
const WINDOW: (u32, u32) = (900, 1100);

/// The magnification `viewer-ui` switches quorra to its GPU coverage lane above.
const GPU_COVERAGE_MAGNIFICATION: f32 = 10.0;

/// The rungs, as multiples of the page fitted to the window.
///
/// They straddle where the defect lived: whole at 19×, gone at 30× and 46×, back at 64×. The
/// non-monotone shape is the point — a ladder that stopped at the first failure would have
/// called this a magnification limit.
const RUNGS: [f32; 6] = [1.0, 4.0, 12.0, 19.0, 30.0, 64.0];

/// How close two frames' panels have to be to be called the same.
///
/// Not zero, and the residual is measured rather than assumed: a device with no history at all
/// still differs from the rung before it by a mean of 0.0003 over the panel — one glyph edge, at
/// worst 16 of 255 — because the panel's own text is composited over a page drawn at another
/// scale. What this gate was written for was four orders of magnitude above it (3.77).
const SAME: f64 = 0.01;

/// The document every rung draws, which is one of the two the tree already opens by name.
const DOCUMENT: &str = "doc/PDF20_AN001-BPC.pdf";

/// Which page: three, because it is the page the defect was reported on.
const PAGE: usize = 3;

#[test]
fn the_sidebar_does_not_depend_on_the_pages_magnification() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(DOCUMENT);
    let Ok(bytes) = std::fs::read(&path) else {
        // The specification archive is unpacked by hand (`doc/specifications.zip`), so a fresh
        // clone has no document to zoom. Skipping loudly is what every other gate here does.
        println!("skipped: {DOCUMENT} is not unpacked");
        return;
    };
    let mut gpu = match QuorraRasterizer::new_headless_software() {
        Ok(gpu) => gpu,
        Err(error) => {
            println!("skipped: no software adapter on this machine: {error}");
            return;
        }
    };

    let mut viewer = Viewer::new(WINDOW.0, WINDOW.1, 1.0);
    let opened = viewer
        .handle(Command::Open {
            id: DocumentId(1),
            bytes,
            password: None,
            fragment: None,
        })
        .any(|event| matches!(event, viewer_core::Event::Opened { .. }));
    assert!(opened, "the fixture opens");

    let (panel, edge) = sidebar(&viewer);
    assert!(edge > 0, "a shown sidebar insets the page");
    // The viewport the page gets is the window less the panel, which is what the host resizes to.
    let _ = viewer
        .handle(Command::Resize {
            width: WINDOW.0.saturating_sub(edge),
            height: WINDOW.1,
            scale: 1.0,
        })
        .count();
    let _ = viewer
        .handle(Command::GoTo(viewer_core::PageTarget::Index(
            PAGE.saturating_sub(1),
        )))
        .count();

    let mut first: [Option<Vec<u8>>; 2] = [None, None];
    for rung in RUNGS {
        let request = zoom_to(&mut viewer, rung).expect("the viewer asks for a render");
        let origin = match viewer.query(Query::PageGeometry(request.page)) {
            Answer::Geometry(geometry) => geometry.origin,
            _ => (0.0, 0.0),
        };
        #[expect(
            clippy::cast_precision_loss,
            reason = "a panel width in pixels, which is hundreds"
        )]
        let target = TargetSpec {
            width: WINDOW.0,
            height: WINDOW.1,
            transform: request
                .target
                .transform
                .then(Transform::translate(origin.0 + edge as f32, origin.1)),
        };
        let lane = usize::from(rung >= GPU_COVERAGE_MAGNIFICATION);
        gpu.set_coverage(if lane == 1 {
            quorra_gpu::Coverage::Gpu
        } else {
            quorra_gpu::Coverage::Cpu
        });
        let overlays: Vec<&pdf_render::DisplayList> = vec![&panel];
        let raster = gpu
            .rasterize_frame(&PresentFrame {
                width: WINDOW.0,
                height: WINDOW.1,
                page: Some((&request.list, target)),
                raster: None,
                overlays: &overlays,
            })
            .unwrap_or_else(|error| panic!("the frame at {:.0}% refused: {error}", rung * 100.0));

        let cropped = crop(&raster, edge);
        if let Some(reference) = first.get(lane).and_then(Option::as_ref) {
            let (mean, worst) = difference(reference, &cropped);
            assert!(
                mean < SAME,
                "the sidebar changed when only the page's magnification did: at {:.0}% it is \
                 {mean:.4} from the same panel at the first rung of this coverage lane (worst \
                 channel {worst} of 255). `cargo run --release -p viewer-ui --example \
                 chrome_ladder -- {DOCUMENT} {PAGE} <out-dir>` writes the frames.",
                rung * 100.0,
            );
        }
        if let Some(slot) = first.get_mut(lane) {
            slot.get_or_insert(cropped);
        }
        // Tier 2: saying the frame was presented is what tells `viewer-core` no whole-page raster
        // will be held for this host, so `MAX_PIXELS` stops bounding an allocation nobody makes.
        // Without it the ladder cannot pass about 20× at all.
        let _ = viewer
            .handle(Command::RenderReady {
                token: request.token,
                rendered: Rendered::Presented,
            })
            .count();
    }
}

/// §12.3.3's outline panel, built from what the document says about itself.
fn sidebar(viewer: &Viewer) -> (pdf_render::DisplayList, u32) {
    let chrome = Chrome::new().expect("the compiled-in Helvetica loads");
    let mut sidebar = Sidebar::default();
    sidebar.show(Tab::Contents);
    let outline = match viewer.query(Query::Outline) {
        Answer::Outline(outline) => outline,
        _ => pdf_model::outline::Outline::default(),
    };
    let information = pdf_model::metadata::Information::default();
    let content = Content {
        outline: &outline,
        layers: &[],
        attachments: &[],
        articles: &[],
        collection: None,
        information: &information,
        metadata: None,
        pages: &[],
    };
    let list = sidebar.draw(&chrome, content, WINDOW.1, 1.0);
    (list, sidebar.inset(1.0))
}

/// The render request a zoom to `rung` times the fitted size produces.
fn zoom_to(viewer: &mut Viewer, rung: f32) -> Option<viewer_core::RenderRequest> {
    let _ = viewer
        .handle(Command::Zoom {
            zoom: Zoom::FitPage,
            at: None,
        })
        .count();
    let fitted = match viewer.query(Query::PageGeometry(0)) {
        Answer::Geometry(geometry) => geometry.scale,
        _ => 1.0,
    };
    viewer
        .handle(Command::Zoom {
            zoom: Zoom::Scale(fitted * rung),
            at: None,
        })
        .filter_map(|event| match event {
            viewer_core::Event::NeedsRender(request) => Some(request),
            _ => None,
        })
        .last()
}

/// The panel's own columns, out of a whole frame.
fn crop(raster: &pdf_render::Raster, width: u32) -> Vec<u8> {
    let mut out =
        Vec::with_capacity((width.saturating_mul(raster.height) as usize).saturating_mul(4));
    for row in 0..raster.height {
        let start = (row.saturating_mul(raster.width) as usize).saturating_mul(4);
        let end = start.saturating_add((width as usize).saturating_mul(4));
        out.extend_from_slice(raster.data.get(start..end).unwrap_or_default());
    }
    out
}

/// Mean and worst absolute channel difference between two crops of the same size.
fn difference(a: &[u8], b: &[u8]) -> (f64, u8) {
    let mut total = 0_u64;
    let mut worst = 0_u8;
    for (left, right) in a.iter().zip(b.iter()) {
        let delta = left.abs_diff(*right);
        total = total.saturating_add(u64::from(delta));
        worst = worst.max(delta);
    }
    #[expect(
        clippy::cast_precision_loss,
        reason = "a byte count, far inside f64's exact range"
    )]
    let mean = total as f64 / a.len().max(1) as f64;
    (mean, worst)
}
