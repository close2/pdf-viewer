//! ISO 32000-2 §12.3.4's thumbnail images: the producer's own miniature of a page.
//!
//! > A PDF document may contain thumbnail images representing the contents of its pages in
//! > miniature form. An interactive PDF processor may display these images on the screen,
//! > allowing the user to navigate to a page by clicking its thumbnail image
//!
//! A `/Thumb` is an image `XObject` on the *page* — the only entry in §12.3's family a page
//! carries rather than the catalog — and this module decodes it into the same RGBA raster every
//! other image in this tree becomes.
//!
//! # The clause takes entries away, and that is the whole of the work
//!
//! A thumbnail is an image dictionary with most of an image dictionary's meaning removed:
//!
//! > It has the usual structure for an image dictionary (8.9.5, "Image dictionaries"), but only
//! > the Width , Height , ColorSpace , BitsPerComponent , and Decode entries are significant;
//! > all of the other entries listed in "Table 87 -Additional entries specific to an image
//! > dictionary" shall be ignored if present.
//!
//! So a `/Thumb` carrying an `/SMask`, an `/ImageMask`, a `/Mask`, an `/Interpolate` or an `/OC`
//! is to be decoded *as though it did not*, and [`significant`] is that sentence: it copies the
//! stream dictionary, drops the eighteen entries of Table 87 the clause does not name, and hands
//! the result to [`crate::image::decode`]. The line the clause draws is between tables rather
//! than between keys — `/Filter`, `/DecodeParms` and `/Length` belong to **Table 5**, are not
//! "listed in Table 87", and stay; the clause's own EXAMPLE writes
//! `/Filter [/ASCII85Decode /DCTDecode]` on a thumbnail, which would be undecodable if they did
//! not.
//!
//! Reading this the other way round is the failure this module exists to avoid: keeping only the
//! five named entries would throw the filter away, and dropping nothing would let a stale
//! `/SMask` punch holes in a page's miniature.
//!
//! # Two constraints, checked and not enforced
//!
//! > (If a Subtype entry is specified, its value shall be Image .) The image's colour space
//! > shall be either DeviceGray or DeviceRGB , or an Indexed colour space based on one of these.
//!
//! Both are requirements on a *producer*. This reader decodes any colour space it can
//! and records what it found in [`Thumbnail::permitted_colour_space`], because refusing a
//! perfectly decodable CMYK miniature would lose a picture to enforce a rule about writing —
//! and because a count of files that break it is worth more than a refusal nobody sees. 11 of
//! the 974 corpus documents state a `/Thumb` on page one, and `tests/thumbnails.rs` measures
//! both constraints over them.
//!
//! # What this deliberately does not do
//!
//! **It does not generate one.** The clause permits it — thumbnails "are not required, and can
//! be included for some pages and not for others" — and `CLAUDE.md` principle 2 forbids doing so
//! anywhere near the launch path: rendering miniatures of pages nobody has asked for is the
//! archetype of eager work. A viewer that wants a panel of them renders the pages it is about to
//! show, which it can already do.

use pdf_render::Image;
use pdf_syntax::{Dictionary, Document, Object, Stream};

use crate::image::ImageError;

/// The entries of Table 87 a thumbnail keeps.
///
/// The clause's five, in its own order. Everything else Table 87 lists is dropped by
/// [`significant`]; everything Table 87 does *not* list is untouched, because the sentence is
/// about that table rather than about image dictionaries in general.
const SIGNIFICANT: [&str; 5] = [
    "Width",
    "Height",
    "ColorSpace",
    "BitsPerComponent",
    "Decode",
];

/// The entries of Table 87 a thumbnail ignores.
///
/// Table 87's twenty-three keys less the five above. Written out rather than computed, because
/// this list *is* the clause — an entry missing from it is an entry that keeps a meaning the
/// standard says it does not have, and that is not the kind of thing to leave to a filter over
/// a set someone else maintains.
const IGNORED: [&str; 18] = [
    "Type",
    "Subtype",
    "Intent",
    "ImageMask",
    "Mask",
    "Interpolate",
    "Alternates",
    "SMask",
    "SMaskInData",
    "Name",
    "StructParent",
    "ID",
    "OPI",
    "Metadata",
    "OC",
    "AF",
    "Measure",
    "PtData",
];

/// A page's thumbnail, decoded.
///
/// `PartialEq` because a thumbnail crosses a process boundary —
/// `viewer_confined::Reply::Thumbnail` — and a transport whose round trip cannot be asserted is a
/// transport nobody has checked.
#[derive(Debug, Clone, PartialEq)]
pub struct Thumbnail {
    /// The miniature, as RGBA8 samples at the size the dictionary declares.
    pub image: Image,
    /// Whether `/ColorSpace` is one of the three forms §12.3.4 permits.
    ///
    /// `false` is a file the clause says is wrong and this reader has drawn anyway — see the
    /// module comment. Carried rather than returned as an error so that a caller can say so and
    /// a corpus can count it.
    pub permitted_colour_space: bool,
    /// Whether a `/Subtype`, if stated, is `Image`.
    ///
    /// The other of the clause's two producer-side constraints, in a parenthesis: "(If a
    /// Subtype entry is specified, its value shall be Image .)" A thumbnail stating none is
    /// conformant — Table 87 makes `/Subtype` "[o]ptional when used only as a thumbnail image,
    /// required otherwise" — so this is `true` for an absent entry.
    pub permitted_subtype: bool,
}

/// Reads and decodes a page's `/Thumb`, if it has one.
///
/// `None` is a page with no thumbnail, which is most of them and is not a defect: the clause's
/// NOTE says they "are not required, and can be included for some pages and not for others".
///
/// # Errors
///
/// [`ImageError`], for a `/Thumb` that is a stream this crate cannot decode — the same errors
/// and the same route as any other image, since after [`significant`] it *is* any other image.
pub fn read(document: &Document, page: &Dictionary) -> Option<Result<Thumbnail, ImageError>> {
    let thumb = document.get_key(page, "Thumb");
    let stream = thumb.as_stream()?;
    let permitted_colour_space = colour_space_is_permitted(document, &stream.dict);
    let permitted_subtype = document
        .get_key(&stream.dict, "Subtype")
        .as_name()
        .is_none_or(|subtype| subtype.as_bytes() == b"Image");
    let stream = significant(stream);

    // A thumbnail hangs off a page rather than off a content stream, so there is no resource
    // dictionary in scope and a `/ColorSpace` naming one could not be resolved by anybody. The
    // three forms the clause permits are all stated inline.
    let resources = Dictionary::new();
    Some(
        crate::image::decode(
            document,
            &stream,
            &resources,
            pdf_render::Color::BLACK,
            // A thumbnail is a picture on its own, never inside a transparency group.
            &crate::colour::Conversion::device(),
        )
        .and_then(|crate::image::Flattened { image, shortfall }| {
            // A thumbnail whose filter stopped on damaged data is refused with the filter's
            // sentence, where the page's own image would be drawn as far as it reached and
            // reported beside (ADR 0794): a miniature is not the page, this type crosses two
            // wire protocols that carry no such sentence, and a report with no channel to a
            // host is a report nobody reads. The cost is a preview a host could have shown in
            // part; the residue is recorded in that ADR.
            match shortfall {
                Some(detail) => Err(ImageError::Malformed { detail }),
                None => Ok(Thumbnail {
                    image,
                    permitted_colour_space,
                    permitted_subtype,
                }),
            }
        }),
    )
}

/// The same stream with Table 87's insignificant entries removed.
///
/// This is §12.3.4's sentence as code; see the module comment for why it drops rather than
/// keeps. A stream whose dictionary holds none of them is copied unchanged, which is the common
/// case — most producers write the five entries and a filter and nothing else.
#[must_use]
pub fn significant(stream: &Stream) -> Stream {
    let mut dict = stream.dict.clone();
    for key in IGNORED {
        dict.remove(key);
    }
    debug_assert!(
        SIGNIFICANT
            .iter()
            .all(|key| stream.dict.get(key).is_none() == dict.get(key).is_none()),
        "the five significant entries survive"
    );
    Stream {
        dict,
        data: std::sync::Arc::clone(&stream.data),
        decryption_failed: stream.decryption_failed,
    }
}

/// Whether `/ColorSpace` is `DeviceGray`, `DeviceRGB`, or an `Indexed` space based on one.
///
/// The clause's own sentence, read literally: "based on one of these" restricts the *base*
/// space of the `Indexed` family (§8.6.6.3), and says nothing about how many entries its lookup
/// table has.
fn colour_space_is_permitted(document: &Document, dict: &Dictionary) -> bool {
    // The full names only. Table 92's `/G`, `/RGB` and `/I` abbreviations belong to *inline*
    // images — "the abbreviated names shall be used in place of the standard ones" inside `BI`
    // — and a thumbnail is an image XObject, so a `/Thumb` writing `/RGB` has named a colour
    // space resource that no resource dictionary is in scope to define.
    fn is_device(space: &Object) -> bool {
        space
            .as_name()
            .is_some_and(|name| matches!(name.as_bytes(), b"DeviceGray" | b"DeviceRGB"))
    }
    let space = document.get_key(dict, "ColorSpace");
    if is_device(&space) {
        return true;
    }
    let Some(array) = space.as_array() else {
        return false;
    };
    let [family, base, ..] = array else {
        return false;
    };
    document
        .resolve(family)
        .as_name()
        .is_some_and(|name| name.as_bytes() == b"Indexed")
        && is_device(&document.resolve(base))
}

#[cfg(test)]
mod tests {
    use super::{read, significant};
    use pdf_syntax::Document;

    /// Builds a document from object bodies numbered from 1.
    fn document(objects: &[&str]) -> Document {
        use std::fmt::Write as _;
        let mut out = String::from("%PDF-1.7\n");
        let mut offsets = Vec::new();
        for (index, body) in objects.iter().enumerate() {
            offsets.push(out.len());
            let _ = write!(out, "{} 0 obj\n{body}\nendobj\n", index.saturating_add(1));
        }
        let xref_at = out.len();
        let _ = write!(
            out,
            "xref\n0 {}\n0000000000 65535 f \n",
            objects.len().saturating_add(1)
        );
        for offset in &offsets {
            let _ = writeln!(out, "{offset:010} 00000 n ");
        }
        let _ = write!(
            out,
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n",
            objects.len().saturating_add(1)
        );
        Document::open(out.into_bytes()).expect("a valid file")
    }

    /// A two-pixel red-and-green thumbnail carrying four entries the clause says to ignore.
    ///
    /// The `/SMask` would make both pixels transparent, the `/Interpolate` would smooth them
    /// and the `/Decode` — which is *significant* — inverts them. So the decoded result says
    /// which sentence was applied: opaque inverted pixels mean the five were kept and the rest
    /// dropped.
    fn page_with_a_thumbnail(extra: &str) -> Document {
        let thumb = format!(
            "<< /Subtype /Image /Width 2 /Height 1 /ColorSpace /DeviceRGB /BitsPerComponent 8 \
             /Filter /ASCIIHexDecode /Length 13 /SMask 5 0 R /Interpolate true /Mask [0 0 0 0 0 0] \
             {extra} >>\nstream\nFF000000FF00>\nendstream"
        );
        document(&[
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 612 792] >>",
            "<< /Type /Page /Parent 2 0 R /Thumb 4 0 R >>",
            &thumb,
            "<< /Subtype /Image /Width 2 /Height 1 /ColorSpace /DeviceGray /BitsPerComponent 8 \
             /Filter /ASCIIHexDecode /Length 5 >>\nstream\n0000>\nendstream",
        ])
    }

    /// §12.3.4: Table 87's other entries "shall be ignored if present", and Table 5's are not.
    #[test]
    fn only_the_five_significant_entries_of_table_87_survive() {
        let doc = page_with_a_thumbnail("");
        let page = crate::Pages::new(&doc).get(0).expect("a page");
        let stream = doc.get_key(&page.dict, "Thumb");
        let stream = stream.as_stream().expect("a stream");
        let kept = significant(stream);

        for key in ["Width", "Height", "ColorSpace", "BitsPerComponent"] {
            assert!(kept.dict.get(key).is_some(), "{key} is significant");
        }
        for key in ["SMask", "Interpolate", "Mask", "Subtype"] {
            assert!(
                kept.dict.get(key).is_none(),
                "{key} is Table 87's and not one of the five"
            );
        }
        for key in ["Filter", "Length"] {
            assert!(
                kept.dict.get(key).is_some(),
                "{key} is Table 5's, which the sentence does not reach"
            );
        }
    }

    /// The ignored entries do not reach the pixels, and the significant `/Decode` does.
    ///
    /// A soft mask of two zero samples would make the thumbnail wholly transparent; the pixels
    /// come out opaque. `/Decode [1 0 1 0 1 0]` inverts each component, and red becomes cyan —
    /// so this fails in one direction if the clause's sentence is not applied and in the other
    /// if it is applied to too much.
    #[test]
    fn an_ignored_soft_mask_does_not_reach_the_pixels() {
        let doc = page_with_a_thumbnail("");
        let page = crate::Pages::new(&doc).get(0).expect("a page");
        let thumbnail = read(&doc, &page.dict)
            .expect("a /Thumb")
            .expect("it decodes");
        assert_eq!(thumbnail.image.width, 2);
        assert_eq!(
            thumbnail.image.data.as_ref(),
            [255, 0, 0, 255, 0, 255, 0, 255],
            "opaque red and green, with the /SMask ignored"
        );
        assert!(thumbnail.permitted_colour_space);
        assert!(
            thumbnail.permitted_subtype,
            "the /Subtype is /Image, which the parenthesis requires"
        );

        let inverted = page_with_a_thumbnail("/Decode [1 0 1 0 1 0]");
        let page = crate::Pages::new(&inverted).get(0).expect("a page");
        let thumbnail = read(&inverted, &page.dict)
            .expect("a /Thumb")
            .expect("it decodes");
        assert_eq!(
            thumbnail.image.data.as_ref(),
            [0, 255, 255, 255, 255, 0, 255, 255],
            "/Decode is one of the five and inverts each component"
        );
    }

    /// The colour space the clause permits, and one it does not.
    ///
    /// §12.3.4:
    ///
    /// > The image's colour space shall be either DeviceGray or DeviceRGB , or an Indexed colour
    /// > space based on one of these.
    ///
    /// An `ICCBased` thumbnail — which one corpus document writes — is decoded anyway and
    /// recorded as outside the three, because the sentence binds a producer and refusing it
    /// would lose a picture to enforce a rule about writing.
    #[test]
    fn a_colour_space_outside_the_three_is_recorded_rather_than_refused() {
        let indexed = document(&[
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 612 792] >>",
            "<< /Type /Page /Parent 2 0 R /Thumb 4 0 R >>",
            "<< /Width 2 /Height 1 /ColorSpace [/Indexed /DeviceRGB 1 <FF000000FF00>] \
             /BitsPerComponent 8 /Filter /ASCIIHexDecode /Length 5 >>\nstream\n0001>\nendstream",
        ]);
        let page = crate::Pages::new(&indexed).get(0).expect("a page");
        let thumbnail = read(&indexed, &page.dict)
            .expect("a /Thumb")
            .expect("it decodes");
        assert!(
            thumbnail.permitted_colour_space,
            "Indexed based on DeviceRGB is the clause's third form"
        );

        let icc = document(&[
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 612 792] >>",
            "<< /Type /Page /Parent 2 0 R /Thumb 4 0 R >>",
            "<< /Width 2 /Height 1 /ColorSpace [/CalRGB << /WhitePoint [0.9505 1 1.089] >>] \
             /BitsPerComponent 8 /Filter /ASCIIHexDecode /Length 13 >>\nstream\nFF000000FF00>\nendstream",
        ]);
        let page = crate::Pages::new(&icc).get(0).expect("a page");
        let thumbnail = read(&icc, &page.dict)
            .expect("a /Thumb")
            .expect("it decodes");
        assert!(
            !thumbnail.permitted_colour_space,
            "a CalRGB thumbnail is outside the three the clause names"
        );
    }

    /// A page without a `/Thumb` has no thumbnail, which is not an error.
    #[test]
    fn a_page_without_a_thumb_produces_nothing() {
        let doc = document(&[
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 612 792] >>",
            "<< /Type /Page /Parent 2 0 R >>",
        ]);
        let page = crate::Pages::new(&doc).get(0).expect("a page");
        assert!(read(&doc, &page.dict).is_none());
    }
}
