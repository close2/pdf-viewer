//! How much of a first page's interpretation is image decoding, and how much of that a parallel
//! decode could take off the critical path.
//!
//! `doc/todo/36`'s second item and ADR 1272 section 5 propose decoding a page's image `XObject`s
//! in parallel with each other. What that can buy on a page is bounded by two sums this example
//! measures: `decode`, the time the page's images take to decode one after another, and
//! `longest`, the one that takes longest — so `decode - longest` is what running the rest beside
//! it could save with cores to spare, and `interp - decode` is the other work a decode could
//! overlap.
//!
//! ```sh
//! cargo run --release -p pdf-model --example image_decode_census -- doc/pdf.js/test/pdfs
//! ```
//!
//! # What is timed, and the approximation in it
//!
//! `interp` is `pdf_model::interpret` of page one on a freshly opened document, so it pays the
//! page's font loading as a first view does. The images are then decoded on a *second* freshly
//! opened document, so that `Document`'s memo of §7.4's filter chains holds nothing either
//! interpretation left in it. The population is every image `XObject` the page's resource
//! dictionary names, directly or through a form `XObject`'s own resources — which may include an
//! image the content never draws — decoded under `Conversion::device()` and a black fill rather
//! than under the state the `Do` would meet. Both approximations make `decode` an upper bound on
//! what the interpretation spent decoding, which is the direction that cannot flatter the case
//! for a parallel decode. `drawn` is the number of image commands the display list holds, a
//! repeat drawn from `RasterCache` counting again.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]
#![allow(clippy::print_stdout, clippy::print_stderr)]
#![allow(
    clippy::arithmetic_side_effects,
    clippy::cast_precision_loss,
    clippy::too_many_lines,
    reason = "counters and milliseconds over a corpus's first pages, measured and summarised in \
              one pass; a measurement rather than a shipped path"
)]

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use pdf_model::colour::Conversion;
use pdf_syntax::{Dictionary, Document, Object, Stream};

/// Every `.pdf` under the directories named on the command line.
fn corpus(args: &[String]) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for arg in args {
        let root = Path::new(arg);
        if root.is_file() {
            files.push(root.to_path_buf());
            continue;
        }
        let Ok(entries) = std::fs::read_dir(root) else {
            eprintln!("cannot read {}", root.display());
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "pdf") {
                files.push(path);
            }
        }
    }
    files.sort();
    files
}

/// The image `XObject`s a resource dictionary names, with the dictionary each is named under,
/// through form `XObject`s to a bounded depth.
fn images(
    document: &Document,
    resources: &Dictionary,
    depth: usize,
    seen: &mut HashSet<usize>,
    out: &mut Vec<(Arc<Stream>, Dictionary)>,
) {
    if depth > 8 {
        return;
    }
    let xobjects = document.get_key(resources, "XObject");
    let Some(xobjects) = xobjects.as_dict() else {
        return;
    };
    for (_, value) in xobjects.iter() {
        let resolved = document.resolve(value);
        let Some(stream) = resolved.as_stream() else {
            continue;
        };
        if !seen.insert(Arc::as_ptr(stream) as usize) {
            continue;
        }
        match document.get_key(&stream.dict, "Subtype") {
            Object::Name(n) if n.as_bytes() == b"Image" => {
                out.push((Arc::clone(stream), resources.clone()));
            }
            Object::Name(n) if n.as_bytes() == b"Form" => {
                let inner = document.get_key(&stream.dict, "Resources");
                if let Some(inner) = inner.as_dict() {
                    images(document, inner, depth + 1, seen, out);
                }
            }
            _ => {}
        }
    }
}

/// The last entry of `/Filter`, which is the codec where there is one.
fn codec(document: &Document, stream: &Stream) -> String {
    let name = match document.get_key(&stream.dict, "Filter") {
        Object::Name(name) => Some(name),
        Object::Array(items) => items.last().and_then(|item| match document.resolve(item) {
            Object::Name(name) => Some(name),
            _ => None,
        }),
        _ => None,
    };
    name.map_or_else(
        || "none".to_owned(),
        |n| String::from_utf8_lossy(n.as_bytes()).into_owned(),
    )
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args = if args.is_empty() {
        vec![
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../doc/pdf.js/test/pdfs")
                .to_string_lossy()
                .into_owned(),
        ]
    } else {
        args
    };
    // (name, images, drawn, interp ms, decode ms, longest ms, longest codec)
    let mut rows: Vec<(String, usize, usize, f64, f64, f64, String)> = Vec::new();
    for path in corpus(&args) {
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        let Ok(first) = Document::open(bytes.clone()) else {
            continue;
        };
        let Some(page) = pdf_model::Pages::new(&first).get(0) else {
            continue;
        };
        let started = Instant::now();
        let interpretation = pdf_model::interpret(&first, &page);
        let interp = started.elapsed().as_secs_f64() * 1e3;
        let drawn = interpretation
            .display_list
            .commands()
            .iter()
            .filter(|command| matches!(command, pdf_render::Command::Image { .. }))
            .count();
        drop(interpretation);
        drop(first);

        let Ok(second) = Document::open(bytes) else {
            continue;
        };
        let Some(page) = pdf_model::Pages::new(&second).get(0) else {
            continue;
        };
        let mut found = Vec::new();
        images(&second, &page.resources, 0, &mut HashSet::new(), &mut found);
        let mut decode = 0.0;
        let mut longest = (0.0, String::new());
        for (stream, resources) in &found {
            let mut masks = pdf_model::image::MaskCache::default();
            let started = Instant::now();
            let _answer = pdf_model::image::decode_parts(
                &second,
                stream,
                resources,
                pdf_render::Color::BLACK,
                &Conversion::device(),
                &mut masks,
            );
            let spent = started.elapsed().as_secs_f64() * 1e3;
            decode += spent;
            if spent > longest.0 {
                longest = (spent, codec(&second, stream));
            }
        }
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        println!(
            "{name}\t{}\t{drawn}\t{interp:.3}\t{decode:.3}\t{:.3}\t{}",
            found.len(),
            longest.0,
            longest.1
        );
        rows.push((
            name,
            found.len(),
            drawn,
            interp,
            decode,
            longest.0,
            longest.1,
        ));
    }

    eprintln!("{} first pages", rows.len());
    let buckets: [(usize, usize, &str); 6] = [
        (0, 0, "0"),
        (1, 1, "1"),
        (2, 2, "2"),
        (3, 5, "3-5"),
        (6, 10, "6-10"),
        (11, usize::MAX, "11+"),
    ];
    for (low, high, label) in buckets {
        let here: Vec<_> = rows
            .iter()
            .filter(|row| row.1 >= low && row.1 <= high)
            .collect();
        let decode: f64 = here.iter().map(|row| row.4).sum();
        let interp: f64 = here.iter().map(|row| row.3).sum();
        eprintln!(
            "  {label:>4} images: {:>4} pages, interp {interp:.1} ms, decode {decode:.1} ms",
            here.len()
        );
    }
    let divisible: Vec<_> = rows
        .iter()
        .filter(|row| row.1 >= 2 && row.4 - row.5 >= 1.0)
        .collect();
    eprintln!(
        "{} pages where decode - longest is at least 1 ms",
        divisible.len()
    );
    let mut by_saving: Vec<_> = rows.iter().collect();
    by_saving.sort_by(|a, b| (b.4 - b.5).total_cmp(&(a.4 - a.5)));
    eprintln!("largest decode - longest (name, images, drawn, interp, decode, longest, codec):");
    for row in by_saving.iter().take(20) {
        eprintln!(
            "  {} {} {} {:.2} {:.2} {:.2} {}",
            row.0, row.1, row.2, row.3, row.4, row.5, row.6
        );
    }
    let mut by_decode: Vec<_> = rows.iter().collect();
    by_decode.sort_by(|a, b| b.4.total_cmp(&a.4));
    eprintln!("largest decode:");
    for row in by_decode.iter().take(10) {
        eprintln!(
            "  {} {} {} {:.2} {:.2} {:.2} {}",
            row.0, row.1, row.2, row.3, row.4, row.5, row.6
        );
    }
}
