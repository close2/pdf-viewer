//! The window's history of views: a jump leaves the view it was taken from behind, a step back
//! restores it exactly, and a step forward retraces the step back (ADR 2805).
//!
//! The standard keeps no history of views, so what is pinned here is the documented choice and
//! the clauses it rests on: §12.3.2.1's three items are what a view is — "\[t]he page of the
//! document that shall be displayed", "\[t]he location of the document window on that page" and
//! "\[t]he magnification (zoom) factor" — and §12.3.3's outline item, "causing the interactive PDF
//! processor to jump to a destination", is the jump.

#![expect(
    clippy::panic,
    reason = "test code: a viewer that answers no view must fail loudly rather than pass by doing \
              nothing"
)]

use std::fmt::Write as _;
use std::sync::Arc;

use pdf_model::view::{
    ScriptEdit, ScriptEvent, ScriptResult, ScriptRunner, ScriptSite, ViewChange,
};
use pdf_syntax::ObjectId;
use viewer_core::{
    Answer, Command, DocumentId, Event, PageTarget, Query, Retrace, ScriptRunners, Scripting,
    Viewer, Viewing, Zoom,
};

const DOCUMENT: DocumentId = DocumentId(1);

/// The outline item whose `/Dest` names the third page, with Table 149's `/XYZ` and three nulls —
/// "the current value of that parameter shall be retained unchanged" — so the jump changes the
/// page and nothing else.
const ITEM: ObjectId = ObjectId::new(7, 0);

/// Three pages 400 units square, an outline of one item that jumps to the third, and an open
/// action whose script the runner below answers — run only once a host says it presented.
fn three_pages() -> Vec<u8> {
    let bodies = [
        "<< /Type /Catalog /Pages 2 0 R /Outlines 6 0 R \
         /OpenAction << /S /JavaScript /JS (go\\(\\);) >> >>",
        "<< /Type /Pages /Kids [3 0 R 4 0 R 5 0 R] /Count 3 /MediaBox [0 0 400 400] >>",
        "<< /Type /Page /Parent 2 0 R >>",
        "<< /Type /Page /Parent 2 0 R >>",
        "<< /Type /Page /Parent 2 0 R >>",
        "<< /Type /Outlines /First 7 0 R /Last 7 0 R /Count 1 >>",
        "<< /Title (Three) /Parent 6 0 R /Dest [5 0 R /XYZ null null null] >>",
    ];
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for (index, body) in bodies.iter().enumerate() {
        offsets.push(out.len());
        let _ = write!(out, "{} 0 obj\n{body}\nendobj\n", index.saturating_add(1));
    }
    let xref_at = out.len();
    let size = bodies.len().saturating_add(1);
    let _ = write!(out, "xref\n0 {size}\n0000000000 65535 f \n");
    for offset in &offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

/// A viewer 800 by 1000 device pixels with the document open, magnified four times and scrolled
/// down, so that the view a jump leaves has a scroll a restore could get wrong.
fn opened(scripting: Option<Scripting>) -> Viewer {
    let mut viewer = Viewer::new(800, 1000, 1.0);
    if let Some(scripting) = scripting {
        viewer.handle(Command::Scripts(scripting)).for_each(drop);
    }
    viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: three_pages().into(),
            password: None,
            fragment: None,
        })
        .for_each(drop);
    viewer
        .handle(Command::Zoom {
            zoom: Zoom::Scale(4.0),
            at: None,
        })
        .for_each(drop);
    viewer
        .handle(Command::Scroll { dx: 0.0, dy: 300.0 })
        .for_each(drop);
    viewer
}

/// Where the viewer has put the reader.
fn view(viewer: &Viewer) -> Viewing {
    match viewer.query(Query::View) {
        Answer::View(viewing) => viewing,
        other => panic!("the document is open and has a view: {other:?}"),
    }
}

/// Every sentence the command made the viewer say.
fn said(viewer: &mut Viewer, command: Command) -> Vec<String> {
    viewer
        .handle(command)
        .filter_map(|event| match event {
            Event::Reported { notes, .. } => Some(notes),
            _ => None,
        })
        .flatten()
        .collect()
}

/// §12.3.3's outline item jumps to the third page; going back restores the first page at the
/// magnification and the scroll the reader left it at — all three of §12.3.2.1's items, exactly —
/// and going forward returns to the third.
#[test]
fn a_destination_followed_leaves_the_view_behind_and_back_and_forward_walk_it() {
    let mut viewer = opened(None);
    let left = view(&viewer);
    assert_eq!(left.page, 0);
    assert!(left.scroll.1 > 0.0, "the page is scrolled: {left:?}");
    viewer.handle(Command::Activate(ITEM)).for_each(drop);
    let reached = view(&viewer);
    assert_eq!(
        reached.page, 2,
        "the outline item's /Dest names the third page"
    );

    let notes = said(&mut viewer, Command::History(Retrace::Back));
    assert!(notes.is_empty(), "{notes:?}");
    assert_eq!(view(&viewer), left);

    viewer
        .handle(Command::History(Retrace::Forward))
        .for_each(drop);
    assert_eq!(view(&viewer), reached);
}

/// A turn to the next page is a step and leaves nothing behind, so there is no earlier view, and
/// the viewer says so rather than moving.
#[test]
fn a_step_is_not_a_jump_and_a_step_back_with_nowhere_to_go_is_said() {
    let mut viewer = opened(None);
    viewer
        .handle(Command::GoTo(PageTarget::Next))
        .for_each(drop);
    let stepped = view(&viewer);
    assert_eq!(stepped.page, 1);
    let notes = said(&mut viewer, Command::History(Retrace::Back));
    assert!(
        notes.iter().any(|note| note.contains("no earlier view")),
        "{notes:?}"
    );
    assert_eq!(view(&viewer), stepped);
}

/// A page named outright is a jump; a new jump after a step back forgets the view that lay ahead.
#[test]
fn a_page_named_outright_is_a_jump_and_a_new_jump_forgets_what_lay_ahead() {
    let mut viewer = opened(None);
    viewer
        .handle(Command::GoTo(PageTarget::Last))
        .for_each(drop);
    viewer
        .handle(Command::History(Retrace::Back))
        .for_each(drop);
    assert_eq!(view(&viewer).page, 0);
    viewer
        .handle(Command::GoTo(PageTarget::Index(1)))
        .for_each(drop);
    let notes = said(&mut viewer, Command::History(Retrace::Forward));
    assert!(
        notes.iter().any(|note| note.contains("no later view")),
        "{notes:?}"
    );
    assert_eq!(view(&viewer).page, 1);
    viewer
        .handle(Command::History(Retrace::Back))
        .for_each(drop);
    assert_eq!(view(&viewer).page, 0);
}

/// A runner that answers the open action with a named destination's jump and then
/// `app.goBack()`, as the realm records them.
#[derive(Debug)]
struct GoBack;

impl ScriptRunner for GoBack {
    fn run(&self, event: &ScriptEvent<'_>) -> ScriptResult {
        let edits = if event.site == ScriptSite::OpenAction {
            vec![
                ScriptEdit::View {
                    change: ViewChange::Destination {
                        page: 2,
                        view: pdf_model::destination::View::Fit,
                    },
                },
                ScriptEdit::View {
                    change: ViewChange::History(-1),
                },
            ]
        } else {
            Vec::new()
        };
        ScriptResult {
            rc: true,
            value: None,
            change: None,
            edits,
            report: Vec::new(),
        }
    }
}

impl ScriptRunners for GoBack {
    fn runner(&self) -> Arc<dyn ScriptRunner> {
        Arc::new(GoBack)
    }
}

/// A script's `app.goBack` is the step a person takes: after its own `gotoNamedDest` jumped to the
/// third page, it is back on the first at the view it left, and a person's step forward retraces
/// it.
#[test]
fn a_script_s_go_back_walks_the_history_a_person_walks() {
    let mut viewer = opened(Some(Scripting::Run(Arc::new(GoBack))));
    let left = view(&viewer);
    viewer.handle(Command::Presented).for_each(drop);
    assert_eq!(view(&viewer), left);
    viewer
        .handle(Command::History(Retrace::Forward))
        .for_each(drop);
    assert_eq!(view(&viewer).page, 2);
}
