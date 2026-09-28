//! A luminosity soft mask this backend cannot state is refused by what it is, not by a space it
//! is not in.
//!
//! ISO 32000-2 §11.5.3 computes a luminosity mask's values from the group composited in its own
//! blending colour space, and §11.3.4 composites each component of that space separately. A group
//! of four components — an `ICCBased` space of four, or `DeviceCMYK` whose group blends (ADR 1342)
//! — is therefore carried as §11.4.7's pair of rasters, chromatic and black, and a scene's mask is
//! one body; a group of three CIE-based components has a `Y` of its own that the scene's shader
//! does not compute. Both refusals send the frame to the CPU backend, and what this file asks is
//! that each sentence names its own reason — the named wrong answer being a `DeviceCMYK` pair
//! described as a CIE-based space.

#![expect(
    clippy::expect_used,
    reason = "test code: an explanatory panic is the intended failure mode"
)]

use std::sync::Arc;

use pdf_render::{
    BlackHalf, BlendMode, Color, Command, DisplayList, FillRule, Luminance, Paint, Path,
    PathCommand, Point, Rasterizer, Size, SoftMask, SoftMaskKind, TargetSpec, Transform,
};
use render_raster::QuorraRasterizer;

/// Pixel budget for a target; far above anything this test requests.
const GENEROUS: u64 = 1 << 30;

/// The page every fixture here is drawn on.
const PAGE: Size = Size {
    width: 64.0,
    height: 64.0,
};

/// A white, opaque colour.
const WHITE: Color = Color {
    r: 1.0,
    g: 1.0,
    b: 1.0,
    a: 1.0,
};

/// A fill of a 32-unit square in `paint`, under `mask`.
fn square(paint: Color, mask: Option<pdf_render::SoftMaskId>) -> Command {
    let mut path = Path::new();
    for command in [
        PathCommand::MoveTo(Point::new(16.0, 16.0)),
        PathCommand::LineTo(Point::new(48.0, 16.0)),
        PathCommand::LineTo(Point::new(48.0, 48.0)),
        PathCommand::LineTo(Point::new(16.0, 48.0)),
        PathCommand::Close,
    ] {
        path.push(command);
    }
    Command::Fill {
        path: Arc::new(path),
        transform: Transform::IDENTITY,
        fill_rule: FillRule::NonZero,
        paint: Paint::Solid(paint),
        clip: None,
        mask,
        blend: BlendMode::Normal,
    }
}

/// A page with one square drawn under a luminosity mask of the given shape.
fn masked(luminance: Luminance, black: Option<BlackHalf>) -> DisplayList {
    let mut list = DisplayList::new(PAGE);
    let mask = list
        .add_soft_mask(SoftMask {
            commands: vec![square(WHITE, None)],
            kind: SoftMaskKind::Luminosity { backdrop: WHITE },
            transfer: None,
            luminance: Some(luminance),
            black,
        })
        .expect("the first soft mask of this list");
    list.push(square(
        Color {
            r: 1.0,
            g: 0.0,
            b: 0.0,
            a: 1.0,
        },
        Some(mask),
    ));
    list
}

/// The refusal the scene gives for `list`.
fn refusal(list: &DisplayList) -> String {
    let target = TargetSpec::for_page(list, 1.0, GENEROUS).expect("the target fits the budget");
    QuorraRasterizer::new_headless()
        .expect("a device is available (ADR 0004)")
        .rasterize(list, target)
        .expect_err("a mask the scene cannot state is refused")
        .to_string()
}

/// `DeviceCMYK`'s blended group is the pair, and the sentence says so and does not call it
/// CIE-based.
#[test]
fn a_pair_of_rasters_is_refused_as_the_pair() {
    let list = masked(
        Luminance::device_ink(),
        Some(BlackHalf {
            commands: vec![square(WHITE, None)],
            backdrop: WHITE,
        }),
    );
    let said = refusal(&list);
    assert!(
        said.contains("pair of rasters")
            && said.contains("DeviceCMYK where the group blends")
            && said.contains("§11.4.7"),
        "{said}"
    );
    assert!(!said.contains("CIE-based"), "{said}");
}

/// Three CIE-based components have no pair, and the sentence names the `Y` alone.
#[test]
fn a_three_component_cie_based_luminosity_is_refused_as_its_own_y() {
    let list = masked(Luminance::curves(Arc::new([[0.0; 3]; 256])), None);
    let said = refusal(&list);
    assert!(
        said.contains("CIE-based space of three components") && said.contains("§11.5.3"),
        "{said}"
    );
    assert!(!said.contains("pair"), "{said}");
}
