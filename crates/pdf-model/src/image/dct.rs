//! `DCTDecode` (ISO 32000-2 §7.4.8): a JPEG codestream decoded in this process by `zune-jpeg`.
//!
//! The frame's own markers decide what the dictionary cannot — the component count (ADR 0266),
//! the dimensions (ADR 0340), Table 13's colour transform and Adobe's marker, a `DNL` height —
//! and a frame with restart intervals, or one cut at its rows, is decoded in bands ([`restart`],
//! [`cut`]).

use pdf_syntax::ImageStream;

use super::convert::{convert_channels, widened};
use super::dispatch::{SamplesOnGrid, matte_through_unpack};
use super::space::Dictionaries;
use super::unpack::Painting;
use super::{ImageError, InFlight, MAX_SAMPLES};

pub(super) mod cut;
pub(super) mod restart;

/// Decodes a `DCTDecode` image: §7.4.8's frame, converted from the space its dictionary names.
///
/// One of [`samples_of`]'s five routes, split out for the same reason the other four are: what a
/// route does with the bytes is its own, and everything around it is about masks.
///
/// [`samples_of`]: super::dispatch::samples_of
pub(super) fn decode_dct(
    at: Dictionaries,
    source: &ImageStream,
    grid: (u32, u32),
    painting: Painting,
    in_flight: InFlight,
) -> Result<SamplesOnGrid, ImageError> {
    let frame = dct_frame(at, source, in_flight)?;
    samples_of_frame(at, frame, grid, painting)
}

/// A `DCTDecode` image's frame, decoded by the codec and nothing more: what [`samples_of_frame`]
/// makes the image's samples of.
///
/// Apart from it because nothing in it reads the image's mask, so a `/Matte`'s mask can be decoded
/// beside it ([`decode_parts`], ADR 1481); the inversion that does read the mask is the next step's.
///
/// [`decode_parts`]: super::decode_parts
pub(super) fn dct_frame(
    at: Dictionaries,
    source: &ImageStream,
    in_flight: InFlight,
) -> Result<DecodedJpeg, ImageError> {
    // The codestream's grid rather than the dictionary's, on §7.4.8's own statement of
    // where a JPEG's dimensions live; [`decode_jpeg`] has the reading and says why the
    // two disagreeing costs no mark.
    //
    // Table 13's `/ColorTransform` is read out of the filter's own parameter dictionary
    // and nowhere else, because that is the one place both clauses mentioning it put a
    // filter's parameters — §7.4.1's "These optional parameters shall be specified by
    // the DecodeParms entry in the stream's dictionary", and this clause's own "the
    // parameter need not be present in the encoded data but shall be specified in the
    // filter parameter dictionary". ADR 1183.
    let stated = source
        .parms
        .as_ref()
        .map(|parms| at.document.get_key(parms, "ColorTransform"))
        .and_then(|value| value.as_integer());
    decode_jpeg(&source.data, stated, in_flight)
}

/// [`decode_dct`]'s second step: the frame [`dct_frame`] decoded, made the image's samples under
/// what the dictionary says — §8.9.6.4's colour key, §11.6.5.2's inversion, the colour space.
pub(super) fn samples_of_frame(
    at: Dictionaries,
    frame: DecodedJpeg,
    (width, height): (u32, u32),
    painting: Painting,
) -> Result<SamplesOnGrid, ImageError> {
    let DecodedJpeg {
        samples: mut rgba,
        components,
        grid,
        ..
    } = frame;
    // §8.9.6.4's ranges cover "colour components before decoding", which for this
    // filter are the bytes just answered with; the conversion below replaces them, so
    // the test is taken here and its answer applied afterwards. It cannot be applied
    // here as well: [`convert_channels`] writes the alpha byte of a four-component
    // frame, where it carries `k` until then, and would paint the sample back in.
    let masked = painting
        .colour_key
        .map(|ranges| jpeg_colour_key(&rgba, ranges));
    // §11.6.5.2's inversion, before the colour conversion as the clause orders it, and
    // computed where every other route computes it (`matte_through_unpack`).
    let mut shortfall = None;
    let (matted, mask_in_alpha) = match painting.matte {
        Some(_) if !painting.is_mask => {
            match matte_through_unpack(
                at,
                &painting,
                (&mut rgba, components),
                grid,
                (width, height),
            )? {
                Ok(mask_in_alpha) => (true, mask_in_alpha),
                Err(sentence) => {
                    shortfall = Some(sentence);
                    (false, false)
                }
            }
        }
        _ => (false, false),
    };
    if !matted {
        convert_channels(at, painting.is_mask, components, &mut rgba, painting.into)?;
    }
    if let Some(masked) = masked {
        // The same answer [`unpack`] gives a masked sample: its position, and no
        // opacity at all.
        for (pixel, covered) in rgba.chunks_exact_mut(4).zip(masked) {
            if covered {
                pixel.fill(0);
            }
        }
    }
    Ok(SamplesOnGrid {
        rgba,
        grid,
        opacity_included: false,
        stencil_opacity: None,
        shortfall,
        mask_in_alpha,
    })
}

/// Which of a `DCTDecode` frame's pixels §8.9.6.4's ranges cover, one flag per pixel.
///
/// The clause's test is on "all of its colour components before decoding", and §8.9.5.1's
/// Table 87 says what this filter delivers those as:
///
/// > a CCITTFaxDecode or JBIG2Decode filter shall always deliver 1-bit samples, a
/// > RunLengthDecode or DCTDecode filter shall always deliver 8-bit samples
///
/// So the bytes [`decode_jpeg`] answers with are the values the ranges are stated in, and
/// nothing has to be un-decoded to compare them.
///
/// **A flag per pixel rather than an opacity cleared in place**, which is the one thing about
/// this that is not obvious: the answer is wanted before [`convert_channels`] replaces the
/// components and applied after it, because that conversion writes the fourth byte of a
/// four-component frame — where `k` lives until then — and would paint a masked sample back
/// in. The flags cost a quarter of what the raster beside them does.
///
/// `ranges` has one entry per component of the image's *colour space*, which
/// [`colour_key_entry`] checked against the array's length; the first that many bytes of a
/// pixel are the components [`convert_channels`] goes on to read, in each of the three
/// pairings of space and frame it converts. A longer `ranges` than a pixel has bytes belongs
/// to a space that conversion refuses outright, so the flags it produces are never applied.
///
/// [`colour_key_entry`]: super::mask::colour_key_entry
fn jpeg_colour_key(rgba: &[u8], ranges: &[(u32, u32)]) -> Vec<bool> {
    rgba.chunks_exact(4)
        .map(|pixel| {
            ranges.iter().enumerate().all(|(component, (low, high))| {
                pixel.get(component).is_some_and(|sample| {
                    let raw = u32::from(*sample);
                    raw >= *low && raw <= *high
                })
            })
        })
        .collect()
}

/// Undoes Adobe's APP14 transform 2, turning four YCCK channels into the four CMYK ones.
///
/// # Why the colour space is chosen here rather than left to the decoder
///
/// `zune-jpeg` converts to RGB by default, and for a four-component codestream that means it
/// applies a **CMYK to RGB conversion of its own** — `blinn_8x8(c, k)`, which is
/// `(1 − C)(1 − K)` on samples it assumes are stored inverted, the convention a *standalone*
/// Adobe CMYK JPEG follows. Two things are wrong with letting that happen. It is a second
/// route from colour to pixels, which trap 6 in `doc/HANDOVER.md` forbids outright —
/// `ColourSpace::to_rgb` is the only place a colour becomes RGB — and inside a PDF the
/// inversion is not the marker's to state. §8.9.5.2's `/Decode` array says what a sample
/// means, its Table 87 default for `DeviceCMYK` is `[0 1 0 1 0 1 0 1]`, and §7.4.8 defers to
/// Adobe Technical Note #5116 for *markers* and the YCbCr/YCCK colour transform, not for the
/// polarity of a sample.
///
/// `cmykjpeg.pdf` is the corpus witness and it settles it: an Adobe-marked four-component
/// JPEG with no `/Decode`, whose samples are ordinary CMYK. Read as inverted, its sky comes
/// out **black**; read as the clause states, it is the photograph all four references draw.
/// So a CMYK codestream is asked for as CMYK and converted where every other colour is.
/// Adobe's APP14 transform 2, in place: four YCCK channels become the four CMYK ones.
///
/// §7.4.8 hands a `DCTDecode` codestream's syntax to ISO/IEC 10918, and the four-component
/// transform is Adobe's extension to it (APP14, `transform = 2`): the first three channels carry
/// a JFIF luminance-chrominance transform of what would otherwise be the first three *inverted*
/// CMYK channels, and the fourth carries K unchanged. So undoing it is JFIF's own YCbCr → RGB
/// followed by the inversion the convention already assumes:
///
/// ```text
/// C = 255 − R    M = 255 − G    Y = 255 − B    K = K
/// ```
///
/// **The inversion is not undone here, and that is the point.** An Adobe four-component JPEG
/// stores CMYK inverted whichever transform it uses, and a PDF that means the ordinary reading
/// says so with `/Decode [1 0 1 0 1 0 1 0]` — §8.9.5.2's entry, applied by
/// [`apply_decode_to_channels`] one step later. A decoder that un-inverted here would undo the
/// file's `/Decode` twice. What this function owes is only that transform 2 and transform 0
/// deliver the *same convention*, which is what libjpeg's `ycck_cmyk_convert` also does.
///
/// **Why not ask `zune-jpeg` for CMYK directly**: it has no `YCCK → CMYK` conversion. Its two
/// YCCK arms both go to RGB and composite the black channel in on the way, which throws away the
/// component §8.9.5.1 needs `/ColorSpace` to interpret. Asking for `YCCK` out takes the raw four.
///
/// The witness is outside this repository — a 92-page commercial catalogue whose every page is one
/// such image, and which drew nothing at all until this (`doc/todo/28`).
///
/// [`apply_decode_to_channels`]: super::convert::apply_decode_to_channels
pub(super) fn ycck_to_cmyk(pixels: &mut [u8]) {
    for pixel in pixels.chunks_exact_mut(4) {
        let (Some(y), Some(cb), Some(cr)) = (
            pixel.first().copied(),
            pixel.get(1).copied(),
            pixel.get(2).copied(),
        ) else {
            continue;
        };
        let [red, green, blue] = jfif_inverse(y, cb, cr);
        if let Some(three) = pixel.get_mut(..3) {
            three.copy_from_slice(&[
                255_u8.saturating_sub(red),
                255_u8.saturating_sub(green),
                255_u8.saturating_sub(blue),
            ]);
        }
    }
}

/// Undoes §7.4.8's Table 13 three-component transform, in place: `YCbCr` channels become `RGB`.
///
/// > If the image has three colour components, RGB values shall be transformed to YCbCr before
/// > encoding and from YCbCr to RGB after decoding.
///
/// `zune-jpeg` does this itself for every frame it reads *as* `YCbCr`, and it is the whole of
/// what this filter's default case asks for — so the only samples that reach here are a frame
/// the decoder read as already `RGB` (its component identifiers spell the three letters) over
/// which Table 13's entry states the transform anyway. [`decode_jpeg`] has that reading.
fn ycbcr_to_rgb(pixels: &mut [u8]) {
    for pixel in pixels.chunks_exact_mut(3) {
        let (Some(y), Some(cb), Some(cr)) = (
            pixel.first().copied(),
            pixel.get(1).copied(),
            pixel.get(2).copied(),
        ) else {
            continue;
        };
        pixel.copy_from_slice(&jfif_inverse(y, cb, cr));
    }
}

/// One sample through ITU-T T.871's inverse of the JFIF luminance-chrominance transform.
///
/// The one implementation of it in this crate, because both of Table 13's transforms are built
/// on it: the three-component one is this and nothing else, and the four-component one is this
/// over the first three channels with the inversion an Adobe four-component JPEG assumes left in
/// place ([`ycck_to_cmyk`] says why that inversion is not undone).
///
/// The clamp is the standard's own, since the transform's range exceeds a byte.
fn jfif_inverse(y: u8, cb: u8, cr: u8) -> [u8; 3] {
    let (y, cb, cr) = (f32::from(y), f32::from(cb) - 128.0, f32::from(cr) - 128.0);
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "clamped to 0..=255 in the same expression, which is what makes it a byte"
    )]
    let clamp = |value: f32| value.clamp(0.0, 255.0).round() as u8;
    [
        clamp(1.402_f32.mul_add(cr, y)),
        clamp((-0.714_136_f32).mul_add(cr, (-0.344_136_f32).mul_add(cb, y))),
        clamp(1.772_f32.mul_add(cb, y)),
    ]
}

/// Whether §7.4.8's Table 13 says a codestream's samples are to be transformed, where it says.
///
/// `None` is the ranking ending without an answer, and the codestream's own declaration answers
/// there — which is what the decoder inferred. Three things produce it: a component count Table 13
/// describes nothing for, a stated code naming a channel count the frame does not have, and the
/// clause's third case, where the default *is* stated and this tree departs from it for a reason
/// ADR 1183 records.
///
/// The ranking is the clause's, in its order:
///
/// - **The marker.** "[T]he colours shall be transformed, or not, after the DCT decoding has
///   been performed according to the value provided in the encoded data and the value of this
///   dictionary entry shall be ignored." The *value* is Adobe's rather than Table 13's, which is
///   what §7.4.8 defers to Adobe Technical Note #5116 for — 0 is no transformation, 1 names a
///   three-channel one and 2 a four-channel one — and a code naming a channel count the frame
///   does not have states nothing about this frame.
/// - **The entry**, "[i]f the Adobe-defined marker code in the encoded data indicating the
///   ColorTransform value is not present". Its values *are* Table 13's, and Table 13 states them
///   in terms of the frame: 1 is `YCbCr → RGB` for three components and `YCbCrK → CMYK` for
///   four, 0 is "No transformation", and the option "shall be ignored if the image has one or
///   two colour components".
/// - **The default**, with neither: "1 if the image has three components and 0 otherwise".
#[expect(
    clippy::doc_markdown,
    reason = "verbatim quotations: Table 13 spells ColorTransform without backticks"
)]
fn colour_transform(data: &[u8], components: u8, stated: Option<i64>) -> Option<bool> {
    // "This option shall be ignored if the image has one or two colour components", and a count
    // outside three and four is one no part of Table 13 describes.
    if components != 3 && components != 4 {
        return None;
    }
    match adobe_transform(data) {
        Some(0) => return Some(false),
        Some(1) => return (components == 3).then_some(true),
        Some(2) => return (components == 4).then_some(true),
        // A code Adobe's marker does not define says nothing, and the entry is still ignored:
        // the marker "indicating the ColorTransform value" is present.
        Some(_) => return None,
        None => {}
    }
    // Table 13 gives the entry two values and describes no others, so an entry stating anything
    // else has specified no transformation code and falls through to the default — the same
    // reading ADR 1177 gave a producer who writes the name where §7.4.1 puts no parameter.
    //
    // And with no entry either, the answer would be the clause's default, `Some(components == 3)`.
    // It is `None` instead, and that is this tree's one departure from Table 13: where the frame's
    // component identifiers spell `R`, `G`, `B` the decoder reads the samples as already
    // transformed, and this tree keeps that reading against the clause's default of 1, on a page
    // three other renderers agree with us about. ADR 1183 prices it.
    match stated {
        Some(0) => Some(false),
        Some(1) => Some(true),
        _ => None,
    }
}

/// The transform code of a `DCTDecode` codestream's Adobe `APP14` marker segment, if it has one.
///
/// Walks ISO/IEC 10918-1's marker segments to the first scan rather than searching for bytes,
/// for the reason [`defined_number_of_lines`] and `filter::jpeg_extent` do: an `APPn` segment
/// may carry an entire second JPEG, whose own `APP14` is not this image's. Only a segment
/// beginning with Adobe's `Adobe` identifier counts, which is the same test the decoder makes.
fn adobe_transform(data: &[u8]) -> Option<u8> {
    /// Walks the header segments, answering `None` on a codestream that runs out or is not one.
    fn walk(data: &[u8]) -> Option<u8> {
        if data.get(..2)? != [0xFF, 0xD8] {
            return None;
        }
        let mut at = 2_usize;
        loop {
            // A marker is a run of `FF` and the byte after it.
            if data.get(at)? != &0xFF {
                return None;
            }
            while data.get(at) == Some(&0xFF) {
                at = at.checked_add(1)?;
            }
            let marker = *data.get(at)?;
            at = at.checked_add(1)?;
            match marker {
                // `SOS`: everything after this is entropy-coded data, and a header segment
                // cannot follow it.
                0xDA => return None,
                // The standalone markers, which carry no length.
                0x01 | 0xD0..=0xD9 => continue,
                _ => {}
            }
            // The two-byte length counts itself.
            let length = usize::from(u16::from_be_bytes([
                *data.get(at)?,
                *data.get(at.checked_add(1)?)?,
            ]));
            let payload = data.get(at.checked_add(2)?..at.checked_add(length)?)?;
            // `Adobe`, a two-byte version, two-byte `flags0`, two-byte `flags1`, then the
            // transform byte — so a segment too short to hold one is not the marker either.
            if marker == 0xEE && payload.get(..5) == Some(b"Adobe") {
                return payload.get(11).copied();
            }
            at = at.checked_add(length)?;
        }
    }

    walk(data)
}

/// What a `DCTDecode` codestream answered with.
pub(super) struct DecodedJpeg {
    /// Straight-alpha RGBA8 for one and three components; four components stay four, with `k`
    /// in the fourth byte until [`convert_channels`] runs.
    pub(super) samples: Vec<u8>,
    /// How many components the frame carries, which is what interprets [`Self::samples`].
    components: usize,
    /// How many components the frame header states: [`Self::components`] except for a grey
    /// frame, which is delivered as three equal channels and states one.
    pub(super) stated_components: usize,
    /// The grid the *codestream* states, which is not always the dictionary's.
    pub(super) grid: (u32, u32),
}

/// The offset of the next `FF` at or after `from`, or `None` where the data ends without one.
///
/// **This is where the walk over a codestream's entropy-coded data spends nearly all of its
/// time, and the byte it looks for is rare.** ISO/IEC 10918-1 section B.1.1.5 has an encoder
/// stuff a zero byte after every `FF` its entropy coder produces, so inside a scan the byte
/// stands only at a restart, at a stuffed pair, or at the marker that ends the scan — a few
/// thousand places in a five-megabyte photograph, against five million bytes that are none of
/// them. A byte-at-a-time loop therefore pays a bounds check and a branch per byte for an answer
/// it gets every few thousand: the walk cost 70.1 M instructions of interpreting a
/// five-megapixel photograph's page and reading eight bytes at a time costs 20.6 M
/// (ADR 1271, `examples/callgrind_interpret`).
///
/// The word test is the classic one for a zero byte: `x - 0x0101…` borrows into the high bit of
/// a lane only where that lane is zero, so `(x - 0x0101…) & !x & 0x8080…` is non-zero exactly
/// when some lane of `x` is zero. Applied to the complement of the data, a zero lane is an `FF`.
/// The lane itself is then found by looking, so that a word the test lets through costs a second
/// look rather than a wrong answer.
fn next_ff_byte(data: &[u8], from: usize) -> Option<usize> {
    /// One in every byte lane.
    const ONES: u64 = u64::from_ne_bytes([0x01; 8]);
    /// The high bit of every byte lane.
    const HIGHS: u64 = u64::from_ne_bytes([0x80; 8]);

    let tail = data.get(from..)?;
    let mut words = tail.chunks_exact(8);
    let mut at = from;
    for chunk in words.by_ref() {
        let mut eight = [0u8; 8];
        // `chunks_exact(8)` yields eight bytes, so the lengths agree and this cannot panic.
        eight.copy_from_slice(chunk);
        let bytes = u64::from_ne_bytes(eight);
        let holes = !bytes;
        if holes.wrapping_sub(ONES) & bytes & HIGHS != 0
            && let Some(offset) = chunk.iter().position(|&byte| byte == 0xFF)
        {
            return at.checked_add(offset);
        }
        at = at.checked_add(8)?;
    }
    let offset = words.remainder().iter().position(|&byte| byte == 0xFF)?;
    at.checked_add(offset)
}

/// One `FF` inside a scan, or the marker that ends it: where the `FF` stands, the code after
/// any fill bytes, and the offset after that code.
#[derive(Debug, Clone, Copy)]
struct ScanMarker {
    opens: usize,
    code: u8,
    resumes: usize,
}

/// What one walk over a codestream's first scan finds, which is everything its two readers
/// need: the `DNL` that may end it ([`defined_number_of_lines`]) and its restart intervals
/// (`restart::layout`). One walk serves both, so the 9.4 M instructions a walk over a
/// five-megapixel photograph's entropy-coded data costs are paid once before any band starts
/// rather than twice (ADR 1457).
#[derive(Debug)]
pub(super) struct FirstScan {
    /// Where the last frame header before the scan states `Y`.
    y_at: Option<usize>,
    /// Where the scan's entropy-coded data begins, after its header.
    data_starts: usize,
    /// Every stuffed byte and restart marker inside the data, in order.
    inside: Vec<ScanMarker>,
    /// The marker that ends the scan, or `None` where the data ends first.
    ends: Option<ScanMarker>,
}

/// Walks a codestream's marker segments to its first scan and that scan's entropy-coded data
/// to the marker that ends it, or `None` for a codestream with no scan, one whose `EOI` comes
/// first, or one whose data ends inside a run of fill bytes.
pub(super) fn first_scan(data: &[u8]) -> Option<FirstScan> {
    if data.get(..2)? != [0xFF, 0xD8] {
        return None;
    }
    let field = |at: usize| -> Option<u16> {
        Some(u16::from_be_bytes([
            *data.get(at)?,
            *data.get(at.checked_add(1)?)?,
        ]))
    };
    let mut y_at = None;
    let mut at = 2usize;
    loop {
        // A marker is a run of `FF` and the byte after it.
        while data.get(at) == Some(&0xFF) {
            at = at.checked_add(1)?;
        }
        let code = *data.get(at)?;
        at = at.checked_add(1)?;
        match code {
            // End of image, before any scan.
            0xD9 => return None,
            // Markers 10918-1 gives no length: TEM, a second SOI, the restarts.
            0x01 | 0xD8 | 0xD0..=0xD7 => {}
            _ => {
                let length = usize::from(field(at)?);
                if length < 2 {
                    return None;
                }
                // Every frame header — baseline, extended, progressive, lossless, and their
                // arithmetic-coded forms — puts `P` then `Y` after its length.
                if matches!(code, 0xC0..=0xC3 | 0xC5..=0xC7 | 0xC9..=0xCB | 0xCD..=0xCF) {
                    y_at = Some(at.checked_add(3)?);
                }
                at = at.checked_add(length)?;
                if code != 0xDA {
                    continue;
                }
                // The first scan's entropy-coded data: inside it, `FF` is followed only by a
                // stuffed `00` or a restart marker. The next marker is what ends the scan.
                let data_starts = at;
                let mut inside = Vec::new();
                loop {
                    let Some(opens) = next_ff_byte(data, at) else {
                        return Some(FirstScan {
                            y_at,
                            data_starts,
                            inside,
                            ends: None,
                        });
                    };
                    at = opens.checked_add(1)?;
                    while data.get(at) == Some(&0xFF) {
                        at = at.checked_add(1)?;
                    }
                    let marker = ScanMarker {
                        opens,
                        code: *data.get(at)?,
                        resumes: at.checked_add(1)?,
                    };
                    at = marker.resumes;
                    if !matches!(marker.code, 0x00 | 0xD0..=0xD7) {
                        return Some(FirstScan {
                            y_at,
                            data_starts,
                            inside,
                            ends: Some(marker),
                        });
                    }
                    inside.push(marker);
                }
            }
        }
    }
}

/// The number of lines a codestream's first scan ends by defining, and where the frame header
/// states it.
///
/// §7.4.8 puts a JPEG's dimensions in the encoded data — "[t]he values of these parameters,
/// which include the dimensions of the image […] shall be stored in the encoded data" — and
/// ISO/IEC 10918-1 gives the encoded data two places to state its number of lines: the frame
/// header's `Y`, and a `DNL` marker segment at the end of the first scan, which section B.2.5 of that
/// standard describes as defining *or redefining* `Y`. A frame header of `Y = 0` means the lines
/// are the `DNL`'s to define; a scanner that does not know the page length when it writes the
/// header writes `0` or `65535` there and the true count after the data. `zune-jpeg` reads the
/// header only: it refuses a `DNL` it meets before the scan and, meeting one after the scan's
/// data, pads the frame header's grid — so a 2480 × 3486 letter was drawn as the top five per
/// cent of a 2480 × 65535 image with grey below it, and both reference renderers drew the letter
/// (`poppler-61994-0.pdf`, `batch5/poppler`, ADR 0795).
///
/// Read off [`FirstScan`]'s walk — 10918-1's byte stuffing makes the end of entropy-coded data
/// findable without decoding, exactly as `pdf_syntax`'s `jpeg_extent` finds `EOI` — and `None`
/// for a codestream whose first scan is not followed by a `DNL`, which is nearly every one.
/// `Some` is the offset of the frame header's two-byte `Y`, the `DNL`'s `NL`, and the byte range
/// of the `DNL` segment itself, from the `FF` that opens it to the end of `NL`.
fn defined_number_of_lines(
    data: &[u8],
    scan: &FirstScan,
) -> Option<(usize, u16, std::ops::Range<usize>)> {
    let ends = scan.ends?;
    if ends.code != 0xDC {
        return None;
    }
    // `DNL`: the code, a length of 4, then `NL`, which 10918-1 bounds to 1..=65535.
    let code_at = ends.resumes.checked_sub(1)?;
    let lines = u16::from_be_bytes([
        *data.get(code_at.checked_add(3)?)?,
        *data.get(code_at.checked_add(4)?)?,
    ]);
    let closes = code_at.checked_add(5)?;
    (lines != 0).then_some((scan.y_at?, lines, ends.opens..closes))
}

/// The codestream as the decoder should read it: `Y` set to what the `DNL` defined, and the
/// `DNL` segment itself taken out.
///
/// `zune-jpeg` sizes the frame from the header alone, so the one way to have it decode the
/// number of lines the encoded data actually states is to write that number where it reads —
/// and it refuses a `DNL` marker it meets ("Parsing of the following header `DNL` is not
/// supported"), so once `Y` carries the count the segment that carried it is removed; a frame
/// whose header and `DNL` already agree still needs that. Borrowed and untouched for the
/// codestreams that state no `DNL`, which is nearly all of them; a copy six bytes shorter for
/// the rest. See [`defined_number_of_lines`], which reads `scan`, the walk of `data`.
pub(super) fn frame_as_defined<'a>(
    data: &'a [u8],
    scan: Option<&FirstScan>,
) -> std::borrow::Cow<'a, [u8]> {
    let Some((y_at, lines, dnl)) = scan.and_then(|scan| defined_number_of_lines(data, scan)) else {
        return std::borrow::Cow::Borrowed(data);
    };
    let mut defined = Vec::with_capacity(data.len());
    defined.extend_from_slice(data.get(..dnl.start).unwrap_or(data));
    defined.extend_from_slice(data.get(dnl.end..).unwrap_or_default());
    if let Some(field) = defined.get_mut(y_at..y_at.saturating_add(2)) {
        field.copy_from_slice(&lines.to_be_bytes());
    }
    std::borrow::Cow::Owned(defined)
}

/// Decodes §7.4.8's `DCTDecode` codestream.
///
/// # The dictionary's `/Width` and `/Height` do not have to agree with the codestream's
///
/// Table 87 requires both of them and a producer that writes two different numbers has written
/// a file no clause describes. What §7.4.8 does say is where a JPEG's own dimensions live, and
/// it puts them in the data:
///
/// > The values of these parameters, which include the dimensions of the image and the number
/// > of components per sample, are entirely under the control of the encoder and shall be
/// > stored in the encoded data. DCTDecode may obtain the parameter values it requires directly
/// > from the encoded data.
///
/// So the samples arrive on the codestream's grid and there is no second reading of them
/// available. The dictionary's numbers cost nothing to disagree with, because §8.9.5.1 puts
/// every image in the same place whatever its resolution — "the unit square of user space,
/// bounded by user coordinates (0, 0) and (1, 1), corresponds to the boundary of the image in
/// image space" — which is the same sentence §8.9.6.3 leans on when it lets an explicit mask
/// and its base image have different `/Width` and `/Height` outright. A grid is a sampling
/// density here and not a size on the page.
///
/// **This used to refuse the image outright**, on a comment saying the two "must agree,
/// because the display list carries the dictionary's and the samples carry the JPEG's". The
/// display list has carried the raster's own grid since a reduced JPEG 2000 decode needed it
/// to, so the premise had expired; the refusal cost a whole photograph where the disagreement
/// was one row of 1227 (`pdfCabinetOfHorrors/veraPDFHiResChangedHeight.pdf`, whose stated
/// defect is exactly that).
///
/// Compare §7.4.9, where the same disagreement is still a refusal and should be: that clause
/// states the constraint this one does not — "Width and Height shall match the corresponding
/// width and height values in the JPEG 2000 data".
///
/// The decoder options every `DCTDecode` codestream in this crate is read with.
///
/// **What this exists for is a single number, and it is a budget rather than a format limit.**
/// `zune-jpeg`'s `DecoderOptions` default a frame to 16384 samples in each axis, and §7.4.8 puts
/// no ceiling anywhere:
///
/// > The values of these parameters, which include the dimensions of the image and the number of
/// > components per sample, are entirely under the control of the encoder and shall be stored in
/// > the encoded data.
///
/// ISO/IEC 10918-1 gives each axis sixteen bits, so 65535 is the largest a codestream can *say*
/// and there is nothing below it for a reader to enforce on the clause's behalf. What bounds this
/// crate is [`MAX_SAMPLES`], which is an allocation this crate makes and has its argument written
/// beside it — and a library default that fires first has answered a question this project
/// answers deliberately. The witness is a crawled full-page scan 28341 rows tall, refused as
/// `Image height 28341 greater than height limit 16384` and drawn as a blank sheet where three
/// reference renderers agree (ADR 0454); 28341 × its width is far inside [`MAX_SAMPLES`].
///
/// The bomb the default was written against is refused by [`MAX_SAMPLES`] for the reason it
/// always was: a frame of 65535 by 65535 is 4.29 G samples against a budget of 268 M.
pub(super) fn jpeg_options() -> zune_jpeg::zune_core::options::DecoderOptions {
    /// The largest either axis of a JPEG frame can state, from ISO/IEC 10918-1's two-byte
    /// number of lines and samples per line.
    const AXIS: usize = 0xFFFF;

    zune_jpeg::zune_core::options::DecoderOptions::default()
        .set_max_width(AXIS)
        .set_max_height(AXIS)
}

/// # Table 13's `/ColorTransform`, and which of its three cases `stated` reaches
///
/// `stated` is the entry as the filter's parameter dictionary writes it, or `None` where it
/// writes none. §7.4.8's Table 13 ranks three cases and this function executes all three:
///
/// > If the encoding algorithm has inserted the Adobe-defined marker code in the encoded data
/// > indicating the ColorTransform value, then the colours shall be transformed, or not, after
/// > the DCT decoding has been performed according to the value provided in the encoded data and
/// > the value of this dictionary entry shall be ignored. If the Adobe-defined marker code in
/// > the encoded data indicating the ColorTransform value is not present then the value
/// > specified in this dictionary entry will be used. If the Adobe-defined marker code (APP14)
/// > in the encoded data indicating the ColorTransform value is not present and this dictionary
/// > entry is not present in the filter dictionary then the default value of ColorTransform
/// > shall be 1 if the image has three components and 0 otherwise.
///
/// An `APP14` segment therefore silences the entry, which [`adobe_transform`] is here to detect:
/// `zune-jpeg` reads the marker's value but reports it through the same `input_colorspace()` it
/// reports its own defaults through — a three-component frame is `YCbCr` whether the marker said
/// so or was absent — so the marker has to be read off the codestream separately or the clause's
/// three cases cannot be told apart. [`colour_transform`] is the ranking.
///
/// What the entry asks for is stated in terms of the frame's component count — "If the image
/// has three colour components, RGB values shall be transformed to YCbCr before encoding and
/// from YCbCr to RGB after decoding. If the image has four components, CMYK values shall be
/// transformed to YCbCrK before encoding and from YCbCrK to CMYK after decoding" — and a
/// sentence above both takes the small counts out: "This option shall be ignored if the image
/// has one or two colour components".
///
/// **The marker and the entry are both honoured over the frame's component identifiers.** A
/// three-component frame whose identifiers spell `R`, `G`, `B` is read by `zune-jpeg` as
/// already transformed, and no clause of ISO 32000-2 or of ISO/IEC 10918-1 gives an identifier
/// any meaning — the convention is `libjpeg`'s, and a convention no standard defines does not
/// outrank a value this clause reads. What is left to the identifiers is the third case alone,
/// where this tree departs from the clause's stated default of 1 for a page
/// `AMBIGUOUS_JPEG_COMPONENT_IDS` in `oracle.rs` holds and ADR 1183 prices.
///
/// # Errors
///
/// See [`ImageError`]. The grid the codestream states is bounded by [`MAX_SAMPLES`] here,
/// because it is no longer the dictionary's grid that [`decode_parts`] already checked.
///
/// [`decode_parts`]: super::decode_parts
#[expect(
    clippy::doc_markdown,
    reason = "verbatim quotations: Table 13 spells ColorTransform and YCbCrK without backticks"
)]
pub(super) fn decode_jpeg(
    data: &[u8],
    stated: Option<i64>,
    in_flight: InFlight,
) -> Result<DecodedJpeg, ImageError> {
    use zune_jpeg::zune_core::colorspace::ColorSpace;

    // One walk over the first scan serves the `DNL`, the restart intervals and the bands' bits;
    // a `DNL` taken out moves what follows it, so its copy is walked again (ADR 1457).
    let walked = first_scan(data);
    let data = frame_as_defined(data, walked.as_ref());
    let scan = match &data {
        std::borrow::Cow::Borrowed(_) => walked,
        std::borrow::Cow::Owned(defined) => first_scan(defined),
    };
    // `ZCursor` is the reader `zune-jpeg` wants; a bare slice does not implement its
    // trait because the decoder needs to seek.
    let mut decoder = zune_jpeg::JpegDecoder::new_with_options(
        zune_jpeg::zune_core::bytestream::ZCursor::new(&*data),
        jpeg_options(),
    );
    decoder
        .decode_headers()
        .map_err(|e| ImageError::Malformed {
            detail: format!("JPEG headers: {e}"),
        })?;
    let info = decoder.info().ok_or_else(|| ImageError::Malformed {
        detail: "JPEG has no frame".to_owned(),
    })?;
    let grid = (u32::from(info.width), u32::from(info.height));
    // A frame stating no samples in either axis is not an image at all, and the loops below
    // would answer it with an empty raster rather than saying so. `zune-jpeg` refuses it at the
    // header today — "Image width or height is set to zero, cannot continue" — so this states
    // the bound where the grid is chosen rather than being the only thing enforcing it.
    if info.width == 0 || info.height == 0 {
        return Err(ImageError::Malformed {
            detail: format!("JPEG frame is {}x{}", info.width, info.height),
        });
    }
    // The dictionary's grid passed [`MAX_SAMPLES`] in `decode_parts`; this one has not, and
    // since the samples are built on it rather than on the dictionary's it is the one that
    // decides what this allocates. A codestream may state up to 65535 in each axis.
    let samples = u64::from(grid.0).saturating_mul(u64::from(grid.1));
    if samples > MAX_SAMPLES {
        return Err(ImageError::TooLarge { samples });
    }
    // **Four components stay four**, whichever of the two ways a codestream spells them.
    //
    // `zune-jpeg`'s default output is RGB, and its own conversions for both four-component inputs
    // composite the black channel away — which is exactly what §8.9.5.1 must not have: the
    // dictionary's `/ColorSpace` is what interprets the components, and a decoder that had
    // already turned them into three has answered the clause's question for it.
    //
    // `CMYK` is Adobe's APP14 transform 0 and needs only asking. `YCCK` is transform 2, where the
    // first three channels are a luminance-chrominance transform of the other three and the
    // fourth is carried alongside — and asking for `YCCK` out gets the four *raw* channels, which
    // is why the conversion below is here rather than in the decoder.
    //
    // **The frame's component count decides this, and the APP14 transform code does not**
    // (ADR 0266). §7.4.8 says
    // where the number comes from:
    //
    // > The values of these parameters, which include the dimensions of the image and the number
    // > of components per sample, are entirely under the control of the encoder and shall be
    // > stored in the encoded data.
    //
    // and Table 13 states the transform in terms of that number rather than the other way round —
    // "If the image has four components, CMYK values shall be transformed to YCbCrK before
    // encoding and from YCbCrK to CMYK after decoding" — so transform 0, "No transformation",
    // says only that nothing was applied. `zune-jpeg` maps it to `CMYK` at the marker and
    // *defers* the correction to a three-component `RGB` until it reads the frame, so
    // `input_colorspace()` is provisional here and `info.components` is not. Asking a
    // three-component codestream for four components made every such image an
    // `Unimplemented colorspace mapping from RGB to CMYK` — 21 images over four documents of a
    // 4000-document web sample, whole photographs lost (ADR 0266).
    let input = decoder.input_colorspace();
    let four = info.components == 4 && matches!(input, Some(ColorSpace::CMYK | ColorSpace::YCCK));
    // Table 13's ranking, in the order the clause states it: the Adobe `APP14` segment, then the
    // filter's parameter dictionary, then the clause's own default. `None` is where the ranking
    // reaches its end without an answer, and there the codestream's own declaration is what
    // decides — which is `zune-jpeg`'s inference, already in `input`. ADR 1183.
    let transform = colour_transform(&data, info.components, stated);
    // Where this crate applies the transform rather than the decoder, the decoder is asked for the
    // frame's own components untouched — which `zune-jpeg` gives when the space asked for out is
    // the one it read in, since an input and an output that agree are a copy to it. That is every
    // four-component frame, and a three-component one whose transform this crate is about to
    // decide against the decoder's own reading.
    //
    // **A three-component frame's `input_colorspace()` is provisional and its untouched space is
    // therefore named rather than echoed**: the Adobe marker's transform 0 maps to `CMYK` at the
    // marker and is corrected to `RGB` only once the frame has been read (ADR 0266), so echoing
    // it here would ask a three-channel decode for four channels — which is not a conversion the
    // decoder has. `RGB` is the corrected answer in that case and in the identifiers' case alike,
    // and `YCbCr` in the remaining one.
    let untouched = if four {
        input
    } else if info.components == 3
        && (transform == Some(false) || (transform == Some(true) && input == Some(ColorSpace::RGB)))
    {
        Some(if input == Some(ColorSpace::YCbCr) {
            ColorSpace::YCbCr
        } else {
            ColorSpace::RGB
        })
    } else {
        None
    };
    if let Some(space) = untouched {
        decoder.set_options(jpeg_options().jpeg_set_out_colorspace(space));
    }

    let components = decoder
        .output_colorspace()
        .map_or(3, |space| space.num_components());
    // **Where the decoder can write this crate's raster itself, it is asked to.** The loops
    // below exist to widen the decoder's components into the four-byte pixels the display list
    // carries, and for a frame whose components the decoder is converting anyway that widening
    // is work done twice: `zune-jpeg` holds a `YCbCr → RGBA` and a `Luma → RGBA` beside the
    // three-channel forms — the same arithmetic, writing four lanes instead of three — so
    // asking for the four saves a second buffer, the pass that fills it and the pass that
    // clears it. It is 10.6% of interpreting a five-megapixel photograph's page (ADR 1271).
    //
    // Only where the decoder is converting: a frame this crate takes `untouched` is one whose
    // samples Table 13 says are *not* the decoder's to transform, and `zune-jpeg` has no
    // four-lane copy for those in any case. Four-component frames stay four for the reason
    // above — they are `/ColorSpace`'s to interpret.
    let direct = untouched.is_none()
        && matches!(components, 1 | 3)
        && matches!(input, Some(ColorSpace::Luma | ColorSpace::YCbCr));
    // **And it is asked of a decoder that has not read the headers yet**, which is not a
    // formality: `zune-jpeg` chooses its colour-conversion function while it reads them, and
    // `set_options` after that changes the shape of the output buffer without changing the
    // function that fills it. Asking the decoder above would have had a three-lane conversion
    // write into a four-lane raster — every row three quarters written and its colours walking
    // along it — and the instrument would have called it faster, because it is. The second
    // decoder re-reads the marker segments, which is a walk over a few hundred bytes; the
    // scan's data is still read once. ADR 1271.
    //
    // **And where its restart intervals allow, the frame is decoded in bands beside each other**,
    // each band a codestream of its own and the whole frame the one decoder's answer whenever the
    // codestream is not one `restart` admits (ADR 1433), or the rows an entropy pass finds where
    // nothing else of the page is decoding beside it (`cut`, ADR 1481): [`in_bands`].
    let mut pixels = if direct {
        let options = jpeg_options().jpeg_set_out_colorspace(ColorSpace::RGBA);
        scan.as_ref()
            .and_then(|scan| in_bands(&data, scan, options, in_flight))
            .map_or_else(
                || {
                    zune_jpeg::JpegDecoder::new_with_options(
                        zune_jpeg::zune_core::bytestream::ZCursor::new(&*data),
                        options,
                    )
                    .decode()
                },
                Ok,
            )
    } else {
        decoder.decode()
    }
    .map_err(|e| ImageError::Malformed {
        detail: format!("JPEG data: {e}"),
    })?;
    // Table 13's transform, in the two places it is this crate's to apply rather than the
    // decoder's. Four components always, because the decoder has no `YCCK → CMYK` of its own;
    // three only where the decoder read the frame as already transformed and the clause says
    // otherwise, which is [`colour_transform`]'s reading.
    //
    // Both arms are gated on the count the samples are actually on, for the reason the first was
    // when it was the only one: a *three*-component frame the decoder resolved to `YCbCr` would
    // have a `chunks_exact_mut(4)` walked across its pixel boundaries rather than refused.
    if components == 4 && transform.unwrap_or(input == Some(ColorSpace::YCCK)) {
        ycck_to_cmyk(&mut pixels);
    } else if components == 3 && transform == Some(true) && input == Some(ColorSpace::RGB) {
        ycbcr_to_rgb(&mut pixels);
    }
    let count = usize::from(info.width).saturating_mul(usize::from(info.height));
    // How many channels the decoder wrote for each pixel: four wherever it was asked for the
    // raster directly, and the frame's own component count otherwise.
    let delivered = if direct { 4 } else { components };
    // A buffer that is already four channels a pixel *is* the raster, and moving it into
    // another one of the same size would be a copy for nothing. `zune-jpeg` sizes its output as
    // the frame's grid times the channels asked for, so the length is the test for that.
    if delivered == 4 && pixels.len() == count.saturating_mul(4) {
        return Ok(DecodedJpeg {
            samples: pixels,
            components,
            stated_components: usize::from(info.components),
            grid,
        });
    }

    Ok(DecodedJpeg {
        samples: widened(&pixels, count, delivered)?,
        components,
        stated_components: usize::from(info.components),
        grid,
    })
}

/// A `DCTDecode` frame decoded in bands beside each other, or `None` where it is to be decoded
/// whole: at its restart intervals where they begin on MCU rows ([`restart`], ADR 1433), and
/// otherwise at the rows an entropy pass finds, where nothing else of the page is decoding beside
/// it ([`cut`], ADR 1481). The caller's RGBA output is what both were measured for.
fn in_bands(
    data: &[u8],
    scan: &FirstScan,
    options: zune_jpeg::zune_core::options::DecoderOptions,
    in_flight: InFlight,
) -> Option<Vec<u8>> {
    restart::decode(data, scan, options, 4).or_else(|| {
        (in_flight == InFlight::Alone)
            .then(|| cut::decode(data, scan, options, 4))
            .flatten()
    })
}

/// One baseline `DCTDecode` codestream decoded three ways into RGBA: whole by the one decoder,
/// in bands at its restart intervals (the `restart` module), and in bands at the rows an entropy
/// pass finds (the `cut` module) — each band plan `None` where it declines the frame.
///
/// Both band plans claim the same thing, that a frame they admit decodes to the whole decoder's
/// bytes, and each module's tests hold it on a handful of encoder outputs. This is the seam that
/// lets the `jpeg_bands` fuzz target hold it on codestreams nobody chose: [`banded_decodes`]
/// answers it below the production floor and at a band height the caller states, so that a frame
/// of a few hundred bytes is cut into several bands at all (ADR 1495).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BandedDecodes {
    /// The whole frame through the one decoder, or `None` where it refuses the codestream.
    pub whole: Option<Vec<u8>>,
    /// The frame in bands at its restart intervals, or `None` where `restart` declines it.
    pub at_restarts: Option<Vec<u8>>,
    /// The frame in bands at the rows the entropy pass finds, or `None` where `cut` declines.
    pub at_rows: Option<Vec<u8>>,
}

/// [`BandedDecodes`] for `data`, with bands of at least `band_lines` lines and no sample floor.
///
/// The codestream is read as `decode_jpeg` reads it — a `DNL` segment's count moved into the
/// frame header first, and a grid past `MAX_SAMPLES` answered with nothing — and every decode is
/// asked for RGBA under the options that function uses, which is the output both band plans were
/// argued and measured for.
#[must_use]
pub fn banded_decodes(data: &[u8], band_lines: u32) -> BandedDecodes {
    use zune_jpeg::zune_core::colorspace::ColorSpace;

    let walked = first_scan(data);
    let data = frame_as_defined(data, walked.as_ref());
    let scan = match &data {
        std::borrow::Cow::Borrowed(_) => walked,
        std::borrow::Cow::Owned(defined) => first_scan(defined),
    };
    let options = jpeg_options().jpeg_set_out_colorspace(ColorSpace::RGBA);
    // The grid `decode_jpeg` bounds by `MAX_SAMPLES` before it decodes is bounded here too, so
    // the seam allocates no more than the path it stands for.
    let mut headers = zune_jpeg::JpegDecoder::new_with_options(
        zune_jpeg::zune_core::bytestream::ZCursor::new(&*data),
        options,
    );
    let within = headers.decode_headers().is_ok()
        && headers.info().is_some_and(|info| {
            u64::from(info.width).saturating_mul(u64::from(info.height)) <= MAX_SAMPLES
        });
    if !within {
        return BandedDecodes {
            whole: None,
            at_restarts: None,
            at_rows: None,
        };
    }
    let whole = zune_jpeg::JpegDecoder::new_with_options(
        zune_jpeg::zune_core::bytestream::ZCursor::new(&*data),
        options,
    )
    .decode()
    .ok();
    let (at_restarts, at_rows) = scan.as_ref().map_or((None, None), |scan| {
        (
            restart::decode_at(&data, scan, (options, 4), 0, band_lines),
            cut::decode_at(&data, scan, (options, 4), 0, band_lines),
        )
    });
    BandedDecodes {
        whole,
        at_restarts,
        at_rows,
    }
}
