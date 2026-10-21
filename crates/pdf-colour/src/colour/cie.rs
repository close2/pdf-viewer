//! CIE XYZ, `Lab`, Bradford adaptation and the sRGB transfer: the one route from XYZ to the screen.

use pdf_render::Color;

use super::device_cmyk::channel;

/// The inverse of the L*a*b* companding function.
fn expand(value: f32) -> f32 {
    if value >= 6.0 / 29.0 {
        value * value * value
    } else {
        108.0 / 841.0 * (value - 4.0 / 29.0)
    }
}

/// Converts CIE L*a*b* to D50 XYZ, for callers outside this module.
#[expect(
    clippy::many_single_char_names,
    reason = "a*, b*, and the intermediate m, l, n are the formula's own names"
)]
pub(crate) fn lab_to_xyz(lightness: f32, a: f32, b: f32) -> [f32; 3] {
    let m = (lightness.clamp(0.0, 100.0) + 16.0) / 116.0;
    let l = m + a / 500.0;
    let n = m - b / 200.0;
    [0.964_2 * expand(l), expand(m), 0.824_9 * expand(n)]
}

/// The CIE D50 white point, which is the ICC profile connection space's own.
///
/// Every CIE-based colour in this module is adapted to it before the one matrix that turns
/// an XYZ into a pixel, so it is the single hinge the whole colour path turns on.
pub(crate) const D50: [f32; 3] = [0.964_2, 1.0, 0.824_9];

/// The Bradford cone response matrix, and its inverse.
///
/// ISO 32000-2 §10.3.1 says conversion from a CIE-based source to the destination shall be
/// performed based on the appropriate ICC specification, and that specification's media-relative
/// colorimetric intent adapts the source's white point onto the connection space's D50.
/// Bradford is the transform ICC's own `chad` tag carries, so this is the adaptation the
/// referenced standard describes rather than a choice made here.
///
/// **This sentence is prose rather than a quotation** because the words it would quote are
/// retired (ADR 0484): Errata Collection 3's Issue
/// #181 (`Review`/`Completed`) strikes the dated *ISO 15076-1:2010 (ICC.1:2010)* out of §10.3.1
/// and puts the appropriate ICC specification, with a pointer to Table 66, in its place — so
/// `icc.rs` accepting both 2.x and 4.x profile headers is what the amended sentence asks for
/// where the dated one named a single edition. `doc/errata-read.md` has the row; `spec-errata
/// emit` files it under §10.4.1's heading and the caret's `/Rect` is over §10.3.1's last line.
///
/// Both matrices are the published constants, row-major.
#[rustfmt::skip]
const BRADFORD: [f32; 9] = [
     0.895_1,  0.266_4, -0.161_4,
    -0.750_2,  1.713_5,  0.036_7,
     0.038_9, -0.068_5,  1.029_6,
];

/// The inverse of [`BRADFORD`], row-major.
#[rustfmt::skip]
const BRADFORD_INVERSE: [f32; 9] = [
     0.986_992_9, -0.147_054_3,  0.159_962_7,
     0.432_305_3,  0.518_360_3,  0.049_291_2,
    -0.008_528_7,  0.040_042_8,  0.968_486_7,
];

/// Multiplies a row-major 3×3 matrix by a column vector.
pub(super) fn transform(matrix: &[f32; 9], vector: [f32; 3]) -> [f32; 3] {
    let mut out = [0.0f32; 3];
    for (row, output) in out.iter_mut().enumerate() {
        for (column, input) in vector.iter().enumerate() {
            let entry = matrix
                .get(row.saturating_mul(3).saturating_add(column))
                .copied()
                .unwrap_or(0.0);
            *output += entry * input;
        }
    }
    out
}

/// Takes a CIE-based colour from its own XYZ to sRGB: adapt the white point, then convert.
///
/// `white` is the source space's own diffuse white, which the adaptation maps onto the
/// connection space's D50. Both `CalGray` and `CalRGB` end here, which is why the two share
/// every step after their own decoding stage.
///
/// # `BlackPoint` is read and deliberately not applied
///
/// A Cal space's `BlackPoint` is its *source's* diffuse shadow — §8.6.5.3 says outright that
/// it "is limited by the dynamic range of the input device" and "varies with exposure,
/// system response, and artistic intent". Stretching the range so that shadow lands on the
/// display's black is black point compensation, and ISO 32000-2 §8.6.5.9 makes that a
/// processor decision: `ON` means "according to the provisions in ISO 18619", `OFF` means
/// none, and `Default` — which is what every document in the corpus leaves it at — is
/// "left to the PDF processor to determine".
///
/// So this is a choice, not a derivation, and the choice is to reproduce the colorimetry the
/// space states. Two things decided it:
///
/// - The stretch is **undefined on input the specification permits**. Table 63 requires only
///   that the three numbers be non-negative — nothing puts the black below the white.
///   `calrgb.pdf` page 14 states `BlackPoint [0.2 1.0 1.7]` against `WhitePoint [1 1 1]`, so
///   the Y axis has zero span and the Z axis a negative one. A construction that has to be
///   guarded into doing nothing on two of three axes is not the construction the clause
///   means, and what it does on the third is arbitrary.
/// - The quantity is **not the one `icc.rs` compensates**, which is why that path keeps its
///   compensation and this one has none. There the black point is *measured* from the
///   profile — the darkest colour the device can actually reach — and aligning it is what
///   `PDF20_AN001-BPC` argues for. A source's stated shadow is a different quantity that
///   happens to share a name.
/// - And the procedure §8.6.5.9 defers to says so itself. ISO/CD 18619 (2013) section 4.1
///   takes two ICC profiles and a rendering intent as the whole of its input, and constrains
///   both profiles to be ICC profiles of one of four colour spaces; a `CalRGB` dictionary's
///   three numbers are not among its inputs at any point. So this is no longer only a
///   choice about an undefined stretch — the operation `ON` names has nothing here to
///   operate on (ADR 1253).
///
/// The cost, stated plainly: a document raising its `BlackPoint` gets shadows at the
/// lightness it states rather than stretched down to the display's black. **This paragraph
/// named `calgray.pdf` page 3 and `calrgb.pdf` page 14 as "the corpus's only examples" and
/// there are eleven**, all in those same two files, which are written to probe this entry
/// rather than to display anything — `cargo run --release -p pdf-model --example
/// black_point_census` is what counts them, so that the number is not written here again.
///
/// All three reference renderers do the same, which is evidence that this is how §8.6.5.2
/// and §8.6.5.3 are commonly read — not the reason for the choice, which is above. **The
/// corpus states that evidence as an A/B rather than as an impression**: `calrgb.pdf` pages
/// 1, 5, 11 and 12 state one `CalRGB` in three of Table 63's four entries and differ only in
/// `/BlackPoint`, at `[0 0 0]`, `[1 1 1]`, `[8 8 8]` and `[50 50 50]`, and this tree's
/// rasters, `poppler`'s, `mupdf`'s and `ghostscript`'s are byte-identical across the four
/// below the header that prints those values. `oracle.rs`'s `CONTRADICTED_CALRGB_TO_SCREEN`
/// has the measurement; `hayro` is the one renderer the entry moves.
#[must_use]
pub fn cie_to_srgb(xyz: [f32; 3], white: [f32; 3]) -> Color {
    xyz_d50_to_srgb(adapt(xyz, white, D50))
}

/// Chromatically adapts an XYZ from one white point onto another.
///
/// Scaling the three cone responses is the von Kries construction; Bradford is which cone
/// responses. `from` maps exactly onto `to`, which is the property everything here relies
/// on: a `CalGray` states its colours as multiples of its own white, so adapting turns them
/// into the same multiples of D50 and the grey stays grey.
pub(super) fn adapt(xyz: [f32; 3], from: [f32; 3], to: [f32; 3]) -> [f32; 3] {
    let source = transform(&BRADFORD, from);
    let destination = transform(&BRADFORD, to);
    let mut cone = transform(&BRADFORD, xyz);
    for (axis, value) in cone.iter_mut().enumerate() {
        // A white point with a zero or negative cone response is not a white point; leaving
        // the axis alone is the only answer that cannot produce an infinity.
        if source[axis].abs() > 1e-9 {
            *value *= destination[axis] / source[axis];
        }
    }
    transform(&BRADFORD_INVERSE, cone)
}

/// Converts a D50 XYZ to sRGB.
///
/// The only place in this crate where an XYZ becomes a pixel. `Lab`, `CalGray`, `CalRGB`
/// and every ICC profile arrive here, and the module documentation says why that matters.
pub(crate) fn xyz_d50_to_srgb(xyz: [f32; 3]) -> Color {
    let [r, g, b] = xyz_d50_to_linear_srgb(xyz);
    Color::rgb(gamma(r), gamma(g), gamma(b))
}

/// Converts a D50 XYZ to *linear* sRGB, unclamped: [`xyz_d50_to_srgb`] before its transfer
/// function.
///
/// XYZ (D50) to linear sRGB is the sRGB primaries' matrix with a Bradford adaptation from
/// D50 to sRGB's own D65 white already folded in, which is why it is not the matrix
/// IEC 61966-2-1 prints. `a_folded_matrix_equals_adapting_then_converting` derives it.
///
/// Unclamped on purpose: a colour outside sRGB's gamut lands outside the unit cube here, and
/// [`super::rgb_route::RgbRoute`] carries this stage as the corners of a grid that interpolates the *map*, so
/// the clamp belongs after the interpolation, where [`gamma`] applies it.
#[expect(
    clippy::many_single_char_names,
    reason = "X, Y, Z and R, G, B are the colour spaces' own axis names"
)]
pub(crate) fn xyz_d50_to_linear_srgb(xyz: [f32; 3]) -> [f32; 3] {
    let (x, y, z) = (xyz[0], xyz[1], xyz[2]);
    let r = 3.134_136 * x - 1.617_036 * y - 0.490_662 * z;
    let g = -0.978_755 * x + 1.916_142 * y + 0.033_454 * z;
    let b = 0.071_95 * x - 0.228_988 * y + 1.405_386 * z;
    [r, g, b]
}

/// Converts CIE L\*a\*b\* to sRGB through XYZ, under the space's own diffuse white.
///
/// Table 64 makes `/WhitePoint` required and gives it no default, and the clause's second
/// stage multiplies by it, so a `Lab` colour's XYZ is relative to that white rather than to
/// D50; it then takes the adaptation every CIE-based space takes onto the connection space
/// (ADRs 0012, 1712). §8.6.5.4's own EXAMPLE states D65.
pub(super) fn lab(lightness: f32, a: f32, b: f32, range: [f32; 4], white: [f32; 3]) -> Color {
    cie_to_srgb(lab_xyz(lightness, a, b, range, white), white)
}

/// The XYZ of a `Lab` colour under its own diffuse white, with §8.6.5.4's ranges applied: the
/// clause's two transformation stages, the second of which multiplies `g(L)`, `g(M)` and
/// `g(N)` by the white point's `X_W`, `Y_W` and `Z_W` — the last line here.
#[expect(
    clippy::many_single_char_names,
    reason = "L*, a*, b*, X, Y and Z are the colour space's own names for its axes; \
              renaming them would make this harder to check against the formulae"
)]
pub(super) fn lab_xyz(
    lightness: f32,
    a: f32,
    b: f32,
    range: [f32; 4],
    white: [f32; 3],
) -> [f32; 3] {
    let lightness = nearest(lightness, (0.0, 100.0));
    let a = nearest(a, (range[0], range[1]));
    let b = nearest(b, (range[2], range[3]));

    let m = (lightness + 16.0) / 116.0;
    let l = m + a / 500.0;
    let n = m - b / 200.0;

    [
        white[0] * expand(l),
        white[1] * expand(m),
        white[2] * expand(n),
    ]
}

/// The nearest value to `value` inside a component's range, which is what §8.6.5.4 and Table
/// 64 ask of a component "falling outside the specified range" and §8.6.5.1 of an initial
/// colour whose 0.0 falls outside it.
///
/// Table 64 states the range as `a_min ≤ a* ≤ a_max` and nothing stops a file writing the pair
/// the other way round; the bounds are taken in either order, so such a space draws its
/// components inside the interval its two numbers span rather than panicking, which is what
/// `f32::clamp` does when its minimum exceeds its maximum. A `NaN` is the range's first bound.
pub(super) fn nearest(value: f32, (low, high): (f32, f32)) -> f32 {
    if value.is_nan() {
        return low;
    }
    let (low, high) = if low <= high {
        (low, high)
    } else {
        (high, low)
    };
    if low.is_nan() || high.is_nan() {
        return value;
    }
    value.clamp(low, high)
}

/// The L*a*b* companding function, the inverse of [`expand`].
fn compand(value: f32) -> f32 {
    if value >= 216.0 / 24389.0 {
        value.cbrt()
    } else {
        841.0 / 108.0 * value + 4.0 / 29.0
    }
}

/// Converts a D50 XYZ to CIE L*a*b*: [`lab_to_xyz`] run backwards, for a profile whose
/// connection space is `Lab` and whose "from CIE" table therefore reads one.
///
/// `a_lab_round_trips_through_xyz` holds the two together.
pub(crate) fn xyz_to_lab(xyz: [f32; 3]) -> [f32; 3] {
    let fx = compand(xyz[0] / D50[0]);
    let fy = compand(xyz[1] / D50[1]);
    let fz = compand(xyz[2] / D50[2]);
    [
        116.0f32.mul_add(fy, -16.0),
        500.0 * (fx - fy),
        200.0 * (fy - fz),
    ]
}

/// The XYZ a `CalRGB` colour's decoded components state: §8.6.5.3's `Matrix`, one column per
/// component, applied to the three gamma-decoded values.
pub(super) fn cal_rgb_xyz(decoded: [f32; 3], matrix: &[f32; 9]) -> [f32; 3] {
    let mut xyz = [0.0f32; 3];
    for (column, input) in decoded.iter().enumerate() {
        for (axis, output) in xyz.iter_mut().enumerate() {
            let entry = matrix
                .get(column.saturating_mul(3).saturating_add(axis))
                .copied()
                .unwrap_or(0.0);
            *output += entry * input;
        }
    }
    xyz
}

/// Converts an sRGB colour to D50 XYZ: [`xyz_d50_to_srgb`] run backwards.
///
/// The inverse of the folded matrix, to the precision it was printed at, and the inverse of
/// the transfer function; `srgb_to_xyz_is_the_inverse_of_xyz_to_srgb` holds the pair to a
/// tenth of a level. This is what a device colour states when it is taken *into* a CIE-based
/// space — §10.3.2 has the processor establish a CIE-based definition for its device spaces,
/// and this processor's is sRGB (ADR 0009) — and it is the one route by which such a colour
/// reaches a profile's `B2A`.
pub(crate) fn srgb_to_xyz_d50(colour: Color) -> [f32; 3] {
    linear_srgb_to_xyz_d50([degamma(colour.r), degamma(colour.g), degamma(colour.b)])
}

/// Converts *linear* sRGB to D50 XYZ: [`srgb_to_xyz_d50`] without its decoding.
///
/// The matrix alone, so that a caller carrying the decoding on a curve of its own has the
/// linear stage by itself — which is what [`super::rgb_route::inward_cube`] separates the conversion into a
/// blending colour space at, and it is the inverse of [`xyz_d50_to_linear_srgb`]'s role in
/// the conversion out. Unclamped, for the same reason that one is.
pub(super) fn linear_srgb_to_xyz_d50(linear: [f32; 3]) -> [f32; 3] {
    let (r, g, b) = (linear[0], linear[1], linear[2]);
    [
        0.436_035_2 * r + 0.385_068_1 * g + 0.143_066_6 * b,
        0.222_481_3 * r + 0.716_877_6 * g + 0.060_610_2 * b,
        0.013_927 * r + 0.097_091_3 * g + 0.714_099_4 * b,
    ]
}

/// The sRGB transfer function.
pub(super) fn gamma(value: f32) -> f32 {
    let value = channel(value);
    if value <= 0.003_130_8 {
        channel(value * 12.92)
    } else {
        channel(1.055 * value.powf(1.0 / 2.4) - 0.055)
    }
}

/// The inverse of the sRGB transfer function: IEC 61966-2-1's decoding of an encoded channel.
pub(super) fn degamma(value: f32) -> f32 {
    let value = channel(value);
    if value <= 0.040_45 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}
