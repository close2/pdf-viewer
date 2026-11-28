//! ISO 32000-2 §13.7.2.2's Tables 335 and 336 — a rich media annotation activated and deactivated
//! for the `Sound` and `Video` configurations this program's players present (ADRs 3854, 3855).
//!
//! Each test names the wrong answer it excludes: a press that hands a played video's asset out as a
//! file too, a `PO` annotation that plays only when pressed, a `PV` one activated by a page it is
//! not on, an `XD` annotation stopped by a page turn, a `PC` or `PI` one left playing, and a `3D`
//! configuration played or left unsaid.

use viewer_core::{Answer, Command, DocumentId, Event, PageTarget, PointerAction, Query, Viewer};

/// The document under test.
const DOCUMENT: DocumentId = DocumentId(1);

/// The rich media annotation, object 4.
const ANNOTATION: pdf_syntax::ObjectId = pdf_syntax::ObjectId::new(4, 0);

/// Assembles a document from object bodies, with object 1 the catalog.
fn assembled(objects: &[String]) -> Vec<u8> {
    let mut out: Vec<u8> = b"%PDF-2.0\n".to_vec();
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

/// Two 100-unit pages; the rich media annotation (object 4, over `/Rect [0 0 50 50]`) is on page
/// `on` (0 or 1), with `settings` as its `/RichMediaSettings` and one configuration of `kind`
/// whose one instance's asset is an embedded file of Table 44 `/Subtype` `media_type`.
fn document(on: usize, settings: &str, kind: &str, media_type: &str) -> Vec<u8> {
    let (first, second) = if on == 0 {
        ("/Annots [4 0 R]", "")
    } else {
        ("", "/Annots [4 0 R]")
    };
    assembled(&[
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Count 2 /Kids [3 0 R 9 0 R] >>".to_owned(),
        format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] {first} >>"),
        format!(
            "<< /Type /Annot /Subtype /RichMedia /Rect [0 0 50 50] /RichMediaSettings {settings} \
             /RichMediaContent << /Assets << /Names [(clip.mp4) 7 0 R] >> /Configurations \
             [5 0 R] >> >>"
        ),
        format!("<< /Type /RichMediaConfiguration /Subtype /{kind} /Instances [6 0 R] >>"),
        format!("<< /Type /RichMediaInstance /Subtype /{kind} /Asset 7 0 R >>"),
        "<< /Type /Filespec /F (clip.mp4) /UF (clip.mp4) /EF << /F 8 0 R >> >>".to_owned(),
        format!(
            "<< /Type /EmbeddedFile /Subtype /{media_type} /Length 4 >>\nstream\nmoov\nendstream"
        ),
        format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] {second} >>"),
    ])
}

/// A viewer whose window plays sound and shows video, the document opened, and the open's events
/// followed by those of the host's `Command::Presented` at its first frame — where the presented
/// page's rich media `PO` and `PV` are raised (ADR 3854).
fn opened(bytes: Vec<u8>) -> (Viewer, Vec<Event>, Vec<Event>) {
    let mut viewer = Viewer::new(400, 400, 1.0);
    viewer.handle(Command::PlaysHere(true)).for_each(drop);
    viewer.handle(Command::ShowsVideoHere(true)).for_each(drop);
    let opening = viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: bytes.into(),
            password: None,
            fragment: None,
        })
        .collect();
    let presented = viewer.handle(Command::Presented).collect();
    (viewer, opening, presented)
}

/// The events of a press and a release in the middle of the annotation on page one; `None` where
/// the page on the screen has no geometry to press in.
fn pressed(viewer: &mut Viewer) -> Option<Vec<Event>> {
    let Answer::Geometry(geometry) = viewer.query(Query::PageGeometry(0)) else {
        return None;
    };
    let at = (
        geometry.origin.0 + 25.0 * geometry.scale,
        geometry.origin.1 + (100.0 - 25.0) * geometry.scale,
    );
    let mut events = Vec::new();
    for action in [PointerAction::Pressed, PointerAction::Released] {
        events.extend(viewer.handle(Command::Pointer { at, action }));
    }
    Some(events)
}

/// The clips the events hand to this program's player, with the annotation each is associated
/// with and whether as Table 218's `/OP` 4.
fn played(events: &[Event]) -> Vec<(viewer_core::Recording, Option<viewer_core::Associated>)> {
    events
        .iter()
        .filter_map(|event| match event {
            Event::Media {
                sound: Some(sound), ..
            } => Some((sound.recording.clone(), sound.rendition)),
            _ => None,
        })
        .collect()
}

/// The annotations a stop is asked of.
fn stopped(events: &[Event]) -> Vec<pdf_syntax::ObjectId> {
    events
        .iter()
        .filter_map(|event| match event {
            Event::OperateMedia {
                annotation,
                operation: viewer_core::MediaOperation::Stop,
                ..
            } => Some(*annotation),
            _ => None,
        })
        .collect()
}

/// Every sentence the events report.
fn said(events: &[Event]) -> Vec<String> {
    events
        .iter()
        .flat_map(|event| match event {
            Event::Reported { notes, .. } => notes.clone(),
            _ => Vec::new(),
        })
        .collect()
}

/// Whether the events hand a file out to be saved.
fn extracted(events: &[Event]) -> bool {
    events
        .iter()
        .any(|event| matches!(event, Event::Extracted { .. }))
}

/// Table 335's default `XA`: a press activates the `Video` configuration, whose MP4 asset this
/// program's player shows in the annotation's rectangle, associated with the annotation as `/OP` 4
/// associates — and the asset is not handed out as a file as well. The page's open activates
/// nothing.
#[test]
fn a_press_plays_a_video_configuration_and_hands_no_file_out() {
    let (mut viewer, opening, presented) = opened(document(0, "<< >>", "Video", "video#2Fmp4"));
    assert!(played(&opening).is_empty(), "XA waits for the reader");
    assert!(played(&presented).is_empty(), "XA waits for the reader");
    let events = pressed(&mut viewer).expect("the page has a geometry");
    let played = played(&events);
    assert_eq!(played.len(), 1, "{events:?}");
    assert!(matches!(played[0].0, viewer_core::Recording::Mp4Video(_)));
    assert_eq!(
        played[0].1,
        Some(viewer_core::Associated {
            annotation: ANNOTATION,
            or_resume: true,
        })
    );
    assert!(!extracted(&events), "{events:?}");
}

/// The `3D` configuration is refused by name on a press, and its assets come out as files to save
/// as before (ADRs 2399, 3819); a `Sound` configuration whose asset is a RIFF WAVE file plays.
#[test]
fn a_three_d_configuration_is_said_and_a_sound_configuration_plays() {
    let (mut viewer, _, _) = opened(document(0, "<< >>", "3D", "model#2Fu3d"));
    let events = pressed(&mut viewer).expect("the page has a geometry");
    assert!(played(&events).is_empty(), "{events:?}");
    assert!(
        said(&events)
            .iter()
            .any(|sentence| sentence.contains("U3D and PRC")),
        "{events:?}"
    );
    assert!(extracted(&events), "{events:?}");
    let (mut viewer, _, _) = opened(document(0, "<< >>", "Sound", "audio#2Fwav"));
    let events = pressed(&mut viewer).expect("the page has a geometry");
    let played = played(&events);
    assert!(
        matches!(played.as_slice(), [(viewer_core::Recording::Wave(_), _)]),
        "{events:?}"
    );
}

/// `PO` activates the annotation when its page becomes the current page — for page one once the
/// host has presented the first frame, and never inside the open, which is page one's critical path
/// (ADR 3854); for page two at a turn, and not while another page is current. `PV` the same where
/// the page comes onto the screen.
#[test]
fn page_open_and_page_visible_activate_the_annotation_on_its_page() {
    for condition in ["PO", "PV"] {
        let settings = format!("<< /Activation << /Condition /{condition} >> >>");
        let (mut viewer, opening, presented) =
            opened(document(0, &settings, "Video", "video#2Fmp4"));
        assert!(
            played(&opening).is_empty(),
            "{condition}: page one pays nothing"
        );
        assert_eq!(played(&presented).len(), 1, "{condition}: {presented:?}");
        let again: Vec<Event> = viewer.handle(Command::Presented).collect();
        assert!(played(&again).is_empty(), "{condition}: once a document");
        let (mut viewer, opening, presented) =
            opened(document(1, &settings, "Video", "video#2Fmp4"));
        assert!(played(&opening).is_empty(), "{condition}: not its page");
        assert!(played(&presented).is_empty(), "{condition}: not its page");
        let turned: Vec<Event> = viewer.handle(Command::GoTo(PageTarget::Index(1))).collect();
        assert_eq!(played(&turned).len(), 1, "{condition}: {turned:?}");
    }
}

/// `PC` and `PI` deactivate the annotation when its page stops being current or visible — a stop
/// asked of whatever plays associated with it — and the default `XD` leaves that to the reader.
#[test]
fn page_close_and_page_invisible_deactivate_and_explicit_leaves_it_playing() {
    for condition in ["PC", "PI"] {
        let settings = format!(
            "<< /Activation << /Condition /PO >> /Deactivation << /Condition /{condition} >> >>"
        );
        let (mut viewer, _, _) = opened(document(0, &settings, "Video", "video#2Fmp4"));
        let turned: Vec<Event> = viewer.handle(Command::GoTo(PageTarget::Index(1))).collect();
        assert_eq!(stopped(&turned), [ANNOTATION], "{condition}: {turned:?}");
    }
    let (mut viewer, _, _) = opened(document(
        0,
        "<< /Activation << /Condition /PO >> >>",
        "Video",
        "video#2Fmp4",
    ));
    let turned: Vec<Event> = viewer.handle(Command::GoTo(PageTarget::Index(1))).collect();
    assert!(stopped(&turned).is_empty(), "XD: {turned:?}");
}
