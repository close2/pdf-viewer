//! The host driven with no `QApplication`: a document opened at a stated viewport and every
//! question the C++ side asks answered from this side alone.

use super::forms::describe_kind;
use super::panels::{flatten, pages_panel_index};
use super::{Host, PANELS};
use pdf_model::view::WidgetAppearances;
use pdf_syntax::ObjectId;
use std::path::{Path, PathBuf};
use viewer_host::Tab;
use viewer_host::form::ControlKind;
use viewer_host::panel::{PanelRow, RowAction};
use viewer_host::trace::Trace;

/// A byte of `/Contents` and a position of Qt's plain text document over it are one place
/// both ways round: a carriage return and line feed together are one block separator, a
/// letter outside the basic plane two units (ADR 1782).
#[test]
fn a_note_s_bytes_and_its_editor_s_positions_are_one_place() {
    let contents = "a\r\nb\u{1F600}c\rd";
    // a ␍␊ b 😀 c ␍ d — positions 0 1 2 3 5 6 7, bytes 0 1 3 4 8 9 10.
    let places = [(0, 0), (1, 1), (3, 2), (4, 3), (8, 5), (9, 6), (10, 7)];
    for (byte, position) in places {
        assert_eq!(
            super::notes::editor_position(contents, byte),
            Some(position)
        );
        assert_eq!(super::notes::contents_byte(contents, position), byte);
    }
    assert_eq!(
        super::notes::editor_position(contents, 5),
        None,
        "inside a character"
    );
    assert_eq!(super::notes::contents_byte(contents, 99), contents.len());
}

/// Every panel the shared list states has a widget in this toolkit, and one is not a tree.
///
/// **The instrument `doc/todo/30`'s "all three hosts stay level" never had for a *panel***, and
/// the same shape as the key test in `crate::keys` (ADR 0526). The match is exhaustive over
/// [`viewer_host::Tab`], so a panel added there fails to compile here — and in
/// [`Host::panel_of`], which is where its rows have to come from. The runtime half is the
/// *place*: `window.cpp` builds a `QListView` at whatever [`pages_panel_index`] answers and a
/// `QTreeView` everywhere else, so a panel inserted before §12.3.4's in the list must move
/// both, and this is what says they moved together.
///
/// It needs no display: nothing here calls into Qt.
#[test]
fn every_panel_the_list_states_has_a_widget_in_this_toolkit() {
    let mut lists = 0;
    for (place, tab) in Tab::ALL.iter().enumerate() {
        // Exhaustive on purpose. A new panel lands here as a compile error.
        let tree = match tab {
            Tab::Contents | Tab::Layers | Tab::Files | Tab::Articles | Tab::Document => true,
            Tab::Pages => false,
        };
        if !tree {
            assert_eq!(
                place,
                usize::from(pages_panel_index()),
                "the C++ builds its list of miniatures somewhere else"
            );
            lists += 1;
        }
    }
    assert_eq!(lists, 1, "exactly one panel is a list of pictures");
    assert_eq!(PANELS, Tab::ALL.len());
}

/// A tab whose label is Chinese asks the machine for a face off this thread, and the window is
/// handed the file the search named (ADR 1406).
///
/// A document stating no `/Title` is labelled by its file's name, so a one-page document with no
/// information dictionary is written under a Chinese one. Where the machine offers no face
/// stating the characters the test says so and skips, as ADR 1154's tests do.
#[test]
fn a_chinese_tab_label_is_handed_the_machines_face() {
    let directory = std::env::temp_dir().join(format!("quorra-qt-faces-{}", std::process::id()));
    std::fs::create_dir_all(&directory).expect("a temporary directory");
    let path = directory.join("多边形批注.pdf");
    std::fs::write(&path, untitled()).expect("the document is written");
    let mut host = opened(&path);
    assert_eq!(
        host.faces_wait(),
        -1,
        "nothing is asked before the strip is"
    );
    let labels = host.documents();
    assert_eq!(labels, vec!["多边形批注.pdf".to_owned()]);
    assert!(
        host.faces_wait() >= 0,
        "the search is running or has landed"
    );
    host.faces.settle();
    if host.faces.found() > 0 {
        assert!(
            host.take_face().is_empty(),
            "nothing is handed to Qt before the first frame is on the screen"
        );
        assert!(
            host.faces_wait() > 0,
            "a face found early is asked for again"
        );
    }
    host.painted(0, 0);
    let bytes = host.take_face();
    std::fs::remove_dir_all(&directory).expect("the temporary directory is removed");
    if bytes.is_empty() {
        eprintln!("skipped: this machine offers no face stating 多边形批注's characters");
        return;
    }
    let asked = viewer_host::machine_faces::Wanted {
        character: '多',
        bold: false,
        italic: false,
    };
    assert_eq!(
        host.faces.ask(asked),
        viewer_host::machine_faces::Answer::Face(0),
        "the first face handed over is the one that answered the first character"
    );
    while !host.take_face().is_empty() {}
    assert!(host.take_face().is_empty(), "every face is taken once");
    assert_eq!(host.faces_wait(), -1, "nothing left to look for");
}

/// An outline title Qt's own fallback drew as boxes asks the machine for a face as a tab's label
/// does, and nothing is asked before the panel's rows are (ADR 1418).
///
/// §12.3.3's `/Title` is "[t]he text that shall be displayed on the screen for this item", in a
/// `QTreeView` that falls back by family exactly as the tab bar does. Skips with a sentence
/// where the machine offers no face stating the characters, as ADR 1154's tests do.
#[test]
fn a_chinese_outline_title_asks_the_machine_for_a_face() {
    let directory =
        std::env::temp_dir().join(format!("quorra-qt-outline-faces-{}", std::process::id()));
    std::fs::create_dir_all(&directory).expect("a temporary directory");
    let path = directory.join("outlined.pdf");
    let title = "多边形批注"
        .encode_utf16()
        .fold(String::new(), |mut hex, unit| {
            use std::fmt::Write as _;
            let _ = write!(hex, "{unit:04X}");
            hex
        });
    std::fs::write(&path, outlined(&title)).expect("the document is written");
    let host = opened(&path);
    std::fs::remove_dir_all(&directory).expect("the temporary directory is removed");
    assert_eq!(
        host.faces_wait(),
        -1,
        "nothing is asked before the rows are"
    );
    let contents = u8::try_from(Tab::Contents.index()).expect("a handful of panels");
    let rows = host.rows(contents);
    assert_eq!(
        rows.first().map(|row| row.label.as_str()),
        Some("多边形批注")
    );
    assert!(
        host.faces_wait() >= 0,
        "the search is running or has landed"
    );
    host.faces.settle();
    if host.faces.found() == 0 {
        eprintln!("skipped: this machine offers no face stating 多边形批注's characters");
        return;
    }
    let asked = viewer_host::machine_faces::Wanted {
        character: '批',
        bold: false,
        italic: false,
    };
    assert!(
        matches!(
            host.faces.ask(asked),
            viewer_host::machine_faces::Answer::Face(_)
        ),
        "every character of the title was asked for"
    );
}

/// §12.2's `/DisplayDocTitle` puts XMP's `dc:title` in the title bar, and `/FitWindow` and
/// `/CenterWindow` are answered to the window at its first frame: resize until the viewport is
/// the page, then move to the centre, then nothing (ADR 1429).
#[test]
fn table_147s_window_entries_reach_the_window() {
    use crate::bridge::ffi::QtWindowExtents;
    let directory = std::env::temp_dir().join(format!("quorra-qt-window-{}", std::process::id()));
    std::fs::create_dir_all(&directory).expect("a temporary directory");
    let path = directory.join("placed.pdf");
    let packet = "<x:xmpmeta xmlns:x=\"adobe:ns:meta/\"><rdf:RDF \
        xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\"><rdf:Description \
        rdf:about=\"\" xmlns:dc=\"http://purl.org/dc/elements/1.1/\"><dc:title><rdf:Alt>\
        <rdf:li xml:lang=\"x-default\">The stated title</rdf:li></rdf:Alt></dc:title>\
        </rdf:Description></rdf:RDF></x:xmpmeta>";
    let metadata = format!(
        "<< /Type /Metadata /Subtype /XML /Length {} >>\nstream\n{packet}\nendstream",
        packet.len()
    );
    std::fs::write(
        &path,
        written(&[
            "<< /Type /Catalog /Pages 2 0 R /Metadata 4 0 R /ViewerPreferences \
             << /DisplayDocTitle true /FitWindow true /CenterWindow true >> >>",
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 200] >>",
            &metadata,
        ]),
    )
    .expect("the document is written");
    let mut host = opened(&path);
    std::fs::remove_dir_all(&directory).expect("the temporary directory is removed");
    assert!(
        host.title().starts_with("The stated title"),
        "{}",
        host.title()
    );

    let viewer_core::Answer::Geometry(geometry) =
        host.state.viewer.query(viewer_core::Query::PageGeometry(0))
    else {
        panic!("page one has a geometry");
    };
    let (wide, tall) = (
        i32::try_from(geometry.width).expect("a page's width in pixels"),
        i32::try_from(geometry.height).expect("a page's height in pixels"),
    );
    // A window 200 wider and 100 taller than its 800 by 1000 viewport, on a 1400 by 1200
    // screen with no frame.
    let resize = host.place_window(QtWindowExtents {
        window_width: 1000,
        window_height: 1100,
        frame_width: 0,
        frame_height: 0,
        screen_left: 0,
        screen_top: 0,
        screen_width: 1400,
        screen_height: 1200,
        positions: true,
    });
    assert_eq!(resize, vec![1, 200 + wide, 100 + tall, 0, 0, 0]);
    host.resized(
        u32::try_from(wide).expect("positive"),
        u32::try_from(tall).expect("positive"),
        1.0,
    );
    drawn(&mut host);
    let centre = host.place_window(QtWindowExtents {
        window_width: 200 + wide,
        window_height: 100 + tall,
        frame_width: 0,
        frame_height: 0,
        screen_left: 0,
        screen_top: 0,
        screen_width: 1400,
        screen_height: 1200,
        positions: true,
    });
    assert_eq!(
        centre,
        vec![0, 0, 0, 1, (1400 - 200 - wide) / 2, (1200 - 100 - tall) / 2]
    );
    assert!(
        host.place_window(QtWindowExtents {
            window_width: 200 + wide,
            window_height: 100 + tall,
            frame_width: 0,
            frame_height: 0,
            screen_left: 0,
            screen_top: 0,
            screen_width: 1400,
            screen_height: 1200,
            positions: true,
        })
        .is_empty(),
        "paid once"
    );
}

/// `/CenterWindow` on a platform that places every window itself — Wayland, where `move` is
/// ignored — asks for no move, says why, and names it in the title (ADR 2308).
#[test]
fn a_centre_the_platform_will_not_give_is_said_and_titled() {
    use crate::bridge::ffi::QtWindowExtents;
    let directory =
        std::env::temp_dir().join(format!("quorra-qt-uncentred-{}", std::process::id()));
    std::fs::create_dir_all(&directory).expect("a temporary directory");
    let path = directory.join("centred.pdf");
    std::fs::write(
        &path,
        written(&[
            "<< /Type /Catalog /Pages 2 0 R /ViewerPreferences << /CenterWindow true >> >>",
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 200] >>",
        ]),
    )
    .expect("the document is written");
    let mut host = opened(&path);
    std::fs::remove_dir_all(&directory).expect("the temporary directory is removed");
    drawn(&mut host);
    assert!(
        !host
            .title()
            .contains(viewer_host::native_window::NOT_CENTRED_TITLE)
    );
    let unmoved = host.place_window(QtWindowExtents {
        window_width: 1000,
        window_height: 800,
        frame_width: 0,
        frame_height: 0,
        screen_left: 0,
        screen_top: 0,
        screen_width: 1400,
        screen_height: 1200,
        positions: false,
    });
    assert!(unmoved.is_empty(), "{unmoved:?}");
    assert!(
        host.title()
            .contains(viewer_host::native_window::NOT_CENTRED_TITLE),
        "{}",
        host.title()
    );
    assert_eq!(host.status(), viewer_host::native_window::NOT_CENTRED);
}

/// A document stating none of Table 147's window entries is titled by its file and placed
/// nowhere.
#[test]
fn a_silent_document_is_titled_by_its_file_and_placed_nowhere() {
    use crate::bridge::ffi::QtWindowExtents;
    let directory = std::env::temp_dir().join(format!("quorra-qt-silent-{}", std::process::id()));
    std::fs::create_dir_all(&directory).expect("a temporary directory");
    let path = directory.join("silent.pdf");
    std::fs::write(&path, untitled()).expect("the document is written");
    let mut host = opened(&path);
    std::fs::remove_dir_all(&directory).expect("the temporary directory is removed");
    assert!(host.title().starts_with("silent.pdf"), "{}", host.title());
    assert!(
        host.place_window(QtWindowExtents {
            window_width: 1000,
            window_height: 1100,
            frame_width: 0,
            frame_height: 0,
            screen_left: 0,
            screen_top: 0,
            screen_width: 1400,
            screen_height: 1200,
            positions: true,
        })
        .is_empty()
    );
}

/// One empty page and nothing else: no information dictionary, so no title to be labelled by.
fn untitled() -> Vec<u8> {
    written(&[
        "<< /Type /Catalog /Pages 2 0 R >>",
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 200] >>",
    ])
}

/// One empty page and an outline of one item, whose `/Title` is the UTF-16BE `title` in hex.
fn outlined(title: &str) -> Vec<u8> {
    let item = format!("<< /Title <FEFF{title}> /Parent 4 0 R /Dest [3 0 R /Fit] >>");
    written(&[
        "<< /Type /Catalog /Pages 2 0 R /Outlines 4 0 R >>",
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 200] >>",
        "<< /Type /Outlines /First 5 0 R /Last 5 0 R /Count 1 >>",
        &item,
    ])
}

/// A file of `objects`, numbered from 1, with a cross-reference table and the first as root.
fn written(objects: &[&str]) -> Vec<u8> {
    let mut out = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (index, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(
            format!("{} 0 obj\n{body}\nendobj\n", index.saturating_add(1)).as_bytes(),
        );
    }
    let xref = out.len();
    let size = objects.len().saturating_add(1);
    out.extend_from_slice(format!("xref\n0 {size}\n0000000000 65535 f \n").as_bytes());
    for offset in offsets {
        out.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!("trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes(),
    );
    out
}

/// A document committed in `doc/`, which every checkout has once the archive is unpacked.
fn committed() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc/PDF20_AN001-BPC.pdf")
}

/// A corpus document, or `None` when the submodule is not checked out.
fn corpus(name: &str) -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../doc/pdf.js/test/pdfs")
        .join(name);
    path.exists().then_some(path)
}

/// A host with the document open at a viewport of a stated size, and no window anywhere.
///
/// **This is the whole of what a workspace test suite can see of a Qt host**, and it is more
/// than it could see of the GTK one: `Host` here is a plain struct C++ happens to own, so a
/// test can build one, drive it and read its answers with no display and no `QApplication`.
/// That is the ownership inversion paying a dividend rather than costing one (ADR 0246).
fn opened(path: &Path) -> Host {
    opened_under(path, viewer_core::RestrictionLevel::On)
}

/// The same, with the reader's answer to what the document asserts stated explicitly.
///
/// `CLAUDE.md`'s rule is that turning a document's restrictions off is always possible, and
/// this is where a test can see whether the value the command line supplies reaches the core
/// at all — no display, no `QApplication`, and a real file asserting a real `/P`.
fn opened_under(path: &Path, level: viewer_core::RestrictionLevel) -> Host {
    let restrictions = viewer_core::RestrictionPolicy::uniform(level);
    let mut host = Host::open(
        path,
        None,
        WidgetAppearances::Delegated,
        viewer_host::Settings {
            restrictions,
            links: viewer_host::Links::Refuse,
            remote_documents: viewer_host::RemoteDocuments::Refuse,
            embedded_documents: viewer_host::EmbeddedDocuments::Refuse,
            submissions: viewer_host::Submissions::Refuse,
            separations: false,
            scripts: viewer_host::Scripts::Off,
            launched_documents: viewer_host::LaunchedDocuments::Refuse,
        },
        viewer_host::ReaderWords::default(),
        Trace::off(std::time::Instant::now()),
    )
    .expect("the document is readable");
    host.resized(800, 1000, 1.0);
    drawn(&mut host);
    host
}

/// Waits for the page to come back from the drawing thread.
///
/// **A test is a host with no event loop**, so it supplies what `MainWindow::pumpDrawing`
/// supplies in a running window: the page is rasterised on a thread of its own since ADR 0668,
/// and a test asking about a frame the moment `resized` returns would be asking before it had
/// been drawn. Bounded rather than looping for ever, because a thread that never answers is a
/// defect and not something to hang on.
fn drawn(host: &mut Host) {
    let began = std::time::Instant::now();
    while host.drawing_wait() >= 0 {
        assert!(
            began.elapsed() < std::time::Duration::from_mins(2),
            "the drawing thread never answered"
        );
        std::thread::sleep(viewer_host::drawing::POLL);
        host.drawing_pump();
    }
}

/// §12.3.3's outline reaches the bridge flattened, in the order a tree shows it.
#[test]
fn the_outline_crosses_depth_first_with_a_depth_on_every_row() {
    let host = opened(&committed());
    let rows = host.rows(0);
    assert_eq!(rows.len(), 14, "the document's outline is 14 items");
    assert_eq!(rows[0].depth, 0, "the first row is top level");
    // Depth-first order means a row is never more than one deeper than the row above it,
    // which is exactly what lets the C++ side rebuild the parentage from a stack.
    for pair in rows.windows(2) {
        assert!(
            pair[1].depth <= pair[0].depth.saturating_add(1),
            "{:?} follows {:?}",
            pair[1],
            pair[0]
        );
    }
    // §12.3.3: "[i]f the outline item is open, Count is the sum of the number of visible
    // descendent outline items." So the flag belongs to an item that *has* descendants, and
    // every one of this document's does ask to be open — which is what makes all 14 rows
    // visible. A leaf carries `false` and is shown anyway, which is the distinction ADR
    // 0244's "every one of which asks to be open" ran together.
    for (above, below) in rows.iter().zip(rows.iter().skip(1)) {
        if below.depth > above.depth {
            assert!(above.expanded, "a parent this file opened: {above:?}");
        }
    }
    let open = rows.iter().filter(|row| row.expanded).count();
    let parents = rows
        .iter()
        .zip(rows.iter().skip(1))
        .filter(|(above, below)| below.depth > above.depth)
        .count();
    assert_eq!(
        open, parents,
        "every item with children is one this file opened"
    );
    // Every row activates an object, because an outline item is a destination or an action
    // and which of the two it is belongs to the document (ADR 0144).
    assert!(rows.iter().all(|row| row.action == 1));
}

/// The panels this document states nothing for say so rather than showing an empty list.
///
/// Trap 5's shape for a panel: an empty list is indistinguishable from a list this program
/// failed to fill (ADR 0564). The sentence is `viewer_host::panel`'s, so all three hosts say
/// the same one.
#[test]
fn a_document_with_no_layers_and_no_files_says_so_rather_than_showing_nothing() {
    let host = opened(&committed());
    for tab in [Tab::Layers, Tab::Files] {
        let rows = host.rows(index(tab));
        assert_eq!(rows.len(), 1, "{tab:?}");
        assert!(rows[0].note, "{tab:?} says nothing about being empty");
    }
    // §12.3.4's panel is the one with no rows at all: its pictures are `page_row`'s.
    assert!(host.rows(index(Tab::Pages)).is_empty());
    // And a panel number this host does not have answers nothing rather than the first one.
    assert!(host.rows(99).is_empty());
}

/// A panel's place in the shared list, as the C++ side spells one.
fn index(tab: Tab) -> u8 {
    u8::try_from(tab.index()).expect("six panels fit in a byte")
}

/// §12.7's form becomes controls, and the count is the one both native hosts place.
#[test]
fn a_real_form_becomes_the_same_controls_the_other_host_places() {
    let Some(path) = corpus("160F-2019.pdf") else {
        println!("skipped: the doc/pdf.js submodule is not checked out");
        return;
    };
    let host = opened(&path);
    let controls = host.controls();
    // 76 widgets over this form's 67 fields, less the 10 whose field `ControlKind::is_placed`
    // leaves to the page's own appearance (ADR 1357).
    assert_eq!(
        controls.len(),
        66,
        "the same 66 controls `viewer-gtk` places over this form's 67 fields"
    );
    // Every one of them names the field an edit is addressed by and the widget that
    // distinguishes it from the field's other widgets.
    assert!(controls.iter().all(|control| !control.field.is_empty()));
    // And every kind is one the C++ side has a widget for: 0 to 7, less 5, the push-button
    // number no control is built for.
    assert!(
        controls
            .iter()
            .all(|control| control.kind < 8 && control.kind != 5)
    );
}

/// `CLAUDE.md`: a document's restrictions are the reader's, and turning them off must work.
///
/// **The one test in this tree that drives the whole chain a person walks**: a real file
/// asserting a real §7.6.4.2 `/P`, a real key press, the sentence the window puts on the
/// screen, and the same run again with the command line's answer changed. Everything else
/// about this is checked one link at a time — `viewer-core`'s headless harness has the
/// refusal, `viewer-host` has the sentence, each binary has its parser — and the defect ADR
/// 0604 found lived exactly between two of those links, where nothing looked.
///
/// `issue17215.pdf` is one of the corpus's seven witnesses (ADR 0212): it opens with the empty
/// user password and withholds both filling in a form and annotating.
#[test]
fn a_restricted_document_refuses_and_the_reader_can_turn_that_off() {
    let Some(path) = corpus("issue17215.pdf") else {
        println!("skipped: the doc/pdf.js submodule is not checked out");
        return;
    };
    // `a` selects the page and `h` is §12.5.6.10's highlight, which is an annotation and is
    // what this document's `/P` withholds.
    let mut host = opened_under(&path, viewer_core::RestrictionLevel::On);
    host.key(0x41, false, false);
    host.key(0x48, false, false);
    let said = host.status();
    assert!(
        said.contains(viewer_host::IGNORE_RESTRICTIONS),
        "a refusal names the way out: {said}"
    );

    let mut host = opened_under(&path, viewer_core::RestrictionLevel::Off);
    host.key(0x41, false, false);
    host.key(0x48, false, false);
    let said = host.status();
    assert!(
        !said.contains(viewer_host::IGNORE_RESTRICTIONS),
        "the reader said not to obey it, so nothing is refused: {said}"
    );
    assert!(
        host.state.showing.dirty,
        "and the annotation is in the edit log: {said}"
    );
}

/// A Control this program does not bind does **nothing**, and the unbound row is untouched.
///
/// This host's seam is `Host::key`, which C++ calls with `Qt::ShiftModifier` and
/// `Qt::ControlModifier` already read — so what is checked here is the whole of what crosses:
/// the same letter, with and without Control, one of which marks up the text selection and
/// one of which is a press addressed to something else. Before ADR 1192 the window discarded
/// the modifier and both did the same thing.
///
/// `issue17215.pdf` with the restrictions turned off, so that the only thing deciding the
/// outcome is the modifier.
#[test]
fn a_control_this_program_does_not_bind_leaves_the_document_alone() {
    let Some(path) = corpus("issue17215.pdf") else {
        println!("skipped: the doc/pdf.js submodule is not checked out");
        return;
    };
    let mut host = opened_under(&path, viewer_core::RestrictionLevel::Off);
    host.key(0x41, false, false);
    host.key(0x48, false, true);
    assert!(
        !host.state.showing.dirty,
        "Ctrl and `h` is not a binding this program has, so nothing was annotated"
    );
    host.key(0x48, false, false);
    assert!(
        host.state.showing.dirty,
        "and the unmodified key still means §12.5.6.10's highlight"
    );
}

/// §12.7.5.4: what a `QListWidget` in `ExtendedSelection` has to say, and where it goes.
///
/// The C++ collects the selected rows ascending and hands them over as a slice; this is the
/// Rust half of that, driven without Qt because ADR 0246's rule is that no Qt type appears in
/// this file. Table 233 bit 22 is what makes the mode `ExtendedSelection` at all (ADR 0248).
#[test]
fn several_items_of_a_list_box_are_chosen_and_read_back() {
    let Some(path) = corpus("issue17492.pdf") else {
        println!("skipped: the doc/pdf.js submodule is not checked out");
        return;
    };
    let mut host = opened(&path);
    let Some(index) = host
        .controls()
        .iter()
        .position(|control| control.field == "databases")
    else {
        panic!("Table 233 bit 22's list box has a control")
    };
    // 7 is the list box, and `multi` is the flag the C++ turns into a selection mode.
    assert_eq!(host.controls()[index].kind, 7);
    assert!(host.controls()[index].multi, "Table 233 bit 22");
    assert_eq!(host.control_options(index).len(), 4);

    host.choose_control(index, &[0, 2]);
    assert_eq!(
        host.control_selection(index),
        vec![0_u32, 2],
        "both, which is what a single value could not have said"
    );
    assert!(
        host.state.showing.dirty,
        "and the document knows it was edited"
    );
}

/// A frame the viewer is holding crosses as its own dimensions and a borrowed slice.
#[test]
fn the_frame_and_its_pixels_agree_about_how_many_bytes_there_are() {
    let host = opened(&committed());
    let views = host.frame_views();
    let view = views.first().expect("a page has been rasterised");
    let frame = &view.frame;
    assert!(frame.present, "a page has been rasterised");
    let need =
        usize::try_from(frame.width).unwrap_or(0) * usize::try_from(frame.height).unwrap_or(0) * 4;
    assert!(
        view.pixels.len() >= need,
        "a whole raster crosses the bridge"
    );
}

/// The update flags clear when they are taken, so nothing is rebuilt twice.
#[test]
fn taking_the_update_leaves_nothing_behind() {
    let mut host = opened(&committed());
    let first = host.take_update();
    assert!(first.frame && first.panels, "opening changes both");
    let second = host.take_update();
    assert!(!second.frame && !second.panels && !second.controls);
}

/// The flattening is depth first and carries §8.11.4.3's switch state with it.
#[test]
fn a_tree_of_rows_flattens_to_the_order_a_tree_shows() {
    let rows = vec![
        PanelRow {
            label: "top".to_owned(),
            detail: None,
            expanded: true,
            action: RowAction::Toggle {
                group: ObjectId {
                    number: 7,
                    generation: 0,
                },
                on: true,
                locked: true,
            },
            note: false,
            emphasis: false,
            cells: Vec::new(),
            picture: None,
            children: vec![PanelRow {
                label: "under".to_owned(),
                detail: Some("said".to_owned()),
                expanded: false,
                action: RowAction::Inert,
                note: false,
                emphasis: false,
                cells: Vec::new(),
                picture: None,
                children: Vec::new(),
            }],
        },
        PanelRow {
            label: "beside".to_owned(),
            detail: None,
            expanded: false,
            action: RowAction::Extract {
                name: "a.txt".to_owned(),
            },
            note: false,
            // §12.3.5.1's `/D` crosses the bridge like every other row flag, and this one
            // carries it so that the bold row is the one the *document* named.
            emphasis: true,
            cells: Vec::new(),
            picture: None,
            children: Vec::new(),
        },
    ];
    let flat = flatten(&rows);
    let shape: Vec<(&str, u32, u8)> = flat
        .iter()
        .map(|one| (one.row.label.as_str(), one.row.depth, one.row.action))
        .collect();
    assert_eq!(
        shape,
        vec![("top", 0, 2), ("under", 1, 0), ("beside", 0, 3)]
    );
    // Table 99's `/Locked` crosses beside the state rather than instead of it: the switch is
    // shown as the document set it and is not a person's to move.
    assert!(flat[0].row.on && flat[0].row.locked);
    assert_eq!(flat[1].row.detail, "said");
    // §12.3.5.1's mark travels with the row it is on and with no other.
    assert_eq!(
        flat.iter().map(|one| one.row.emphasis).collect::<Vec<_>>(),
        vec![false, false, true]
    );
}

/// Every control kind has a number, and the two the clause gives no control to have none.
#[test]
fn every_control_kind_crosses_as_a_number_the_window_knows() {
    assert_eq!(
        describe_kind(&ControlKind::Entry {
            multiline: false,
            password: true,
            max_len: Some(8),
            file_select: false,
        }),
        (2, 8, false, false)
    );
    assert_eq!(
        describe_kind(&ControlKind::Entry {
            multiline: true,
            password: false,
            max_len: None,
            file_select: false,
        }),
        (1, -1, false, false)
    );
    // §12.7.5.2.2's push-button, §12.7.5.5's signature and Table 226's absent `/FT` are never
    // placed, so their number is one the window refuses by name rather than one it draws.
    assert_eq!(describe_kind(&ControlKind::Push).0, 255);
    assert_eq!(describe_kind(&ControlKind::Signature).0, 255);
    assert_eq!(describe_kind(&ControlKind::Unstated).0, 255);
}
/// RFC 0004 §5's print path, driven end to end with no `QApplication` and no printer.
///
/// **What this can see and what it cannot.** Everything from the key press to the pixels is
/// Rust and is exercised here: ISO 32000-2 §7.6.4.2's bit 3 asked as an operation, the grant,
/// §12.2's Table 147 as the dialogue's opening state, §12.5.6.22's placement for a two-up
/// sheet, one page rasterised on the processor, and §8.11.4.5's revert. What it cannot see is
/// `QPrinter` and `QPrintDialog`, which are `window.cpp`'s and need a person in front of a
/// modal dialogue — so those are compiled and linked and not driven, exactly as ADR 1180 left
/// the GTK path (ADR 1203).
#[test]
fn the_print_path_answers_a_dialogue_without_one_being_on_the_screen() {
    let mut host = opened_under(&committed(), viewer_core::RestrictionLevel::Off);
    // `Qt::Key_P` with Shift, which is what `viewer_host::keys` makes the print key.
    host.key(0x50, true, false);
    let update = host.take_update();
    assert!(update.print_dialogue, "the grant asks for a dialogue");

    let job = host.print_job();
    assert!(job.pages > 0, "a job names the document's pages");
    assert!(
        job.page_width > 0.0 && job.page_height > 0.0,
        "and the page a placement is computed from: {job:?}"
    );
    assert!(!job.degraded, "this document states no /Encrypt at all");
    assert!(job.refusal.is_empty(), "so no destination is refused");

    // Two up on A4 in points: the sheet's longer axis is halved, both cells are one size, and
    // neither overlaps the other — which is §12.5.6.22's "single portion of the page".
    let cells = host.print_cells(595.0, 842.0, 1, 2);
    assert_eq!(cells.len(), 2);
    assert!(cells[0].y > cells[1].y, "the first cell is the upper one");
    assert!(
        (cells[0].width - cells[1].width).abs() < 0.01
            && (cells[0].height - cells[1].height).abs() < 0.01,
        "one size for every cell"
    );
    host.print_paper(595.0, 842.0, 300.0, 1, 2);

    let size = host.print_page(0);
    assert_eq!(size.len(), 2);
    assert!(size[0] > 0 && size[1] > 0, "page one came back as pixels");
    let pixels = host.print_page_pixels();
    assert_eq!(
        pixels.len(),
        (size[0] as usize) * (size[1] as usize) * 4,
        "row-major RGBA8 with no padding, which is what the C++ borrows"
    );

    host.print_finish();
    assert!(!host.printing, "§8.11.4.5's revert");
    assert_eq!(
        host.print_job().pages,
        0,
        "and no job to open a dialogue on"
    );
}

/// A document opened behind the first is asked for what it says about itself and for its open
/// sequence once its own frame is on the screen, and the first is not asked again (ADR 2544).
///
/// The second document's page is drawn while its tab is behind, so the frame it shows when it is
/// brought to the front is one the viewer already holds and the viewer's answer for a page never
/// arrives for it: `painted` is the moment, and `arrive` asks.
#[test]
fn a_document_opened_behind_is_asked_for_its_report_once_its_frame_is_shown() {
    use viewer_host::native_window::NativeWindow as _;
    let directory = std::env::temp_dir().join(format!("quorra-qt-report-{}", std::process::id()));
    std::fs::create_dir_all(&directory).expect("a temporary directory");
    let (first, second) = (directory.join("first.pdf"), directory.join("second.pdf"));
    std::fs::write(&first, untitled()).expect("the first document is written");
    std::fs::write(&second, untitled()).expect("the second document is written");
    let mut host = opened(&first);
    host.painted(0, 0);
    assert!(
        !host.state.showing.report_due.is_owed(),
        "the first document was asked when its page was answered"
    );
    host.state
        .arrivals
        .wait(viewer_host::Named::file(second), true);
    host.open_the_next();
    drawn(&mut host);
    assert_eq!(host.documents().len(), 2, "the second document has its tab");
    assert_eq!(
        host.focused_document(),
        0,
        "and the first has the front back"
    );
    host.choose_document(1);
    drawn(&mut host);
    host.painted(0, 0);
    let owed_before = host.state.showing.report_due.is_owed();
    assert!(
        host.arrival_due(),
        "C++ is asked to call back once the frame is shown"
    );
    host.arrive();
    std::fs::remove_dir_all(&directory).expect("the temporary directory is removed");
    assert!(
        owed_before,
        "the frame the second document showed was one the viewer held, so `painted` found it owing"
    );
    assert!(
        !host.state.showing.report_due.is_owed(),
        "the second document has been asked"
    );
    host.choose_document(0);
    drawn(&mut host);
    host.painted(0, 0);
    assert!(
        host.owes_its_report.is_none() && !host.state.showing.report_due.is_owed(),
        "the first document is not asked twice"
    );
}
