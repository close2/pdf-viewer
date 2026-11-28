//! ISO 32000-2 §13.7.2.3.5's snapshot, drawn on the page from a rich media annotation's activation
//! to its deactivation (ADR 3889).
//!
//! Table 344's `/Snapshot` is an image `XObject` to be displayed when the view is invoked, and
//! under the table:
//!
//! > If the Snapshot entry exists, the image it describes shall be displayed in place of the
//! > current artwork.
//!
//! The annotation's normal appearance (green here) is what Table 333 has "for the initial display
//! of the annotation"; the snapshot (red) stands in its place while a clip associated with the
//! annotation is in the window's players' hands, which the window says with `Command::InHand`.
//! Each test names the wrong answer it excludes: the image drawn before activation or after the
//! clip left the players, drawn by a press alone before any player took the clip, taken away when
//! one of two players holding the clip let it go, or drawn for an activation that stays refused.

#![expect(
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "test code: a malformed fixture or an out-of-range pixel should fail loudly, and the \
              page is small enough that no index can overflow"
)]

use pdf_render::Rasterizer as _;
use render_cpu::CpuRasterizer;
use viewer_core::{Answer, Command, DocumentId, Event, PointerAction, Query, Rendered, Viewer};

/// The document under test.
const DOCUMENT: DocumentId = DocumentId(1);

/// The rich media annotation, object 4.
const ANNOTATION: pdf_syntax::ObjectId = pdf_syntax::ObjectId::new(4, 0);

/// The normal appearance's colour and the snapshot's.
const GREEN: [u8; 3] = [0, 255, 0];
const RED: [u8; 3] = [255, 0, 0];

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

/// One 100-unit page holding the rich media annotation (object 4, `/Rect [0 0 50 50]`), whose
/// normal appearance fills its rectangle green and whose activation's `/View` names a one-sample
/// red image as Table 344's `/Snapshot`; one configuration of `kind` whose instance's asset is an
/// embedded file of Table 44 `/Subtype` `media_type`.
fn document(kind: &str, media_type: &str) -> Vec<u8> {
    let green = "0 1 0 rg 0 0 50 50 re f";
    assembled(&[
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Annots [4 0 R] >>".to_owned(),
        "<< /Type /Annot /Subtype /RichMedia /Rect [0 0 50 50] /F 4 /AP << /N 10 0 R >> \
         /RichMediaSettings << /Activation << /View << /XN (shot) /Snapshot 9 0 R >> >> >> \
         /RichMediaContent << /Assets << /Names [(clip.mp4) 7 0 R] >> /Configurations \
         [5 0 R] >> >>"
            .to_owned(),
        format!("<< /Type /RichMediaConfiguration /Subtype /{kind} /Instances [6 0 R] >>"),
        format!("<< /Type /RichMediaInstance /Subtype /{kind} /Asset 7 0 R >>"),
        "<< /Type /Filespec /F (clip.mp4) /UF (clip.mp4) /EF << /F 8 0 R >> >>".to_owned(),
        format!(
            "<< /Type /EmbeddedFile /Subtype /{media_type} /Length 4 >>\nstream\nmoov\nendstream"
        ),
        "<< /Type /XObject /Subtype /Image /Width 1 /Height 1 /ColorSpace /DeviceRGB \
         /BitsPerComponent 8 /Filter /ASCIIHexDecode /Length 7 >>\nstream\nFF0000>\nendstream"
            .to_owned(),
        format!(
            "<< /Type /XObject /Subtype /Form /BBox [0 0 50 50] /Length {} >>\nstream\n{green}\n\
             endstream",
            green.len()
        ),
    ])
}

/// A viewer whose window plays sound and shows video, the document opened and its first frame
/// presented; with the colour the annotation's centre is drawn in then.
fn opened(bytes: Vec<u8>) -> (Viewer, [u8; 3]) {
    let mut viewer = Viewer::new(400, 400, 1.0);
    viewer.handle(Command::PlaysHere(true)).for_each(drop);
    viewer.handle(Command::ShowsVideoHere(true)).for_each(drop);
    let events: Vec<Event> = viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: bytes.into(),
            password: None,
            fragment: None,
        })
        .collect();
    let colour = drawn(&mut viewer, &events).expect("the open renders the page");
    viewer.handle(Command::Presented).for_each(drop);
    (viewer, colour)
}

/// The colour of the annotation's centre — (25, 25) in default user space — in the page the
/// events ask to be rendered, which is rendered on the processor and handed back; `None` where
/// they ask for no render, the page on the screen being what it was.
fn drawn(viewer: &mut Viewer, events: &[Event]) -> Option<[u8; 3]> {
    let request = events.iter().find_map(|event| match event {
        Event::NeedsRender(request) => Some(request),
        _ => None,
    })?;
    let raster = CpuRasterizer::new()
        .rasterize(&request.list, request.target)
        .expect("the processor draws a fill and an image");
    let scale = f64::from(raster.width) / 100.0;
    let (x, y) = ((25.0 * scale) as usize, ((100.0 - 25.0) * scale) as usize);
    let at = (y * raster.width as usize + x) * 4;
    let colour = [raster.data[at], raster.data[at + 1], raster.data[at + 2]];
    viewer
        .handle(Command::RenderReady {
            token: request.token,
            rendered: Rendered::Raster(raster),
        })
        .for_each(drop);
    Some(colour)
}

/// The events of a press and a release in the middle of the annotation.
fn pressed(viewer: &mut Viewer) -> Vec<Event> {
    let Answer::Geometry(geometry) = viewer.query(Query::PageGeometry(0)) else {
        panic!("the page on the screen has a geometry");
    };
    let at = (
        geometry.origin.0 + 25.0 * geometry.scale,
        geometry.origin.1 + (100.0 - 25.0) * geometry.scale,
    );
    let mut events = Vec::new();
    for action in [PointerAction::Pressed, PointerAction::Released] {
        events.extend(viewer.handle(Command::Pointer { at, action }));
    }
    events
}

/// The window's player taking the clip, or letting it go.
fn in_hand(viewer: &mut Viewer, in_hand: bool) -> Vec<Event> {
    viewer
        .handle(Command::InHand {
            document: DOCUMENT,
            annotation: ANNOTATION,
            in_hand,
        })
        .collect()
}

/// Whether the events say the page changed.
fn damaged(events: &[Event]) -> bool {
    events.iter().any(|event| matches!(event, Event::Damage(_)))
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

/// Before the activation the normal appearance is drawn; the press hands the clip to the player
/// and says the snapshot will stand while it is held, and the page does not change until the
/// player says it holds the clip; while it does the snapshot is drawn in place of the appearance;
/// once the clip has left its hands — the reader's stop, its end, a deactivation's stop, which
/// the player says alike — the appearance is drawn again.
#[test]
fn the_snapshot_stands_in_for_the_appearance_while_the_player_holds_the_clip() {
    let (mut viewer, before) = opened(document("Video", "video#2Fmp4"));
    assert_eq!(before, GREEN, "the normal appearance before activation");

    let events = pressed(&mut viewer);
    assert!(
        events.iter().any(|event| matches!(
            event,
            Event::Media { sound: Some(sound), .. }
                if sound.rendition.map(|associated| associated.annotation) == Some(ANNOTATION)
        )),
        "the press hands the clip to the player, associated with the annotation: {events:?}"
    );
    assert!(
        said(&events)
            .iter()
            .any(|note| note.contains("Table 344's /Snapshot) is displayed")),
        "{:?}",
        said(&events)
    );
    assert_eq!(
        drawn(&mut viewer, &events),
        None,
        "nothing is drawn in place of the appearance before a player holds the clip"
    );

    let events = in_hand(&mut viewer, true);
    assert!(damaged(&events), "the page changed: {events:?}");
    assert_eq!(
        drawn(&mut viewer, &events),
        Some(RED),
        "the snapshot while held"
    );

    let events = in_hand(&mut viewer, false);
    assert!(damaged(&events), "the page changed back: {events:?}");
    assert_eq!(
        drawn(&mut viewer, &events),
        Some(GREEN),
        "the appearance after"
    );
}

/// A clip with sound and pictures is held by two players, which each say so: the snapshot stands
/// until both have let it go, and a player saying again what it said changes nothing.
#[test]
fn the_snapshot_stands_until_every_player_holding_the_clip_has_let_it_go() {
    let (mut viewer, _) = opened(document("Video", "video#2Fmp4"));
    pressed(&mut viewer);
    assert!(damaged(&in_hand(&mut viewer, true)));
    assert!(!damaged(&in_hand(&mut viewer, true)), "the second player");
    let events = in_hand(&mut viewer, false);
    assert!(!damaged(&events), "one player still holds it: {events:?}");
    assert_eq!(drawn(&mut viewer, &events), None, "the snapshot stands");
    let events = in_hand(&mut viewer, false);
    assert!(damaged(&events));
    assert_eq!(drawn(&mut viewer, &events), Some(GREEN));
    assert!(
        !damaged(&in_hand(&mut viewer, false)),
        "a clip no player holds cannot leave again"
    );
}

/// A `3D` configuration's activation is refused, so the annotation stays inactive and no view is
/// invoked: the press says the snapshot is not displayed, and the appearance stays.
#[test]
fn a_refused_activation_displays_no_snapshot_and_says_so() {
    let (mut viewer, _) = opened(document("3D", "model#2Fu3d"));
    let events = pressed(&mut viewer);
    assert!(
        said(&events)
            .iter()
            .any(|note| note.contains("/Snapshot) is not displayed")),
        "{:?}",
        said(&events)
    );
    assert_eq!(drawn(&mut viewer, &events), None, "the appearance stays");
}
