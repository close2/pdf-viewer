//! An inline image under a redaction region, ISO 32000-2 §8.9.7 and §12.5.6.23.
//!
//! The samples are in the content stream itself, so destroying them is a splice: the whole
//! `BI` ... `EI` run is replaced by a freshly built inline image whose region samples are zero, and
//! every other byte of the stream is left as it was.

use pdf_syntax::lexer::Lexer;
use pdf_syntax::object::{Name, Object, Stream};
use pdf_syntax::serialize::flate_encode;

use super::{ImageLayout, Walk, build_inline_image, clear_region, end_of_ei, overlaps, row_stride};

impl Walk<'_> {
    /// An inline image (§8.9.7): its `BI`\u{2026}`ID`\u{2026}`EI` run is spliced when it meets the
    /// region, and skipped so its data is never lexed otherwise.
    ///
    /// Unlike an image `XObject`, an inline image lives in the content stream itself, so
    /// destroying the samples §12.5.6.23 asks for is a byte-range edit of `/Contents` rather than
    /// the replacement of a referenced object — the whole run is replaced with a freshly built
    /// inline image whose region samples are zero, and every byte outside `[bi_start, resume)` is
    /// left exactly as it was.
    pub(super) fn inline_image(
        &mut self,
        content: &[u8],
        lexer: &mut Lexer<'_>,
        bi_start: usize,
    ) -> Result<(), String> {
        let scan = pdf_model::inline_image::scan(
            self.document,
            content,
            lexer.position(),
            &self.resources,
            true,
        );
        lexer.seek(scan.resume);
        let bbox = self.unit_square();
        if !self.regions.iter().any(|region| overlaps(*region, bbox)) {
            // The image is nowhere near a redaction: its run crosses the output byte for byte.
            return Ok(());
        }
        // The run meets the region, so its samples must be destroyed. A run that could not be read
        // is refused rather than left in place under a redaction (principle 1).
        let stream = scan.image.map_err(|error| {
            format!(
                "§8.9.7: an inline image meets the region but could not be read ({error}); the \
                 page is refused rather than leave its samples under a redaction"
            )
        })?;
        let replacement = self.spliced_inline_image(&stream)?;
        // `scan.resume` is past `EI` *and* the white space that delimits it (§7.2.3); the edit
        // ends at `EI` itself so that separator crosses the output byte for byte with the rest of
        // the surrounding stream. Trimming the white space back off `resume` finds it.
        let end = end_of_ei(content, bi_start, scan.resume);
        self.edits.push((bi_start, end, replacement));
        self.inline_cleared = self.inline_cleared.saturating_add(1);
        Ok(())
    }

    /// Builds the replacement `BI`\u{2026}`ID`\u{2026}`EI` run for an inline image whose region
    /// samples are destroyed (§12.5.6.23), or refuses the page by name where it cannot be cleared
    /// without trace (trap 5, principle 1).
    ///
    /// A codec-free image's samples come from [`Document::image_stream`] exactly as an image
    /// `XObject`'s do, are zeroed by [`clear_region`] under this placement, re-encoded
    /// `FlateDecode`, and written back under a dictionary carrying every §8.9.7 key the source
    /// stated (its `/Width`, `/Height`, `/BitsPerComponent`, colour space, `/Decode`,
    /// `/ImageMask`\u{2026}) with only the old encoding replaced. Zero is the sample domain's own
    /// constant: §8.9.5.2 maps it through any `/Decode` to that array's `Dmin`, carrying none of
    /// the original sample (ADR 1126). An image behind one of the two codecs Table 92 lets an
    /// inline image name, `DCTDecode` and `CCITTFaxDecode`, is decoded and re-expressed as an
    /// image `XObject` behind one is ([`inline_codec_raster`]).
    fn spliced_inline_image(&self, stream: &Stream) -> Result<Vec<u8>, String> {
        let image = self.document.image_stream(stream).ok_or_else(|| {
            "§8.9.7: an inline image meeting the region did not decode to samples; the page is \
             refused"
                .to_owned()
        })?;
        let mut dict = stream.dict.clone();
        let (mut samples, layout) = if let Some(codec) = &image.codec {
            let (samples, layout) = self.inline_filter_output(stream, codec)?;
            // Table 87: `/BitsPerComponent` "shall be consistent with the size of the data
            // samples that the filter delivers", and those are the samples now written.
            dict.insert(
                Name::new(&b"BitsPerComponent"[..]),
                Object::Integer(i64::try_from(layout.bits).unwrap_or(i64::MAX)),
            );
            (samples, layout)
        } else {
            let layout = self.image_layout(&stream.dict, "an inline image")?;
            (image.data.to_vec(), layout)
        };
        let stride = row_stride(layout).ok_or_else(|| {
            "§8.9.5: an inline image meeting the region has a grid that overflows; the page is \
             refused"
                .to_owned()
        })?;
        let expected = stride.checked_mul(layout.height).ok_or_else(|| {
            "§8.9.5: an inline image meeting the region has a grid that overflows; the page is \
             refused"
                .to_owned()
        })?;
        if samples.len() < expected {
            return Err(
                "§8.9.5: an inline image meeting the region has less sample data than its declared \
                 grid; the page is refused rather than clear samples that are not there"
                    .to_owned(),
            );
        }
        clear_region(
            &mut samples,
            stride,
            layout,
            (self.ctm, &self.regions),
            None,
        );
        let encoded = flate_encode(&samples, 6).ok_or_else(|| {
            "§8.9.7: an inline image's cleared samples could not be re-encoded; the page is refused"
                .to_owned()
        })?;
        build_inline_image(self.document, &dict, &self.resources, &encoded)
    }

    /// The samples an inline image's codec delivers as a filter, and their layout, or the
    /// refusal by name.
    ///
    /// §8.9.7 names the codecs an inline image may use by leaving the others out: "JBIG2Decode ,
    /// Crypt and JPXDecode are not listed … because those filters shall not be used with inline
    /// images". The two it leaves, `DCTDecode` and `CCITTFaxDecode`, deliver the image's samples
    /// as an image `XObject`'s codec does ([`pdf_model::image::filter_samples`], ADR 1371), held
    /// to what the inline dictionary states ([`Walk::filtered_layout`]), so the splice carries
    /// every key the producer wrote with only the encoding replaced. A filter the clause forbids
    /// here is refused by name rather than decoded on the file's word.
    #[expect(
        clippy::doc_markdown,
        reason = "the comment quotes §8.9.7 verbatim, and a quotation is not marked up"
    )]
    fn inline_filter_output(
        &self,
        stream: &Stream,
        codec: &[u8],
    ) -> Result<(Vec<u8>, ImageLayout), String> {
        if !matches!(codec, b"DCTDecode" | b"DCT" | b"CCITTFaxDecode" | b"CCF") {
            return Err(format!(
                "§8.9.7: an inline image meeting the region is encoded with {}, a filter that \
                 \"shall not be used with inline images\"; the page is refused",
                String::from_utf8_lossy(codec)
            ));
        }
        let shown = "an inline image";
        let filtered = pdf_model::image::filter_samples(self.document, stream).map_err(|error| {
            format!(
                "§8.9.7: an inline image meeting the region is encoded with the {} codec and did \
                 not decode to samples ({error}); the page is refused",
                String::from_utf8_lossy(codec)
            )
        })?;
        if let Some(said) = &filtered.shortfall {
            return Err(format!(
                "§8.9.7: an inline image meeting the region decoded short of its grid ({said}); \
                 the page is refused rather than redact a partial decode"
            ));
        }
        let layout = self.filtered_layout(
            shown,
            &stream.dict,
            (
                (filtered.width, filtered.height),
                filtered.components,
                filtered.bits,
            ),
        )?;
        Ok((filtered.data, layout))
    }
}
