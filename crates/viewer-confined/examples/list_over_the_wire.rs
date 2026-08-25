//! What a display list *actually* costs to send, and what the round trip costs in time.
//!
//! `examples/list_against_raster` walked a list and summed what an encoder **must** write; ADR
//! 0607 decided on those figures. This runs the encoder, so the same column is a measurement
//! rather than a prediction — which is the point: **a price is a claim that decays**, and the
//! honest way to keep one is to re-derive it from the thing that now exists.
//!
//! Three columns the prediction could not have:
//!
//! - **what the format really writes**, tables, tags, padding and all;
//! - **which payload the page crosses as**, by ADR 0607's own per-page rule, with the refusal
//!   named where it is one of the two deferred producers;
//! - **what the codec costs in time**, both ways, beside the transport's own measured rate — and
//!   since the seven-hundred-and-thirty-sixth session, the *choice* timed separately from the
//!   exact encoding, because the frame path makes the choice and never pays for the rest.
//!
//! Every list is decoded and compared against the one that was encoded, so a run that prints a
//! number has also checked that the number is of a faithful round trip.
//!
//! ```sh
//! cargo run --release -p viewer-confined --example list_over_the_wire -- \
//!     [--scale N] [--seeds DIR] <file.pdf>…
//! ```
//!
//! `--scale` defaults to 1.0, which is 72 dpi and smaller than any window asks for: a display
//! list is scale-invariant and a raster is quadratic in the scale, so the default column is the
//! list's **worst** case.
//!
//! `--seeds` writes each encoded list into a directory, which is how
//! `fuzz/fuzz_targets/display_list.rs` gets a corpus of real pages rather than of noise — this
//! example is that target's seeder, in Rust rather than in Python, because producing a display
//! list means running the interpreter. `--seed-max` bounds what is written, at [`SEED_MAX`] by
//! default and for the reason `doc/todo/02` records of the `page` target: libFuzzer merges its
//! corpus once per run at one execution per seed, and a scanned page's list is tens of megabytes
//! of samples that state nothing about this *format*. The whole pdf.js corpus unbounded is 841
//! MB of seeds and almost all of it is four documents' pixels.
#![expect(
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::arithmetic_side_effects,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::too_many_lines,
    reason = "an example whose entire output is a measurement; its counters are bounded by one \
              corpus, the ratios it prints are printed to four decimals, and its one f64-to-index \
              cast is a percentile position clamped into the sorted vector it indexes"
)]

use std::time::Instant;

/// The largest seed `--seeds` writes unless `--seed-max` says otherwise, in bytes.
///
/// The same ceiling the `page` target uses. A seed past it is a scanned page, whose bytes are
/// image samples: it exercises one branch of this format and pays for itself in every merge.
const SEED_MAX: usize = 256 << 10;

use pdf_render::TargetSpec;
use viewer_confined::{Crossing, RasterReason, wire};

fn main() {
    let mut scale = 1.0_f32;
    let mut seeds: Option<String> = None;
    let mut seed_max = SEED_MAX;
    let mut paths = Vec::new();
    let mut args = std::env::args().skip(1);
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--scale" => {
                scale = args
                    .next()
                    .and_then(|value| value.parse().ok())
                    .unwrap_or(1.0);
            }
            "--seeds" => seeds = args.next(),
            "--seed-max" => {
                seed_max = args
                    .next()
                    .and_then(|value| value.parse().ok())
                    .unwrap_or(SEED_MAX);
            }
            _ => paths.push(argument),
        }
    }
    if let Some(directory) = &seeds
        && let Err(error) = std::fs::create_dir_all(directory)
    {
        eprintln!("{directory}: {error}");
        return;
    }

    println!(
        "# name\tmarks\tlist_B\traster_B\tlist/raster\tcrossing_ms\tencode_ms\tdecode_ms\t\
         crossing"
    );
    let mut ratios: Vec<f64> = Vec::new();
    let mut list_total = 0_u64;
    let mut raster_total = 0_u64;
    let mut as_pixels = 0_usize;
    let mut deferred = 0_usize;
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
        let Ok(target) = TargetSpec::for_page(&list, scale, 1 << 28) else {
            continue;
        };
        let raster = u64::from(target.width) * u64::from(target.height) * 4;
        if raster == 0 {
            continue;
        }
        pages += 1;
        raster_total += raster;

        // **The choice as the confined process makes it**, timed first and separately from the
        // exact encoding below. `crossing` hands the encoder the raster's own size and the
        // encoder stops when it passes it, so on a page that crosses as pixels this is the
        // column that says what the *frame path* pays — where `encode_ms` beside it is what
        // finishing would have cost, which is the number that made the stop worth having.
        let started = Instant::now();
        let chosen = wire::crossing(&list, raster);
        let crossing_ms = started.elapsed().as_secs_f64() * 1000.0;

        let started = Instant::now();
        let encoded = wire::encode_display_list(&list);
        let encode_ms = started.elapsed().as_secs_f64() * 1000.0;

        let encoded = match encoded {
            Ok(encoded) => encoded,
            Err(refusal) => {
                deferred += 1;
                as_pixels += 1;
                println!(
                    "{name}\t-\t-\t{raster}\t-\t{crossing_ms:.3}\t{encode_ms:.3}\t-\t\
                     raster: {refusal}"
                );
                continue;
            }
        };

        let started = Instant::now();
        let back = wire::display_list(&encoded);
        let decode_ms = started.elapsed().as_secs_f64() * 1000.0;
        match back {
            Ok(back) if back == list => {}
            Ok(_) => {
                eprintln!("{name}: the decoded list is not the one that was encoded");
                continue;
            }
            Err(error) => {
                eprintln!("{name}: this build could not read back what it wrote: {error}");
                continue;
            }
        }

        if let Some(directory) = &seeds
            && encoded.len() <= seed_max
            && let Err(error) =
                std::fs::write(std::path::Path::new(directory).join(&name), &encoded)
        {
            eprintln!("{name}: {error}");
        }

        let ratio = encoded.len() as f64 / raster as f64;
        ratios.push(ratio);
        list_total += encoded.len() as u64;
        let crossing = match chosen {
            Crossing::List(_) => "list".to_owned(),
            Crossing::Raster(RasterReason::Larger { .. }) => {
                as_pixels += 1;
                "raster: larger than its pixels".to_owned()
            }
            Crossing::Raster(reason) => {
                as_pixels += 1;
                format!("raster: {reason}")
            }
        };
        println!(
            "{name}\t{}\t{}\t{raster}\t{ratio:.4}\t{crossing_ms:.3}\t{encode_ms:.3}\t\
             {decode_ms:.3}\t{crossing}",
            list.commands().len(),
            encoded.len(),
        );
    }

    ratios.sort_by(f64::total_cmp);
    let at = |fraction: f64| {
        ratios
            .get(((ratios.len() as f64 - 1.0) * fraction).round().max(0.0) as usize)
            .copied()
            .unwrap_or(f64::NAN)
    };
    println!(
        "# {pages} page(s) at scale {scale}: list {list_total} B, raster {raster_total} B, \
         aggregate {:.4}",
        list_total as f64 / raster_total.max(1) as f64
    );
    println!(
        "# list/raster median {:.4}, p90 {:.4}, p99 {:.4}, worst {:.4}",
        at(0.5),
        at(0.9),
        at(0.99),
        at(1.0),
    );
    println!(
        "# {as_pixels} page(s) cross as pixels, {deferred} of them for a producer this format \
         does not carry"
    );
}
