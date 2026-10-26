//! ISO 32000-2 §12.5.6.16, §12.5.6.17, §12.6.4.9 and §12.6.4.10 — a sound and a movie, as far as a
//! processor that plays nothing performs them.
//!
//! `CLAUDE.md`'s clause 13 entry admits the media file handed to this machine's own player at the
//! reader's level, and each clause hands itself to 13.2, so a movie's Table 306 `/F` and a sound's
//! `/F` cross as `Event::Media` the way a rendition's clip does (`doc/questions/A631`, ADR 2382).
//! Each test names the wrong answer it excludes: a movie withheld because its annotation is not a
//! link, a click on `/A false` handed over anyway, a stop that hands a file over, a movie found by
//! title on the wrong page, and a reader obeying the document refused a movie that no Table 286
//! restricts.

use pdf_model::restriction::Operation;
use viewer_core::{
    Answer, Command, DocumentId, Event, Media, PointerAction, Query, RestrictionLevel,
    RestrictionPolicy, RestrictionScope, Viewer,
};

/// The document under test.
const DOCUMENT: DocumentId = DocumentId(1);

/// Assembles a document from object bodies, with object 1 the catalog.
fn assembled(objects: &[String]) -> Vec<u8> {
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

/// One page holding a movie annotation (object 4, titled `intro`, its movie embedded) and a link
/// (object 5) whose `/A` is `action`; `activation` is the movie annotation's Table 189 `/A`.
fn document(activation: &str, action: &str) -> Vec<u8> {
    assembled(&[
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Annots [4 0 R 5 0 R] >>".to_owned(),
        format!(
            "<< /Type /Annot /Subtype /Movie /Rect [0 0 50 50] /T (intro) /Movie << /F 6 0 R \
             >> {activation} >>"
        ),
        format!("<< /Type /Annot /Subtype /Link /Rect [50 50 100 100] /A {action} >>"),
        "<< /Type /Filespec /F (intro.mov) /UF (intro.mov) /EF << /F 7 0 R >> >>".to_owned(),
        "<< /Type /EmbeddedFile /Subtype /video#2Fquicktime /Length 4 >>\nstream\nmoov\nendstream"
            .to_owned(),
        "<< /Type /Sound /R 22050 /F << /FS /URL /F (https://example.org/a.wav) >> /Length 0 >>\
         \nstream\n\nendstream"
            .to_owned(),
        "<< /Type /Annot /Subtype /Sound /Rect [0 0 9 9] /Sound 8 0 R >>".to_owned(),
        "<< /Type /Annot /Subtype /Sound /Rect [0 0 9 9] /Sound 11 0 R >>".to_owned(),
        "<< /Type /Sound /R 8000 /E /muLaw /Length 2 >>\nstream\nab\nendstream".to_owned(),
        "<< /Type /Annot /Subtype /Sound /Rect [0 0 9 9] /Sound 13 0 R >>".to_owned(),
        "<< /Type /Sound /R 11025 /E /Signed /Filter /ASCIIHexDecode /Length 9 >>\nstream\n\
         00017F80>\nendstream"
            .to_owned(),
        "<< /Type /Annot /Subtype /Sound /Rect [0 0 9 9] /Sound 15 0 R >>".to_owned(),
        "<< /Type /Sound /R 8000 /CO /Private /Length 2 >>\nstream\nab\nendstream".to_owned(),
    ])
}

/// Opens the document under a level for the hand-off and activates object `number`.
fn activated(bytes: Vec<u8>, level: RestrictionLevel, number: u32) -> Vec<Event> {
    let mut viewer = Viewer::new(400, 400, 1.0);
    viewer
        .handle(Command::Restrict(RestrictionScope::Window(
            RestrictionPolicy::default().with(Operation::HandOverMedia, level),
        )))
        .for_each(drop);
    viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: bytes.into(),
            password: None,
            fragment: None,
        })
        .for_each(drop);
    viewer
        .handle(Command::Activate(pdf_syntax::ObjectId::new(number, 0)))
        .collect()
}

/// The files an event list hands over.
fn handed(events: &[Event]) -> Vec<(Option<String>, Media)> {
    events
        .iter()
        .filter_map(|event| match event {
            Event::Media {
                content_type,
                media,
                ..
            } => Some((content_type.clone(), media.clone())),
            _ => None,
        })
        .collect()
}

/// Every sentence the events report.
fn notes(events: &[Event]) -> Vec<String> {
    events
        .iter()
        .flat_map(|event| match event {
            Event::Reported { notes, .. } => notes.clone(),
            _ => Vec::new(),
        })
        .collect()
}

/// The embedded movie, as it crosses.
fn the_movie() -> (Option<String>, Media) {
    (
        Some("video/quicktime".to_owned()),
        Media::Embedded {
            name: "intro.mov".to_owned(),
            bytes: b"moov".to_vec(),
        },
    )
}

/// §12.5.6.17's "the movie shall be played" on activation: the decoded file is handed over, and
/// every Table 307 entry the activation states is said, because the player is told none of them.
#[test]
fn an_activated_movie_annotation_hands_its_movie_over_and_says_what_the_player_is_not_told() {
    let events = activated(
        document("/A << /Mode /Repeat >>", "null"),
        RestrictionLevel::Off,
        4,
    );
    assert_eq!(handed(&events), [the_movie()]);
    let said = notes(&events);
    assert!(
        said.iter()
            .any(|note| note.contains("\"intro\"") && note.contains("not told Table 307's /Mode")),
        "{said:?}"
    );
}

/// Table 189's `/A false`: "the movie shall not be played" — said, and nothing handed over.
#[test]
fn a_movie_annotation_whose_activation_is_false_hands_nothing_over() {
    let events = activated(document("/A false", "null"), RestrictionLevel::Off, 4);
    assert!(handed(&events).is_empty(), "{events:?}");
    let said = notes(&events);
    assert!(
        said.iter().any(|note| note.contains("/A is false")),
        "{said:?}"
    );
}

/// §12.6.4.10's `Play`, the movie found by Table 213's `/T` on the page the link is on, and handed
/// over though Table 189's `/A` is `false` — the action's dictionary is the stronger statement.
#[test]
fn a_movie_action_finds_its_annotation_by_title_and_hands_the_movie_over() {
    let events = activated(
        document("/A false", "<< /S /Movie /T (intro) /Operation /Play >>"),
        RestrictionLevel::Off,
        5,
    );
    assert_eq!(handed(&events), [the_movie()]);
    let missing = activated(
        document("", "<< /S /Movie /T (outro) >>"),
        RestrictionLevel::Off,
        5,
    );
    assert!(handed(&missing).is_empty(), "{missing:?}");
    assert!(
        notes(&missing)
            .iter()
            .any(|note| note.contains("no movie annotation on the page")),
        "{:?}",
        notes(&missing)
    );
}

/// `Stop`, `Pause` and `Resume` act on nothing playing: each is said and none hands a file over.
#[test]
fn stop_pause_and_resume_hand_nothing_over_and_say_so() {
    for operation in ["Stop", "Pause", "Resume"] {
        let events = activated(
            document(
                "",
                &format!("<< /S /Movie /Annotation 4 0 R /Operation /{operation} >>"),
            ),
            RestrictionLevel::Off,
            5,
        );
        assert!(handed(&events).is_empty(), "{operation}: {events:?}");
        assert!(
            notes(&events)
                .iter()
                .any(|note| note.contains("nothing plays in this program")),
            "{operation}: {:?}",
            notes(&events)
        );
    }
}

/// No Table 286 stands over a movie, so a reader obeying every restriction the document asserts is
/// handed it — the opposite of a rendition's clip at its default `TEMPNEVER`.
#[test]
fn a_reader_obeying_the_document_is_handed_a_movie_no_media_permissions_restrict() {
    let events = activated(
        document("", "<< /S /Movie /Annotation 4 0 R >>"),
        RestrictionLevel::On,
        5,
    );
    assert_eq!(handed(&events), [the_movie()]);
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Event::Refused { .. } | Event::Asking { .. })),
        "{events:?}"
    );
}

/// §13.3's `/F`: a sound action's and a sound annotation's file is handed over by its URL, and
/// Table 212's `/Repeat` is said.
#[test]
fn a_sound_is_handed_over_by_its_file() {
    let action = activated(
        document("", "<< /S /Sound /Sound 8 0 R /Repeat true >>"),
        RestrictionLevel::Off,
        5,
    );
    let url = (None, Media::Url("https://example.org/a.wav".to_owned()));
    assert_eq!(handed(&action), std::slice::from_ref(&url));
    assert!(
        notes(&action)
            .iter()
            .any(|note| note.contains("not told Table 212's /Repeat")),
        "{:?}",
        notes(&action)
    );
    assert_eq!(
        handed(&activated(document("", "null"), RestrictionLevel::Off, 9)),
        [url]
    );
}

/// The RIFF WAVE file the hand-off writes for samples of `rate`, mono and 8-bit, under RFC 2361's
/// `tag` — the PCM header, or the non-PCM one with its `fact` chunk — around `data`.
fn wave_file(tag: u16, rate: u32, data: &[u8]) -> Vec<u8> {
    let companded = tag != 1;
    let mut out = Vec::new();
    let header: u32 = if companded { 58 } else { 44 };
    let length = u32::try_from(data.len()).unwrap_or(u32::MAX);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(
        &header
            .saturating_sub(8)
            .saturating_add(length)
            .saturating_add(length & 1)
            .to_le_bytes(),
    );
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&(if companded { 18_u32 } else { 16 }).to_le_bytes());
    out.extend_from_slice(&tag.to_le_bytes());
    out.extend_from_slice(&1_u16.to_le_bytes());
    out.extend_from_slice(&rate.to_le_bytes());
    out.extend_from_slice(&rate.to_le_bytes());
    out.extend_from_slice(&1_u16.to_le_bytes());
    out.extend_from_slice(&8_u16.to_le_bytes());
    if companded {
        out.extend_from_slice(&0_u16.to_le_bytes());
        out.extend_from_slice(b"fact");
        out.extend_from_slice(&4_u32.to_le_bytes());
        out.extend_from_slice(&length.to_le_bytes());
    }
    out.extend_from_slice(b"data");
    out.extend_from_slice(&length.to_le_bytes());
    out.extend_from_slice(data);
    if length % 2 == 1 {
        out.push(0);
    }
    out
}

/// §13.3's "the sound object itself shall contain the sample data": the hand-off decodes the
/// stream through Table 5's filters and hands a RIFF WAVE file over — µ-law codes as they are, and
/// 8-bit `Signed` samples re-biased to the form's unsigned bytes. The wrong answers excluded: the
/// samples withheld, handed over still filtered, or handed over in the stream's own signedness;
/// and a `/CO`, for which no standard value is defined, is said and hands nothing over.
#[test]
fn a_sound_holding_its_own_samples_is_handed_over_as_a_wave_file() {
    let wave = |media: Vec<u8>, codec: u16| {
        vec![(
            Some(format!("audio/vnd.wave; codec={codec}")),
            Media::Embedded {
                name: "sound.wav".to_owned(),
                bytes: media,
            },
        )]
    };
    let law = activated(document("", "null"), RestrictionLevel::Off, 10);
    assert_eq!(handed(&law), wave(wave_file(7, 8000, b"ab"), 7), "{law:?}");
    let signed = activated(document("", "null"), RestrictionLevel::Off, 12);
    assert_eq!(
        handed(&signed),
        wave(wave_file(1, 11025, &[0x80, 0x81, 0xFF, 0x00]), 1),
        "{signed:?}"
    );
    let compressed = activated(document("", "null"), RestrictionLevel::Off, 14);
    assert!(handed(&compressed).is_empty(), "{compressed:?}");
    assert!(
        notes(&compressed)
            .iter()
            .any(|note| note.contains("/CO /Private")),
        "{:?}",
        notes(&compressed)
    );
}

/// A press and a release on the movie annotation — the path a window's click takes — hand the movie
/// over: §12.5.1's "exhibits its associated object, such as … by playing a sound or a movie" for an
/// annotation that is not a link.
#[test]
fn a_click_on_a_movie_annotation_hands_its_movie_over() {
    let mut viewer = Viewer::new(400, 400, 1.0);
    viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: document("", "null").into(),
            password: None,
            fragment: None,
        })
        .for_each(drop);
    let Answer::Geometry(geometry) = viewer.query(Query::PageGeometry(0)) else {
        panic!("the page on the screen has a geometry");
    };
    // The middle of the movie annotation's `/Rect [0 0 50 50]` on the 100-unit page, flipped about
    // the page's height into the viewport (trap 12a).
    let at = (
        geometry.origin.0 + 25.0 * geometry.scale,
        geometry.origin.1 + (100.0 - 25.0) * geometry.scale,
    );
    let mut events: Vec<Event> = Vec::new();
    for action in [PointerAction::Pressed, PointerAction::Released] {
        events.extend(viewer.handle(Command::Pointer { at, action }));
    }
    assert_eq!(handed(&events), [the_movie()], "{events:?}");
}
