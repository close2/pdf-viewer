//! A tiny document that costs an unbounded amount to draw.
//!
//! Shared by `tests/confined.rs`, which cancels one, and `examples/confined_cancel`, which shows
//! a person the same thing with the numbers printed. It lives under `tests/` in a *subdirectory*,
//! which Cargo does not compile as a test target of its own, and both consumers reach it with
//! `#[path]` — one file rather than the same forty lines written twice.
//!
//! # What makes it hostile
//!
//! §8.10.1's form XObject may draw another form XObject. `pdf_model`'s `MAX_FORM_DEPTH` bounds
//! how *deeply* that may nest, because depth is where a cycle lives; nothing bounds the
//! **breadth**, and nothing sensibly could — a page legitimately draws a form a thousand times.
//! So each level here draws [`BRANCH`] copies of the level below it and the bottom one fills the
//! whole page, which turns a linear file into an exponential page: four levels branching ten ways
//! is ten thousand page-covering fills out of well under two kilobytes.
//!
//! This is written out rather than committed as a fixture on purpose. The point is not that some
//! particular file is slow; it is that a producer can write one in a few hundred bytes, which is
//! why a *deadline* is not the answer and a cancel is. ADR 0241.

#![allow(
    dead_code,
    reason = "two consumers include this file by path and neither of them uses all of it"
)]

/// How deep the forms nest by default. `MAX_FORM_DEPTH` is 64 (ADR 0793), so this is well
/// inside it.
///
/// **Five rather than four since ADR 0640, and the extra level is not decoration.** A confined
/// worker draws a page only where its *pixels* are the payload that crosses; a page whose marks
/// are smaller is shipped undrawn, so its rasterisation is never work the worker does and never
/// work a cancel has anything to stop. Four levels is ten thousand page-covering fills in 990 KB
/// of marks, which is under a window-sized raster and therefore the marks arm. Five is a hundred
/// thousand and 9.9 MB — larger than a raster of any window — so the page crosses as pixels, the
/// worker draws it, and there is something for a cancel to be about.
///
/// Both consumers check that premise rather than assuming it: `tests/confined.rs` asks
/// `wire::crossing` which arm the page takes before it blocks on one.
pub(crate) const LEVELS: usize = 5;

/// How many copies of the level below each level draws.
pub(crate) const BRANCH: usize = 10;

/// How many page-covering fills `levels` levels branching [`BRANCH`] ways amount to.
pub(crate) fn fills(levels: usize) -> u64 {
    u64::try_from(BRANCH)
        .unwrap_or(1)
        .saturating_pow(u32::try_from(levels).unwrap_or(u32::MAX))
}

/// The document, whole.
///
/// Object 1 is the catalog, 2 the page tree, 3 the page, 4 its contents, and 5 onwards the forms
/// from the bottom level upwards.
pub(crate) fn document(levels: usize, branch: usize) -> Vec<u8> {
    /// Object number of the form for level `k`.
    fn form(k: usize) -> usize {
        5usize.saturating_add(k)
    }

    let mut objects: Vec<Vec<u8>> = Vec::new();
    objects.push(b"<< /Type /Catalog /Pages 2 0 R >>".to_vec());
    objects.push(b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec());
    objects.push(
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] \
             /Resources << /XObject << /F{levels} {} 0 R >> >> /Contents 4 0 R >>",
            form(levels)
        )
        .into_bytes(),
    );

    let mut stream = |body: &str, dictionary: &str| {
        objects.push(
            format!(
                "<< {dictionary} /Length {} >>\nstream\n{body}\nendstream",
                body.len()
            )
            .into_bytes(),
        );
    };
    stream(&format!("q /F{levels} Do Q"), "");
    for level in 0..=levels {
        if level == 0 {
            stream(
                "0.5 0.2 0.1 rg 0 0 612 792 re f",
                "/Type /XObject /Subtype /Form /BBox [0 0 612 792] /Resources << >>",
            );
        } else {
            let below = level.saturating_sub(1);
            stream(
                &format!("/F{below} Do ").repeat(branch),
                &format!(
                    "/Type /XObject /Subtype /Form /BBox [0 0 612 792] \
                     /Resources << /XObject << /F{below} {} 0 R >> >>",
                    form(below)
                ),
            );
        }
    }

    let mut bytes: Vec<u8> = b"%PDF-1.7\n".to_vec();
    let mut offsets: Vec<usize> = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(bytes.len());
        bytes.extend_from_slice(format!("{} 0 obj\n", index.saturating_add(1)).as_bytes());
        bytes.extend_from_slice(object);
        bytes.extend_from_slice(b"\nendobj\n");
    }
    let table = bytes.len();
    let size = objects.len().saturating_add(1);
    bytes.extend_from_slice(format!("xref\n0 {size}\n0000000000 65535 f \n").as_bytes());
    for offset in &offsets {
        bytes.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    bytes.extend_from_slice(
        format!("trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{table}\n%%EOF\n").as_bytes(),
    );
    bytes
}
