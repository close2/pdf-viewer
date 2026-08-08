//! What [`pdf_render::Image::area_averaged`] costs, against the serial shape it replaced.
//!
//! The reduction is paid per *source sample*, not per output cell, so it is invisible in any
//! count of a page's commands: session 390 measured a 388-command page of the project owner's
//! own document translating into a GPU scene sixteen times slower than a 3675-command page of
//! text, and the whole of the difference was one photograph being averaged down (ADR 0228).
//!
//! Two changes came out of that, and this is the instrument that says what each is worth:
//! the column bands computed once per image rather than once per output cell, and the output
//! rows divided across rayon's pool above `pdf_render`'s own measured floor. **Neither changes
//! an arithmetic step**, so the two forms must agree byte for byte, and this asserts that on
//! every size it times rather than only reporting the clock.
//!
//! ```sh
//! cargo run --release -p pdf-render --example area_bench -- [runs]
//! ```
//!
//! **Best of `runs`, with the mean beside it.** Load only ever adds time, so the minimum is
//! the closest a loaded machine gets to the work itself; the mean is printed too because a
//! parallel division's overhead shows up there first and a figure that hid it would argue for
//! the division on the strength of its best case.
#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::print_stdout,
    reason = "an example: it prints, its arithmetic and its casts are over literal image \
              dimensions under four thousand and residues under 256, and a failure to reduce \
              means the fixture is wrong rather than the library"
)]

use std::sync::Arc;
use std::time::Instant;

use pdf_render::{Image, Transform};

/// A fixture whose samples vary in every channel, so an averaging bug cannot hide behind a
/// flat colour. `opaque` chooses between the common scanned-page case and one where the
/// premultiplication in `Image::area_averaged` actually does something.
fn make(width: u32, height: u32, opaque: bool) -> Image {
    let mut data = Vec::with_capacity((width as usize) * (height as usize) * 4);
    for y in 0..height {
        for x in 0..width {
            data.push((x % 251) as u8);
            data.push((y % 241) as u8);
            data.push(((x ^ y) % 239) as u8);
            data.push(if opaque { 255 } else { ((x + y) % 256) as u8 });
        }
    }
    Image {
        width,
        height,
        data: data.into(),
        interpolate: false,
    }
}

/// `pdf_render`'s own `Bands`, which is private to that crate — copied rather than exposed,
/// because widening a type's interface so a benchmark can re-implement its predecessor is a
/// worse trade than twenty lines that never change.
#[derive(Clone, Copy)]
struct Bands {
    samples: u64,
    cells: u64,
}

impl Bands {
    fn new(samples: u32, cells: u32) -> Self {
        Self {
            samples: u64::from(samples),
            cells: u64::from(cells.max(1)),
        }
    }

    fn at(self, index: u32) -> (u32, u32) {
        let edge = |i: u64| {
            let scaled = i.saturating_mul(self.samples).checked_div(self.cells);
            u32::try_from(scaled.unwrap_or(0).min(self.samples)).unwrap_or(u32::MAX)
        };
        let start = edge(u64::from(index));
        (start, edge(u64::from(index) + 1).max(start))
    }
}

fn round_div(numerator: u64, denominator: u64) -> u8 {
    if denominator == 0 {
        return 0;
    }
    let rounded = numerator
        .saturating_add(denominator / 2)
        .checked_div(denominator)
        .unwrap_or(0);
    u8::try_from(rounded.min(u64::from(u8::MAX))).unwrap_or(u8::MAX)
}

fn average_block(image: &Image, x0: u32, y0: u32, x1: u32, y1: u32) -> [u8; 4] {
    let mut colour = [0u64; 3];
    let mut alpha_sum = 0u64;
    let mut count = 0u64;
    for y in y0..y1 {
        let row = (y as usize) * (image.width as usize);
        let from = (row + x0 as usize) * 4;
        let to = (row + x1 as usize) * 4;
        let Some(span) = image.data.get(from..to) else {
            continue;
        };
        for sample in span.chunks_exact(4) {
            let alpha = u64::from(sample[3]);
            for (sum, component) in colour.iter_mut().zip(sample) {
                *sum += u64::from(*component) * alpha;
            }
            alpha_sum += alpha;
            count += 1;
        }
    }
    if count == 0 || alpha_sum == 0 {
        return [0, 0, 0, 0];
    }
    let mut out = [0u8; 4];
    for (channel, sum) in out.iter_mut().zip(colour) {
        *channel = round_div(sum, alpha_sum);
    }
    out[3] = round_div(alpha_sum, count);
    out
}

/// The shape `Image::area_averaged` had until session 391: one walk, `Bands::at` asked once
/// per output cell. Kept here rather than left in the library's history because a ratio
/// nobody can re-derive is a ratio that rots.
fn serial(image: &Image, factor: u32) -> Arc<[u8]> {
    let width = image.width.div_ceil(factor);
    let height = image.height.div_ceil(factor);
    let columns = Bands::new(image.width, width);
    let rows = Bands::new(image.height, height);
    let mut data: Vec<u8> = Vec::with_capacity((width as usize) * (height as usize) * 4);
    for out_y in 0..height {
        let (y0, y1) = rows.at(out_y);
        for out_x in 0..width {
            let (x0, x1) = columns.at(out_x);
            data.extend_from_slice(&average_block(image, x0, y0, x1, y1));
        }
    }
    data.into()
}

/// The same walk with the column bands hoisted and nothing else changed — the half of the
/// improvement that costs no thread, timed separately so the division is not credited with it.
fn serial_hoisted(image: &Image, factor: u32) -> Arc<[u8]> {
    let width = image.width.div_ceil(factor);
    let height = image.height.div_ceil(factor);
    let columns = Bands::new(image.width, width);
    let rows = Bands::new(image.height, height);
    let spans: Vec<(u32, u32)> = (0..width).map(|out_x| columns.at(out_x)).collect();
    let mut data: Vec<u8> = Vec::with_capacity((width as usize) * (height as usize) * 4);
    for out_y in 0..height {
        let (y0, y1) = rows.at(out_y);
        for &(x0, x1) in &spans {
            data.extend_from_slice(&average_block(image, x0, y0, x1, y1));
        }
    }
    data.into()
}

/// The hoisted walk with its rows divided across rayon's pool **whatever** the image's size,
/// so that the crossover `pdf_render`'s parallel floor is set at can be read off rather than
/// asserted. The library takes this path only above the floor.
fn divided(image: &Image, factor: u32) -> Arc<[u8]> {
    use rayon::prelude::*;
    let width = image.width.div_ceil(factor);
    let height = image.height.div_ceil(factor);
    let columns = Bands::new(image.width, width);
    let rows = Bands::new(image.height, height);
    let spans: Vec<(u32, u32)> = (0..width).map(|out_x| columns.at(out_x)).collect();
    let row_bytes = (width as usize) * 4;
    let mut data: Vec<u8> = vec![0; row_bytes * (height as usize)];
    data.par_chunks_exact_mut(row_bytes)
        .enumerate()
        .for_each(|(out_y, row)| {
            let (y0, y1) = rows.at(u32::try_from(out_y).unwrap_or(u32::MAX));
            for (cell, &(x0, x1)) in row.chunks_exact_mut(4).zip(&spans) {
                cell.copy_from_slice(&average_block(image, x0, y0, x1, y1));
            }
        });
    data.into()
}

fn time(label: &str, runs: u32, mut produce: impl FnMut() -> Arc<[u8]>) -> Arc<[u8]> {
    let first = produce();
    let mut best = f64::INFINITY;
    let mut total = 0.0;
    for _ in 0..runs {
        let began = Instant::now();
        let out = std::hint::black_box(produce());
        let each = began.elapsed().as_secs_f64() * 1.0e3;
        drop(out);
        best = best.min(each);
        total += each;
    }
    println!(
        "  {label:20} best {best:8.3} ms   mean {:8.3} ms",
        total / f64::from(runs.max(1))
    );
    first
}

fn run(label: &str, image: &Image, factor: u32, runs: u32) {
    println!(
        "{label}: {}x{} reduced {factor}-fold, {} source sample(s)",
        image.width,
        image.height,
        u64::from(image.width) * u64::from(image.height)
    );
    // A placement carries the unit square onto the device, so an edge of this length asks for
    // exactly `factor` source samples per device pixel — which is what `Image::reduction`
    // reads off it.
    let placement = Transform::new(
        image.width as f32 / factor as f32,
        0.0,
        0.0,
        image.height as f32 / factor as f32,
        0.0,
        0.0,
    );
    let library = time("library", runs, || {
        image.area_averaged(placement).expect("reduces").data
    });
    let before = time("before, serial", runs, || serial(image, factor));
    let hoisted = time("of which hoisting", runs, || serial_hoisted(image, factor));
    let parallel = time("divided regardless", runs, || divided(image, factor));
    assert_eq!(library, before, "the reduced samples moved");
    assert_eq!(library, hoisted, "the reduced samples moved");
    assert_eq!(library, parallel, "the reduced samples moved");
    println!("  (byte-identical)");
}

fn main() {
    let runs: u32 = std::env::args()
        .nth(1)
        .and_then(|argument| argument.parse().ok())
        .unwrap_or(40);
    // The first four straddle the parallel floor; the last four are the shapes the owner's own
    // `NorthAmerican.30MB.pdf` puts on a page, taken from session 391's trace of it.
    run("small", &make(64, 64, true), 3, runs * 20);
    run("small", &make(128, 128, true), 3, runs * 10);
    run("small", &make(256, 256, true), 3, runs * 5);
    run("small", &make(512, 512, true), 3, runs * 2);
    run("witness, opaque", &make(1374, 1374, true), 3, runs);
    run("witness, alpha", &make(1374, 1374, false), 3, runs);
    run("witness, opaque", &make(2100, 1448, true), 3, runs);
    run("witness, page one", &make(2700, 3450, true), 3, runs);
}
