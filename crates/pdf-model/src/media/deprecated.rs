//! §13.3's sounds and §13.4's movies, the two media objects PDF 2.0 deprecates, as more sources for
//! the hand-off a rendition's clip uses.
//!
//! Both clauses open with the sentence that places them:
//!
//! > The features described in this subclause are deprecated with PDF 2.0. They are superseded by
//! > the general multimedia framework described in 13.2, "Multimedia".
//!
//! and `CLAUDE.md`'s clause 13 entry admits what that framework's processor owes a reader without a
//! media engine: the media file handed to this machine's own player at the reader's level, nothing
//! played in-process. A movie's Table 306 `/F` and a sound's Table 5 `/F` each name such a file — a
//! "self-describing" one, in both clauses' words — so each becomes a [`Clip`] the way a viable
//! §13.2.4.2 clip does, and crosses through the same event (`doc/questions/A631`, ADR 2382).
//!
//! **What the hand-off cannot carry is said, never dropped** (trap 5): Table 307's activation
//! entries and Table 212's playing parameters are each something a player would have to be *told*,
//! and a player handed only the file is told nothing, so every one the document states becomes a
//! sentence in [`Clip::unhonoured`]. §13.2.2's viability is not asked of either — it is a property
//! of §13.2's renditions, and neither object is one.

use pdf_syntax::{Dictionary, Document, Object};

use super::{Clip, Data};
use crate::file_spec::FileSpec;

/// Table 307's entries, in the table's order: how a movie is to be presented, which a player
/// handed only the file is told nothing of.
const ACTIVATION: [&str; 9] = [
    "Start",
    "Duration",
    "Rate",
    "Volume",
    "ShowControls",
    "Mode",
    "Synchronous",
    "FWScale",
    "FWPosition",
];

/// The clip a movie annotation's movie is handed over as, or the sentence saying why there is none.
///
/// `action` is the movie action where one asked for the movie, `None` where §12.5.6.17's annotation
/// was activated itself. ISO 32000-2 §12.6.4.10 makes the action's entries the stronger statement:
///
/// > The contents of the activation dictionary associated with the movie annotation provide the
/// > default values. Any information specified in the movie action dictionary overrides these
/// > values.
///
/// and Table 189's `/A` `false` — "the movie shall not be played" — is about the annotation's own
/// activation, so it is read only where no action asked. Where the movie's file is embedded, Table
/// 44's `/Subtype` is the content type, because Table 306 states none: "\[t]he format of a
/// self-describing movie file shall be left unspecified".
///
/// # Errors
///
/// The sentence a person is told: an annotation whose `/A` is `false`, one with no movie
/// dictionary, and a movie whose `/F` is not a file specification.
pub fn movie(
    document: &Document,
    annotation: &Dictionary,
    action: Option<&Dictionary>,
) -> Result<Clip, String> {
    let activation = document.get_key(annotation, "A");
    if action.is_none() && matches!(activation, Object::Boolean(false)) {
        return Err(
            "Table 189's /A is false, so §12.5.6.17 says the movie is not played when the \
             annotation is activated"
                .to_owned(),
        );
    }
    let movie = document.get_key(annotation, "Movie");
    let Some(movie) = movie.as_dict() else {
        return Err(
            "Table 189 makes /Movie required and this annotation states no movie dictionary"
                .to_owned(),
        );
    };
    let Some(data) = file(document, &document.get_key(movie, "F"), "movie") else {
        return Err(
            "Table 306's required /F is not a file specification this reader can read (§7.11)"
                .to_owned(),
        );
    };
    let mut unhonoured: Vec<String> = ACTIVATION
        .iter()
        .filter(|key| {
            activation
                .as_dict()
                .is_some_and(|dict| !matches!(document.get_key(dict, key), Object::Null))
                || action.is_some_and(|dict| !matches!(document.get_key(dict, key), Object::Null))
        })
        .map(|key| format!("Table 307's /{key}"))
        .collect();
    // Table 306's /Aspect describes the movie and asks nothing of a player; /Rotate asks for the
    // picture to be turned, which a player handed the file is not told.
    if let Some(degrees) = document
        .get_key(movie, "Rotate")
        .as_integer()
        .filter(|degrees| *degrees != 0)
    {
        unhonoured.push(format!("Table 306's /Rotate {degrees}"));
    }
    Ok(Clip {
        content_type: content_type(&data),
        data,
        temporary_file: None,
        unhonoured,
    })
}

/// The clip a sound object is handed over as, or the sentence saying why there is none.
///
/// `sound` is Table 212's or Table 188's `/Sound`, `table` the name of the one it came from, and
/// `stated` the entries of the action's that a player would have to be told. §13.3 says what makes
/// a sound a file a player can be handed:
///
/// > In particular, if it contains an F (file specification) entry, the sound shall be defined in
/// > an external file. This sound file shall be selfdescribing, containing all information needed
/// > to render the sound; no additional information need be present in the PDF file.
///
/// # Errors
///
/// The sentence a person is told: a `/Sound` that is not a stream, one whose `/F` is not a file
/// specification, and one that holds its own samples — Table 305's stream, which reaches a player
/// only once a wrapper writes it as a file, and that wrapper is not built (`doc/questions/A631`).
pub fn sound(
    document: &Document,
    sound: &Object,
    table: &str,
    stated: &[&str],
) -> Result<Clip, String> {
    let resolved = document.resolve(sound);
    let Some(stream) = resolved.as_stream() else {
        return Err(format!(
            "{table}'s required /Sound is not a stream, which §13.3 says a sound object shall be"
        ));
    };
    let specification = document.get_key(&stream.dict, "F");
    if matches!(specification, Object::Null) {
        return Err(
            "the sound object holds its own samples (§13.3, Table 305) rather than naming a file, \
             and this program writes no file a player could be handed for them, so nothing was \
             handed over"
                .to_owned(),
        );
    }
    let Some(data) = file(document, &specification, "sound") else {
        return Err(
            "the sound object's /F is not a file specification this reader can read (§7.11, \
             Table 5)"
                .to_owned(),
        );
    };
    Ok(Clip {
        content_type: content_type(&data),
        data,
        temporary_file: None,
        unhonoured: stated
            .iter()
            .map(|key| format!("{table}'s /{key}"))
            .collect(),
    })
}

/// A file specification of either form (§7.11.1) as the data a player is handed: its embedded file
/// where §7.11.4 gives it one, its URL where §7.11.5 names one, and its name otherwise.
///
/// Both forms, unlike §13.2.4.2's `/D`: Table 306 and Table 5 each say "file specification", and the
/// sentence that excludes "the use of simple file specifications" is §13.2.4.2's own.
fn file(document: &Document, specification: &Object, fallback: &str) -> Option<Data> {
    let resolved = document.resolve(specification);
    let parsed = FileSpec::parse(document, &resolved)?;
    let name = parsed.display_name().unwrap_or_else(|| fallback.to_owned());
    if let Some(dict) = resolved.as_dict()
        && let Some(file) = crate::attachment::read(document, dict, name.clone())
    {
        return Some(Data::Embedded(Box::new(file)));
    }
    if let Some(url) = parsed.url() {
        return Some(Data::Url(url));
    }
    Some(Data::File(name))
}

/// Table 44's `/Subtype` of an embedded file, the one content type a self-describing file states.
fn content_type(data: &Data) -> Option<String> {
    match data {
        Data::Embedded(file) => file.media_type.clone(),
        Data::Url(_) | Data::File(_) | Data::Form => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{movie, sound};
    use crate::media::Data;
    use pdf_syntax::{Document, Object, ObjectId};

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

    /// The dictionary of object `number`.
    fn dict(doc: &Document, number: u32) -> pdf_syntax::Dictionary {
        doc.get(ObjectId::new(number, 0))
            .as_dict()
            .cloned()
            .expect("the object is a dictionary")
    }

    /// Table 306's `/F` embedded, with Table 44's `/Subtype` as the type, and every Table 307 entry
    /// the annotation's activation states said rather than dropped.
    #[test]
    fn an_embedded_movie_is_handed_over_with_its_activation_entries_said() {
        let doc = document(&[
            "<< /Type /Catalog >>",
            "<< /Type /Annot /Subtype /Movie /Rect [0 0 10 10] /Movie << /F 3 0 R /Rotate 90 >> \
             /A << /Mode /Repeat /Volume 0.5 >> >>",
            "<< /Type /Filespec /F (clip.mov) /EF << /F 4 0 R >> >>",
            "<< /Type /EmbeddedFile /Subtype /video#2Fquicktime /Length 4 >>\nstream\nmoov\nendstream",
        ]);
        let clip = movie(&doc, &dict(&doc, 2), None).expect("the movie is handed over");
        assert_eq!(clip.content_type.as_deref(), Some("video/quicktime"));
        assert!(matches!(clip.data, Data::Embedded(_)), "{:?}", clip.data);
        assert_eq!(clip.temporary_file, None, "Table 286 stands over no movie");
        assert_eq!(
            clip.unhonoured,
            [
                "Table 307's /Volume",
                "Table 307's /Mode",
                "Table 306's /Rotate 90"
            ]
        );
    }

    /// Table 189's `/A false` withholds the annotation's own activation and not a movie action's,
    /// whose dictionary the clause makes the stronger statement.
    #[test]
    fn an_activation_of_false_withholds_the_click_and_not_the_action() {
        let doc = document(&[
            "<< /Type /Catalog >>",
            "<< /Type /Annot /Subtype /Movie /Rect [0 0 10 10] /Movie << /F (clip.mov) >> /A false >>",
            "<< /S /Movie /Annotation 2 0 R /Rate 2 >>",
        ]);
        let clicked = movie(&doc, &dict(&doc, 2), None);
        assert!(
            clicked
                .as_ref()
                .is_err_and(|why| why.contains("/A is false")),
            "{clicked:?}"
        );
        let action = dict(&doc, 3);
        let played = movie(&doc, &dict(&doc, 2), Some(&action)).expect("the action plays it");
        assert!(
            matches!(&played.data, Data::File(name) if name == "clip.mov"),
            "a simple file specification is Table 306's too: {:?}",
            played.data
        );
        assert_eq!(played.unhonoured, ["Table 307's /Rate"]);
    }

    /// Table 189's required `/Movie`, absent, is said.
    #[test]
    fn a_movie_annotation_with_no_movie_dictionary_hands_nothing_over() {
        let doc = document(&[
            "<< /Type /Catalog >>",
            "<< /Type /Annot /Subtype /Movie /Rect [0 0 10 10] >>",
        ]);
        let why = movie(&doc, &dict(&doc, 2), None).expect_err("no movie, no hand-off");
        assert!(why.contains("/Movie required"), "{why}");
    }

    /// §13.3's `/F` names a self-describing file, which is handed over with the action's playing
    /// parameters said; a stream holding its own samples is said and handed nowhere.
    #[test]
    fn a_sound_is_handed_over_by_its_file_and_its_own_samples_are_said() {
        let doc = document(&[
            "<< /Type /Catalog >>",
            "<< /Type /Sound /R 22050 /F << /FS /URL /F (https://example.org/a.wav) >> /Length 0 \
             >>\nstream\n\nendstream",
            "<< /Type /Sound /R 8000 /E /muLaw /Length 2 >>\nstream\nab\nendstream",
        ]);
        let reference = |number| Object::Reference(ObjectId::new(number, 0));
        let clip = sound(&doc, &reference(2), "Table 212", &["Volume", "Repeat"])
            .expect("a sound naming its file is handed over");
        assert!(
            matches!(&clip.data, Data::Url(url) if url == "https://example.org/a.wav"),
            "{:?}",
            clip.data
        );
        assert_eq!(
            clip.unhonoured,
            ["Table 212's /Volume", "Table 212's /Repeat"]
        );
        let why = sound(&doc, &reference(3), "Table 212", &[]).expect_err("samples stay");
        assert!(why.contains("holds its own samples"), "{why}");
        let why = sound(&doc, &Object::Null, "Table 188", &[]).expect_err("not a stream");
        assert!(why.starts_with("Table 188's required /Sound"), "{why}");
    }
}
