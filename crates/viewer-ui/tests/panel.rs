//! The outline panel, drawn and clicked with no window at all.
//!
//! `viewer-ui`'s panel is chrome and chrome has no gate — the corpus interprets page one, the
//! oracle rasterises pages it is handed, and neither of them opens a viewer. So the two things
//! that can silently go wrong here are checked directly: that the panel **draws ink where it
//! says it does**, by rasterising it with the same `render-cpu` the oracle is built on, and that
//! a click lands on the row a person aimed at, by asking the same function the window asks.
//!
//! Trap 1's rule applies to an interface as much as to a page: a display list holding the right
//! commands can still draw nothing.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "test code: a fixture that cannot exercise the rule must fail loudly rather than \
              pass by doing nothing, and a pixel index over a raster this file sized itself \
              cannot overflow"
)]

use pdf_model::destination::{Destination, Target, View};
use pdf_model::outline::{Item, Outline};
use pdf_render::{Rasterizer as _, TargetSpec, Transform};
use pdf_syntax::ObjectId;
use render_cpu::CpuRasterizer;
use viewer_core::Layer;
use viewer_ui::chrome::{About, Chrome, Content, Hit, Sidebar, Style};

/// A window this test pretends to have: wide enough for the panel, tall enough for the rows.
const WIDTH: u32 = 400;
/// How tall the pretend window is.
const HEIGHT: u32 = 300;

/// An item with a destination naming an object, which is §12.3.2.2's first form.
fn item(title: &str, page: u32, children: Vec<Item>) -> Item {
    Item {
        // The item's own object. `page + 100` keeps it distinct from the page it points at, so
        // a test that confused the two would fail rather than pass by coincidence.
        id: ObjectId::new(page + 100, 0),
        title: title.to_owned(),
        destination: Some(Destination {
            target: Target::Object(ObjectId::new(page, 0)),
            view: View::Fit,
        }),
        open: true,
        italic: false,
        bold: false,
        colour: [0.0, 0.0, 0.0],
        children,
    }
}

/// A sidebar showing nothing but this outline, which is what the outline tests need.
fn only(outline: &Outline) -> Content<'_> {
    Content {
        outline,
        layers: &[],
        attachments: &[],
        articles: &[],
        collection: None,
        information: &NOTHING,
        metadata: None,
        pages: &[],
    }
}

/// A document that states no §14.3.3 information, which most of them do.
static NOTHING: std::sync::LazyLock<pdf_model::metadata::Information> =
    std::sync::LazyLock::new(pdf_model::metadata::Information::default);

/// Two levels, so that indentation, disclosure and identity all have something to act on.
fn outline() -> Outline {
    Outline {
        items: vec![
            item("Chapter one", 10, vec![item("Section 1.1", 11, Vec::new())]),
            item("Chapter two", 20, Vec::new()),
        ],
        stated_count: None,
    }
}

/// How many pixels of the panel are darker than its background, within a band of rows.
///
/// The background is a light grey (240 of 255) and the hover highlight is 219, so "ink" is
/// anything under 180 — which counts black text and the 107 a dimmed row is set in, and counts
/// neither background. Counting rather than comparing to a golden image: a golden would pin the
/// shape of Liberation Sans, and what is being checked is that glyphs reached the raster at all.
fn ink(panel: &pdf_render::DisplayList, rows: std::ops::Range<u32>) -> usize {
    ink_within(panel, rows, 0..WIDTH)
}

/// [`ink`], counted only inside a range of columns.
fn ink_within(
    panel: &pdf_render::DisplayList,
    rows: std::ops::Range<u32>,
    columns: std::ops::Range<u32>,
) -> usize {
    let raster = CpuRasterizer::new()
        .rasterize(
            panel,
            TargetSpec {
                width: WIDTH,
                height: HEIGHT,
                transform: Transform::IDENTITY,
            },
        )
        .expect("the panel is paths and nothing else");
    let mut dark = 0;
    for y in rows {
        for x in columns.clone() {
            let at = ((y * WIDTH + x) * 4) as usize;
            if raster.data.get(at).is_some_and(|red| *red < 180) {
                dark += 1;
            }
        }
    }
    dark
}

/// The panel draws its heading and its rows, and a hidden panel draws nothing.
/// The middle of one tab in the strip, in logical pixels.
///
/// `chrome`'s `PANEL_WIDTH` divided by the number of tabs, which is what `Sidebar::draw` does —
/// written out here rather than exported, because a test that computed the position with the
/// code under test would follow it into any mistake. **Update the divisor when a tab is added**:
/// two tests failed the day §12.3.4's arrived, on a hard-coded 100.0 that had meant "the second
/// of four" and came to mean "the second of five", and three failed the day §12.4.3's did.
fn tab(index: usize) -> f32 {
    #[expect(clippy::cast_precision_loss, reason = "one of six tabs")]
    let middle = (index as f32 + 0.5) / 6.0;
    300.0 * middle
}

#[test]
fn the_panel_puts_ink_on_the_rows_it_lists() {
    let chrome = Chrome::new().expect("§9.6.2.2's fourteen are compiled in");
    let outline = outline();
    let mut panel = Sidebar::default();

    let hidden = panel.draw(&chrome, only(&outline), HEIGHT, 1.0);
    assert_eq!(ink(&hidden, 0..HEIGHT), 0, "a hidden panel draws nothing");

    panel.toggle();
    let shown = panel.draw(&chrome, only(&outline), HEIGHT, 1.0);
    // The heading sits above the separator and the first row below it; both must have glyphs,
    // because a panel that drew its background and no text would pass any command count.
    assert!(ink(&shown, 0..26) > 40, "the heading has no glyphs");
    assert!(ink(&shown, 26..46) > 40, "the first row has no glyphs");
    assert!(
        ink(&shown, 90..HEIGHT) == 0,
        "three rows, and nothing drawn below them"
    );
}

/// A title §9.6.2.2's fourteen cannot set draws a box per character rather than nothing.
///
/// `doc/todo/27`: the popup window has said how many characters it could not set since the
/// three-hundred-and-twelfth session, and every other string this host draws from a document —
/// an outline title, a layer name, an `/Info` value — was dropped in silence, so a Japanese
/// document's outline was a panel of empty rows. Trap 5 one clause over: a person shown an empty
/// row has been told the document states nothing there.
#[test]
fn a_title_this_interfaces_font_cannot_set_draws_a_box_for_each_character() {
    let chrome = Chrome::new().expect("§9.6.2.2's fourteen are compiled in");
    let title = "多边形批注";
    let outline = Outline {
        items: vec![item(title, 10, Vec::new())],
        stated_count: None,
    };
    let mut panel = Sidebar::default();
    panel.toggle();
    let shown = panel.draw(&chrome, only(&outline), HEIGHT, 1.0);
    assert!(
        ink(&shown, 26..46) > 40,
        "five characters with no code drew nothing at all"
    );

    // And the boxes are measured as they are drawn, which is what elision and the popup's title
    // bar depend on: five of them at 0.6 em each, at the panel's 12 logical pixels.
    let width = chrome.width(title, 12.0, Style::default());
    assert!(
        (width - 5.0 * 0.6 * 12.0).abs() < 0.01,
        "a placeholder that is drawn and not measured: {width}"
    );
    assert_eq!(
        chrome.without_a_code(title, Style::default()),
        5,
        "and the count a caller says out loud is unchanged"
    );
}

/// A click follows a destination, a click on the triangle discloses, and neither does the other.
#[test]
fn a_click_lands_on_the_row_it_was_aimed_at() {
    let outline = outline();
    let mut panel = Sidebar::default();
    assert_eq!(
        panel.click((10.0, 40.0), only(&outline), 1.0),
        None,
        "a hidden panel owns no clicks"
    );
    panel.toggle();

    // Outside it: the page's, and the panel says so by answering nothing.
    assert_eq!(panel.click((300.0, 40.0), only(&outline), 1.0), None);
    // In the tab strip: the first of three tabs, which is the one already showing. Answered
    // rather than passed on — a click falling through would start a text selection on a page
    // nobody can see.
    assert_eq!(
        panel.click((tab(0), 8.0), only(&outline), 1.0),
        Some(Hit::Redraw)
    );

    // The first row's title. §12.3.3 asks for the *item* to be activated rather than for its
    // destination to be followed, because the same sentence covers `/A`.
    assert_eq!(
        panel.click((80.0, 36.0), only(&outline), 1.0),
        Some(Hit::Activate(outline.items[0].id))
    );

    // The first row's disclosure triangle, which is the left fourteen pixels.
    assert_eq!(
        panel.click((6.0, 36.0), only(&outline), 1.0),
        Some(Hit::Redraw),
        "the triangle discloses rather than navigating"
    );
}

/// Closing a subtree hides it, and does **not** renumber what is below it.
///
/// The identities are what a toggle is keyed by, and the trap is numbering them by *visible*
/// row: close the first chapter and the second would inherit the first's identity, so the next
/// click would toggle the wrong thing. They are numbered in pre-order over every item instead,
/// and this is what says so.
#[test]
fn closing_a_subtree_does_not_renumber_the_items_below_it() {
    let chrome = Chrome::new().expect("§9.6.2.2's fourteen are compiled in");
    let outline = outline();
    let mut panel = Sidebar::default();
    panel.toggle();

    let three_rows = ink(&panel.draw(&chrome, only(&outline), HEIGHT, 1.0), 26..90);
    assert_eq!(
        panel.click((6.0, 36.0), only(&outline), 1.0),
        Some(Hit::Redraw),
        "chapter one closes"
    );
    let two_rows = ink(&panel.draw(&chrome, only(&outline), HEIGHT, 1.0), 26..90);
    assert!(
        two_rows < three_rows,
        "the closed subtree is still being drawn: {two_rows} against {three_rows}"
    );

    // Chapter two is now the second visible row, and clicking *it* must reach chapter two
    // rather than the section that was hidden.
    assert_eq!(
        panel.click((80.0, 56.0), only(&outline), 1.0),
        Some(Hit::Activate(outline.items[1].id)),
        "the row below a closed subtree is chapter two"
    );

    assert_eq!(
        panel.click((6.0, 36.0), only(&outline), 1.0),
        Some(Hit::Redraw),
        "chapter one opens again"
    );
    assert_eq!(
        ink(&panel.draw(&chrome, only(&outline), HEIGHT, 1.0), 26..90),
        three_rows,
        "reopening restores exactly what closing hid"
    );
}

/// A title longer than the panel is cut with an ellipsis rather than drawn over the page.
#[test]
fn a_long_title_is_elided_to_the_width_available() {
    let chrome = Chrome::new().expect("§9.6.2.2's fourteen are compiled in");
    let long = "Annex Q (normative) Method for determining something at very great length";
    let mut panel = Sidebar::default();
    panel.toggle();
    let outline = Outline {
        items: vec![item(long, 10, Vec::new())],
        stated_count: None,
    };
    let list = panel.draw(&chrome, only(&outline), HEIGHT, 1.0);
    // The whole title is wider than the panel, so something must have been dropped — and what
    // is drawn must stay inside it. Measured through the same function that laid it out.
    assert!(
        chrome.width(long, 12.0, Style::default()) > 300.0,
        "the fixture is not long enough to elide"
    );
    assert!(ink(&list, 26..46) > 40, "the row has glyphs");
    assert_eq!(
        ink(&list, 26..46),
        ink_within(&list, 26..46, 0..300),
        "a glyph was drawn outside the panel"
    );
}

/// §8.11's layer tab: a switch throws, a locked one does not, and a heading is not a layer.
///
/// The locked case is the clause speaking, not a preference. §8.11.4.3 on Table 99's `/Locked`:
/// "[t]he state of a locked group cannot be changed through the user interface of an interactive
/// PDF processor." So the switch is *drawn* — a person is entitled to see the state — and
/// clicking it produces nothing.
#[test]
fn a_layer_switch_throws_unless_the_document_locked_it() {
    let chrome = Chrome::new().expect("§9.6.2.2's fourteen are compiled in");
    let layers = vec![
        Layer::Collection {
            label: Some("Plans".to_owned()),
            children: vec![Layer::Group {
                group: ObjectId::new(30, 0),
                name: Some("Walls".to_owned()),
                on: true,
                locked: false,
            }],
        },
        Layer::Group {
            group: ObjectId::new(31, 0),
            name: Some("Watermark".to_owned()),
            on: false,
            locked: true,
        },
    ];
    let outline = Outline::default();
    let content = Content {
        outline: &outline,
        layers: &layers,
        attachments: &[],
        articles: &[],
        collection: None,
        information: &NOTHING,
        metadata: None,
        pages: &[],
    };
    let mut panel = Sidebar::default();
    panel.toggle();
    // The Layers tab, third of five.
    assert_eq!(
        panel.click((tab(2), 8.0), content, 1.0),
        Some(Hit::Redraw),
        "the Layers tab"
    );

    // Row 0 is the collection's heading; its left edge acts on nothing.
    assert_eq!(panel.click((6.0, 36.0), content, 1.0), Some(Hit::Nothing));
    // Row 1 is the group inside it, indented one level, so its switch is further right.
    assert_eq!(
        panel.click((20.0, 56.0), content, 1.0),
        Some(Hit::SetGroup {
            group: ObjectId::new(30, 0),
            on: false,
        }),
        "an unlocked switch offers the opposite of what is on"
    );
    // Row 2 is the locked group at depth zero.
    assert_eq!(
        panel.click((6.0, 76.0), content, 1.0),
        Some(Hit::Nothing),
        "§8.11.4.3 forbids changing a locked group from the interface"
    );

    // And all three rows are drawn, which is the other half: refusing the click must not mean
    // hiding the state.
    let list = panel.draw(&chrome, content, HEIGHT, 1.0);
    for (row, band) in [(0, 26..46), (1, 46..66), (2, 66..86)] {
        assert!(ink(&list, band) > 20, "row {row} is blank");
    }
}

/// §7.11.4's file tab lists what the document embeds, and says so when it embeds nothing.
///
/// An empty list and a list this program failed to fill look identical on a screen, and only one
/// of them is a fact about the file — so the empty case is a sentence rather than blank space,
/// and this checks there is ink either way.
#[test]
fn the_file_tab_names_what_is_embedded_and_says_when_nothing_is() {
    let chrome = Chrome::new().expect("§9.6.2.2's fourteen are compiled in");
    let outline = Outline::default();
    let empty = Content {
        outline: &outline,
        layers: &[],
        attachments: &[],
        articles: &[],
        collection: None,
        information: &NOTHING,
        metadata: None,
        pages: &[],
    };
    let mut panel = Sidebar::default();
    panel.toggle();
    assert_eq!(
        panel.click((tab(3), 8.0), empty, 1.0),
        Some(Hit::Redraw),
        "the Files tab"
    );
    assert!(
        ink(&panel.draw(&chrome, empty, HEIGHT, 1.0), 26..46) > 40,
        "an empty list must say it is empty"
    );

    // A document that does embed one: the row carries the file's name and a click asks for the
    // bytes.
    let Some(bytes) = corpus_bytes("attachment.pdf") else {
        println!("skipped: the doc/pdf.js submodule is not checked out");
        return;
    };
    let document = pdf_syntax::Document::open(bytes).expect("a corpus document");
    let files = pdf_model::attachment::attachments(&document);
    assert!(!files.is_empty(), "the fixture embeds a file");
    let listed = Content {
        outline: &outline,
        layers: &[],
        attachments: &files,
        articles: &[],
        collection: None,
        information: &NOTHING,
        metadata: None,
        pages: &[],
    };
    assert!(ink(&panel.draw(&chrome, listed, HEIGHT, 1.0), 26..46) > 40);
    // The row's click is §7.11.4 from a person's side: the bytes are inside the document, and
    // the key is the one the `/EmbeddedFiles` tree filed them under.
    assert_eq!(
        panel.click((80.0, 36.0), listed, 1.0),
        Some(Hit::Extract(files[0].name.clone()))
    );
}

/// §12.3.5's collection: the same files, in folders, with the schema's columns.
///
/// > If this dictionary is present in a PDF document, the interactive PDF processor shall present
/// > the document as a portable collection.
///
/// The files tab *becomes* that presentation where a document states one, rather than a seventh
/// tab: it is the same files, and §12.3.5.2's folder tree is how a collection says they are
/// arranged. **Not one of the 974 corpus documents states a `/Collection`**, so the fixture is
/// written here — trap 8's converse again.
#[test]
fn a_collection_puts_its_files_in_folders_with_the_schemas_columns() {
    use pdf_model::collection::{Collection, Field, FieldKind, Folder, Item};
    let chrome = Chrome::new().expect("§9.6.2.2's fourteen are compiled in");
    let outline = Outline::default();

    let schema = [
        (
            "FN".to_owned(),
            Field {
                kind: FieldKind::FileName,
                name: "File".to_owned(),
                order: Some(1),
                visible: true,
                editable: false,
            },
        ),
        (
            "SZ".to_owned(),
            Field {
                kind: FieldKind::Size,
                name: "Bytes".to_owned(),
                order: Some(2),
                visible: true,
                editable: false,
            },
        ),
        // A field the schema hides is a field the panel does not draw: Table 155's `/V` is "[t]he
        // initial visibility of the field", and a hidden column with a value is the case that
        // separates "read the schema" from "read the item".
        (
            "HD".to_owned(),
            Field {
                kind: FieldKind::Description,
                name: "Hidden".to_owned(),
                order: Some(3),
                visible: false,
                editable: false,
            },
        ),
    ]
    .into_iter()
    .collect();

    let collection = Collection {
        schema,
        folders: Some(Folder {
            id: 3,
            name: "Chapters".to_owned(),
            description: Some("the parts of it".to_owned()),
            item: Item::default(),
            has_thumbnail: false,
            children: Vec::new(),
        }),
        ..Collection::default()
    };
    // Two files: one whose key names folder 3 and one whose key names no folder at all, which
    // §12.3.5.2 puts in the root.
    let files = vec![
        attachment("<3>report.pdf", 4096),
        attachment("readme.txt", 12),
    ];
    let content = Content {
        outline: &outline,
        layers: &[],
        attachments: &files,
        articles: &[],
        collection: Some(viewer_ui::chrome::Presentation {
            collection: &collection,
            initial: &pdf_model::collection::Initial::Container,
        }),
        information: &NOTHING,
        metadata: None,
        pages: &[],
    };
    let mut panel = Sidebar::default();
    panel.toggle();
    assert_eq!(panel.click((tab(3), 8.0), content, 1.0), Some(Hit::Redraw));

    // Three rows: the rootless file, the folder, and the file inside it.
    let list = panel.draw(&chrome, content, HEIGHT, 1.0);
    for rows in [26..46, 46..66, 66..86] {
        assert!(ink(&list, rows.clone()) > 40, "row {rows:?} is drawn");
    }
    // The click still extracts, by the *tree's* key — which is what carries the folder number.
    assert_eq!(
        panel.click((80.0, 36.0), content, 1.0),
        Some(Hit::Extract("readme.txt".to_owned())),
        "the rootless file is first, above the folders"
    );
    assert_eq!(
        panel.click((80.0, 56.0), content, 1.0),
        Some(Hit::Nothing),
        "a folder has no bytes to take out, so its row acts through its children"
    );
    assert_eq!(
        panel.click((80.0, 76.0), content, 1.0),
        Some(Hit::Extract("<3>report.pdf".to_owned()))
    );

    // And with no collection the same files are a flat list, which is the case every corpus
    // document is in.
    let flat = Content {
        collection: None,
        ..content
    };
    assert_eq!(
        panel.click((80.0, 36.0), flat, 1.0),
        Some(Hit::Extract("<3>report.pdf".to_owned())),
        "a flat list is the /EmbeddedFiles tree's own order"
    );
}

/// §12.3.5.1's `/D`: the row of the initial document is set apart, and an empty tree says so.
///
/// ISO 32000-2 Table 153's `/D` determines "the document that shall be initially presented in the
/// user interface", with three fallbacks stated as `shall`s. The clause states no *appearance*
/// for any of it, so bold is this program's choice — what the test can check is that the four
/// outcomes reach the panel and that only one of them marks a row.
#[test]
fn a_collections_initial_document_is_the_row_set_in_bold() {
    use pdf_model::collection::{Collection, Initial};
    let chrome = Chrome::new().expect("§9.6.2.2's fourteen are compiled in");
    let outline = Outline::default();
    let collection = Collection::default();
    let files = vec![attachment("readme.txt", 12), attachment("report.pdf", 4096)];

    let drawn = |initial: &Initial| {
        let content = Content {
            outline: &outline,
            layers: &[],
            attachments: &files,
            articles: &[],
            collection: Some(viewer_ui::chrome::Presentation {
                collection: &collection,
                initial,
            }),
            information: &NOTHING,
            metadata: None,
            pages: &[],
        };
        let mut panel = Sidebar::default();
        panel.toggle();
        assert_eq!(panel.click((tab(3), 8.0), content, 1.0), Some(Hit::Redraw));
        panel.draw(&chrome, content, HEIGHT, 1.0)
    };

    // The container is what is already on the screen, so no row is marked — the baseline every
    // other case is compared against.
    let plain = drawn(&Initial::Container);
    let (first, second) = (26..46, 46..66);
    let (plain_first, plain_second) = (ink(&plain, first.clone()), ink(&plain, second.clone()));
    assert!(
        plain_first > 40 && plain_second > 40,
        "both files are drawn"
    );

    // "the interactive PDF processor shall select the first item from the list of files to
    // display in its user interface" — which is the first row, and not the second.
    let first_file = drawn(&Initial::FirstFile);
    assert!(
        ink(&first_file, first.clone()) > plain_first,
        "the first row is set apart"
    );
    assert_eq!(
        ink(&first_file, second.clone()),
        plain_second,
        "and the second row is not"
    );

    // A `/D` the tree holds names one row wherever it sits in the list.
    let named = drawn(&Initial::Embedded("report.pdf".to_owned()));
    assert_eq!(
        ink(&named, first.clone()),
        plain_first,
        "the file /D did not name is left alone"
    );
    assert!(
        ink(&named, second.clone()) > plain_second,
        "and the one it named is set apart"
    );

    // "if no files exist in the name tree, the interactive PDF processor shall display an empty
    // preview window" — a sentence rather than a blank panel, which is this file's own habit for
    // a list that is empty because the document said so.
    let empty = Content {
        outline: &outline,
        layers: &[],
        attachments: &[],
        articles: &[],
        collection: Some(viewer_ui::chrome::Presentation {
            collection: &collection,
            initial: &Initial::Empty,
        }),
        information: &NOTHING,
        metadata: None,
        pages: &[],
    };
    let mut panel = Sidebar::default();
    panel.toggle();
    assert_eq!(panel.click((tab(3), 8.0), empty, 1.0), Some(Hit::Redraw));
    let list = panel.draw(&chrome, empty, HEIGHT, 1.0);
    assert!(ink(&list, first) > 40, "the empty tree is said out loud");
}

/// One embedded file, as `Query::Attachments` answers with it.
fn attachment(name: &str, size: i64) -> pdf_model::attachment::Attachment {
    pdf_model::attachment::Attachment {
        name: name.to_owned(),
        file_name: Some(name.rsplit('>').next().unwrap_or(name).to_owned()),
        description: Some("a description the schema hides".to_owned()),
        media_type: None,
        size: Some(size),
        created: None,
        modified: None,
        checksum: None,
        relationship: pdf_model::attachment::Relationship::default(),
        stream: std::sync::Arc::new(pdf_syntax::Stream {
            dict: pdf_syntax::Dictionary::new(),
            data: std::sync::Arc::from(&b""[..]),
            decryption_failed: false,
        }),
    }
}

/// §12.4.3's threads, listed and followed — the sixth tab.
///
/// The clause states the structure and makes the *way in* a permission: "[i]nteractive PDF
/// processors may provide navigation facilities to allow the user to follow a thread from one bead
/// to the next". So the panel is this host's answer to that permission, and what it owes is a row
/// per thread and a click that means follow it.
///
/// **Not one of the 974 corpus documents states an article thread**, which is why the fixture is
/// written here: trap 8's converse, and the same position §12.7.4.3's comb and password fixtures
/// are in.
#[test]
fn the_read_tab_lists_a_thread_and_a_click_follows_it() {
    let chrome = Chrome::new().expect("§9.6.2.2's fourteen are compiled in");
    let outline = Outline::default();
    let empty = Content {
        outline: &outline,
        layers: &[],
        attachments: &[],
        articles: &[],
        collection: None,
        information: &NOTHING,
        metadata: None,
        pages: &[],
    };
    let mut panel = Sidebar::default();
    panel.toggle();
    assert_eq!(
        panel.click((tab(4), 8.0), empty, 1.0),
        Some(Hit::Redraw),
        "the Read tab"
    );
    assert!(
        ink(&panel.draw(&chrome, empty, HEIGHT, 1.0), 26..46) > 40,
        "an empty list must say it is empty"
    );

    let threads = vec![
        pdf_model::article::Thread {
            id: ObjectId::new(7, 0),
            title: Some("The leading story".to_owned()),
            beads: vec![bead(11), bead(12), bead(13)],
        },
        // A thread with no `/I` is still a thread: Table 158 makes the information dictionary
        // optional, so the row falls back to the clause's own noun and the array's order.
        pdf_model::article::Thread {
            id: ObjectId::new(8, 0),
            title: None,
            beads: vec![bead(14)],
        },
    ];
    let listed = Content {
        outline: &outline,
        layers: &[],
        attachments: &[],
        articles: &threads,
        collection: None,
        information: &NOTHING,
        metadata: None,
        pages: &[],
    };
    assert!(
        ink(&panel.draw(&chrome, listed, HEIGHT, 1.0), 26..46) > 40,
        "the first thread's title is drawn"
    );
    assert!(
        ink(&panel.draw(&chrome, listed, HEIGHT, 1.0), 46..66) > 40,
        "and the untitled one below it is a row too"
    );
    // The same message §12.3.3's outline sends: the object, not a destination. What activating a
    // thread means is the *document*'s to decide, and `viewer_core::interact` composes §12.6.4.7's
    // own thread action out of it.
    assert_eq!(
        panel.click((80.0, 36.0), listed, 1.0),
        Some(Hit::Activate(ObjectId::new(7, 0)))
    );
    assert_eq!(
        panel.click((80.0, 56.0), listed, 1.0),
        Some(Hit::Activate(ObjectId::new(8, 0)))
    );
}

/// A bead with an object number and nothing else the panel reads.
fn bead(number: u32) -> pdf_model::article::Bead {
    pdf_model::article::Bead {
        id: ObjectId::new(number, 0),
        page: None,
        rect: None,
    }
}

/// A corpus document's bytes, or `None` when the submodule is not checked out.
fn corpus_bytes(name: &str) -> Option<Vec<u8>> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../doc/pdf.js/test/pdfs")
        .join(name);
    std::fs::read(path).ok()
}

/// The About card draws `/NOTICE` verbatim, in a face whose columns line up, and scrolls.
///
/// Both licences covering the compiled-in standard 14 fonts require a *binary* distribution to
/// reproduce their notices; `--licences` prints them and `tests/notices.rs` checks the file's
/// content, so what is left to check here is that the card **shows** it — a licence obligation
/// with a surface nobody can read would be worse than the flag alone.
///
/// The fixed pitch is checked rather than assumed: §9.6.2.2's Courier advances every code 600
/// thousandths of an em, and a proportional face would turn the notice's aligned columns into
/// ragged prose.
#[test]
fn the_about_card_shows_the_notice_and_scrolls_it() {
    let chrome = Chrome::new().expect("§9.6.2.2's fourteen are compiled in");
    assert!(
        (chrome.mono_advance(10.0) - 6.0).abs() < 0.01,
        "the notice's face is not fixed at 600 thousandths: {}",
        chrome.mono_advance(10.0)
    );

    let notice = include_str!("../../../NOTICE");
    let mut about = About::default();
    let hidden = about.draw(&chrome, notice, WIDTH, HEIGHT, 1.0);
    assert_eq!(ink(&hidden, 0..HEIGHT), 0, "a hidden card draws nothing");

    about.toggle();
    let shown = about.draw(&chrome, notice, WIDTH, HEIGHT, 1.0);
    let top = ink(&shown, 30..120);
    assert!(top > 200, "the card's first lines have no glyphs: {top}");

    // Scrolling moves the text and nothing else: the same band of the card shows different ink
    // afterwards. A card that ignored the scroll would give the identical count.
    about.scroll(400.0, notice, HEIGHT, 1.0);
    let scrolled = about.draw(&chrome, notice, WIDTH, HEIGHT, 1.0);
    assert_ne!(ink(&scrolled, 30..120), top, "the notice did not move");

    // And it cannot be scrolled past its own end: this window is 300 pixels tall and the notice
    // is 123 lines, so the clamp is what stops the card going blank.
    about.scroll(1_000_000.0, notice, HEIGHT, 1.0);
    let far = about.draw(&chrome, notice, WIDTH, HEIGHT, 1.0);
    assert!(
        ink(&far, 30..120) > 200,
        "scrolled past the end and the card is empty"
    );
}

/// §14.3.3's tab shows what the document says about itself, in both places it says it.
///
/// Table 349's every text entry carries a NOTE pointing at an XMP counterpart and §12.2's
/// `/DisplayDocTitle` names `dc:title` outright, so a document with a metadata stream may be
/// saying something else about itself than the dictionary does. **Since the
/// two-hundred-and-ninety-fourth session the panel shows both rather than naming the second**
/// (ADR 0186), and the two are kept apart on the screen for the reason the standard keeps them
/// apart: nothing ranks them except §12.2, and only for the title.
#[test]
fn the_document_tab_shows_table_349_and_the_xmp_beside_it() {
    let chrome = Chrome::new().expect("§9.6.2.2's fourteen are compiled in");
    let outline = Outline::default();
    let information = pdf_model::metadata::Information {
        title: Some("Annual report".to_owned()),
        producer: Some("An exporter".to_owned()),
        created: Some("D:20140314124211+01'00'".to_owned()),
        ..pdf_model::metadata::Information::default()
    };
    let stated = Content {
        outline: &outline,
        layers: &[],
        attachments: &[],
        articles: &[],
        collection: None,
        information: &information,
        metadata: None,
        pages: &[],
    };
    let mut panel = Sidebar::default();
    panel.toggle();
    // The fourth of four tabs.
    assert_eq!(
        panel.click((tab(5), 8.0), stated, 1.0),
        Some(Hit::Redraw),
        "the About tab"
    );
    // Three stated entries, three rows, and a fourth row only where there is XMP to name.
    for (row, band) in [(0, 26..46), (1, 46..66), (2, 66..86)] {
        assert!(
            ink(&panel.draw(&chrome, stated, HEIGHT, 1.0), band) > 20,
            "row {row}"
        );
    }
    assert_eq!(
        ink(&panel.draw(&chrome, stated, HEIGHT, 1.0), 86..106),
        0,
        "a fourth row with nothing to say"
    );

    let packet = pdf_model::xmp::Xmp::parse(
        br#"<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
              <rdf:Description rdf:about="" xmlns:pdf="http://ns.adobe.com/pdf/1.3/"
                               pdf:Producer="Another exporter"/></rdf:RDF>"#,
    );
    let with_xmp = Content {
        metadata: Some(&packet),
        pages: &[],
        ..stated
    };
    assert!(
        ink(&panel.draw(&chrome, with_xmp, HEIGHT, 1.0), 86..126) > 40,
        "a document carrying XMP must have it shown, under its own heading"
    );

    // A stream this reader refused is a different sentence, and it is about us rather than
    // about the document — so it is drawn where a document that stated nothing draws nothing.
    let refused: Result<pdf_model::xmp::Xmp, _> = pdf_model::xmp::Xmp::parse(b"<a></b>");
    assert!(refused.is_err(), "the fixture is unbalanced XML");
    let broken = Content {
        metadata: Some(&refused),
        pages: &[],
        ..stated
    };
    assert!(
        ink(&panel.draw(&chrome, broken, HEIGHT, 1.0), 86..106) > 40,
        "a stream that would not read must say so"
    );

    // A document that states nothing says so, rather than showing an empty list.
    let silent = Content {
        information: &NOTHING,
        metadata: None,
        pages: &[],
        ..stated
    };
    assert!(ink(&panel.draw(&chrome, silent, HEIGHT, 1.0), 26..46) > 40);
}

/// §12.3.4's tab draws the miniature the page states, and a click on it shows that page.
///
/// > A PDF document may contain thumbnail images representing the contents of its pages in
/// > miniature form.
///
/// The clause states no size for one, no placement and no list — those are this program's, and
/// what the assertion pins is that the *image* reaches the display list rather than only its
/// label: the ink is counted inside the picture box, above the row's own line of text, and a
/// page with no `/Thumb` is still a row because §12.3.4's NOTE says thumbnails "are not
/// required, and can be included for some pages and not for others".
#[test]
fn the_pages_tab_draws_a_thumbnail_and_a_click_goes_to_its_page() {
    let chrome = Chrome::new().expect("§9.6.2.2's fourteen are compiled in");
    let outline = Outline::default();
    // A four-sample chequer stands in for a page's miniature: what is under test is that an
    // image reaches the list at all, and a fixture whose samples are known makes the ink exact.
    let image = pdf_render::Image {
        width: 2,
        height: 2,
        data: std::sync::Arc::from(
            [
                0, 0, 0, 255, 255, 255, 255, 255, 255, 255, 255, 255, 0, 0, 0, 255,
            ]
            .as_slice(),
        ),
        interpolate: false,
    };
    let pages = [
        viewer_ui::chrome::Page {
            label: "i".to_owned(),
            thumbnail: Some(image),
        },
        viewer_ui::chrome::Page {
            label: "ii".to_owned(),
            thumbnail: None,
        },
    ];
    let content = Content {
        outline: &outline,
        layers: &[],
        attachments: &[],
        articles: &[],
        collection: None,
        information: &NOTHING,
        metadata: None,
        pages: &pages,
    };
    let mut panel = Sidebar::default();
    panel.toggle();
    assert_eq!(
        panel.click((tab(1), 8.0), content, 1.0),
        Some(Hit::Redraw),
        "the Pages tab"
    );

    // The first row is seven row heights tall — the picture, then its label on the last line —
    // so the miniature is in the rows above that line and the second page's row is below it.
    let drawn = panel.draw(&chrome, content, HEIGHT, 1.0);
    assert!(
        ink(&drawn, 30..130) > 200,
        "the miniature is drawn, not only its label"
    );

    // A click on the first row shows the first page, and one on the second shows the second.
    assert_eq!(panel.click((150.0, 60.0), content, 1.0), Some(Hit::GoTo(0)));
    assert_eq!(
        panel.click((150.0, 170.0), content, 1.0),
        Some(Hit::GoTo(1))
    );
}

/// A popup window the core would answer with, at a rectangle in the pretend window's pixels.
fn window(text: &str, title: Option<&str>) -> viewer_core::PopupWindow {
    viewer_core::PopupWindow {
        annotation: ObjectId::new(7, 0),
        parent: Some(ObjectId::new(6, 0)),
        // 200 × 120 at (40, 30), clockwise from the top-left, y downwards.
        quad: [40.0, 30.0, 240.0, 30.0, 240.0, 150.0, 40.0, 150.0],
        title: title.map(str::to_owned),
        text: Some(text.to_owned()),
        modified: Some("D:20260805120000Z".to_owned()),
        colour: None,
    }
}

#[test]
fn a_popup_window_puts_its_note_on_the_page() {
    // §12.5.6.14's window is chrome, so no gate in this tree can see it — the same argument the
    // panel's own tests make, one clause over. Ink inside the window's body and none outside it.
    let chrome = Chrome::new().expect("§9.6.2.2's fourteen are compiled in");
    let windows = [window(
        "A note whose words this face can set.",
        Some("author"),
    )];
    let list = viewer_ui::chrome::popup_windows(&chrome, &windows, WIDTH, HEIGHT, 1.0)
        .expect("one window is drawn");
    // The title bar is rows 30 to about 50; the body is under it.
    assert!(ink(&list, 55..145) > 100, "the note's words are drawn");
    assert!(ink(&list, 32..48) > 20, "and its title bar carries the /T");
    assert_eq!(ink(&list, 160..HEIGHT), 0, "and nothing below the window");

    assert!(
        viewer_ui::chrome::popup_windows(&chrome, &[], WIDTH, HEIGHT, 1.0).is_none(),
        "no open window is no display list at all"
    );
}

#[test]
fn a_note_this_interfaces_font_cannot_set_says_so() {
    // Six of the corpus's seven open popups are in Chinese and §9.6.2.2's Helvetica states no
    // code for one character of it. Drawing a blank window would be this program telling a
    // person the note is empty (trap 5).
    let chrome = Chrome::new().expect("§9.6.2.2's fourteen are compiled in");
    let windows = [window("多边形批注", None)];
    let list = viewer_ui::chrome::popup_windows(&chrome, &windows, WIDTH, HEIGHT, 1.0)
        .expect("one window is drawn");
    assert!(
        ink(&list, 55..145) > 50,
        "the sentence about what could not be set is drawn"
    );
}
