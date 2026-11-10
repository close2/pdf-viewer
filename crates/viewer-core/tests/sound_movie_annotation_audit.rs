//! Legacy annotation activation reaches the ratified media-file handoff boundary.
//! This tests activation-to-file hand-off, not audio/video decoding or playback.

#![expect(
    clippy::expect_used,
    clippy::panic,
    clippy::arithmetic_side_effects,
    reason = "test fixtures must fail loudly if the consumer route cannot be reached"
)]

use std::fmt::Write as _;

use pdf_model::restriction::Operation;
use viewer_core::{
    Answer, Command, DocumentId, Event, Media, PointerAction, Query, RestrictionLevel,
    RestrictionPolicy, RestrictionScope, Viewer,
};

const DOCUMENT: DocumentId = DocumentId(1);
const SOUND_URL: &str = "https://example.org/annotation.wav";
const MOVIE_URL: &str = "https://example.org/annotation.mp4";

/// Complete PDF 1.7 page with a file-backed Sound and Movie, each with a stored normal appearance.
/// Stream lengths are calculated from the exact ASCII payload bytes; xref offsets use byte offsets.
fn document() -> Vec<u8> {
    let sound_icon = "0 0 1 rg 0 0 20 20 re f";
    let movie_icon = "0 0 1 rg 0 0 20 20 re f";
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> \
         /Contents 4 0 R /Annots [5 0 R 7 0 R] >>"
            .to_owned(),
        "<< /Length 0 >>\nstream\n\nendstream".to_owned(),
        "<< /Type /Annot /Subtype /Sound /Rect [10 60 40 90] /F 4 \
         /Sound 9 0 R /AP << /N 6 0 R >> >>"
            .to_owned(),
        form(sound_icon),
        format!(
            "<< /Type /Annot /Subtype /Movie /Rect [60 60 90 90] /F 4 \
             /Movie << /F << /FS /URL /F ({MOVIE_URL}) >> >> /AP << /N 8 0 R >> >>"
        ),
        form(movie_icon),
        format!(
            "<< /Type /Sound /R 44100 /F << /FS /URL /F ({SOUND_URL}) >> /Length 0 >>\n\
             stream\n\nendstream"
        ),
    ];
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = vec![0usize];
    for (index, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        writeln!(out, "{} 0 obj\n{}\nendobj", index + 1, body).expect("String write");
    }
    let xref = out.len();
    writeln!(out, "xref\n0 {}", offsets.len()).expect("String write");
    out.push_str("0000000000 65535 f \n");
    for offset in offsets.iter().skip(1) {
        writeln!(out, "{offset:010} 00000 n ").expect("String write");
    }
    write!(
        out,
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
        offsets.len()
    )
    .expect("String write");
    out.into_bytes()
}

fn form(content: &str) -> String {
    format!(
        "<< /Type /XObject /Subtype /Form /BBox [0 0 20 20] /Resources << >> \
         /Length {} >>\nstream\n{}\nendstream",
        content.len(),
        content
    )
}

fn opened() -> Viewer {
    let mut viewer = Viewer::new(400, 400, 1.0);
    viewer
        .handle(Command::Restrict(RestrictionScope::Window(
            RestrictionPolicy::default().with(Operation::HandOverMedia, RestrictionLevel::Off),
        )))
        .for_each(drop);
    let bytes = document();
    let parsed = pdf_syntax::Document::open(bytes.clone()).expect("complete fixture");
    assert!(!parsed.was_recovered());
    let page = pdf_model::Pages::new(&parsed).get(0).expect("page one");
    let interpreted = pdf_model::interpret(&parsed, &page);
    assert!(
        interpreted.unsupported.is_empty(),
        "{:?}",
        interpreted.unsupported
    );
    assert_eq!(
        interpreted.display_list.commands().len(),
        2,
        "both stored appearances paint"
    );
    let events = viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: bytes.into(),
            password: None,
            fragment: None,
        })
        .collect::<Vec<_>>();
    assert!(
        events
            .iter()
            .any(|event| matches!(event, Event::Opened { .. }))
    );
    assert!(media(&events).is_empty(), "opening does not activate media");
    viewer
}

fn click(viewer: &mut Viewer, rect: [f32; 4]) -> Vec<Event> {
    let Answer::Geometry(geometry) = viewer.query(Query::PageGeometry(0)) else {
        panic!("the page with the annotation is on screen");
    };
    let center_x = f32::midpoint(rect[0], rect[2]);
    let center_y = f32::midpoint(rect[1], rect[3]);
    let at = (
        geometry.origin.0 + center_x * geometry.scale,
        geometry.origin.1 + (100.0 - center_y) * geometry.scale,
    );
    let mut events = Vec::new();
    for action in [PointerAction::Pressed, PointerAction::Released] {
        events.extend(viewer.handle(Command::Pointer { at, action }));
    }
    events
}

fn media(events: &[Event]) -> Vec<Media> {
    events
        .iter()
        .filter_map(|event| match event {
            Event::Media { media, .. } => Some(media.clone()),
            _ => None,
        })
        .collect()
}

/// Missing-AP robustness fixture for the implementation's chosen Sound icon default.
/// This intentionally violates Table166's producer AP requirement; it is not conformance input.
fn sound_icon_fallback(name: Option<&str>) -> pdf_model::Interpretation {
    let name = name.map_or_else(String::new, |value| format!(" /Name /{value}"));
    let sound_bytes = b"x";
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> \
         /Contents 4 0 R /Annots [5 0 R] >>"
            .to_owned(),
        "<< /Length 0 >>\nstream\n\nendstream".to_owned(),
        format!("<< /Type /Annot /Subtype /Sound /Rect [10 60 40 90] /F 4 /Sound 6 0 R{name} >>"),
        format!(
            "<< /Type /Sound /R 8000 /C 1 /B 8 /E /Raw /Length {} >>\nstream\n{}\nendstream",
            sound_bytes.len(),
            String::from_utf8_lossy(sound_bytes)
        ),
    ];
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = vec![0usize];
    for (index, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        writeln!(out, "{} 0 obj\n{}\nendobj", index + 1, body).expect("String write");
    }
    let xref = out.len();
    writeln!(out, "xref\n0 {}", offsets.len()).expect("String write");
    out.push_str("0000000000 65535 f \n");
    for offset in offsets.iter().skip(1) {
        writeln!(out, "{offset:010} 00000 n ").expect("String write");
    }
    write!(
        out,
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
        offsets.len()
    )
    .expect("String write");
    let document = pdf_syntax::Document::open(out.into_bytes()).expect("valid syntax");
    assert!(!document.was_recovered());
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    pdf_model::interpret(&document, &page)
}

#[test]
fn a_sound_without_name_uses_the_chosen_speaker_icon_default() {
    let default = sound_icon_fallback(None);
    let speaker = sound_icon_fallback(Some("Speaker"));
    let mic = sound_icon_fallback(Some("Mic"));
    for interpreted in [&default, &speaker, &mic] {
        assert!(!interpreted.display_list.commands().is_empty());
        assert!(
            interpreted.unsupported.is_empty(),
            "{:?}",
            interpreted.unsupported
        );
    }
    assert_eq!(
        default.display_list.commands(),
        speaker.display_list.commands()
    );
    assert_ne!(default.display_list.commands(), mic.display_list.commands());
}

#[test]
fn clicking_sound_and_movie_annotations_hands_off_their_media_files() {
    let mut viewer = opened();
    assert!(media(&click(&mut viewer, [45.0, 10.0, 55.0, 20.0])).is_empty());
    let sound_events = click(&mut viewer, [10.0, 60.0, 40.0, 90.0]);
    assert_eq!(
        media(&sound_events),
        [Media::Url(SOUND_URL.to_owned())],
        "activating Sound must reach the media-file hand-off"
    );

    let movie_events = click(&mut viewer, [60.0, 60.0, 90.0, 90.0]);
    assert_eq!(
        media(&movie_events),
        [Media::Url(MOVIE_URL.to_owned())],
        "activating Movie (whose /A defaults true) must reach the media-file hand-off"
    );
}
