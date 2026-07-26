//! Canonical display lists shared by backend tests and the comparison harness.
//!
//! These exist so that every backend is tested against *identical* input. If each
//! backend's tests built their own scenes, the two could drift apart, and a
//! cross-backend comparison would no longer be evidence of anything: a difference
//! could just as easily mean the scenes differed as that a backend was wrong.
//!
//! This is a normal dependency rather than a dev-dependency, because the comparison
//! harness in `tools/` will use the same scenes to check backends against external
//! reference renderers.

#![forbid(unsafe_code)]

use std::sync::Arc;

use pdf_render::display_list::Clip;
use pdf_render::{
    BlendMode, Color, Command, DisplayList, FillRule, Paint, Path, PathCommand, Point, Size,
    Stroke, Transform,
};

/// A4 in PDF user-space units: 210mm x 297mm at 72 units per inch.
pub const A4: Size = Size {
    width: 595.0,
    height: 842.0,
};

/// Opaque red.
pub const RED: Color = Color {
    r: 1.0,
    g: 0.0,
    b: 0.0,
    a: 1.0,
};
/// Opaque green.
pub const GREEN: Color = Color {
    r: 0.0,
    g: 1.0,
    b: 0.0,
    a: 1.0,
};
/// Opaque blue.
pub const BLUE: Color = Color {
    r: 0.0,
    g: 0.0,
    b: 1.0,
    a: 1.0,
};

/// Builds an axis-aligned rectangle as a closed path.
#[must_use]
pub fn rect(x0: f32, y0: f32, x1: f32, y1: f32) -> Path {
    let mut path = Path::new();
    path.push(PathCommand::MoveTo(Point::new(x0, y0)));
    path.push(PathCommand::LineTo(Point::new(x1, y0)));
    path.push(PathCommand::LineTo(Point::new(x1, y1)));
    path.push(PathCommand::LineTo(Point::new(x0, y1)));
    path.push(PathCommand::Close);
    path
}

/// A scene exercising an axis-aligned fill, a nested clip, and a thick stroke.
///
/// Deliberately built from axis-aligned edges at integer coordinates, so that with
/// antialiasing disabled a fill covers whole pixels exactly and assertions can be
/// exact rather than approximate. The one diagonal is in [`diagonal_stroke`], where
/// antialiasing differences are the point.
///
/// Contents, in page coordinates (origin bottom-left):
///
/// - a red square from (100,100) to (300,300);
/// - a green square from (400,400) to (500,500), clipped to its lower-left quarter, so
///   that a backend ignoring clips is caught;
/// - a 10-unit blue stroke along y=600.
///
/// # Panics
///
/// Cannot panic in practice: the only fallible step is registering a clip, which
/// fails only past `u32::MAX` clips, and this scene registers one.
#[must_use]
#[expect(
    clippy::expect_used,
    reason = "a single add_clip cannot exhaust a u32 clip index; a Result here would \
              push an impossible error case onto every caller"
)]
pub fn basic() -> DisplayList {
    let mut list = DisplayList::new(A4);

    list.push(Command::Fill {
        path: Arc::new(rect(100.0, 100.0, 300.0, 300.0)),
        transform: Transform::IDENTITY,
        fill_rule: FillRule::NonZero,
        paint: Paint::Solid(RED),
        clip: None,
        blend: BlendMode::Normal,
    });

    let clip = list
        .add_clip(Clip {
            path: rect(400.0, 400.0, 450.0, 450.0),
            transform: Transform::IDENTITY,
            fill_rule: FillRule::NonZero,
            parent: None,
        })
        .expect("a single clip is always addressable");

    list.push(Command::Fill {
        path: Arc::new(rect(400.0, 400.0, 500.0, 500.0)),
        transform: Transform::IDENTITY,
        fill_rule: FillRule::NonZero,
        paint: Paint::Solid(GREEN),
        clip: Some(clip),
        blend: BlendMode::Normal,
    });

    let mut line = Path::new();
    line.push(PathCommand::MoveTo(Point::new(50.0, 600.0)));
    line.push(PathCommand::LineTo(Point::new(545.0, 600.0)));
    list.push(Command::Stroke {
        path: Arc::new(line),
        transform: Transform::IDENTITY,
        stroke: Stroke {
            width: 10.0,
            ..Stroke::default()
        },
        paint: Paint::Solid(BLUE),
        clip: None,
        blend: BlendMode::Normal,
    });

    list
}

/// A single diagonal stroke, which is where antialiasing differences between backends
/// actually appear.
///
/// Axis-aligned geometry agrees exactly between rasterisers; a diagonal edge is where
/// coverage is computed differently. This scene therefore sets the realistic floor for
/// cross-backend comparison tolerances.
#[must_use]
pub fn diagonal_stroke() -> DisplayList {
    let mut list = DisplayList::new(A4);

    let mut line = Path::new();
    line.push(PathCommand::MoveTo(Point::new(50.0, 100.0)));
    line.push(PathCommand::LineTo(Point::new(545.0, 742.0)));
    list.push(Command::Stroke {
        path: Arc::new(line),
        transform: Transform::IDENTITY,
        stroke: Stroke {
            width: 12.0,
            ..Stroke::default()
        },
        paint: Paint::Solid(BLUE),
        clip: None,
        blend: BlendMode::Normal,
    });

    list
}

/// A curved shape: two cubic Béziers forming a closed lens.
///
/// Curves are flattened to line segments before rasterisation, and the two backends
/// choose their own flattening tolerance, so this is the scene most likely to expose a
/// genuine geometric disagreement rather than mere edge antialiasing.
#[must_use]
pub fn curves() -> DisplayList {
    let mut list = DisplayList::new(A4);

    let mut path = Path::new();
    path.push(PathCommand::MoveTo(Point::new(100.0, 400.0)));
    path.push(PathCommand::CurveTo(
        Point::new(200.0, 700.0),
        Point::new(400.0, 700.0),
        Point::new(500.0, 400.0),
    ));
    path.push(PathCommand::CurveTo(
        Point::new(400.0, 100.0),
        Point::new(200.0, 100.0),
        Point::new(100.0, 400.0),
    ));
    path.push(PathCommand::Close);

    list.push(Command::Fill {
        path: Arc::new(path),
        transform: Transform::IDENTITY,
        fill_rule: FillRule::NonZero,
        paint: Paint::Solid(RED),
        clip: None,
        blend: BlendMode::Normal,
    });

    list
}

/// A page filled edge to edge, on a page whose pixel width is not a multiple of the
/// GPU row alignment.
///
/// `copy_texture_to_buffer` pads each row to a 256-byte boundary. A backend that fails
/// to strip that padding produces a progressively sheared image rather than an
/// obviously broken one, so a uniform fill at an unaligned width makes the bug
/// unmissable: any stray pixel is not the fill colour.
///
/// 101 units wide gives 404 bytes per row, which pads to 512.
#[must_use]
pub fn unaligned_full_bleed() -> DisplayList {
    let size = Size {
        width: 101.0,
        height: 37.0,
    };
    let mut list = DisplayList::new(size);

    list.push(Command::Fill {
        path: Arc::new(rect(0.0, 0.0, size.width, size.height)),
        transform: Transform::IDENTITY,
        fill_rule: FillRule::NonZero,
        paint: Paint::Solid(RED),
        clip: None,
        blend: BlendMode::Normal,
    });

    list
}

/// The [`basic`] scene expressed as a minimal PDF file.
///
/// # The invariant this crate exists to hold
///
/// These bytes and [`basic`] must describe *the same page*. That pairing is what makes
/// the reference-comparison harness meaningful: it renders this PDF with `pdftoppm`,
/// `mutool` and `gs`, renders [`basic`] with our own backends, and compares. If the two
/// descriptions drifted apart, every comparison would report a difference that is
/// nobody's bug.
///
/// Keeping both in one file is the whole point — a reviewer can check them against each
/// other without leaving the module.
///
/// The content stream mirrors [`basic`] operator for operator:
///
/// ```text
/// 1 0 0 rg  100 100 200 200 re f          red square (100,100)-(300,300)
/// q  400 400 50 50 re  W n                clip to the lower-left quarter
///    0 1 0 rg  400 400 100 100 re f       green square, clipped
/// Q
/// 0 0 1 RG  10 w  50 600 m 545 600 l S    blue stroke along y=600
/// ```
///
/// Once `pdf-syntax` exists, this same file becomes a parser fixture, and the display
/// list will be produced *from* it rather than written alongside it.
#[must_use]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "object indices and the cross-reference size are bounded by a four-element \
              literal array, so no operation here can overflow"
)]
pub fn basic_pdf() -> Vec<u8> {
    // Mirrors `basic()`. Any edit here needs the matching edit there.
    let content = b"1 0 0 rg\n\
                    100 100 200 200 re\n\
                    f\n\
                    q\n\
                    400 400 50 50 re\n\
                    W n\n\
                    0 1 0 rg\n\
                    400 400 100 100 re\n\
                    f\n\
                    Q\n\
                    0 0 1 RG\n\
                    10 w\n\
                    50 600 m\n\
                    545 600 l\n\
                    S\n"
    .to_vec();

    let objects: Vec<Vec<u8>> = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {} {}] \
             /Contents 4 0 R /Resources << >> >>",
            A4.width, A4.height
        )
        .into_bytes(),
        [
            format!("<< /Length {} >>\nstream\n", content.len()).into_bytes(),
            content,
            b"endstream".to_vec(),
        ]
        .concat(),
    ];

    // The header's binary comment marks the file as containing 8-bit data, which is what
    // tells tools not to treat it as text. Required by the specification for any file
    // with binary content, and harmless here.
    let mut out: Vec<u8> = b"%PDF-1.7\n%\xe2\xe3\xcf\xd3\n".to_vec();

    // Byte offsets are recorded as objects are written, because the cross-reference table
    // must point at each object's first byte. Computing them afterwards from a finished
    // buffer is the classic way to get an off-by-one that only some readers tolerate.
    let mut offsets = Vec::with_capacity(objects.len());
    for (index, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", index + 1).as_bytes());
        out.extend_from_slice(body);
        out.extend_from_slice(b"\nendobj\n");
    }

    let xref_offset = out.len();
    let size = objects.len() + 1;
    out.extend_from_slice(format!("xref\n0 {size}\n").as_bytes());
    // Entry zero is the head of the free list, and its format is fixed by the spec.
    out.extend_from_slice(b"0000000000 65535 f \n");
    for offset in &offsets {
        // Exactly 20 bytes per entry, trailing space included: readers may index into
        // this table arithmetically rather than parsing it.
        out.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!("trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_offset}\n%%EOF\n")
            .as_bytes(),
    );

    out
}
