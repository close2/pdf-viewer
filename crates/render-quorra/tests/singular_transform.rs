//! A mark that cannot be positioned costs itself and nothing else, on **all three** backends.
//!
//! ISO 32000-2 §8.3.4's third NOTE is the standard's whole statement about a matrix with no
//! inverse:
//!
//! > When rendering graphics objects, it is sometimes necessary for a PDF reader to perform the
//! > inverse of a transformation -that is, to find the user space coordinates that correspond to
//! > a given pair of device space coordinates. Not all transformations are invertible, however.
//! > For example, if a matrix contains a, b, c, and d elements that are all zero, all user
//! > coordinates map to the same device coordinates and there is no unique inverse
//! > transformation. Such noninvertible transformations are not very useful and generally arise
//! > from unintended operations, such as scaling by 0. Use of a noninvertible matrix when
//! > painting graphics objects can result in unpredictable behaviour.
//!
//! It makes the **mark's** result undefined and says nothing about the page the mark is on. Until
//! ADR 0482 all three backends said otherwise, each in its own spelling and each fatally:
//!
//! - `render-cpu` and `render-gpu` inverted a command's transform to place its paint — which is
//!   `tiny-skia`'s and Vello's requirement rather than the clause's, since both apply a draw's
//!   transform to the paint as well as to the shape — and turned the missing inverse into
//!   `UnsupportedPaint`, which propagated out of `rasterize`.
//! - `render-quorra` needs no such inverse, and had the same defect one step along: it resolves a
//!   stroke's device width as `path_width × max_stretch`, which a collapsing transform makes zero,
//!   and the *scene* refused it with `InvalidStroke`.
//!
//! Any of the three cost the reader the whole page, and one degenerate `cm` in `4605705.pdf`'s
//! damaged content cost it 293 commands that had drawn. That three libraries found three ways to
//! refuse one condition is why the condition is now stated once, in `pdf_render::paint_space`,
//! and why this file holds all three to it (trap 2).
//!
//! # The pair
//!
//! Each scene here is drawn twice: once with the unpositionable command in it, once with that
//! command taken out. The two rasters must be **byte-identical**, which asserts both halves of the
//! answer at once — the surviving marks are all still there, and the refused one deposited
//! nothing. Identity rather than a tolerance is the right expectation and is derived rather than
//! hoped for: a page transform is invertible, so a singular command transform makes the device
//! transform singular too and the command's whole path lands on a line or a point, covering no
//! area at any scale.
//!
//! Both paints are exercised and both painting operators, because the four combinations reached
//! four different refusals. A `Paint::Solid` never needed an inverse at all — a solid colour is
//! the same at every point — so the two library backends were refusing pages for a quantity
//! nobody was going to read, and that is the whole of the corpus population
//! (`pdf-model/examples/singular_transform_census`).
//!
//! Run against the defect before believing it: restore the `?` on `page_to_path` in
//! `render-cpu/src/lib.rs`, on `Spaces::new` in `render-gpu/src/scene.rs`, or take the
//! `paint_space` guard out of `render-quorra/src/stroke.rs`, and the `expect`s below fail on the
//! first scene that reaches the one you restored.

#![expect(
    clippy::expect_used,
    reason = "test code: a backend that cannot draw one rectangle onto a small page is exactly \
              the failure this file exists to report, and it reports it by name"
)]
#![expect(
    clippy::print_stdout,
    reason = "test code: the ink beside its twin's is the measurement, and `--nocapture` is how \
              it is read"
)]

use std::sync::Arc;

use pdf_render::{
    BlendMode, Color, Command, DisplayList, FillRule, Paint, Path, PathCommand, Point, Ramp,
    Raster, Rasterizer, Shading, ShadingKind, Size, Stop, Stroke, TargetSpec, Transform,
};
use render_quorra::QuorraRasterizer;

/// The page every scene is drawn on.
const PAGE: Size = Size {
    width: 120.0,
    height: 80.0,
};

/// The rectangle that must survive whatever its neighbour does — the "293 commands" of the
/// witness, reduced to one.
fn survivor() -> Command {
    let mut path = Path::new();
    path.push(PathCommand::MoveTo(Point::new(10.0, 10.0)));
    path.push(PathCommand::LineTo(Point::new(50.0, 10.0)));
    path.push(PathCommand::LineTo(Point::new(50.0, 40.0)));
    path.push(PathCommand::LineTo(Point::new(10.0, 40.0)));
    path.push(PathCommand::Close);
    Command::Fill {
        path: Arc::new(path),
        transform: Transform::IDENTITY,
        fill_rule: FillRule::NonZero,
        paint: Paint::Solid(Color::BLACK),
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    }
}

/// A square, in its own space, for the unpositionable command to state.
fn square() -> Arc<Path> {
    let mut path = Path::new();
    path.push(PathCommand::MoveTo(Point::new(0.0, 0.0)));
    path.push(PathCommand::LineTo(Point::new(1.0, 0.0)));
    path.push(PathCommand::LineTo(Point::new(1.0, 1.0)));
    path.push(PathCommand::LineTo(Point::new(0.0, 1.0)));
    path.push(PathCommand::Close);
    Arc::new(path)
}

/// A gradient across the unit square, so that the positioned branch has a paint to refuse.
fn gradient() -> Paint {
    Paint::Shading(Arc::new(Shading {
        kind: Arc::new(ShadingKind::Axial {
            start: Point::new(0.0, 0.0),
            end: Point::new(1.0, 0.0),
            ramp: Ramp {
                stops: Arc::from(
                    [
                        Stop {
                            at: 0.0,
                            colour: Color::BLACK,
                        },
                        Stop {
                            at: 1.0,
                            colour: Color {
                                r: 1.0,
                                g: 0.0,
                                b: 0.0,
                                a: 1.0,
                            },
                        },
                    ]
                    .as_slice(),
                ),
            },
            extend: (true, true),
        }),
        transform: Transform::IDENTITY,
    }))
}

/// Every matrix this file calls unpositionable, each with the name §8.3.4's NOTE gives it.
///
/// The first is the clause's own example — "a, b, c, and d elements that are all zero" — the
/// second is "scaling by 0" in one axis alone, which still stretches the other and is the case a
/// determinant test catches where an "is it all zeroes" test would not, and the third is a matrix
/// of rank one whose entries are none of them zero, which is the case a per-entry test misses
/// entirely.
const SINGULAR: [(&str, Transform); 3] = [
    (
        "all four elements zero",
        Transform {
            a: 0.0,
            b: 0.0,
            c: 0.0,
            d: 0.0,
            e: 30.0,
            f: 30.0,
        },
    ),
    (
        "scaled by 0 in y",
        Transform {
            a: 40.0,
            b: 0.0,
            c: 0.0,
            d: 0.0,
            e: 60.0,
            f: 50.0,
        },
    ),
    (
        "rank one with no zero entry",
        Transform {
            a: 20.0,
            b: 40.0,
            c: 10.0,
            d: 20.0,
            e: 60.0,
            f: 20.0,
        },
    ),
];

/// The pair: `survivor` alone, and `survivor` beside a command under `at` carrying `paint`.
fn pair(at: Transform, paint: &Paint, stroked: bool) -> (DisplayList, DisplayList) {
    let mut twin = DisplayList::new(PAGE);
    twin.push(survivor());

    let mut defect = DisplayList::new(PAGE);
    defect.push(survivor());
    defect.push(if stroked {
        Command::Stroke {
            path: square(),
            transform: at,
            stroke: Stroke {
                width: 0.1,
                ..Stroke::default()
            },
            paint: paint.clone(),
            clip: None,
            mask: None,
            blend: BlendMode::Normal,
        }
    } else {
        Command::Fill {
            path: square(),
            transform: at,
            fill_rule: FillRule::NonZero,
            paint: paint.clone(),
            clip: None,
            mask: None,
            blend: BlendMode::Normal,
        }
    });
    (defect, twin)
}

/// One backend: its name and a closure that draws a scene with it.
///
/// A closure rather than a `dyn Rasterizer`, because the three rasterisers have three error types
/// and what this file wants from each is one raster or a loud failure — the loud failure being
/// precisely the defect under test.
type Backend = (
    &'static str,
    Box<dyn FnMut(&DisplayList, TargetSpec) -> Raster>,
);

/// The three backends, **built once for the whole test** and each arm omitted where its device
/// cannot be created.
///
/// **Both halves of that sentence were paid for by one CI run.** This returned a fresh set per
/// scene, and it was called from the innermost loop — two graphics devices created twenty-four
/// times in one test. On a runner with no display `wgpu` falls back to GL, and `wgpu-hal`'s EGL
/// path `unwrap()`s a `BadDisplay` on the *second* display it is asked for, so the first scene
/// passed and the second aborted the test inside a dependency. That is
/// `doc/traps/pixels-and-rasterisers.md` trap 12b's fourth lesson again: where a dependency
/// returns success, ask what it does when it fails — and here it does not return at all.
///
/// So the devices are built once, which is what `render-gpu/tests/real_pages.rs` has always done
/// and is why that test survives the same runner. And a device that cannot be built **omits its
/// arm with a reason printed** rather than failing the test: a machine without a graphics adapter
/// cannot answer this question, and a test that fails there is a coin toss rather than a gate —
/// `doc/todo/02` §2's argument for the C compiler line, one crate over. The processor's arm is
/// never omitted, so the test still asserts something everywhere.
fn backends() -> Vec<Backend> {
    let mut cpu = render_cpu::CpuRasterizer::new();
    let mut backends: Vec<Backend> = vec![(
        "processor",
        Box::new(move |list, target| cpu.rasterize(list, target).expect("a cpu raster")),
    )];

    match QuorraRasterizer::new_headless() {
        Ok(mut quorra) => backends.push((
            "quorra",
            Box::new(move |list, target| quorra.rasterize(list, target).expect("a quorra raster")),
        )),
        Err(why) => println!("  omitted: quorra has no device here ({why})"),
    }
    match render_gpu::GpuRasterizer::new_headless() {
        Ok(mut gpu) => backends.push((
            "vello",
            Box::new(move |list, target| gpu.rasterize(list, target).expect("a vello raster")),
        )),
        Err(why) => println!("  omitted: vello has no device here ({why})"),
    }

    backends
}

/// Total ink over a raster, in device pixels, against the white medium.
///
/// The darkest channel rather than the red one, because one of the two paints here is a gradient
/// that reaches saturated red: measuring the red channel would read a fully painted red pixel as
/// blank, and the control below is exactly the test that would then pass while drawing nothing.
fn ink(raster: &Raster) -> f64 {
    raster
        .data
        .chunks_exact(4)
        .map(|pixel| 1.0 - f64::from(pixel[0].min(pixel[1]).min(pixel[2])) / 255.0)
        .sum()
}

/// The item this file exists for: the page keeps every mark that could be drawn.
///
/// At two scales, one of them fractional, because a singular transform composed with a page
/// transform is singular at every scale and a construction that only happened to work at 1 would
/// be indistinguishable here otherwise.
#[test]
fn a_mark_that_cannot_be_positioned_costs_only_itself() {
    let mut backends = backends();
    for (kind, at) in SINGULAR {
        assert_eq!(at.invert(), None, "{kind} has to be the case under test");
        for (paint_name, paint) in [
            ("solid", Paint::Solid(Color::BLACK)),
            ("shading", gradient()),
        ] {
            for stroked in [false, true] {
                let (defect, twin) = pair(at, &paint, stroked);
                for scale in [1.0_f32, 2.5] {
                    let target = TargetSpec::for_page(&defect, scale, 1 << 30).expect("a target");
                    for (name, draw) in &mut backends {
                        let with = draw(&defect, target);
                        let without = draw(&twin, target);
                        println!(
                            "  {kind:>27} {paint_name:>7} {} scale {scale} {name:>9}: \
                             ink {:.3} against its twin's {:.3}",
                            if stroked { "stroke" } else { "fill  " },
                            ink(&with),
                            ink(&without),
                        );
                        assert_eq!(
                            with.data,
                            without.data,
                            "{name} at scale {scale}: a {paint_name} {} under a matrix {kind} \
                             changed the rest of the page",
                            if stroked { "stroke" } else { "fill" },
                        );
                        assert!(
                            ink(&with) > 100.0,
                            "{name} at scale {scale}: the surviving rectangle is not on the page",
                        );
                    }
                }
            }
        }
    }
}

/// And the discrimination in the other direction: an *invertible* transform still draws.
///
/// Without this the test above passes on a backend that drew nothing at all under any transform,
/// which is the shape trap 2's fifth instance warns about — deleting the code a scene guards is
/// the only thing that establishes the scene guards it, and this is the same question asked of the
/// condition instead of the code.
#[test]
fn the_same_command_under_an_invertible_matrix_marks_the_page() {
    let at = Transform {
        a: 40.0,
        b: 0.0,
        c: 0.0,
        d: 20.0,
        e: 60.0,
        f: 50.0,
    };
    assert!(at.invert().is_some(), "the control has to be invertible");
    let mut backends = backends();
    for (paint_name, paint) in [
        ("solid", Paint::Solid(Color::BLACK)),
        ("shading", gradient()),
    ] {
        let (drawn, twin) = pair(at, &paint, false);
        let target = TargetSpec::for_page(&drawn, 1.0, 1 << 30).expect("a target");
        for (name, draw) in &mut backends {
            let with = ink(&draw(&drawn, target));
            let without = ink(&draw(&twin, target));
            println!("  control {paint_name:>7} {name:>9}: ink {with:.3} against {without:.3}");
            assert!(
                with > without + 100.0,
                "{name}: a {paint_name} fill under an invertible matrix put {:.3} device pixels \
                 of ink on the page, and the 40 x 20 square it states is 800",
                with - without,
            );
        }
    }
}
