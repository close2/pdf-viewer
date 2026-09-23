//! ISO 32000-2 §11.3.5's sixteen blend functions, transcribed from the clause for the
//! suite's references: `m6.rs` holds the device's composite against them, and
//! `non_isolated_blended_groups.rs` holds §11.4.4's group result under each.

use raster_scene::BlendMode;

/// §11.3.5's blend function B, transcribed from the clause for the reference.
pub fn blend_reference(mode: BlendMode, cb: [f32; 3], cs: [f32; 3]) -> [f32; 3] {
    let lum = |c: [f32; 3]| 0.30 * c[0] + 0.59 * c[1] + 0.11 * c[2];
    let clip_color = |c: [f32; 3]| {
        let l = lum(c);
        let n = c[0].min(c[1]).min(c[2]);
        let x = c[0].max(c[1]).max(c[2]);
        let mut out = c;
        if n < 0.0 {
            for v in &mut out {
                *v = l + (*v - l) * l / (l - n);
            }
        }
        if x > 1.0 {
            for v in &mut out {
                *v = l + (*v - l) * (1.0 - l) / (x - l);
            }
        }
        out
    };
    let set_lum = |c: [f32; 3], l: f32| {
        let d = l - lum(c);
        clip_color([c[0] + d, c[1] + d, c[2] + d])
    };
    let sat = |c: [f32; 3]| c[0].max(c[1]).max(c[2]) - c[0].min(c[1]).min(c[2]);
    let set_sat = |c: [f32; 3], s: f32| {
        let mn = c[0].min(c[1]).min(c[2]);
        let mx = c[0].max(c[1]).max(c[2]);
        if mx <= mn {
            return [0.0; 3];
        }
        [
            (c[0] - mn) * s / (mx - mn),
            (c[1] - mn) * s / (mx - mn),
            (c[2] - mn) * s / (mx - mn),
        ]
    };
    let per = |f: &dyn Fn(f32, f32) -> f32| [f(cb[0], cs[0]), f(cb[1], cs[1]), f(cb[2], cs[2])];
    match mode {
        BlendMode::Normal => cs,
        BlendMode::Multiply => per(&|b, s| b * s),
        BlendMode::Screen => per(&|b, s| b + s - b * s),
        BlendMode::Overlay => per(&|b, s| {
            if b <= 0.5 {
                s * (2.0 * b)
            } else {
                let b2 = 2.0 * b - 1.0;
                s + b2 - s * b2
            }
        }),
        BlendMode::Darken => per(&|b, s| b.min(s)),
        BlendMode::Lighten => per(&|b, s| b.max(s)),
        BlendMode::ColorDodge => per(&|b, s| {
            if b <= 0.0 {
                0.0
            } else if s >= 1.0 {
                1.0
            } else {
                (b / (1.0 - s)).min(1.0)
            }
        }),
        BlendMode::ColorBurn => per(&|b, s| {
            if b >= 1.0 {
                1.0
            } else if s <= 0.0 {
                0.0
            } else {
                1.0 - ((1.0 - b) / s).min(1.0)
            }
        }),
        BlendMode::HardLight => per(&|b, s| {
            if s <= 0.5 {
                b * (2.0 * s)
            } else {
                let s2 = 2.0 * s - 1.0;
                b + s2 - b * s2
            }
        }),
        BlendMode::SoftLight => per(&|b, s| {
            let d = if b <= 0.25 {
                ((16.0 * b - 12.0) * b + 4.0) * b
            } else {
                b.sqrt()
            };
            if s <= 0.5 {
                b - (1.0 - 2.0 * s) * b * (1.0 - b)
            } else {
                b + (2.0 * s - 1.0) * (d - b)
            }
        }),
        BlendMode::Difference => per(&|b, s| (b - s).abs()),
        BlendMode::Exclusion => per(&|b, s| b + s - 2.0 * b * s),
        BlendMode::Hue => set_lum(set_sat(cs, sat(cb)), lum(cb)),
        BlendMode::Saturation => set_lum(set_sat(cb, sat(cs)), lum(cb)),
        BlendMode::Color => set_lum(cs, lum(cb)),
        BlendMode::Luminosity => set_lum(cb, lum(cs)),
    }
}

/// The sixteen, in the clause's own table order, which is `BlendMode`'s.
pub const ALL_MODES: [BlendMode; 16] = [
    BlendMode::Normal,
    BlendMode::Multiply,
    BlendMode::Screen,
    BlendMode::Overlay,
    BlendMode::Darken,
    BlendMode::Lighten,
    BlendMode::ColorDodge,
    BlendMode::ColorBurn,
    BlendMode::HardLight,
    BlendMode::SoftLight,
    BlendMode::Difference,
    BlendMode::Exclusion,
    BlendMode::Hue,
    BlendMode::Saturation,
    BlendMode::Color,
    BlendMode::Luminosity,
];
