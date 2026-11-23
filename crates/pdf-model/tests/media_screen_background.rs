//! ISO 32000-2 Table 294's `/W` 3 on the page: while a sound plays in a screen annotation's
//! rectangle, `/B` is painted over it at `/O`'s opacity (ADR 3407), drawn on the CPU path.
#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "the controlled fixtures should fail loudly if malformed or indexed incorrectly"
)]

use pdf_model::media::Background;
use pdf_model::optional_content::Purpose;
use pdf_model::view::ViewState;
use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::{Document, ObjectId};
use render_cpu::CpuRasterizer;

/// A page whose screen annotation (object 5) at `[10 10 90 90]`, with Table 167 flags `flags`,
/// shows an appearance filled pure green.
fn screen_page(flags: u32) -> Vec<u8> {
    let appearance = "0 1 0 rg 0 0 80 80 re f";
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> /Contents 4 0 R /Annots [5 0 R] \
         >>"
        .to_owned(),
        "<< /Length 0 >>\nstream\n\nendstream".to_owned(),
        format!(
            "<< /Type /Annot /Subtype /Screen /Rect [10 10 90 90] /F {flags} /AP << /N 6 0 R >> >>"
        ),
        format!(
            "<< /Type /XObject /Subtype /Form /BBox [0 0 80 80] /Length {} >>\nstream\n{appearance}\nendstream",
            appearance.len()
        ),
    ];
    let mut output = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (number, object) in (1_usize..).zip(&objects) {
        offsets.push(output.len());
        output.extend_from_slice(format!("{number} 0 obj\n{object}\nendobj\n").as_bytes());
    }
    let xref = output.len();
    let size = offsets.len() + 1;
    output.extend_from_slice(format!("xref\n0 {size}\n0000000000 65535 f \n").as_bytes());
    for offset in offsets {
        output.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    output.extend_from_slice(
        format!("trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes(),
    );
    output
}

/// The pixel at the rectangle's centre, drawn for `purpose` with `playing` in the view state.
fn centre(flags: u32, purpose: Purpose, playing: Option<Background>) -> [u8; 4] {
    let document = Document::open(screen_page(flags)).expect("the fixture parses");
    assert!(!document.was_recovered(), "the fixture's offsets are exact");
    let page = pdf_model::Pages::new(&document).get(0).expect("one page");
    let mut view = ViewState::of(&document);
    view.set_purpose(purpose);
    view.set_playing_in(playing.map(|background| (ObjectId::new(5, 0), background)));
    let list = pdf_model::content::interpret_with(&document, &page, &view).display_list;
    let target = TargetSpec::for_page(&list, 1.0, 1 << 20).expect("bounded target");
    let raster = CpuRasterizer::new()
        .with_medium(pdf_render::Medium::NONE)
        .rasterize(&list, target)
        .expect("supported raster");
    let offset = ((50 * raster.width + 50) as usize) * 4;
    [
        raster.data[offset],
        raster.data[offset + 1],
        raster.data[offset + 2],
        raster.data[offset + 3],
    ]
}

/// Table 294: `/B` "shall be used if the media object does not entirely cover the rectangle", and
/// a sound covers none of it, so the whole rectangle is `/B` over the annotation's own appearance:
/// opaque red where `/O` is 1, half red over the green appearance where it is 0.5, and the
/// appearance again once the sound has ended. **The controls**: nothing playing shows the green
/// appearance; a printed sheet plays nothing; and an annotation §12.5.3's Hidden flag keeps off the
/// screen leaves no rectangle to paint.
#[test]
fn the_background_is_painted_over_the_rectangle_while_the_sound_plays() {
    let red = |opacity| Background {
        colour: [1.0, 0.0, 0.0],
        opacity,
    };
    assert_eq!(centre(0, Purpose::View, None)[..3], [0, 255, 0]);
    assert_eq!(centre(0, Purpose::View, Some(red(1.0)))[..3], [255, 0, 0]);
    let half = centre(0, Purpose::View, Some(red(0.5)));
    assert!(
        (126..=129).contains(&half[0]) && (126..=129).contains(&half[1]) && half[2] == 0,
        "half red over green: {half:?}"
    );
    assert_eq!(
        centre(0, Purpose::View, Some(Background::default()))[..3],
        [0, 0, 0],
        "the stated default, opaque black"
    );
    // Table 167's Print flag (bit 3) puts the annotation on the sheet, and the sound does not.
    assert_eq!(centre(4, Purpose::Print, Some(red(1.0)))[..3], [0, 255, 0]);
    let hidden = centre(2, Purpose::View, Some(red(1.0)));
    assert_ne!(hidden[..3], [255, 0, 0], "{hidden:?}");
}
