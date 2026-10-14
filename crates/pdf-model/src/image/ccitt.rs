//! `CCITTFaxDecode` (ISO 32000-2 §7.4.6), decoded by the sandboxed worker; Table 11's
//! `/Rows`, `/EndOfBlock` and `/Columns` decide how many lines are owed and what a shortfall
//! leaves unpainted.

use pdf_sandbox::{Decoded, Request};
use pdf_syntax::{Document, ImageStream, Object};

use super::ImageError;
use super::space::{ColourSpace, Decode, Dictionaries, colour_space};
use super::unpack::{Painting, Samples, unpack};

/// How many scan lines the filter is asked for, from Table 11's `/Rows` and `/EndOfBlock` and
/// the image dictionary's `/Height`.
///
/// **`/Rows` is not the authority its name suggests, and reading it as one paints a block over
/// half a scan.** ISO 32000-2 §7.4.6 Table 11 gives `/EndOfBlock` the last word:
///
/// > A flag indicating whether the filter shall expect the encoded data to be terminated by an
/// > end-of-block pattern, overriding the Rows parameter. If false , the filter shall stop when
/// > it has decoded the number of lines indicated by Rows or when its data has been exhausted,
/// > whichever occurs first.
///
/// Two sentences, and both matter. The first says an `/EndOfBlock` of true — which is its
/// default, so it is what a document that never mentions the entry has — *overrides* `/Rows`
/// outright: the end-of-block pattern terminates the data, and the number of lines is whatever
/// the pattern is reached after. The second gives `/Rows` its stopping power only "[i]f false".
///
/// So a decode bounded by `/Rows` is right in exactly one of the two cases, and the height the
/// other case is bounded by is the image dictionary's — §8.9.5.1's `/Height`, which is the only
/// statement of the image's extent that Table 11 does not override. Table 11's own `/Rows` row
/// says the same thing where the entry is missing: a zero or absent value means the height "is
/// not predetermined".
///
/// This matters because producers get `/Rows` wrong. `hayro`'s issue 1337 records a CAD plot
/// whose driver stamped `/Rows 272` — a copy of `/Columns` — onto a 523-line logo, with
/// `/EndOfBlock` left at its default; honouring the 272 truncates the decode at 52% and, under
/// an `/Indexed` palette whose index 0 is dark, paints the rest black. This tree did not paint
/// that block, because the short raster then fails the height check below and the image is
/// refused with a note — but a refused logo and a black logo are two ways of not drawing the
/// producer's page, and the clause asks for neither.
///
/// **The one case where the two numbers differ is `/EndOfBlock` false with a `/Rows` below
/// `/Height`**, and there the clause binds the filter to `/Rows` on purpose: the raster it
/// delivers is genuinely shorter than the image. That is why this function's answer and the
/// image's height travel *separately* over [`pdf_sandbox`]'s pipe — the first bounds the decode,
/// the second is the grid the worker pads to — and why [`ccitt_bound_below_its_height`] says so
/// beside the drawing. One number for both jobs would refuse such an image for being the size the
/// clause asked for.
///
/// [`ccitt_bound_below_its_height`]: super::mask::ccitt_bound_below_its_height
fn ccitt_rows(rows: u32, end_of_block: bool, height: u32) -> u32 {
    match rows {
        0 => height,
        _ if end_of_block => height,
        rows => rows,
    }
}

/// What a CCITT decode says beside its drawing: the damaged rows it concealed, and where it
/// stopped short of the grid.
///
/// Worded here, where both numbers are known, so that the report names the scan lines the file
/// carried and the grid it stated rather than the decoder's error alone.
pub(super) fn ccitt_shortfall(
    bilevel: &pdf_sandbox::Bilevel,
    height: u32,
    damaged_rows: u32,
) -> Option<String> {
    let concealed = (bilevel.concealed > 0).then(|| {
        format!(
            "CCITTFaxDecode concealed {} damaged scan line(s) as Table 11's \
             /DamagedRowsBeforeError {damaged_rows} asks: each is drawn as the line above it, or \
             white where that line was concealed too",
            bilevel.concealed
        )
    });
    let stopped = bilevel.stopped_by.as_ref().map(|reason| {
        format!(
            "{reason}: the filter delivered {} of the {height} scan lines the image states \
             before the damage, which are drawn; the rest are left unpainted (§7.4.6 forbids \
             the filter any error correction or resynchronization beyond the {damaged_rows} \
             damaged row(s) Table 11's /DamagedRowsBeforeError tolerates)",
            bilevel.delivered
        )
    });
    match (concealed, stopped) {
        (Some(concealed), Some(stopped)) => Some(format!("{concealed}; {stopped}")),
        (one, other) => one.or(other),
    }
}

/// Decodes a CCITT image through the sandbox: the samples, and beside them the filter's
/// sentence where it stopped on damaged data short of the grid or concealed damaged rows.
///
/// ISO 32000-2 §7.4.6. Everything the decoder needs is in the stream's `/DecodeParms` (Table 5),
/// whose CCITT entries Table 11 defines, and every entry there has a default, so this function's
/// job is to turn a dictionary that may be absent into a complete description — and to refuse
/// the one case where the dictionary says something that cannot be honoured: a `/Columns`
/// disagreeing with `/Width` by more than Table 11's byte padding (below).
///
/// **`/DamagedRowsBeforeError`** travels as the dictionary states it, and `pdf_ccitt` applies it:
/// Table 11 defines it as "[t]he number of damaged rows of data that shall be tolerated before an
/// error occurs", where tolerating one means "locating its end in the encoded data by searching
/// for an `EndOfLine` pattern and then substituting decoded data from the previous row if the
/// previous row was not damaged, or a white scan line if the previous row was also damaged", and
/// the entry "shall apply only if `EndOfLine` is true and K is non-negative". A concealed row is
/// drawn — it is what the filter delivers — and the count is said beside the drawing, because
/// the samples in it are the row above's or white rather than the file's (ADR 1349).
///
/// §7.4.6: "The filter shall not perform any error correction or resynchronization" beyond that,
/// so the first damaged row past the tolerated number — the first of all at the default of zero —
/// is the one where "an error occurs". The scan lines before it are the filter's output and are
/// drawn; what the rest show is stated nowhere, and they are left **unpainted** — no sample, no
/// colour — which is what ADR 0356 chose for §7.3.8.2's short image, the same clause's error met
/// in a byte count rather than in a codec. (The worker pads them for the wire, in the filter's
/// own white; that colour is not painted here, because under `/BlackIs1 true` with no `/Decode`
/// it is the page's black, and a colour the file never stated is not this reader's to choose.)
/// The pair is reported beside the drawing through [`Parts::shortfall`] (ADR 0794).
///
/// [`Parts::shortfall`]: super::Parts::shortfall
pub(super) fn decode_ccitt(
    at: Dictionaries,
    source: &ImageStream,
    (width, height): (u32, u32),
    painting: Painting,
) -> Result<(Vec<u8>, Option<String>), ImageError> {
    let Dictionaries {
        document,
        dict,
        resources,
    } = at;
    let (bilevel, damaged_rows) = ccitt_bilevel(document, source, (width, height))?;
    let shortfall = ccitt_shortfall(&bilevel, height, damaged_rows);

    let space = if painting.is_mask {
        ColourSpace::Mask
    } else {
        colour_space(document, dict, resources, painting.into)?
    };
    let decode = Decode::read(document, dict, &space, 1);
    let mut rgba = unpack(
        &bilevel.rows,
        width,
        height,
        &Samples {
            bits: 1,
            space: &space,
            decode: &decode,
            // As in [`decode_jbig2`]: Table 87 makes this filter's samples one bit, which is
            // what §8.9.6.4's ranges are stated over and what [`unpack`] reads.
            colour_key: painting.colour_key,
            fill: painting.fill,
            into: painting.into,
            matte: painting.matte,
        },
    )?;
    if shortfall.is_some() {
        leave_unpainted(&mut rgba, bilevel.delivered, width);
    }
    Ok((rgba, shortfall))
}

/// The one-bit rows a `CCITTFaxDecode` filter delivers (§7.4.6) under Table 11's parameters,
/// read through the sandbox and held to the dictionary's grid, with the `/DamagedRowsBeforeError`
/// its shortfall is judged by.
pub(super) fn ccitt_bilevel(
    document: &Document,
    source: &ImageStream,
    (width, height): (u32, u32),
) -> Result<(pdf_sandbox::Bilevel, u32), ImageError> {
    let parms = source.parms.as_ref();
    let integer = |key: &str, default: i64| -> i64 {
        parms
            .map(|parms| document.get_key(parms, key))
            .and_then(|value| value.as_integer())
            .unwrap_or(default)
    };
    let flag = |key: &str, default: bool| -> bool {
        match parms.map(|parms| document.get_key(parms, key)) {
            Some(Object::Boolean(value)) => value,
            _ => default,
        }
    };

    let damaged_rows =
        u32::try_from(integer("DamagedRowsBeforeError", 0).max(0)).unwrap_or(u32::MAX);
    let k = integer("K", 0);
    let end_of_line = flag("EndOfLine", false);

    // §7.4.6 Table 11 defines `/Columns` and then says what the filter does with it:
    //
    // > The width of the image in pixels. If the value is not a multiple of 8, the filter shall
    // > adjust the width of the unencoded image to the next multiple of 8 so that each line
    // > starts on a byte boundary.
    //
    // **So a `/Columns` above `/Width` is not necessarily a disagreement**: a producer that wrote
    // the *adjusted* width wrote the width this sentence hands the filter, and the runs in the
    // data are for that many columns — believing `/Columns` is what decodes them at all, which is
    // why it and not `width` is what travels over the pipe below. §8.9.5.1's `/Width` then says
    // how many of each line's samples are the image, and the two round to the same number of
    // bytes a line, so nothing has to be reshaped and not one sample moves.
    //
    // Anything wider than the padding *is* a disagreement, because then the runs are for a line
    // this image is not and §7.3.8.2's "[a]ll of these constraints shall be consistent" has been
    // broken in a way that shifts every sample after the first run. That stays a refusal.
    let columns = u32::try_from(integer("Columns", 1728)).unwrap_or(0);
    let padded = width.checked_next_multiple_of(8).unwrap_or(width);
    if columns != width && columns != padded {
        return Err(ImageError::Malformed {
            detail: format!("CCITTFaxDecode /Columns {columns} is not the image's width {width}"),
        });
    }

    let end_of_block = flag("EndOfBlock", true);
    let stated_rows = u32::try_from(integer("Rows", 0)).unwrap_or(0);
    let parameters = pdf_sandbox::CcittParameters {
        k: i32::try_from(k).unwrap_or(0),
        columns,
        rows: ccitt_rows(stated_rows, end_of_block, height),
        height,
        end_of_line,
        encoded_byte_align: flag("EncodedByteAlign", false),
        end_of_block,
        black_is_1: flag("BlackIs1", false),
        damaged_rows_before_error: damaged_rows,
    };

    let decoded = pdf_sandbox::decode(&Request::Ccitt {
        data: &source.data,
        parameters,
    })
    .map_err(|error| ImageError::Sandboxed {
        detail: error.to_string(),
    })?;
    let Decoded::Bilevel(bilevel) = decoded else {
        return Err(ImageError::Malformed {
            detail: "CCITTFaxDecode did not decode to bilevel samples".to_owned(),
        });
    };
    // The raster comes back on the filter's own line width, which is `columns` — the same
    // number that went in, and the one the sentence above makes the line's extent. `unpack`
    // below reads `width` samples from each line and both round to the same stride.
    if (bilevel.width, bilevel.height) != (columns, height) {
        return Err(ImageError::Malformed {
            detail: format!(
                "CCITTFaxDecode produced {}x{} but the dictionary says {width}x{height}",
                bilevel.width, bilevel.height
            ),
        });
    }
    Ok((bilevel, damaged_rows))
}

/// Clears every sample from row `delivered` on: the rows a filter never delivered, unpainted.
///
/// The worker's padding of those rows is a fill for the wire's fixed size and not a statement
/// about the page, and the RGBA the unpacker made of it is cleared here for that reason.
fn leave_unpainted(rgba: &mut [u8], delivered: u32, width: u32) {
    let painted = (delivered as usize)
        .saturating_mul(width as usize)
        .saturating_mul(4);
    if let Some(rest) = rgba.get_mut(painted..) {
        rest.fill(0);
    }
}

#[cfg(test)]
mod tests {
    use super::ccitt_rows;

    /// ISO 32000-2 §7.4.6 Table 11: `/EndOfBlock` overrides `/Rows`, and its default is true.
    ///
    /// The four cases are the entry's two sentences crossed with the two values of the flag.
    /// The first row is the one `hayro`'s issue 1337 is about — a producer's wrong `/Rows`
    /// under the default flag — and it is the reason this function exists rather than a
    /// `match` at the call site: the answer is derived from a clause, so it is worth a place
    /// a test can reach.
    #[test]
    fn end_of_block_overrides_the_rows_parameter() {
        // "overriding the Rows parameter": true is the default, so a `/Rows` disagreeing with
        // `/Height` says nothing about where the decode stops.
        assert_eq!(ccitt_rows(272, true, 523), 523);
        assert_eq!(ccitt_rows(9000, true, 523), 523);
        // "If false , the filter shall stop when it has decoded the number of lines indicated
        // by Rows or when its data has been exhausted, whichever occurs first."
        assert_eq!(ccitt_rows(272, false, 523), 272);
        // Table 11's `/Rows` row: zero or absent means the height "is not predetermined", so
        // §8.9.5.1's `/Height` is the only statement of it left — whatever the flag says.
        assert_eq!(ccitt_rows(0, false, 523), 523);
        assert_eq!(ccitt_rows(0, true, 523), 523);
        // The ordinary file, where the two agree and nothing above can be observed.
        assert_eq!(ccitt_rows(523, true, 523), 523);
        assert_eq!(ccitt_rows(523, false, 523), 523);
    }
}
