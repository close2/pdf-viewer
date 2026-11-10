//! Public Viewer regressions for §12.4.4.2 arrival and Table165 timing.

#![expect(
    clippy::arithmetic_side_effects,
    reason = "test fixture object IDs and fixed pixel-free times are deliberately small"
)]
#![expect(
    clippy::panic,
    reason = "a fixture that does not reach its assertion must fail rather than pass vacuously"
)]

use std::fmt::Write as _;

use pdf_syntax::ObjectId;
use viewer_core::{
    Answer, Command, DocumentId, Event, Layer, PageTarget, PresentationMode, Query, Viewer,
};

/// Three complete pages; destination pages two and three have a two-node, doubly linked sequence.
/// Arriving there should run the primary node's NA and make Next current, but should not run
/// the second node's NA until a later user request. The second node's PA exposes which node
/// became current. It also has a two-second Dur for the boundary test below.
fn fixture() -> Vec<u8> {
    fixture_with_transition("")
}

fn fixture_with_transition(transition: &str) -> Vec<u8> {
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R /OCProperties << /OCGs [11 0 R 12 0 R] /D << /BaseState /OFF /Order [11 0 R 12 0 R] >> >> >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R 4 0 R 5 0 R] /Count 3 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 20 20] /Resources <<>> /Contents 6 0 R >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 20 20] /Resources <<>> /Contents 7 0 R /PresSteps 9 0 R >>".to_owned(),
        format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 20 20] /Resources <<>> /Contents 8 0 R /PresSteps 9 0 R {transition} >>"),
        "<< /Length 0 >>\nstream\nendstream".to_owned(),
        "<< /Length 0 >>\nstream\nendstream".to_owned(),
        "<< /Length 0 >>\nstream\nendstream".to_owned(),
        "<< /Type /NavNode /NA 13 0 R /Next 10 0 R >>".to_owned(),
        "<< /Type /NavNode /NA 14 0 R /PA 15 0 R /Prev 9 0 R /Dur 2 >>".to_owned(),
        "<< /Type /OCG /Name (arrival action) >>".to_owned(),
        "<< /Type /OCG /Name (next node action) >>".to_owned(),
        "<< /S /SetOCGState /State [/ON 11 0 R] >>".to_owned(),
        "<< /S /SetOCGState /State [/ON 12 0 R] >>".to_owned(),
        "<< /S /SetOCGState /State [/OFF 11 0 R] >>".to_owned(),
    ];
    let mut bytes = String::from("%PDF-2.0\n");
    let mut offsets = Vec::with_capacity(objects.len());
    for (i, body) in objects.iter().enumerate() {
        offsets.push(bytes.len());
        let _ = writeln!(bytes, "{} 0 obj\n{body}\nendobj", i + 1);
    }
    let xref = bytes.len();
    let _ = writeln!(bytes, "xref\n0 {}\n0000000000 65535 f ", objects.len() + 1);
    for offset in offsets {
        let _ = writeln!(bytes, "{offset:010} 00000 n ");
    }
    let _ = write!(
        bytes,
        "trailer\n<< /Size {} /Root 1 0 R /ID [<0123456789abcdef0123456789abcdef> <fedcba9876543210fedcba9876543210>] >>\nstartxref\n{xref}\n%%EOF\n",
        objects.len() + 1
    );
    bytes.into_bytes()
}

fn presenting_viewer() -> Viewer {
    viewer_for(fixture())
}

fn viewer_for(bytes: Vec<u8>) -> Viewer {
    let mut viewer = Viewer::new(20, 20, 1.0);
    let events: Vec<Event> = viewer
        .handle(Command::Open {
            id: DocumentId(1),
            bytes: bytes.into(),
            password: None,
            fragment: None,
        })
        .collect();
    assert!(
        events
            .iter()
            .any(|event| matches!(event, Event::Opened { .. }))
    );
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Event::OpenFailed { .. })),
        "fixture is accepted without a reader failure"
    );
    let _ = viewer
        .handle(Command::Present(PresentationMode::On))
        .count();
    viewer
}

fn layer_on(viewer: &Viewer, object: u32) -> bool {
    let Answer::Layers(layers) = viewer.query(Query::Layers) else {
        panic!("open document answers the layer query");
    };
    layers
        .iter()
        .find_map(|layer| match layer {
            Layer::Group { group, on, .. } if *group == ObjectId::new(object, 0) => Some(*on),
            _ => None,
        })
        .unwrap_or_else(|| panic!("fixture OCG {object} is present"))
}

fn current_page(viewer: &Viewer) -> usize {
    let Answer::Page { index, .. } = viewer.query(Query::CurrentPage) else {
        panic!("open document answers the current page query");
    };
    index
}

/// A direct/random-access arrival applies the target page's primary node, executes its NA,
/// and selects its Next node. The later backward request runs the selected second node's PA.
#[test]
fn random_access_arrival_runs_primary_action_then_selects_next_node() {
    let mut viewer = presenting_viewer();
    assert!(!layer_on(&viewer, 11));
    assert!(!layer_on(&viewer, 12));

    let _ = viewer.handle(Command::GoTo(PageTarget::Index(1))).count();
    assert_eq!(
        current_page(&viewer),
        1,
        "requested target page is displayed"
    );
    assert!(layer_on(&viewer, 11), "primary node NA ran on arrival");
    assert!(
        !layer_on(&viewer, 12),
        "arrival executes one request, not every node on the page"
    );

    let _ = viewer.handle(Command::GoTo(PageTarget::Previous)).count();
    assert_eq!(current_page(&viewer), 1);
    assert!(
        !layer_on(&viewer, 11),
        "the selected second node's PA ran before Prev"
    );
}

/// Table 165 defines Dur as a maximum and says omission means no automatic advance. Check
/// immediately below and exactly at the stated bound, on the current node, independently of
/// the page's own /Dur (the fixture has none).
#[test]
fn navigation_node_duration_runs_at_its_maximum_without_a_page_duration() {
    let mut viewer = presenting_viewer();
    let _ = viewer.handle(Command::GoTo(PageTarget::Index(2))).count();
    assert_eq!(current_page(&viewer), 2, "arrived at the final page");
    assert!(
        layer_on(&viewer, 11),
        "arrival reached the two-node sequence"
    );
    assert!(!layer_on(&viewer, 12), "Next node's NA has not run yet");

    let _ = viewer.handle(Command::Tick { millis: 1_999 }).count();
    assert!(
        !layer_on(&viewer, 12),
        "one millisecond remains before /Dur 2"
    );
    let _ = viewer.handle(Command::Tick { millis: 1 }).count();
    assert!(
        layer_on(&viewer, 12),
        "at the maximum the current node advances"
    );
    assert_eq!(
        current_page(&viewer),
        2,
        "on the final page there is no page to turn to"
    );
}

fn arriving_transition(dictionary: &str) -> pdf_model::navigation::Transition {
    let mut viewer = viewer_for(fixture_with_transition(&format!(
        "/Trans << {dictionary} >>"
    )));
    viewer
        .handle(Command::GoTo(PageTarget::Index(2)))
        .find_map(|event| match event {
            Event::Transition { transition, .. } => Some(transition),
            _ => None,
        })
        .unwrap_or_else(|| panic!("the arriving page exposes its transition to the host"))
}

#[test]
fn transition_defaults_are_delivered_to_the_host_for_the_styles_that_use_them() {
    use pdf_model::navigation::{Dimension, Direction, Motion, Style};
    let split = arriving_transition("/S /Split");
    assert_eq!(split.style, Style::Split);
    assert!((split.duration - 1.0).abs() < f32::EPSILON);
    assert_eq!(split.dimension, Dimension::Horizontal);
    assert_eq!(split.motion, Motion::Inward);
    let changed = arriving_transition("/S /Split /D 3 /Dm /V /M /O");
    assert!((changed.duration - 3.0).abs() < f32::EPSILON);
    assert_eq!(changed.dimension, Dimension::Vertical);
    assert_eq!(changed.motion, Motion::Outward);
    let wipe = arriving_transition("/S /Wipe");
    assert_eq!(wipe.direction, Direction::Degrees(0.0));
    let reversed = arriving_transition("/S /Wipe /Di 180");
    assert_eq!(reversed.direction, Direction::Degrees(180.0));
    let fly = arriving_transition("/S /Fly");
    assert_eq!(fly.style, Style::Fly);
    assert!((fly.scale - 1.0).abs() < f32::EPSILON);
    assert!(!fly.opaque);
    let scaled = arriving_transition("/S /Fly /SS 0.5 /B true /Di /None");
    assert!((scaled.scale - 0.5).abs() < f32::EPSILON);
    assert!(scaled.opaque);
    assert_eq!(scaled.direction, Direction::None);
}

#[test]
fn an_omitted_transition_style_replaces_immediately_despite_a_stated_duration() {
    for entries in ["", "/D 99", "/S /R /D 99"] {
        let transition = arriving_transition(entries);
        assert_eq!(transition.style, pdf_model::navigation::Style::Replace);
        assert!(
            transition.duration.abs() < f32::EPSILON,
            "R ignores D, including when S defaults: {entries}"
        );
    }
}
