//! Colour conversion of decoded samples into the page's compositing space (ISO 32000-2
//! §8.6.5.5, §10.4.2): per sample tuple through a memo, banded across the pool for a large
//! image.
//!
//! A conversion is a pure function of one sample tuple, so neither the memo nor a band edge
//! moves a pixel (ADR 1469).

use pdf_syntax::{Dictionary, Document};
use rayon::iter::ParallelIterator as _;
use rayon::slice::ParallelSliceMut as _;

use crate::colour::Conversion;

use super::ImageError;
use super::space::{ColourSpace, Decode, Dictionaries, colour_space};

/// Converts a three-component raster in place, optionally on more than one thread.
///
/// **A band's memo is its own, and that is what makes the split exact**: [`SampleMemo`]
/// memoises a pure function of a sample tuple, so two bands meeting the same tuple convert it
/// twice and agree. Nothing in the result depends on how the raster was divided, which is the
/// property `a_band_boundary_changes_no_pixel` exists to check — and it is the difference
/// between this split and the rasteriser's, where a curve clipped by a strip's edge is
/// re-parameterised and the pixels *do* move (ADR 0138).
///
/// The table is sized from the band rather than from the image, because one proportioned to the
/// whole would be allocated once per thread.
pub(super) fn convert_three(
    space: &crate::colour::ColourSpace,
    decode: &Decode,
    rgba: &mut [u8],
    band: Option<usize>,
    into: &Conversion,
) {
    let convert = |chunk: &mut [u8], slots: usize| {
        let mut cache = SampleMemo::for_pixels(slots.max(1), [0u8; 3]);
        // The pixel before this one, as its three samples and its three converted bytes in the
        // low bytes of a word; the sentinel has a fourth byte, so no sample tuple equals it.
        let (mut seen, mut converted) = (u32::MAX, 0u32);
        for pixel in chunk.as_chunks_mut::<4>().0 {
            let word = u32::from_le_bytes(*pixel);
            // A pixel the same as the one before it converts to the same bytes; see
            // [`convert_four`] for why this is asked before the memo. The alpha byte is kept.
            // With the memo holding bytes, `22060_A1_01_Plans.pdf`'s four photographs went from
            // 2 143 M to 455 M instructions here, and its pinned turn from 94.4 to 76.0 ms with
            // this lever alone (ADR 1469).
            if word & 0x00FF_FFFF == seen {
                *pixel = (converted | (word & 0xFF00_0000)).to_le_bytes();
                continue;
            }
            let sample = [pixel[0], pixel[1], pixel[2]];
            let key = (1u64 << 32)
                | (u64::from(sample[0]) << 16)
                | (u64::from(sample[1]) << 8)
                | u64::from(sample[2]);
            let bytes = if let Some(bytes) = cache.get(key) {
                bytes
            } else {
                let colour = into.paint(
                    space,
                    &[
                        decode.value(0, usize::from(sample[0])),
                        decode.value(1, usize::from(sample[1])),
                        decode.value(2, usize::from(sample[2])),
                    ],
                );
                let bytes = [channel(colour.r), channel(colour.g), channel(colour.b)];
                cache.put(key, bytes);
                bytes
            };
            seen = word & 0x00FF_FFFF;
            converted = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], 0]);
            [pixel[0], pixel[1], pixel[2]] = bytes;
        }
    };
    match band {
        Some(band) => rgba
            .par_chunks_mut(band.saturating_mul(4).max(4))
            .for_each(|chunk| convert(chunk, band)),
        None => convert(rgba, rgba.len() / 4),
    }
}

/// Converts a four-component raster in place, on the same terms as [`convert_three`].
///
/// The four bytes of a pixel are the four components — `decode_jpeg` writes them there and
/// nothing has read them as colour yet — so the alpha byte is restored here, after the fourth
/// component has been consumed. Everything else is [`convert_three`]'s argument, including
/// why a band boundary changes no pixel: the memo is of a pure function of one sample tuple.
pub(super) fn convert_four(
    space: &crate::colour::ColourSpace,
    decode: &Decode,
    rgba: &mut [u8],
    band: Option<usize>,
    into: &Conversion,
) {
    let convert = |chunk: &mut [u8], slots: usize| {
        let mut cache = SampleMemo::for_pixels(slots.max(1), [0u8; 4]);
        let mut previous: Option<([u8; 4], [u8; 4])> = None;
        for pixel in chunk.as_chunks_mut::<4>().0 {
            let sample = *pixel;
            // **A pixel the same as the one before it is answered before the memo is asked.**
            // A scan's background is long runs of one sample, and the comparison is what a run
            // costs instead of a hash, a probe and a key compare; the bytes are the ones the
            // memo would have answered, because both are the conversion of the same tuple.
            if let Some((seen, bytes)) = previous
                && seen == sample
            {
                *pixel = bytes;
                continue;
            }
            // A tag of its own, so a four-component tuple cannot collide with a
            // three-component one in a memo shared by neither.
            let key = (1u64 << 33)
                | (u64::from(sample[0]) << 24)
                | (u64::from(sample[1]) << 16)
                | (u64::from(sample[2]) << 8)
                | u64::from(sample[3]);
            let bytes = if let Some(bytes) = cache.get(key) {
                bytes
            } else {
                let colour = into.paint(
                    space,
                    &[
                        decode.value(0, usize::from(sample[0])),
                        decode.value(1, usize::from(sample[1])),
                        decode.value(2, usize::from(sample[2])),
                        decode.value(3, usize::from(sample[3])),
                    ],
                );
                let bytes = [channel(colour.r), channel(colour.g), channel(colour.b), 255];
                cache.put(key, bytes);
                bytes
            };
            previous = Some((sample, bytes));
            *pixel = bytes;
        }
    };
    match band {
        Some(band) => rgba
            .par_chunks_mut(band.saturating_mul(4).max(4))
            .for_each(|chunk| convert(chunk, band)),
        None => convert(rgba, rgba.len() / 4),
    }
}

/// How many pixels one parallel band of a colour conversion covers, or `None` for one thread.
///
/// **Both numbers here are measured and the measurement is a wall clock**, because a parallel
/// change makes the instruction count go *up* while the page appears sooner — the same
/// distinction the strips make, and the reason both are quoted below.
///
/// `issue19971.pdf`'s 2500×1364 `ICCBased` photograph, interpreted whole, on 24 cores:
///
/// | bands | median clock | instructions |
/// |---|---|---|
/// | serial | ~110 ms | 1 085 M |
/// | 4 | ~85 ms | 1 206 M |
/// | **8** | **~57 ms** | **1 365 M** |
/// | 24 (one per core) | ~55 ms | 1 605 M |
///
/// So the cap is 8: it is the whole of the clock and two thirds of the extra processor time.
/// What the extra buys nothing is a [`SampleMemo`] table per band — each is allocated and
/// zeroed, and a table proportioned to a 24th of the image collides no less than one
/// proportioned to an eighth.
///
/// Below [`PARALLEL_PIXELS`] the split is refused for the same reason: a small image would pay a
/// table per thread to save a few hundred conversions.
pub(super) fn band_pixels(pixels: usize) -> Option<usize> {
    if pixels < PARALLEL_PIXELS {
        return None;
    }
    let bands = rayon::current_num_threads().clamp(1, MAX_BANDS);
    (bands >= 2).then(|| pixels.div_ceil(bands))
}

/// The smallest image worth colour-managing on more than one thread.
///
/// A quarter of a megapixel, which is where the two cross on this machine. The corpus gate —
/// 974 documents whose images are mostly far smaller — is 2.4 s either side of it, which is the
/// check that the threshold is not paid for by everything else.
pub(super) const PARALLEL_PIXELS: usize = 1 << 18;

/// The most bands one image is divided into. See [`band_pixels`] for the measurement.
const MAX_BANDS: usize = 8;

/// A memo of what a sample tuple converts to, for one image.
///
/// **Exact, not an approximation**: the key is the raw sample bytes, so a hit returns the
/// colour the conversion would have returned for those very samples. Nothing here rounds,
/// interpolates or resamples.
///
/// It exists because converting an image through a real colour space costs transcendental
/// functions per pixel. Callgrind on `issue19971.pdf`'s 2500x1364 `ICCBased` photograph
/// puts 36% of the page in `libm` — an ICC profile's tone curves and the sRGB encode are
/// both powers — and applying §8.6.5.5 without a memo took the whole corpus gate from 1.7 s
/// to 11.4 s. A one-component space never gets here: [`palette`] converts each of its at
/// most 256 possible samples once, which is the same idea where an exact table fits.
///
/// Direct-mapped rather than a hash map, and small enough to stay in cache, because an
/// image's colours are *spatially* clustered: neighbouring pixels are usually the same
/// colour or a near one, so a fixed table with no chaining and no growth answers most of
/// them. A collision costs one conversion, which is what the code did before, so the worst
/// case is the old cost plus a bounded probe.
///
/// `V` is what a tuple converted to, in whatever form its caller writes: a colour where the
/// caller still has arithmetic to do on it, the three bytes of a pixel where it has none — which
/// is what [`convert_three`] and [`convert_four`] keep, so that a hit is a copy rather than three
/// roundings of a float (ADR 1469).
///
/// [`palette`]: super::unpack::palette
pub(super) struct SampleMemo<V = pdf_render::Color> {
    /// Packed sample tuple, with bit 32 set where the entry is occupied.
    keys: Vec<u64>,
    /// What that tuple converted to.
    values: Vec<V>,
}

impl<V: Copy> SampleMemo<V> {
    /// Smallest and largest table, in entries.
    ///
    /// Sized from the image rather than fixed, because both ends cost. A 2^18-entry table
    /// is 2.9 MB to allocate and zero, which a 16x16 icon should not pay; a 2^12-entry one
    /// collides constantly on a photograph. Measured on `issue19971.pdf`'s 2500x1364
    /// `ICCBased` photograph and on the whole corpus gate, which is the population of small
    /// images: 2^14 gives 1.68 G instructions on the photograph and 1.9 s on the gate, 2^16
    /// gives 1.36 G and 1.8 s, and a fixed 2^18 gives 1.05 G and 1.9 s — the gate paying
    /// back what the photograph saves. Sizing by the image takes both.
    const MIN_SLOTS: usize = 1 << 12;
    const MAX_SLOTS: usize = 1 << 18;

    /// A table proportioned to an image of `pixels` samples.
    ///
    /// A quarter of the pixel count, rounded up to a power of two: an image whose colours
    /// are all distinct cannot be helped by any table, and one with structure repeats long
    /// before a quarter of its pixels.
    pub(super) fn for_pixels(pixels: usize, empty: V) -> Self {
        let wanted = pixels.next_power_of_two() / 4;
        let slots = wanted.clamp(Self::MIN_SLOTS, Self::MAX_SLOTS);
        Self {
            keys: vec![0; slots],
            values: vec![empty; slots],
        }
    }

    /// Where a tuple lives, if it lives anywhere.
    ///
    /// Knuth's multiplicative hash on the packed tuple: the samples of one pixel are
    /// adjacent bytes, so the low bits alone would put every shade of one hue in one slot.
    fn slot(&self, key: u64) -> usize {
        let mixed = key.wrapping_mul(0x9E37_79B9_7F4A_7C15);
        // Shifted down to 24 bits before the mask, so a table smaller than 2^24 entries is
        // indexed by mixed bits rather than by the tuple's own low bytes.
        usize::try_from(mixed >> 40).unwrap_or(0) & (self.keys.len().saturating_sub(1))
    }

    pub(super) fn get(&self, key: u64) -> Option<V> {
        let at = self.slot(key);
        (self.keys.get(at) == Some(&key))
            .then(|| self.values.get(at).copied())
            .flatten()
    }

    pub(super) fn put(&mut self, key: u64, colour: V) {
        let at = self.slot(key);
        if let Some(slot) = self.keys.get_mut(at) {
            *slot = key;
        }
        if let Some(slot) = self.values.get_mut(at) {
            *slot = colour;
        }
    }
}

pub(super) fn channel(value: f32) -> u8 {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "clamped to 0.0..=1.0 and scaled, so the conversion is exact"
    )]
    {
        // `+ 0.5` and a truncating cast rather than `.round()`, which is the *same* answer
        // on this domain — the value is clamped non-negative, where round-half-away-from-zero
        // and round-half-up agree — and does not call `roundf`. This runs once per component
        // per pixel and callgrind put the library call at 10.7% of interpreting one
        // 2500x1364 photograph, 60 instructions a pixel to round three numbers.
        (value.clamp(0.0, 1.0) * 255.0 + 0.5) as u8
    }
}

/// `count` four-byte pixels built from `channels` channels each, with alpha set.
///
/// The frames that reach this are the ones [`decode_jpeg`] could not have the decoder write
/// directly: its components, widened into the raster a display list carries.
///
/// Filled with 255 so that alpha is already set and the loops below touch three bytes per pixel
/// instead of four.
///
/// This is written for speed, and the benchmark that says it had to be is worth recording: on
/// `22060_A1_01_Plans.pdf`, `callgrind` put an earlier version of it at 6.89 G instructions, 38%
/// of the whole run and nearly twice what `zune-jpeg` spent actually decoding the JPEG. The cost
/// was per pixel and structural — a `match` on the component count inside the loop, three
/// bounds-checked `get`s with `unwrap_or`, saturating index arithmetic, and an
/// `extend_from_slice` that re-checks capacity every time. Pairing two `chunks_exact` iterators
/// removes all four: the component count is decided once, the chunk lengths are known to the
/// compiler, and nothing is indexed.
///
/// [`decode_jpeg`]: super::dct::decode_jpeg
pub(super) fn widened(pixels: &[u8], count: usize, channels: usize) -> Result<Vec<u8>, ImageError> {
    let mut out = vec![255u8; count.saturating_mul(4)];
    match channels {
        1 => {
            for (destination, source) in out.chunks_exact_mut(4).zip(pixels.iter()) {
                if let Some(rgb) = destination.get_mut(..3) {
                    rgb.fill(*source);
                }
            }
        }
        3 => {
            for (destination, source) in out.chunks_exact_mut(4).zip(pixels.chunks_exact(3)) {
                if let Some(rgb) = destination.get_mut(..3) {
                    rgb.copy_from_slice(source);
                }
            }
        }
        // Four components stay four: they are `/ColorSpace`'s to interpret, and
        // `convert_channels` is where they become pixels. The alpha byte carries `k` until
        // then, which is why that conversion restores it rather than leaving it alone.
        4 => {
            for (destination, source) in out.chunks_exact_mut(4).zip(pixels.chunks_exact(4)) {
                destination.copy_from_slice(source);
            }
        }
        _ => {
            return Err(ImageError::UnsupportedColourSpace {
                space: format!("{channels}-component JPEG"),
            });
        }
    }
    Ok(out)
}

/// Applies §8.9.5.2's map to a raster that is already eight-bit RGB.
///
/// The `DCTDecode` route reaches this rather than [`unpack`], because `zune-jpeg` delivers
/// components rather than packed samples and the decoder's own colour transform has already
/// run. So the map is applied afterwards, over channels — which is the same arithmetic on
/// the same domain, since a JPEG component *is* an integer in 0 to 255.
///
/// `/Decode [1 0 1 0 1 0]` on a `DCTDecode` image is the one form of this the corpus states
/// (`issue7406.pdf`), and it was being dropped: this route never consulted the entry at all.
/// A greyscale JPEG's single component is written to all three channels by [`decode_jpeg`],
/// so the first pair governs each of them.
///
/// **A channel and a component are the same thing only where the space's components run from 0
/// to 1**, so this runs on the paths
/// [`convert_channels`] does *not* convert — a stencil, an unreadable space, and the device
/// spaces the decoder already delivered. Everywhere else the map is indexed in the space's own
/// component values, where a `Lab` lightness of 100 and an `Indexed` index of 255 both fit and
/// an eight-bit channel does not (ADR 0464).
///
/// [`unpack`]: super::unpack::unpack
/// [`decode_jpeg`]: super::dct::decode_jpeg
fn apply_decode_to_channels(
    document: &Document,
    dict: &Dictionary,
    components: usize,
    rgba: &mut [u8],
) {
    if document.get_key(dict, "Decode").as_array().is_none() {
        return;
    }
    // One or three components are device channels whatever `/ColorSpace` names, which is the
    // approximation this route takes and which both become RGB here; four are CMYK, and the
    // clause's default ranges are the same `[0 1]` per component either way. The space is
    // named rather than resolved because `Decode::read` wants it only for those defaults and
    // for how many pairs to expect.
    let space = if components == 4 {
        ColourSpace::Cmyk
    } else {
        ColourSpace::Rgb
    };
    let decode = Decode::read(document, dict, &space, 8);
    if decode.is_identity() {
        return;
    }
    let grey =
        matches!(document.get_key(dict, "Decode").as_array(), Some(items) if items.len() < 6);
    // A greyscale JPEG's one component was written to all three channels, so three is right
    // for one component as well as for three; four components are still four channels.
    let touched = if components == 4 { 4 } else { 3 };
    for pixel in rgba.chunks_exact_mut(4) {
        for (component, channel) in pixel.iter_mut().take(touched).enumerate() {
            let pair = if grey { 0 } else { component };
            *channel = decode.channel(pair, usize::from(*channel));
        }
    }
}

/// Converts a `DCTDecode` raster from the space its dictionary names into device RGB.
///
/// The other four routes decide what a sample means in [`unpack`], through
/// `crate::colour::ColourSpace::to_rgb`; this one had `zune-jpeg`'s components written
/// straight into the raster as though `/ColorSpace` said `DeviceGray` or `DeviceRGB`. For
/// most JPEGs it does — but §8.6.5.5's `ICCBased` and §8.6.5.6's `/DefaultRGB` both put a
/// space there that means something else, and a page cannot tell you which it got.
///
/// A JPEG's components are eight-bit integers, so the conversion is a lookup for a
/// one-component space and a per-pixel call for three. Nothing runs at all where the space
/// is the device one the decoder already delivered, which is every corpus JPEG but a
/// handful.
///
/// **§8.9.5.2's `/Decode` is applied here too, and that is what makes the conversion right**
/// rather than an extra job taken on. A sample is an integer and a component is a value in the
/// space's own range, and Table 88 says which range: this route used to divide by 255 and let
/// [`apply_decode_to_channels`] remap eight-bit channels beforehand, which is the same thing
/// only for a space whose components run from 0 to 1. Where the two paths below convert, they
/// index the map; where they return early — a stencil, a space this crate could not read, and
/// the device spaces the decoder already delivered — the channel remap is still what a
/// `/Decode` array means, and it runs.
///
/// [`unpack`]: super::unpack::unpack
pub(super) fn convert_channels(
    at: Dictionaries,
    is_mask: bool,
    components: usize,
    rgba: &mut [u8],
    into: &Conversion,
) -> Result<(), ImageError> {
    let channels_only = |rgba: &mut [u8]| {
        apply_decode_to_channels(at.document, at.dict, components, rgba);
    };
    if is_mask {
        channels_only(rgba);
        return Ok(());
    }
    // An unreadable space is reported by the ordinary route rather than twice.
    let Ok(space) = colour_space(at.document, at.dict, at.resources, into) else {
        channels_only(rgba);
        return Ok(());
    };
    let space = match space {
        // Grey and RGB are what the decoder already produced. `DeviceCMYK` is *not*:
        // `decode_jpeg` hands over the four components untouched, on purpose, so that
        // §8.6.4.4's four numbers become a pixel where every other colour does. Inside a
        // mask group none of the three is what the decoder produced — `ColourSpace::reduced`
        // does not reduce there — so this early return is unreachable and the arms below
        // convert every sample, which is what the group is composited in.
        ColourSpace::Gray | ColourSpace::Rgb | ColourSpace::Mask => {
            channels_only(rgba);
            return Ok(());
        }
        ColourSpace::Cmyk => crate::colour::ColourSpace::Cmyk,
        ColourSpace::Resolved(space) => space,
    };

    // §8.9.5.2's map from a sample to a component value, which this route used to be a flat
    // division by 255 instead of:
    //
    // > Samples with a value of 0 shall be mapped to D min … those with intermediate values
    // > shall be mapped linearly between D min and D max
    //
    // Table 88 sets D min and D max per *space* where the dictionary states no `/Decode`, and
    // two of the families it lists do not run from 0 to 1: an `Indexed` space's default is
    // `[0 2^n − 1]`, and a `Lab` space's is `[0 100 amin amax bmin bmax]`, whose lightness is a
    // percentage and whose two chromatic axes take the space's own `/Range`. Dividing a sample
    // by 255 hands `Lab` a lightness of at most 1 out of 100, which is a photograph drawn as a
    // **black rectangle** with nothing reported — a crawled schoolbook page, +32.097 of 255
    // against three references agreeing to 1.8 (ADR 0464). The `Indexed` half of the same hole
    // is one arm along (ADR 0448); this is its other two arms.
    //
    // The table is the same object the unfiltered route unpacks through, so `/Decode` reaches
    // this route the way it reaches that one, in the space's own component values rather than
    // as an eight-bit channel remap — which is what [`apply_decode_to_channels`] does for the
    // device spaces above, where the two are the same thing.
    let decode = Decode::read(
        at.document,
        at.dict,
        &ColourSpace::Resolved(space.clone()),
        8,
    );
    // The raster's channel count and the space's need not match, and where they do not it is
    // usually not a defect: a greyscale JPEG is delivered as three equal channels, so a
    // one-component space reads the first of them. What cannot be reconciled is a space of
    // three or four components against a codestream of a different number, which is the
    // dictionary and the codestream contradicting each other.
    match (space.components(), components) {
        (1, _) => {
            // An `Indexed` space takes an *index* where every other one-component space takes
            // a component in 0 to 1, which is §8.6.6.3:
            //
            // > A PDF reader shall treat each sample value as an index into the colour table
            // > and shall use the colour value it finds there.
            //
            // Dividing by 255 first sends every index of a 256-entry table onto 0 or 1, so a
            // scan whose samples sit near 250 draws in the palette's *darkest* two entries —
            // a whole page of black where the table says white. `jpx_samples_to_rgba` states
            // the same rule for the JPEG 2000 route and `Decode`'s default range states it
            // for the other three; this route is the one that had it missing.
            // The same table `palette` builds, for the same reason: 256 possible samples
            // against a photograph's millions.
            let table: Vec<[u8; 3]> = (0..=255u8)
                .map(|value| {
                    let colour = into.paint(&space, &[decode.value(0, usize::from(value))]);
                    [channel(colour.r), channel(colour.g), channel(colour.b)]
                })
                .collect();
            for pixel in rgba.chunks_exact_mut(4) {
                let grey = usize::from(pixel.first().copied().unwrap_or(0));
                if let Some(rgb) = pixel.get_mut(..3)
                    && let Some(converted) = table.get(grey)
                {
                    rgb.copy_from_slice(converted);
                }
            }
            Ok(())
        }
        (3, 3) => {
            convert_three(&space, &decode, rgba, band_pixels(rgba.len() / 4), into);
            Ok(())
        }
        (4, 4) => {
            convert_four(&space, &decode, rgba, band_pixels(rgba.len() / 4), into);
            Ok(())
        }
        (wanted, got) => Err(ImageError::UnsupportedColourSpace {
            space: format!("a {wanted}-component space on a JPEG of {got} components"),
        }),
    }
}
