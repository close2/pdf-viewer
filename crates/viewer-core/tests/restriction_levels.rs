//! `CLAUDE.md`'s four restriction levels, one per operation, against a document that states a `/P`.
//!
//! The reading is `pdf-model`'s and is tested there as arithmetic over §7.6.4.2's Table 22; this is
//! the other end — the *policy*, which `CLAUDE.md` principle 3 makes the reader's. That principle
//! calls a document's restrictions low priority, requires that turning them off shall always be
//! possible, and gives the reason: a restriction a reader cannot switch off is one somebody else's
//! file imposed on them, and this program is the reader's.
//!
//! What is asserted here is that each of the four levels does its own thing, that a level set for
//! one operation says nothing about another, and that a document which restricts nothing is never
//! the subject of a question at any level. ADR 1144.

#![expect(
    clippy::panic,
    clippy::expect_used,
    reason = "test code: a fixture that cannot exercise the rule must fail loudly rather than \
              pass by doing nothing"
)]

use std::path::{Path, PathBuf};

use pdf_model::restriction::Operation;
use pdf_model::view::Markup;
use pdf_render::Rasterizer;
use render_cpu::CpuRasterizer;
use viewer_core::{
    Answer, Command, DocumentId, Edit, Event, Query, Rendered, RestrictionLevel,
    RestrictionOverride, RestrictionPolicy, RestrictionScope, Selection, Viewer,
};

/// The document every test here opens.
const DOCUMENT: DocumentId = DocumentId(1);

/// A corpus document's bytes, or `None` when the submodule is not checked out.
fn corpus_bytes(name: &str) -> Option<Vec<u8>> {
    let path: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../doc/pdf.js/test/pdfs")
        .join(name);
    std::fs::read(path).ok()
}

/// The PDF Association's note, which is committed and states no `/Encrypt` at all.
fn unrestricted_bytes() -> Vec<u8> {
    let path: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc/PDF20_AN001-BPC.pdf");
    std::fs::read(&path).unwrap_or_else(|error| panic!("{} is committed: {error}", path.display()))
}

/// Opens a document under one policy, draws page one and selects all of its text.
///
/// The selection is what a copy is *of*, so it is part of the fixture rather than part of a test:
/// `Command::Copy` on a page with nothing selected sends nothing at all, which is its own
/// assertion below.
fn reading(bytes: Vec<u8>, policy: RestrictionPolicy) -> Viewer {
    let mut viewer = Viewer::new(800, 1000, 1.0);
    viewer
        .handle(Command::Restrict(RestrictionScope::Window(policy)))
        .for_each(drop);
    let events: Vec<Event> = viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: bytes.into(),
            password: None,
            fragment: None,
        })
        .collect();
    let request = events
        .iter()
        .find_map(|event| match event {
            Event::NeedsRender(request) => Some(request.clone()),
            _ => None,
        })
        .expect("a render was asked for");
    let raster = CpuRasterizer::new()
        .rasterize(&request.list, request.target)
        .expect("the CPU backend draws this page");
    viewer
        .handle(Command::RenderReady {
            token: request.token,
            rendered: Rendered::Raster(raster),
        })
        .for_each(drop);
    viewer
        .handle(Command::Select(Selection::All))
        .for_each(drop);
    viewer
}

/// The same viewer over `bug1815476.pdf`, whose flag word is the fixture this file rests on.
///
/// `/R 4` and `/P −1084`, which is `0xFFFFFBC4`: **bit 5 clear**, so §7.6.4.2's Table 22 withholds
/// "[c]opy or otherwise extract text and graphics from the document"; **bit 6 clear**, so it
/// withholds "[a]dd or modify text annotations"; and **bit 9 set**, which at revision 3 or greater
/// grants "[f]ill in existing interactive form fields (including signature fields), even if bit 6
/// is clear". One document, two operations withheld and a third granted, which is what makes it
/// evidence that the three are read apart rather than together.
fn restricted(policy: RestrictionPolicy) -> Option<Viewer> {
    Some(reading(corpus_bytes("bug1815476.pdf")?, policy))
}

/// Everything one command produced.
fn sent(viewer: &mut Viewer, command: Command) -> Vec<Event> {
    viewer.handle(command).collect()
}

/// The text of the one [`Event::Copied`] in a list, or `None` where there is none.
fn copied(events: &[Event]) -> Option<String> {
    events.iter().find_map(|event| match event {
        Event::Copied {
            logical,
            page_order,
            ..
        } => Some(logical.clone().unwrap_or_else(|| page_order.clone())),
        _ => None,
    })
}

/// Whether a list holds an event of this shape.
fn holds(events: &[Event], shape: fn(&Event) -> bool) -> bool {
    events.iter().any(shape)
}

/// All four levels over §7.6.4.2 bit 5, on a document whose `/P` clears it.
///
/// The calibration `CLAUDE.md` principle 3 asks for, one level at a time: `Off` copies, `On`
/// refuses by name, `Ask` holds the text until somebody answers, `Warn` copies and then says what
/// the document said.
#[test]
fn a_copy_obeys_the_level_the_reader_set_for_copying() {
    let Some(mut viewer) = restricted(RestrictionPolicy::default()) else {
        eprintln!("skipped: doc/pdf.js is not checked out");
        return;
    };
    let events = sent(&mut viewer, Command::Copy);
    let text = copied(&events).expect("`Off` is the default and copies");
    assert!(
        text.contains("ANEXO") || text.contains("A N E X O"),
        "the page's own characters: {:?}",
        &text[..40.min(text.len())]
    );
    assert!(
        !holds(&events, |event| matches!(
            event,
            Event::Refused { .. } | Event::Asking { .. } | Event::Warned { .. }
        )),
        "a reader at `Off` asked to be told nothing"
    );

    // `On`: §7.6.4.1's `shall` kept, with the bit named for a person.
    let mut viewer = restricted(RestrictionPolicy::uniform(RestrictionLevel::On))
        .expect("the submodule is checked out");
    let events = sent(&mut viewer, Command::Copy);
    assert!(copied(&events).is_none(), "nothing left the document");
    let Some(Event::Refused {
        operation, notes, ..
    }) = events
        .iter()
        .find(|event| matches!(event, Event::Refused { .. }))
    else {
        panic!("the copy is refused by name: {events:?}");
    };
    assert_eq!(*operation, Operation::Extract);
    assert!(
        notes.iter().any(|note| note.contains("7.6.4.2")),
        "the clause is named: {notes:?}"
    );

    // `Ask`: the text is held, and a `no` forgets it without a word.
    let mut viewer = restricted(RestrictionPolicy::uniform(RestrictionLevel::Ask))
        .expect("the submodule is checked out");
    let events = sent(&mut viewer, Command::Copy);
    assert!(copied(&events).is_none(), "nothing until it is answered");
    assert!(
        holds(&events, |event| matches!(
            event,
            Event::Asking {
                operation: Operation::Extract,
                ..
            }
        )),
        "the question is asked: {events:?}"
    );
    let declined = sent(
        &mut viewer,
        Command::Answer {
            document: DOCUMENT,
            proceed: false,
        },
    );
    assert!(declined.is_empty(), "a question declined says nothing");
    // And the same question answered `yes` copies exactly what it was asked about.
    let mut viewer = restricted(RestrictionPolicy::uniform(RestrictionLevel::Ask))
        .expect("the submodule is checked out");
    sent(&mut viewer, Command::Copy);
    let granted = sent(
        &mut viewer,
        Command::Answer {
            document: DOCUMENT,
            proceed: true,
        },
    );
    assert_eq!(
        copied(&granted),
        Some(text.clone()),
        "what goes ahead on a yes is what was asked about"
    );

    // `Warn`: the copy, and then the sentence about it.
    let mut viewer = restricted(RestrictionPolicy::uniform(RestrictionLevel::Warn))
        .expect("the submodule is checked out");
    let events = sent(&mut viewer, Command::Copy);
    assert_eq!(copied(&events), Some(text));
    let copied_at = events
        .iter()
        .position(|event| matches!(event, Event::Copied { .. }))
        .expect("the copy happened");
    let warned_at = events
        .iter()
        .position(|event| matches!(event, Event::Warned { .. }))
        .expect("and was warned about");
    assert!(copied_at < warned_at, "the state first, then the sentence");
}

/// A level set for one operation says nothing whatever about another.
///
/// The whole reason the policy is six levels rather than one: this document withholds copying
/// *and* annotating, and a reader who asked to be stopped from copying did not ask to be stopped
/// from marking a page up.
#[test]
fn one_operations_level_says_nothing_about_another() {
    let policy = RestrictionPolicy::default().with(Operation::Extract, RestrictionLevel::On);
    let Some(mut viewer) = restricted(policy) else {
        eprintln!("skipped: doc/pdf.js is not checked out");
        return;
    };
    let events = sent(&mut viewer, Command::Copy);
    assert!(copied(&events).is_none(), "copying is the level set to on");

    let events = sent(
        &mut viewer,
        Command::Edit(Edit::Markup {
            kind: Markup::Highlight,
            colour: [1.0, 1.0, 0.0],
        }),
    );
    assert!(
        !holds(&events, |event| matches!(event, Event::Refused { .. })),
        "and annotating, which this /P also withholds, was left off: {events:?}"
    );
    assert!(
        matches!(viewer.query(Query::Dirty), Answer::Dirty(true)),
        "the highlight is in the edit log"
    );

    // And the other way round, which is the half a one-level policy could never express.
    let policy = RestrictionPolicy::default().with(Operation::Annotate, RestrictionLevel::On);
    let mut viewer = restricted(policy).expect("the submodule is checked out");
    let events = sent(
        &mut viewer,
        Command::Edit(Edit::Markup {
            kind: Markup::Highlight,
            colour: [1.0, 1.0, 0.0],
        }),
    );
    assert!(
        holds(&events, |event| matches!(
            event,
            Event::Refused {
                operation: Operation::Annotate,
                ..
            }
        )),
        "annotating is the level set to on: {events:?}"
    );
    assert!(
        copied(&sent(&mut viewer, Command::Copy)).is_some(),
        "and copying was left off"
    );
}

/// A document that restricts nothing is never asked about, at any of the four levels.
///
/// `pdf_model::restriction::Level::verdict` answers `Proceed` for an empty list whatever the level
/// is, and this is that rule where a person would notice it: turning every restriction on does not
/// turn a plain document into one that argues back.
#[test]
fn a_document_that_restricts_nothing_proceeds_silently_at_every_level() {
    for level in [
        RestrictionLevel::Off,
        RestrictionLevel::On,
        RestrictionLevel::Ask,
        RestrictionLevel::Warn,
    ] {
        let mut viewer = reading(unrestricted_bytes(), RestrictionPolicy::uniform(level));
        let events = sent(&mut viewer, Command::Copy);
        assert!(
            copied(&events).is_some(),
            "{level:?}: the copy goes ahead, because nothing withholds it"
        );
        assert!(
            !holds(&events, |event| matches!(
                event,
                Event::Refused { .. } | Event::Asking { .. } | Event::Warned { .. }
            )),
            "{level:?}: and nothing is said about it: {events:?}"
        );

        let events = sent(
            &mut viewer,
            Command::Edit(Edit::Markup {
                kind: Markup::Highlight,
                colour: [1.0, 1.0, 0.0],
            }),
        );
        assert!(
            !holds(&events, |event| matches!(
                event,
                Event::Refused { .. } | Event::Asking { .. } | Event::Warned { .. }
            )),
            "{level:?}: nor about an annotation: {events:?}"
        );
    }
}

/// Nothing selected is nothing copied, and nothing said — at every level.
///
/// A question about an empty copy would be a question about nothing, and a `QUORRA_EVENT_COPIED`
/// carrying no characters would be a lie about the selection (`doc/todo/38`).
#[test]
fn a_copy_of_nothing_asks_nobody_anything() {
    for level in [RestrictionLevel::On, RestrictionLevel::Ask] {
        let mut viewer = reading(unrestricted_bytes(), RestrictionPolicy::uniform(level));
        viewer
            .handle(Command::Select(Selection::None))
            .for_each(drop);
        assert!(
            sent(&mut viewer, Command::Copy).is_empty(),
            "{level:?}: nothing is selected, so there is nothing to ask about"
        );
    }
}

/// A document may depart from the window's levels, and the departure ends with the document.
///
/// **The calibration ADR 1145 rests on.** The window is at `On` — §7.6.4.1's `shall` kept — and the
/// document in front of the reader is moved to `Ask`, which is the sentence a viewer-wide policy
/// could not say: *for this document, ask before copying*. Then the same window opens the document
/// again and the copy is refused, because a departure is about one document rather than about the
/// reader.
#[test]
fn a_document_may_depart_from_the_windows_levels_until_it_closes() {
    let Some(mut viewer) = restricted(RestrictionPolicy::uniform(RestrictionLevel::On)) else {
        eprintln!("skipped: doc/pdf.js is not checked out");
        return;
    };
    // The window's level, unchanged, is what the document inherited.
    assert!(
        holds(&sent(&mut viewer, Command::Copy), |event| matches!(
            event,
            Event::Refused { .. }
        )),
        "the window is at `On`, so the document is"
    );

    viewer
        .handle(Command::Restrict(RestrictionScope::Document(
            RestrictionOverride::NONE.with(Operation::Extract, Some(RestrictionLevel::Ask)),
        )))
        .for_each(drop);
    let events = sent(&mut viewer, Command::Copy);
    assert!(copied(&events).is_none(), "nothing has been answered yet");
    assert!(
        holds(&events, |event| matches!(event, Event::Asking { .. })),
        "the departure is `Ask`: {events:?}"
    );
    let answered = sent(
        &mut viewer,
        Command::Answer {
            document: DOCUMENT,
            proceed: true,
        },
    );
    assert!(
        copied(&answered).is_some(),
        "a `yes` performs what was held: {answered:?}"
    );

    // One operation's departure says nothing about another's: annotating is bit 6, which this
    // document also withholds, and the window's `On` is what still decides it.
    let events = sent(
        &mut viewer,
        Command::Edit(Edit::Markup {
            kind: Markup::Highlight,
            colour: [1.0, 1.0, 0.0],
        }),
    );
    assert!(
        holds(&events, |event| matches!(event, Event::Refused { .. })),
        "annotating was not departed from: {events:?}"
    );

    // And the next document opened in this window is back at the window's levels, because the
    // departure lived beside the document rather than beside the reader.
    let bytes = corpus_bytes("bug1815476.pdf").expect("the submodule is checked out");
    viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: bytes.into(),
            password: None,
            fragment: None,
        })
        .for_each(drop);
    viewer
        .handle(Command::Select(Selection::All))
        .for_each(drop);
    let events = sent(&mut viewer, Command::Copy);
    assert!(
        holds(&events, |event| matches!(event, Event::Refused { .. })),
        "a second document inherits the window's `On`: {events:?}"
    );
}

/// A departure the document does not make is the window's level exactly.
///
/// The other half of the layering, and the one a menu's *use the window's level* entry sends: an
/// override of nothing changes nothing, and an override taken away puts the window's level back.
#[test]
fn an_override_of_nothing_is_the_windows_own_policy() {
    let Some(mut viewer) = restricted(RestrictionPolicy::default()) else {
        eprintln!("skipped: doc/pdf.js is not checked out");
        return;
    };
    viewer
        .handle(Command::Restrict(RestrictionScope::Document(
            RestrictionOverride::NONE,
        )))
        .for_each(drop);
    assert!(
        copied(&sent(&mut viewer, Command::Copy)).is_some(),
        "the window is at `Off`, and the document departs from it in nothing"
    );

    viewer
        .handle(Command::Restrict(RestrictionScope::Document(
            RestrictionOverride::NONE.with(Operation::Extract, Some(RestrictionLevel::On)),
        )))
        .for_each(drop);
    assert!(
        copied(&sent(&mut viewer, Command::Copy)).is_none(),
        "the departure is `On`"
    );

    viewer
        .handle(Command::Restrict(RestrictionScope::Document(
            RestrictionOverride::NONE.with(Operation::Extract, None),
        )))
        .for_each(drop);
    assert!(
        copied(&sent(&mut viewer, Command::Copy)).is_some(),
        "the departure was taken away, so the window's `Off` decides again"
    );
}

/// §12.11.6's processing, at each of the four levels a reader can set for it.
///
/// > If requirements cannot be met, as determined by the computation of the penalty value as
/// > described in 12.11.3, "Requirement penalty values", then the processing of the document
/// > shall not continue.
///
/// The one restriction here that is not a verb a person presses: what it withholds is the
/// document. **0 of the corpus documents state a `/Requirements` array**, so the witness is built
/// — two requirements this program cannot meet, priced at 60 and 55, whose total of 115 is over
/// the 100 §12.11.3's last paragraph states.
///
/// The claim is one per level: `Off` opens the document as though the clause said nothing, which
/// is the default `CLAUDE.md` requires; `On` raises no `Event::Opened` at all, which is what "the
/// processing of the document shall not continue" is; `Ask` opens nothing until the answer comes
/// and then opens everything; `Warn` opens it and says so afterwards. ADR 1167.
#[test]
fn a_document_over_the_penalty_threshold_is_processed_at_the_level_the_reader_set() {
    // Two requirements `Kind::unmet` answers — ECMAScript is excluded by `CLAUDE.md` principle 5
    // and `Markup`'s modification and deletion are not built — and one it meets, priced at 100 to
    // show that a *met* requirement costs nothing: Table 273 prices "the penalty value to be
    // applied when this requirement cannot be met by a PDF processor".
    let bytes: Vec<u8> = b"%PDF-2.0\n\
         1 0 obj\n<< /Type /Catalog /Pages 2 0 R /Requirements [\
         << /S /EnableJavaScripts /Penalty 60 >> << /S /Markup /Penalty 55 >> \
         << /S /Navigation /Penalty 100 >>] >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] >>\nendobj\n\
         trailer\n<< /Root 1 0 R /Size 4 >>\n"
        .to_vec();
    let opening = |level: RestrictionLevel| -> Vec<Event> {
        let mut viewer = Viewer::new(800, 1000, 1.0);
        viewer
            .handle(Command::Restrict(RestrictionScope::Window(
                RestrictionPolicy::default().with(Operation::Process, level),
            )))
            .for_each(drop);
        viewer
            .handle(Command::Open {
                id: DOCUMENT,
                bytes: bytes.clone().into(),
                password: None,
                fragment: None,
            })
            .collect()
    };
    let opened = |events: &[Event]| holds(events, |event| matches!(event, Event::Opened { .. }));
    let about = |events: &[Event], operation: Operation| -> Vec<String> {
        events
            .iter()
            .filter_map(|event| match event {
                Event::Refused {
                    operation: asked,
                    notes,
                    ..
                }
                | Event::Asking {
                    operation: asked,
                    notes,
                    ..
                }
                | Event::Warned {
                    operation: asked,
                    notes,
                    ..
                } if *asked == operation => Some(notes.clone()),
                _ => None,
            })
            .flatten()
            .collect()
    };

    // `Off`: the default, and the document opens with nothing said about the threshold here —
    // what it could not promise is `Command::Report`'s answer either way.
    let off = opening(RestrictionLevel::Off);
    assert!(opened(&off), "the default level opens the document");
    assert!(about(&off, Operation::Process).is_empty(), "{off:?}");

    // `On`: "the processing of the document shall not continue". No `Event::Opened`, and the
    // reason names the clause's own number.
    let on = opening(RestrictionLevel::On);
    assert!(!opened(&on), "nothing was processed: {on:?}");
    let refused = about(&on, Operation::Process);
    assert!(
        refused
            .iter()
            .any(|note| note.contains("115 penalty points") && note.contains("§12.11.3")),
        "the total the clause's computation came to is what a person is told: {refused:?}"
    );

    // `Ask`: nothing is processed while the question stands, and the `yes` releases all of it.
    let asking = opening(RestrictionLevel::Ask);
    assert!(!opened(&asking), "a question is not an open: {asking:?}");
    assert!(
        holds(&asking, |event| matches!(event, Event::Asking { .. })),
        "{asking:?}"
    );
    assert!(!about(&asking, Operation::Process).is_empty());

    // `Warn`: the document opens, and the reason is said after it.
    let warned = opening(RestrictionLevel::Warn);
    assert!(opened(&warned), "warning is not refusing: {warned:?}");
    assert!(
        holds(&warned, |event| matches!(event, Event::Warned { .. })),
        "{warned:?}"
    );

    // A document that states no requirements at all is nobody's question, at every level —
    // §12.11.3's threshold is on a total, and an empty one is zero.
    for level in [
        RestrictionLevel::Off,
        RestrictionLevel::On,
        RestrictionLevel::Ask,
        RestrictionLevel::Warn,
    ] {
        let mut viewer = Viewer::new(800, 1000, 1.0);
        viewer
            .handle(Command::Restrict(RestrictionScope::Window(
                RestrictionPolicy::uniform(level),
            )))
            .for_each(drop);
        let events: Vec<Event> = viewer
            .handle(Command::Open {
                id: DOCUMENT,
                bytes: unrestricted_bytes().into(),
                password: None,
                fragment: None,
            })
            .collect();
        assert!(
            opened(&events),
            "a document stating no /Requirements opens at {level:?}: {events:?}"
        );
    }
}

/// The `yes` and the `no` to §12.11.6's question, which are not an edit's.
///
/// An edit held at `Ask` leaves a document that is open either way; this one holds the *open*, so
/// a `yes` has to release everything opening a document does and a `no` has to leave the viewer
/// holding nothing — a document nobody can see, close or query would be this crate keeping a
/// half-processed file (ADR 1167).
#[test]
fn answering_the_processing_question_opens_the_document_or_forgets_it() {
    // Two unmet requirements rather than one over-priced one: Table 273 bounds a single
    // `/Penalty` at "between 0 and 100 (inclusive)", which is why §12.11.3's threshold is on the
    // *total* — a limit on one entry could never fire.
    let bytes: Vec<u8> = b"%PDF-2.0\n\
         1 0 obj\n<< /Type /Catalog /Pages 2 0 R /Requirements [\
         << /S /EnableJavaScripts /Penalty 60 >> << /S /Markup /Penalty 55 >>] >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] >>\nendobj\n\
         trailer\n<< /Root 1 0 R /Size 4 >>\n"
        .to_vec();
    let asked = |proceed: bool| -> (Vec<Event>, Option<usize>) {
        let mut viewer = Viewer::new(800, 1000, 1.0);
        viewer
            .handle(Command::Restrict(RestrictionScope::Window(
                RestrictionPolicy::default().with(Operation::Process, RestrictionLevel::Ask),
            )))
            .for_each(drop);
        viewer
            .handle(Command::Open {
                id: DOCUMENT,
                bytes: bytes.clone().into(),
                password: None,
                fragment: None,
            })
            .for_each(drop);
        let events: Vec<Event> = viewer
            .handle(Command::Answer {
                document: DOCUMENT,
                proceed,
            })
            .collect();
        let count = match viewer.query(Query::PageCount) {
            Answer::Count(pages) => Some(pages),
            _ => None,
        };
        (events, count)
    };

    let (yes, pages) = asked(true);
    assert!(
        yes.iter()
            .any(|event| matches!(event, Event::Opened { pages: 1, .. })),
        "the `yes` releases the open the question held: {yes:?}"
    );
    assert_eq!(pages, Some(1), "and the document is the focused one");

    let (no, pages) = asked(false);
    assert!(
        !no.iter().any(|event| matches!(event, Event::Opened { .. })),
        "a declined question opens nothing: {no:?}"
    );
    assert_eq!(
        pages, None,
        "and the viewer holds no document the person cannot see"
    );
}
