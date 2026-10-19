//! Destroying an image `XObject`'s samples in the region, ISO 32000-2 §8.9.5, §8.9.6, §11.6.5.2
//! and §12.5.6.23.
//!
//! One planner per way an image's data is held: packed samples, a codec's output taken as a
//! filter, a `JPXDecode` codestream, and the `/SMask` and `/Mask` streams that hold the
//! picture's shape. Each plan is an [`super::ImageClear`] the page's writer then carries out;
//! this module decides what is cleared, whether the image is shared and must be copied, and
//! what is refused.

use pdf_model::colour::ColourSpace;
use pdf_model::image::ImageError;
use pdf_syntax::object::{Dictionary, Object, ObjectId, Stream};
use std::sync::Arc;

use super::{
    Decoded, ImageClear, ImageLayout, OwnRaster, Role, Source, StatedMask, StatedMasks, Walk,
    decode_pairs, matte_samples, narrow, positive_dim, row_stride, widened_decode,
};

impl Walk<'_> {
    /// Plans clearing an image `XObject` that meets the region (§12.5.6.23), or refuses by name.
    ///
    /// The clearing itself is deferred to [`cleared_image_samples`], because a page may draw one
    /// image twice and the destroyed samples are the union across its placements. What this does
    /// is prove the image *can* be cleared without trace — its layout is known, and if it is
    /// behind a codec that codec is one this build runs as a filter and whose output a
    /// `FlateDecode` stream carries losslessly — so the refusal is decided here, at the page. The
    /// answer is the picture's clear and one for each mask it names ([`Walk::stated_masks`]),
    /// because a mask is image data of its own.
    pub(super) fn plan_image_clear(
        &self,
        name: &[u8],
        entry: &Object,
        object: &Object,
    ) -> Result<Vec<ImageClear>, String> {
        let shown = String::from_utf8_lossy(name);
        let Some(image_id) = entry.as_reference() else {
            return Err(format!(
                "the image /{shown} is a direct object rather than an indirect reference this \
                 removal can replace; the page is refused"
            ));
        };
        let Some(stream) = object.as_stream() else {
            return Err(format!(
                "the image /{shown} is not a stream; the page is refused"
            ));
        };
        let private = !self.owns(image_id);
        let image = self.document.image_stream(stream).ok_or_else(|| {
            format!("the image /{shown} did not decode to samples; the page is refused")
        })?;
        let codec = image.codec.clone();
        // §7.4.9 and §11.6.4.3: a non-zero `/SMaskInData` means the opacity came inside the
        // codestream, "the SMask entry shall not be present", and the embedded mask "shall
        // override any explicit or colour key mask specified by the image dictionary's Mask
        // entry". Such a picture's written dictionary names the opacity channel as its soft mask
        // and nothing else, so an entry it overrides is reached from nowhere and left behind.
        let opacity_in_data = codec.as_deref() == Some(&b"JPXDecode"[..])
            && self
                .document
                .get_key(&stream.dict, "SMaskInData")
                .as_integer()
                .is_some_and(|code| code != 0);
        let masks = if opacity_in_data {
            StatedMasks::default()
        } else {
            self.stated_masks(&shown, stream)?
        };
        let mut picture = if let Some(codec) = &codec {
            self.plan_codec_clear(
                &shown,
                image_id,
                stream,
                (codec, &image.data),
                Role::Picture,
            )?
        } else {
            self.plan_packed_clear(&shown, image_id, stream, &image.data, Role::Picture)?
        };
        picture.private = private;
        if let Some(matte) = &masks.matte {
            picture.fill = self.matte_fill(&shown, stream, &picture, matte)?;
        }
        let mut clears = vec![picture];
        for mask in masks.stated {
            clears.push(self.plan_mask_clear(&shown, &mask, private)?);
        }
        Ok(clears)
    }

    /// The sample a picture pre-blended with §11.6.5.2's matte colour takes where it is cleared,
    /// or the refusal by name where the matte does not describe the picture's components.
    ///
    /// Table 144's pre-blending is `c′ = m + α × (c − m)`, and a cleared soft mask states α = 0
    /// across the region, where the formula gives `c′ = m` whatever `c` was. So the picture's
    /// cleared samples are the matte, and the destroyed region is data the formula could have
    /// produced rather than a zero it could not: nothing of `c` survives in either image, and
    /// the two still describe one pre-blended picture (ADR 1333). The clause adds that the
    /// computation "shall use actual colour component values, with the effects of the Filter
    /// and Decode transformations already performed", so the matte is carried into samples by
    /// §8.9.5.2's map run backwards, component by component at each one's own depth.
    ///
    /// Every picture is written in the domain its dictionary states — a codec's output is its
    /// samples (Table 87), and a `JPXDecode` image's own integers are carried — so the matte,
    /// whose components are "valid colour components in that colour space" of the parent image
    /// (Table 144), is carried as the file stated it in every colour space (ADR 1371). An
    /// `Indexed` picture's matte is a colour of the base space, which no index is guaranteed to
    /// name, so its cleared indices stay the domain's zero: the region's opacity is zero either
    /// way, and §11.6.5.2 lets a reader choose any `c` there.
    fn matte_fill(
        &self,
        shown: &str,
        stream: &Stream,
        picture: &ImageClear,
        matte: &[f32],
    ) -> Result<Option<Vec<u32>>, String> {
        let refused = |why: &str| {
            format!(
                "§11.6.5.2: the image /{shown} is pre-blended with its soft mask's /Matte, and \
                 {why}; the page is refused"
            )
        };
        let space_entry = self.document.get_key(&stream.dict, "ColorSpace");
        let space = ColourSpace::parse(self.document, &space_entry, &self.resources);
        if matches!(space, Some(ColourSpace::Indexed { .. })) {
            return Ok(None);
        }
        let (pairs, depths) = match &picture.source {
            Source::Own(decoded) => (decoded.own.pairs.clone(), decoded.own.depths.clone()),
            Source::Packed | Source::Filtered(_) => {
                let space = space.ok_or_else(|| refused("its colour space cannot be read"))?;
                let bits = picture.layout.bits;
                let pairs = decode_pairs(self.document, &stream.dict, &space, bits);
                let depth = u8::try_from(bits).unwrap_or(u8::MAX);
                let depths = vec![depth; pairs.len()];
                (pairs, depths)
            }
        };
        matte_samples(matte, &pairs, &depths)
            .map(Some)
            .ok_or_else(|| refused("the matte does not name one value per component"))
    }

    /// The `/SMask` and `/Mask` streams an image names, each to be cleared on its own grid, or a
    /// refusal naming a mask the removal cannot clear.
    ///
    /// §12.5.6.23 forbids hiding what is removed — "clipping or image masks shall not be used to
    /// hide that data" — and a mask is also image data in its own right: §11.6.5.2's soft-mask
    /// image and §8.9.6.3's explicit mask each carry, sample by sample, the shape of the picture
    /// they belong to. So the portion under the region is destroyed in the mask as well as in the
    /// picture. Both are placed on the picture's unit square — "since all images shall be defined
    /// on the unit square in user space, their boundaries on the page will coincide" (§8.9.6.3)
    /// — so the picture's placement is the mask's, and the mask's own `/Width` and `/Height` are
    /// the grid its samples are cleared on.
    ///
    /// A §8.9.6.4 colour-key array is not image data: it is a test on the picture's samples,
    /// "colour values before decoding with the Decode array". Every picture is written as the
    /// integers its dictionary describes, so the ranges still test the samples they were written
    /// against and are carried with the dictionary (ADR 1371). A soft mask's §11.6.5.2 `/Matte`
    /// is read here and answered by [`Walk::matte_fill`].
    fn stated_masks(&self, shown: &str, stream: &Stream) -> Result<StatedMasks, String> {
        let mut masks = StatedMasks::default();
        for (key, role) in [("SMask", Role::SoftMask), ("Mask", Role::ExplicitMask)] {
            let Some(entry) = stream.dict.get(key) else {
                continue;
            };
            let target = self.document.resolve(entry);
            match (&target, entry.as_reference()) {
                (Object::Stream(mask), Some(id)) => {
                    if role == Role::SoftMask {
                        let matte = self.document.get_key(&mask.dict, "Matte");
                        if let Some(items) = matte.as_array() {
                            let values: Option<Vec<f32>> = items
                                .iter()
                                .map(|item| self.document.resolve(item).as_number().map(narrow))
                                .collect();
                            masks.matte = Some(values.ok_or_else(|| {
                                format!(
                                    "§11.6.5.2: the soft mask of the image /{shown} states a \
                                     /Matte that is not an array of numbers; the page is refused"
                                )
                            })?);
                        }
                    }
                    masks.stated.push(StatedMask { id, role });
                }
                (Object::Stream(_), None) => {
                    return Err(format!(
                        "§8.9.5: the image /{shown}'s /{key} is a direct stream this removal \
                         cannot replace; the page is refused"
                    ));
                }
                // Anything else names no mask image: nothing to clear.
                _ => {}
            }
        }
        Ok(masks)
    }

    /// Plans clearing one mask a picture names, on the mask's own grid and under the picture's
    /// placement, or refuses by name.
    ///
    /// The mask is copied rather than replaced wherever the picture is — a picture another page
    /// draws keeps its mask with it — and wherever the mask is reached from anything else as well
    /// (ADR 1196).
    fn plan_mask_clear(
        &self,
        picture: &str,
        mask: &StatedMask,
        picture_private: bool,
    ) -> Result<ImageClear, String> {
        let key = if mask.role == Role::SoftMask {
            "SMask"
        } else {
            "Mask"
        };
        let shown = format!("{picture}'s /{key}");
        let object = self.document.get(mask.id);
        let stream = object
            .as_stream()
            .ok_or_else(|| format!("the image /{shown} is not a stream; the page is refused"))?;
        let image = self.document.image_stream(stream).ok_or_else(|| {
            format!("the image /{shown} did not decode to samples; the page is refused")
        })?;
        let mut clear = if let Some(codec) = &image.codec {
            self.plan_codec_clear(&shown, mask.id, stream, (codec, &image.data), mask.role)?
        } else {
            self.plan_packed_clear(&shown, mask.id, stream, &image.data, mask.role)?
        };
        clear.private = picture_private || !self.owns(mask.id);
        Ok(clear)
    }

    /// Plans clearing a codec-free image in its own packed grid (§8.9.5.2), or refuses by name.
    fn plan_packed_clear(
        &self,
        shown: &str,
        image_id: ObjectId,
        stream: &Stream,
        data: &[u8],
        role: Role,
    ) -> Result<ImageClear, String> {
        let layout = self.image_layout(&stream.dict, shown)?;
        let stride = row_stride(layout)
            .ok_or_else(|| format!("the image /{shown}'s grid overflows; the page is refused"))?;
        let expected = stride
            .checked_mul(layout.height)
            .ok_or_else(|| format!("the image /{shown}'s grid overflows; the page is refused"))?;
        if data.len() < expected {
            return Err(format!(
                "§8.9.5: the image /{shown}'s sample data is shorter than its {}×{} grid; the \
                 page is refused rather than clear samples that are not there",
                layout.width, layout.height
            ));
        }
        Ok(ImageClear {
            image_id,
            ctm: self.ctm,
            regions: self.regions.clone(),
            layout,
            private: !self.owns(image_id),
            role,
            source: Source::Packed,
            fill: None,
        })
    }

    /// Plans clearing an image behind a codec from the samples its filter delivers, or refuses
    /// by name (§8.9.5, §7.4.8, §7.4.6, §7.4.7, §7.4.9).
    ///
    /// A codec's stream bytes are its input, not samples, so the packed-grid destruction the
    /// codec-free path takes cannot address them. But a codec is a filter, and Table 87 says what
    /// it delivers: "a CCITTFaxDecode or JBIG2Decode filter shall always deliver 1-bit samples, a
    /// RunLengthDecode or DCTDecode filter shall always deliver 8-bit samples". Those samples are
    /// the image's own — the integers its `/ColorSpace`, `/Decode`, colour key and a mask's
    /// `/Matte` describe — so they are taken from the codec run as a filter
    /// ([`pdf_model::image::filter_samples`], under the process's isolation as the interpreter
    /// runs it: confined in the program, principle 3), cleared in the grid the dictionary states,
    /// and written under `FlateDecode` with the dictionary carried: a lossless, non-codec filter,
    /// so the cleared region is exactly the zero it was set to and cannot round-trip back through
    /// a codec that would leak it (ADR 1371).
    ///
    /// A `JPXDecode` image is written as its own integers ([`Walk::plan_jpx_clear`]), and one
    /// that is an image mask as the one-bit samples §7.4.9 requires of it
    /// ([`Walk::plan_jpx_stencil_clear`]).
    ///
    /// Refused, each for the clause it answers to (trap 5, principle 1): an image that does not
    /// decode; one whose filter stopped short of its grid or concealed damaged rows, whose missing
    /// rows are not the image's; one decoded off the grid the dictionary states; and one whose
    /// dictionary contradicts what the filter delivers ([`Walk::filtered_layout`]).
    #[expect(
        clippy::doc_markdown,
        reason = "the comment quotes Table 87 and §7.4.9 verbatim, and a quotation is not marked up"
    )]
    fn plan_codec_clear(
        &self,
        shown: &str,
        image_id: ObjectId,
        stream: &Stream,
        (codec, codestream): (&[u8], &[u8]),
        role: Role,
    ) -> Result<ImageClear, String> {
        let stencil = matches!(
            self.document.get_key(&stream.dict, "ImageMask"),
            Object::Boolean(true)
        );
        if codec == b"JPXDecode" {
            return if stencil {
                self.plan_jpx_stencil_clear(shown, image_id, stream, codestream, role)
            } else {
                self.plan_jpx_clear(shown, image_id, stream, codestream, role)
            };
        }
        let filtered =
            pdf_model::image::filter_samples(self.document, stream).map_err(|error| {
                if let ImageError::UnsupportedFilter { filter } = &error {
                    format!(
                        "§8.9.5: the image /{shown} is encoded with the {filter} codec, whose \
                     samples this removal does not re-encode; the page is refused"
                    )
                } else {
                    format!(
                        "§8.9.5: the codec image /{shown} did not decode to samples ({error}); the \
                     page is refused"
                    )
                }
            })?;
        if let Some(said) = &filtered.shortfall {
            return Err(format!(
                "§8.9.5: the codec image /{shown} decoded short of its grid ({said}); the page is \
                 refused rather than redact a partial decode"
            ));
        }
        self.plan_filtered_clear(
            shown,
            image_id,
            stream,
            (
                Arc::from(filtered.data),
                (filtered.width, filtered.height),
                filtered.components,
                filtered.bits,
            ),
            role,
        )
    }

    /// The clear of samples a filter delivered, once they are held to the grid and the shape the
    /// dictionary states ([`Walk::filtered_layout`]).
    fn plan_filtered_clear(
        &self,
        shown: &str,
        image_id: ObjectId,
        stream: &Stream,
        (data, grid, components, bits): (Arc<[u8]>, (u32, u32), usize, u8),
        role: Role,
    ) -> Result<ImageClear, String> {
        let layout = self.filtered_layout(shown, &stream.dict, (grid, components, bits))?;
        let expected = row_stride(layout)
            .and_then(|stride| stride.checked_mul(layout.height))
            .ok_or_else(|| format!("the image /{shown}'s grid overflows; the page is refused"))?;
        if data.len() < expected {
            return Err(format!(
                "§8.9.5: the codec image /{shown} delivered fewer samples than its {}×{} grid; \
                 the page is refused rather than clear samples that are not there",
                layout.width, layout.height
            ));
        }
        Ok(ImageClear {
            image_id,
            ctm: self.ctm,
            regions: self.regions.clone(),
            layout,
            private: !self.owns(image_id),
            role,
            source: Source::Filtered(data),
            fill: None,
        })
    }

    /// The layout of samples a filter delivered, held to what the dictionary states, or the
    /// refusal naming the contradiction.
    ///
    /// The raster has to be the image's own grid, which is what the dictionary states
    /// (§8.9.5.1 Table 87) and, for a `DCTDecode` frame, what §7.4.8 puts in the encoded data;
    /// where the two part, the samples written back would not be the image the dictionary
    /// describes. The components have to be the colour space's, which is what interprets them,
    /// and an image mask's samples one bit: Table 87 says "[i]f ImageMask is true , the value of
    /// BitsPerComponent , if present, shall be 1". The depth is the filter's, which Table 87 makes
    /// the dictionary's to be "consistent with", so the written dictionary states it
    /// ([`Described::Filtered`]).
    #[expect(
        clippy::doc_markdown,
        reason = "the comment quotes Table 87 and §7.4.9 verbatim, and a quotation is not marked up"
    )]
    pub(super) fn filtered_layout(
        &self,
        shown: &str,
        dict: &Dictionary,
        (grid, components, bits): ((u32, u32), usize, u8),
    ) -> Result<ImageLayout, String> {
        let stated = (
            positive_dim(self.document, dict, "Width", shown)?,
            positive_dim(self.document, dict, "Height", shown)?,
        );
        let width = usize::try_from(grid.0).map_err(|_| {
            format!("the codec image /{shown}'s grid overflows; the page is refused")
        })?;
        let height = usize::try_from(grid.1).map_err(|_| {
            format!("the codec image /{shown}'s grid overflows; the page is refused")
        })?;
        if stated != (width, height) {
            return Err(format!(
                "§8.9.5: the codec image /{shown} states a {}×{} grid and decoded to {width}×\
                 {height}, so the samples this removal would write back are not the image's own; \
                 the page is refused",
                stated.0, stated.1
            ));
        }
        let stencil = matches!(
            self.document.get_key(dict, "ImageMask"),
            Object::Boolean(true)
        );
        if stencil {
            if (components, bits) != (1, 1) {
                return Err(format!(
                    "§8.9.5.1: the image mask /{shown}'s filter delivered {components} \
                     component(s) of {bits} bits, where Table 87 gives an image mask one bit a \
                     sample; the page is refused"
                ));
            }
        } else {
            let space = self.document.get_key(dict, "ColorSpace");
            let stated_components = ColourSpace::parse(self.document, &space, &self.resources)
                .map(|resolved| resolved.components())
                .filter(|count| *count > 0)
                .ok_or_else(|| {
                    format!(
                        "the image /{shown}'s colour space is one this removal cannot count the \
                         components of; the page is refused"
                    )
                })?;
            if stated_components != components {
                return Err(format!(
                    "§8.9.5.1: the image /{shown}'s colour space takes {stated_components} \
                     component(s) and its filter delivered {components}, so the samples are not \
                     the ones the dictionary describes; the page is refused"
                ));
            }
        }
        Ok(ImageLayout {
            width,
            height,
            components,
            bits: usize::from(bits),
        })
    }

    /// Plans clearing a `JPXDecode` image mask on the one-bit samples §7.4.9 requires of it, or
    /// refuses by name.
    ///
    /// §7.4.9: "If ImageMask is true , the JPEG 2000 data shall provide a single colour channel
    /// with 1-bit samples". Those are the samples §8.9.6.2's stencil is made of, so they are taken
    /// as the decoder's own integers at full resolution ([`pdf_model::image::jpx_samples`]),
    /// packed as §8.9.5.2 lays out one-bit samples, and cleared and carried as any filter's are
    /// ([`Walk::plan_filtered_clear`]).
    #[expect(
        clippy::doc_markdown,
        reason = "the comment quotes Table 87 and §7.4.9 verbatim, and a quotation is not marked up"
    )]
    fn plan_jpx_stencil_clear(
        &self,
        shown: &str,
        image_id: ObjectId,
        stream: &Stream,
        codestream: &[u8],
        role: Role,
    ) -> Result<ImageClear, String> {
        let samples = self.jpx_whole(shown, stream, codestream)?;
        if samples.components != 1 || samples.depths.as_slice() != [1] {
            return Err(format!(
                "§7.4.9: the JPEG 2000 image mask /{shown} decoded to {} channel(s) of {:?} bits, \
                 where the clause requires \"a single colour channel with 1-bit samples\"; the \
                 page is refused",
                samples.components, samples.depths
            ));
        }
        let width = usize::try_from(samples.width).unwrap_or(0);
        let stride = width.div_ceil(8);
        let mut packed =
            vec![0u8; stride.saturating_mul(usize::try_from(samples.height).unwrap_or(0))];
        if width > 0 {
            for (row, line) in samples.colour.chunks_exact(width).enumerate() {
                for (col, sample) in line.iter().enumerate() {
                    if *sample != 0
                        && let Some(cell) =
                            packed.get_mut(row.saturating_mul(stride).saturating_add(col / 8))
                    {
                        *cell |= 0x80u8 >> (col % 8);
                    }
                }
            }
        }
        self.plan_filtered_clear(
            shown,
            image_id,
            stream,
            (Arc::from(packed), (samples.width, samples.height), 1, 1),
            role,
        )
    }

    /// A `JPXDecode` image's own samples at full resolution, within the operator's budget, or
    /// the refusal naming the budget (ADR 1333).
    fn jpx_whole(
        &self,
        shown: &str,
        stream: &Stream,
        codestream: &[u8],
    ) -> Result<pdf_model::image::JpxSamples, String> {
        pdf_model::image::jpx_samples(
            self.document,
            &stream.dict,
            &self.resources,
            (codestream, self.image_samples),
        )
        .map_err(|error| {
            format!(
                "§7.4.9: the JPEG 2000 image /{shown} was not decoded to its own samples at \
                 full resolution ({error}); a reduced resolution level would resample every \
                 sample outside the region, so the page is refused — an operator whose machine \
                 can hold a larger decode states it with --image-samples"
            )
        })
    }

    /// Plans clearing a `JPXDecode` image on its own samples, or refuses by name (ADRs 1333,
    /// 1371).
    ///
    /// §12.5.6.23 destroys "that portion of the image data" and says nothing of the rest, which
    /// is therefore to be carried as the file had it. The decoder delivers the codestream's
    /// components as integers at a precision Table 87 leaves to the processor —
    /// "[t]he bit depth is determined by the PDF processor in the process of decoding the JPEG
    /// 2000 image" — each component at its own depth where §7.4.9's "[t]he colour components in
    /// an image may have different numbers of bits per sample" applies, and in the colour space
    /// §7.4.9's precedence names. Table 87 has one depth for an image — "the number of bits shall
    /// be the same for all colour components" — so the integers are written back under
    /// `FlateDecode` unchanged in the field of the widest ([`pdf_model::image::jpx_samples`]):
    /// eight bits where every component is eight or fewer, sixteen above, with each component's
    /// own `/Decode` pair widened from its own depth so §8.9.5.2's map gives every integer the
    /// value it had ([`widened_decode`]). Nothing outside the region is converted, requantised or
    /// resampled, so a colour key's ranges and a soft mask's `/Matte` still describe the samples
    /// they were written against and are carried with them. Table 87's `/SMaskInData` opacity
    /// becomes the soft-mask image the table has the processor "create", at the same depth.
    fn plan_jpx_clear(
        &self,
        shown: &str,
        image_id: ObjectId,
        stream: &Stream,
        codestream: &[u8],
        role: Role,
    ) -> Result<ImageClear, String> {
        self.jpx_admits_a_clear(shown, stream, codestream)?;
        let samples = self.jpx_whole(shown, stream, codestream)?;
        let stated = (
            positive_dim(self.document, &stream.dict, "Width", shown)?,
            positive_dim(self.document, &stream.dict, "Height", shown)?,
        );
        let width = usize::try_from(samples.width).unwrap_or(0);
        let height = usize::try_from(samples.height).unwrap_or(0);
        if stated != (width, height) {
            // §7.4.9 NOTE 3: a codestream over the decoder's sample budget "can select and
            // decode only the data making up a lower-resolution version", and a raster on that
            // grid is not the image — written back, it would resample everything outside the
            // region as well.
            return Err(format!(
                "§7.4.9: the JPEG 2000 image /{shown} states a {}×{} grid and the decoder's \
                 budget delivered a reduced resolution level of {width}×{height}, so the \
                 samples this removal would write back are not the image's own; the page is \
                 refused",
                stated.0, stated.1
            ));
        }
        if role == Role::SoftMask && samples.components != 1 {
            return Err(format!(
                "§11.6.5.2: the soft-mask image /{shown} decoded to {} colour components where \
                 Table 143 requires DeviceGray; the page is refused",
                samples.components
            ));
        }
        if samples.premultiplied && samples.space == pdf_model::image::JpxSpace::Stated {
            let entry = self.document.get_key(&stream.dict, "ColorSpace");
            if matches!(
                ColourSpace::parse(self.document, &entry, &self.resources),
                Some(ColourSpace::Indexed { .. })
            ) {
                return Err(format!(
                    "§8.9.5.1: the JPEG 2000 image /{shown} states /SMaskInData 2 over an \
                     Indexed space, whose indices no opacity can have multiplied; the page is \
                     refused"
                ));
            }
        }
        let bits = if samples.precision > 8 { 16 } else { 8 };
        let decode = samples
            .decode
            .iter()
            .zip(&samples.depths)
            .flat_map(|(pair, depth)| widened_decode(&[*pair], *depth, bits))
            .collect();
        let opacity_decode = widened_decode(&[(0.0, 1.0)], samples.opacity_depth, bits);
        let layout = ImageLayout {
            width,
            height,
            components: samples.components,
            bits,
        };
        let own = OwnRaster {
            space: samples.space.clone(),
            bits,
            decode,
            opacity_decode,
            premultiplied: samples.premultiplied,
            depths: samples.depths.clone(),
            pairs: samples.decode.clone(),
        };
        Ok(ImageClear {
            image_id,
            ctm: self.ctm,
            regions: self.regions.clone(),
            layout,
            private: !self.owns(image_id),
            role,
            source: Source::Own(Decoded {
                samples: Arc::from(samples.colour.as_slice()),
                opacity: samples.opacity.map(|alpha| Arc::from(alpha.as_slice())),
                own: Arc::new(own),
            }),
            fill: None,
        })
    }

    /// The conditions a `JPXDecode` image's codestream has to meet before its samples are
    /// cleared.
    ///
    /// §12.5.6.23 asks for one thing of an image — "that portion of the image data shall be
    /// destroyed; clipping or image masks shall not be used to hide that data" — and says nothing
    /// about the encoding the rest of it survives in. So decoding the codestream, zeroing the
    /// region's samples and writing the whole grid back under `FlateDecode` *is* the destruction
    /// the clause asks for: the removed samples are in the output in no form at all, and the ones
    /// outside the region are the values a reader decoded before, carried losslessly. That the
    /// codestream was lossy does not weaken it — the samples a reader sees are the decoder's
    /// output, and those are exactly what is re-encoded. What has to hold before that
    /// (ADRs 1248, 1333, 1371):
    ///
    /// - **Table 87's `/SMaskInData` is one of the three codes it defines.** Codes 1 and 2 say
    ///   the codestream carries an opacity channel from which "[a] PDF processor shall create a
    ///   soft-mask image", and [`Walk::plan_jpx_clear`] writes that image (ADR 1277).
    /// - **Every component fits a sample Table 87 can state.** §7.4.9 lets a component be "between
    ///   1 to 38 inclusive" bits; Table 87 lets an image's samples be at most sixteen — "[t]he
    ///   value shall be 1 , 2 , 4 , 8 , or (from PDF 1.5) 16" — so a deeper component's integers
    ///   have no field in any image this removal can write, and carrying them coarser would
    ///   change content the annotation did not identify.
    fn jpx_admits_a_clear(
        &self,
        shown: &str,
        stream: &Stream,
        codestream: &[u8],
    ) -> Result<(), String> {
        match self
            .document
            .get_key(&stream.dict, "SMaskInData")
            .as_integer()
        {
            None | Some(0..=2) => {}
            Some(other) => {
                return Err(format!(
                    "§8.9.5.1: the JPEG 2000 image /{shown} states /SMaskInData {other}, a code \
                     Table 87 does not define; the page is refused rather than guess what its \
                     samples carry"
                ));
            }
        }
        let headers = pdf_model::jpeg2000::Headers::parse(codestream).map_err(|error| {
            format!(
                "§7.4.9: the JPEG 2000 image /{shown} does not state its own precision \
                 ({error}); the page is refused rather than re-encode a grid this removal \
                 cannot show it preserves"
            )
        })?;
        let depths = headers.component_depths();
        if depths.is_empty() {
            return Err(format!(
                "§7.4.9: the JPEG 2000 image /{shown} states no component precision at all, so \
                 this removal cannot show its re-encode preserves the samples outside the \
                 region; the page is refused"
            ));
        }
        if let Some(deep) = depths
            .iter()
            .map(|depth| depth.bits)
            .find(|bits| *bits > 16)
        {
            return Err(format!(
                "§7.4.9: the JPEG 2000 image /{shown} states a component of {deep} bits, and \
                 Table 87 gives an image sample at most sixteen, so no image this removal writes \
                 can carry its integers; the page is refused rather than coarsen the samples \
                 outside the region"
            ));
        }
        Ok(())
    }

    /// The image's sample layout (§8.9.5): its grid, colour components and bit depth.
    pub(super) fn image_layout(
        &self,
        dict: &Dictionary,
        shown: &str,
    ) -> Result<ImageLayout, String> {
        let width = positive_dim(self.document, dict, "Width", shown)?;
        let height = positive_dim(self.document, dict, "Height", shown)?;
        let is_mask = matches!(
            self.document.get_key(dict, "ImageMask"),
            Object::Boolean(true)
        );
        let (components, bits) = if is_mask {
            // §8.9.6.2: an image mask is one bit per sample and carries no colour space of its
            // own; §8.9.5.1 Table 87 requires /BitsPerComponent 1 where it is stated at all.
            (1, 1)
        } else {
            let space = self.document.get_key(dict, "ColorSpace");
            let components = ColourSpace::parse(self.document, &space, &self.resources)
                .map(|resolved| resolved.components())
                .filter(|components| *components > 0)
                .ok_or_else(|| {
                    format!(
                        "the image /{shown}'s colour space is one this removal cannot count the \
                         components of; the page is refused"
                    )
                })?;
            let bits = match self.document.get_key(dict, "BitsPerComponent").as_integer() {
                Some(bits @ (1 | 2 | 4 | 8 | 16)) => usize::try_from(bits).unwrap_or(8),
                _ => {
                    return Err(format!(
                        "§8.9.5: the image /{shown}'s /BitsPerComponent is not 1, 2, 4, 8 or 16; \
                         the page is refused"
                    ));
                }
            };
            (components, bits)
        };
        Ok(ImageLayout {
            width,
            height,
            components,
            bits,
        })
    }

    /// Whether the redacted page owns an object outright, so the removal may replace it rather
    /// than copy it: the reference test below, **and** no shared form between the page and it —
    /// an object reached only through a form another page draws is that page's too, however few
    /// references name it (ADR 1196).
    pub(super) fn owns(&self, id: ObjectId) -> bool {
        self.inside_shared == 0 && self.exclusively_owned(id)
    }

    /// Whether an object is safe to overwrite in place: referenced exactly once in the
    /// document, and reached by a resource path this page does not share, so its marks are the
    /// redacted page's alone, so the removal may replace it. An object this cannot prove is the
    /// page's is copied for the page instead ([`private_copies`], ADR 1196).
    fn exclusively_owned(&self, image_id: ObjectId) -> bool {
        if self.counts.get(&image_id.number).copied() != Some(1) {
            return false;
        }
        // The page's own /Resources must be private: a direct dictionary on the page dict, or an
        // indirect object referenced once. Inherited resources (no /Resources on the page dict)
        // are an ancestor's, shared by construction.
        match self.page.dict.get("Resources") {
            Some(Object::Dictionary(_)) => self.xobject_private(),
            Some(Object::Reference(id)) => {
                self.counts.get(&id.number).copied() == Some(1) && self.xobject_private()
            }
            _ => false,
        }
    }
}
