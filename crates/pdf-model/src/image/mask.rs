//! An image's `/Mask` (ISO 32000-2 §8.9.6.3, §8.9.6.4): an explicit stencil or colour-key
//! ranges, and the reports that say where one could not be applied.

use std::sync::Arc;

use pdf_render::{Image, SampleAlpha};
use pdf_syntax::{Dictionary, Document, ImageStream, Object, Stream};

use crate::colour::Conversion;

use super::cache::MaskCache;
use super::combine::{MaskSamples, combine_on_the_finer_grid};
use super::dispatch::positive_integer;
use super::jpx::jpx_sample_domain;
use super::space::colour_space;
use super::{Flattened, ImageError, MAX_SAMPLES, Parts, Picture, decode, decode_parts};

/// The combined grid above which §10.7.4's device-scale construction is the better one.
///
/// §8.9.6.3's two rasters "need not have the same resolution" and §11.6.5.2 Table 143 says
/// the same of a soft mask's, so combining either with its image means choosing a grid —
/// [`combine_on_the_finer_grid`] takes the finer of the two, which discards nothing either of
/// them carries and costs the product of the two larger dimensions. 2^24 samples is 64 MB of
/// RGBA, and past it a mask that can be read at a chosen grid is better read at the device's:
/// `issue16263.pdf`'s 2×2 image with a 34862×4332 `/SMask` asks for 604 MB combined and 19 MB
/// packed, for two distinct colours.
///
/// **It is a preference and not a ceiling** (ADR 0456): one number deciding both questions would
/// refuse a grid past it outright wherever [`eligible_for_the_device_scale`] says no — which is
/// every explicit `/Mask`, because a stencil is `/ImageMask true` and Table 87 gives it no
/// colour space. The ceiling is [`MAX_SAMPLES`]: a combined grid is a raster this crate
/// allocates, and one the size of an image the crate would have decoded on its own is not
/// larger for being a pair.
///
/// [`eligible_for_the_device_scale`]: super::soft_mask::eligible_for_the_device_scale
pub(super) const PREFER_DEVICE_SCALE_ABOVE: u64 = 1 << 24;

/// The bound above also caps the grey plane a codec-carrying soft mask decodes to, and it has to
/// admit the ones the corpus states — ISO 32000-2 §11.6.5.2, ADR 1232.
///
/// Reusing a measured constant for a third question costs one thing: a later round lowering it for
/// the grid it was measured on would take a mask out of that route with nothing saying so, and the
/// sentence §11.6.5.2's ledger row carries about the corpus would quietly stop being true. So the
/// measurement is a compile-time assertion rather than a comment — 6 522 400 samples in
/// `22060_A1_01_Plans.pdf`, the largest of the 16 masks standing behind an image codec over the
/// 974 tracked documents (trap 38).
const _: () = assert!(
    PREFER_DEVICE_SCALE_ABOVE >= 6_522_400,
    "the plane bound is under the corpus's largest codec-carrying soft mask"
);

/// The grid a mask and its image would be combined on, if it is one this will allocate.
///
/// The bound is on how much the combination costs *beyond the image*, and it has to be: a
/// mask on the image's own grid costs exactly what the image costs, which [`MAX_SAMPLES`] has
/// already admitted, so a flat limit would refuse a pair no larger than the picture being
/// drawn. `issue19517.pdf` is that case — a 12608×16806 scan with a mask of the same
/// dimensions — and it was reported for its mask's size for as long as this rule was the flat
/// one, which said nothing true about the mask. What a document *can* do with two dimensions
/// it controls is make the product of the two larger ones enormous while both rasters stay
/// small: 2^28 by 1 against 1 by 2^28 is two rasters of a quarter of a megabyte apiece and a
/// grid of 2^56, and that is what the ceiling refuses.
pub(super) fn combined_grid(
    width: u32,
    height: u32,
    mask_width: u32,
    mask_height: u32,
) -> Result<u64, u64> {
    let grid = u64::from(width.max(mask_width)).saturating_mul(u64::from(height.max(mask_height)));
    let image = u64::from(width).saturating_mul(u64::from(height));
    if grid > MAX_SAMPLES.max(image) {
        return Err(grid);
    }
    Ok(grid)
}

/// Whether combining on the finer grid is worth doing here, or the device's grid is better.
///
/// Separate from [`combined_grid`], which answers whether the combination is possible at all.
/// A caller with a second construction available asks this one first and falls back to the
/// combination; a caller with none asks only [`combined_grid`]. Splitting the two is what
/// stops a preference from reading as a refusal — see [`PREFER_DEVICE_SCALE_ABOVE`].
pub(super) fn worth_combining(width: u32, height: u32, mask_width: u32, mask_height: u32) -> bool {
    let grid = u64::from(width.max(mask_width)).saturating_mul(u64::from(height.max(mask_height)));
    let image = u64::from(width).saturating_mul(u64::from(height));
    grid <= PREFER_DEVICE_SCALE_ABOVE.max(image)
}

/// What an image's `/Mask` entry holds, once read.
///
/// ISO 32000-2 §8.9.6.1 lists it twice, because one key spells two mechanisms: `/Mask` is
/// either a second image saying *where* this one is painted (§8.9.6.3) or a set of ranges
/// saying *which colours* are not (§8.9.6.4). They share nothing but the key, which is why
/// this is read once, before the samples, and decided here rather than at three call sites.
pub(super) enum MaskEntry {
    /// No `/Mask`.
    Absent,
    /// Present, and superseded by the document's own precedence rule.
    ///
    /// §11.6.4.3, of an image's `/SMask`: "This mask, if present, shall override any explicit
    /// or colour key mask specified by the image dictionary's Mask entry", and of a non-zero
    /// `/SMaskInData`: "the embedded soft-mask shall override any explicit or colour key
    /// mask". So a `/Mask` beside either is not a gap and nothing is owed — reporting it
    /// would name a key the file itself has told us not to read.
    Overridden,
    /// §8.9.6.4: one inclusive range per colour component, in raw sample values.
    ColourKey(Vec<(u32, u32)>),
    /// §8.9.6.3: an image mask naming the areas of the base image that are painted.
    Explicit(Arc<Stream>),
    /// Present, and not applied. Carries the words the interpreter reports.
    ///
    /// A `/Mask` that is silently dropped paints a region the document says is not part of
    /// the image, and `colorkeymask.pdf` is the standing example: a red band all four
    /// reference renderers hide, drawn by us with nothing reported until this existed.
    Unusable(String),
}

/// Reads `/Mask`, deciding which of §8.9.6's two mechanisms it names and whether it applies.
///
/// Takes the whole stream rather than its dictionary because §8.9.6.4's range array is bounded
/// by a bit depth that, for one filter, is stated in the image data instead of the dictionary
/// (see [`colour_key_entry`]).
pub(super) fn mask_entry(
    document: &Document,
    stream: &Stream,
    resources: &Dictionary,
) -> MaskEntry {
    let dict = &stream.dict;
    let mask = document.get_key(dict, "Mask");
    if matches!(mask, Object::Null) {
        return MaskEntry::Absent;
    }
    // §11.6.4.3 orders the three mechanisms an image dictionary can carry, and `/Mask` is
    // last of them. Checked before anything is read out of it, so that a file writing both
    // cannot have the loser applied.
    let overridden = document.get_key(dict, "SMask").as_stream().is_some()
        || document
            .get_key(dict, "SMaskInData")
            .as_integer()
            .is_some_and(|value| value != 0);
    if overridden {
        return MaskEntry::Overridden;
    }
    match &mask {
        Object::Null => MaskEntry::Absent,
        Object::Array(items) => colour_key_entry(document, stream, resources, items),
        _ => mask.as_stream().map_or_else(
            || MaskEntry::Unusable("/Mask is neither an image mask nor a range array".to_owned()),
            |stream| explicit_entry(document, dict, stream),
        ),
    }
}

/// Reads §8.9.6.4's range array against the image it masks.
///
/// > For colour key masking, the value of the Mask entry shall be an array of 2 × 𝑛
/// > integers, [min1max1 …min𝑛max𝑛] , where n is the number of colour components in the
/// > image's colour space. Each integer shall be in the range 0 to 2 BitsPerComponent - 1,
/// > representing colour values before decoding with the Decode array.
///
/// Both "shall"s are checked rather than assumed, because an array of the wrong length would
/// otherwise mask by whichever components happened to line up.
///
/// # `JPXDecode`, whose domain is stated in the image data rather than in the dictionary
///
/// §8.9.5.1's Table 87 withdraws the entry the blockquote above bounds these integers by, for
/// this one filter:
///
/// > If the image stream uses the JPXDecode filter, this entry is optional and shall be
/// > ignored if present. The bit depth is determined by the PDF processor in the process of
/// > decoding the JPEG 2000 image.
///
/// **That sentence says the depth is determined, not that there is none**, and §7.4.9 says
/// where it is determined from:
///
/// > These packagings contain all the information needed to properly interpret the image data,
/// > including the colour space, bits per component, and image dimensions.
///
/// [`crate::jpeg2000`] reads exactly that, from the `ihdr` and `bpcc` boxes and the codestream's
/// `SIZ` marker, without decoding a sample. So the question this function has to answer is not
/// whether a domain exists but whether the samples the comparison will see are still in it, and
/// that has two answers:
///
/// - **An `Indexed` dictionary space is exact at any depth.** §7.4.9 gives `/ColorSpace`
///   precedence over the codestream's own colour specifications, so the confined decoder is
///   asked for the codestream's sample values unscaled — an index stretched to eight bits is a
///   different index — and §8.6.6.3 caps `hival` at 255. Those raw indices are the integers the
///   ranges are stated over.
/// - **Otherwise the decoder stretches every sample to eight bits**, which is the identity where
///   the codestream declares eight unsigned bits per component and a remapping otherwise. Where
///   it is a remapping, the integers the file wrote are in a domain its samples have left, and
///   §8.9.5.2's "can have different values per colour component" means there may be no single
///   domain to map them from. That case is reported, naming the depth found, rather than
///   guessed at.
///
/// **A `JPXDecode` image is not refused on the reading that Table 87 takes the domain away.** It
/// does not; §7.4.9 hands it to the data, and this crate reads the data in the module above. No
/// document in any corpus this tree holds states the pair — `examples/colour_key_mask_census` over
/// 90 535 finds 660 colour keys and not one `JPXDecode` or `JBIG2Decode` among them — so both arms
/// are pinned by generated fixtures in `tests/image_masks.rs` and by nothing else. ADR 1121.
///
/// **The lossy filters are not refused.** §8.9.6.4 opens with a `shall`
/// addressed to whoever paints the image:
///
/// > Samples in the image that fall within this range shall not be painted, allowing the
/// > existing background to show through.
///
/// and says of the lossy pair only that
///
/// > When colour key masking is specified, the use of a DCTDecode or lossy JPXDecode filter
/// > for the stream can produce unexpected results.
///
/// with NOTE 2 explaining that quantisation can leave a sample outside the range its producer
/// meant it to fall in. That is a warning about a picture, addressed to a writer choosing a
/// filter; it is not a permission to leave the `shall` unmet, and §7.4.8 states no such
/// exclusion either. Refusing on it would be this reader's choice wearing the clause's clothes
/// (ADR 0832).
///
/// **Where the test is taken is settled by §8.9.5.1's Table 87 rather than by this crate's
/// routing**, which is what made the withdrawal a small change:
///
/// > a CCITTFaxDecode or JBIG2Decode filter shall always deliver 1-bit samples, a
/// > RunLengthDecode or DCTDecode filter shall always deliver 8-bit samples
///
/// So the bilevel pair's samples are the packed bits [`unpack`] already reads, and a
/// `DCTDecode` frame's are the bytes [`decode_jpeg`] answers with, before
/// [`convert_channels`] turns them into device RGB.
///
/// [`unpack`]: super::unpack::unpack
/// [`decode_jpeg`]: super::dct::decode_jpeg
/// [`convert_channels`]: super::convert::convert_channels
fn colour_key_entry(
    document: &Document,
    stream: &Stream,
    resources: &Dictionary,
    items: &[Object],
) -> MaskEntry {
    let dict = &stream.dict;
    if matches!(document.get_key(dict, "ImageMask"), Object::Boolean(true)) {
        return MaskEntry::Unusable(
            "colour-key /Mask on an image mask, which has no colour components".to_owned(),
        );
    }
    // Asked for its component count and nothing else, so what the samples are composited
    // into does not enter: `Compositing::Device` is the question rather than an assumption.
    let Ok(space) = colour_space(document, dict, resources, &Conversion::device()) else {
        return MaskEntry::Unusable(
            "colour-key /Mask on an image whose colour space this cannot read".to_owned(),
        );
    };
    let jpeg_2000 = image_codec(document, dict).as_deref() == Some("JPXDecode");
    let jpx_bits = if jpeg_2000 {
        match jpx_sample_domain(document, stream, &space) {
            Ok(bits) => Some(bits),
            Err(refusal) => return MaskEntry::Unusable(refusal),
        }
    } else {
        None
    };

    let values: Vec<i64> = items
        .iter()
        .map(|item| document.resolve(item))
        .filter_map(|item| item.as_integer())
        .collect();
    let components = space.components();
    if values.len() != components.saturating_mul(2) {
        return MaskEntry::Unusable(format!(
            "colour-key /Mask has {} entries against a {components}-component image",
            values.len()
        ));
    }

    // Table 87: for a JPEG 2000 image this entry "shall be ignored if present", so the domain
    // the ranges are bounded by is the one the codestream states and the decoder delivers,
    // which [`jpx_sample_domain`] has just read out of its headers (ADR 1242).
    let bits = jpx_bits.unwrap_or_else(|| {
        u32::try_from(
            document
                .get_key(dict, "BitsPerComponent")
                .as_integer()
                .unwrap_or(8),
        )
        .unwrap_or(8)
    });
    let highest = i64::from(1u32.checked_shl(bits).unwrap_or(u32::MAX).saturating_sub(1));
    let mut ranges = Vec::with_capacity(components);
    for pair in values.chunks_exact(2) {
        let (Some(&low), Some(&high)) = (pair.first(), pair.get(1)) else {
            continue;
        };
        if low < 0 || high > highest {
            return MaskEntry::Unusable(format!(
                "colour-key /Mask range {low}..{high} is outside 0..{highest} at {bits} bits \
                 per component"
            ));
        }
        // A reversed pair is not malformed: `min i ≤ c i ≤ max i` is then satisfied by no
        // sample, so the range masks nothing, which is a statement a file may make.
        ranges.push((
            u32::try_from(low).unwrap_or(0),
            u32::try_from(high).unwrap_or(0),
        ));
    }
    MaskEntry::ColourKey(ranges)
}

/// Reads §8.9.6.3's explicit mask against the image it masks.
///
/// Two conditions decide whether it applies, and only the first is the clause's:
///
/// - **It has to be a stencil.** The clause admits "an image mask, as described in
///   subclause 8.9.6.2", and §8.9.6.2 defines one as an image `XObject` whose `/ImageMask`
///   entry is true. Table 87 says the same of the entry. A stream that carries a colour space and
///   says nothing about `/ImageMask` is therefore outside what `/Mask` is defined to hold,
///   and it is reported rather than interpreted.
///
///   `issue6621.pdf` is why that sentence is a rule rather than a formality. Its `/Mask` is
///   a one-bit `DeviceGray` image with no `/ImageMask`, and the stencil reading was written,
///   run, and thrown away on the evidence of the page: §8.9.6.2's "a sample value of 0 shall
///   mark the page" made the seal's *background* the painted part, so the page came out
///   blank where three renderers draw a court seal. The reading those three use is
///   §11.6.5.2's — luminosity as opacity, so white paints — which is a different clause
///   about a different key, and adopting it here would invert every stencil whose author
///   merely forgot `/ImageMask`. Refusing draws the image with its rectangle showing and
///   says so, which is wrong in a way a reader can see.
/// - **The combined grid has to fit.** See [`combined_grid`]. There is no second construction
///   here — a stencil is `/ImageMask true`, which Table 87 forbids a colour space, so
///   [`eligible_for_the_device_scale`] can never say yes to one — so the only question is
///   whether the raster can be built, and the answer for a full-page scan under a
///   600 dpi stencil is that it can.
///
/// [`eligible_for_the_device_scale`]: super::soft_mask::eligible_for_the_device_scale
fn explicit_entry(document: &Document, dict: &Dictionary, stream: &Arc<Stream>) -> MaskEntry {
    let mask_dict = &stream.dict;
    let dimension =
        |dict: &Dictionary, key| crate::integer_entry::dimension(document, dict, key).unwrap_or(0);
    let (mask_width, mask_height) = (
        dimension(mask_dict, "Width"),
        dimension(mask_dict, "Height"),
    );
    let (width, height) = (dimension(dict, "Width"), dimension(dict, "Height"));

    if !matches!(
        document.get_key(mask_dict, "ImageMask"),
        Object::Boolean(true)
    ) {
        return MaskEntry::Unusable(format!(
            "/Mask is a {mask_width}x{mask_height} image that is not an image mask"
        ));
    }

    // Table 87 forbids the line above's image mask a mask of its own, twice over — of
    // `/ImageMask`:
    //
    // > If this flag is true , the value of BitsPerComponent , if present, shall be 1 and Mask
    // > and ColorSpace shall not be specified
    //
    // and of `/Mask` itself, "shall not be present for image masks". So a stencil that states one
    // is a file no clause describes, and **following it is unbounded recursion**: `decode_parts` →
    // `apply_explicit_mask` → `decode` → `decode_parts`, which the `page` fuzz target overflows the
    // stack on (ADR 0399). A stack overflow is an abort rather than a report, and the address-space
    // ceiling cannot see it (`doc/todo/10` §2's `MAX_FORM_DEPTH` row, the same argument one clause
    // along).
    //
    // Refused rather than depth-bounded, because the standard's bound is *one* and stating it
    // needs no constant: a mask's mask does not exist. Table 143's `/SMask` twin is
    // `soft_mask_entry`'s. ADR 0399.
    for nested in ["Mask", "SMask"] {
        if document.get_key(mask_dict, nested).as_stream().is_some() {
            return MaskEntry::Unusable(format!(
                "/Mask is an image mask carrying a /{nested} of its own, which Table 87 says \
                 shall not be present"
            ));
        }
    }

    if let Err(grid) = combined_grid(width, height, mask_width, mask_height) {
        return MaskEntry::Unusable(format!(
            "/Mask is {mask_width}x{mask_height} against a {width}x{height} image, needing a \
             grid of {grid} samples"
        ));
    }
    MaskEntry::Explicit(Arc::clone(stream))
}

/// Names the image codec at the end of a `/Filter` chain, if there is one.
///
/// Read from the dictionary rather than from a decoded stream, because the callers that need
/// it are deciding whether to *report* something and must not pay for a decode to do it.
pub(super) fn image_codec(document: &Document, dict: &Dictionary) -> Option<String> {
    let filter = document.get_key(dict, "Filter");
    let last = match &filter {
        Object::Name(name) => Some(name.as_bytes().to_vec()),
        Object::Array(items) => items
            .last()
            .map(|item| document.resolve(item))
            .and_then(|item| item.as_name().map(|name| name.as_bytes().to_vec())),
        _ => None,
    }?;
    matches!(
        last.as_slice(),
        b"DCTDecode" | b"DCT" | b"JPXDecode" | b"JBIG2Decode" | b"CCITTFaxDecode" | b"CCF"
    )
    .then(|| String::from_utf8_lossy(&last).into_owned())
}

/// Names a `/Mask` this crate cannot apply, for the caller to report.
///
/// Asked of the dictionary alone, so that the interpreter's report and this module's
/// behaviour cannot drift apart: every reason returned here is a reason [`decode`] will not
/// have applied the mask, and there is no other. The one case not covered is a mask whose
/// *data* will not decode, which [`decode`] turns into an error naming it — an image that
/// cannot be masked at all is refused rather than painted through a mask nobody could read.
#[must_use]
pub fn unapplied_mask(
    document: &Document,
    stream: &Stream,
    resources: &Dictionary,
) -> Option<String> {
    match mask_entry(document, stream, resources) {
        MaskEntry::Unusable(reason) => Some(reason),
        MaskEntry::Absent
        | MaskEntry::Overridden
        | MaskEntry::ColourKey(_)
        | MaskEntry::Explicit(_) => None,
    }
}

/// Names a `DCTDecode` frame whose dimensions the dictionary contradicts, for the caller to
/// report *beside* the drawing.
///
/// [`decode_jpeg`] builds the raster on the grid the codestream states, and its own comment has
/// the reading: §7.4.8 puts a JPEG's dimensions in the encoded data and states no requirement
/// that the dictionary agree. That decides what to *draw*. What it does not decide is whether
/// to say anything, and the answer is yes, because the dictionary's numbers are a statement the
/// file made about this image and the page now shows something else: `xobject-image.pdf`
/// describes a 200×100 picture and its codestream holds one red sample, so the page is a flat
/// red rectangle that no reader could otherwise know was not the picture. Suppressing either
/// half — the drawing or the report — loses information, which is the test `doc/HANDOVER.md`
/// sets for reporting while drawing.
///
/// Asked of the grid the decode built on, `frame`, rather than of the dictionary alone, for the
/// same reason [`unapplied_mask`] is asked of the entry: a report that reads only what the file
/// says can outlive the gap it describes. It answers for the `DCTDecode` arm alone, and
/// [`decode_parts`] carries the answer in [`Parts::contradiction`]: §7.4.9's mismatch is a refusal inside
/// [`decode_jpx`], a reduced JPEG 2000 grid is §7.4.9 NOTE 3's rather than a contradiction, and
/// the other codecs carry no dimensions of their own.
///
/// [`decode_jpeg`]: super::dct::decode_jpeg
/// [`decode_jpx`]: super::jpx::decode_jpx
pub(super) fn contradicted_frame(
    source: &ImageStream,
    frame: (u32, u32),
    (width, height): (u32, u32),
) -> Option<String> {
    let dct = matches!(source.codec.as_deref(), Some(b"DCTDecode" | b"DCT"));
    (dct && frame != (width, height)).then(|| {
        format!(
            "the JPEG frame is {}x{} where the dictionary says {width}x{height} (§7.4.8 puts \
             the dimensions in the encoded data); its samples are drawn on their own grid, so \
             the image is the codestream's rather than the one the dictionary describes",
            frame.0, frame.1
        )
    })
}

/// [`decode`], with the sentence [`Parts::contradiction`] carries beside the one raster.
///
/// For the interpreter's stencil painted through a pattern, which needs one raster and reports
/// what the picture's decode reports; [`Flattened`]'s other callers write or thumbnail an image
/// rather than report on a page, and are handed [`decode`]'s pair.
///
/// # Errors
///
/// See [`ImageError`].
pub(crate) fn decode_reporting_frame(
    document: &Document,
    stream: &Stream,
    resources: &Dictionary,
    fill: pdf_render::Color,
    into: &Conversion,
) -> Result<(Flattened, Option<String>), ImageError> {
    let mut masks = MaskCache::default();
    let Parts {
        picture,
        shortfall,
        contradiction,
    } = decode_parts(document, stream, resources, fill, into, &mut masks)?;
    let (image, _) = flattened(picture);
    Ok((Flattened { image, shortfall }, contradiction))
}

/// [`decode_reporting_frame`] for a §8.9.6.2 stencil, with §11.6.4.2's shape beside the raster
/// where the two are not the same: a stencil under an `/SMask` of its own, whose flattened alpha
/// is the stencil's shape times §11.6.5.2's opacity. `None` beside every other raster, whose
/// alpha is the stencil's shape already.
///
/// The one caller is a stencil painted through a pattern, which becomes a mask on the fill and
/// so cannot travel as the [`Picture::Masked`] pair an image command carries; the pair is kept
/// as two masks instead (ADR 1301).
///
/// # Errors
///
/// See [`ImageError`].
pub(crate) fn decode_stencil_reporting_frame(
    document: &Document,
    stream: &Stream,
    resources: &Dictionary,
    into: &Conversion,
) -> Result<(Flattened, Option<Image>, Option<String>), ImageError> {
    let mut masks = MaskCache::default();
    let Parts {
        picture,
        shortfall,
        contradiction,
    } = decode_parts(
        document,
        stream,
        resources,
        pdf_render::Color::BLACK,
        into,
        &mut masks,
    )?;
    let (image, shape) = flattened(picture);
    Ok((Flattened { image, shortfall }, shape, contradiction))
}

/// A picture as one raster, and the base raster it was made from where a soft mask was
/// multiplied in to make it.
fn flattened(picture: Picture) -> (Image, Option<Image>) {
    match picture {
        Picture::Complete(image) => (image, None),
        Picture::Masked { base, opacity } => {
            let grid = pdf_render::Grid {
                width: base.width,
                height: base.height,
            };
            let mut shape = base.clone();
            shape.sample_alpha = SampleAlpha::Shape;
            (opacity.over(base).samples(grid), Some(shape))
        }
    }
}

/// Names an image whose decoded samples fall short of the grid its dictionary states, for the
/// caller to report *beside* the drawing.
///
/// ISO 32000-2 §7.3.8.2 makes the extent of such a stream inferable from the object itself:
///
/// > Finally, streams are used to represent many objects from whose attributes a length can be
/// > inferred. All of these constraints shall be consistent.
///
/// and its EXAMPLE is an image, with the arithmetic and the verdict spelled out — a 10-row,
/// 20-column, 8-bit, one-component image "requires exactly 200 bytes of image data", and "[a]n
/// error occurs if Length is too small, if an explicit EOD marker occurs too soon, or if the
/// decoded data does not contain 200 bytes."
///
/// [`unpack`] draws the samples the file did carry and leaves the rest of the grid unpainted;
/// this is the other half, and the pair is the test `doc/HANDOVER.md` sets for reporting while
/// drawing — suppressing either loses information. Without the report a page missing the bottom
/// nine tenths of a picture is indistinguishable from a page whose picture is that shape.
///
/// Asked only of the streams whose decoded bytes *are* samples. [`Document::image_stream`] names
/// the codec it stopped in front of, and where there is one the length belongs to `DCTDecode`,
/// `JPXDecode`, `JBIG2Decode` or `CCITTFaxDecode` rather than to this arithmetic: a codestream
/// states its own extent, which is why §7.4.8's disagreement has `contradicted_frame` of its
/// own.
///
/// [`unpack`]: super::unpack::unpack
#[must_use]
pub fn short_of_its_grid(
    document: &Document,
    stream: &Stream,
    resources: &Dictionary,
) -> Option<String> {
    let dict = &stream.dict;
    let width = positive_integer(document, dict, "Width").ok()? as usize;
    let height = positive_integer(document, dict, "Height").ok()? as usize;
    // The codec first, out of Table 5, because this arithmetic applies to no stream that has
    // one — and where there is one the decode below would be spent only to find that out.
    if document.image_codec(stream).is_some() {
        return None;
    }
    let source = document.image_stream(stream)?;

    // §8.9.6.2: a stencil mask is one bit per sample and carries no colour space, so its
    // arithmetic is fixed rather than read.
    let is_mask = matches!(document.get_key(dict, "ImageMask"), Object::Boolean(true));
    let (bits, components) = if is_mask {
        (1usize, 1usize)
    } else {
        let bits = usize::try_from(
            document
                .get_key(dict, "BitsPerComponent")
                .as_integer()
                .unwrap_or(8),
        )
        .ok()?;
        let space = document.get_key(dict, "ColorSpace");
        // Asked for its component count alone, which no output intent can change:
        // `ColourSpace::device_family` substitutes the intent only for a family with the
        // same number of components, so the answer here is the answer under any intent.
        let resolved = crate::colour::ColourSpace::parse(document, &space, resources)?;
        (bits, resolved.components())
    };

    // §8.9.5.1: "Each row of samples shall begin on a byte boundary", so the rounding up is per
    // row and the total is not simply the whole grid's bits.
    let row = width
        .checked_mul(components)?
        .checked_mul(bits)?
        .checked_add(7)?
        / 8;
    let need = row.checked_mul(height)?;
    let have = source.data.len();
    (have < need).then(|| {
        format!(
            "its samples stop at {have} bytes where {width}x{height} at {bits} bits and \
             {components} component(s) needs {need} (§7.3.8.2 infers the extent from the \
             dictionary); what the stream carries is drawn and the rest of the grid is left \
             unpainted"
        )
    })
}

/// Names a `CCITTFaxDecode` image whose filter Table 11 stops short of the grid §8.9.5.1 states,
/// for the caller to report *beside* the drawing.
///
/// ISO 32000-2 §7.4.6 Table 11's `/EndOfBlock` row is the whole condition:
///
/// > A flag indicating whether the filter shall expect the encoded data to be terminated by an
/// > end-of-block pattern, overriding the Rows parameter. If false , the filter shall stop when
/// > it has decoded the number of lines indicated by Rows or when its data has been exhausted,
/// > whichever occurs first.
///
/// So `/EndOfBlock false` with a `/Rows` below the image's `/Height` is not a malformed file: the
/// producer has told the filter to stop before the image is full, and the filter obeys.
/// [`ccitt_rows`] is that derivation. What the undelivered scan lines then show is stated
/// **nowhere** in ISO 32000-2 — neither Table 11 nor §8.9.5.1 has a sentence about it — so this
/// tree chooses blank, which is what an unsent fax scan line is, and `pdf_sandbox`'s
/// `PackedRows::pad_to_height` is where the choice is made.
///
/// That choice is why this is a report rather than a refusal, and the pair is the test
/// `doc/HANDOVER.md` sets for reporting while drawing: suppressing the drawing throws away the
/// scan lines the file does carry, and suppressing the report makes a page whose lower half this
/// program invented indistinguishable from one whose lower half the producer left white.
///
/// The condition is the clause's and nothing wider. An `/EndOfBlock` left at its default is
/// silent because the default is true and `/Rows` then does not bind at all; a `/Rows` at or
/// above `/Height` is silent because the filter reaches the whole grid; a `/Rows` of zero is
/// silent because Table 11 makes that "not predetermined" rather than short. A decode that
/// simply runs out of data early is also not this — it is the same padding for a different
/// reason, and the clause's "or when its data has been exhausted" makes it as legal as an
/// end-of-block pattern.
///
/// [`ccitt_rows`]: super::ccitt::ccitt_rows
#[must_use]
pub fn ccitt_bound_below_its_height(document: &Document, stream: &Stream) -> Option<String> {
    let height = positive_integer(document, &stream.dict, "Height").ok()?;
    // Table 5 rather than the decode: this report is about `CCITTFaxDecode` alone, and asking
    // `image_stream` first would run every filter in front of the codec for the sole purpose of
    // discovering that the codec is somebody else's. ADR 0585.
    if !matches!(
        document.image_codec(stream).as_deref(),
        Some(b"CCITTFaxDecode" | b"CCF")
    ) {
        return None;
    }
    let source = document.image_stream(stream)?;
    let parms = source.parms.as_ref()?;
    if !matches!(
        document.get_key(parms, "EndOfBlock"),
        Object::Boolean(false)
    ) {
        return None;
    }
    let rows = u32::try_from(document.get_key(parms, "Rows").as_integer()?).ok()?;
    (rows > 0 && rows < height).then(|| {
        format!(
            "its /EndOfBlock is false and /Rows {rows} stops the filter short of the {height} \
             scan lines /Height states (§7.4.6 Table 11); the {rows} the filter delivers are \
             drawn and the remaining {} are blank, which the standard does not state",
            height.saturating_sub(rows)
        )
    })
}

/// Whether this image carries a mask of its own, which supersedes the graphics state's.
///
/// ISO 32000-2 §11.6.4.3, having listed the three ways a soft mask reaches a compositing
/// operation:
///
/// > Either form of mask in the image dictionary shall override, for this image object
/// > only, the current soft mask in the graphics state.
///
/// So an image with an `/SMask`, an `/SMaskInData` or a `/Mask` is painted through that and
/// *not* through the mask a `gs` left in force — and the state's mask is untouched for
/// everything drawn after it. Asked of the keys rather than of the decoded image, because
/// the clause is about what the dictionary says: a `/Mask` this crate cannot apply still
/// overrides, and is reported by [`unapplied_mask`] rather than quietly replaced by a
/// different mask than the file named.
#[must_use]
pub fn overrides_graphics_state_mask(document: &Document, dict: &Dictionary) -> bool {
    !matches!(document.get_key(dict, "SMask"), Object::Null)
        || document
            .get_key(dict, "SMaskInData")
            .as_integer()
            .is_some_and(|value| value != 0)
        || !matches!(document.get_key(dict, "Mask"), Object::Null)
}

/// Applies §8.9.6.3's explicit mask: where the stencil does not mark, the image is not drawn.
///
/// The second half of the answer is the stencil's own [`Flattened::shortfall`], where its
/// filter stopped on damaged data: the places it did deliver mask as the clause says, and the
/// caller reports the rest beside the drawing.
pub(super) fn apply_explicit_mask(
    document: &Document,
    image: &Image,
    resources: &Dictionary,
    stream: &Stream,
) -> Result<(Image, Option<String>), ImageError> {
    let Flattened {
        image: stencil,
        shortfall,
    } = decode(
        document,
        stream,
        resources,
        pdf_render::Color::BLACK,
        // A stencil is read for where it marks, so it carries no colour to redirect.
        &Conversion::device(),
    )
    .map_err(|error| ImageError::Malformed {
        detail: format!("/Mask did not decode: {error}"),
    })?;
    // The stencil came back from `decode` as the fill colour where its samples mark the page
    // and as nothing where they do not, so alpha is where the answer already is —
    // §8.9.6.2's rule about which bit marks, and any `/Decode [1 0]` reversing it, were both
    // applied there. This is the whole reason the mask goes through the ordinary image route
    // rather than a private one: `issue4379.pdf`'s stencil is `CCITTFaxDecode`d.
    //
    // The sense is the clause's — the mask indicates which places on the page are painted and
    // which are masked out — so a sample that marks paints and one that does not is left
    // unchanged.
    let masked =
        combine_on_the_finer_grid(image, MaskSamples::of_raster(&stencil), |colour, sample| {
            let marks = sample.get(3).is_some_and(|alpha| *alpha != 0);
            (colour, if marks { u8::MAX } else { 0 })
        });
    Ok((masked, shortfall))
}
