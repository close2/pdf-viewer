//! Evaluating embedded ICC profiles.
//!
//! A third of real documents embed one. Until now they were discarded and the component
//! count used to guess a device space, which is the difference between rendering the
//! colours a document specifies and rendering colours that happen to have the same number
//! of numbers.
//!
//! # Why this is written here rather than taken from a library
//!
//! The part of ICC a PDF reader needs is small and well specified: take *n* components,
//! push them through curves and a multidimensional lookup table, and arrive at a
//! profile connection space. The lookup table is a sampled function interpolated
//! multilinearly — the same construction as a PDF type 0 function, which this crate
//! already implements — so the machinery was largely here already.
//!
//! The alternative was a C library, which `#![forbid(unsafe_code)]` and `CLAUDE.md`'s
//! rule about C dependencies both argue against for something parsing untrusted bytes off
//! a page. See `doc/adr/0009-icc-colour-management.md`.
//!
//! # What is implemented
//!
//! The `A2B0` and `A2B1` transforms in all three encodings that occur — `mft1` and `mft2`
//! from ICC v2, `mAB ` from v4 — and the matrix/curve form that RGB and grey display
//! profiles use instead. Between them these cover every profile in the corpus.
//!
//! **And the other direction, since ADR 0796**: the `B2A1` or `B2A0` transform — "from CIE"
//! in ISO 32000-2 §8.6.5.5's words — in the same three encodings (`mBA ` for v4), which is
//! what a profile used as a *blending* colour space has to carry:
//!
//! > When such a space is used as the blending colour space for a transparency group in the
//! > transparent imaging model (see 11.3.4, "Blending colour space"; 11.4, "Transparency
//! > groups"; and 11.6.6, "Transparency group XObjects"), it shall have both "to CIE" ( AToB )
//! > and "from CIE" ( BToA ) information. This is because the group colour space shall be
//! > used as both the destination for objects being painted within the group and the source
//! > for the group's results.
//!
//! [`Profile::to_device`] is that destination route, and `crate::colour`'s press takes a colour
//! into a four-component `ICCBased` blending space through it (§11.6.6, §11.7.2).
//!
//! Rendering intents beyond picking the `1` table over the `0` table are not modelled; black
//! point compensation is [`Profile::to_rgb_with`]'s, and [`Profile::to_device`] undoes it.

use pdf_render::Color;

/// Largest profile this will parse, in bytes.
///
/// Real profiles run from a few hundred bytes to a few hundred kilobytes; the largest in
/// the corpus is 120 KB. This bounds what a hostile stream can make us allocate.
const MAX_PROFILE: usize = 1 << 24;

/// Largest colour lookup table, in entries.
///
/// A four-input table with 64 grid points would be sixteen million entries. This is the
/// decompression-bomb bound for colour management.
const MAX_CLUT: usize = 1 << 22;

/// A parsed ICC profile, ready to convert colours.
#[derive(Debug, Clone)]
pub struct Profile {
    /// How many components a colour in this profile's space has.
    channels: usize,
    /// Whether the connection space is `Lab` rather than `XYZ`.
    lab_pcs: bool,
    transform: Transform,
    /// The "from CIE" transform — `B2A1`, or `B2A0` — where the profile carries one.
    ///
    /// `None` for a profile that states none, which §8.6.5.5 permits of a profile used as a
    /// *source* space and forbids of one used as a blending space; a press without one keeps
    /// the right inverse of its own `A2B` that every press took before ADR 0796.
    inverse: Option<Box<Lut>>,
    /// The darkest colour this profile's device can make, in connection-space XYZ.
    ///
    /// `None` for a profile whose black is already zero, and for the matrix and grey
    /// forms, where it does not arise.
    black: Option<[f32; 3]>,
    /// What distinguishes this profile's bytes from another's. See [`Profile::identity`].
    identity: u128,
}

/// How a profile gets from its own space to the connection space.
#[derive(Debug, Clone)]
enum Transform {
    /// Curves, a lookup table, and more curves. Used by CMYK and by most v4 profiles.
    Lut(Box<Lut>),
    /// Per-channel curves and a 3×3 matrix to XYZ, which is how display profiles work.
    Matrix {
        curves: Vec<Curve>,
        /// Columns are the red, green and blue colourants in XYZ.
        columns: [[f32; 3]; 3],
    },
    /// A single tone curve to luminance, which is how grey profiles work.
    Grey(Curve),
}

/// A colour lookup table with curves on either side.
///
/// The stages run in this order: [`Lut::matrix`], [`Lut::input`], [`Lut::offsets`],
/// [`Lut::mid`], the table, [`Lut::output`]. A v2 `mft` table uses the first, second, fifth
/// and sixth; a v4 `mAB ` the second, fifth and sixth; a v4 `mBA ` — whose B curves, matrix,
/// M curves, table and A curves come in exactly that order (ISO 15076-1 section 10.11) — all but
/// the first.
#[derive(Debug, Clone)]
struct Lut {
    /// Applied to each input before the table is sampled.
    input: Vec<Curve>,
    /// A `mBA ` tag's 3×3 matrix and three offsets, applied after [`Lut::input`] and only
    /// to three inputs.
    offsets: Option<[f32; 12]>,
    /// A `mBA ` tag's M curves, applied after [`Lut::offsets`] and before the table.
    mid: Vec<Curve>,
    /// Applied to each output after it.
    output: Vec<Curve>,
    /// Grid points per input axis.
    grid: Vec<usize>,
    /// How many values each grid point holds.
    outputs: usize,
    /// The samples, normalised to `0.0..=1.0`, with the *last* input varying fastest.
    samples: Vec<f32>,
    /// A matrix applied before the input curves, used only when the input is XYZ.
    matrix: Option<[f32; 9]>,
    /// How this table's outputs encode the connection space.
    encoding: Encoding,
}

/// A tone curve.
#[derive(Debug, Clone)]
enum Curve {
    /// The identity.
    None,
    /// A pure power law.
    Gamma(f32),
    /// Sampled at even intervals, interpolated linearly.
    Sampled(Vec<f32>),
    /// One of the ICC parametric forms, already reduced to its coefficients.
    ///
    /// The five types are the same function with progressively more terms, so they are
    /// stored as one shape rather than five variants.
    Parametric {
        kind: u16,
        /// `g, a, b, c, d, e, f` — unused terms are zero.
        values: [f32; 7],
    },
}

impl Curve {
    fn apply(&self, x: f32) -> f32 {
        // Only the forms that index a table need their input clamped. `None` must be a
        // true identity: clamping there quietly destroyed connection-space values, which
        // are not confined to `0.0..=1.0` at all.
        if matches!(self, Self::None) {
            return x;
        }
        let x = x.clamp(0.0, 1.0);
        match self {
            Self::None => x,
            Self::Gamma(gamma) => x.powf(*gamma),
            Self::Sampled(points) => {
                let last = points.len().saturating_sub(1);
                if last == 0 {
                    return points.first().copied().unwrap_or(x);
                }
                #[expect(
                    clippy::cast_precision_loss,
                    reason = "curve tables have at most 65535 entries, exact in f32"
                )]
                let scaled = x * last as f32;
                #[expect(
                    clippy::cast_possible_truncation,
                    clippy::cast_sign_loss,
                    reason = "scaled is clamped to 0..=last by x being clamped to 0..=1"
                )]
                let low = scaled.floor() as usize;
                let high = low.saturating_add(1).min(last);
                let t = scaled - scaled.floor();
                let a = points.get(low).copied().unwrap_or(0.0);
                let b = points.get(high).copied().unwrap_or(a);
                a + (b - a) * t
            }
            Self::Parametric { kind, values } => parametric(*kind, values, x),
        }
    }
}

/// The ICC parametric curve types, which are one function with more or fewer terms.
#[expect(
    clippy::many_single_char_names,
    reason = "g, a, b, c, d, e and f are the specification's own names for the \
              coefficients; renaming them would make this unreviewable against it"
)]
fn parametric(kind: u16, v: &[f32; 7], x: f32) -> f32 {
    let (g, a, b, c, d, e, f) = (v[0], v[1], v[2], v[3], v[4], v[5], v[6]);
    let power = |base: f32| {
        if base <= 0.0 { 0.0 } else { base.powf(g) }
    };
    let value = match kind {
        0 => power(x),
        1 => {
            if x >= -b / a {
                power(a * x + b)
            } else {
                0.0
            }
        }
        2 => {
            if x >= -b / a {
                power(a * x + b) + c
            } else {
                c
            }
        }
        3 => {
            if x >= d {
                power(a * x + b)
            } else {
                c * x
            }
        }
        4 => {
            if x >= d {
                power(a * x + b) + e
            } else {
                c * x + f
            }
        }
        // An unknown type is better treated as the identity than as zero: the curve is
        // usually near-linear, and black is never the right guess for a colour.
        _ => x,
    };
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        x
    }
}

/// What [`Profile::identity`] returns: the byte length beside an FNV-1a hash of the bytes.
///
/// FNV-1a because it is four lines a reader can check and needs no dependency; the length is
/// carried in the high half so that two profiles of different sizes can never be confused
/// whatever their hashes do.
fn identity_of(data: &[u8]) -> u128 {
    // The 64-bit FNV-1a offset basis and prime, from the algorithm's own definition.
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in data {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    (u128::try_from(data.len()).unwrap_or(u128::MAX) << 64) | u128::from(hash)
}

/// Reads a big-endian `u16`.
fn u16_at(data: &[u8], at: usize) -> Option<u16> {
    let bytes = data.get(at..at.checked_add(2)?)?;
    Some(u16::from_be_bytes([*bytes.first()?, *bytes.get(1)?]))
}

/// Reads a big-endian `u32`.
fn u32_at(data: &[u8], at: usize) -> Option<u32> {
    let bytes = data.get(at..at.checked_add(4)?)?;
    Some(u32::from_be_bytes([
        *bytes.first()?,
        *bytes.get(1)?,
        *bytes.get(2)?,
        *bytes.get(3)?,
    ]))
}

/// Reads an `s15Fixed16Number`, ICC's fixed-point real.
fn fixed_at(data: &[u8], at: usize) -> Option<f32> {
    let raw = u32_at(data, at)?;
    #[expect(
        clippy::cast_possible_wrap,
        reason = "s15Fixed16 is a signed quantity stored in four bytes"
    )]
    let signed = raw as i32;
    #[expect(
        clippy::cast_precision_loss,
        reason = "the fractional part is 16 bits, well inside f32's mantissa"
    )]
    Some(signed as f32 / 65536.0)
}

impl Profile {
    /// Parses a profile, returning `None` if it is not one this can evaluate.
    ///
    /// Every failure is a `None` rather than an error because the caller always has a
    /// fallback — the profile's `/Alternate` or a device space — and a document with an
    /// unreadable profile should still render.
    #[must_use]
    pub fn parse(data: &[u8]) -> Option<Self> {
        if data.len() > MAX_PROFILE {
            return None;
        }
        // The header is 128 bytes and its signature sits at offset 36.
        if data.get(36..40)? != b"acsp" {
            return None;
        }
        let channels = match data.get(16..20)? {
            b"GRAY" => 1,
            b"RGB " => 3,
            b"CMYK" => 4,
            // Other input spaces exist but do not occur in PDFs.
            _ => return None,
        };
        let lab_pcs = match data.get(20..24)? {
            b"Lab " => true,
            b"XYZ " => false,
            _ => return None,
        };
        // Byte 8 of the header is the major version, which decides how Lab is encoded in
        // the v2 lookup tables.
        let version_4 = data.get(8).copied().unwrap_or(2) >= 4;

        let count = usize::try_from(u32_at(data, 128)?).ok()?;
        if count > 1024 {
            return None;
        }
        let mut tags = Vec::with_capacity(count);
        for index in 0..count {
            let at = 132usize.checked_add(index.checked_mul(12)?)?;
            let signature = data.get(at..at.checked_add(4)?)?.to_vec();
            let offset = usize::try_from(u32_at(data, at.checked_add(4)?)?).ok()?;
            let length = usize::try_from(u32_at(data, at.checked_add(8)?)?).ok()?;
            tags.push((signature, offset, length));
        }
        let find = |name: &[u8]| {
            tags.iter()
                .find(|(signature, ..)| signature == name)
                .and_then(|(_, offset, length)| data.get(*offset..offset.checked_add(*length)?))
        };

        // A perceptual or relative-colorimetric lookup table is the general form; the
        // matrix and curve tags are the shorthand display profiles use instead.
        // `A2B1` is the relative-colorimetric table, which is PDF's default rendering
        // intent; `A2B0` is perceptual and is only the fallback. Taking them the other way
        // round renders every dark colour too light — this profile's registration black
        // came out at (28,27,23) where every other reader shows (0,0,0).
        let transform = if let Some(table) = find(b"A2B1").or_else(|| find(b"A2B0")) {
            Transform::Lut(Box::new(parse_lut(table, lab_pcs, version_4, false)?))
        } else if channels == 3 {
            let curves = [b"rTRC", b"gTRC", b"bTRC"]
                .into_iter()
                .map(|name| find(name).and_then(parse_curve).unwrap_or(Curve::None))
                .collect();
            let column = |name: &[u8]| -> Option<[f32; 3]> {
                let tag = find(name)?;
                Some([fixed_at(tag, 8)?, fixed_at(tag, 12)?, fixed_at(tag, 16)?])
            };
            let (r, g, b) = (column(b"rXYZ")?, column(b"gXYZ")?, column(b"bXYZ")?);
            Transform::Matrix {
                curves,
                columns: [r, g, b],
            }
        } else if channels == 1 {
            Transform::Grey(find(b"kTRC").and_then(parse_curve).unwrap_or(Curve::None))
        } else {
            return None;
        };

        // The "from CIE" table, by the same ranking as the "to CIE" one: `B2A1` is relative
        // colorimetric, which is PDF's default rendering intent, and `B2A0` is the fallback.
        // A table this cannot read costs the profile nothing it had — the conversion out is
        // the profile's whatever the conversion in is — so a refusal here is a `None` and
        // not a failed parse.
        let inverse = find(b"B2A1")
            .or_else(|| find(b"B2A0"))
            .and_then(|table| parse_lut(table, lab_pcs, version_4, true))
            .filter(|table| table.grid.len() == 3 && table.outputs >= channels)
            .map(Box::new);

        let mut profile = Self {
            channels,
            lab_pcs,
            transform,
            inverse,
            black: None,
            identity: identity_of(data),
        };
        profile.black = profile.detect_black();
        Some(profile)
    }

    /// Finds the darkest colour the device can produce, for black point compensation.
    ///
    /// A press cannot make a colour as dark as a screen's black, so a profile's darkest
    /// output is a very dark grey rather than zero. Reproducing that literally leaves every
    /// black on the page washed out — this profile's registration black comes out at
    /// (28,27,23), a visibly grey "black".
    ///
    /// ISO 32000-2 §8.6.5.9 is about exactly this: `/UseBlackPtComp` in the graphics state
    /// takes `ON`, `OFF` or `Default`, `ON` means "according to the provisions in
    /// ISO 18619", and `Default` — the initial value — is "left to the PDF processor to
    /// determine". Compensating by default is therefore a choice the specification provides
    /// for, and the cases where it is *not* permitted are honoured by
    /// [`Self::to_rgb_with`].
    ///
    /// What compensation must achieve is defined without ambiguity, and not only by
    /// ISO 18619. PDF 2.0 Application Note 001 (`doc/md/PDF20_AN001-BPC.md`), written by
    /// ISO 32000's own co-project-leader to interpret this feature, states it as "aligning
    /// the darkest colour that could be described by the colour space of the data to be
    /// displayed with the darkest colour that the output profile for the display device
    /// (screen or print) can produce". That sentence is what the code below implements, and
    /// it settles the design question the arithmetic cannot: the black to be aligned is the
    /// one *the source colour space* describes, which is why it is found at the end of this
    /// profile's own device range rather than read off the display's.
    ///
    /// The same note observes that BPC is "very similar to what switching between absolute
    /// and relative colorimetric rendering intents does at the highlight end" — the reason
    /// [`Self::to_rgb_with`] must refuse to compensate under `AbsoluteColorimetric`.
    ///
    /// The *arithmetic* by which the alignment is done is ISO 18619's, and that is a
    /// normative reference this project does not hold; a linear mapping between the two
    /// black points meets the stated goal, but it is not a transcription of the standard.
    /// Worth knowing if the numbers in the last few percent ever have to be defended.
    ///
    /// The device range is walked rather than a `bkpt` tag read, because the CMYK profiles
    /// that need this most often carry no such tag, and one that is absent cannot be
    /// honoured.
    ///
    /// This yields a *colorimetric* black point: the darkest colour the profile itself says
    /// the space reaches, which is what the application note's wording asks for. An
    /// alternative construction estimates a *perceptual* one by round-tripping through the
    /// profile's `B2A` table; readers built on Little CMS take that route, and the two agree
    /// everywhere except in the darkest few percent. Recorded because it explains a residual
    /// disagreement that comes from a choice of construction rather than a free parameter —
    /// there is nothing here to tune.
    fn detect_black(&self) -> Option<[f32; 3]> {
        if !matches!(self.transform, Transform::Lut(_)) {
            return None;
        }
        // **Which end of the device range is dark is a property of the space, not of the
        // profile**, and this read the subtractive answer for every space until the
        // six-hundred-and-fifteenth session. Full ink is black in `CMYK`, `2CLR`.. `FCLR`
        // and a `Separation`'s colourant; in `RGB` and `GRAY` — ICC's own header signatures
        // for the additive spaces — every component at 1.0 is *white*. So an RGB scanner
        // profile had its white point taken as its black, and where the two differed by a
        // rounding — which is every profile whose white corner is not D50 to the bit — the
        // span below was a thousandth of a unit wide and stretched the whole page onto
        // black. `2268885.pdf` in the `SafeDocs` crawl is a floor plan drawn as a negative for
        // that reason, with nothing reported.
        //
        // Both ends are evaluated and the darker taken, which needs no table of which
        // signature is which and is what the application note's wording asks for directly:
        // the darkest colour the source space describes. `Y` is luminance, so it is the axis
        // that decides.
        let black = [vec![0.0f32; self.channels], vec![1.0f32; self.channels]]
            .into_iter()
            .map(|end| self.connection(&end))
            .min_by(|left, right| {
                left.get(1)
                    .unwrap_or(&0.0)
                    .total_cmp(right.get(1).unwrap_or(&0.0))
            })?;
        // Compensation aligns a *range*, so it needs one: the colour found has to be
        // darker than the white it is being stretched away from, on every axis. A profile
        // whose fullest ink is no darker than its white describes no range to align — and
        // stretching one anyway divides by a span at or below zero, which does not produce
        // a slightly wrong colour but an arbitrary one.
        let usable = black
            .iter()
            .zip(WHITE)
            .all(|(value, white)| *value < white && *value > 1e-4);
        usable.then_some(black)
    }

    /// The connection-space XYZ a colour maps to, before compensation or transfer.
    fn connection(&self, values: &[f32]) -> [f32; 3] {
        let raw = match &self.transform {
            Transform::Lut(lut) => {
                let out = lut.apply(values, self.channels);
                lut.encoding.decode([out[0], out[1], out[2]])
            }
            Transform::Matrix { curves, columns } => {
                let mut xyz = [0.0f32; 3];
                for (index, curve) in curves.iter().enumerate().take(3) {
                    let linear = curve.apply(values.get(index).copied().unwrap_or(0.0));
                    for (axis, out) in xyz.iter_mut().enumerate() {
                        *out += linear
                            * columns
                                .get(index)
                                .and_then(|column| column.get(axis))
                                .copied()
                                .unwrap_or(0.0);
                    }
                }
                return xyz;
            }
            Transform::Grey(curve) => {
                let y = curve.apply(values.first().copied().unwrap_or(0.0));
                return [y * WHITE[0], y * WHITE[1], y * WHITE[2]];
            }
        };
        if self.lab_pcs {
            crate::colour::lab_to_xyz(raw[0], raw[1], raw[2])
        } else {
            raw
        }
    }

    /// How many components a colour in this profile's space has.
    #[must_use]
    pub fn channels(&self) -> usize {
        self.channels
    }

    /// What tells this profile's bytes apart from another's.
    ///
    /// Two profiles parsed from the same bytes have the same identity, and that is the whole
    /// contract: `crate::colour`'s press registry uses it to recognise a profile it has
    /// already sampled, and §8.6.5.7's implicit conversion uses it to recognise the space a
    /// colour is already *in* — "in the case of 4 component colour spaces avoids the
    /// conversion from 4 components to 3 and back to 4, a process that loses critical colour
    /// information".
    ///
    /// It is the profile's length beside a 64-bit FNV-1a of its bytes rather than a
    /// cryptographic digest, because what it has to survive is a document's own profiles being
    /// distinguished from each other, not an adversary constructing a pair. Two *different*
    /// profiles of the same length whose hashes also collide would be treated as one press;
    /// at 2⁻⁶⁴ per pair and a handful of profiles per document, that is a stated bound rather
    /// than an assumed impossibility.
    #[must_use]
    pub fn identity(&self) -> u128 {
        self.identity
    }

    /// Converts a colour in this profile's space to sRGB, with black point compensation.
    #[must_use]
    pub fn to_rgb(&self, values: &[f32]) -> Color {
        self.to_rgb_with(values, true)
    }

    /// Converts a colour, choosing whether to compensate for the black point.
    #[must_use]
    pub fn to_rgb_with(&self, values: &[f32], black_point: bool) -> Color {
        crate::colour::xyz_d50_to_srgb(self.to_xyz_with(values, black_point))
    }

    /// The D50 XYZ a colour in this profile's space becomes, compensated or not.
    ///
    /// What [`Self::to_rgb_with`] hands the one matrix that turns an XYZ into a pixel, and
    /// what a caller converting *between* CIE-based spaces wants before that matrix — a
    /// colour going into another profile's [`Self::to_device`] has no business passing
    /// through sRGB's gamut on the way.
    #[must_use]
    pub fn to_xyz_with(&self, values: &[f32], black_point: bool) -> [f32; 3] {
        let mut xyz = self.connection(values);

        // Black point compensation: stretch the profile's range so its darkest colour
        // lands on the display's black instead of on a dark grey. Linear in XYZ, with the
        // white point fixed, which is the standard construction.
        if let Some(black) = self.black.filter(|_| black_point) {
            for (axis, value) in xyz.iter_mut().enumerate() {
                let white = WHITE.get(axis).copied().unwrap_or(1.0);
                let low = black.get(axis).copied().unwrap_or(0.0);
                let span = white - low;
                // Positive by construction — `detect_black` refuses anything else — but
                // the division is guarded rather than assumed, since the alternative is
                // silently emitting infinities into the page.
                if span > 1e-9 {
                    *value = (*value - low) / span * white;
                }
            }
        }
        xyz
    }

    /// Whether this profile carries the "from CIE" information §8.6.5.5 asks of a blending
    /// colour space — a `B2A1` or `B2A0` table this crate can evaluate.
    #[must_use]
    pub fn is_bidirectional(&self) -> bool {
        self.inverse.is_some()
    }

    /// The device colour that reproduces a D50 XYZ, through the profile's `B2A` table.
    ///
    /// This is the conversion *into* an `ICCBased` blending colour space — the destination
    /// half of §8.6.5.5's sentence quoted in the module header — and it is the profile's
    /// own answer rather than a search over its `A2B`: ISO 32000-2 §10.3.1 hands a
    /// CIE-to-CIE conversion to the ICC specification, and a `B2A` table is that
    /// specification's statement of which device colour a connection-space colour becomes,
    /// gamut mapping included. Where the colour lies outside the device's gamut the table's
    /// answer is the profile writer's mapping, which is what §11.7.5.3 means by the rendering
    /// intent "taking into account the target space's colour gamut".
    ///
    /// `black_point` says whether `xyz` was produced *with* [`Self::to_rgb_with`]'s black
    /// point compensation, in which case the stretch is undone first so that the two
    /// directions are inverses in the same sense: a colour taken out of the press with the
    /// compensation on and brought back in lands on the device colour it came from, to the
    /// profile's own round-trip precision.
    ///
    /// The result has the profile's channel count and zeros beyond it. `None` where the
    /// profile carries no table this crate reads, which [`Self::is_bidirectional`] says in
    /// advance.
    #[must_use]
    pub fn to_device(&self, xyz: [f32; 3], black_point: bool) -> Option<[f32; MAX_OUTPUTS]> {
        let inverse = self.inverse.as_ref()?;
        let mut xyz = xyz;
        if let Some(black) = self.black.filter(|_| black_point) {
            for (axis, value) in xyz.iter_mut().enumerate() {
                let white = WHITE.get(axis).copied().unwrap_or(1.0);
                let low = black.get(axis).copied().unwrap_or(0.0);
                let span = white - low;
                if span > 1e-9 && white.abs() > 1e-9 {
                    *value = *value / white * span + low;
                }
            }
        }
        let pcs = if self.lab_pcs {
            crate::colour::xyz_to_lab(xyz)
        } else {
            xyz
        };
        let mut out = inverse.apply(&inverse.encoding.encode(pcs), 3);
        for value in out.iter_mut().skip(self.channels) {
            *value = 0.0;
        }
        Some(out)
    }
}

/// Largest number of output channels a table is evaluated for.
///
/// Four: PDF's `/N` permits one, three or four, and a table with more outputs — ICC permits
/// fifteen — has the rest unread, which costs nothing this crate could consume.
pub const MAX_OUTPUTS: usize = 4;

/// Largest number of input channels a table is evaluated for without allocating.
///
/// ICC permits fifteen; PDF's `/N` permits one, three or four. The bound is the format's
/// rather than PDF's so that a profile outside PDF's range is still evaluated, and it is a
/// bound rather than a `Vec` because this runs **once per pixel** of an image in an
/// `ICCBased` space. The five `Vec`s the first version allocated per call cost 900
/// instructions a pixel, which is 3.1 G on one 2500x1364 photograph — twenty times the rest
/// of that page put together.
const MAX_INPUTS: usize = 15;

impl Lut {
    /// Runs a colour through the input curves, the table and the output curves.
    ///
    /// The output holds [`MAX_OUTPUTS`] components, of which a "to CIE" table fills three —
    /// the connection space has three, and [`Encoding::decode`] reads exactly that many — and
    /// a "from CIE" table fills the device's count.
    fn apply(&self, values: &[f32], channels: usize) -> [f32; MAX_OUTPUTS] {
        let count = channels.min(MAX_INPUTS);
        let mut inputs = [0.0f32; MAX_INPUTS];
        for (index, slot) in inputs.iter_mut().enumerate().take(count) {
            *slot = values.get(index).copied().unwrap_or(0.0).clamp(0.0, 1.0);
        }

        // A `mft` table's matrix applies only to an XYZ input space, which for a "to CIE"
        // table a PDF never has; it is read and applied anyway so that a profile carrying one
        // is not silently ignored. For a "from CIE" table the input *is* the connection
        // space, and the parser drops the matrix where that space is Lab.
        if let Some(matrix) = &self.matrix
            && count == 3
        {
            let mut out = [0.0f32; 3];
            for (row, value) in out.iter_mut().enumerate() {
                for (column, input) in inputs.iter().enumerate().take(3) {
                    *value += matrix
                        .get(row.saturating_mul(3).saturating_add(column))
                        .copied()
                        .unwrap_or(0.0)
                        * input;
                }
            }
            for (slot, value) in inputs.iter_mut().zip(out) {
                *slot = value;
            }
        }

        for (index, value) in inputs.iter_mut().enumerate().take(count) {
            if let Some(curve) = self.input.get(index) {
                *value = curve.apply(*value);
            }
        }

        // A `mBA ` tag's matrix and M curves, between the B curves and the table. The
        // matrix is three rows of three with an offset each, over three inputs only, and its
        // result is clamped like every other stage's input — ISO 15076-1 section 10.11 has the
        // stages produce values in the table's own encoding.
        if let Some(offsets) = &self.offsets
            && count == 3
        {
            let mut out = [0.0f32; 3];
            for (row, value) in out.iter_mut().enumerate() {
                for (column, input) in inputs.iter().enumerate().take(3) {
                    *value += offsets
                        .get(row.saturating_mul(3).saturating_add(column))
                        .copied()
                        .unwrap_or(0.0)
                        * input;
                }
                *value += offsets
                    .get(9usize.saturating_add(row))
                    .copied()
                    .unwrap_or(0.0);
            }
            for (slot, value) in inputs.iter_mut().zip(out) {
                *slot = value.clamp(0.0, 1.0);
            }
        }
        for (index, value) in inputs.iter_mut().enumerate().take(count) {
            if let Some(curve) = self.mid.get(index) {
                *value = curve.apply(*value);
            }
        }

        let mut out = self.sample(inputs.get(..count).unwrap_or_default());
        for (index, value) in out.iter_mut().enumerate() {
            if let Some(curve) = self.output.get(index) {
                *value = curve.apply(*value);
            }
        }
        out
    }

    /// Samples the table, interpolating multilinearly between grid points.
    fn sample(&self, inputs: &[f32]) -> [f32; MAX_OUTPUTS] {
        let dimensions = self.grid.len();
        let mut result = [0.0f32; MAX_OUTPUTS];
        if dimensions > MAX_INPUTS {
            return result;
        }
        let mut base = [0usize; MAX_INPUTS];
        let mut fraction = [0.0f32; MAX_INPUTS];
        for (index, points) in self.grid.iter().enumerate() {
            #[expect(
                clippy::cast_precision_loss,
                reason = "grid sizes are bounded by MAX_CLUT"
            )]
            let last = points.saturating_sub(1) as f32;
            let position = (inputs.get(index).copied().unwrap_or(0.0) * last).clamp(0.0, last);
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "position is clamped to 0..=last"
            )]
            let floor = position.floor() as usize;
            if let Some(slot) = base.get_mut(index) {
                *slot = floor.min(points.saturating_sub(1));
            }
            if let Some(slot) = fraction.get_mut(index) {
                *slot = position - position.floor();
            }
        }

        let Some(corners) = 1usize.checked_shl(u32::try_from(dimensions).unwrap_or(32)) else {
            return result;
        };

        for corner in 0..corners {
            let mut weight = 1.0f32;
            let mut offset = 0usize;
            for (dimension, points) in self.grid.iter().enumerate() {
                let up = corner
                    .checked_shr(u32::try_from(dimension).unwrap_or(0))
                    .unwrap_or(0)
                    & 1;
                let f = fraction.get(dimension).copied().unwrap_or(0.0);
                weight *= if up == 1 { f } else { 1.0 - f };
                let index = base
                    .get(dimension)
                    .copied()
                    .unwrap_or(0)
                    .saturating_add(up)
                    .min(points.saturating_sub(1));
                // The *last* input varies fastest in an ICC table, which is the opposite
                // of a PDF sampled function. Reading it the other way round produces a
                // colour that is smooth, plausible and wrong.
                offset = offset.saturating_mul(*points).saturating_add(index);
            }
            if weight == 0.0 {
                continue;
            }
            for (component, value) in result.iter_mut().enumerate().take(self.outputs) {
                let at = offset
                    .saturating_mul(self.outputs)
                    .saturating_add(component);
                *value += weight * self.samples.get(at).copied().unwrap_or(0.0);
            }
        }
        result
    }
}

/// The D50 white point, which is the connection space's own.
const WHITE: [f32; 3] = crate::colour::D50;

/// Parses a `curv` or `para` tag.
fn parse_curve(tag: &[u8]) -> Option<Curve> {
    match tag.get(..4)? {
        b"curv" => {
            let count = usize::try_from(u32_at(tag, 8)?).ok()?;
            match count {
                0 => Some(Curve::None),
                // A single entry is a u8Fixed8 gamma.
                1 => Some(Curve::Gamma(f32::from(u16_at(tag, 12)?) / 256.0)),
                _ => {
                    if count > 1 << 17 {
                        return None;
                    }
                    let mut points = Vec::with_capacity(count);
                    for index in 0..count {
                        let at = 12usize.checked_add(index.checked_mul(2)?)?;
                        points.push(f32::from(u16_at(tag, at)?) / 65535.0);
                    }
                    Some(Curve::Sampled(points))
                }
            }
        }
        b"para" => {
            let kind = u16_at(tag, 8)?;
            // Types 0 to 4 take one, three, four, five and seven parameters.
            let needed = match kind {
                0 => 1,
                1 => 3,
                2 => 4,
                3 => 5,
                4 => 7,
                _ => return Some(Curve::None),
            };
            let mut values = [0.0f32; 7];
            for (index, slot) in values.iter_mut().enumerate().take(needed) {
                *slot = fixed_at(tag, 12usize.checked_add(index.checked_mul(4)?)?)?;
            }
            Some(Curve::Parametric { kind, values })
        }
        _ => None,
    }
}

/// Parses an `A2B` or `B2A` tag in any of its three encodings.
///
/// `from_pcs` says which: a "from CIE" table's *inputs* are the connection space, which
/// decides what its `mft` matrix applies to and which v4 tag type it is stored as.
fn parse_lut(tag: &[u8], lab_pcs: bool, version_4: bool, from_pcs: bool) -> Option<Lut> {
    match (tag.get(..4)?, from_pcs) {
        (b"mft1", _) => parse_mft(tag, 1, lab_pcs, version_4, from_pcs),
        (b"mft2", _) => parse_mft(tag, 2, lab_pcs, version_4, from_pcs),
        (b"mAB ", false) => parse_mab(tag, lab_pcs),
        (b"mBA ", true) => parse_mba(tag, lab_pcs),
        _ => None,
    }
}

/// Parses the v2 `lut8Type` and `lut16Type` tags, which differ only in sample width.
fn parse_mft(
    tag: &[u8],
    width: usize,
    lab_pcs: bool,
    version_4: bool,
    from_pcs: bool,
) -> Option<Lut> {
    let inputs = usize::from(*tag.get(8)?);
    let outputs = usize::from(*tag.get(9)?);
    let points = usize::from(*tag.get(10)?);
    if inputs == 0 || outputs == 0 || points < 2 || inputs > 8 {
        return None;
    }

    let mut matrix = [0.0f32; 9];
    for (index, slot) in matrix.iter_mut().enumerate() {
        *slot = fixed_at(tag, 12usize.checked_add(index.checked_mul(4)?)?)?;
    }
    // The identity is the overwhelmingly common case and applying it costs nine
    // multiplications per colour, so it is dropped rather than carried.
    // Exact comparison is intended: the identity is what a profile writes when it means
    // "no matrix", byte for byte, so anything else really is a matrix to apply.
    let identity = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];
    #[expect(
        clippy::float_cmp,
        reason = "detecting the literal identity a profile writes, not comparing results"
    )]
    let matrix = (matrix != identity).then_some(matrix);
    // ISO 15076-1 section 10.10: the matrix "shall only be used when the input colour space is
    // PCSXYZ". A "from CIE" table's input is the connection space, so the question has an
    // answer here: it applies for an XYZ connection space and not for a Lab one.
    let matrix = matrix.filter(|_| !(from_pcs && lab_pcs));

    // `lut8` has fixed 256-entry tables; `lut16` states its own sizes.
    let (input_entries, output_entries, mut at) = if width == 1 {
        (256usize, 256usize, 48usize)
    } else {
        (
            usize::from(u16_at(tag, 48)?),
            usize::from(u16_at(tag, 50)?),
            52usize,
        )
    };
    if input_entries < 2 || output_entries < 2 {
        return None;
    }

    let read = |data: &[u8], at: usize| -> Option<f32> {
        if width == 1 {
            Some(f32::from(*data.get(at)?) / 255.0)
        } else {
            Some(f32::from(u16_at(data, at)?) / 65535.0)
        }
    };

    let mut input = Vec::with_capacity(inputs);
    for _ in 0..inputs {
        let mut points = Vec::with_capacity(input_entries);
        for index in 0..input_entries {
            points.push(read(tag, at.checked_add(index.checked_mul(width)?)?)?);
        }
        at = at.checked_add(input_entries.checked_mul(width)?)?;
        input.push(Curve::Sampled(points));
    }

    let total = points
        .checked_pow(u32::try_from(inputs).ok()?)?
        .checked_mul(outputs)?;
    if total > MAX_CLUT {
        return None;
    }
    let mut samples = Vec::with_capacity(total);
    for index in 0..total {
        samples.push(read(tag, at.checked_add(index.checked_mul(width)?)?)?);
    }
    at = at.checked_add(total.checked_mul(width)?)?;

    let mut output = Vec::with_capacity(outputs);
    for _ in 0..outputs {
        let mut points = Vec::with_capacity(output_entries);
        for index in 0..output_entries {
            points.push(read(tag, at.checked_add(index.checked_mul(width)?)?)?);
        }
        at = at.checked_add(output_entries.checked_mul(width)?)?;
        output.push(Curve::Sampled(points));
    }

    let encoding = match (lab_pcs, width == 2 && !version_4) {
        (false, _) => Encoding::Xyz,
        (true, true) => Encoding::LabLegacy,
        (true, false) => Encoding::Lab,
    };
    Some(Lut {
        input,
        offsets: None,
        mid: Vec::new(),
        output,
        grid: vec![points; inputs],
        outputs,
        samples,
        matrix,
        encoding,
    })
}

/// Parses the v4 `lutBToAType` tag: B curves, a matrix, M curves, the table, A curves.
///
/// The mirror image of [`parse_mab`] — the same five offsets in the same header slots, with
/// the stages run the other way round, which is why the B curves are this table's *input*
/// and the A curves its output. Unlike the "to CIE" parser, the matrix and M curves are
/// modelled rather than refused: a "from CIE" table's input is the connection space, and a
/// matrix over it is how a v4 profile shapes Lab or XYZ before the table.
fn parse_mba(tag: &[u8], lab_pcs: bool) -> Option<Lut> {
    let inputs = usize::from(*tag.get(8)?);
    let outputs = usize::from(*tag.get(9)?);
    if inputs != 3 || outputs == 0 || outputs > MAX_INPUTS {
        return None;
    }

    let offset = |at: usize| -> Option<usize> { usize::try_from(u32_at(tag, at)?).ok() };
    let (b_at, matrix_at, m_at, clut_at, a_at) = (
        offset(12)?,
        offset(16)?,
        offset(20)?,
        offset(24)?,
        offset(28)?,
    );

    let curves = |start: usize, count: usize| -> Option<Vec<Curve>> {
        if start == 0 {
            return Some(vec![Curve::None; count]);
        }
        let mut out = Vec::with_capacity(count);
        let mut at = start;
        for _ in 0..count {
            let rest = tag.get(at..)?;
            out.push(parse_curve(rest)?);
            at = at.checked_add(curve_length(rest)?)?;
            at = at.checked_add(at.wrapping_neg() % 4)?;
        }
        Some(out)
    };

    let input = curves(b_at, inputs)?;
    let mid = curves(m_at, inputs)?;
    let output = curves(a_at, outputs)?;

    let offsets = if matrix_at == 0 {
        None
    } else {
        let mut matrix = [0.0f32; 12];
        for (index, slot) in matrix.iter_mut().enumerate() {
            *slot = fixed_at(tag, matrix_at.checked_add(index.checked_mul(4)?)?)?;
        }
        Some(matrix)
    };

    // A table is not optional here: three connection-space components become the device's
    // count only through one, and a `mBA ` tag with none describes a 3 → 3 map this crate
    // has no use for.
    if clut_at == 0 {
        return None;
    }
    let (grid, samples) = parse_clut(tag, clut_at, inputs, outputs)?;

    Some(Lut {
        input,
        offsets,
        mid,
        output,
        grid,
        outputs,
        samples,
        matrix: None,
        encoding: if lab_pcs {
            Encoding::Lab
        } else {
            Encoding::Xyz
        },
    })
}

/// Reads a v4 tag's CLUT: grid sizes per input, a sample width, and the samples.
fn parse_clut(
    tag: &[u8],
    clut_at: usize,
    inputs: usize,
    outputs: usize,
) -> Option<(Vec<usize>, Vec<f32>)> {
    let mut grid = Vec::with_capacity(inputs);
    for index in 0..inputs {
        grid.push(usize::from(*tag.get(clut_at.checked_add(index)?)?));
    }
    if grid.iter().any(|points| *points < 2) {
        return None;
    }
    let width = usize::from(*tag.get(clut_at.checked_add(16)?)?);
    if width != 1 && width != 2 {
        return None;
    }
    let total = grid
        .iter()
        .try_fold(outputs, |acc, points| acc.checked_mul(*points))?;
    if total > MAX_CLUT {
        return None;
    }
    let start = clut_at.checked_add(20)?;
    let mut samples = Vec::with_capacity(total);
    for index in 0..total {
        let at = start.checked_add(index.checked_mul(width)?)?;
        samples.push(if width == 1 {
            f32::from(*tag.get(at)?) / 255.0
        } else {
            f32::from(u16_at(tag, at)?) / 65535.0
        });
    }
    Some((grid, samples))
}

/// Parses the v4 `lutAToBType` tag.
///
/// The pipeline runs A curves, then the table, then M curves, then a matrix, then B
/// curves — and every stage is optional, signalled by a zero offset.
fn parse_mab(tag: &[u8], lab_pcs: bool) -> Option<Lut> {
    let inputs = usize::from(*tag.get(8)?);
    let outputs = usize::from(*tag.get(9)?);
    if inputs == 0 || outputs == 0 || inputs > 8 {
        return None;
    }

    let offset = |at: usize| -> Option<usize> { usize::try_from(u32_at(tag, at)?).ok() };
    let (b_at, matrix_at, m_at, clut_at, a_at) = (
        offset(12)?,
        offset(16)?,
        offset(20)?,
        offset(24)?,
        offset(28)?,
    );

    // Curves are stored consecutively, each padded to a four-byte boundary.
    let curves = |start: usize, count: usize| -> Option<Vec<Curve>> {
        if start == 0 {
            return Some(vec![Curve::None; count]);
        }
        let mut out = Vec::with_capacity(count);
        let mut at = start;
        for _ in 0..count {
            let rest = tag.get(at..)?;
            out.push(parse_curve(rest)?);
            at = at.checked_add(curve_length(rest)?)?;
            at = at.checked_add(at.wrapping_neg() % 4)?;
        }
        Some(out)
    };

    let input = curves(a_at, inputs)?;
    // The B curves are the last stage and act on the output, so they take that role here;
    // the M curves and matrix sit between the table and them.
    let output = curves(b_at, outputs)?;

    let (grid, samples) = if clut_at == 0 {
        (vec![2usize; inputs], vec![0.0; 1usize << inputs])
    } else {
        parse_clut(tag, clut_at, inputs, outputs)?
    };

    // The M curves and the matrix are not modelled: they only appear in profiles whose
    // table output is already the connection space, and treating them as the identity is
    // exact in that case. A profile that needs them is rare enough to be worth reporting
    // rather than approximating, so it is refused.
    if matrix_at != 0 || m_at != 0 {
        return None;
    }

    Some(Lut {
        input,
        offsets: None,
        mid: Vec::new(),
        output,
        grid,
        outputs,
        samples,
        matrix: None,
        // A v4 tag always uses the modern encoding, whatever the profile's version says.
        encoding: if lab_pcs {
            Encoding::Lab
        } else {
            Encoding::Xyz
        },
    })
}

/// The byte length of a curve tag, for walking a list of them.
fn curve_length(tag: &[u8]) -> Option<usize> {
    match tag.get(..4)? {
        b"curv" => {
            let count = usize::try_from(u32_at(tag, 8)?).ok()?;
            12usize.checked_add(count.checked_mul(2)?)
        }
        b"para" => {
            let needed: usize = match u16_at(tag, 8)? {
                0 => 1,
                1 => 3,
                2 => 4,
                3 => 5,
                4 => 7,
                _ => return None,
            };
            12usize.checked_add(needed.checked_mul(4)?)
        }
        _ => None,
    }
}

/// How a lookup table's outputs encode the connection space.
///
/// A table stores integers; what they mean depends on the connection space and, for Lab,
/// on the profile's version. Decoding this wrongly is the classic ICC error, because the
/// result is a smooth, plausible image in entirely the wrong colours.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Encoding {
    /// `XYZ`, where `1.0` is stored as `0x8000` — so the range runs a little past one.
    Xyz,
    /// `L*a*b*` as ICC v4 encodes it: the full integer range spans each axis.
    Lab,
    /// `L*a*b*` as ICC v2's sixteen-bit tables encode it, with `L*` = 100 at `0xFF00`
    /// rather than `0xFFFF`.
    ///
    /// The difference is a factor of 65535/65280, about 0.4% — small enough to look like
    /// rounding, large enough to shift every colour on the page.
    LabLegacy,
}

impl Encoding {
    /// Turns connection-space values into the normalised inputs a "from CIE" table reads.
    ///
    /// [`Self::decode`] run backwards, written beside it so that the two cannot drift;
    /// `an_encoding_round_trips_through_its_inverse` holds them together.
    fn encode(self, values: [f32; 3]) -> [f32; 3] {
        let at = |index: usize| values.get(index).copied().unwrap_or(0.0);
        match self {
            Self::Xyz => [
                at(0) * 32768.0 / 65535.0,
                at(1) * 32768.0 / 65535.0,
                at(2) * 32768.0 / 65535.0,
            ],
            Self::Lab => [
                at(0) / 100.0,
                (at(1) + 128.0) / 255.0,
                (at(2) + 128.0) / 255.0,
            ],
            Self::LabLegacy => {
                let scale = 65280.0 / 65535.0;
                [
                    at(0) / 100.0 * scale,
                    (at(1) + 128.0) / 255.0 * scale,
                    (at(2) + 128.0) / 255.0 * scale,
                ]
            }
        }
    }

    /// Turns a table's normalised outputs into actual connection-space values.
    fn decode(self, values: [f32; 3]) -> [f32; 3] {
        let at = |index: usize| values.get(index).copied().unwrap_or(0.0);
        match self {
            // The table normalised by 65535, but the encoding puts 1.0 at 0x8000.
            Self::Xyz => [
                at(0) * 65535.0 / 32768.0,
                at(1) * 65535.0 / 32768.0,
                at(2) * 65535.0 / 32768.0,
            ],
            Self::Lab => [at(0) * 100.0, at(1) * 255.0 - 128.0, at(2) * 255.0 - 128.0],
            Self::LabLegacy => {
                let scale = 65535.0 / 65280.0;
                [
                    (at(0) * scale) * 100.0,
                    (at(1) * scale) * 255.0 - 128.0,
                    (at(2) * scale) * 255.0 - 128.0,
                ]
            }
        }
    }
}

/// Profiles assembled byte by byte, for this module's tests and `crate::colour`'s.
///
/// Everything here is positional, so each builder doubles as a statement of the layout the
/// parser is expected to read.
#[cfg(test)]
pub(crate) mod fixtures {
    #![expect(
        clippy::arithmetic_side_effects,
        reason = "layout arithmetic on a fixture's own constants, which cannot overflow"
    )]

    /// A profile of `version` over `space` and `pcs`, holding `tags` in order.
    ///
    /// A 128-byte header, the tag count, one 12-byte entry per tag, then the tags themselves.
    pub(crate) fn profile_of(
        space: [u8; 4],
        pcs: [u8; 4],
        version: u8,
        tags: &[([u8; 4], Vec<u8>)],
    ) -> Vec<u8> {
        let mut out = vec![0u8; 128];
        out[8] = version;
        out[12..16].copy_from_slice(b"prtr");
        out[16..20].copy_from_slice(&space);
        out[20..24].copy_from_slice(&pcs);
        out[36..40].copy_from_slice(b"acsp");
        out.extend_from_slice(&u32::try_from(tags.len()).expect("small").to_be_bytes());
        let mut offset = 128 + 4 + 12 * tags.len();
        for (name, tag) in tags {
            out.extend_from_slice(name);
            out.extend_from_slice(&u32::try_from(offset).expect("small").to_be_bytes());
            out.extend_from_slice(&u32::try_from(tag.len()).expect("small").to_be_bytes());
            offset += tag.len();
        }
        for (_, tag) in tags {
            out.extend_from_slice(tag);
        }
        out
    }

    /// An `mft2` tag with two grid points per axis and identity curves, so that the table
    /// *is* its corners and the profile's own interpolation fills in between them.
    ///
    /// Sizes, matrix, input curves, CLUT, output curves, in that order. `clut` holds the
    /// corners with the *last* input varying fastest, which is ICC's own order.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the fixture's constants are written as the fixed-point values it encodes"
    )]
    pub(crate) fn mft2_tag(inputs: usize, outputs: usize, clut: &[u16]) -> Vec<u8> {
        let mut tag = Vec::new();
        tag.extend_from_slice(b"mft2");
        tag.extend_from_slice(&[0; 4]);
        tag.push(u8::try_from(inputs).expect("small"));
        tag.push(u8::try_from(outputs).expect("small"));
        tag.push(2);
        tag.push(0);
        for value in [1.0f32, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0] {
            tag.extend_from_slice(&((value * 65536.0) as i32).to_be_bytes());
        }
        tag.extend_from_slice(&2u16.to_be_bytes());
        tag.extend_from_slice(&2u16.to_be_bytes());
        for _ in 0..inputs {
            for value in [0u16, 0xFFFF] {
                tag.extend_from_slice(&value.to_be_bytes());
            }
        }
        assert_eq!(
            clut.len(),
            (1 << inputs) * outputs,
            "a corner per grid point"
        );
        for value in clut {
            tag.extend_from_slice(&value.to_be_bytes());
        }
        for _ in 0..outputs {
            for value in [0u16, 0xFFFF] {
                tag.extend_from_slice(&value.to_be_bytes());
            }
        }
        tag
    }

    /// A `mBA ` tag over three inputs: no B curves, `matrix` if given, no M curves, a 2×2×2
    /// table of sixteen-bit samples, no A curves — the five offsets in the header slots
    /// ISO 15076-1 section 10.11 gives them, and a zero offset for a stage that is absent.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the fixture's constants are written as the fixed-point values it encodes"
    )]
    pub(crate) fn mba_tag(outputs: usize, clut: &[u16], matrix: Option<[f32; 12]>) -> Vec<u8> {
        let mut tag = Vec::new();
        tag.extend_from_slice(b"mBA ");
        tag.extend_from_slice(&[0; 4]);
        tag.push(3);
        tag.push(u8::try_from(outputs).expect("small"));
        tag.extend_from_slice(&[0; 2]);
        let matrix_at: u32 = if matrix.is_some() { 32 } else { 0 };
        let clut_at: u32 = 32 + if matrix.is_some() { 48 } else { 0 };
        for at in [0u32, matrix_at, 0, clut_at, 0] {
            tag.extend_from_slice(&at.to_be_bytes());
        }
        if let Some(matrix) = matrix {
            for value in matrix {
                tag.extend_from_slice(&((value * 65536.0) as i32).to_be_bytes());
            }
        }
        let mut grid = [0u8; 16];
        grid[..3].copy_from_slice(&[2, 2, 2]);
        tag.extend_from_slice(&grid);
        tag.push(2);
        tag.extend_from_slice(&[0; 3]);
        assert_eq!(clut.len(), 8 * outputs, "a corner per grid point");
        for value in clut {
            tag.extend_from_slice(&value.to_be_bytes());
        }
        tag
    }

    /// A "from CIE" table whose corners state one minus each input on the three chromatic
    /// inks and no black — an affine rule, so trilinear interpolation of the corners *is* the
    /// rule and a test's expected value is `1 − input` and nothing else.
    pub(crate) fn complement_clut() -> Vec<u16> {
        let mut clut = Vec::with_capacity(32);
        for corner in 0..8usize {
            for axis in 0..3usize {
                let high = (corner >> (2 - axis)) & 1 == 1;
                clut.push(if high { 0 } else { 0xFFFF });
            }
            clut.push(0);
        }
        clut
    }

    /// A "to CIE" table of a press whose black ink alone darkens: `XYZ = D50 × (1 − 0.9 k)`,
    /// so its darkest colour is a tenth of white and black point compensation has a range
    /// to align.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "the fixture's constants are written as the fixed-point values it encodes"
    )]
    pub(crate) fn black_only_clut() -> Vec<u16> {
        let mut clut = Vec::with_capacity(48);
        for corner in 0..16usize {
            let black = if corner & 1 == 1 { 1.0f32 } else { 0.0 };
            for white in super::WHITE {
                clut.push((white * (1.0 - 0.9 * black) * 32768.0) as u16);
            }
        }
        clut
    }

    /// A v2 CMYK profile over an XYZ connection space carrying both directions:
    /// [`black_only_clut`] out and [`complement_clut`] in.
    pub(crate) fn two_way_cmyk_profile() -> Vec<u8> {
        profile_of(
            *b"CMYK",
            *b"XYZ ",
            2,
            &[
                (*b"A2B1", mft2_tag(4, 3, &black_only_clut())),
                (*b"B2A1", mft2_tag(3, 4, &complement_clut())),
            ],
        )
    }

    /// The same press with its "to CIE" table only.
    pub(crate) fn one_way_cmyk_profile() -> Vec<u8> {
        profile_of(
            *b"CMYK",
            *b"XYZ ",
            2,
            &[(*b"A2B1", mft2_tag(4, 3, &black_only_clut()))],
        )
    }
}

#[cfg(test)]
mod tests {
    use super::fixtures::{
        black_only_clut, complement_clut, mba_tag, mft2_tag, one_way_cmyk_profile, profile_of,
        two_way_cmyk_profile,
    };
    use super::{Encoding, Profile};

    /// A real CMYK profile, taken from the pdf.js corpus at test time.
    ///
    /// Read from the corpus rather than checked in: an ICC profile is a third party's
    /// copyrighted work, and this crate has no business redistributing one to test with.
    fn corpus_cmyk_profile() -> Option<Vec<u8>> {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../doc/pdf.js/test/pdfs/bug886717.pdf");
        let bytes = std::fs::read(path).ok()?;
        let document = pdf_syntax::Document::open(bytes).ok()?;
        for number in document.xref().object_numbers() {
            let object = document.get(pdf_syntax::ObjectId {
                number,
                generation: 0,
            });
            let Some(stream) = object.as_stream() else {
                continue;
            };
            let Some(data) = document.decoded_stream_data(stream) else {
                continue;
            };
            if data.len() > 128
                && data.get(36..40) == Some(b"acsp")
                && data.get(16..20) == Some(b"CMYK")
            {
                return Some(data.to_vec());
            }
        }
        None
    }

    fn bytes(colour: pdf_render::Color) -> (u8, u8, u8) {
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "clamped to 0.0..=1.0 before scaling"
        )]
        let byte = |value: f32| (value.clamp(0.0, 1.0) * 255.0).round() as u8;
        (byte(colour.r), byte(colour.g), byte(colour.b))
    }

    /// Assembles a v2 ICC profile with a single `A2B1` lookup table.
    ///
    /// Everything here is positional, so it doubles as a statement of the layout the parser
    /// is expected to read: a 128-byte header, a tag count, one 12-byte tag entry, then the
    /// `mft2` tag itself — sizes, input curves, CLUT, output curves, in that order.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the fixture's constants are written as the fixed-point values it encodes"
    )]
    fn lut16_profile(space: [u8; 4], pcs: [u8; 4], clut: &[u16], outputs: usize) -> Vec<u8> {
        let mut header = vec![0u8; 128];
        header[8] = 2; // major version: v2, which is what selects the legacy Lab encoding
        header[16..20].copy_from_slice(&space);
        header[20..24].copy_from_slice(&pcs);
        header[36..40].copy_from_slice(b"acsp");

        let mut tag = Vec::new();
        tag.extend_from_slice(b"mft2");
        tag.extend_from_slice(&[0; 4]);
        tag.push(1); // one input channel
        tag.push(u8::try_from(outputs).expect("small"));
        tag.push(2); // two grid points, so the CLUT is just the two ends
        tag.push(0);
        for value in [1.0f32, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0] {
            tag.extend_from_slice(&((value * 65536.0) as i32).to_be_bytes());
        }
        tag.extend_from_slice(&2u16.to_be_bytes()); // input table entries
        tag.extend_from_slice(&2u16.to_be_bytes()); // output table entries
        // An identity input curve, the CLUT, then identity output curves.
        for value in [0u16, 0xFFFF] {
            tag.extend_from_slice(&value.to_be_bytes());
        }
        for value in clut {
            tag.extend_from_slice(&value.to_be_bytes());
        }
        for _ in 0..outputs {
            for value in [0u16, 0xFFFF] {
                tag.extend_from_slice(&value.to_be_bytes());
            }
        }

        let mut out = header;
        out.extend_from_slice(&1u32.to_be_bytes()); // one tag
        out.extend_from_slice(b"A2B1");
        out.extend_from_slice(&144u32.to_be_bytes()); // 128 + 4 + 12
        out.extend_from_slice(&u32::try_from(tag.len()).expect("small").to_be_bytes());
        out.extend_from_slice(&tag);
        out
    }

    /// The PCS decoding must follow the ICC encoding, which is not a plain 0..1 scaling.
    ///
    /// XYZ in a lookup table is `u1Fixed15`: `0x8000` is 1.0, so the representable range
    /// runs a little past one rather than stopping at it. Reading the entries as a fraction
    /// of `0xFFFF` — the obvious thing, and what every other table in the format does —
    /// halves every value, which turns white into a mid grey and every colour into a darker
    /// version of itself. That is a difference no test comparing colours *within* one
    /// rendering can see, since everything moves together.
    ///
    /// So the CLUT here holds D50 white in that encoding and the profile must produce
    /// white. `0.9642 × 32768` is 31596; the entry read as a fraction of `0xFFFF` would be
    /// 0.482, and the result a grey around 187.
    #[test]
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "the fixture's constants are written as the fixed-point values it encodes"
    )]
    fn xyz_in_a_lookup_table_is_decoded_as_the_icc_encoding_specifies() {
        let white = [
            (0.964_2 * 32768.0) as u16,
            32768,
            (0.824_9 * 32768.0) as u16,
        ];
        // Grid point zero is black, grid point one is the white above.
        let clut = [0, 0, 0, white[0], white[1], white[2]];
        let profile = Profile::parse(&lut16_profile(*b"GRAY", *b"XYZ ", &clut, 3))
            .expect("the assembled profile parses");

        assert_eq!(bytes(profile.to_rgb(&[1.0])), (255, 255, 255));
        assert_eq!(bytes(profile.to_rgb(&[0.0])), (0, 0, 0));
    }

    /// Interpolation between grid points is linear in the connection space.
    ///
    /// With two grid points there is nothing to interpolate *between* except the ends, so
    /// the midpoint is exactly half the white point in XYZ — and half of D50's Y is 0.5,
    /// which sRGB encodes as 188 rather than 128. That the answer is not 128 is the point:
    /// interpolating after the transfer curve rather than before it would give 128, and
    /// would be wrong by 60 levels in the middle of every gradient.
    #[test]
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "the fixture's constants are written as the fixed-point values it encodes"
    )]
    fn the_lookup_table_interpolates_in_the_connection_space() {
        let white = [
            (0.964_2 * 32768.0) as u16,
            32768,
            (0.824_9 * 32768.0) as u16,
        ];
        let clut = [0, 0, 0, white[0], white[1], white[2]];
        let profile = Profile::parse(&lut16_profile(*b"GRAY", *b"XYZ ", &clut, 3))
            .expect("the assembled profile parses");

        let (r, g, b) = bytes(profile.to_rgb(&[0.5]));
        // Neutral to within a level: the white point's components do not land on exact
        // multiples of 1/32768, so the fixture's own encoding of them is a fraction of a
        // level off. That is the table's precision, not the interpolation's.
        assert!(
            (187..=189).contains(&r) && r.abs_diff(g) <= 1 && g.abs_diff(b) <= 1,
            "half of D50 white is a neutral 188, got {r},{g},{b}"
        );
    }

    /// A profile that cannot reach black has its range stretched so that it does.
    ///
    /// PDF 2.0 Application Note 001 defines this as "aligning the darkest colour that could
    /// be described by the colour space of the data ... with the darkest colour that the
    /// output profile for the display device ... can produce". Here the profile's darkest
    /// colour is a tenth of the white point, and the display's is zero, so full ink must
    /// come out at zero — and turning compensation off must leave it where the profile put
    /// it. Both directions matter: §8.6.5.9 lets a document demand either.
    #[test]
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "the fixture's constants are written as the fixed-point values it encodes"
    )]
    fn black_point_compensation_aligns_the_darkest_colour_the_profile_reaches() {
        let dark = [
            (0.096_42 * 32768.0) as u16,
            3277,
            (0.082_49 * 32768.0) as u16,
        ];
        let white = [
            (0.964_2 * 32768.0) as u16,
            32768,
            (0.824_9 * 32768.0) as u16,
        ];
        // Grid point one is full ink here, so the darkest the profile reaches is `dark`.
        let clut = [white[0], white[1], white[2], dark[0], dark[1], dark[2]];
        let profile = Profile::parse(&lut16_profile(*b"GRAY", *b"XYZ ", &clut, 3))
            .expect("the assembled profile parses");

        assert_eq!(
            bytes(profile.to_rgb(&[1.0])),
            (0, 0, 0),
            "compensation must bring the profile's darkest colour to the display's"
        );
        let (r, g, b) = bytes(profile.to_rgb_with(&[1.0], false));
        assert!(
            r > 80 && g > 80 && b > 80,
            "without compensation the same colour stays the grey the profile describes, \
             got {r},{g},{b}"
        );
    }

    /// The darkest colour a profile reaches is at the end of the range its *space* darkens
    /// towards, and for `RGB` that is zero rather than one.
    ///
    /// The same fixture as the test above with its two grid points swapped, which is what an
    /// additive space writes: nothing in is black and everything in is white. Compensation
    /// must therefore stretch the *zero* end onto the display's black and leave full input
    /// where the profile put it — and the failure this pins is not a slightly wrong colour
    /// but the whole page. Taking the one end as black gives a span of a few thousandths
    /// between "black" and D50 white, and dividing by it puts every input on zero:
    /// `2268885.pdf` in the crawl is a floor plan drawn as its own negative for that reason,
    /// with nothing reported (ADR 0451).
    #[test]
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "the fixture's constants are written as the fixed-point values it encodes"
    )]
    fn an_additive_profiles_black_is_the_zero_end_of_its_range() {
        let dark = [
            (0.096_42 * 32768.0) as u16,
            3277,
            (0.082_49 * 32768.0) as u16,
        ];
        let white = [
            (0.964_2 * 32768.0) as u16,
            32768,
            (0.824_9 * 32768.0) as u16,
        ];
        // Grid point zero is the darkest this space reaches, which is how `RGB` and `GRAY`
        // are written and the opposite of the subtractive fixture above.
        let clut = [dark[0], dark[1], dark[2], white[0], white[1], white[2]];
        let profile = Profile::parse(&lut16_profile(*b"GRAY", *b"XYZ ", &clut, 3))
            .expect("the assembled profile parses");

        assert_eq!(
            bytes(profile.to_rgb(&[0.0])),
            (0, 0, 0),
            "compensation must bring the profile's darkest colour to the display's"
        );
        assert_eq!(
            bytes(profile.to_rgb(&[1.0])),
            (255, 255, 255),
            "and must leave the profile's white where it is rather than stretching onto it"
        );
    }

    /// Independent evaluators of the same profile should agree with us.
    ///
    /// This is corroboration, not a definition: the ICC encoding decides what this profile
    /// means, and the tests above pin that directly. What agreement here adds is evidence
    /// that we read a *real* profile's tags the way their author intended — a profile with
    /// curves, a sixteen-cubed CLUT and four input channels exercises far more of the
    /// format than anything hand-assembled. If it ever disagrees, the profile's own tables
    /// decide who is wrong, not the majority.
    ///
    /// The tolerance is wider on the darkest patches, where black point compensation is
    /// doing the most work and the construction differs as `detect_black` describes.
    #[test]
    fn a_real_cmyk_profile_agrees_with_independent_evaluators() {
        let Some(data) = corpus_cmyk_profile() else {
            println!("skipped: the pdf.js submodule is not checked out");
            return;
        };
        let profile = Profile::parse(&data).expect("a real CMYK profile parses");
        assert_eq!(profile.channels(), 4);

        // patch, expected, tolerance
        let cases = [
            ([0.0, 0.0, 0.0, 0.0], (255, 255, 255), 0),
            ([1.0, 0.0, 0.0, 0.0], (0, 158, 226), 2),
            ([0.0, 1.0, 0.0, 0.0], (229, 0, 126), 2),
            ([0.0, 0.0, 1.0, 0.0], (255, 237, 0), 2),
            ([0.5, 0.0, 0.0, 0.0], (130, 207, 245), 2),
            ([0.0, 0.0, 0.0, 1.0], (27, 27, 24), 8),
            ([1.0, 1.0, 1.0, 1.0], (0, 0, 0), 1),
        ];
        for (input, expected, tolerance) in cases {
            let got = bytes(profile.to_rgb(&input));
            let worst = [
                got.0.abs_diff(expected.0),
                got.1.abs_diff(expected.1),
                got.2.abs_diff(expected.2),
            ]
            .into_iter()
            .max()
            .unwrap_or(0);
            assert!(
                worst <= tolerance,
                "{input:?}: got {got:?}, other readers give {expected:?} ({worst} away)"
            );
        }
    }

    /// The profile's own inks must differ from the generic fallback, or applying it
    /// achieved nothing.
    #[test]
    fn applying_a_profile_changes_the_answer() {
        let Some(data) = corpus_cmyk_profile() else {
            return;
        };
        let profile = Profile::parse(&data).expect("parses");
        let generic = crate::colour::ColourSpace::Cmyk.to_rgb(&[1.0, 0.0, 0.0, 0.0]);
        let managed = profile.to_rgb(&[1.0, 0.0, 0.0, 0.0]);
        assert_ne!(
            bytes(generic),
            bytes(managed),
            "this profile's cyan differs from the generic one, so the two must not agree"
        );
    }

    /// The two encodings written beside each other are inverses.
    #[test]
    fn an_encoding_round_trips_through_its_inverse() {
        for encoding in [Encoding::Xyz, Encoding::Lab, Encoding::LabLegacy] {
            for values in [[0.0, 0.0, 0.0], [0.5, 0.25, 0.75], [1.0, 1.0, 1.0]] {
                let back = encoding.encode(encoding.decode(values));
                for (got, want) in back.iter().zip(values) {
                    assert!(
                        (got - want).abs() < 1e-5,
                        "{encoding:?}: {values:?} came back as {back:?}"
                    );
                }
            }
        }
    }

    /// §8.6.5.5's "from CIE" information is read, and it states the device colour of a
    /// connection-space colour.
    ///
    /// The table's corners state `1 − input` on the three chromatic inks and no black, over
    /// the table's own input encoding — XYZ with 1.0 at `0x8000`, so a connection-space value
    /// `v` is the input `v × 32768 ÷ 65535` — and trilinear interpolation of an affine rule is
    /// the rule. So the expected inks are that arithmetic on the XYZ handed in, and nothing
    /// this module computes goes into them.
    #[test]
    fn a_from_cie_table_states_the_device_colour_of_a_connection_space_colour() {
        let profile = Profile::parse(&two_way_cmyk_profile()).expect("parses");
        assert!(profile.is_bidirectional());

        let xyz = [0.482_1, 0.5, 0.412_45];
        let inks = profile.to_device(xyz, false).expect("a from-CIE table");
        let want = [
            1.0 - 0.482_1 * 32768.0 / 65535.0,
            1.0 - 0.5 * 32768.0 / 65535.0,
            1.0 - 0.412_45 * 32768.0 / 65535.0,
            0.0,
        ];
        for (axis, (got, want)) in inks.iter().zip(want).enumerate() {
            assert!(
                (got - want).abs() < 1e-4,
                "ink {axis}: {got} where the table states {want} ({inks:?})"
            );
        }
    }

    /// A profile stating no such table is not bi-directional, and keeps everything it had.
    #[test]
    fn a_profile_without_a_from_cie_table_is_not_bidirectional() {
        let profile = Profile::parse(&one_way_cmyk_profile()).expect("parses");
        assert!(!profile.is_bidirectional());
        assert!(profile.to_device([0.5, 0.5, 0.5], false).is_none());
        // The "to CIE" half is untouched: black point compensation still finds the press's
        // black at a tenth of white and stretches it to the display's.
        assert_eq!(bytes(profile.to_rgb(&[0.0, 0.0, 0.0, 1.0])), (0, 0, 0));
    }

    /// The conversion in undoes the black point compensation the conversion out applied, so
    /// the two are inverses in the same sense whichever way §8.6.5.9's flag is set.
    #[test]
    fn the_conversion_in_undoes_the_compensation_the_conversion_out_applied() {
        let profile = Profile::parse(&two_way_cmyk_profile()).expect("parses");
        for inks in [
            [0.0f32, 0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0, 0.5],
            [0.3, 0.2, 0.1, 0.7],
        ] {
            let compensated = profile
                .to_device(profile.to_xyz_with(&inks, true), true)
                .expect("a from-CIE table");
            let plain = profile
                .to_device(profile.to_xyz_with(&inks, false), false)
                .expect("a from-CIE table");
            for (axis, (got, want)) in compensated.iter().zip(plain).enumerate() {
                assert!(
                    (got - want).abs() < 1e-4,
                    "ink {axis} of {inks:?}: {got} compensated against {want} plain"
                );
            }
        }
        // And the test discriminates: a compensated XYZ read *without* undoing the stretch is
        // a different colour, by the whole of the range the stretch aligned.
        let inks = [0.0f32, 0.0, 0.0, 0.5];
        let mixed = profile
            .to_device(profile.to_xyz_with(&inks, true), false)
            .expect("a from-CIE table");
        let plain = profile
            .to_device(profile.to_xyz_with(&inks, false), false)
            .expect("a from-CIE table");
        assert!(
            (mixed[1] - plain[1]).abs() > 0.02,
            "the stretch moves a mid grey: {mixed:?} against {plain:?}"
        );
    }

    /// A v2 sixteen-bit table over a Lab connection space reads the legacy encoding on its
    /// input — `L* = 100` at `0xFF00` — exactly as [`Encoding::decode`] reads it on an output.
    #[test]
    fn a_legacy_lab_from_cie_table_reads_its_input_as_the_legacy_encoding() {
        let profile = Profile::parse(&profile_of(
            *b"CMYK",
            *b"Lab ",
            2,
            &[
                (*b"A2B1", mft2_tag(4, 3, &[0x8000u16; 48])),
                (*b"B2A1", mft2_tag(3, 4, &complement_clut())),
            ],
        ))
        .expect("parses");
        let inks = profile
            .to_device(crate::colour::lab_to_xyz(50.0, 0.0, 0.0), false)
            .expect("a from-CIE table");
        let scale = 65280.0 / 65535.0;
        let want = [
            1.0 - 0.5 * scale,
            1.0 - 128.0 / 255.0 * scale,
            1.0 - 128.0 / 255.0 * scale,
            0.0,
        ];
        for (axis, (got, want)) in inks.iter().zip(want).enumerate() {
            assert!(
                (got - want).abs() < 1e-3,
                "ink {axis}: {got} where the legacy encoding states {want} ({inks:?})"
            );
        }
    }

    /// A v4 `mBA ` tag runs its B curves, matrix and M curves before its table, in that order.
    ///
    /// Without a matrix the corner rule reads the v4 Lab encoding straight — `L* = 50` is
    /// `0.5`, `a* = b* = 0` is `128 ÷ 255` — and with one whose offsets add a tenth to the
    /// first channel, that channel's input is `0.6`.
    #[test]
    fn a_v4_from_cie_table_runs_its_matrix_before_its_table() {
        let expect = |matrix: Option<[f32; 12]>, first: f32| {
            let profile = Profile::parse(&profile_of(
                *b"CMYK",
                *b"Lab ",
                4,
                &[
                    (*b"A2B1", mft2_tag(4, 3, &[0x8000u16; 48])),
                    (*b"B2A1", mba_tag(4, &complement_clut(), matrix)),
                ],
            ))
            .expect("parses");
            let inks = profile
                .to_device(crate::colour::lab_to_xyz(50.0, 0.0, 0.0), false)
                .expect("a from-CIE table");
            let want = [1.0 - first, 1.0 - 128.0 / 255.0, 1.0 - 128.0 / 255.0, 0.0];
            for (axis, (got, want)) in inks.iter().zip(want).enumerate() {
                assert!(
                    (got - want).abs() < 1e-3,
                    "ink {axis} with matrix {matrix:?}: {got} where the table states {want}"
                );
            }
        };
        expect(None, 0.5);
        expect(
            Some([1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.1, 0.0, 0.0]),
            0.6,
        );
    }

    /// A "to CIE" table is still read from a profile whose "from CIE" one cannot be.
    #[test]
    fn an_unreadable_from_cie_table_costs_the_profile_nothing_it_had() {
        let profile = Profile::parse(&profile_of(
            *b"CMYK",
            *b"XYZ ",
            2,
            &[
                (*b"A2B1", mft2_tag(4, 3, &black_only_clut())),
                (*b"B2A1", b"junk".to_vec()),
            ],
        ))
        .expect("the to-CIE half parses on its own");
        assert!(!profile.is_bidirectional());
        assert_eq!(
            bytes(profile.to_rgb(&[0.0, 0.0, 0.0, 0.0])),
            (255, 255, 255)
        );
    }

    /// Garbage must be refused rather than parsed into nonsense.
    #[test]
    fn a_profile_that_is_not_one_is_refused() {
        assert!(Profile::parse(&[]).is_none());
        assert!(Profile::parse(&[0u8; 256]).is_none(), "no acsp signature");
        // A valid signature with a truncated tag table.
        let mut damaged = vec![0u8; 200];
        damaged[36..40].copy_from_slice(b"acsp");
        damaged[16..20].copy_from_slice(b"CMYK");
        damaged[20..24].copy_from_slice(b"Lab ");
        damaged[128..132].copy_from_slice(&999u32.to_be_bytes());
        assert!(Profile::parse(&damaged).is_none());
    }
}
