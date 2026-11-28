//! What the windows' video tests share: a document whose screen annotation plays an embedded MP4
//! clip with pictures in its own rectangle, and links that stop, pause and resume it (RFC 0015
//! section 6; ADR 3545). The clip is the caller's — `pdf-media-worker`'s hand-built `I_PCM`
//! stream, whose every picture is known before it is decoded.
//!
//! **No person has watched a clip through either window in these tests**: they run under a virtual
//! display. What is proved is that the picture a window holds up over the rectangle is the clip's
//! own samples converted, and that the window's keys, links and closes end it — and, for a rich
//! media annotation, that the window tells the core when its players take the clip and when it
//! leaves them, so that Table 344's snapshot stands for exactly that long (ADR 3889).

use std::fmt::Write as _;

/// The screen annotation the clip plays in.
pub(super) const SCREEN: pdf_syntax::ObjectId = pdf_syntax::ObjectId::new(4, 0);

/// The link annotations whose rendition actions stop, pause and resume it, in that order.
pub(super) const STOP: pdf_syntax::ObjectId = pdf_syntax::ObjectId::new(10, 0);
/// See [`STOP`].
pub(super) const PAUSE: pdf_syntax::ObjectId = pdf_syntax::ObjectId::new(11, 0);
/// See [`STOP`].
pub(super) const RESUME: pdf_syntax::ObjectId = pdf_syntax::ObjectId::new(12, 0);

/// A page with a screen annotation (object 4) whose rendition action — Table 218's `/OP` 0 naming
/// it as `/AN` — plays `clip`, an embedded MP4 file of `/CT (video/mp4)`, in the annotation's
/// rectangle (Table 294's `/W` 3, stated outright in the rendition's `/SP /BE` though it is the
/// default); and three links (objects 10, 11, 12) whose rendition actions name the same annotation
/// with `/OP` 1, 2 and 3 (ISO 32000-2 §12.6.4.14).
#[must_use]
pub(super) fn a_video_document(clip: &[u8]) -> Vec<u8> {
    a_video_document_in(clip, "/BE << /W 3 >>")
}

/// [`a_video_document`] with the rendition's Table 293 media screen parameters' entries `screen`
/// in place of `/BE << /W 3 >>` — a floating or full-screen window (Table 294's `/W` 0 or 1) for
/// the windows that open one (ADR 3587).
#[must_use]
pub(super) fn a_video_document_in(clip: &[u8], screen: &str) -> Vec<u8> {
    video_document(clip, screen, "[20 100 84 164]", "")
}

/// [`a_video_document`] whose screen annotation is two points square, `[20 100 22 102]`, and
/// whose rendition's media play parameters must honour Table 291's `/F 3` — "The media shall not
/// be scaled. A scrolling user interface shall be provided if the media rectangle is wider or
/// taller than the play rectangle" — so a picture of sixteen samples a side overflows the
/// rectangle at any magnification up to eight, and the window offers its scrolling interface
/// (ADR 3901).
#[must_use]
pub(super) fn a_scrolling_video_document(clip: &[u8]) -> Vec<u8> {
    video_document(
        clip,
        "/BE << /W 3 >>",
        "[20 100 22 102]",
        "/P << /MH << /F 3 >> >>",
    )
}

/// The video document with the screen annotation's `rect` and the rendition's further entries
/// `parameters`.
fn video_document(clip: &[u8], screen: &str, rect: &str, parameters: &str) -> Vec<u8> {
    let mut hex = String::with_capacity(clip.len().saturating_mul(2).saturating_add(1));
    for byte in clip {
        let _ = write!(hex, "{byte:02x}");
    }
    hex.push('>');
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Annots [4 0 R 10 0 R 11 0 R 12 0 \
         R] >>"
            .to_owned(),
        format!("<< /Type /Annot /Subtype /Screen /P 3 0 R /Rect {rect} /A 5 0 R >>"),
        "<< /Type /Action /S /Rendition /OP 0 /AN 4 0 R /R 6 0 R >>".to_owned(),
        format!("<< /Type /Rendition /S /MR /N (clip) /C 7 0 R /SP << {screen} >> {parameters} >>"),
        "<< /Type /MediaClip /S /MCD /CT (video/mp4) /D 8 0 R /P << /TF (TEMPACCESS) >> >>"
            .to_owned(),
        "<< /Type /Filespec /F (clip.mp4) /UF (clip.mp4) /EF << /F 9 0 R >> >>".to_owned(),
        format!(
            "<< /Type /EmbeddedFile /Filter /ASCIIHexDecode /Length {} >>\nstream\n{hex}\nendstream",
            hex.len()
        ),
        "<< /Type /Annot /Subtype /Link /Rect [100 0 110 10] /A << /S /Rendition /OP 1 /AN 4 0 R \
         >> >>"
            .to_owned(),
        "<< /Type /Annot /Subtype /Link /Rect [110 0 120 10] /A << /S /Rendition /OP 2 /AN 4 0 R \
         >> >>"
            .to_owned(),
        "<< /Type /Annot /Subtype /Link /Rect [120 0 130 10] /A << /S /Rendition /OP 3 /AN 4 0 R \
         >> >>"
            .to_owned(),
    ];
    file_of(&objects)
}

/// The movie annotation [`a_movie_document`] places.
pub(super) const MOVIE: pdf_syntax::ObjectId = pdf_syntax::ObjectId::new(4, 0);

/// A page with a movie annotation (object 4, ISO 32000-2 §12.5.6.17) over the same rectangle as
/// [`a_video_document`]'s screen annotation, whose Table 306 movie is `clip` — embedded, Table 44's
/// `/Subtype` naming it `video/mp4` — with the movie dictionary's further entries `movie` (Table
/// 306's `/Poster true`, say) and the activation dictionary `activation` (Table 307's entries, the
/// annotation's `/A`) (ADRs 3594, 3609).
#[must_use]
pub(super) fn a_movie_document(clip: &[u8], movie: &str, activation: &str) -> Vec<u8> {
    let mut hex = String::with_capacity(clip.len().saturating_mul(2).saturating_add(1));
    for byte in clip {
        let _ = write!(hex, "{byte:02x}");
    }
    hex.push('>');
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Annots [4 0 R] >>".to_owned(),
        format!(
            "<< /Type /Annot /Subtype /Movie /P 3 0 R /Rect [20 100 84 164] /Movie << /F 5 0 R \
             {movie} >> /A {activation} >>"
        ),
        "<< /Type /Filespec /F (movie.mp4) /UF (movie.mp4) /EF << /F 6 0 R >> >>".to_owned(),
        format!(
            "<< /Type /EmbeddedFile /Subtype /video#2Fmp4 /Filter /ASCIIHexDecode /Length {} \
             >>\nstream\n{hex}\nendstream",
            hex.len()
        ),
    ];
    file_of(&objects)
}

/// The rich media annotation [`a_rich_media_document`] places.
pub(super) const RICH_MEDIA: pdf_syntax::ObjectId = pdf_syntax::ObjectId::new(4, 0);

/// What a page interpreted while [`a_rich_media_document`]'s snapshot is drawn reports: the
/// placement this program chose for Table 344's image (ADR 3889).
pub(super) const SNAPSHOT_SHOWN: &str = "view's snapshot is displayed";

/// A page with a rich media annotation (object 4, ISO 32000-2 §13.7.2) over the same rectangle as
/// [`a_video_document`]'s screen annotation, whose one `Video` configuration's instance plays
/// `clip`, an embedded file of Table 44 `/Subtype` `video/mp4`, embedded in the annotation's
/// rectangle (Table 338's default); its normal appearance fills the rectangle green, and the view
/// its activation invokes names a one-sample red image as Table 344's `/Snapshot`, which is drawn
/// in place of the appearance while a player holds the clip (ADR 3889).
#[must_use]
pub(super) fn a_rich_media_document(clip: &[u8]) -> Vec<u8> {
    let mut hex = String::with_capacity(clip.len().saturating_mul(2).saturating_add(1));
    for byte in clip {
        let _ = write!(hex, "{byte:02x}");
    }
    hex.push('>');
    let green = "0 1 0 rg 0 0 64 64 re f";
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Annots [4 0 R] >>".to_owned(),
        "<< /Type /Annot /Subtype /RichMedia /P 3 0 R /Rect [20 100 84 164] /F 4 \
         /AP << /N 10 0 R >> /RichMediaSettings << /Activation << /View << /XN (shot) \
         /Snapshot 9 0 R >> >> >> /RichMediaContent << /Assets << /Names [(clip.mp4) 7 0 R] >> \
         /Configurations [5 0 R] >> >>"
            .to_owned(),
        "<< /Type /RichMediaConfiguration /Subtype /Video /Instances [6 0 R] >>".to_owned(),
        "<< /Type /RichMediaInstance /Subtype /Video /Asset 7 0 R >>".to_owned(),
        "<< /Type /Filespec /F (clip.mp4) /UF (clip.mp4) /EF << /F 8 0 R >> >>".to_owned(),
        format!(
            "<< /Type /EmbeddedFile /Subtype /video#2Fmp4 /Filter /ASCIIHexDecode /Length {} \
             >>\nstream\n{hex}\nendstream",
            hex.len()
        ),
        "<< /Type /XObject /Subtype /Image /Width 1 /Height 1 /ColorSpace /DeviceRGB \
         /BitsPerComponent 8 /Filter /ASCIIHexDecode /Length 7 >>\nstream\nFF0000>\nendstream"
            .to_owned(),
        format!(
            "<< /Type /XObject /Subtype /Form /BBox [0 0 64 64] /Length {} >>\nstream\n{green}\n\
             endstream",
            green.len()
        ),
    ];
    file_of(&objects)
}

/// A one-revision file of `objects`, numbered from 1, with its exact cross-reference table.
fn file_of(objects: &[String]) -> Vec<u8> {
    let mut out: Vec<u8> = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (index, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(
            format!("{} 0 obj\n{body}\nendobj\n", index.saturating_add(1)).as_bytes(),
        );
    }
    let xref_at = out.len();
    let size = objects.len().saturating_add(1);
    out.extend_from_slice(format!("xref\n0 {size}\n0000000000 65535 f \n").as_bytes());
    for offset in &offsets {
        out.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!("trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n")
            .as_bytes(),
    );
    out
}
