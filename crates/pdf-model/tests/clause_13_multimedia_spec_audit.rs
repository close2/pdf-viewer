//! Clause 13 requirements the model's readers and writers do not keep (audit 2278).
//!
//! Each test states the clause's sentence, the case that breaks it, and a control the reader
//! already gets right, so that the failure is the finding and nothing else.

#![expect(
    clippy::expect_used,
    clippy::panic,
    reason = "test code: a fixture that cannot exercise what the test is about is a failure"
)]

use std::fmt::Write as _;

use pdf_model::media::{Players, Processor, Screens, judge, movie};
use pdf_syntax::{Dictionary, Document, Object, ObjectId};

/// Assembles a document from object bodies numbered from 1, with object 1 the catalog.
fn document(objects: &[&str]) -> Document {
    let mut out = String::from("%PDF-2.0\n");
    let mut offsets = Vec::new();
    for (index, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        let _ = write!(out, "{} 0 obj\n{body}\nendobj\n", index.saturating_add(1));
    }
    let xref_at = out.len();
    let size = objects.len().saturating_add(1);
    let _ = write!(out, "xref\n0 {size}\n0000000000 65535 f \n");
    for offset in &offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    let document = Document::open(out.into_bytes()).expect("a valid fixture");
    assert!(!document.was_recovered(), "exact xref offsets");
    document
}

/// Whether the rendition `body`, object 2 beside an embedded file, is viable for the hand-off.
fn viable(body: &str) -> bool {
    let doc = document(&[
        "<< /Type /Catalog >>",
        body,
        "<< /Type /Filespec /F (clip.mp4) /UF (clip.mp4) /EF << /F 4 0 R >> >>",
        "<< /Length 4 /Type /EmbeddedFile >>\nstream\nmoov\nendstream",
    ]);
    let rendition = Object::Reference(ObjectId {
        number: 2,
        generation: 0,
    });
    judge(&doc, &rendition, Processor::default())
        .chosen()
        .is_some()
}

/// A media rendition of a clip the hand-off takes, with `options` added to the rendition.
fn rendition(options: &str) -> String {
    format!("<< /S /MR {options} /C << /S /MCD /CT (video/mp4) /D 3 0 R >> >>")
}

/// 2278-B01. ISO 32000-2 §13.2.2: "Unless otherwise stated, an object shall be considered
/// non-viable if its MH dictionary contains an unrecognised key or an unrecognised value for a
/// recognised key." §13.2.3.1 states otherwise only for an unrecognised *entry* in `BE`: a media
/// criteria dictionary's entries "shall be met regardless of whether they are in an MH or a BE
/// dictionary", and a rendition's `MH` and `BE` "shall have a single entry C … whose value shall
/// have a media criteria dictionary". Table 279's `/V`, `/P` and `/L` are arrays; a value of another
/// type in `MH` is an unrecognised value for a recognised key, and so is an `MH` `/C` that is no
/// dictionary. The media rendition reads each as absent and finds the rendition viable.
#[test]
fn an_unrecognised_value_in_a_rendition_must_honour_makes_it_non_viable_spec_audit() {
    // Controls: the plain rendition is viable, and an unrecognised key in MH is already refused.
    assert!(viable(&rendition("")));
    assert!(!viable(&rendition("/MH << /C << /Unknown true >> >>")));
    let read_as_met: Vec<&str> = [
        "/MH << /C << /V 5 >> >>",
        "/MH << /C << /P /2.0 >> >>",
        "/MH << /C << /L (en) >> >>",
        "/MH << /C 5 >>",
    ]
    .into_iter()
    .filter(|options| viable(&rendition(options)))
    .collect();
    assert!(
        read_as_met.is_empty(),
        "an unrecognised value for a recognised key in MH is non-viable (§13.2.2), and these were \
         found viable: {read_as_met:?}"
    );
}

/// The movie annotation of `movie_entries`, object 2, whose movie file is an embedded MP4 of
/// `video/` type, judged for a host whose player shows video in the annotation's rectangle.
fn shown_movie(movie_entries: &str) -> pdf_model::media::Clip {
    let doc = document(&[
        "<< /Type /Catalog >>",
        &format!(
            "<< /Type /Annot /Subtype /Movie /Rect [0 0 10 10] /Movie << /F 3 0 R \
             {movie_entries} >> >>"
        ),
        "<< /Type /Filespec /F (clip.mp4) /EF << /F 4 0 R >> >>",
        "<< /Type /EmbeddedFile /Subtype /video#2Fmp4 /Length 4 >>\nstream\nmoov\nendstream",
    ]);
    let object = doc.get(ObjectId::new(2, 0));
    let annotation: &Dictionary = object.as_dict().expect("the annotation is a dictionary");
    let processor = Processor {
        players: Players::AlsoVideoHere,
        screens: Screens {
            windows: false,
            placed: false,
            monitors: [true; 7],
        },
        ..Processor::default()
    };
    movie(&doc, annotation, None, processor).expect("the movie is shown")
}

/// 2278-B02. ISO 32000-2 §13.4, Table 306's `/Rotate`: "The number of degrees by which the movie
/// shall be rotated clockwise relative to the page. The value shall be a multiple of 90." A movie
/// this program shows itself (an embedded MP4 file of `video/` type, ADR 3594) is played unrotated
/// and the entry is only said, so the pictures stand at the page's orientation, not the one the
/// movie dictionary states.
#[test]
fn a_movie_shown_here_is_rotated_as_its_movie_dictionary_states_spec_audit() {
    // Control: the movie is played here, and with no /Rotate nothing about rotation is set aside.
    let upright = shown_movie("");
    assert!(upright.played_here, "{upright:?}");
    assert!(
        !upright
            .unhonoured
            .iter()
            .any(|said| said.contains("/Rotate")),
        "{:?}",
        upright.unhonoured
    );
    let turned = shown_movie("/Rotate 90");
    assert!(turned.played_here, "{turned:?}");
    assert!(
        !turned
            .unhonoured
            .iter()
            .any(|said| said.contains("/Rotate")),
        "Table 306's /Rotate 90 \"shall be rotated clockwise relative to the page\" by the player \
         that shows the movie, not set aside: {:?}",
        turned.unhonoured
    );
}

/// The sound object the first annotation of an XFDF file carries.
fn xfdf_sound(encoding: &str) -> Dictionary {
    let xfdf = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <xfdf xmlns=\"http://ns.adobe.com/xfdf/\" xml:space=\"preserve\"><annots>\
         <sound page=\"0\" rect=\"170,10,185,30\" rate=\"8000\" bits=\"8\" channels=\"1\" \
         encoding=\"{encoding}\"><data mode=\"filtered\" encoding=\"ascii\" length=\"4\" \
         filter=\"\">ABCD</data></sound></annots></xfdf>"
    );
    let data = pdf_model::xfdf::read(xfdf.as_bytes()).expect("a well-formed XFDF file");
    let annotation = data.annotations.first().expect("one annotation");
    let dict = annotation.dictionary.as_ref().expect("a dictionary");
    match dict.get("Sound") {
        Some(Object::Stream(sound)) => sound.dict.clone(),
        other => panic!("a sound object: {other:?}"),
    }
}

/// 2278-B03. ISO 32000-2 §13.3, Table 305's `/E` as the standard prints it lists four encodings —
/// `Raw`, `Signed`, `muLaw` and `ALaw` ("A-law–encoded samples"); `doc/md/`'s extraction drops the
/// last row, which ADR 2398 records and `pdf_model::media::Encoding::ALaw` reads. The XFDF importer
/// believes the extraction: an XFDF `<sound encoding="alaw">` writes a sound object with no `/E`,
/// which Table 305 reads as its default `Raw`, so A-law codes are played as unsigned samples.
#[test]
fn an_xfdf_a_law_sound_is_written_with_table_305_s_alaw_spec_audit() {
    // Control: µ-law, which the importer names, crosses as /E /muLaw.
    let mu = xfdf_sound("mulaw");
    assert_eq!(
        mu.get("E")
            .and_then(Object::as_name)
            .map(pdf_syntax::Name::as_bytes),
        Some(&b"muLaw"[..])
    );
    let a = xfdf_sound("alaw");
    assert_eq!(
        a.get("E")
            .and_then(Object::as_name)
            .map(pdf_syntax::Name::as_bytes),
        Some(&b"ALaw"[..]),
        "Table 305 prints ALaw; an absent /E is Raw: {a:?}"
    );
}
