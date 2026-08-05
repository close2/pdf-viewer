//! Does the sidebar survive a magnified page? The window's whole frame, offscreen.
//!
//! `doc/todo/12`'s first half: open a page in a window, press `+` until the magnification passes
//! about 2000%, and §12.3.3's outline panel stops being drawn on the graphics device — the
//! background rectangle survives, the rows and the tab strip do not, and the whole thing shifts
//! down by an amount that grows with the zoom. Under `--cpu` it is drawn whole at every rung.
//!
//! **No gate in this tree could see it**, and that is the reason this file exists. The corpus and
//! the oracle rasterise a page; `render-quorra/tests/corpus.rs` rasterises a page at 1×, 2× and
//! 4×; `render-quorra/examples/zoom_ladder` magnifies a page and draws no chrome. Every one of
//! them rasterises **one** display list, and the window draws several into one scene — the page
//! under its target transform and the overlays at identity over it. That combination is what
//! breaks, so that combination is what this walks.
//!
//! The closed form needs no reference renderer, and that is the point: **the overlay is the same
//! display list at the same target on every rung, so its pixels may not depend on the page's
//! magnification.**
//!
//! So the panel's own columns are cropped out of each frame and compared with the first rung's.
//! Anything that moves is the defect, measured without asking anybody what a sidebar should look
//! like.
//!
//! ```sh
//! cargo run --release -p viewer-ui --example chrome_ladder -- [file.pdf] [page] [out-dir]
//! ```

#![expect(
    clippy::print_stdout,
    reason = "an example whose entire output is a measurement"
)]
#![expect(
    clippy::expect_used,
    reason = "an instrument that cannot set itself up must fail loudly rather than measure nothing"
)]

use pdf_render::{TargetSpec, Transform};
use render_quorra::{PresentFrame, QuorraRasterizer};
use viewer_core::{Answer, Command, DocumentId, Layer, Query, Viewer};
use viewer_ui::chrome::{Chrome, Content, Sidebar, Tab};

/// The window this pretends to be — `doc/todo/12`'s own 900 × 1100.
const WINDOW: (u32, u32) = (900, 1100);

/// The magnification `viewer-ui` switches quorra to its GPU coverage lane above.
///
/// The same constant `pdf-viewer.rs` uses, restated rather than shared because a binary's
/// constant is not an API — and a ladder that does not switch lanes is not measuring what a
/// person sees past 1000% (`doc/QUORRA_FEEDBACK.md` §11).
const GPU_COVERAGE_MAGNIFICATION: f32 = 10.0;

/// How close two frames' panels have to be to be called the same.
///
/// Not zero, and the residual is measured rather than assumed: a device with no history at all
/// still differs from the rung before it by a mean of 0.0003 over the panel, one glyph edge at
/// worst 16 — the sub-pixel placement of the panel's own text against a page drawn at a
/// different scale. The defect this instrument exists for is four orders of magnitude above it.
const SAME: f64 = 0.01;

/// The rungs, as multiples of the page fitted to the window.
///
/// Chosen to straddle `doc/todo/12`'s table: the panel is whole at 12 presses (about 19×) and
/// gone at 14 (about 30×).
const RUNGS: [f32; 7] = [1.0, 4.0, 12.0, 19.0, 30.0, 46.0, 64.0];

fn main() {
    let mut arguments = std::env::args().skip(1);
    let path = arguments.next().unwrap_or_else(|| {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../doc/PDF20_AN001-BPC.pdf")
            .to_string_lossy()
            .into_owned()
    });
    let page: usize = arguments
        .next()
        .and_then(|index| index.parse().ok())
        .unwrap_or(3);
    let out = arguments.next();

    let bytes = std::fs::read(&path).expect("the document is readable");
    let mut viewer = Viewer::new(WINDOW.0, WINDOW.1, 1.0);
    let opened = viewer
        .handle(Command::Open {
            id: DocumentId(1),
            bytes: bytes.clone(),
            password: None,
        })
        .any(|event| matches!(event, viewer_core::Event::Opened { .. }));
    assert!(opened, "the document opens");

    // The panel, built from what the document says about itself — the same four queries the
    // window asks, so what is drawn here is what a person sees.
    let chrome = Chrome::new().expect("the compiled-in Helvetica loads");
    let mut sidebar = Sidebar::default();
    sidebar.show(Tab::Contents);
    let outline = match viewer.query(Query::Outline) {
        Answer::Outline(outline) => outline.clone(),
        _ => pdf_model::outline::Outline::default(),
    };
    let layers: Vec<Layer> = match viewer.query(Query::Layers) {
        Answer::Layers(layers) => layers,
        _ => Vec::new(),
    };
    let attachments = match viewer.query(Query::Attachments) {
        Answer::Attachments(files) => files,
        _ => Vec::new(),
    };
    let (information, metadata) = match viewer.query(Query::Properties) {
        Answer::Properties {
            information,
            metadata,
        } => (information, metadata),
        _ => (pdf_model::metadata::Information::default(), None),
    };
    let content = Content {
        outline: &outline,
        layers: &layers,
        attachments: &attachments,
        information: &information,
        metadata: metadata.as_ref(),
        pages: &[],
    };
    let panel = sidebar.draw(&chrome, content, WINDOW.1, 1.0);
    let edge = sidebar.inset(1.0);
    assert!(edge > 0, "the sidebar is showing, so it insets the page");

    // **The page's placement is `viewer-core`'s own**, taken from the render request the viewer
    // raises rather than computed here: the fit, the flip, the centring and the scroll are four
    // decisions this instrument must not make a second copy of, and a ladder measured against a
    // transform of its own would be measuring the copy.
    let _ = viewer.handle(Command::Resize {
        width: WINDOW.0.saturating_sub(edge),
        height: WINDOW.1,
        scale: 1.0,
    });
    let _ = viewer.handle(Command::GoTo(viewer_core::PageTarget::Index(
        page.saturating_sub(1),
    )));

    println!("page {page} of {path}");
    println!("panel {edge} px wide, {} commands", panel.commands().len());

    // **Two passes, and the second is the control.** One device drawing every rung is what a
    // person's window is; a device per rung is a device with no history. If the ladder is clean
    // on the second and not on the first, whatever goes wrong is *state carried between frames*
    // rather than anything about the magnification — which is the same control
    // `doc/QUORRA_FEEDBACK.md` §11 used on the page's own glyphs, and finding the same answer
    // here is what says the two are one defect.
    for fresh in [false, true] {
        println!();
        println!(
            "{}",
            if fresh {
                "a new device per rung — no frame has been drawn on it before"
            } else {
                "one device, every rung in order — which is what a window is"
            }
        );
        ladder(&mut viewer, &panel, edge, fresh, out.as_deref());
    }
}

/// Walks the rungs, drawing the window's whole frame and comparing the panel's own columns.
#[expect(
    clippy::too_many_lines,
    reason = "one rung end to end, in the order a window does it: the zoom, the placement, the \
              lane, the frame, the crop and the comparison. Splitting it would put that order in \
              two places"
)]
fn ladder(
    viewer: &mut Viewer,
    panel: &pdf_render::DisplayList,
    edge: u32,
    fresh: bool,
    out: Option<&str>,
) {
    let mut gpu = QuorraRasterizer::new_headless_software().expect("a software adapter exists");
    println!("adapter: {}", gpu.adapter_description());
    println!("    zoom        page target      panel mean   panel worst     ink   verdict");

    // **One reference per coverage lane**, and that is not a convenience. The two lanes are two
    // rasterisers, so a rung that switches lanes differs from the rung before it for a reason
    // that is not a defect — on this page by a constant 0.42 mean over the panel, which is glyph
    // antialiasing. What may not differ is one lane against itself at another magnification.
    let mut first: [Option<Vec<u8>>; 2] = [None, None];
    for rung in RUNGS {
        if fresh {
            gpu = QuorraRasterizer::new_headless_software().expect("a software adapter exists");
        }
        // What a person's `+` does, through the same command a key press sends.
        let Some(request) = latest_request(viewer, rung) else {
            println!("  {:>5.0}%   the viewer asked for no render", rung * 100.0);
            continue;
        };
        // Where the host puts it: the core centres and scrolls the page, and the host composes
        // that offset with the panel's width into the target's own transform (`present`).
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
        let overlays: Vec<&pdf_render::DisplayList> = vec![panel];
        let frame = PresentFrame {
            width: WINDOW.0,
            height: WINDOW.1,
            page: Some((&request.list, target)),
            raster: None,
            overlays: &overlays,
        };
        let raster = match gpu.rasterize_frame(&frame) {
            Ok(raster) => raster,
            Err(error) => {
                println!("  {:>5.0}%   refused: {error}", rung * 100.0);
                continue;
            }
        };
        let cropped = crop(&raster, edge);
        let ink = ink_of(&cropped);
        let held = first.get(lane).and_then(Option::as_ref);
        let (mean, worst) = match held {
            None => (0.0, 0),
            Some(reference) => difference(reference, &cropped),
        };
        let verdict = if held.is_none() {
            if lane == 1 {
                "reference (GPU lane)"
            } else {
                "reference (CPU lane)"
            }
        } else if mean < SAME {
            "same"
        } else {
            "**MOVED**"
        };
        println!(
            "  {:>5.0}%  {:>7} × {:<7}   {mean:>9.4}   {worst:>11}   {ink:>5.2}   {verdict}",
            rung * 100.0,
            request.target.width,
            request.target.height,
        );
        if let Some(directory) = out {
            let _ = std::fs::create_dir_all(directory);
            let held = if fresh { "fresh" } else { "held" };
            write_png(
                &format!("{directory}/panel-{held}-{:04.0}.png", rung * 100.0),
                &cropped,
                edge,
                WINDOW.1,
            );
            write_png(
                &format!("{directory}/frame-{held}-{:04.0}.png", rung * 100.0),
                &raster.data,
                WINDOW.0,
                WINDOW.1,
            );
        }
        if let Some(slot) = first.get_mut(lane) {
            slot.get_or_insert(cropped);
        }
        // **Answering is what makes this a tier-2 host**, and it is not bookkeeping: `MAX_PIXELS`
        // bounds a raster `viewer-core` hands back, and a host that says `Presented` has told it
        // there will not be one — so the budget stops refusing pages nothing was going to
        // allocate. Without this line the ladder stops at about 20×, which is a fact about the
        // instrument rather than about the window.
        let _ = viewer
            .handle(Command::RenderReady {
                token: request.token,
                rendered: viewer_core::Rendered::Presented,
            })
            .count();
    }

    println!(
        "  the panel is the same display list at the same target on every rung, so any row but \
         `same` is a frame whose chrome moved with the page's magnification"
    );
}

/// The render request a zoom to `rung` times the fitted size produces.
///
/// The viewer coalesces: one `Command::Zoom` raises at most one `NeedsRender`, and the *last* one
/// is the frame that would be drawn. `Zoom::Scale` is logical pixels per user-space unit, so the
/// rung has to be turned into one — `Query::PageGeometry` answers with the scale the fit chose.
fn latest_request(viewer: &mut Viewer, rung: f32) -> Option<viewer_core::RenderRequest> {
    let _ = viewer
        .handle(Command::Zoom {
            zoom: viewer_core::Zoom::FitPage,
            at: None,
        })
        .count();
    let fitted = match viewer.query(Query::PageGeometry(0)) {
        Answer::Geometry(geometry) => geometry.scale,
        _ => 1.0,
    };
    viewer
        .handle(Command::Zoom {
            zoom: viewer_core::Zoom::Scale(fitted * rung),
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
        reason = "a pixel count, far inside f64's exact range"
    )]
    let mean = total as f64 / a.len().max(1) as f64;
    (mean, worst)
}

/// How dark the crop is, on the same 0..255 scale `doc/todo/00`'s ink is.
///
/// A panel drawn whole and a panel drawn as a bare rectangle differ in this by the rows, which is
/// what makes it worth printing beside a difference: the difference says something moved, the ink
/// says whether the text is there at all.
fn ink_of(crop: &[u8]) -> f64 {
    let mut total = 0_u64;
    let mut pixels = 0_u64;
    for pixel in crop.chunks_exact(4) {
        let grey = u64::from(pixel[0])
            .saturating_add(u64::from(pixel[1]))
            .saturating_add(u64::from(pixel[2]));
        total = total.saturating_add(grey / 3);
        pixels = pixels.saturating_add(1);
    }
    #[expect(
        clippy::cast_precision_loss,
        reason = "a pixel count, far inside f64's exact range"
    )]
    let mean = total as f64 / pixels.max(1) as f64;
    255.0 - mean
}

/// Writes an RGBA buffer where a person can look at it.
fn write_png(path: &str, data: &[u8], width: u32, height: u32) {
    let Ok(file) = std::fs::File::create(path) else {
        println!("  (could not write {path})");
        return;
    };
    let mut encoder = png::Encoder::new(file, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let written = encoder
        .write_header()
        .and_then(|mut writer| writer.write_image_data(data));
    if let Err(error) = written {
        println!("  (could not write {path}: {error})");
    }
}
