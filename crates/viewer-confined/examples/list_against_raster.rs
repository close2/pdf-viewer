//! What a display list would cost to send, against what its raster costs.
//!
//! `doc/todo/34` §2 names two ways to put a window on the confinement: the confined process
//! ships **display lists** and the host's warm device rasterises them, or the confined process
//! is handed a window and drives a device itself. The first is decided in large part by one
//! number nobody had measured — **how big a display list is beside the pixels it produces** —
//! because a display list is what would cross in place of a raster, and it crosses the same
//! pipe.
//!
//! # What is counted, and why it is a wire size rather than a heap size
//!
//! `std::mem::size_of` would count pointers, capacity slack and the memoised hulls a
//! [`pdf_render::Path`] builds on first use — none of which travels. So this walks the list
//! and sums what an encoder **must** write: one byte per tag, four per index, four per `f32`,
//! and the payload of everything the receiving side cannot reconstruct. It is the same
//! accounting `viewer_confined::protocol` already does for a `Raster`, applied to the other
//! artefact.
//!
//! Two figures are printed rather than one, and the gap between them is a design constraint:
//!
//! - **shared** counts each distinct `Arc<Path>`, `Arc<[u8]>` and `Arc<Shading>` once, with
//!   every later occurrence costing a four-byte index. This is what an encoder that preserves
//!   the sharing writes.
//! - **flat** counts each occurrence whole, which is what the obvious encoder writes.
//!
//! [`pdf_render::Command`]'s own documentation says why the gap is large: "3005 fill commands
//! on a dense specification page carried 101 320 path segments between them".
//!
//! # The population this cannot price, which is the finding rather than a caveat
//!
//! `ImageSource::AtDeviceScale` and `ShadingKind::Sampled` carry an
//! `Arc<dyn ImageAtDeviceScale>` and an `Arc<dyn ColoursAtDeviceScale>` — **producers, not
//! data**, invoked by the backend once it knows how many device pixels the mark covers (ADR
//! 0210). Both are self-contained rather than closures over the document, so an encoding for
//! them is possible; what it is not is *this* accounting, because its size depends on §7.10's
//! function types and on `pdf-model`'s colour conversion rather than on anything the display
//! list holds. So these are counted, not sized, and the count is the population ADR 0607's
//! decision leaves to the raster arm.
//!
//! ```sh
//! cargo run --release -p viewer-confined --example list_against_raster -- [--scale N] <file.pdf>…
//! ```
//!
//! The raster it is compared against is the whole page at `--scale` (default 1.0, which is 72
//! dpi and the smallest a window ever asks for). **A display list is scale-invariant and a
//! raster is quadratic in the scale**, so the default column is the display list's *worst*
//! case rather than its typical one, and a ratio has to be read beside the scale it was taken
//! at.
#![expect(
    clippy::print_stdout,
    clippy::arithmetic_side_effects,
    clippy::cast_precision_loss,
    reason = "an example whose entire output is a measurement; its counters are bounded by one \
              page's commands, and the ratios it prints are printed to three decimals"
)]

use std::collections::BTreeSet;
use std::sync::Arc;

use pdf_render::{
    Clip, ClipId, Command, DisplayList, ImageSource, Paint, Shading, ShadingKind, SoftMask,
    SoftMaskId, TargetSpec,
};

/// Bytes an encoder writes for a tag byte.
const TAG: usize = 1;
/// Bytes for a 32-bit index or count.
const INDEX: usize = 4;
/// Bytes for an `f32`.
const F32: usize = 4;
/// Bytes for a [`pdf_render::Transform`]: six `f32`.
const TRANSFORM: usize = 6 * F32;
/// Bytes for a [`pdf_render::Color`]: four `f32`.
const COLOUR: usize = 4 * F32;
/// Bytes for an `Option<Id>`: a presence byte and an index.
const OPTIONAL_ID: usize = 1 + INDEX;
/// Bytes for the clip, mask and blend every mark carries.
const MARK_STATE: usize = OPTIONAL_ID + OPTIONAL_ID + TAG;

/// What one page's list would cost to send, split so that the shape of the cost is legible.
#[derive(Default)]
struct Weight {
    /// Command records, exclusive of the tables they index into.
    commands: usize,
    /// Distinct path geometry.
    paths: usize,
    /// Distinct image samples.
    images: usize,
    /// Distinct shadings, exclusive of anything deferred.
    shadings: usize,
    /// Clip and soft-mask tables.
    regions: usize,
    /// Occurrences of a producer that cannot be encoded at all.
    deferred: usize,
    /// How many commands were walked, groups' elements included.
    marks: usize,
}

impl Weight {
    fn total(&self) -> usize {
        self.commands + self.paths + self.images + self.shadings + self.regions
    }
}

/// Which shared objects have already been written, so that the second occurrence costs an index.
#[derive(Default)]
struct Seen {
    paths: BTreeSet<usize>,
    images: BTreeSet<usize>,
    shadings: BTreeSet<usize>,
}

/// Whether `pointer` is new to `set`, recording it either way.
fn first_time(set: &mut BTreeSet<usize>, pointer: usize) -> bool {
    set.insert(pointer)
}

fn path_bytes(path: &pdf_render::Path) -> usize {
    INDEX
        + path
            .commands()
            .iter()
            .map(|step| {
                TAG + match step {
                    pdf_render::PathCommand::MoveTo(_) | pdf_render::PathCommand::LineTo(_) => {
                        2 * F32
                    }
                    pdf_render::PathCommand::CurveTo(..) => 6 * F32,
                    pdf_render::PathCommand::Close => 0,
                }
            })
            .sum::<usize>()
}

fn ramp_bytes(ramp: &pdf_render::Ramp) -> usize {
    INDEX + ramp.stops.len() * (F32 + COLOUR)
}

fn shading_bytes(shading: &Shading, weight: &mut Weight) -> usize {
    let kind = match shading.kind.as_ref() {
        ShadingKind::Axial { ramp, .. } => 2 * 2 * F32 + 2 + ramp_bytes(ramp),
        ShadingKind::Radial { ramp, .. } => 2 * 2 * F32 + 2 * F32 + 2 + ramp_bytes(ramp),
        ShadingKind::Sampled { program, .. } => {
            // The colours are a producer. Nothing here can encode one; see the module comment.
            weight.deferred += 1;
            4 * F32
                + program.as_ref().map_or(0, |program| {
                    INDEX + program.steps().len() * (TAG + F32) + 2 * F32
                })
        }
        ShadingKind::Mesh { triangles, ramp } => {
            INDEX
                + triangles.len() * (3 * 2 * F32 + TAG + 3 * COLOUR)
                + ramp.as_ref().map_or(0, ramp_bytes)
        }
        // The enum is `#[non_exhaustive]`; a kind added later is priced as its tag alone and
        // the count of marks below is what would show the omission.
        _ => 0,
    };
    TAG + TRANSFORM + 1 + shading.background.map_or(0, |_| COLOUR) + kind
}

fn paint_bytes(paint: &Paint, seen: &mut Seen, weight: &mut Weight) -> usize {
    match paint {
        Paint::Solid(_) => TAG + COLOUR,
        Paint::Shading(shading) => {
            if first_time(&mut seen.shadings, Arc::as_ptr(shading) as usize) {
                weight.shadings += shading_bytes(shading, weight);
            }
            TAG + INDEX
        }
        _ => TAG,
    }
}

/// Walks one command, adding what it and anything newly shared would cost.
fn walk(command: &Command, seen: &mut Seen, weight: &mut Weight, flat: bool) {
    weight.marks += 1;
    match command {
        Command::Fill { path, paint, .. } => {
            weight.commands += TAG + TRANSFORM + TAG + MARK_STATE + INDEX;
            if flat || first_time(&mut seen.paths, Arc::as_ptr(path) as usize) {
                weight.paths += path_bytes(path);
            }
            weight.commands += paint_bytes(paint, seen, weight);
        }
        Command::Stroke {
            path,
            paint,
            stroke,
            ..
        } => {
            weight.commands += TAG
                + TRANSFORM
                + MARK_STATE
                + INDEX
                + F32
                + 3 * TAG
                + F32
                + INDEX
                + stroke.dash_array.len() * F32
                + F32;
            if flat || first_time(&mut seen.paths, Arc::as_ptr(path) as usize) {
                weight.paths += path_bytes(path);
            }
            weight.commands += paint_bytes(paint, seen, weight);
        }
        Command::Image { image, .. } => {
            weight.commands += TAG + TRANSFORM + F32 + MARK_STATE + INDEX;
            match image {
                ImageSource::Decoded(decoded) => {
                    let identity = Arc::as_ptr(&decoded.data).cast::<u8>() as usize;
                    if flat || first_time(&mut seen.images, identity) {
                        weight.images += 2 * INDEX + 1 + decoded.data.len();
                    }
                }
                ImageSource::AtDeviceScale(_) => weight.deferred += 1,
                _ => {}
            }
        }
        Command::Group { commands, .. } => {
            weight.commands += TAG + F32 + MARK_STATE + 3 + INDEX;
            for element in commands {
                walk(element, seen, weight, flat);
            }
        }
        Command::Shaped { object, shape } => {
            weight.commands += TAG;
            walk(object, seen, weight, flat);
            walk(shape, seen, weight, flat);
        }
        _ => weight.commands += TAG,
    }
}

/// The clips a command chain actually references, which is what would cross.
fn reachable_clips(command: &Command, into: &mut Vec<ClipId>) {
    if let Some(clip) = command.clip() {
        into.push(clip);
    }
    match command {
        Command::Group { commands, .. } => {
            for element in commands {
                reachable_clips(element, into);
            }
        }
        Command::Shaped { object, shape } => {
            reachable_clips(object, into);
            reachable_clips(shape, into);
        }
        _ => {}
    }
}

fn clip_bytes(clip: &Clip) -> usize {
    path_bytes(&clip.path) + TRANSFORM + TAG + OPTIONAL_ID
}

fn soft_mask_bytes(mask: &SoftMask, seen: &mut Seen, weight: &mut Weight, flat: bool) -> usize {
    for element in &mask.commands {
        walk(element, seen, weight, flat);
    }
    TAG + COLOUR + mask.transfer.as_ref().map_or(1, |_| 1 + 256)
}

/// Everything one list would cost, at `flat` or with the sharing preserved.
fn weigh(list: &DisplayList, flat: bool) -> Weight {
    let mut weight = Weight::default();
    let mut seen = Seen::default();
    for command in list.commands() {
        walk(command, &mut seen, &mut weight, flat);
    }

    // Every soft mask the list holds: its elements are commands and cost what commands cost.
    for index in 0..list.soft_mask_count() {
        let Ok(index) = u32::try_from(index) else {
            continue;
        };
        if let Some(mask) = list.soft_mask(SoftMaskId::new(index)) {
            weight.regions += soft_mask_bytes(mask, &mut seen, &mut weight, flat);
        }
    }

    // The clip table, closed under the parent chains. Only the clips something references
    // travel, which is why this walks the commands rather than the table's whole length.
    let mut pending = Vec::new();
    for command in list.commands() {
        reachable_clips(command, &mut pending);
    }
    for index in 0..list.soft_mask_count() {
        let Ok(index) = u32::try_from(index) else {
            continue;
        };
        if let Some(mask) = list.soft_mask(SoftMaskId::new(index)) {
            for element in &mask.commands {
                reachable_clips(element, &mut pending);
            }
        }
    }
    let mut written = BTreeSet::new();
    while let Some(id) = pending.pop() {
        if !written.insert(id.index()) {
            continue;
        }
        let Some(clip) = list.clip(id) else { continue };
        weight.regions += clip_bytes(clip);
        if let Some(parent) = clip.parent {
            pending.push(parent);
        }
    }
    weight
}

fn main() {
    let mut scale = 1.0_f32;
    let mut paths = Vec::new();
    let mut args = std::env::args().skip(1);
    while let Some(argument) = args.next() {
        if argument == "--scale" {
            scale = args
                .next()
                .and_then(|value| value.parse().ok())
                .unwrap_or(1.0);
        } else {
            paths.push(argument);
        }
    }

    println!(
        "# name\tpage\tmarks\tshared_B\tflat_B\traster_B\tshared/raster\tflat/raster\tdeferred"
    );
    let mut shared_total = 0_u64;
    let mut flat_total = 0_u64;
    let mut raster_total = 0_u64;
    let mut deferred_pages = 0_usize;
    let mut bigger = 0_usize;
    let mut pages = 0_usize;
    for path in paths {
        let name = std::path::Path::new(&path)
            .file_name()
            .map_or_else(|| path.clone(), |name| name.to_string_lossy().into_owned());
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        let Ok(document) = pdf_syntax::Document::open(bytes) else {
            continue;
        };
        let Some(page) = pdf_model::Pages::new(&document).get(0) else {
            continue;
        };
        let list = pdf_model::interpret(&document, &page).display_list;
        let shared = weigh(&list, false);
        let flat = weigh(&list, true);
        let Ok(target) = TargetSpec::for_page(&list, scale, 1 << 28) else {
            continue;
        };
        let raster = u64::from(target.width) * u64::from(target.height) * 4;
        if raster == 0 {
            continue;
        }
        pages += 1;
        shared_total += shared.total() as u64;
        flat_total += flat.total() as u64;
        raster_total += raster;
        if shared.deferred > 0 {
            deferred_pages += 1;
        }
        if shared.total() as u64 > raster {
            bigger += 1;
        }
        println!(
            "{name}\t1\t{}\t{}\t{}\t{raster}\t{:.3}\t{:.3}\t{}",
            shared.marks,
            shared.total(),
            flat.total(),
            shared.total() as f64 / raster as f64,
            flat.total() as f64 / raster as f64,
            shared.deferred,
        );
    }
    println!(
        "# {pages} page(s) at scale {scale}: shared {shared_total} B, flat {flat_total} B, \
         raster {raster_total} B"
    );
    println!(
        "# shared/raster {:.3} in aggregate; {bigger} page(s) whose list exceeds its raster; \
         {deferred_pages} page(s) carrying a producer that cannot be encoded",
        shared_total as f64 / raster_total.max(1) as f64,
    );
}
