//! ISO 32000-2 §12.6.4's actions, as far as they change what a page draws.
//!
//! The unit tests beside `action.rs` and `view.rs` check the *reading* — what Table 217's
//! `/State` array means, how §12.7.4.2's names are built, that a `/Next` cycle terminates.
//! What they cannot check is the claim this session actually makes: that performing an action
//! changes the display list the next render produces. That needs a page with marks on it, an
//! interpretation before and one after, and a comparison of the two.
//!
//! Two actions can do it, and they take different routes to the same answer:
//! §12.6.4.13 through §8.11's optional content, which decides whether a `BDC /OC` span marks
//! the page at all, and §12.6.4.11 through §12.5.3's Hidden flag, which decides whether an
//! annotation is drawn over it.

#![expect(
    clippy::expect_used,
    reason = "test code: a fixture that cannot exercise what the test is about is a failure"
)]

use std::fmt::Write as _;

use pdf_model::view::ViewState;
use pdf_syntax::{Document, Object, ObjectId};

/// Assembles a document from object bodies numbered from 1, with object 1 the catalog.
fn document(objects: &[&str]) -> Document {
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
    Document::open(out.into_bytes()).expect("a valid fixture")
}

/// A stream object body with the right `/Length`.
fn stream(content: &str) -> String {
    format!(
        "<< /Length {} >>\nstream\n{content}\nendstream",
        content.len().saturating_add(1)
    )
}

fn id(number: u32) -> ObjectId {
    ObjectId {
        number,
        generation: 0,
    }
}

/// How many drawing commands a page produces under a given view state.
fn commands(document: &Document, state: &ViewState) -> usize {
    let pages = pdf_model::Pages::new(document);
    let page = pages.get(0).expect("the fixture has one page");
    pdf_model::content::interpret_with(document, &page, state)
        .display_list
        .commands()
        .len()
}

/// §12.6.4.13 performed: a layer switched off stops marking the page.
///
/// The fixture draws two rectangles, one of them inside a `BDC /OC` span whose group the
/// default configuration turns on. Before the action both are drawn; after it, one is.
///
/// This is the assertion that makes `interpret_with` mean something. Every other test in the
/// tree renders a page as the file states it, and this is the first that renders one as a
/// person left it.
#[test]
fn a_set_ocg_state_action_changes_what_the_next_render_draws() {
    let doc = document(&[
        "<< /Type /Catalog /Pages 2 0 R /OCProperties << /OCGs [6 0 R] /D << >> >> >>",
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Contents 4 0 R \
         /Resources << /Properties << /L1 6 0 R >> >> >>",
        &stream("0 0 10 10 re f /OC /L1 BDC 20 20 10 10 re f EMC"),
        "<< /S /SetOCGState /State [/OFF 6 0 R] >>",
        "<< /Type /OCG /Name (layer) >>",
    ]);

    let mut state = ViewState::of(&doc);
    assert_eq!(commands(&doc, &state), 2, "both rectangles, layer on");

    let actions = pdf_model::action::read(&doc, &Object::Reference(id(5)));
    state.perform_all(&doc, &actions);
    assert_eq!(
        commands(&doc, &state),
        1,
        "the layer is off, so its rectangle does not mark the page"
    );

    // And the file itself is untouched: a fresh state draws both again, which is what says
    // this is a *viewer's* state and not a mutation of the document.
    assert_eq!(commands(&doc, &ViewState::of(&doc)), 2);
}

/// A group turned back on draws again, and `Toggle` is what turns it.
#[test]
fn toggle_returns_a_layer_to_where_it_was() {
    let doc = document(&[
        "<< /Type /Catalog /Pages 2 0 R /OCProperties << /OCGs [6 0 R] /D << >> >> >>",
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Contents 4 0 R \
         /Resources << /Properties << /L1 6 0 R >> >> >>",
        &stream("0 0 10 10 re f /OC /L1 BDC 20 20 10 10 re f EMC"),
        "<< /S /SetOCGState /State [/Toggle 6 0 R] >>",
        "<< /Type /OCG /Name (layer) >>",
    ]);
    let mut state = ViewState::of(&doc);
    let toggle = pdf_model::action::read(&doc, &Object::Reference(id(5)));
    state.perform_all(&doc, &toggle);
    assert_eq!(commands(&doc, &state), 1);
    state.perform_all(&doc, &toggle);
    assert_eq!(commands(&doc, &state), 2, "toggled back");
}

/// §12.6.4.11 performed: an annotation named by a hide action is not drawn.
///
/// The annotation is a `Square` with no `/AP`, whose appearance §12.5.6.8 has this tree
/// construct — so it produces commands of its own, and the count is what says whether it was
/// drawn. Its identity is what the action names, which is why the fixture writes it as an
/// indirect object in `/Annots`.
#[test]
fn a_hide_action_stops_an_annotation_being_drawn() {
    let doc = document(&[
        "<< /Type /Catalog /Pages 2 0 R >>",
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Contents 4 0 R \
         /Annots [6 0 R] >>",
        &stream("0 0 10 10 re f"),
        "<< /S /Hide /T 6 0 R >>",
        "<< /Type /Annot /Subtype /Square /Rect [20 20 60 60] /IC [1 0 0] /C [0 0 1] >>",
    ]);

    let mut state = ViewState::of(&doc);
    let with_annotation = commands(&doc, &state);
    assert!(
        with_annotation > 1,
        "the square's constructed appearance draws something: {with_annotation}"
    );

    state.perform_all(
        &doc,
        &pdf_model::action::read(&doc, &Object::Reference(id(5))),
    );
    assert_eq!(
        commands(&doc, &state),
        1,
        "only the page's own rectangle is left"
    );
}

/// Table 214's `/H false` shows an annotation the file itself marks Hidden.
///
/// §12.6.4.11 makes the action "hide or show" by "setting or clearing their Hidden flags", so
/// the flag the file wrote is exactly the flag the action clears — and clearing it is not the
/// same as ignoring §12.5.3, which is why the fixture's annotation stays hidden until the
/// action runs.
#[test]
fn a_show_action_clears_the_flag_the_file_set() {
    let doc = document(&[
        "<< /Type /Catalog /Pages 2 0 R >>",
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Contents 4 0 R \
         /Annots [6 0 R] >>",
        &stream("0 0 10 10 re f"),
        "<< /S /Hide /T 6 0 R /H false >>",
        "<< /Type /Annot /Subtype /Square /Rect [20 20 60 60] /IC [1 0 0] /C [0 0 1] /F 2 >>",
    ]);

    let mut state = ViewState::of(&doc);
    assert_eq!(
        commands(&doc, &state),
        1,
        "Table 167 bit 2 is set, so §12.5.3 hides it"
    );

    state.perform_all(
        &doc,
        &pdf_model::action::read(&doc, &Object::Reference(id(5))),
    );
    assert!(
        commands(&doc, &state) > 1,
        "the action cleared the Hidden flag"
    );
}

/// `interpret` and `interpret_with` at the opening state are the same interpretation.
///
/// The one property that keeps every existing caller — both gates, every other test, the
/// viewer's first frame — meaning what it meant before this session.
#[test]
fn the_opening_state_draws_what_the_file_states() {
    let doc = document(&[
        "<< /Type /Catalog /Pages 2 0 R /OCProperties << /OCGs [5 0 R] /D << /OFF [5 0 R] >> >> >>",
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Contents 4 0 R \
         /Resources << /Properties << /L1 5 0 R >> >> >>",
        &stream("0 0 10 10 re f /OC /L1 BDC 20 20 10 10 re f EMC"),
        "<< /Type /OCG /Name (layer) >>",
    ]);
    let pages = pdf_model::Pages::new(&doc);
    let page = pages.get(0).expect("one page");
    let plain = pdf_model::interpret(&doc, &page);
    let opened = pdf_model::content::interpret_with(&doc, &page, &ViewState::of(&doc));
    assert_eq!(plain.display_list.commands().len(), 1, "the layer is off");
    assert_eq!(
        plain.display_list.commands().len(),
        opened.display_list.commands().len()
    );
    assert_eq!(plain.unsupported, opened.unsupported);
}

/// §12.6.3's trigger events: a press on an annotation performs what Table 197 names.
///
/// The events are the same four §12.5.5's appearances answer to, and this is the other half —
/// what a press *does* rather than what it looks like. The fixture's widget turns a layer off
/// on `/D` and on again on `/X`, so performing the events through `ViewState` changes what the
/// next render draws, which is the only thing worth asserting about an action.
#[test]
fn an_annotations_trigger_events_perform_their_actions() {
    let doc = document(&[
        "<< /Type /Catalog /Pages 2 0 R /OCProperties << /OCGs [6 0 R] /D << >> >> >>",
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Contents 4 0 R /Annots [5 0 R] \
         /Resources << /Properties << /L1 6 0 R >> >> >>",
        &stream("/OC /L1 BDC 20 20 10 10 re f EMC"),
        "<< /Type /Annot /Subtype /Widget /Rect [0 0 20 20] /F 4 \
         /AA << /D 7 0 R /X 8 0 R >> >>",
        "<< /Type /OCG /Name (layer) >>",
        "<< /S /SetOCGState /State [/OFF 6 0 R] >>",
        "<< /S /SetOCGState /State [/ON 6 0 R] >>",
    ]);
    let pages = pdf_model::Pages::new(&doc);
    let page = pages.get(0).expect("one page");
    let annotation = doc
        .get_key(&page.dict, "Annots")
        .as_array()
        .and_then(|entries| entries.first().cloned())
        .expect("one annotation");
    let annotation = doc.resolve(&annotation);
    let annotation = annotation.as_dict().expect("a dictionary");

    let marks = |state: &ViewState| {
        pdf_model::content::interpret_with(&doc, &page, state)
            .display_list
            .commands()
            .len()
    };

    let mut state = ViewState::of(&doc);
    assert_eq!(marks(&state), 1, "the layer opens on");

    let down =
        pdf_model::action::for_annotation(&doc, annotation, pdf_model::action::Trigger::Down);
    assert_eq!(down.len(), 1, "one action for `/D`");
    state.perform_all(&doc, &down);
    assert_eq!(marks(&state), 0, "pressing it switched the layer off");

    let exit =
        pdf_model::action::for_annotation(&doc, annotation, pdf_model::action::Trigger::Exit);
    state.perform_all(&doc, &exit);
    assert_eq!(
        marks(&state),
        1,
        "and leaving it switched the layer back on"
    );

    assert!(
        pdf_model::action::for_annotation(&doc, annotation, pdf_model::action::Trigger::Focus)
            .is_empty(),
        "an event the file states nothing for performs nothing"
    );
}

/// §12.6.3's one precedence rule: Table 197's `/A` beats `/AA /U`.
///
/// > For backward compatibility, the A entry in an annotation dictionary, if present, takes
/// > precedence over this entry
///
/// So an annotation stating both performs its `/A` on mouse-up — which is what a click on a
/// link already follows — and never its `/AA /U`. Every other event is unaffected, which is
/// the half a looser reading would break.
#[test]
fn an_annotations_own_action_takes_precedence_over_its_mouse_up_trigger() {
    let doc = document(&[
        "<< /Type /Catalog /Pages 2 0 R >>",
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Annots [4 0 R] >>",
        "<< /Type /Annot /Subtype /Link /Rect [0 0 20 20] /A 5 0 R \
         /AA << /U 6 0 R /D 6 0 R >> >>",
        "<< /S /GoTo /D [3 0 R /Fit] >>",
        "<< /S /SetOCGState /State [/OFF 7 0 R] >>",
        "<< /Type /OCG /Name (layer) >>",
    ]);
    let pages = pdf_model::Pages::new(&doc);
    let page = pages.get(0).expect("one page");
    let entry = doc
        .get_key(&page.dict, "Annots")
        .as_array()
        .and_then(|entries| entries.first().cloned())
        .expect("one annotation");
    let resolved = doc.resolve(&entry);
    let annotation = resolved.as_dict().expect("a dictionary");

    let performed = |event| pdf_model::action::for_annotation(&doc, annotation, event);
    assert!(
        matches!(
            performed(pdf_model::action::Trigger::Up).as_slice(),
            [pdf_model::action::Action::GoTo(_)]
        ),
        "mouse-up performs /A: {:?}",
        performed(pdf_model::action::Trigger::Up)
    );
    assert!(
        matches!(
            performed(pdf_model::action::Trigger::Down).as_slice(),
            [pdf_model::action::Action::SetOcgState(_)]
        ),
        "and every other event still reads its own entry"
    );
}

/// §12.6.4.12's named actions reach the viewer as a request rather than a state change.
///
/// The whole of what a `ViewState` can do with `NextPage` is *say so*: which page is on
/// screen is the window's, and the clause's four commands are stated relative to it. So the
/// assertion is about the boundary — a document's `/Named` action becomes a
/// [`pdf_model::view::Request::Page`], and `Named::page_from` turns it into an index against
/// the page count the caller holds.
#[test]
fn a_named_action_asks_the_viewer_to_turn_the_page() {
    let doc = document(&[
        "<< /Type /Catalog /Pages 2 0 R >>",
        "<< /Type /Pages /Count 2 /Kids [3 0 R 4 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] >>",
        "<< /S /Named /N /LastPage >>",
    ]);
    let mut state = ViewState::of(&doc);
    let before = state.clone();
    let actions = pdf_model::action::read(&doc, &Object::Reference(id(5)));
    let requests = state.perform_all(&doc, &actions);
    assert_eq!(
        state, before,
        "a page command changes nothing in the document"
    );

    let [pdf_model::view::Request::Page(named)] = requests.as_slice() else {
        panic!("one page request, got {requests:?}");
    };
    assert_eq!(
        named.page_from(0, pdf_model::Pages::new(&doc).len()),
        Some(1)
    );
}

/// §12.6.4.8's URI reaches the viewer resolved, and nothing here opens it.
///
/// The document is `issue14802.pdf`'s shape: a catalog `/URI` dictionary with Table 211's
/// `/Base`, and an action whose `/URI` is a partial reference. What the gate pins is that the
/// two are combined *before* the request leaves this crate — a caller that received the
/// document's own string would have to implement RFC 3986 to use it.
#[test]
fn a_uri_action_arrives_at_the_viewer_already_resolved() {
    let doc = document(&[
        "<< /Type /Catalog /Pages 2 0 R /URI << /Base (http://example.com/docs/a.pdf) >> >>",
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] >>",
        "<< /S /URI /URI (../other/b.html#top) >>",
    ]);
    let mut state = ViewState::of(&doc);
    let requests = state.perform_all(
        &doc,
        &pdf_model::action::read(&doc, &Object::Reference(id(4))),
    );
    let [pdf_model::view::Request::Resolve(uri)] = requests.as_slice() else {
        panic!("one URI request, got {requests:?}");
    };
    assert_eq!(uri.uri, "http://example.com/other/b.html#top");
    assert!(!uri.relative);
}
