//! An image's `/SMask` (ISO 32000-2 §11.6.5.2): the soft-mask image read as a grey plane,
//! its `/Matte` undone from the parent's samples, and the plane combined with the image.

use std::sync::Arc;

use pdf_render::Image;
use pdf_syntax::{Dictionary, Document, ImageStream, Object, Stream};

use crate::colour::Conversion;

use super::combine::{
    MaskSamples, alpha_on_grid, combine_on_the_finer_grid, opacity_multiplied_in_place,
};
use super::dct::{first_scan, frame_as_defined, jpeg_options, restart};
use super::device_scale::SoftMaskAtDeviceScale;
use super::dispatch::{EagerMask, positive_integer};
use super::mask::{PREFER_DEVICE_SCALE_ABOVE, combined_grid, image_codec, worth_combining};
use super::space::{ColourSpace, Decode, colour_space, mask_colour_space};
use super::unpack::{Samples, eight_bit_device_tables};
use super::{Flattened, MAX_SAMPLES, decode};

/// What an image's `/SMask` entry holds, once read.
///
/// The same shape as [`MaskEntry`] and for the same reason: what the entry means has to be
/// decided once, so that what the interpreter reports and what [`decode`] does cannot drift
/// apart. Unlike `/Mask` there is only one mechanism here — §11.6.5.2's soft-mask image —
/// and the variants are whether it can be used.
///
/// [`MaskEntry`]: super::mask::MaskEntry
pub(super) enum SoftMaskEntry {
    /// No `/SMask`, or one that is not a stream.
    Absent,
    /// §11.6.5.2, on a grid whose refinement with the image's is too large to build.
    ///
    /// The mask is carried to the backend beside the image instead, and the two are combined
    /// where the device scale is known — which is what §10.7.4 asks for in the first place.
    /// See [`eligible_for_the_device_scale`] for what makes a mask eligible.
    AtDeviceScale,
    /// §11.6.5.2: a `DeviceGray` image whose samples are this image's opacity.
    ///
    /// `matte` is Table 144's colour, in the raster's own components, when the image's samples
    /// are pre-blended with one and this crate can undo it. `owed` names a requirement of the
    /// clause that applying the mask does not satisfy, and there is exactly one: a `/Matte`
    /// this crate cannot undo, because inverting it "shall precede the colour conversion" and
    /// the conversion has happened by the time an image reaches here. It is reported *beside*
    /// the mask rather than instead of it, which is the second place in this tree where a
    /// report accompanies drawing (the first is `/NeedAppearances`), and it needs the same
    /// argument. Here it is: the mask itself is fully specified and applying it is right,
    /// while the pre-blending is a defect in the colours. Refusing the mask because of the
    /// matte would draw an opaque rectangle whose edges are *entirely* the matte colour —
    /// where α is 0, `c′ = m` — which is worse on the page and no more honest.
    Image {
        stream: Arc<Stream>,
        matte: Option<Vec<f32>>,
        owed: Option<String>,
    },
    /// Present, and not applied. Carries the words the interpreter reports.
    Unusable(String),
}

/// Reads `/SMask` against the image it masks, deciding whether §11.6.5.2's mask applies.
///
/// Table 143 restricts a soft-mask image dictionary, and three of its restrictions decide
/// whether this crate can use one at all:
///
/// - **`/Width` and `/Height` are "independent of" the parent image's** unless `/Matte` is
///   present, and "[b]oth images shall be mapped to the unit square in user space (as are all
///   images), regardless of whether the samples coincide individually". So a mask of another
///   size still applies; [`combine_on_the_finer_grid`] is where the choice that costs is
///   made, and [`PREFER_DEVICE_SCALE_ABOVE`] is where it stops being the cheaper choice —
///   `issue16263.pdf` gives a 2×2 image a 34862×4332 mask, 151 million samples for two
///   distinct colours.
/// - **`/ColorSpace` is "Required; shall be `DeviceGray`"**, and this is not pedantry: the
///   mask is decoded by the ordinary image route, so a mask in some other space arrives as a
///   colour, and there is no clause saying which of its components is the opacity. §11.5.3's
///   luminosity is a rule about a *transparency group*'s colour, not about this key.
/// - **`/ImageMask` "[s]hall be false or absent"**, which matters more than it looks: a
///   stencil decodes to the fill colour and an alpha, carrying no grey level at all, so
///   reading its first component as opacity would make every such image fully transparent.
///   Refusing draws the image opaque, which is wrong in a way a reader can see.
///
/// A `/Matte` is the fourth thing read here and the only one that does not decide whether the
/// mask applies — see [`matte_colour`], which decides whether the pre-blending it announces
/// can be undone in the raster this crate holds.
///
/// `base` is the grid of the parent image's raster as the *caller* knows it, because the
/// routing between the two grid answers is about the two rasters that exist and there are two
/// callers with two kinds of knowledge. [`MaskCache::read`] passes the grid the base actually
/// decoded to — which for a JPEG 2000 image over the worker's budget is a reduced resolution
/// level, not the dictionary's statement — so `issue19517.pdf`'s 3152×4202 base under its
/// 12608×16806 mask routes to §10.7.4's device-scale combination instead of an 848 MB eager
/// one. [`unapplied_soft_mask`] and [`apply_soft_mask`] pass the dictionary's own grid, the
/// only one knowable without decoding, and those two asking the *same* question is what keeps
/// the interpreter's report and the eager route's behaviour from drifting apart: the deferred
/// route that consults the decoded grid never leaves a mask unapplied — it exists to apply
/// one — so a report computed from the dictionary's grid cannot name a gap that route opened.
///
/// [`MaskCache::read`]: super::cache::MaskCache::read
pub(super) fn soft_mask_entry(
    document: &Document,
    dict: &Dictionary,
    resources: &Dictionary,
    base: (u32, u32),
) -> SoftMaskEntry {
    let smask = document.get_key(dict, "SMask");
    let Some(mask) = smask.as_stream() else {
        return SoftMaskEntry::Absent;
    };
    let dimension =
        |dict: &Dictionary, key| crate::integer_entry::dimension(document, dict, key).unwrap_or(0);
    let (mask_width, mask_height) = (
        dimension(&mask.dict, "Width"),
        dimension(&mask.dict, "Height"),
    );
    let (width, height) = base;

    if matches!(
        document.get_key(&mask.dict, "ImageMask"),
        Object::Boolean(true)
    ) {
        return SoftMaskEntry::Unusable(format!(
            "/SMask is a {mask_width}x{mask_height} image mask, which carries no opacity"
        ));
    }
    if document.get_key(&mask.dict, "SMask").as_stream().is_some() {
        // Table 143 again: an `/SMask` inside an `/SMask` "[s]hall be absent". Applied to the
        // mask it would change what the mask says, which is not a thing the clause defines.
        return SoftMaskEntry::Unusable("/SMask carries a soft mask of its own".to_owned());
    }
    if document.get_key(&mask.dict, "Mask").as_stream().is_some() {
        // Table 143 says the same of `/Mask` — "Shall be absent" — and this half was missing
        // while the `/SMask` half above was here. It is the second door into the recursion
        // `explicit_entry` closes: a soft mask carrying an explicit one sends `apply_soft_mask`
        // into `decode` → `decode_parts` → `apply_explicit_mask` → `decode`, with nothing
        // bounding the descent. ADR 0399.
        return SoftMaskEntry::Unusable(
            "/SMask carries a /Mask of its own, which Table 143 says shall be absent".to_owned(),
        );
    }
    // Table 143: "ColorSpace | Required; shall be DeviceGray." A mask stating another
    // one-component space is the **file's** failure, and what a reader does with it is
    // §11.6.5.2's own arithmetic rather than a second clause: the mask supplies one number per
    // sample and the clause reads that number as the image's opacity, so the number is what is
    // taken — the sample through §8.9.5.2's `/Decode`, whose default Table 143 fixes at
    // `[0 1]`. It is the reading ADR 1054 already settled one key along, on §8.6.5.6's own
    // sentence that a remapping "shall be passed unchanged" to the default space: a space says
    // which colour a value denotes and never what the value is, and what this clause uses is
    // the value. Refusing instead would draw the image opaque, which loses the transparency
    // the producer did state; so the mask applies and the departure from Table 143 is
    // reported beside it. ADR 1268.
    let stated_space = match mask_colour_space(document, &mask.dict) {
        Ok(ColourSpace::Gray) => None,
        Ok(space) if space.components() == 1 => {
            Some(match document.get_key(&mask.dict, "ColorSpace") {
                Object::Name(name) => format!("/{name}"),
                _ => "a one-component colour space".to_owned(),
            })
        }
        Ok(space) => {
            return SoftMaskEntry::Unusable(format!(
                "/SMask has a {}-component colour space where Table 143 requires DeviceGray",
                space.components()
            ));
        }
        Err(error) => {
            return SoftMaskEntry::Unusable(format!("/SMask colour space: {error}"));
        }
    };
    let departure = stated_space.map(|named| {
        format!(
            "/SMask states {named} where Table 143 requires DeviceGray; its samples are read \
             as the mask values they state"
        )
    });

    // §11.6.4.2 makes a stencil's painted areas this element's **shape** — "[f]or image masks
    // (8.9.6.2, "Stencil masking"), the shape shall be 1.0 for painted areas and 0.0 for masked
    // areas" — while §11.6.4.3 makes an `/SMask` its **opacity**: "a soft mask ... shall override
    // any explicit or colour key mask". §11.3.7.2 keeps the two as separate quantities, and
    // §11.4.6's knockout reads the first of them apart from the second. One alpha channel holding
    // their product cannot be asked for either again, so a stencil's pair is carried to the
    // device rather than multiplied here — which is the same routing a refinement too large to
    // build already takes, on a reason from the clause rather than from the grids. ADR 1218.
    let stencil = matches!(document.get_key(dict, "ImageMask"), Object::Boolean(true));
    if stencil
        && eligible_for_the_device_scale(document, &mask.dict)
        && matches!(
            matte_colour(document, dict, resources, &mask.dict),
            Matte::Absent
        )
    {
        return SoftMaskEntry::AtDeviceScale;
    }
    if !worth_combining(width, height, mask_width, mask_height) {
        // The refinement of the two grids is large enough that §10.7.4's answer — combine at
        // device resolution — is the better one. It needs the mask's samples readable at a
        // chosen grid; see [`device_scaled_soft_mask`] for the three things that decides.
        if eligible_for_the_device_scale(document, &mask.dict)
            && matches!(
                matte_colour(document, dict, resources, &mask.dict),
                Matte::Absent
            )
        {
            return SoftMaskEntry::AtDeviceScale;
        }
    }
    if let Err(grid) = combined_grid(width, height, mask_width, mask_height) {
        return SoftMaskEntry::Unusable(format!(
            "/SMask is {mask_width}x{mask_height} against a {width}x{height} image, needing a \
             grid of {grid} samples"
        ));
    }
    let (matte, unreadable) = match matte_colour(document, dict, resources, &mask.dict) {
        Matte::Absent => (None, None),
        Matte::Colour(colour) => (Some(colour), None),
        Matte::Unreadable(reason) => (None, Some(reason)),
    };
    let stream = if departure.is_some() {
        as_device_grey(mask)
    } else {
        Arc::clone(mask)
    };
    SoftMaskEntry::Image {
        stream,
        matte,
        // Two departures can stand at once and the report names one thing, so the `/Matte`'s
        // comes first: it is about the colours on the page, where the space's is about which
        // number the mask supplies and that number has been taken.
        owed: unreadable.or(departure),
    }
}

/// The same stream under Table 143's own `/ColorSpace`, for a mask that states another.
///
/// Carried as a substituted dictionary rather than as a flag, so that the one route which reads
/// the samples reads them as values by construction: every decode goes through the dictionary, and
/// a dictionary saying `DeviceGray` is what makes a sample the mask value. The bytes are the same
/// `Arc`, so the document's memo of the decoded stream is the same entry and nothing is inflated
/// twice. ADR 1268.
fn as_device_grey(mask: &Arc<Stream>) -> Arc<Stream> {
    let mut dict = mask.dict.clone();
    dict.insert(
        pdf_syntax::Name::new(*b"ColorSpace"),
        Object::Name(pdf_syntax::Name::new(*b"DeviceGray")),
    );
    Arc::new(Stream {
        dict,
        data: Arc::clone(&mask.data),
        decryption_failed: mask.decryption_failed,
    })
}

/// Table 144's `/Matte`, once read against the image whose samples it was blended into.
enum Matte {
    /// No `/Matte`: the image's samples are its colours.
    Absent,
    /// The matte colour, in the parent image's own components.
    Colour(Vec<f32>),
    /// Present, and not undone. Carries the words the interpreter reports.
    Unreadable(String),
}

/// Reads `/Matte` from a soft-mask image dictionary, in the parent image's own components.
///
/// Table 144 gives the entry `n` numbers in the *parent* image's colour space, and §11.6.5.2
/// says what they mean: its samples are `c′ = m + α × (c - m)`, a weighted average of the
/// colour and the matte, so a processor "may sometimes need to invert the formula" to get `c`
/// back. Two of the clause's sentences decide how much of that can be done here — the
/// computation belongs to the parent image's own colour space, and:
///
/// > If a colour conversion is required, inversion of the pre-blending shall precede the
/// > colour conversion.
///
/// This crate holds one RGBA raster per image, so the conversion has already happened by the
/// time a mask is applied — and the inversion is exact afterwards only where the conversion
/// was the identity on components, which is the two device spaces `DeviceGray` and
/// `DeviceRGB`. In any other space the raster's bytes are a *function* of the pre-blended
/// components rather than the components, and dividing them by α computes something the
/// clause does not describe. Those are reported instead, which is a gap this has never been
/// asked for: `issue13931.pdf` is the corpus's only `/Matte` and its parent image is
/// `DeviceRGB`.
fn matte_colour(
    document: &Document,
    dict: &Dictionary,
    resources: &Dictionary,
    mask_dict: &Dictionary,
) -> Matte {
    let matte = document.get_key(mask_dict, "Matte");
    let Object::Array(items) = &matte else {
        return Matte::Absent;
    };
    #[expect(
        clippy::cast_possible_truncation,
        reason = "a colour component, which `channel` clamps to 0.0..=1.0 in any case"
    )]
    let components: Vec<f32> = items
        .iter()
        .map(|item| document.resolve(item))
        .filter_map(|item| item.as_number().map(|value| value as f32))
        .collect();
    let space = match colour_space(document, dict, resources, &Conversion::device()) {
        Ok(space) => space,
        Err(error) => return Matte::Unreadable(format!("/SMask has a /Matte and {error}")),
    };
    // Table 144 counts the numbers in "the colour space specified by the ColorSpace entry (or
    // the base entry of the colour space, if the colour space is Indexed ) in the parent
    // image's image dictionary", which is the same sentence that decides what the inversion
    // runs on: an `Indexed` image's table entries, not its indices.
    let counted = match &space {
        ColourSpace::Resolved(resolved) => resolved.indexed_base().map_or_else(
            || space.components(),
            crate::colour::ColourSpace::components,
        ),
        _ => space.components(),
    };
    if components.len() != counted {
        return Matte::Unreadable(format!(
            "/SMask has a /Matte of {} components against a {counted}-component image",
            components.len()
        ));
    }
    // Table 143: with a `/Matte` the mask's `/Width` "shall be the same as the Width value of
    // the parent image", and `/Height` likewise. The inversion is per sample and pairs the two
    // grids by position, so a file that states two grids has stated no pairing at all.
    let dimension =
        |from: &Dictionary, key| crate::integer_entry::dimension(document, from, key).unwrap_or(0);
    if (
        dimension(mask_dict, "Width"),
        dimension(mask_dict, "Height"),
    ) != (dimension(dict, "Width"), dimension(dict, "Height"))
    {
        return Matte::Unreadable(
            "/SMask has a /Matte on a grid Table 143 requires to be the parent image's and is \
             not"
            .to_owned(),
        );
    }
    Matte::Colour(components)
}

/// The soft mask's samples as one byte apiece on the parent's stated grid, for undoing
/// §11.6.5.2's pre-blending.
///
/// The mask is `DeviceGray` by Table 143 — and by substitution where the file states another
/// one-component space — so its decoded raster holds one value in each of its three colour
/// channels and the first of them is it. Table 143 states the mask on the parent's grid wherever
/// a `/Matte` is present, and a mask whose own decode came back on another — a `JPXDecode` mask
/// over the worker's budget, at a reduced level — is carried onto it by [`alpha_on_grid`].
/// `None` where the mask does not decode, which is the same answer [`apply_soft_mask`] gives
/// such a mask: the image is drawn, with its samples left as the file wrote them.
pub(super) fn prematte_alpha(
    document: &Document,
    mask: &Stream,
    parent: (u32, u32),
) -> Option<Arc<[u8]>> {
    Some(mask_plane(document, mask)?.on_grid(parent))
}

/// A §11.6.5.2 soft mask's samples, decoded: one byte each, the opacity, on the mask's own grid.
///
/// **One byte a sample, because the mask is one component.** Table 143 makes the mask
/// `DeviceGray` (and [`soft_mask_entry`] substitutes that space for a file stating another), so
/// every consumer of a decoded mask — the `/Matte` inversion's `α`, the multiplication into the
/// image, the plane kept beside a stencil, the plane at device scale — reads one number a sample
/// and nothing else. Held as an RGBA raster it was four: on `issue13931.pdf`'s 2 996 × 4 256 mask
/// that was 51 MB written and read again for the 13 MB that carry anything (ADR 1481).
pub(super) struct MaskPlane {
    /// Samples per row.
    pub(super) width: u32,
    /// Rows.
    pub(super) height: u32,
    /// The opacities, row by row: §8.9.5.2's `/Decode` already applied.
    pub(super) opacity: Arc<[u8]>,
    /// See [`Parts::shortfall`]: the mask's own filter's sentence, where it stopped short.
    ///
    /// [`Parts::shortfall`]: super::Parts::shortfall
    pub(super) shortfall: Option<String>,
}

impl MaskPlane {
    /// The plane of a decoded mask raster, whose first channel is the opacity: what
    /// [`mask_plane`] answers for a mask its direct route does not admit.
    pub(super) fn of_decoded(Flattened { image, shortfall }: Flattened) -> Self {
        Self {
            width: image.width,
            height: image.height,
            opacity: image
                .data
                .chunks_exact(4)
                .map(|sample| sample.first().copied().unwrap_or(0))
                .collect(),
            shortfall,
        }
    }

    /// The opacities carried onto the parent's grid ([`alpha_on_grid`]): the same plane, not a
    /// copy of it, where the two grids are one — which Table 143 makes them under a `/Matte`.
    pub(super) fn on_grid(&self, parent: (u32, u32)) -> Arc<[u8]> {
        if (self.width, self.height) == parent {
            return Arc::clone(&self.opacity);
        }
        Arc::from(alpha_on_grid(
            &self.opacity,
            (self.width, self.height),
            parent,
        ))
    }

    /// The plane as [`combine_on_the_finer_grid`] reads a mask.
    pub(super) fn samples(&self) -> MaskSamples<'_> {
        MaskSamples {
            width: self.width,
            height: self.height,
            data: &self.opacity,
            stride: 1,
        }
    }
}

/// A soft-mask stream decoded as [`MaskPlane`], or `None` where it will not decode.
///
/// Every mask goes through [`decode`], the route every image takes, and its first channel is
/// taken — except the shapes that route would only widen and narrow again: eight-bit `DeviceGray`
/// samples with neither a `/Mask` nor an `/SMask` of their own ([`grey_mask`]), behind no codec
/// ([`grey_plane_of_samples`]) or behind `DCTDecode` in a frame of one component
/// ([`grey_plane_of_frame`]). Those are written as the plane directly, through the same `/Decode`
/// table the raster's route reads.
fn mask_plane(document: &Document, mask: &Stream) -> Option<MaskPlane> {
    if let Some(plane) =
        grey_plane_of_samples(document, mask).or_else(|| grey_plane_of_frame(document, mask))
    {
        return Some(plane);
    }
    decode(
        document,
        mask,
        // No resource dictionary and no colour to redirect, for [`mask_colour_space`]'s
        // reason: a mask's samples are mask values rather than colours.
        &Dictionary::new(),
        pdf_render::Color::BLACK,
        // §11.6.5.2's mask is read for its one channel of opacity, not for colour.
        &Conversion::device(),
    )
    .ok()
    .map(MaskPlane::of_decoded)
}

/// What [`mask_plane`]'s two direct routes share: a mask [`decode_parts`] would send through no
/// step but its codec (if any) and its `/Decode` table, its grid, its data with every filter before
/// the codec applied, and that table — or `None` for any other mask.
///
/// Admitted is what [`decode_parts`] would treat with nothing else to do: no `/ImageMask`, no
/// `/Mask` (which a colour key would read in the table's place) and no `/SMask`,
/// `/BitsPerComponent` read as [`samples_of`] reads it and found 8, and the space read as
/// [`mask_colour_space`] reads it and found `DeviceGray`. The dimensions are checked as
/// [`decode_parts`] checks them.
///
/// [`decode_parts`]: super::decode_parts
/// [`samples_of`]: super::dispatch::samples_of
fn grey_mask(document: &Document, mask: &Stream) -> Option<(u32, u32, ImageStream, [u8; 256])> {
    let dict = &mask.dict;
    let absent = |key: &str| matches!(document.get_key(dict, key), Object::Null);
    if !absent("Mask")
        || !absent("SMask")
        || matches!(document.get_key(dict, "ImageMask"), Object::Boolean(true))
    {
        return None;
    }
    let width = positive_integer(document, dict, "Width").ok()?;
    let height = positive_integer(document, dict, "Height").ok()?;
    if u64::from(width).saturating_mul(u64::from(height)) > MAX_SAMPLES {
        return None;
    }
    let bits = u32::try_from(
        document
            .get_key(dict, "BitsPerComponent")
            .as_integer()
            .unwrap_or(8),
    )
    .unwrap_or(8);
    if bits != 8 {
        return None;
    }
    let space = mask_colour_space(document, dict).ok()?;
    if !matches!(space, ColourSpace::Gray) {
        return None;
    }
    let source = document.image_stream(mask)?;
    let decode = Decode::read(document, dict, &space, bits);
    let conversion = Conversion::device();
    let samples = Samples {
        bits,
        space: &space,
        decode: &decode,
        colour_key: None,
        fill: pdf_render::Color::BLACK,
        into: &conversion,
        matte: None,
    };
    let tables = eight_bit_device_tables(&samples)?;
    let [table] = tables.as_slice() else {
        return None;
    };
    Some((width, height, source, **table))
}

/// [`mask_plane`]'s direct route for samples no codec stands in front of: the plane [`decode`]'s
/// first channel would be, written from the samples without the raster between, or `None` for any
/// mask outside [`grey_mask`]'s shape or behind a codec.
///
/// **The same bytes as [`decode`]'s, by the same steps**: what [`decode_parts`] sends through
/// [`unpack`]'s [`eight_bit_device_tables`] arm, each sample that arm's table entry, and a sample
/// past what its row carries 0, the first byte of the `[0, 0, 0, 0]` [`unpack`] leaves there for
/// §7.3.8.2's reason. The arm reports no shortfall, and nor does this.
/// `a_grey_mask_plane_is_its_decoded_first_channel` holds the two routes to one answer.
///
/// [`decode_parts`]: super::decode_parts
/// [`unpack`]: super::unpack::unpack
fn grey_plane_of_samples(document: &Document, mask: &Stream) -> Option<MaskPlane> {
    let (width, height, source, table) = grey_mask(document, mask)?;
    if source.codec.is_some() {
        return None;
    }
    // A row of one-byte samples is `width` bytes with no padding, so the rows are the data's
    // first `width × height` bytes in order, and what the data does not carry is the tail.
    let count = (width as usize).saturating_mul(height as usize);
    let carried = source
        .data
        .get(..source.data.len().min(count))
        .unwrap_or_default();
    let lookup = |&sample: &u8| table[usize::from(sample)];
    // Where the data covers the grid, which is every mask that is not damaged, the plane is
    // allocated once and already shared; a slice's mapped iterator states its length, which is
    // what lets the collection skip the copy into the shared allocation.
    let opacity: Arc<[u8]> = if carried.len() == count {
        carried.iter().map(lookup).collect()
    } else {
        let mut plane = vec![0u8; count];
        for (out, sample) in plane.iter_mut().zip(carried) {
            *out = lookup(sample);
        }
        Arc::from(plane)
    };
    Some(MaskPlane {
        width,
        height,
        opacity,
        shortfall: None,
    })
}

/// [`mask_plane`]'s direct route for a `DCTDecode` mask of one component: the codec asked for its
/// one channel rather than for the four-byte raster [`decode`] would build and the first channel
/// of which is all a mask is read for — or `None` for any mask outside [`grey_mask`]'s shape, behind
/// another codec, or whose frame is not one `Luma` component.
///
/// **The same bytes as [`decode`]'s.** That route asks `zune-jpeg` for the frame's luminance
/// written to the three colour channels of each pixel, and [`apply_decode_to_channels`] maps each
/// by the first `/Decode` pair, which is [`grey_mask`]'s table; this asks for the luminance once and
/// maps it by the same table. The grid is the frame's, as [`decode`]'s raster is (§7.4.8), and a
/// `DNL` is honoured and a frame cut at its restart intervals as [`decode_jpeg`] does both.
/// `a_grey_mask_plane_is_its_decoded_first_channel` holds the two routes to one answer. On
/// `22060_A1_01_Plans.pdf` it leaves four 2 480 × 2 630 masks a byte a sample where each was
/// 26 MB of raster (ADR 1481).
///
/// [`apply_decode_to_channels`]: super::convert::apply_decode_to_channels
/// [`decode_jpeg`]: super::dct::decode_jpeg
fn grey_plane_of_frame(document: &Document, mask: &Stream) -> Option<MaskPlane> {
    use zune_jpeg::zune_core::colorspace::ColorSpace;

    let (_, _, source, table) = grey_mask(document, mask)?;
    if !matches!(source.codec.as_deref(), Some(b"DCTDecode" | b"DCT")) {
        return None;
    }
    let walked = first_scan(&source.data);
    let data = frame_as_defined(&source.data, walked.as_ref());
    let scan = match &data {
        std::borrow::Cow::Borrowed(_) => walked,
        std::borrow::Cow::Owned(defined) => first_scan(defined),
    };
    let options = jpeg_options().jpeg_set_out_colorspace(ColorSpace::Luma);
    let mut decoder = zune_jpeg::JpegDecoder::new_with_options(
        zune_jpeg::zune_core::bytestream::ZCursor::new(&*data),
        options,
    );
    decoder.decode_headers().ok()?;
    let info = decoder.info()?;
    if info.components != 1 || decoder.input_colorspace() != Some(ColorSpace::Luma) {
        return None;
    }
    let (width, height) = (u32::from(info.width), u32::from(info.height));
    let count = usize::from(info.width).saturating_mul(usize::from(info.height));
    if count == 0 || u64::try_from(count).ok()? > MAX_SAMPLES {
        return None;
    }
    let mut luma = scan
        .as_ref()
        .and_then(|scan| restart::decode(&data, scan, options, 1))
        .map_or_else(|| decoder.decode().ok(), Some)?;
    if luma.len() != count {
        return None;
    }
    for sample in &mut luma {
        *sample = table[usize::from(*sample)];
    }
    Some(MaskPlane {
        width,
        height,
        opacity: Arc::from(luma),
        shortfall: None,
    })
}

/// Names what §11.6.5.2 asks of an `/SMask` and this crate does not do, for the caller to
/// report.
///
/// Asked of the dictionary alone, so that the report and the behaviour cannot drift apart —
/// the same contract as [`unapplied_mask`], and the same one case it does not cover: a mask
/// whose data will not decode leaves the image opaque, because an image visibly present and
/// slightly wrong beats one dropped entirely. Unlike that function it also reports where the
/// mask *is* applied, for the one requirement that is not about the mask — see
/// [`SoftMaskEntry::Image`].
///
/// Reading it costs no decode, which is what makes it safe to ask of every image: the
/// 34862×4332 mask above cost 19 seconds and 600 MB when this question was answered by
/// decoding first and comparing afterwards.
///
/// [`unapplied_mask`]: super::mask::unapplied_mask
#[must_use]
pub fn unapplied_soft_mask(
    document: &Document,
    dict: &Dictionary,
    resources: &Dictionary,
) -> Option<String> {
    // The dictionary's grid, deliberately: it is the only one knowable without a decode,
    // and the eager route asks with the same one, so the report and that route cannot
    // disagree. See `soft_mask_entry` for why the route that asks with the decoded grid
    // cannot open a gap for this to miss.
    match soft_mask_entry(document, dict, resources, stated_grid(document, dict)) {
        SoftMaskEntry::Unusable(reason)
        | SoftMaskEntry::Image {
            owed: Some(reason), ..
        } => Some(reason),
        // Nothing is owed for a mask carried to the backend: §11.6.5.2 is met there, at the
        // resolution §10.7.4 states, and a report would name a gap that has been closed.
        SoftMaskEntry::Absent
        | SoftMaskEntry::AtDeviceScale
        | SoftMaskEntry::Image { owed: None, .. } => None,
    }
}

/// The grid an image dictionary states for itself, for the callers that cannot know better.
///
/// [`decode_parts`] knows the grid its raster actually decoded to and passes that instead;
/// this is for the two callers that must answer without decoding — the interpreter's report,
/// and the eager route, which agree with each other by asking with the same pair.
///
/// [`decode_parts`]: super::decode_parts
pub(super) fn stated_grid(document: &Document, dict: &Dictionary) -> (u32, u32) {
    let dimension = |key| crate::integer_entry::dimension(document, dict, key).unwrap_or(0);
    (dimension("Width"), dimension("Height"))
}

/// Whether a soft mask's samples can be read at a grid chosen by the device.
///
/// Three things decide it, and each is the clause's own restriction rather than a convenience:
///
/// - **A stream carrying an image codec states a grid [`PREFER_DEVICE_SCALE_ABOVE`] admits.**
///   [`unpack`]'s samples are packed rows in the file's own bytes, so any grid can be read out
///   of them by indexing and the file's own stream is the whole cost; a `DCTDecode` or
///   `JPXDecode` codestream has a sample at no position until it is decoded, so the route's
///   cost for one is the decoded plane instead. [`MaskCache`] is keyed by the mask's own
///   `ObjectId`, so that plane is decoded once per document rather than once per raster
///   request, and what the route then owes is a bound on it — which is this same constant, the
///   one that already decides that a pair this large is better read at the device's grid and
///   that already bounds the grey plane [`SoftMaskAtDeviceScale::cells`] produces. A grid past
///   it takes the eager combination it took before, up to [`combined_grid`]'s ceiling. One byte
///   per sample of plane and four of transient RGBA, so 2^24 samples is 16 MiB kept against
///   `issue16263.pdf`'s 604 MB, which is the number this route exists for (ADR 1232).
///   `22060_A1_01_Plans.pdf`'s 6 522 400-sample mask is the largest of the 16 that stand behind
///   a codec over the 974 tracked documents, and it is inside the bound.
/// - **The colour space is `DeviceGray`.** Table 143 requires it — "Required; shall be
///   `DeviceGray`" — and it is what makes a sample's opacity a lookup rather than a colour
///   conversion. `soft_mask_entry` tolerates any one-component space for the ordinary route,
///   which decodes through the whole colour module; this route reads a byte.
/// - **The depth is one Table 87 names.** The same five [`unpack`] admits, for the same
///   reason: a depth the standard does not name says nothing about how the bytes are packed.
///
/// [`unpack`]: super::unpack::unpack
/// [`MaskCache`]: super::cache::MaskCache
fn eligible_for_the_device_scale(document: &Document, mask_dict: &Dictionary) -> bool {
    if image_codec(document, mask_dict).is_some()
        && !decodes_within_the_plane_bound(document, mask_dict)
    {
        return false;
    }
    if !matches!(
        mask_colour_space(document, mask_dict),
        Ok(ColourSpace::Gray)
    ) {
        return false;
    }
    matches!(
        document
            .get_key(mask_dict, "BitsPerComponent")
            .as_integer()
            .unwrap_or(8),
        1 | 2 | 4 | 8 | 16
    )
}

/// Whether a codec-carrying mask's grid is one this route will decode a grey plane for.
///
/// The plane is one byte per sample and [`MaskCache`] holds it for the document's life, so the
/// bound is on the grid the file states rather than on anything a device asks for — a decode is
/// a cost paid once and a raster request is not.
///
/// [`MaskCache`]: super::cache::MaskCache
fn decodes_within_the_plane_bound(document: &Document, mask_dict: &Dictionary) -> bool {
    let dimension =
        |key| crate::integer_entry::dimension(document, mask_dict, key).filter(|value| *value > 0);
    let (Some(width), Some(height)) = (dimension("Width"), dimension("Height")) else {
        return false;
    };
    u64::from(width).saturating_mul(u64::from(height)) <= PREFER_DEVICE_SCALE_ABOVE
}

/// Reads a soft mask the caller has already routed to the device-scale combination.
///
/// [`MaskCache::read`] establishes [`SoftMaskEntry::AtDeviceScale`] before calling this, so
/// what is left here is the read itself. `None` where the stream will not decode — the one
/// case [`unapplied_soft_mask`] does not cover and this does not either: an image visibly
/// present and opaque beats one dropped entirely, which is the same choice
/// [`apply_soft_mask`] makes for the same reason.
///
/// # Two shapes of sample, and one shape of answer
///
/// A stream whose filters all leave samples behind is kept as the file wrote it: packed rows,
/// the file's own depth, and §8.9.5.2's map alongside. A stream carrying an image codec has no
/// sample at any position until it is decoded, so it is decoded once here into an eight-bit
/// grey plane and kept as that — the map already applied, which is why the map stored beside it
/// is Table 88's identity for eight-bit `DeviceGray` rather than the dictionary's. Both shapes
/// answer [`SoftMaskAtDeviceScale::raster`] the same way, and [`MaskCache`]'s key makes the
/// decode a cost per document rather than per raster request (ADR 1232).
///
/// [`MaskCache::read`]: super::cache::MaskCache::read
/// [`MaskCache`]: super::cache::MaskCache
pub(super) fn device_scaled_soft_mask(
    document: &Document,
    dict: &Dictionary,
) -> Option<SoftMaskAtDeviceScale> {
    let smask = document.get_key(dict, "SMask");
    let mask = smask.as_stream()?;
    let mask_dict = &mask.dict;
    let dimension =
        |key| crate::integer_entry::dimension(document, mask_dict, key).filter(|value| *value > 0);
    let (width, height) = (dimension("Width")?, dimension("Height")?);
    if image_codec(document, mask_dict).is_some() {
        return decoded_grey_plane(document, mask);
    }
    let bits = u32::try_from(
        document
            .get_key(mask_dict, "BitsPerComponent")
            .as_integer()
            .unwrap_or(8),
    )
    .ok()?;
    // `eligible_for_the_device_scale` established that the space is `DeviceGray`, which is the
    // one this reads a byte out of.
    let space = ColourSpace::Gray;
    let decode = Decode::read(document, mask_dict, &space, bits);
    let source = document.image_stream(mask)?;
    Some(SoftMaskAtDeviceScale {
        data: source.data,
        width,
        height,
        bits,
        decode: Arc::new(decode),
    })
}

/// A codec-carrying soft mask decoded once, as one byte of opacity per sample.
///
/// The decode is [`apply_soft_mask`]'s, made on the same two terms and for the same reasons: no
/// resource dictionary, so that §8.6.5.6's defaults cannot remap a mask value, and
/// [`Conversion::device`], because "§11.6.5.2's mask is read for its one channel of opacity, not
/// for colour". Table 143 required `DeviceGray` and [`eligible_for_the_device_scale`] checked
/// it, so the three colour channels of a decoded sample hold one value and the first of them is
/// it.
///
/// The grid comes back from the decode rather than from the dictionary, because a `JPXDecode`
/// codestream may have been decoded at one of its own reduced levels (§7.4.9 NOTE 3) and the
/// plane this keeps is the one that exists.
fn decoded_grey_plane(document: &Document, mask: &Stream) -> Option<SoftMaskAtDeviceScale> {
    let MaskPlane {
        width,
        height,
        opacity,
        ..
    } = mask_plane(document, mask)?;
    Some(SoftMaskAtDeviceScale::grey_plane(width, height, opacity))
}

/// What [`apply_soft_mask`] delivers: the image, what the mask's filter said where it stopped
/// short, and whether a mask was multiplied in at all — which is what decides
/// [`SampleAlpha`], and which the image cannot say for itself.
///
/// [`SampleAlpha`]: pdf_render::SampleAlpha
pub(super) struct Softened {
    pub(super) image: Image,
    pub(super) shortfall: Option<String>,
    pub(super) applied: bool,
    /// A stencil's mask, kept apart from it rather than multiplied in; see [`apply_soft_mask`].
    pub(super) apart: Option<SoftMaskAtDeviceScale>,
}

/// Applies §11.6.5.2's soft mask: each of its samples is the image's opacity there.
///
/// A soft mask that cannot be read leaves the image opaque rather than failing it: an
/// opaque image is visibly present and slightly wrong, whereas dropping it loses content
/// entirely. One whose filter stopped on damaged data is applied as far as it was delivered,
/// and its [`Flattened::shortfall`] comes back beside the image for the caller to report.
///
/// # A stencil's mask is kept apart instead
///
/// A §8.9.6.2 stencil (`stencil`) under an `/SMask` the device-scale route declined — one behind a
/// codec past the plane bound, one stating a one-component space other than Table 143's, one
/// carrying a `/Matte` — is still a pair, and it is kept one: the mask is decoded here as the
/// multiplication would have decoded it and handed back as [`Softened::apart`], a plane beside the
/// stencil. §11.6.4.2 makes the stencil's painted areas its shape and §11.6.4.3 the mask its
/// opacity, and a raster holding their product answers for neither (ADR 1218). Table 144's
/// `/Matte` is no reason to combine, for the reason the stencil branch below gives. ADR 1279.
///
/// `made` is [`eager_soft_mask`]'s answer where the caller already has it — decoded beside the
/// image's samples, or once for a `/Matte` (ADR 1469) — and it is asked here otherwise. Where the
/// `/Matte`'s inversion already wrote it as the alpha ([`EagerMask::InTheSamples`]) it is applied,
/// and nothing is multiplied a second time (ADR 1519).
pub(super) fn apply_soft_mask(
    document: &Document,
    dict: &Dictionary,
    resources: &Dictionary,
    image: Image,
    (stencil, made): (bool, EagerMask),
) -> Softened {
    let made = match made {
        EagerMask::Asked(made) => made,
        EagerMask::InTheSamples(mask) => {
            return Softened {
                image,
                shortfall: mask.shortfall,
                applied: true,
                apart: None,
            };
        }
        EagerMask::Unasked => eager_soft_mask(document, dict, resources),
    };
    let Some(mask) = made else {
        return Softened {
            image,
            shortfall: None,
            applied: false,
            apart: None,
        };
    };
    if stencil {
        // Table 144 counts a `/Matte`'s numbers in "the colour space specified by the ColorSpace
        // entry (or the base entry of the colour space, if the colour space is Indexed ) in the
        // parent image's image dictionary", which Table 87 does not permit a stencil to have: a
        // stencil's samples carry no pre-blending to undo, and [`unpack`]'s stencil arm reads
        // none. So nothing here needs the pair to be one raster.
        return Softened {
            image,
            shortfall: mask.shortfall,
            applied: true,
            apart: Some(SoftMaskAtDeviceScale::grey_plane(
                mask.width,
                mask.height,
                mask.opacity,
            )),
        };
    }
    let mut image = image;
    if opacity_multiplied_in_place(&mut image, mask.samples()) {
        return Softened {
            image,
            shortfall: mask.shortfall,
            applied: true,
            apart: None,
        };
    }
    let masked = combine_on_the_finer_grid(&image, mask.samples(), |colour, sample| {
        // The stream `soft_mask_entry` handed on states `DeviceGray` — Table 143's own
        // requirement, and the space it substitutes for a file that states another
        // one-component space and is reported for it — so the three colour channels of a mask
        // sample hold one value and the first of them is it.
        let opacity = sample.first().copied().unwrap_or(0);
        // Nothing but the opacity is taken here: §11.6.5.2's `/Matte` was undone in the
        // parent's own components, before the colour conversion the clause puts it before,
        // which is where [`Prematte`] travels to.
        (colour, opacity)
    });
    Softened {
        image: masked,
        shortfall: mask.shortfall,
        applied: true,
        apart: None,
    }
}

/// The soft mask the eager route applies, decoded on its own grid, or `None` where there is none
/// to apply or it will not decode.
///
/// [`apply_soft_mask`] multiplies it into an ordinary image and keeps it apart as a plane beside
/// a §8.9.6.2 stencil (ADR 1279). A mask whose data will not decode leaves the image opaque
/// on both, for [`apply_soft_mask`]'s reason.
pub(super) fn eager_soft_mask(
    document: &Document,
    dict: &Dictionary,
    resources: &Dictionary,
) -> Option<MaskPlane> {
    // The dictionary's grid rather than the raster's, deliberately: this route and
    // `unapplied_soft_mask` must answer the same question, or the interpreter's report and
    // what actually happened drift apart. For the one image whose raster is coarser than its
    // dictionary — a reduced JPEG 2000 decode whose deferred route declined the mask — the
    // eager combination this admits is bounded by the same `MAX_SAMPLES` the dictionary's
    // grid already passed, and the mask's own decode reduces the same way the base's did.
    let SoftMaskEntry::Image {
        stream: mask_stream,
        ..
    } = soft_mask_entry(document, dict, resources, stated_grid(document, dict))
    else {
        return None;
    };
    // Read with no resource dictionary, which [`mask_colour_space`] is the argument for: the
    // mask's samples are mask values and §8.6.5.6's defaults remap colours. The two readings have
    // to agree, because `soft_mask_entry` decided this mask was usable from the first.
    mask_plane(document, &mask_stream)
}

#[cfg(test)]
mod tests {
    /// **A soft mask's plane written directly is the plane [`crate::image::decode`] would have
    /// given.**
    /// Each admitted shape is decoded both ways and held to the same bytes: raw samples under the
    /// default `/Decode`, an inverted one and a narrowed one, short of the grid, past it and with
    /// no `/BitsPerComponent`; and a `DCTDecode` frame of one component (`tests/cut/grey.jpg`)
    /// plain and inverted. A four-bit mask, one carrying a colour key, and a three-component frame
    /// a dictionary calls `DeviceGray` are left to the decode route (ADR 1481).
    #[test]
    fn a_grey_mask_plane_is_its_decoded_first_channel() {
        let samples: Vec<u8> = (0..48_u8).map(|index| index.wrapping_mul(37)).collect();
        let grey: &[u8] = include_bytes!("../../tests/cut/grey.jpg");
        let colour: &[u8] = include_bytes!("../../tests/cut/hv.jpg");
        let raw = "/Width 8 /Height 5 /BitsPerComponent 8";
        let frame = "/Width 48 /Height 200 /BitsPerComponent 8 /Filter /DCTDecode";
        let cases: [(String, &[u8], Option<&str>); 10] = [
            (raw.to_owned(), &samples, Some("samples")),
            (format!("{raw} /Decode [1 0]"), &samples, Some("samples")),
            (
                format!("{raw} /Decode [0.25 0.75]"),
                &samples,
                Some("samples"),
            ),
            (raw.to_owned(), &samples[..29], Some("samples")),
            ("/Width 8 /Height 5".to_owned(), &samples, Some("samples")),
            (
                "/Width 8 /Height 5 /BitsPerComponent 4".to_owned(),
                &samples[..24],
                None,
            ),
            (format!("{raw} /Mask [0 100]"), &samples, None),
            (frame.to_owned(), grey, Some("frame")),
            (format!("{frame} /Decode [1 0]"), grey, Some("frame")),
            (frame.to_owned(), colour, None),
        ];
        for (entries, data, route) in cases {
            let mut file = b"%PDF-1.7\n".to_vec();
            let mut offsets = Vec::new();
            offsets.push(file.len());
            file.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
            offsets.push(file.len());
            file.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [] /Count 0 >>\nendobj\n");
            offsets.push(file.len());
            file.extend_from_slice(
                format!(
                    "3 0 obj\n<< /Type /XObject /Subtype /Image /ColorSpace /DeviceGray {entries} \
                     /Length {} >>\nstream\n",
                    data.len()
                )
                .as_bytes(),
            );
            file.extend_from_slice(data);
            file.extend_from_slice(b"\nendstream\nendobj\n");
            let at = file.len();
            file.extend_from_slice(b"xref\n0 4\n0000000000 65535 f \n");
            for offset in &offsets {
                file.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
            }
            file.extend_from_slice(
                format!("trailer\n<< /Size 4 /Root 1 0 R >>\nstartxref\n{at}\n%%EOF\n").as_bytes(),
            );
            let document = pdf_syntax::Document::open(file).expect("the fixture opens");
            let object = document.get(pdf_syntax::ObjectId::new(3, 0));
            let stream = object.as_stream().expect("object 3 is the mask");
            let decoded = super::MaskPlane::of_decoded(
                crate::image::decode(
                    &document,
                    stream,
                    &pdf_syntax::Dictionary::new(),
                    pdf_render::Color::BLACK,
                    &crate::colour::Conversion::device(),
                )
                .expect("the mask decodes"),
            );
            let direct = match route {
                Some("samples") => super::grey_plane_of_samples(&document, stream),
                Some(_) => super::grey_plane_of_frame(&document, stream),
                None => super::grey_plane_of_samples(&document, stream)
                    .or_else(|| super::grey_plane_of_frame(&document, stream)),
            };
            assert_eq!(direct.is_some(), route.is_some(), "{entries}");
            if let Some(direct) = direct {
                assert_eq!(
                    (direct.width, direct.height),
                    (decoded.width, decoded.height)
                );
                assert_eq!(direct.opacity, decoded.opacity, "{entries}");
                assert_eq!(direct.shortfall, decoded.shortfall, "{entries}");
            }
            let either = super::mask_plane(&document, stream).expect("the mask decodes");
            assert_eq!(either.opacity, decoded.opacity, "{entries}");
        }
    }
}
