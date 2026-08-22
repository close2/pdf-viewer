//! Which composition multiplies a rectangle's edge coverage by itself, and which does not.
//!
//! ISO 32000-2 §10.7.4 gives clipping a paragraph of its own, and it is about sets rather than
//! about coverage:
//!
//! > For clipping, the clipping region consists of the set of pixels that would be included by a
//! > fill operation. Subsequent painting operations shall affect a region that is the intersection
//! > of the set of pixels defined by the clipping region with the set of pixels for the region to
//! > be painted.
//!
//! `doc/todo/11` item 4 is the debt that sentence names: where a document states one rectangle
//! twice — a clip and a fill on the same coordinates — composing the two coverages as a *product*
//! paints the boundary pixel at the square of its own coverage, and an intersection of a set with
//! itself is that set. Two of the tree's compositions have been taken (a clip *chain* by ADR 0280
//! and a fill's own coverage by ADR 0355) and the item's remaining bullets are prose about where
//! the rest are.
//!
//! This turns that prose into a number. Each rung is one 40 × 40 page holding one black fill of
//! `[10 10 30 30.504]`, so a single device row holds the shape's lower edge at coverage 0.504, and
//! the rungs differ only in **how the same rectangle is stated a second time**: not at all, as a
//! `W n` clip, as a form `XObject`'s `/BBox`, as a transparency group's, and each of those again with
//! a luminosity soft mask in force whose `/BC` is white — a mask worth 1.0 at every pixel of the
//! page, which cannot change what any pixel should be and does change which composition the mark
//! goes through.
//!
//! Every rung should read the same coverage, because every rung states the same geometry. The one
//! that does not is the composition still owed, and it is the eighth: §11.4.4's NOTE 5 flattens a
//! group away unless a soft mask is in force, so a group's raster only *reaches* the blit that
//! multiplies when there is a mask beside it. That is why the residual had no small witness for
//! nineteen sessions and why `issue7891_bc1.pdf` page 1 is one now.
//!
//! `cargo run -p pdf-model --example coincident_edge_probe`

#![expect(
    clippy::print_stdout,
    clippy::expect_used,
    reason = "a diagnostic binary: the ladder it prints is the point, and a document it built \
              itself failing to open should stop it loudly rather than be handled"
)]

use pdf_render::{Rasterizer, TargetSpec};

/// The page is square and small enough that the whole raster is one glance.
const PAGE: f32 = 40.0;

/// The rectangle every rung states, in PDF units. Its lower edge falls inside a device row.
const RECT: &str = "10 10 20 20.504 re";

/// The device row holding the rectangle's lower edge, and the column read out of it.
///
/// The `re` above spans PDF y `[10, 30.504]`, so the upper edge is at device y `40 − 30.504`
/// and the row containing it is 9, covered from 9.496 to 10 — 0.504 of a pixel.
const ROW: usize = 9;
/// A column well inside the rectangle, where only the row's own edge is in question.
const COLUMN: usize = 20;
/// What the shape covers of [`ROW`], from the file's own numbers.
const COVERAGE: f64 = 0.504;

/// How the rectangle is stated a second time.
#[derive(Clone, Copy)]
enum Restatement {
    /// Not at all: the fill alone.
    Alone,
    /// As `W n` on the same coordinates.
    Clip,
    /// As a form `XObject`'s `/BBox` (§8.10.1 step c).
    FormBox,
    /// As a form `XObject`'s `/BBox` where the form also carries `/Group` (§11.4.1).
    GroupBox,
}

impl Restatement {
    /// The name printed in the ladder's first column.
    fn label(self) -> &'static str {
        match self {
            Self::Alone => "fill alone",
            Self::Clip => "W n clip",
            Self::FormBox => "form /BBox",
            Self::GroupBox => "group /BBox",
        }
    }
}

fn main() {
    println!("one rectangle whose lower edge covers {COVERAGE:.3} of device row {ROW}");
    println!("stated twice each way, read at column {COLUMN}\n");
    println!(
        "  {:<14}{:>14}{:>14}",
        "restated as", "no soft mask", "soft mask"
    );
    for restatement in [
        Restatement::Alone,
        Restatement::Clip,
        Restatement::FormBox,
        Restatement::GroupBox,
    ] {
        let bare = coverage(&document(restatement, false));
        let masked = coverage(&document(restatement, true));
        println!("  {:<14}{bare:>14.4}{masked:>14.4}", restatement.label());
    }
    println!(
        "\n  the shape's own coverage {COVERAGE:.4}, its square {:.4}",
        COVERAGE * COVERAGE
    );
}

/// Rasterises the page at one device pixel per PDF unit and returns what the boundary row got.
///
/// The fill is black on white, so one minus the red channel is the coverage the backend painted.
fn coverage(bytes: &[u8]) -> f64 {
    let document = pdf_syntax::Document::open(bytes.to_vec()).expect("a PDF this file just built");
    let pages = pdf_model::Pages::new(&document);
    let page = pages.get(0).expect("its only page");
    let view = pdf_model::view::ViewState::of(&document);
    let list = pdf_model::content::interpret_with(&document, &page, &view).display_list;
    let target = TargetSpec::for_page(&list, 1.0, 1 << 24).expect("a 40 by 40 target");
    let raster = render_cpu::CpuRasterizer::new()
        .rasterize(&list, target)
        .expect("a rasterised page");
    let width = raster.width as usize;
    let index = ROW
        .saturating_mul(width)
        .saturating_add(COLUMN)
        .saturating_mul(4);
    let red = raster.data[index];
    1.0 - f64::from(red) / 255.0
}

/// Assembles one probe document.
///
/// Written out by hand rather than through a fixture builder because every byte of it is part of
/// what is being measured: the rectangle's coordinates appear in the content stream, in the clip
/// and in the `/BBox`, and a helper that shared them between the three would hide the very
/// coincidence the ladder is about.
fn document(restatement: Restatement, soft_mask: bool) -> Vec<u8> {
    let fill = format!("0 0 0 rg\n{RECT}\nf\n");
    let gs = if soft_mask { "/GS gs\n" } else { "" };
    let (content, resources, form) = match restatement {
        Restatement::Alone => (format!("{gs}{fill}"), String::new(), None),
        Restatement::Clip => (
            format!("{gs}q\n{RECT}\nW n\n{fill}Q\n"),
            String::new(),
            None,
        ),
        Restatement::FormBox | Restatement::GroupBox => {
            let group = if matches!(restatement, Restatement::GroupBox) {
                " /Group << /S /Transparency /CS /DeviceRGB >>"
            } else {
                ""
            };
            (
                format!("{gs}/Fm Do\n"),
                "/XObject << /Fm 5 0 R >>".to_owned(),
                Some(stream(
                    &format!(
                        "<< /Type /XObject /Subtype /Form /BBox [10 10 30 30.504] \
                         /Resources << >>{group}"
                    ),
                    &fill,
                )),
            )
        }
    };

    // §11.6.5.2's luminosity mask over a group that is white everywhere, with a white `/BC`, so
    // its value is 1.0 at every pixel and it is the *route* rather than the picture that changes.
    let mask_group = stream(
        "<< /Type /XObject /Subtype /Form /BBox [0 0 40 40] /Resources << >> \
         /Group << /S /Transparency /CS /DeviceRGB /I true >>",
        "1 1 1 rg\n0 0 40 40 re\nf\n",
    );
    let resources = if soft_mask {
        format!("<< {resources} /ExtGState << /GS 8 0 R >> >>")
    } else {
        format!("<< {resources} >>")
    };

    let objects = vec![
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {PAGE} {PAGE}] \
             /Resources {resources} /Contents 4 0 R >>"
        ),
        stream("<<", &content),
        form.unwrap_or_else(|| "<< /Type /Null >>".to_owned()),
        mask_group,
        "<< /Type /Mask /S /Luminosity /G 6 0 R /BC [1 1 1] >>".to_owned(),
        "<< /Type /ExtGState /SMask 7 0 R /AIS true /CA 1 /ca 1 >>".to_owned(),
    ];
    assemble(&objects)
}

/// A stream object: the dictionary without its closing `>>`, then `/Length` and the data.
fn stream(dictionary: &str, data: &str) -> String {
    format!(
        "{dictionary} /Length {} >>\nstream\n{data}endstream",
        data.len()
    )
}

/// Writes the objects out with a cross-reference table §7.5.4 can be read back through.
fn assemble(objects: &[String]) -> Vec<u8> {
    let mut out = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::with_capacity(objects.len());
    for (index, object) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(
            format!("{} 0 obj\n{object}\nendobj\n", index.saturating_add(1)).as_bytes(),
        );
    }
    let start = out.len();
    let size = objects.len().saturating_add(1);
    out.extend_from_slice(format!("xref\n0 {size}\n0000000000 65535 f \n").as_bytes());
    for offset in &offsets {
        out.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!("trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{start}\n%%EOF\n").as_bytes(),
    );
    out
}
