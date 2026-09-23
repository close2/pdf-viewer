//! ISO 32000-2 §11.4.4's non-isolated group composited under a blend mode of its own —
//! the case where the backdrop removal does not cancel against the composite, so the
//! group's own colour is needed as the clause's Result step states it.
//!
//! # Where the expected values come from
//!
//! [`clause_group`] transcribes §11.4.4 from the standard — the initialisation, the
//! per-element recurrence, the Result step `C = Cn + (Cn − C0) × (α0/αgn − α0)` with
//! `α = αgn`, and the recursion into an element that is itself a group — and
//! [`clause_composite`] transcribes §11.3.6's compositing formula, with §11.3.5's
//! sixteen blend functions from `common::blend`. Nothing here is read from another
//! renderer: the expectations are the clause's arithmetic on the scene's own numbers.
//!
//! The first test is the derivation `raster_scene::GroupSpec::isolated` rests on: with
//! the group alpha `αg` in hand — NOTE 4's second set of accumulators, which the device
//! keeps by drawing the elements again onto transparency — the Result step is the
//! premultiplied `αg × C = E(B) − (1 − αg) × B`, so the seeded raster and the group
//! alpha together are the group. The device tests then hold the pixels to the clause.

// Test-file lint policy as in m1.rs; the reference math mirrors clause arithmetic.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::arithmetic_side_effects
)]

use raster_scene::{
    Affine, BlendMode, Color, Compose, GroupSpec, Point, Rect, SceneBuilder, SceneError,
};

mod common;

use common::blend::{ALL_MODES, blend_reference};
use common::headless::{device, render};
use common::probe::pixel;

// ---------------------------------------------------------------- the clause itself

/// §11.3.1's Union(b, s) = b + s − b·s.
fn union(b: f32, s: f32) -> f32 {
    b + s - b * s
}

type Rgb = [f32; 3];

fn lerp(a: Rgb, b: Rgb, t: f32) -> Rgb {
    [
        (1.0 - t) * a[0] + t * b[0],
        (1.0 - t) * a[1] + t * b[1],
        (1.0 - t) * a[2] + t * b[2],
    ]
}

/// One group element: an elementary object with a straight colour, a source alpha and a
/// blend mode, or a non-isolated group with its elements, constant alpha and blend mode.
#[derive(Clone, Debug)]
enum Element {
    Object {
        colour: Rgb,
        alpha: f32,
        mode: BlendMode,
    },
    Group {
        elements: Vec<Element>,
        alpha: f32,
        mode: BlendMode,
    },
}

/// §11.4.4's group compositing function for a non-isolated, non-knockout group over the
/// backdrop `(c0, a0)`: Table 139's results `C` and `α = αgn`.
///
/// "For an element that is a group, the group compositing function shall be applied
/// recursively to the subgroup", onto the backdrop the parent has accumulated so far,
/// and its constant alpha multiplies its object alpha as §11.3.7.2's opacity input.
fn clause_group(c0: Rgb, a0: f32, elements: &[Element]) -> (Rgb, f32) {
    let mut ag = 0.0_f32;
    let mut c_prev = c0;
    let mut a_prev = a0;
    for element in elements {
        let (cs, alpha_s, mode) = match element {
            Element::Object {
                colour,
                alpha,
                mode,
            } => (*colour, *alpha, *mode),
            Element::Group {
                elements,
                alpha,
                mode,
            } => {
                let (c, a) = clause_group(c_prev, a_prev, elements);
                (c, a * alpha, *mode)
            }
        };
        // αgi = Union(αg(i−1), αsi); αi = Union(α0, αgi).
        ag = union(ag, alpha_s);
        let a_i = union(a0, ag);
        // Ci = (1 − αsi/αi)·C(i−1) + (αsi/αi)·((1 − α(i−1))·Csi + α(i−1)·Bi(C(i−1), Csi)).
        let blended = blend_reference(mode, c_prev, cs);
        let mixed = lerp(cs, blended, a_prev);
        if a_i > 0.0 {
            c_prev = lerp(c_prev, mixed, alpha_s / a_i);
        }
        a_prev = a_i;
    }
    // Result: C = Cn + (Cn − C0)·(α0/αgn − α0).
    if ag <= 0.0 {
        return (c_prev, 0.0);
    }
    let k = a0 / ag - a0;
    let c = [
        c_prev[0] + (c_prev[0] - c0[0]) * k,
        c_prev[1] + (c_prev[1] - c0[1]) * k,
        c_prev[2] + (c_prev[2] - c0[2]) * k,
    ];
    (c, ag)
}

/// §11.3.6's compositing formula for one object of alpha `object_alpha × w` over a
/// backdrop: straight colours in, **premultiplied** colour and alpha out.
fn clause_composite(
    cb: Rgb,
    ab: f32,
    cs: Rgb,
    object_alpha: f32,
    w: f32,
    mode: BlendMode,
) -> [f32; 4] {
    let as_ = object_alpha * w;
    let ar = union(ab, as_);
    if ar <= 0.0 {
        return [0.0; 4];
    }
    let mixed = lerp(cs, blend_reference(mode, cb, cs), ab);
    let cr = lerp(cb, mixed, as_ / ar);
    [cr[0] * ar, cr[1] * ar, cr[2] * ar, ar]
}

/// The construction the device performs, premultiplied: the elements composited one by
/// one onto the seeded backdrop (`E(B)`), their alphas united onto transparency (`αg`,
/// the second accumulator), the group recovered as `(E(B) − (1 − αg) × B, αg)`, and that
/// composited by §11.3.6 under the group's mode.
fn construction(c0: Rgb, a0: f32, elements: &[Element], w: f32, mode: BlendMode) -> [f32; 4] {
    let (e, ag) = seeded(c0, a0, elements);
    let group = [
        e[0] - (1.0 - ag) * a0 * c0[0],
        e[1] - (1.0 - ag) * a0 * c0[1],
        e[2] - (1.0 - ag) * a0 * c0[2],
    ];
    let straight = if ag > 0.0 {
        [group[0] / ag, group[1] / ag, group[2] / ag]
    } else {
        [0.0; 3]
    };
    clause_composite(c0, a0, straight, ag, w, mode)
}

/// `E(B)` premultiplied, and the group alpha, for a list of elements seeded with `(c0, a0)`.
fn seeded(c0: Rgb, a0: f32, elements: &[Element]) -> ([f32; 4], f32) {
    let (mut cb, mut ab) = (c0, a0);
    let mut ag = 0.0_f32;
    for element in elements {
        let out = match element {
            Element::Object {
                colour,
                alpha,
                mode,
            } => {
                ag = union(ag, *alpha);
                clause_composite(cb, ab, *colour, *alpha, 1.0, *mode)
            }
            Element::Group {
                elements,
                alpha,
                mode,
            } => {
                let (_, inner_ag) = seeded(cb, ab, elements);
                ag = union(ag, inner_ag * alpha);
                construction(cb, ab, elements, *alpha, *mode)
            }
        };
        ab = out[3];
        cb = if ab > 0.0 {
            [out[0] / ab, out[1] / ab, out[2] / ab]
        } else {
            [0.0; 3]
        };
    }
    ([cb[0] * ab, cb[1] * ab, cb[2] * ab, ab], ag)
}

/// A fixed generator: a proof obligation, not a fuzz run.
struct Fixed(u64);

impl Fixed {
    fn next(&mut self) -> f32 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        ((self.0 >> 40) as f32) / ((1_u64 << 24) as f32)
    }

    fn colour(&mut self) -> Rgb {
        [self.next(), self.next(), self.next()]
    }

    fn mode(&mut self) -> BlendMode {
        ALL_MODES[((self.next() * 16.0) as usize).min(15)]
    }

    fn elements(&mut self, depth: u32) -> Vec<Element> {
        let n = 1 + (self.next() * 3.0) as usize;
        (0..n)
            .map(|_| {
                if depth > 0 && self.next() < 0.3 {
                    Element::Group {
                        elements: self.elements(depth - 1),
                        alpha: self.next(),
                        mode: self.mode(),
                    }
                } else {
                    Element::Object {
                        colour: self.colour(),
                        alpha: self.next(),
                        mode: self.mode(),
                    }
                }
            })
            .collect()
    }
}

/// The derivation: for every one of §11.3.5's modes on the group, the seeded raster and
/// the group alpha reproduce §11.4.4's Result step composited by §11.3.6 — including
/// groups nested inside the group, whose own group alpha the second accumulator draws.
#[test]
fn the_seeded_raster_and_the_group_alpha_are_the_clause() {
    const TRIALS: usize = 50_000;
    let mut rng = Fixed(0x1144_0bad_5eed);
    for mode in ALL_MODES {
        let mut worst = 0.0_f32;
        for _ in 0..TRIALS {
            let (c0, a0, w) = (rng.colour(), rng.next(), rng.next());
            let elements = rng.elements(2);
            let (c, ag) = clause_group(c0, a0, &elements);
            let want = clause_composite(c0, a0, c, ag, w, mode);
            let got = construction(c0, a0, &elements, w, mode);
            for ch in 0..4 {
                worst = worst.max((want[ch] - got[ch]).abs());
            }
        }
        assert!(
            worst < 1e-4,
            "{mode:?}: the construction must be §11.4.4 composited by §11.3.6; worst \
             premultiplied deviation {worst:e}"
        );
    }
}

// ------------------------------------------------------------------ on the device

/// Each mode's patch is `PATCH` pixels square, sixteen of them in a row.
const PATCH: u32 = 4;
const W: u32 = PATCH * 16;
const H: u32 = PATCH;

fn patch(i: u32) -> Rect {
    let x = (i * PATCH) as f32;
    Rect::new(Point::new(x, 0.0), Point::new(x + PATCH as f32, H as f32))
}

fn group(isolated: bool, alpha: f32, blend: BlendMode) -> GroupSpec {
    GroupSpec {
        alpha,
        blend,
        clip: None,
        knockout: false,
        mask: None,
        isolated,
        compose: Compose::SrcOver,
    }
}

fn colour(c: Rgb, a: f32) -> Color {
    Color::new(c[0], c[1], c[2], a)
}

/// The scene's two elements: a translucent Normal rectangle and a rectangle that
/// **blends** by Multiply (§11.3.5 for a single element is an implicit one-element
/// group, and `Command::Rect` composites Normal, so this is how a scene states one).
const FIRST: (Rgb, f32) = ([0.3, 0.7, 0.85], 0.75);
const SECOND: (Rgb, f32) = ([0.95, 0.25, 0.5], 0.6);

fn elements(rect: Rect, body: &mut SceneBuilder) -> Result<(), SceneError> {
    body.rect(rect, Affine::IDENTITY, colour(FIRST.0, FIRST.1), None, None)?;
    body.group(group(true, 1.0, BlendMode::Multiply), |inner| {
        inner.rect(
            rect,
            Affine::IDENTITY,
            colour(SECOND.0, SECOND.1),
            None,
            None,
        )
    })
}

fn clause_elements() -> Vec<Element> {
    vec![
        Element::Object {
            colour: FIRST.0,
            alpha: FIRST.1,
            mode: BlendMode::Normal,
        },
        Element::Object {
            colour: SECOND.0,
            alpha: SECOND.1,
            mode: BlendMode::Multiply,
        },
    ]
}

/// The clause's premultiplied answer as the straight-alpha bytes a readback hands back.
fn bytes(premultiplied: [f32; 4]) -> [u8; 4] {
    let a = premultiplied[3];
    let straight = |v: f32| if a > 0.0 { v / a } else { 0.0 };
    let byte = |v: f32| (v * 255.0).round().clamp(0.0, 255.0) as u8;
    [
        byte(straight(premultiplied[0])),
        byte(straight(premultiplied[1])),
        byte(straight(premultiplied[2])),
        byte(a),
    ]
}

/// The worst channel difference between a patch's centre pixel and the clause's answer.
fn worst(got: [u8; 4], want: [u8; 4]) -> i32 {
    (0..4)
        .map(|ch| (i32::from(got[ch]) - i32::from(want[ch])).abs())
        .max()
        .unwrap()
}

/// The picture §11.4.4 and §11.3.6 ask for, under each of the sixteen modes on the group:
/// the elements see the page under the group, the backdrop is removed once, and the
/// group is composited with it under its own `/BM` — over an opaque backdrop and a
/// translucent one, at full and half constant alpha.
#[test]
fn a_non_isolated_group_under_each_mode_is_the_clause() {
    let mut device = device();
    let backdrop: Rgb = [0.9, 0.55, 0.2];
    for (backdrop_alpha, alpha) in [(1.0_f32, 1.0_f32), (1.0, 0.5), (0.6, 1.0), (0.6, 0.5)] {
        let mut builder = SceneBuilder::new();
        for (i, mode) in ALL_MODES.into_iter().enumerate() {
            let rect = patch(i as u32);
            builder
                .rect(
                    rect,
                    Affine::IDENTITY,
                    colour(backdrop, backdrop_alpha),
                    None,
                    None,
                )
                .unwrap();
            builder
                .group(group(false, alpha, mode), |body| elements(rect, body))
                .expect("a non-isolated group under any blend mode is drawn");
        }
        let pixels = render(&mut device, &builder.finish(), W, H);
        for (i, mode) in ALL_MODES.into_iter().enumerate() {
            let got = pixel(&pixels, W, i as u32 * PATCH + PATCH / 2, H / 2);
            let (c, ag) = clause_group(backdrop, backdrop_alpha, &clause_elements());
            let want = bytes(clause_composite(
                backdrop,
                backdrop_alpha,
                c,
                ag,
                alpha,
                mode,
            ));
            let diff = worst(got, want);
            assert!(
                diff <= 3,
                "{mode:?}, backdrop alpha {backdrop_alpha}, group alpha {alpha}: device \
                 {got:?} vs §11.4.4 then §11.3.6 {want:?} (diff {diff})"
            );
        }
    }
}

/// The mode on the group reaches the pixels, and the isolation flag still does: under
/// Multiply the non-isolated group, the isolated one and the same group under Normal are
/// three pictures, and each is its clause's. The group holds the blending element
/// alone, since §11.4.4's NOTE 2 puts the whole difference isolation makes there.
#[test]
fn the_groups_mode_and_its_isolation_each_change_the_picture() {
    let mut device = device();
    let backdrop: Rgb = [0.9, 0.55, 0.2];
    let draw = |device: &mut raster_gpu::Device, spec: GroupSpec| {
        let mut builder = SceneBuilder::new();
        builder
            .rect(
                patch(0),
                Affine::IDENTITY,
                colour(backdrop, 1.0),
                None,
                None,
            )
            .unwrap();
        builder
            .group(spec, |body| {
                body.group(group(true, 1.0, BlendMode::Multiply), |inner| {
                    inner.rect(
                        patch(0),
                        Affine::IDENTITY,
                        colour(SECOND.0, SECOND.1),
                        None,
                        None,
                    )
                })
            })
            .unwrap();
        pixel(
            &render(device, &builder.finish(), PATCH, H),
            PATCH,
            PATCH / 2,
            H / 2,
        )
    };
    let non_isolated = draw(&mut device, group(false, 1.0, BlendMode::Multiply));
    let isolated = draw(&mut device, group(true, 1.0, BlendMode::Multiply));
    let normal = draw(&mut device, group(false, 1.0, BlendMode::Normal));

    let blending = [clause_elements()[1].clone()];
    let (c, ag) = clause_group(backdrop, 1.0, &blending);
    let want = bytes(clause_composite(
        backdrop,
        1.0,
        c,
        ag,
        1.0,
        BlendMode::Multiply,
    ));
    assert!(
        worst(non_isolated, want) <= 3,
        "{non_isolated:?} vs {want:?}"
    );
    let (c, ag) = clause_group([0.0; 3], 0.0, &blending);
    let want = bytes(clause_composite(
        backdrop,
        1.0,
        c,
        ag,
        1.0,
        BlendMode::Multiply,
    ));
    assert!(worst(isolated, want) <= 3, "{isolated:?} vs {want:?}");

    for (a, b, what) in [
        (non_isolated, isolated, "isolation (§11.4.4 NOTE 2)"),
        (
            non_isolated,
            normal,
            "the group's own mode (§11.4.4 NOTE 3)",
        ),
    ] {
        assert!(
            worst(a, b) > 10,
            "{what} must reach the pixels: {a:?} vs {b:?}"
        );
    }
}

/// A non-isolated group under Screen inside a non-isolated group under Multiply: the
/// inner group's backdrop is what the outer one has accumulated, and the outer group's
/// alpha counts the inner one's — which the second accumulator draws with no layer of
/// its own for the inner group's colour.
#[test]
fn a_nested_non_isolated_group_under_a_mode_is_the_clause() {
    let mut device = device();
    let backdrop: Rgb = [0.2, 0.6, 0.9];
    let inner_colour: (Rgb, f32) = ([0.8, 0.4, 0.1], 0.7);
    let mut builder = SceneBuilder::new();
    let rect = patch(0);
    builder
        .rect(rect, Affine::IDENTITY, colour(backdrop, 1.0), None, None)
        .unwrap();
    builder
        .group(group(false, 0.8, BlendMode::Multiply), |outer| {
            elements(rect, outer)?;
            outer.group(group(false, 0.5, BlendMode::Screen), |inner| {
                inner.group(group(true, 1.0, BlendMode::Difference), |leaf| {
                    leaf.rect(
                        rect,
                        Affine::IDENTITY,
                        colour(inner_colour.0, inner_colour.1),
                        None,
                        None,
                    )
                })
            })
        })
        .unwrap();
    let got = pixel(
        &render(&mut device, &builder.finish(), PATCH, H),
        PATCH,
        PATCH / 2,
        H / 2,
    );

    let mut outer = clause_elements();
    outer.push(Element::Group {
        elements: vec![Element::Object {
            colour: inner_colour.0,
            alpha: inner_colour.1,
            mode: BlendMode::Difference,
        }],
        alpha: 0.5,
        mode: BlendMode::Screen,
    });
    let (c, ag) = clause_group(backdrop, 1.0, &outer);
    let want = bytes(clause_composite(
        backdrop,
        1.0,
        c,
        ag,
        0.8,
        BlendMode::Multiply,
    ));
    assert!(
        worst(got, want) <= 3,
        "device {got:?} vs the clause applied recursively {want:?}"
    );
}
