//! Are the resource identifiers this backend hands quorra the same from one render of
//! one display list to the next?
//!
//! quorra keys every glyph-lane tile on the outline identifier the fill names, and its
//! atlas holds those keys until a whole-atlas repack throws them away. So an identifier
//! that moves between two renders of the *same* content makes every key foreign, fills
//! the atlas twice over and repacks — for ever, at period two. The quorra developers
//! measured that on this crate's single-list [`Rasterizer::rasterize`] path and handed it
//! back (`render-lib/doc/notes-atlas-budget.md` section 5); this is the instrument on
//! this side of the boundary.
//!
//! What it prints per frame: how many resources went to the device
//! ([`render_quorra::FrameCost::uploads`] — cache misses plus transients), whether the
//! atlas repacked, and what the page's own tiles would cost. **A still page must upload
//! nothing after its first frame.**
//!
//! ```sh
//! cargo run --release -p render-quorra --example outline_stability \
//!     -- [file.pdf] [page] [scale] [atlas budget in bytes]
//! ```
//!
//! The atlas budget is last because it is the argument that reproduces *their* run rather than
//! this program's: at 1 MiB the oscillation appears within two frames, at quorra's default it
//! takes several — the budget decides how soon the atlas is full, not whether the keys move.
#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]

use pdf_render::{Command, Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_quorra::QuorraRasterizer;

/// Commands by kind, over the whole tree — a group's children are commands too, and a page
/// whose top level is one group would otherwise read as five commands.
struct Census {
    to_device: pdf_render::Transform,
    fills: u32,
    collapsing: u32,
    strokes: u32,
    degenerate: u32,
    dashed: u32,
    anisotropic: u32,
    images: u32,
    groups: u32,
}

impl Census {
    fn new(to_device: pdf_render::Transform) -> Self {
        Self {
            to_device,
            fills: 0,
            collapsing: 0,
            strokes: 0,
            degenerate: 0,
            dashed: 0,
            anisotropic: 0,
            images: 0,
            groups: 0,
        }
    }

    fn walk(&mut self, commands: &[Command]) {
        for command in commands {
            match command {
                Command::Fill { path, .. } => {
                    self.fills = self.fills.saturating_add(1);
                    if path.collapses() {
                        self.collapsing = self.collapsing.saturating_add(1);
                    }
                }
                Command::Stroke {
                    path,
                    transform,
                    stroke,
                    ..
                } => {
                    self.strokes = self.strokes.saturating_add(1);
                    let to_device = transform.then(self.to_device);
                    let width = stroke.device_width(to_device);
                    if pdf_render::split_degenerate(path, stroke.cap, width, None).is_some() {
                        self.degenerate = self.degenerate.saturating_add(1);
                    }
                    if !stroke.dash_array.is_empty() && stroke.dash_array.iter().sum::<f32>() > 0.0
                    {
                        self.dashed = self.dashed.saturating_add(1);
                    }
                    let max = to_device.max_stretch();
                    let determinant = to_device.determinant().abs();
                    if max.is_finite()
                        && max > 0.0
                        && determinant.is_finite()
                        && determinant > 0.0
                        && (max * max) / determinant > 1.01
                    {
                        self.anisotropic = self.anisotropic.saturating_add(1);
                    }
                }
                Command::Image { .. } => self.images = self.images.saturating_add(1),
                Command::Group { commands, .. } => {
                    self.groups = self.groups.saturating_add(1);
                    self.walk(commands);
                }
                Command::Shaped { object, .. } => {
                    self.walk(std::slice::from_ref(object.as_ref()));
                }
                _ => {}
            }
        }
    }
}

fn main() {
    let mut arguments = std::env::args().skip(1);
    let path = arguments.next().unwrap_or_else(|| {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../doc/pdf.js/test/pdfs/issue12295.pdf")
            .to_string_lossy()
            .into_owned()
    });
    let number: usize = arguments
        .next()
        .and_then(|index| index.parse().ok())
        .unwrap_or(1);
    let scale: f32 = arguments
        .next()
        .and_then(|value| value.parse().ok())
        .unwrap_or(4.0);
    // The atlas the repack is a property of. Its default is generous enough that a single page
    // rendered alone takes many frames to fill it, which is why the quorra developers reported
    // the oscillation at 1 MiB: a smaller atlas reaches the same steady state sooner, it does
    // not create it.
    let default_atlas = quorra_gpu::Options::default().atlas_budget;
    let atlas_budget: u64 = arguments
        .next()
        .and_then(|value| value.parse().ok())
        .unwrap_or(default_atlas);

    let document = Document::open(std::fs::read(&path).unwrap()).expect("opens");
    let pages = pdf_model::Pages::new(&document);
    let page = pages
        .get(number.saturating_sub(1))
        .expect("the page exists");
    let list = pdf_model::content::interpret(&document, &page).display_list;
    let target = TargetSpec::for_page(&list, scale, 1 << 30).unwrap();

    // What the display list itself says about the routes `scene::Encoder` takes: a fill whose
    // path has a collapsed subpath is split and re-uploaded per frame, a stroke's outline is
    // computed per frame, and an ordinary fill is looked up by its `Arc`'s identity.
    let mut census = Census::new(target.transform);
    census.walk(list.commands());

    let mut backend = QuorraRasterizer::with_options(&quorra_gpu::Options {
        atlas_budget,
        ..render_quorra::options()
    })
    .expect("adapter");
    println!("{path} page {number} at {scale}×");
    println!("{}", backend.adapter_description());
    println!(
        "{} × {}, atlas budget {atlas_budget}",
        target.width, target.height
    );
    println!(
        "fills {} (collapsing {}), images {}, groups {}",
        census.fills, census.collapsing, census.images, census.groups
    );
    println!(
        "strokes {} — degenerate {}, dashed {}, anisotropic {}",
        census.strokes, census.degenerate, census.dashed, census.anisotropic
    );
    println!(
        "{:>5}  {:>9}  {:>7}  {:>12}  {:>9}",
        "frame", "uploads", "repack", "working set", "ms"
    );
    for frame in 1..=8 {
        backend.rasterize(&list, target).expect("draws");
        let cost = backend.last_frame();
        println!(
            "{frame:>5}  {:>9}  {:>7}  {:>12}  {:>9.1}",
            cost.uploads,
            if cost.atlas_repacked { "yes" } else { "." },
            cost.atlas_working_set_bytes,
            cost.total.as_secs_f64() * 1000.0,
        );
    }
}
