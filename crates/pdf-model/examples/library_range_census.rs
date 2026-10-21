//! Which first pages pass `tiny-skia`'s 8191-pixel range at the corpus gates' two scales, and which
//! of them hold an element of a §11.4.6 knockout group — the marks this backend hands to the
//! library's own converters, whose anti-aliasing the range decides (ADRs 2149, 2166).
//!
//! ```sh
//! cargo run --release -p pdf-model --example library_range_census -- doc/pdf.js/test/pdfs/*.pdf
//! ```
//!
//! One line per page past the range at either scale — its extent at each, its knockout groups and
//! the elements they hold — then one line for the run. A page under the range at both scales
//! prints nothing: the range is asked of its whole extent, so no mark on it can pass it except by
//! lying off the page, which a strip, a band and a window share with the page drawn whole.

#![expect(
    clippy::print_stdout,
    reason = "an example whose entire output is a measurement"
)]

use pdf_model::{Pages, interpret};
use pdf_render::{Command, TargetSpec, command_extents};
use pdf_syntax::Document;

/// The largest device coordinate the library's anti-aliased converter expresses, its own figure
/// (`render_cpu::scan::SUPERSAMPLED_LIMIT`).
const RANGE: u32 = 8_191;

/// [`RANGE`] as a device coordinate, which `f32` holds exactly.
const RANGE_F32: f32 = 8_191.0;

/// The scales `render-raster`'s corpus gate draws every first page at, as the six arms do.
const SCALES: [f32; 2] = [1.0, 4.0];

/// No pixel budget: this asks how large a page is, not whether a gate would draw it.
const UNBOUNDED: u64 = u64::MAX;

/// Knockout groups in `commands` and the elements directly inside them, nested groups included.
fn knockouts(commands: &[Command]) -> (usize, usize) {
    let mut found = (0_usize, 0_usize);
    for command in commands {
        let inner = match command {
            Command::Group {
                commands, knockout, ..
            } => {
                if *knockout {
                    found.0 = found.0.saturating_add(1);
                    found.1 = found.1.saturating_add(commands.len());
                }
                knockouts(commands)
            }
            Command::Shaped { object, .. } => knockouts(std::slice::from_ref(&**object)),
            _ => (0, 0),
        };
        found = (
            found.0.saturating_add(inner.0),
            found.1.saturating_add(inner.1),
        );
    }
    found
}

fn main() {
    let (mut documents, mut past, mut past_with_knockout) = (0_usize, [0_usize; 2], [0_usize; 2]);
    for path in std::env::args().skip(1) {
        let name = std::path::Path::new(&path)
            .file_name()
            .map_or_else(|| path.clone(), |name| name.to_string_lossy().into_owned());
        let Ok(bytes) = std::fs::read(&path) else {
            println!("{name}\tunreadable");
            continue;
        };
        let Ok(document) = Document::open(bytes) else {
            println!("{name}\tunopened");
            continue;
        };
        documents = documents.saturating_add(1);
        let Some(page) = Pages::new(&document).get(0) else {
            continue;
        };
        let list = interpret(&document, &page).display_list;
        let extents = SCALES.map(|scale| {
            TargetSpec::for_page(&list, scale, UNBOUNDED)
                .ok()
                .map(|target| (target.width, target.height))
        });
        let beyond = extents
            .map(|extent| extent.is_none_or(|(width, height)| width > RANGE || height > RANGE));
        if !beyond.contains(&true) {
            continue;
        }
        let (groups, elements) = knockouts(list.commands());
        for (index, &is_past) in beyond.iter().enumerate() {
            if is_past {
                past[index] = past[index].saturating_add(1);
                if groups > 0 {
                    past_with_knockout[index] = past_with_knockout[index].saturating_add(1);
                }
            }
        }
        println!(
            "{name}\t1x {:?}\t4x {:?}\t{groups} knockout group(s), {elements} element(s)",
            extents[0], extents[1]
        );
        // Where on the page each top-level command holding a knockout group lies, at each scale
        // past the range: only a mark beyond 8191 on the page's grid is one the range decides.
        for (scale, &is_past) in SCALES.iter().zip(&beyond) {
            let Some(target) = TargetSpec::for_page(&list, *scale, UNBOUNDED).ok() else {
                continue;
            };
            if !is_past {
                continue;
            }
            let reach = command_extents(&list, target);
            for (command, extent) in list.commands().iter().zip(&reach) {
                if knockouts(std::slice::from_ref(command)).0 > 0 {
                    let past_it =
                        extent.is_some_and(|rect| rect.max.x > RANGE_F32 || rect.max.y > RANGE_F32);
                    println!("  {scale}x\tknockout at {extent:?}\tpast the range: {past_it}");
                }
            }
        }
    }
    println!(
        "# {documents} documents; past the range at 1x {} page(s), {} with a knockout group; at \
         4x {} page(s), {} with a knockout group",
        past[0], past_with_knockout[0], past[1], past_with_knockout[1]
    );
}
