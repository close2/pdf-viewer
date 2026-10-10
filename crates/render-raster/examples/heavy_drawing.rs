//! The biggest page the device draws: a deterministic drawing in the shape of the project
//! owner's witness, a size dial, and each size drawn on the device and on the CPU oracle.
//!
//! The owner's `demo_heavy_engineering_A1.pdf` is one file, and a ceiling read off one file is a
//! fact about that file. Its shape is not: an A1 sheet (2384 × 1684 points) of equipment racks in
//! cubicle grids, rooms filled and hatched, grid lines and text labels — and, the part that makes
//! it heavy, every rack's shading line stated as **unit steps in a `0.12 0 0 0.12 0 0 cm` space**,
//! one `q … cm m l S Q` per step and a colour change every few. Its one content stream inflates to
//! 278 579 415 bytes and holds 4 400 073 such strokes, each 0.12 points long, which is 0.12 of a
//! device pixel at a scale of one. This example writes that shape from a formula, so a size can be
//! asked for and the same bytes come back on every machine.
//!
//! **The dial is the producer's chop**, `steps_per_point`: how many strokes a point of rack line
//! is cut into. The layout is fixed — 20 × 15 blocks, one in four a hatched room and the rest
//! racks of 4 × 3 cubicles with eight shading lines — so the page's pixels at window size stay
//! where they are and only the command count moves, which is the axis the witness is extreme on.
//! Each unit of chop adds 505 920 strokes to 43 739 other commands: `steps_per_point = 8` is
//! 4 091 099 commands, nine-tenths of the witness's 4 408 405, and the witness's own 0.12-point
//! step is a chop of 8⅓.
//!
//! ```sh
//! # write one size to a file, to open in a window
//! cargo run --release -p render-raster --example heavy_drawing -- write 8 tmp/heavy/a1-8.pdf
//! # draw sizes on the device and on the oracle, and compare their bytes
//! cargo run --release -p render-raster --example heavy_drawing -- ladder 1,2,4,8
//! # the same comparison for any document's page
//! cargo run --release -p render-raster --example heavy_drawing -- page /tmp/demo_heavy_engineering_A1.pdf 1
//! ```
//!
//! The knobs are named like `zoom_frame`'s: `HEAVY_COVERAGE` (`cpu`, `gpu` or `compute`, default
//! raster's own default), `HEAVY_SCALE` (the page's scale, default 1), and `HEAVY_BUDGET_MIB`,
//! which states `Options::max_frame_bytes` for the run — the one input this example exists to
//! vary, because the question it answers is which budget a heavy page meets first and by how much.
//! The oracle is drawn at `HEAVY_STRIPS` strips (default 4), stated rather than asked of the
//! machine, so its bytes are a fact about the page and not about the processor count (trap 134).
//! `HEAVY_DIGEST` names a file each drawn size appends its device bytes' FNV-1a digest to, so two
//! builds' or two budgets' draws can be held equal; `HEAVY_PNG` names a directory both rasters of
//! each drawn size are written to as pictures; `HEAVY_PHASES=1` prints the frame's phases;
//! `limits` prints what each adapter states about one buffer.
//!
//! **What a line of output says.** `drawn` with a byte comparison against the oracle — the number
//! of channel bytes that differ and the largest difference in levels — or `refused` with raster's
//! own sentence, which names the budget and the bytes asked. A size the device draws whose bytes
//! differ from the oracle's is *drawn*, and the comparison says by how much; whether that is
//! inside the lane's stated bound is the corpus gate's question, not this example's.

#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::arithmetic_side_effects,
    clippy::too_many_lines,
    missing_docs,
    reason = "a measurement tool: it stops loudly on a missing input, and its arithmetic is on \
              points of an A1 sheet and step counts far inside every type it uses"
)]

use std::fmt::Write as _;
use std::sync::Arc;

use pdf_render::{Rasterizer as _, TargetSpec};
use pdf_syntax::Document;

/// The sheet: ISO 216's A1, landscape, in points.
const SHEET: (f32, f32) = (2384.0, 1684.0);
/// Rack blocks across and down the drawing area.
const BLOCKS: (u32, u32) = (20, 15);
/// Cubicles across and down one block.
const CUBICLES: (u32, u32) = (4, 3);
/// Shading lines drawn down one cubicle, each as unit steps.
const SHADING_LINES: u32 = 8;
/// Strokes between two colour changes on a shading line — the witness's 1 288 210 `SC` over
/// 4 400 073 strokes is one change every 3.4.
const STEPS_PER_COLOUR: u32 = 4;
/// Hatching lines across one room.
const HATCHING: u32 = 120;

/// One drawing at one chop, as the bytes of a whole PDF file.
///
/// Four indirect objects and a cross-reference table, written so that every offset is the one
/// ISO 32000-2 §7.5.4 requires: the catalog, the page tree, the page and its content stream; the
/// font is §9.6.2.2's Helvetica, which a file may name without carrying, so the labels draw from
/// the program this tree compiles in rather than from anything on the machine.
fn drawing(steps_per_point: u32) -> Vec<u8> {
    let content = content_stream(steps_per_point);
    let mut file: Vec<u8> = b"%PDF-1.7\n%\xe2\xe3\xcf\xd3\n".to_vec();
    let mut offsets = Vec::new();
    let mut object = |file: &mut Vec<u8>, body: &[u8]| {
        offsets.push(file.len());
        let number = offsets.len();
        file.extend_from_slice(format!("{number} 0 obj\n").as_bytes());
        file.extend_from_slice(body);
        file.extend_from_slice(b"\nendobj\n");
    };
    object(&mut file, b"<< /Type /Catalog /Pages 2 0 R >>");
    object(&mut file, b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>");
    object(
        &mut file,
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {} {}] /Contents 4 0 R \
             /Resources << /Font << /F1 5 0 R >> >> >>",
            SHEET.0, SHEET.1
        )
        .as_bytes(),
    );
    let mut stream = format!("<< /Length {} >>\nstream\n", content.len()).into_bytes();
    stream.extend_from_slice(&content);
    stream.extend_from_slice(b"\nendstream");
    object(&mut file, &stream);
    object(
        &mut file,
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>",
    );
    let table = file.len();
    let mut xref = format!("xref\n0 {}\n0000000000 65535 f \n", offsets.len() + 1);
    for offset in &offsets {
        writeln!(xref, "{offset:010} 00000 n ").expect("a String takes every write");
    }
    write!(
        xref,
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{table}\n%%EOF\n",
        offsets.len() + 1
    )
    .expect("a String takes every write");
    file.extend_from_slice(xref.as_bytes());
    file
}

/// The page's one content stream.
fn content_stream(steps_per_point: u32) -> Vec<u8> {
    let mut out = String::new();
    let margin = 60.0;
    let (width, height) = (SHEET.0 - 2.0 * margin, SHEET.1 - 2.0 * margin);
    // The sheet's frame and the title block, as the witness opens: heavy `re S` rules.
    out.push_str("1 J 0.05 0.05 0.05 RG 10 w\n");
    let _ = writeln!(out, "{margin} {margin} {width} {height} re S");
    let _ = writeln!(
        out,
        "4 w {} {} 400 220 re S",
        SHEET.0 - margin - 400.0,
        margin
    );
    // Grid lines across the whole drawing, dashed as a draughtsman's grid is.
    out.push_str("0.85 0.2 0.2 RG 0.5 w [6 3] 0 d\n");
    for column in 0..=BLOCKS.0 {
        let x = margin + width * column as f32 / BLOCKS.0 as f32;
        let _ = writeln!(out, "{x:.2} {margin} m {x:.2} {:.2} l S", margin + height);
    }
    out.push_str("[] 0 d\n");
    let block = (width / BLOCKS.0 as f32, height / BLOCKS.1 as f32);
    let mut colour_step = 0u32;
    for row in 0..BLOCKS.1 {
        for column in 0..BLOCKS.0 {
            let origin = (
                margin + block.0 * column as f32,
                margin + block.1 * row as f32,
            );
            // One block in four, on a diagonal, is a room: filled and hatched at 45°.
            if (row + column) % 4 == 0 {
                room(&mut out, origin, block, row * BLOCKS.0 + column);
            } else {
                rack(
                    &mut out,
                    origin,
                    block,
                    steps_per_point,
                    row * BLOCKS.0 + column,
                    &mut colour_step,
                );
            }
        }
    }
    out.into_bytes()
}

/// A room: a filled rectangle, hatched at 45° by lines clipped to it in arithmetic, and named.
fn room(out: &mut String, origin: (f32, f32), block: (f32, f32), index: u32) {
    let inset = 6.0;
    let (x, y) = (origin.0 + inset, origin.1 + inset);
    let (w, h) = (block.0 - 2.0 * inset, block.1 - 2.0 * inset);
    let _ = writeln!(out, "0.82 0.9 0.98 rg {x:.2} {y:.2} {w:.2} {h:.2} re f");
    out.push_str("0.3 0.45 0.7 RG 0.3 w\n");
    // A family of lines x - y = offset, each cut to the rectangle's sides.
    for line in 0..HATCHING {
        let offset = -h + (w + h) * (line as f32 + 0.5) / HATCHING as f32;
        let start = if offset >= 0.0 {
            (offset, 0.0)
        } else {
            (0.0, -offset)
        };
        let end = if offset + h <= w {
            (offset + h, h)
        } else {
            (w, w - offset)
        };
        let _ = writeln!(
            out,
            "{:.2} {:.2} m {:.2} {:.2} l S",
            x + start.0,
            y + start.1,
            x + end.0,
            y + end.1
        );
    }
    let _ = writeln!(
        out,
        "0.05 0.05 0.05 RG 3 w {x:.2} {y:.2} {w:.2} {h:.2} re S"
    );
    label(
        out,
        (x + w * 0.4, y + h * 0.5),
        6.0,
        &format!("ER-{index:03}"),
    );
}

/// A rack block: cubicles filled and framed, each shaded by lines stated as unit steps.
fn rack(
    out: &mut String,
    origin: (f32, f32),
    block: (f32, f32),
    steps_per_point: u32,
    index: u32,
    colour_step: &mut u32,
) {
    let inset = 8.0;
    let cell = (
        (block.0 - 2.0 * inset) / CUBICLES.0 as f32,
        (block.1 - 2.0 * inset - 6.0) / CUBICLES.1 as f32,
    );
    label(
        out,
        (origin.0 + inset, origin.1 + block.1 - inset),
        3.0,
        &format!("DEMO-GEN-{index:03}"),
    );
    let unit = 1.0 / steps_per_point as f32;
    for down in 0..CUBICLES.1 {
        for across in 0..CUBICLES.0 {
            let x = origin.0 + inset + cell.0 * across as f32;
            let y = origin.1 + inset + cell.1 * down as f32;
            let _ = writeln!(
                out,
                "0.72 0.75 0.8 rg {:.2} {:.2} {:.2} {:.2} re f 0.1 0.1 0.12 RG 1 w {:.2} {:.2} {:.2} {:.2} re S",
                x + 1.0,
                y + 1.0,
                cell.0 - 2.0,
                cell.1 - 2.0,
                x + 1.0,
                y + 1.0,
                cell.0 - 2.0,
                cell.1 - 2.0
            );
            // The shading lines, in the producer's own unit space: integer coordinates in a
            // `1/steps_per_point` scale, one stroke a step, as the witness writes them. The
            // width is set outside the `q`, so each step draws at 1.5 of those units.
            let (first, last) = (
                ((y + 2.0) * steps_per_point as f32) as u32,
                ((y + cell.1 - 2.0) * steps_per_point as f32) as u32,
            );
            out.push_str("1.5 w\n");
            for line in 0..SHADING_LINES {
                let at = x + 2.0 + (cell.0 - 4.0) * (line as f32 + 0.5) / SHADING_LINES as f32;
                let column = (at * steps_per_point as f32) as u32;
                for step in first..last {
                    if step % STEPS_PER_COLOUR == 0 {
                        *colour_step = colour_step.wrapping_add(1);
                        let shade = (*colour_step % 64) as f32 / 640.0;
                        let _ = writeln!(
                            out,
                            "{:.6} {:.6} {:.6} RG",
                            0.41 + shade,
                            0.47 + shade,
                            0.57 + shade
                        );
                    }
                    let _ = writeln!(
                        out,
                        "q {unit} 0 0 {unit} 0 0 cm {column} {step} m {column} {} l S Q",
                        step + 1
                    );
                }
            }
            label(
                out,
                (x + 1.5, y + 1.5),
                1.6,
                &format!("PNL-{index}.{down}{across}"),
            );
        }
    }
}

/// One text label in §9.6.2.2's Helvetica.
fn label(out: &mut String, at: (f32, f32), size: f32, text: &str) {
    let _ = writeln!(
        out,
        "0 g BT /F1 {size} Tf {:.2} {:.2} Td ({text}) Tj ET",
        at.0, at.1
    );
}

/// Whether `HEAVY_PHASES` asks for the device frame's own phases under each line.
fn backend_phases_wanted() -> bool {
    variable("HEAVY_PHASES").is_some_and(|on| on != "0")
}

fn variable(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|value| !value.is_empty())
}

/// The device backend this run asks: the lane and the budget the environment states.
fn device() -> render_raster::QuorraRasterizer {
    let mut options = render_raster::options();
    if let Some(mib) = variable("HEAVY_BUDGET_MIB") {
        let mib: u64 = mib.parse().expect("HEAVY_BUDGET_MIB in mebibytes");
        options.max_frame_bytes = mib.saturating_mul(1 << 20);
    }
    let mut backend = render_raster::QuorraRasterizer::with_options(&options).expect("an adapter");
    backend.set_coverage(match variable("HEAVY_COVERAGE").as_deref() {
        None | Some("cpu") => raster_gpu::Coverage::Cpu,
        Some("gpu") => raster_gpu::Coverage::Gpu,
        Some("compute") => raster_gpu::Coverage::Compute,
        Some(other) => panic!("HEAVY_COVERAGE={other}: cpu, gpu or compute"),
    });
    backend
}

/// One page drawn on both backends, and one line saying what came of it.
fn compare(
    name: &str,
    bytes: Vec<u8>,
    page_index: usize,
    backend: &mut render_raster::QuorraRasterizer,
) {
    let file_bytes = bytes.len();
    let document = Document::open(bytes).expect("the drawing opens");
    let pages = pdf_model::Pages::new(&document);
    let page = pages.get(page_index).expect("that page exists");
    let interpreting = std::time::Instant::now();
    let list = Arc::new(pdf_model::content::interpret(&document, &page).display_list);
    let interpret_ms = interpreting.elapsed().as_secs_f64() * 1e3;
    let scale: f32 = variable("HEAVY_SCALE").map_or(1.0, |s| s.parse().expect("a scale"));
    let target = TargetSpec::for_page(&list, scale, 1 << 30).expect("a target");
    let strips: u32 = variable("HEAVY_STRIPS").map_or(4, |s| s.parse().expect("a strip count"));

    let drawing = std::time::Instant::now();
    let oracle = render_cpu::CpuRasterizer::new()
        .with_strips(strips)
        .rasterize(&list, target)
        .expect("the oracle draws every page this example writes");
    let oracle_ms = drawing.elapsed().as_secs_f64() * 1e3;

    let drawing = std::time::Instant::now();
    let device = backend.rasterize(&list, target);
    let device_ms = drawing.elapsed().as_secs_f64() * 1e3;
    let cost = backend.last_frame();
    println!(
        "{name}: {file_bytes} file bytes, {} list commands, {}x{} px, interpreted in {interpret_ms:.0} ms, \
         oracle {oracle_ms:.0} ms at {strips} strips",
        list.commands().len(),
        target.width,
        target.height
    );
    match device {
        Ok(raster) => {
            let (mut differ, mut most) = (0usize, 0u8);
            for (a, b) in raster.data.iter().zip(&oracle.data) {
                if a != b {
                    differ = differ.saturating_add(1);
                    most = most.max(a.abs_diff(*b));
                }
            }
            let verdict = raster_compare::compare(&oracle, &raster).map_or_else(
                |error| format!("not comparable: {error}"),
                |c| {
                    format!(
                        "mean {:.4}, worst tile {:.2}, ssim {:.5}",
                        c.mean_error, c.worst_tile_error, c.structural_similarity
                    )
                },
            );
            println!(
                "    drawn in {device_ms:.0} ms: {} commands encoded, {} culled, {} bytes uploaded; \
                 {differ} of {} bytes differ from the oracle, by up to {most} levels; {verdict}",
                cost.commands,
                cost.commands_culled,
                cost.bytes_uploaded,
                oracle.data.len()
            );
            if let Some(directory) = variable("HEAVY_PNG") {
                // Both rasters, to be looked at: a count that agrees is not a picture (trap 1).
                for (what, drawn) in [("oracle", &oracle), ("device", &raster)] {
                    let file = std::fs::File::create(format!(
                        "{directory}/{}-{what}.png",
                        name.rsplit('/').next().unwrap_or(name).replace(' ', "-")
                    ))
                    .expect("the picture's file opens");
                    let mut encoder =
                        png::Encoder::new(std::io::BufWriter::new(file), drawn.width, drawn.height);
                    encoder.set_color(png::ColorType::Rgba);
                    encoder.set_depth(png::BitDepth::Eight);
                    encoder
                        .write_header()
                        .and_then(|mut writer| writer.write_image_data(&drawn.data))
                        .expect("the picture writes");
                }
            }
            if let Some(path) = variable("HEAVY_DIGEST") {
                // The device's own bytes, folded, so two budgets' draws of one size can be held
                // equal without keeping either raster.
                let digest = raster
                    .data
                    .iter()
                    .fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
                        (hash ^ u64::from(*byte)).wrapping_mul(0x0100_0000_01b3)
                    });
                let line = format!("{name}\t{digest:016x}\n");
                let mut file = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(path)
                    .expect("the digest file opens");
                std::io::Write::write_all(&mut file, line.as_bytes()).expect("the digest writes");
            }
        }
        Err(error) => println!("    refused after {device_ms:.0} ms: {error}"),
    }
    if backend_phases_wanted() {
        let ms = |span: std::time::Duration| span.as_secs_f64() * 1e3;
        println!(
            "    scene {:.0} ms, encode {:.0} ms, upload {:.0} ms, execute {:.0} ms, readback {:.0} ms",
            ms(cost.scene),
            ms(cost.encode),
            ms(cost.upload),
            ms(cost.execute),
            ms(cost.readback)
        );
    }
}

fn main() {
    let mut arguments = std::env::args().skip(1);
    match arguments.next().as_deref() {
        Some("write") => {
            let chop: u32 = arguments
                .next()
                .expect("a chop")
                .parse()
                .expect("a chop as a whole number of steps per point");
            let path = arguments.next().expect("a path to write");
            std::fs::write(&path, drawing(chop)).expect("the file writes");
        }
        Some("ladder") => {
            let chops: Vec<u32> = arguments
                .next()
                .unwrap_or_else(|| "1,2,4,8".to_owned())
                .split(',')
                .map(|chop| chop.trim().parse().expect("a chop"))
                .collect();
            let mut backend = device();
            println!("{}", backend.adapter_description());
            for chop in chops {
                compare(&format!("chop {chop}"), drawing(chop), 0, &mut backend);
            }
        }
        Some("page") => {
            let path = arguments.next().expect("a document");
            let index: usize = arguments
                .next()
                .map_or(1, |n| n.parse().expect("a page number"));
            let mut backend = device();
            println!("{}", backend.adapter_description());
            let bytes = std::fs::read(&path).expect("the document is readable");
            compare(&path, bytes, index.saturating_sub(1), &mut backend);
        }
        Some("limits") => {
            // What each adapter states about one buffer: the ceiling any frame budget meets
            // whatever number is stated for it, because a stream of instances is one buffer.
            let instance = raster_gpu::wgpu::Instance::default();
            for adapter in
                pollster::block_on(instance.enumerate_adapters(raster_gpu::wgpu::Backends::PRIMARY))
            {
                let limits = adapter.limits();
                println!(
                    "{}: max_buffer_size {} bytes, max_storage_buffer_binding_size {} bytes, \
                     max_texture_dimension_2d {}",
                    adapter.get_info().name,
                    limits.max_buffer_size,
                    limits.max_storage_buffer_binding_size,
                    limits.max_texture_dimension_2d
                );
            }
        }
        _ => {
            panic!("heavy_drawing write <chop> <path> | ladder <chops> | page <path> [n] | limits")
        }
    }
}
