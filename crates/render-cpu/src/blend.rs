//! The four non-separable blend modes, ISO 32000-2 §11.3.5.3, and the compositing
//! formula that applies them.
//!
//! # Why this backend computes them itself
//!
//! `tiny-skia` offers all sixteen modes, and twelve separable ones plus `Saturation`
//! agree with Vello to the channel. `Hue`, `Color` and `Luminosity` do not: they come
//! out 113 of 255 away from the clause's own arithmetic, because an intermediate in the
//! library's `u16x16` lane arithmetic exceeds sixteen bits and wraps — a debug build
//! panics inside `wide/u16x16_t.rs` with "attempt to multiply with overflow" rather than
//! producing the wrong number quietly. ADR 0046 has how that was found; ADR 0047 has why
//! the fix is here.
//!
//! Table 135's four are the ones worth taking back, because they are *non-separable*:
//! each is defined over all three components at once through [`lum`], [`clip_colour`],
//! [`set_lum`] and [`set_sat`], so no per-channel formula produces one and a backend
//! that got one subtly wrong would still draw a plausible picture.
//!
//! # Why it is not in `pdf-render`
//!
//! Trap 2's rule sends a *decision* the two backends could make differently into the
//! crate they share. This is not one: §11.3.5.3 states the arithmetic exactly, Vello
//! implements it in its own shader, and `cpu_and_gpu_agree_on_every_blend_mode` compares
//! the two. Moving these functions into `pdf-render` would make that scene compare one
//! implementation against itself, which is the thing the cross-backend comparison exists
//! not to do.

use pdf_render::BlendMode;

/// A straight-alpha RGB colour, as §11.3.5.3's formulas are written over.
///
/// The clause's auxiliary functions "operate on colours that are assumed to have red,
/// green, and blue components", and this backend's blending colour space is the device's
/// sRGB (§11.3.4), so there is no conversion to make before applying them.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Rgb {
    r: f32,
    g: f32,
    b: f32,
}

impl Rgb {
    /// Applies `f` to each component.
    fn map(self, f: impl Fn(f32) -> f32) -> Self {
        Self {
            r: f(self.r),
            g: f(self.g),
            b: f(self.b),
        }
    }

    /// The smallest component.
    fn min(self) -> f32 {
        self.r.min(self.g).min(self.b)
    }

    /// The largest component.
    fn max(self) -> f32 {
        self.r.max(self.g).max(self.b)
    }
}

/// §11.3.5.3's `Lum`, the luminosity of a colour: `0.3 × Cred + 0.59 × Cgreen + 0.11 × Cblue`.
///
/// Transliterated rather than quoted, and that is a property of the source: §11.3.5.3's
/// auxiliary functions are *images* in the specification PDF, so `doc/md/` holds
/// `<!-- formula-not-decoded -->` where each one should be and the citation checker can
/// verify no quotation of them. Table 135 survives the conversion because it is a table;
/// the four functions it is written in terms of do not.
///
/// Deliberately not §11.5.3's luminosity coefficients, which are the same three numbers
/// for the same reason but reach a *mask* value rather than a blend; and deliberately not
/// either rasteriser's own luminance, which is neither.
fn lum(c: Rgb) -> f32 {
    0.3_f32.mul_add(c.r, 0.59_f32.mul_add(c.g, 0.11 * c.b))
}

/// §11.3.5.3's `ClipColor`, which brings a colour back into range about its luminosity.
///
/// This is the step the CPU backend used to lose: [`set_lum`] adds the same offset to
/// every component and so routinely sends one outside `0..=1`, and clipping each
/// component on its own would change the colour's luminosity. The clause instead scales
/// the whole colour towards `L`, which leaves `Lum(C)` exactly where [`set_lum`] put it.
///
/// **The formula needs `Lum(C)` itself to be in range, and the clause's structure is what
/// supplies that**: `ClipColor` appears only inside `SetLum`, whose second argument is a
/// `Lum` of a colour the compositing formula has already brought into `0..=1`, and `SetLum`
/// makes that value the colour's luminosity exactly. Fed a colour of luminosity −0.39
/// directly, the same arithmetic returns −3.55 for a component. Not a defence to write in
/// code — there is no caller that can do it — but the reason there is none is worth
/// stating, because it is a property of the clause rather than of this function.
fn clip_colour(c: Rgb) -> Rgb {
    let l = lum(c);
    let (n, x) = (c.min(), c.max());
    let mut c = c;
    if n < 0.0 {
        // `l - n` is positive here, since `n` is negative and `l` is a weighted mean of
        // components no smaller than `n`; the degenerate `l == n` needs every component
        // equal, which contradicts `n < 0.0 <= l`.
        c = c.map(|v| l + (v - l) * l / (l - n));
    }
    if x > 1.0 {
        // Symmetrically, `x - l` is positive because `x` exceeds 1.0 and `l` is a
        // weighted mean of components no larger than `x`.
        c = c.map(|v| l + (v - l) * (1.0 - l) / (x - l));
    }
    c
}

/// §11.3.5.3's `SetLum`, giving a colour the luminosity `l`.
fn set_lum(c: Rgb, l: f32) -> Rgb {
    let d = l - lum(c);
    clip_colour(c.map(|v| v + d))
}

/// §11.3.5.3's `Sat`, the saturation of a colour: the largest component less the smallest.
fn sat(c: Rgb) -> f32 {
    c.max() - c.min()
}

/// §11.3.5.3's `SetSat`, giving a colour the saturation `s`.
///
/// The clause writes this over the components named by rank rather than by channel — "the
/// subscripts min, mid, and max … refer to the colour components having the minimum,
/// middle, and maximum values upon entry to the function" — so the implementation ranks
/// first and writes back afterwards. The minimum becomes 0 and the maximum becomes `s`
/// whatever the ranking, and the middle is placed between them in the same proportion it
/// held before.
///
/// ```text
/// if Cmax > Cmin
///     Cmid = (((Cmid − Cmin) × s) / (Cmax − Cmin))
///     Cmax = s
/// else
///     Cmid = Cmax = 0.0
/// endif
/// Cmin = 0.0
/// ```
///
/// The clause's own NOTE warns that the discontinuity between the two arms "makes it very
/// sensitive to variations between implementations", which is why this is written from
/// the formula rather than from any HSL library.
fn set_sat(c: Rgb, s: f32) -> Rgb {
    let (n, x) = (c.min(), c.max());
    // The middle value, whichever component holds it: the sum less the two extremes,
    // which is exact and needs no sorting network.
    let mid = c.r + c.g + c.b - n - x;
    let (new_mid, new_max) = if x > n {
        ((mid - n) * s / (x - n), s)
    } else {
        (0.0, 0.0)
    };
    // Written back by *value* rather than by channel: a component equal to the minimum
    // and to the middle is both, and either assignment gives the same answer because the
    // two results coincide whenever the two inputs do.
    let place = |v: f32| {
        if v <= n {
            0.0
        } else if v >= x {
            new_max
        } else {
            new_mid
        }
    };
    c.map(place)
}

/// One of Table 135's four modes, which is what this module answers.
///
/// A separate enumeration rather than a checked [`BlendMode`] so that [`NonSeparable::blend`]
/// is *total*: there is no arm for a separable mode to fall into, silently or otherwise, and
/// the one place that decides between the two paths is [`NonSeparable::of`]. The twelve
/// per-channel formulas stay `tiny-skia`'s, where they agree with Vello to the channel; a
/// second copy of them here would be a place for the two to drift.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NonSeparable {
    /// Table 135's `Hue`.
    Hue,
    /// Table 135's `Saturation`.
    Saturation,
    /// Table 135's `Color`.
    Color,
    /// Table 135's `Luminosity`.
    Luminosity,
}

impl NonSeparable {
    /// Recognises the four modes §11.3.5.3 defines, and `None` for the twelve of §11.3.5.2.
    pub(crate) fn of(mode: BlendMode) -> Option<Self> {
        Some(match mode {
            BlendMode::Hue => Self::Hue,
            BlendMode::Saturation => Self::Saturation,
            BlendMode::Color => Self::Color,
            BlendMode::Luminosity => Self::Luminosity,
            _ => return None,
        })
    }

    /// Table 135's blend function `B(Cb, Cs)`.
    ///
    /// # What these four do on a **neutral** pair, and why a whole clause depends on it
    ///
    /// §11.4.7's four-component page is drawn as two three-channel rasters, the second of
    /// which holds the complement of the black component in every channel
    /// (`pdf_render::blending`, `pdf_model::colour`'s `Half::Black`). §11.3.5.3 gives that
    /// component a rule that reads like a second blend function:
    ///
    /// > For the K component, the result shall be the K component of Cb for the Hue ,
    /// > Saturation , and Color blend modes; it shall be the K component of Cs for the
    /// > Luminosity blend mode.
    ///
    /// **It is not one.** With both operands neutral it is what the four lines below already
    /// return, because each of the clause's auxiliary functions degenerates:
    ///
    /// - `Sat` of a neutral colour is 0, and `SetSat(C, 0)` takes the `else` arm of the
    ///   clause's own `if Cmax > Cmin` — its operand is neutral too — and returns black.
    /// - `SetLum(C, l)` on a neutral `C` adds one offset to three equal components, so the
    ///   result is the neutral colour of luminosity `l`, and `ClipColor` has nothing to clip
    ///   because `l` came from a colour already in range.
    /// - `Lum` of a neutral colour is that colour's level, since the clause's three weights
    ///   sum to one.
    ///
    /// So `Hue`, `Saturation` and `Color` all come to `SetLum(neutral, Lum(Cb))` = `Cb`, and
    /// `Luminosity` comes to `SetLum(Cb, Lum(Cs))` = `Cs`. That is the bullet, term for term,
    /// and it means a backend that implements §11.3.5.3 for three components implements the
    /// black component's rule as well, with no second vocabulary anywhere between the
    /// interpreter and the raster. ADR 0277;
    /// [`tests::the_clauses_own_functions_give_the_black_components_rule_on_a_neutral_pair`]
    /// is the check over 200 000 pairs.
    fn blend(self, backdrop: Rgb, source: Rgb) -> Rgb {
        match self {
            // Table 135: B(Cb, Cs) = SetLum(SetSat(Cs, Sat(Cb)), Lum(Cb))
            Self::Hue => set_lum(set_sat(source, sat(backdrop)), lum(backdrop)),
            // Table 135: B(Cb, Cs) = SetLum(SetSat(Cb, Sat(Cs)), Lum(Cb))
            Self::Saturation => set_lum(set_sat(backdrop, sat(source)), lum(backdrop)),
            // Table 135: B(Cb, Cs) = SetLum(Cs, Lum(Cb))
            Self::Color => set_lum(source, lum(backdrop)),
            // Table 135: B(Cb, Cs) = SetLum(Cb, Lum(Cs))
            Self::Luminosity => set_lum(backdrop, lum(source)),
        }
    }
}

/// §11.4.4's interpolation between a group's backdrop and its elements composited onto it.
///
/// Writes `(1 − w) × destination + w × buffer` at every pixel of `surface`, premultiplied,
/// where `w` is `weight` times `mask`'s value there. `buffer` covers the whole surface and
/// `surface` is the band starting at `from_row` of it; `mask` is the band's own.
///
/// # Why this is the whole of a non-isolated group
///
/// `Command::Group`'s `isolated` derives it: §11.4.4 composites the elements onto the
/// group's backdrop and removes that backdrop again (NOTE 3), §11.3.3 then composites the
/// result back onto the same backdrop, and with the Normal blend function the division by
/// Table 140's group alpha and the multiplication by it cancel. What is left is this line,
/// and it holds for every backdrop alpha and every blend mode *inside* the group.
///
/// # Why it is a pass of its own rather than two Porter-Duff draws
///
/// Destination-Out by `w` followed by `Plus` of the buffer at `w` computes the same
/// expression and is what ADR 0234's shaped element uses. It rounds twice: at `w = ½` over
/// an opaque backdrop each draw keeps 127 of 255 and the pair leaves **254**, so a page the
/// clause makes fully opaque comes back one level transparent and the white behind it shows
/// through on every channel. One pass rounds once and the identity `w + (1 − w) = 1` is
/// exact in it.
pub(crate) fn interpolate(
    surface: &mut tiny_skia::PixmapMut<'_>,
    buffer: &tiny_skia::Pixmap,
    from_row: u32,
    weight: f32,
    mask: Option<&tiny_skia::Mask>,
) {
    let width = surface.width() as usize;
    let skipped = (from_row as usize).saturating_mul(width);
    let source = buffer.pixels().get(skipped..).unwrap_or_default();
    let coverage = mask.map(tiny_skia::Mask::data);
    for (index, destination) in surface.pixels_mut().iter_mut().enumerate() {
        let Some(&source) = source.get(index) else {
            break;
        };
        // Outside the group's marks the buffer *is* the backdrop, and interpolating a value
        // with itself is that value. The band is mostly that, so the branch is what keeps a
        // group's cost proportional to what it drew.
        if source == *destination {
            continue;
        }
        let level = coverage.map_or(255, |data| data.get(index).copied().unwrap_or(0));
        let w = weight * f32::from(level) / 255.0;
        if w <= 0.0 {
            continue;
        }
        let channel = |backdrop: u8, drawn: u8| {
            let mixed = f32::from(drawn).mul_add(w, f32::from(backdrop) * (1.0 - w));
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "a convex combination of two values in 0..=255, offset by half a \
                          level, so the truncating cast is the rounding and stays in range"
            )]
            let rounded = mixed.clamp(0.0, 255.0).round() as u8;
            rounded
        };
        // Premultiplied validity survives: each operand's channel is at most its own alpha,
        // so the same convex combination of the two bounds the combination of the alphas,
        // and rounding both the same way keeps the order.
        *destination = tiny_skia::PremultipliedColorU8::from_rgba(
            channel(destination.red(), source.red()),
            channel(destination.green(), source.green()),
            channel(destination.blue(), source.blue()),
            channel(destination.alpha(), source.alpha()),
        )
        .unwrap_or(*destination);
    }
}

/// §11.4.6's stage b) for one element of a non-isolated knockout group: the weighted
/// average of the element's composite with the initial backdrop and the accumulation so
/// far, weighted by the element's shape.
///
/// All four pixmaps cover the same pixels, premultiplied. `accumulated` is `P`, the group's
/// result so far (initially the backdrop itself); `backdrop` is `B`, the group's initial
/// backdrop; `composed` is `S`, the element drawn ordinarily onto a copy of `B` — its shape
/// baked in, where the clause's stage a) wants "a source shape value of 1.0 everywhere" —
/// and `shape` holds the element's shape `f` in its alpha channel.
///
/// # The identity that recovers stage a) from an ordinary draw
///
/// Writing `E` for the element composited at shape 1 (stage a)'s quantity) and expanding
/// §11.3.6's premultiplied form at source alpha `f × q` for the drawn scratch and `q` for
/// `E`, the shape factors out of every source term:
///
/// ```text
/// f × E = S − (1 − f) × B
/// ```
///
/// so stage b), `P' = (1 − f) × P + f × E`, becomes
///
/// ```text
/// P' = (1 − f) × (P − B) + S
/// ```
///
/// — exact for every blend mode and opacity, and computable without ever drawing an
/// object "everywhere". The parenthesis can be negative per channel, which is why the
/// arithmetic is signed and clamped only at the end; the true result is a premultiplied
/// colour, so each channel is clamped to the result alpha to keep rounding from breaking
/// that invariant by one level.
pub(crate) fn knockout_average(
    accumulated: &mut tiny_skia::Pixmap,
    backdrop: &tiny_skia::Pixmap,
    composed: &tiny_skia::Pixmap,
    shape: &tiny_skia::Pixmap,
) {
    for (((target, b), s), f) in accumulated
        .pixels_mut()
        .iter_mut()
        .zip(backdrop.pixels().iter().copied())
        .zip(composed.pixels().iter().copied())
        .zip(shape.pixels().iter().copied())
    {
        // Where the element has no shape and drew nothing, stage b) keeps the previous
        // result exactly; most of the surface is that, and the branch is what keeps an
        // element's cost proportional to its marks.
        if f.alpha() == 0 && s == b {
            continue;
        }
        let weight = 1.0 - f32::from(f.alpha()) / 255.0;
        let channel = |previous: u8, initial: u8, drawn: u8| {
            (f32::from(previous) - f32::from(initial)).mul_add(weight, f32::from(drawn))
        };
        let alpha = channel(target.alpha(), b.alpha(), s.alpha()).clamp(0.0, 255.0);
        let bounded = |value: f32| {
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "clamped to 0..=255 and offset by half a level, so the truncating \
                          cast is the rounding and stays in range"
            )]
            let rounded = (value.clamp(0.0, alpha) + 0.5).min(255.0) as u8;
            rounded
        };
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "clamped to 0..=255 and offset by half a level"
        )]
        let rounded_alpha = (alpha + 0.5).min(255.0) as u8;
        *target = tiny_skia::PremultipliedColorU8::from_rgba(
            bounded(channel(target.red(), b.red(), s.red())),
            bounded(channel(target.green(), b.green(), s.green())),
            bounded(channel(target.blue(), b.blue(), s.blue())),
            rounded_alpha,
        )
        .unwrap_or(*target);
    }
}

/// Composites a layer onto a surface under a non-separable blend mode.
///
/// Both pixmaps hold premultiplied eight-bit RGBA and cover the same pixels; `layer` is
/// what the command drew onto transparency, which is the source colour and shape §11.3.6
/// is written over. A separable mode never reaches here — it is `tiny-skia`'s — and the
/// layer is returned unapplied rather than misapplied if one does.
///
/// # The arithmetic
///
/// §11.3.6's compositing formula and §11.3.7.3's union, with the premultiplication folded
/// in so that only the blend function itself needs straight components. The clause's own
/// form, transliterated for the same reason [`lum`] is:
///
/// ```text
/// Cr = (1 − αs ÷ αr) × Cb + (αs ÷ αr) × ((1 − αb) × Cs + αb × B(Cb, Cs))
/// ```
///
/// Multiplying through by `αr = αb + αs − αb·αs` and substituting the premultiplied
/// `c = α·C` gives `cr = (1 − αs)·cb + (1 − αb)·cs + αs·αb·B(Cb, Cs)`, which has no division
/// by `αr` and so no case for §11.3.6's NOTE 2, "[i]f 𝛼𝑟 is zero, the result colour is
/// undefined": a zero result alpha needs both inputs transparent, and every term is then
/// zero.
pub(crate) fn composite(
    surface: &mut tiny_skia::PixmapMut<'_>,
    layer: &tiny_skia::Pixmap,
    mode: NonSeparable,
) {
    for (destination, source) in surface
        .pixels_mut()
        .iter_mut()
        .zip(layer.pixels().iter().copied())
    {
        let alpha_s = f32::from(source.alpha()) / 255.0;
        if alpha_s == 0.0 {
            // αs = 0 leaves every term but (1 − αs)·cb, which is the backdrop unchanged.
            // Worth the branch: a layer is mostly empty, since it holds one command.
            continue;
        }
        let alpha_b = f32::from(destination.alpha()) / 255.0;
        let straight = |value: u8, alpha: f32| {
            if alpha == 0.0 {
                0.0
            } else {
                f32::from(value) / 255.0 / alpha
            }
        };
        let backdrop = Rgb {
            r: straight(destination.red(), alpha_b),
            g: straight(destination.green(), alpha_b),
            b: straight(destination.blue(), alpha_b),
        };
        let source_colour = Rgb {
            r: straight(source.red(), alpha_s),
            g: straight(source.green(), alpha_s),
            b: straight(source.blue(), alpha_s),
        };
        let blended = mode.blend(backdrop, source_colour);

        let alpha_r = alpha_b.mul_add(-alpha_s, alpha_b + alpha_s);
        let channel = |cb: f32, cs: f32, b: f32| {
            // cb and cs arrive straight, so each is scaled by its own alpha here.
            let backdrop_term = (1.0 - alpha_s) * alpha_b * cb;
            let source_term = (1.0 - alpha_b) * alpha_s * cs;
            let blend_term = alpha_s * alpha_b * b;
            // Rounded rather than truncated, and clamped to the result alpha because a
            // premultiplied channel may not exceed it; the arithmetic cannot exceed it by
            // more than a float rounding, since each term's coefficients sum to αr.
            let premultiplied = (backdrop_term + source_term + blend_term).clamp(0.0, alpha_r);
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "clamped to 0..=alpha_r <= 1 and offset by half a level, so the \
                          truncating cast is the rounding and cannot leave 0..=255"
            )]
            let rounded = (premultiplied * 255.0 + 0.5) as u8;
            rounded
        };
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "alpha_r is a union of two values in 0..=1 and so is itself in that \
                      range; offset by half a level, the truncating cast is the rounding"
        )]
        let alpha = (alpha_r * 255.0 + 0.5) as u8;
        *destination = tiny_skia::PremultipliedColorU8::from_rgba(
            channel(backdrop.r, source_colour.r, blended.r),
            channel(backdrop.g, source_colour.g, blended.g),
            channel(backdrop.b, source_colour.b, blended.b),
            alpha,
        )
        .unwrap_or(*destination);
    }
}

#[cfg(test)]
mod tests {
    use super::{NonSeparable, Rgb, clip_colour, lum, sat, set_lum, set_sat};
    use pdf_render::BlendMode;

    /// Builds a colour, so that the tests below read like the clause's own triples.
    fn rgb(r: f32, g: f32, b: f32) -> Rgb {
        Rgb { r, g, b }
    }

    /// Asserts two colours agree to within eight-bit rounding.
    fn assert_close(actual: Rgb, expected: Rgb, what: &str) {
        for (a, e) in [
            (actual.r, expected.r),
            (actual.g, expected.g),
            (actual.b, expected.b),
        ] {
            assert!(
                (a - e).abs() < 1.0 / 512.0,
                "{what}: {actual:?} against {expected:?}"
            );
        }
    }

    /// §11.3.5.3's three coefficients, checked against the clause rather than against
    /// either rasteriser's luminance — which is the same shape of error §11.5.3's mask
    /// values were written to avoid.
    #[test]
    fn luminosity_is_the_clauses_weighted_sum() {
        assert!((lum(rgb(1.0, 0.0, 0.0)) - 0.30).abs() < 1e-6);
        assert!((lum(rgb(0.0, 1.0, 0.0)) - 0.59).abs() < 1e-6);
        assert!((lum(rgb(0.0, 0.0, 1.0)) - 0.11).abs() < 1e-6);
    }

    /// `ClipColor` brings a colour into range *without* moving its luminosity, which is
    /// the property that makes it different from clamping each component.
    ///
    /// The colours are ones `SetLum` can produce — a component out of range while the
    /// luminosity is in it — because that is `ClipColor`'s whole domain; see its own
    /// comment for what the arithmetic does outside it and why nothing can get there.
    #[test]
    fn clipping_preserves_luminosity() {
        for c in [
            rgb(0.81, -0.19, -0.19),
            rgb(1.4, 0.9, -0.2),
            rgb(-0.5, 0.9, 0.5),
        ] {
            let clipped = clip_colour(c);
            assert!(
                (lum(clipped) - lum(c)).abs() < 1e-5,
                "luminosity moved: {c:?} -> {clipped:?}"
            );
            assert!(
                clipped.min() >= -1e-5 && clipped.max() <= 1.0 + 1e-5,
                "not in range: {clipped:?}"
            );
        }
        // Component-wise clamping would give (0.81, 0.0, 0.0), whose luminosity is 0.243
        // rather than 0.11. That is the defect this whole module exists to fix, so it is
        // asserted as a difference rather than left implied.
        let naive = rgb(0.81, 0.0, 0.0);
        assert!((lum(naive) - lum(rgb(0.81, -0.19, -0.19))).abs() > 0.1);
    }

    /// `SetSat` ranks by value, so it must give the same answer whichever channel holds
    /// the maximum — a sorting mistake would show on one permutation and not another.
    #[test]
    fn saturation_is_set_by_rank_not_by_channel() {
        let permutations = [
            (rgb(0.8, 0.5, 0.2), rgb(1.0, 0.5, 0.0)),
            (rgb(0.2, 0.8, 0.5), rgb(0.0, 1.0, 0.5)),
            (rgb(0.5, 0.2, 0.8), rgb(0.5, 0.0, 1.0)),
        ];
        for (input, expected) in permutations {
            assert_close(set_sat(input, 1.0), expected, "set_sat");
            assert!((sat(set_sat(input, 0.4)) - 0.4).abs() < 1e-6);
        }
        // The clause's else arm: a colour with no saturation cannot be given one.
        assert_close(set_sat(rgb(0.5, 0.5, 0.5), 1.0), rgb(0.0, 0.0, 0.0), "flat");
    }

    /// The worked example from ADR 0046: red painted over blue in `Hue` is 0.367 in the
    /// red channel, and `tiny-skia` produced 0.81 — the value before `ClipColor`.
    ///
    /// Derived from the clause and not from either rasteriser: `Sat(blue)` is 1 and
    /// `SetSat(red, 1)` is red; `Lum(blue)` is 0.11 against `Lum(red)`'s 0.30, so `SetLum`
    /// adds −0.19 to each component and `ClipColor` then maps 0.81 to
    /// `L + (C − L) × L ÷ (L − n)` = 0.11 + 0.70 × 0.11 ÷ 0.30.
    #[test]
    fn hue_of_red_over_blue_is_the_clauses_arithmetic() {
        let (blue, red) = (rgb(0.0, 0.0, 1.0), rgb(1.0, 0.0, 0.0));
        let result = NonSeparable::Hue.blend(blue, red);
        let expected = 0.11 + 0.70 * 0.11 / 0.30;
        assert!(
            (result.r - expected).abs() < 1e-5,
            "{result:?} against {expected}"
        );
        assert!(
            (lum(result) - 0.11).abs() < 1e-5,
            "luminosity is the backdrop's"
        );
    }

    /// Table 135's NOTE 2: "[p]ainting with this mode in an area of the backdrop that is a
    /// pure gray (no saturation) produces no change." A property the clause states about
    /// its own formulas, so it tests them without restating them.
    #[test]
    fn saturation_over_grey_changes_nothing() {
        let grey = rgb(0.4, 0.4, 0.4);
        for source in [rgb(1.0, 0.0, 0.0), rgb(0.2, 0.7, 0.9)] {
            let result = NonSeparable::Saturation.blend(grey, source);
            assert_close(result, grey, "saturation over grey");
        }
    }

    /// Table 135's NOTE 4: `Luminosity` "produces an inverse effect to that of the Color
    /// mode" — `Color` takes the backdrop's luminosity and the source's colour, and
    /// `Luminosity` with the operands exchanged is the same thing.
    #[test]
    fn luminosity_is_colour_with_the_operands_exchanged() {
        let (backdrop, source) = (rgb(0.2, 0.6, 0.9), rgb(0.8, 0.3, 0.1));
        assert_close(
            NonSeparable::Luminosity.blend(backdrop, source),
            NonSeparable::Color.blend(source, backdrop),
            "luminosity against colour",
        );
    }

    /// §11.3.5.3's rule for the black component is what these four functions already return
    /// on a **neutral** pair, which is what a four-component page's second raster holds.
    ///
    /// > For the K component, the result shall be the K component of Cb for the Hue ,
    /// > Saturation , and Color blend modes; it shall be the K component of Cs for the
    /// > Luminosity blend mode.
    ///
    /// 200 000 pairs — 500 backdrop levels against 400 source levels, spread over the whole
    /// range so that the `Lum` weights and `SetSat`'s `if Cmax > Cmin` are met at every
    /// magnitude rather than at a few round numbers. The bound is the *residue of the three
    /// weights*: `0.3 + 0.59 + 0.11` is not exactly 1 in binary floating point, so `Lum` of a
    /// neutral colour is its level times `1 ± 6 × 10⁻⁸` and the identity is exact only up to
    /// that. Reported as the worst gap seen rather than asserted at an invented tolerance.
    /// [`super::NonSeparable::blend`] carries the derivation; ADR 0277 carries the consequence.
    #[test]
    fn the_clauses_own_functions_give_the_black_components_rule_on_a_neutral_pair() {
        let mut worst = 0.0f32;
        for back in 0..500u32 {
            let b = f32::from(u16::try_from(back).unwrap_or(0)) / 499.0;
            for src in 0..400u32 {
                let s = f32::from(u16::try_from(src).unwrap_or(0)) / 399.0;
                let (backdrop, source) = (rgb(b, b, b), rgb(s, s, s));
                for (mode, expected) in [
                    (NonSeparable::Hue, b),
                    (NonSeparable::Saturation, b),
                    (NonSeparable::Color, b),
                    (NonSeparable::Luminosity, s),
                ] {
                    let result = mode.blend(backdrop, source);
                    for channel in [result.r, result.g, result.b] {
                        worst = worst.max((channel - expected).abs());
                    }
                }
            }
        }
        assert!(
            worst < 1e-6,
            "the black component's rule is the clause's own functions on a neutral pair: \
             worst gap {worst:e} over 200 000 pairs"
        );
    }

    /// A separable mode is not this module's, and saying so is what keeps the twelve
    /// per-channel formulas in one place rather than two. The recognition is also what
    /// makes [`super::NonSeparable::blend`] total, so this is the only place a separable
    /// mode is named at all.
    #[test]
    fn the_four_modes_are_recognised_and_no_others() {
        for mode in test_scenes::ALL_BLEND_MODES {
            assert_eq!(
                NonSeparable::of(mode).is_some(),
                !mode.is_separable(),
                "{mode:?}"
            );
        }
        assert_eq!(NonSeparable::of(BlendMode::Multiply), None);
    }

    /// `SetLum` puts the luminosity exactly where it was asked to, including where
    /// `ClipColor` had to intervene — which is the invariant the three broken modes lost.
    #[test]
    fn set_lum_reaches_the_luminosity_it_was_given() {
        for c in [rgb(1.0, 0.0, 0.0), rgb(0.2, 0.4, 0.6), rgb(0.9, 0.9, 0.1)] {
            for target in [0.0, 0.11, 0.5, 0.93, 1.0] {
                let result = set_lum(c, target);
                assert!(
                    (lum(result) - target).abs() < 1e-4,
                    "{c:?} to {target}: {result:?}"
                );
            }
        }
    }
}
