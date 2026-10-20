//! §10.4.2.5: `DeviceCMYK` to RGB, by the sixteen corners of the ink cube.

use pdf_render::Color;

/// Clamps a colour component to `0.0..=1.0`, treating `NaN` as zero.
///
/// `f32::clamp` *propagates* `NaN` rather than removing it, so clamping alone is not
/// enough. A `NaN` reaching a rasteriser is not a wrong colour, it is an undefined one:
/// it survives premultiplication and blending and can leave a pixel with no value at all.
/// Functions are the likely source, since a malformed one can produce anything.
pub(super) fn channel(value: f32) -> f32 {
    if value.is_nan() {
        return 0.0;
    }
    value.clamp(0.0, 1.0)
}

/// The sixteen corners of the CMYK cube, as sRGB, indexed by the bits `c m y k`.
///
/// # The standard states two answers and ranks them, and this is the higher one
///
/// This comment used to open "ISO 32000-2 gives **no** conversion from `DeviceCMYK` to any
/// other space", on the evidence of §8.6.4.4, which says of the components only that they
/// "shall represent the concentrations of these process colourants". That was true of the
/// clause it cited and false of the standard. **§10.4.2.5 states a conversion outright** —
/// each additive component is one minus the sum of its complementary ink and the black,
/// clamped at one — and §10.4.2.1 says what it is for: "a less-capable PDF processor **may** choose to use
/// the algorithms specified in the following subclauses 10.4.2.2 through 10.4.2.5. These
/// algorithms are, however, very simple and as perceived by a human viewer they produce only
/// crude approximations of the original colours." An ICC-enabled processor "should always
/// follow the provisions and recommendations provided in 10.3".
///
/// So the question is not unanswered; it is answered twice, in a stated order, and this tree
/// is on the higher branch. §10.3.2 is the sentence that authorises what this table *is*:
///
/// > A PDF processor should establish CIE-based colour specifications for device colour
/// > spaces ( DeviceGray , DeviceRGB , or DeviceCMYK ), and thus implicitly remap device
/// > colour spaces into CIEbased colour spaces, when those device colour spaces do not
/// > match that of the raster output device.
///
/// A display's native space is not CMYK, so the remapping is required of us — and the sentence
/// that licenses *what* we remap it to is §10.3.2's own NOTE:
///
/// > Establishing a CIE-based source colour space can happen based on a user-driven
/// > configuration, by assumptions made by the PDF processor software, by analysis of the
/// > colour values and other properties, or by other mechanisms.
///
/// Assuming standard process inks *is* such an assumption, made in the one place the clause
/// leaves for it.
///
/// **The NOTE this paragraph rests on is §10.3.2's, not §10.3.1's, and the two differ by one
/// word.** §10.3.1's is about establishing the CIE-based **destination** — what
/// the screen is — and §10.3.2's about the **source**, which is what a document's `DeviceCMYK`
/// means. This table is a claim about the source; the destination here is sRGB. Both NOTEs
/// contain "assumptions made by the PDF processor software", which is exactly how a citation one
/// subclause off survives being read. `CONTRADICTED_DEVICE_CMYK_CONVERSION` is the corpus
/// evidence that the source NOTE is the operative one: four renderers make four different source
/// assumptions on the same pages, three of them by loading a press profile, and the standard
/// licenses all four in that one sentence (ADR 0484).
///
/// What the specification does state is *which press to ask first*, and it names three
/// sources — `/DefaultCMYK` (§8.6.5.6, "shall be used"), an output intent's
/// `/DestOutputProfile` (§14.11.5, §8.6.5.7 NOTE 3), and an `ICCBased` space naming the
/// profile directly. All three are implemented and all three win over this table, which is
/// reached only when the document names no press at all.
///
/// The press assumed is standard process inks, at their published sRGB appearances:
/// `#00AEEF` cyan, `#EC008C` magenta, `#FFF200` yellow, `#231F20` black, with the
/// overprints that follow from them. Written as eight-bit values because that is the
/// precision at which ink appearances are published.
///
/// # And the crude answer was measured rather than dismissed
///
/// §10.4.2.5's formula is off by up to 115 of 255 at these corners and renders process
/// magenta as `#FF00FF`, a colour no ink produces — which is the difference between an answer
/// about subtractive ink and one about additive light. That is an argument; the measurement
/// is the evidence. Over the whole oracle, switching this table for §10.4.2.5's formula moves
/// **802 agreeing and 88 contradicted pages to 800 and 90**. So the standard's own lower
/// branch is worse here than its higher one, which is what §10.4.2.1 says to expect of it.
///
/// Other readers land within a level of these numbers. That is evidence that assuming
/// standard process inks is the conventional reading of §10.3.2's licence — not evidence that
/// these numbers are correct, because the clause states no destination for them to be correct
/// against.
#[rustfmt::skip]
pub(super) const CMYK_CORNERS: [[u8; 3]; 16] = [
    //  R    G    B      c m y k
    [255, 255, 255], // 0 0 0 0  paper
    [  0, 173, 239], // 1 0 0 0  process cyan
    [236,   0, 140], // 0 1 0 0  process magenta
    [ 46,  49, 146], // 1 1 0 0  blue
    [255, 242,   0], // 0 0 1 0  process yellow
    [  0, 166,  80], // 1 0 1 0  green
    [237,  28,  36], // 0 1 1 0  red
    [ 54,  54,  57], // 1 1 1 0  three-colour black
    [ 35,  31,  32], // 0 0 0 1  process black
    [  0,  15,  36], // 1 0 0 1
    [ 36,   0,   0], // 0 1 0 1
    [  0,   0,   2], // 1 1 0 1
    [ 28,  26,   0], // 0 0 1 1
    [  0,  19,   0], // 1 0 1 1
    [ 34,   0,   0], // 0 1 1 1
    [  0,   0,   0], // 1 1 1 1  registration
];

/// Converts `DeviceCMYK` to sRGB by interpolating between the cube's corners.
///
/// Multilinear: each of the four inks contributes independently, which is what makes the
/// result exact at the sixteen corners and continuous everywhere between them. It is the
/// interpolation an ICC lookup table uses over its own grid, so the fallback behaves like
/// the profiles it stands in for rather than like a different kind of thing.
///
/// Interior points land within one level of what other readers produce, which are
/// themselves up to 53 levels apart from each other there — so this agrees with all of
/// them more closely than they agree among themselves.
pub(super) fn cmyk(c: f32, m: f32, y: f32, k: f32) -> Color {
    let (c, m, y, k) = (channel(c), channel(m), channel(y), channel(k));
    let weights = [1.0 - c, c];
    let weights_m = [1.0 - m, m];
    let weights_y = [1.0 - y, y];
    let weights_k = [1.0 - k, k];

    let mut rgb = [0.0f32; 3];
    for (index, corner) in CMYK_CORNERS.iter().enumerate() {
        // The index's bits select which side of each axis this corner sits on, in the
        // order c, m, y, k from the least significant bit.
        let weight = weights[index & 1]
            * weights_m[(index >> 1) & 1]
            * weights_y[(index >> 2) & 1]
            * weights_k[(index >> 3) & 1];
        if weight == 0.0 {
            continue;
        }
        for (channel, value) in rgb.iter_mut().zip(corner.iter()) {
            *channel += weight * f32::from(*value) / 255.0;
        }
    }
    Color::rgb(channel(rgb[0]), channel(rgb[1]), channel(rgb[2]))
}
