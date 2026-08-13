//! ISO 32000-2 §12.4.4.2's states, walked with no window at all.
//!
//! The clause states a state machine and every sentence of it is a `shall`:
//!
//! > An interactive PDF processor shall maintain a current navigation node. When a user navigates
//! > to a page, if the page dictionary has a PresSteps entry, the node specified by that entry
//! > shall become the current node. (Otherwise, there is no current node.)
//!
//! and then what a forward request does, what a backward one does, and what arriving at a page
//! with a `/PresSteps` does. NOTE 3 conditions the whole of it: "[a]n interactive PDF processor
//! needs to respect navigation nodes only when in presentation mode", which is why every test
//! below either sends [`Command::Present`] or is about what happens when nobody has.
//!
//! **Not one page of the 985 documents this tree opens states a `/PresSteps`, a `/Trans` or a
//! `/Dur`** — `pdf-model/examples/presentation_census`, which walks the page tree rather than the
//! bytes. So the fixture is written here, and the two halves of trap 8 are both in force: a corpus
//! finds what documents contain rather than what the standard says, and a rule the corpus cannot
//! reach is pinned by a **pair** of files differing in one entry.

#![expect(
    clippy::arithmetic_side_effects,
    reason = "test code: the object numbers below are the small integers this file wrote"
)]
#![expect(
    clippy::panic,
    reason = "test code: a fixture that cannot exercise the rule must fail loudly rather than \
              pass by doing nothing"
)]

use std::fmt::Write as _;

use pdf_syntax::ObjectId;
use viewer_core::{
    Answer, Command, DocumentId, Event, Layer, PageTarget, PresentationMode, Query, Viewer,
};

/// The two groups §12.4.4.2's NOTE 1 describes, and the two nodes that turn them on.
///
/// > A single page in a PDF presentation could have a series of bullet points that could be
/// > individually turned on and off.
///
/// and in that example the bullets are optional content, with each state of the page represented
/// as a navigation node.
///
/// Written as the NOTE describes it: `/BaseState /OFF`, so the page opens with neither bullet
/// showing, and one §12.6.4.13 action per node per direction. The second node states Table 165's
/// `/Dur` and the first deliberately does not, because "[i]f this entry is not specified, no
/// automatic advance shall occur" is the other half of that entry and is a difference a clock can
/// see.
///
/// `steps` is the one entry the pair differs in.
fn slides(steps: bool) -> Vec<u8> {
    let pres_steps = if steps { "/PresSteps 6 0 R" } else { "" };
    let objects: [String; 13] = [
        "<< /Type /Catalog /Pages 2 0 R /OCProperties << /OCGs [10 0 R 11 0 R] \
         /D << /BaseState /OFF /Order [10 0 R 11 0 R] >> >> >>"
            .to_owned(),
        "<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 >>".to_owned(),
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 400 400] /Contents 5 0 R {pres_steps} >>"
        ),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 400 400] /Contents 5 0 R \
         /Trans << /Type /Trans /S /Wipe /D 2 /Di 0 >> >>"
            .to_owned(),
        "<< /Length 0 >>\nstream\n\nendstream".to_owned(),
        "<< /Type /NavNode /NA 8 0 R /PA 9 0 R /Next 7 0 R >>".to_owned(),
        "<< /Type /NavNode /NA 12 0 R /PA 13 0 R /Prev 6 0 R /Dur 3 >>".to_owned(),
        "<< /S /SetOCGState /State [/ON 10 0 R] >>".to_owned(),
        "<< /S /SetOCGState /State [/OFF 10 0 R] >>".to_owned(),
        "<< /Type /OCG /Name (bullet one) >>".to_owned(),
        "<< /Type /OCG /Name (bullet two) >>".to_owned(),
        "<< /S /SetOCGState /State [/ON 11 0 R] >>".to_owned(),
        "<< /S /SetOCGState /State [/OFF 11 0 R] >>".to_owned(),
    ];
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for (index, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        let _ = write!(out, "{} 0 obj\n{body}\nendobj\n", index + 1);
    }
    let xref_at = out.len();
    let _ = write!(out, "xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1);
    for offset in &offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n",
        objects.len() + 1
    );
    out.into_bytes()
}

/// A viewer with that document open at page one, presenting or not.
fn opened(steps: bool, presenting: PresentationMode) -> Viewer {
    let mut viewer = Viewer::new(400, 400, 1.0);
    let opened = viewer
        .handle(Command::Open {
            id: DocumentId(1),
            bytes: slides(steps),
            password: None,
            fragment: None,
        })
        .any(|event| matches!(event, Event::Opened { .. }));
    assert!(opened, "the fixture is a valid PDF");
    let _ = viewer.handle(Command::Present(presenting)).count();
    viewer
}

/// Whether one of the two bullets is showing, as a panel would ask.
fn bullet(viewer: &Viewer, number: u32) -> Option<bool> {
    let Answer::Layers(layers) = viewer.query(Query::Layers) else {
        panic!("Query::Layers answers with a list");
    };
    layers.iter().find_map(|layer| match layer {
        Layer::Group { group, on, .. } if *group == ObjectId::new(number, 0) => Some(*on),
        _ => None,
    })
}

/// Which page is showing.
fn page(viewer: &Viewer) -> usize {
    let Answer::Page { index, .. } = viewer.query(Query::CurrentPage) else {
        panic!("a document is open");
    };
    index
}

/// §12.4.4.2's own sequence: two states, then the page.
///
/// > If the user requests to navigate forward (such as an arrow key press) and there is a current
/// > navigation node, the following shall occur:
///
/// > a) The sequence of actions specified by NA (if present) shall be executed.
///
/// > b) The node specified by Next (if present) shall become the new current navigation node.
///
/// Three forward requests against a two-node page: the first two are the nodes and the third is
/// the page, because a list has an end and running off it is the state the clause already names.
#[test]
fn a_forward_request_walks_a_pages_states_before_it_turns_the_page() {
    let mut viewer = opened(true, PresentationMode::On);
    assert_eq!(bullet(&viewer, 10), Some(false), "/BaseState OFF");
    assert_eq!(bullet(&viewer, 11), Some(false));

    let _ = viewer.handle(Command::GoTo(PageTarget::Next)).count();
    assert_eq!(page(&viewer), 0, "the first request is a state, not a page");
    assert_eq!(bullet(&viewer, 10), Some(true), "the first node's /NA ran");
    assert_eq!(bullet(&viewer, 11), Some(false));

    let _ = viewer.handle(Command::GoTo(PageTarget::Next)).count();
    assert_eq!(page(&viewer), 0);
    assert_eq!(bullet(&viewer, 11), Some(true), "the second node's /NA ran");

    let _ = viewer.handle(Command::GoTo(PageTarget::Next)).count();
    assert_eq!(page(&viewer), 1, "and then the page turns");
}

/// The same document without the one entry, which is what makes the test above about the entry.
///
/// Trap 8's pair: everything else in these two files is byte for byte the same, so a viewer that
/// turned the page for some other reason would turn it here too.
#[test]
fn the_same_page_without_pres_steps_turns_at_the_first_request() {
    let mut viewer = opened(false, PresentationMode::On);
    let _ = viewer.handle(Command::GoTo(PageTarget::Next)).count();
    assert_eq!(page(&viewer), 1);
    assert_eq!(bullet(&viewer, 10), Some(false), "nothing performed a step");
}

/// §12.4.4.2's NOTE 3, a permission this program takes: no presentation, no navigation nodes.
///
/// > An interactive PDF processor needs to respect navigation nodes only when in presentation
/// > mode
///
/// A person reading a document that happens to state `/PresSteps` presses an arrow key to turn the
/// page, and the alternative — respecting the nodes always — would spend that key press on
/// somebody else's bullets.
#[test]
fn nothing_walks_a_pages_states_while_no_presentation_is_running() {
    let mut viewer = opened(true, PresentationMode::Off);
    let _ = viewer.handle(Command::GoTo(PageTarget::Next)).count();
    assert_eq!(page(&viewer), 1, "the arrow key turned the page");
    assert_eq!(bullet(&viewer, 10), Some(false), "and changed no state");
}

/// §12.4.4.2's NOTE 2, which is what makes a slide show leave no trace.
///
/// > Interactive PDF processors need to save the state of optional content groups when a user
/// > enters presentation mode and restore it when presentation mode ends. This ensures, for
/// > example, that transient changes to bullets do not affect the printing of the document.
#[test]
fn leaving_presentation_mode_puts_the_optional_content_back() {
    let mut viewer = opened(true, PresentationMode::On);
    let _ = viewer.handle(Command::GoTo(PageTarget::Next)).count();
    assert_eq!(bullet(&viewer, 10), Some(true), "a bullet was turned on");

    let _ = viewer
        .handle(Command::Present(PresentationMode::Off))
        .count();
    assert_eq!(
        bullet(&viewer, 10),
        Some(false),
        "and the document is the one the file describes again"
    );
}

/// A page turned by hand during a presentation performs the page's own `/Trans`.
///
/// §12.4.4.1 makes the entry "the style and duration of the visual transition to use when moving
/// from another page to the given page **during a presentation**" — moving, not moving
/// automatically — and §12.4.4.2's step (c) says it again for exactly this case: "[t]he
/// interactive PDF processor shall make the new page the current page and shall display it. Any
/// page transitions specified by the Trans entry of the page dictionary shall be performed."
///
/// Until the four-hundred-and-eighty-first session only §12.4.4.1's clock produced one, so a
/// person stepping through a slide show by hand saw every effect the file asked for skipped.
#[test]
fn a_page_turned_by_hand_during_a_presentation_plays_its_transition() {
    let mut viewer = opened(false, PresentationMode::On);
    let events: Vec<Event> = viewer.handle(Command::GoTo(PageTarget::Next)).collect();
    let transition = events.iter().find_map(|event| match event {
        Event::Transition { transition, .. } => Some(transition.clone()),
        _ => None,
    });
    let transition = transition.expect("page two states a /Trans");
    assert_eq!(transition.style, pdf_model::navigation::Style::Wipe);

    // And a reader who is not presenting gets none, which is the clause's own condition rather
    // than a policy: there is no presentation to have a transition in.
    let mut reading = opened(false, PresentationMode::Off);
    assert!(
        !reading
            .handle(Command::GoTo(PageTarget::Next))
            .any(|event| matches!(event, Event::Transition { .. })),
        "nothing is presenting"
    );
}

/// Table 165's `/Dur` (§12.4.4.2), which nothing read until the four-hundred-and-eighty-first
/// session.
///
/// > The maximum number of seconds before the interactive PDF processor shall automatically
/// > advance forward to the next navigation node. If this entry is not specified, no automatic
/// > advance shall occur.
///
/// Both halves are here, because the fixture's two nodes differ in exactly this entry: the first
/// states none and sits there however long the clock runs, and the second states three seconds and
/// advances itself.
#[test]
fn a_navigation_nodes_own_duration_advances_it_and_its_absence_does_not() {
    let mut viewer = opened(true, PresentationMode::On);
    for _ in 0..10 {
        let _ = viewer.handle(Command::Tick { millis: 1000 }).count();
    }
    assert_eq!(
        bullet(&viewer, 10),
        Some(false),
        "the first node states no /Dur, so ten seconds performed nothing"
    );

    // One request to reach the second node, which does state one.
    let _ = viewer.handle(Command::GoTo(PageTarget::Next)).count();
    assert_eq!(bullet(&viewer, 11), Some(false), "not yet");
    let _ = viewer.handle(Command::Tick { millis: 2000 }).count();
    assert_eq!(bullet(&viewer, 11), Some(false), "two seconds of three");
    let _ = viewer.handle(Command::Tick { millis: 1500 }).count();
    assert_eq!(
        bullet(&viewer, 11),
        Some(true),
        "three seconds is a maximum, so the node advanced itself"
    );
    assert_eq!(page(&viewer), 0, "advancing a node is not turning a page");
}

/// A backward request performs `/PA` and walks the list the other way.
///
/// §12.4.4.2 again, the other way round: step (c) performs the sequence of actions `/PA` states,
/// if present, and then
///
/// > d) The node specified by Prev (if present) shall become the new current navigation node.
///
/// (Step (c) is prose here rather than a quotation because `doc/md/`'s conversion breaks that
/// sentence across a stray list marker; the standard's own PDF states it whole.)
///
/// And running off the *front* of the list is the same state as running off the back: the request
/// after it turns the page, which is what a person paging backwards out of a slide expects.
#[test]
fn a_backward_request_undoes_a_state_and_then_leaves_the_page() {
    let mut viewer = opened(true, PresentationMode::On);
    let _ = viewer.handle(Command::GoTo(PageTarget::Next)).count();
    assert_eq!(bullet(&viewer, 10), Some(true), "one state forward");

    // The current node is the second one, so this performs *its* `/PA` and steps back to the
    // first — which is the list read the other way and not a second list.
    let _ = viewer.handle(Command::GoTo(PageTarget::Previous)).count();
    assert_eq!(page(&viewer), 0);
    assert_eq!(bullet(&viewer, 10), Some(true), "the second node's /PA ran");

    let _ = viewer.handle(Command::GoTo(PageTarget::Previous)).count();
    assert_eq!(bullet(&viewer, 10), Some(false), "the first node's /PA ran");
    assert_eq!(page(&viewer), 0, "and the page has not moved");

    // Off the front of the list there is no current node, so the request is a page turn again.
    // Page one is the first page, so nothing moves — what this asserts is that the request is no
    // longer being spent on a state.
    let _ = viewer.handle(Command::GoTo(PageTarget::Previous)).count();
    assert_eq!(page(&viewer), 0);
    assert_eq!(bullet(&viewer, 10), Some(false));
}
