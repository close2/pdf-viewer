//! `pages`: RFC 0002 section 6.2's single-document verb, held to the clauses each operation
//! rests on.
//!
//! Every expected value below is the standard's: §7.7.3.3's `/Rotate` — "[t]he number of degrees
//! by which the page shall be rotated clockwise when displayed or printed. The value shall be a
//! multiple of 90" — composed against §7.7.3.4's inheritance; §12.4.2's labels, which follow the
//! page rather than the position; §12.3.2.2's destination, which is "an indirect reference to a
//! page object" and therefore follows its page through any permutation and becomes §7.3.10's
//! null when the page leaves; Table 31's one `/Parent`, which is why a page in two places is two
//! page objects; and §12.7.4.2's fully qualified field name, which is why a page carrying a
//! widget is not duplicated at all.
//!
//! The documents are the committed ones — every checkout has them once `doc/specifications.zip`
//! is unpacked (trap 4: a fixture built here would only exercise what this tree already writes).
//!
//! `qpdf --check`, where it is installed, is **evidence about the reading and never its
//! definition** (principle 5).

#![expect(
    clippy::expect_used,
    clippy::print_stderr,
    reason = "test code: a fixture that cannot exercise the rule must fail loudly, and a \
              skipped test says so"
)]

mod support;

use std::collections::BTreeSet;
use std::process::Command;

use pdf_model::Pages;
use pdf_model::page_label::PageLabels;
use pdf_syntax::object::ObjectId;
use pdf_syntax::{Document, Limits, Object};
use pdf_transform::pages::{Angle, Edit, PagesPlan};
use pdf_transform::range::Selection;
use pdf_transform::render::{ImageFormat, RenderPlan, Sizing};
use pdf_transform::{Budget, MemorySinks, Origin, Plan, Policy, Refusal, Report, Source, apply};

use support::{committed, corpus};

/// Five pages, an outline, `/PageLabels` and an `/AcroForm` — the fixture `merge`'s tests use.
const FIVE_PAGES: &str = "PDF20_AN001-BPC.pdf";

/// Fourteen pages, an outline and a `/Names` `/Dests` tree of clause anchors.
const FOURTEEN_PAGES: &str = "ISO_TS_32001-2022_sponsored_EC3.pdf";

/// Applies these edits to one document, answering the report and the file.
fn edit(bytes: &[u8], edits: Vec<Edit>) -> Result<(Report, Vec<u8>), Refusal> {
    let sinks = MemorySinks::new();
    let report = apply(
        &Plan::Pages(PagesPlan {
            source: 0,
            edits,
            names: "edited.pdf".parse().expect("a pattern"),
        }),
        &[Source::new(bytes.to_vec())],
        &sinks,
        &Policy::default(),
        &Budget::default(),
    )?;
    let mut outputs = sinks.into_outputs();
    assert_eq!(outputs.len(), 1, "pages writes one file");
    Ok((report, outputs.remove(0).1))
}

/// A range, parsed.
fn range(text: &str) -> Selection {
    text.parse().expect("a selection")
}

/// One page of `bytes` as a PPM, or `None` where nothing was drawn.
fn draw(bytes: &[u8], page: usize) -> Option<Vec<u8>> {
    let sinks = MemorySinks::new();
    apply(
        &Plan::Render(RenderPlan {
            source: 0,
            pages: range(&page.to_string()),
            size: Sizing::Dpi(72.0),
            format: ImageFormat::Ppm,
            page_box: None,
            annotations: true,
            names: "page.ppm".parse().expect("a pattern"),
        }),
        &[Source::new(bytes.to_vec())],
        &sinks,
        &Policy::default(),
        &Budget::default(),
    )
    .ok()?;
    let mut outputs = sinks.into_outputs();
    (!outputs.is_empty()).then(|| outputs.remove(0).1)
}

/// A page's effective §7.7.3.3 `/Rotate`: its own, else §7.7.3.4's from an ancestor, else 0.
fn rotation(document: &Document, index: usize) -> i64 {
    let Some(page) = Pages::new(document).get(index) else {
        return 0;
    };
    let mut dict = page.dict;
    for _ in 0..64 {
        let stated = document.get_key(&dict, "Rotate");
        if let Some(degrees) = stated.as_integer() {
            return degrees;
        }
        let parent = document.get_key(&dict, "Parent");
        let Some(next) = parent.as_dict() else {
            return 0;
        };
        dict = next.clone();
    }
    0
}

/// `qpdf --check` on these bytes, where qpdf is installed: `Some(accepted)`.
fn qpdf_accepts(bytes: &[u8]) -> Option<bool> {
    let directory =
        std::env::temp_dir().join(format!("pdfv-pages-{}-{}", std::process::id(), bytes.len()));
    std::fs::create_dir_all(&directory).ok()?;
    let path = directory.join("edited.pdf");
    std::fs::write(&path, bytes).ok()?;
    let output = Command::new("qpdf").arg("--check").arg(&path).output().ok();
    let _ = std::fs::remove_dir_all(&directory);
    Some(output?.status.success())
}

/// One committed document read, or the test skipped where the archive is not unpacked.
fn source(name: &str) -> Option<(Vec<u8>, Document)> {
    let bytes = std::fs::read(committed(name)).ok()?;
    let document = Document::open_with_limits(bytes.clone(), Limits::DEFAULT).ok()?;
    Some((bytes, document))
}

#[test]
fn a_deleted_page_is_gone_and_the_others_draw_as_they_did() {
    let Some((bytes, before)) = source(FIVE_PAGES) else {
        eprintln!("skipped: {FIVE_PAGES} is not unpacked");
        return;
    };
    let count = Pages::new(&before).len();
    let (report, out) = edit(&bytes, vec![Edit::Delete(range("2"))]).expect("the edit");
    let after = Document::open_with_limits(out.clone(), Limits::DEFAULT).expect("re-read");
    assert_eq!(
        Pages::new(&after).len(),
        count - 1,
        "one page out of {count}"
    );
    assert!(
        matches!(report.outputs.first().map(|output| &output.origin),
                 Some(Origin::Edited { pages, .. }) if *pages == count - 1),
        "the report counts the pages it wrote"
    );
    // RFC 0002 section 9's layer 3: page 2 of the output is source page 3, bit for bit.
    assert_eq!(
        draw(&out, 2),
        draw(&bytes, 3),
        "the page after the deleted one draws as it did"
    );
}

#[test]
fn a_rotation_is_the_clauses_multiple_of_90_and_the_others_are_untouched() {
    let Some((bytes, before)) = source(FIVE_PAGES) else {
        eprintln!("skipped: {FIVE_PAGES} is not unpacked");
        return;
    };
    let was = rotation(&before, 0);
    let (_, out) = edit(
        &bytes,
        vec![Edit::Rotate {
            angle: Angle::Relative(90),
            pages: range("1"),
        }],
    )
    .expect("the edit");
    let after = Document::open_with_limits(out, Limits::DEFAULT).expect("re-read");
    // §7.7.3.4 gives the page its effective value and §7.7.3.3 makes a rotation clockwise
    // degrees, so a quarter turn from `was` is `was + 90` reduced to one turn.
    assert_eq!(
        rotation(&after, 0),
        (was + 90).rem_euclid(360),
        "a relative quarter turn composes with the effective rotation"
    );
    assert_eq!(
        rotation(&after, 1),
        rotation(&before, 1),
        "a page outside the range keeps what it had"
    );
}

#[test]
fn an_absolute_rotation_replaces_what_the_page_states() {
    let Some((bytes, _)) = source(FIVE_PAGES) else {
        eprintln!("skipped: {FIVE_PAGES} is not unpacked");
        return;
    };
    // Two rotations of the same page: the second is absolute and says what the page is, not
    // what it turns by. RFC 0002 section 6.2's composition is left to right.
    let (_, out) = edit(
        &bytes,
        vec![
            Edit::Rotate {
                angle: Angle::Relative(90),
                pages: range("1"),
            },
            Edit::Rotate {
                angle: Angle::Absolute(180),
                pages: range("1"),
            },
        ],
    )
    .expect("the edit");
    let after = Document::open_with_limits(out, Limits::DEFAULT).expect("re-read");
    assert_eq!(rotation(&after, 0), 180, "the absolute angle is the answer");
}

#[test]
fn a_relative_rotation_composes_with_the_one_before_it() {
    let Some((bytes, before)) = source(FIVE_PAGES) else {
        eprintln!("skipped: {FIVE_PAGES} is not unpacked");
        return;
    };
    let was = rotation(&before, 0);
    let (_, out) = edit(
        &bytes,
        vec![
            Edit::Rotate {
                angle: Angle::Relative(90),
                pages: range("1"),
            },
            Edit::Rotate {
                angle: Angle::Relative(90),
                pages: range("1"),
            },
        ],
    )
    .expect("the edit");
    let after = Document::open_with_limits(out, Limits::DEFAULT).expect("re-read");
    assert_eq!(rotation(&after, 0), (was + 180).rem_euclid(360));
}

#[test]
fn a_negative_rotation_is_reduced_into_one_turn() {
    let Some((bytes, before)) = source(FIVE_PAGES) else {
        eprintln!("skipped: {FIVE_PAGES} is not unpacked");
        return;
    };
    let was = rotation(&before, 0);
    let (_, out) = edit(
        &bytes,
        vec![Edit::Rotate {
            angle: Angle::Relative(-90),
            pages: range("1"),
        }],
    )
    .expect("the edit");
    let after = Document::open_with_limits(out, Limits::DEFAULT).expect("re-read");
    let written = rotation(&after, 0);
    assert_eq!(written, (was - 90).rem_euclid(360));
    assert!(
        (0..360).contains(&written) && written % 90 == 0,
        "§7.7.3.3: a multiple of 90, and this writer's choice of the smallest non-negative one"
    );
}

#[test]
fn an_angle_that_is_not_a_multiple_of_90_is_refused_by_name() {
    let Some((bytes, _)) = source(FIVE_PAGES) else {
        eprintln!("skipped: {FIVE_PAGES} is not unpacked");
        return;
    };
    let refusal = edit(
        &bytes,
        vec![Edit::Rotate {
            angle: Angle::Absolute(45),
            pages: range("1"),
        }],
    )
    .expect_err("§7.7.3.3 has no way to write 45");
    assert!(
        matches!(refusal, Refusal::Rotation { degrees: 45 }),
        "{refusal}"
    );
}

#[test]
fn a_reorder_keeps_each_pages_own_label() {
    let Some((bytes, before)) = source(FIVE_PAGES) else {
        eprintln!("skipped: {FIVE_PAGES} is not unpacked");
        return;
    };
    let labels = PageLabels::read(&before);
    if labels.is_empty() {
        eprintln!("skipped: {FIVE_PAGES} states no /PageLabels");
        return;
    }
    let third = labels.label(2);
    // Page 3 to the front.
    let (_, out) = edit(
        &bytes,
        vec![Edit::Move {
            pages: range("3"),
            to: 1,
        }],
    )
    .expect("the edit");
    let after = Document::open_with_limits(out.clone(), Limits::DEFAULT).expect("re-read");
    assert_eq!(
        PageLabels::read(&after).label(0),
        third,
        "§12.4.2 numbers by position, so the moved page's label is written at its new index"
    );
    assert_eq!(
        draw(&out, 1),
        draw(&bytes, 3),
        "the moved page draws where it landed"
    );
}

#[test]
fn a_moved_page_leaves_the_others_in_order() {
    let Some((bytes, before)) = source(FOURTEEN_PAGES) else {
        eprintln!("skipped: {FOURTEEN_PAGES} is not unpacked");
        return;
    };
    let count = Pages::new(&before).len();
    let (_, out) = edit(
        &bytes,
        vec![Edit::Move {
            pages: range("1"),
            to: 3,
        }],
    )
    .expect("the edit");
    let after = Document::open_with_limits(out.clone(), Limits::DEFAULT).expect("re-read");
    assert_eq!(Pages::new(&after).len(), count, "a move loses no page");
    // Removing page 1 and putting it before what was page 3 leaves 2, 1, 3, 4 …
    assert_eq!(draw(&out, 1), draw(&bytes, 2));
    assert_eq!(draw(&out, 2), draw(&bytes, 1));
    assert_eq!(draw(&out, 3), draw(&bytes, 3));
}

#[test]
fn an_inserted_page_is_a_second_page_object_that_draws_the_same() {
    let Some((bytes, before)) = source(FOURTEEN_PAGES) else {
        eprintln!("skipped: {FOURTEEN_PAGES} is not unpacked");
        return;
    };
    let count = Pages::new(&before).len();
    let (_, out) = edit(
        &bytes,
        vec![Edit::Insert {
            pages: range("2"),
            at: 1,
        }],
    )
    .expect("the edit");
    let after = Document::open_with_limits(out.clone(), Limits::DEFAULT).expect("re-read");
    let pages = Pages::new(&after);
    assert_eq!(pages.len(), count + 1, "one page more");
    // Table 31 gives a page one `/Parent`, so the two places are two objects.
    let first = pages.get(0).and_then(|page| page.id);
    let second = pages.get(2).and_then(|page| page.id);
    assert!(first.is_some() && first != second, "two page objects");
    assert_eq!(
        draw(&out, 1),
        draw(&bytes, 2),
        "the copy draws as its original does"
    );
    assert_eq!(draw(&out, 3), draw(&bytes, 2), "and so does the original");
}

#[test]
fn duplicating_a_page_that_carries_a_widget_is_refused_by_name() {
    // §12.7.4.2 makes a field's fully qualified name its identity, so a widget on two pages is
    // a form edited rather than a page duplicated. The fixture is whichever committed document
    // states a widget; where none does, the rule is stated and skipped rather than faked.
    // No committed document states a widget, measured; the corpus's do, and
    // `annotation-text-widget.pdf` is one whose name says what it holds.
    for name in ["annotation-text-widget.pdf", "issue15096.pdf"] {
        let Some((bytes, document)) = from_corpus(name) else {
            continue;
        };
        let pages = Pages::new(&document);
        let Some(index) = (0..pages.len()).find(|index| has_widget(&document, &pages, *index))
        else {
            continue;
        };
        let refusal = edit(
            &bytes,
            vec![Edit::Insert {
                pages: range(&(index + 1).to_string()),
                at: 1,
            }],
        )
        .expect_err("a widget is not duplicated");
        assert!(
            matches!(refusal, Refusal::DuplicateWidget { .. }),
            "{refusal}"
        );
        return;
    }
    eprintln!("skipped: the corpus submodule holds no widget document here");
}

/// A corpus document read, or `None` where the submodule is not checked out.
fn from_corpus(name: &str) -> Option<(Vec<u8>, Document)> {
    let bytes = std::fs::read(corpus(name)?).ok()?;
    let document = Document::open_with_limits(bytes.clone(), Limits::DEFAULT).ok()?;
    Some((bytes, document))
}

/// Whether a page's `/Annots` holds a widget — the test's own walk, not the crate's.
fn has_widget(document: &Document, pages: &Pages<'_>, index: usize) -> bool {
    let Some(page) = pages.get(index) else {
        return false;
    };
    let annots = document.get_key(&page.dict, "Annots");
    let Some(items) = annots.as_array() else {
        return false;
    };
    items.iter().any(|item| {
        document
            .resolve(item)
            .as_dict()
            .map(|dict| document.get_key(dict, "Subtype"))
            .and_then(|value| {
                value
                    .as_name()
                    .and_then(|name| name.as_str().map(str::to_owned))
            })
            .as_deref()
            == Some("Widget")
    })
}

#[test]
fn a_position_the_list_does_not_have_is_a_usage_refusal() {
    let Some((bytes, before)) = source(FIVE_PAGES) else {
        eprintln!("skipped: {FIVE_PAGES} is not unpacked");
        return;
    };
    let count = Pages::new(&before).len();
    let refusal = edit(
        &bytes,
        vec![Edit::Move {
            pages: range("1"),
            to: count + 2,
        }],
    )
    .expect_err("one past the end appends; two past it names nothing");
    assert!(matches!(refusal, Refusal::Position { .. }), "{refusal}");
    // One past the end is the append, and it is not an error.
    edit(
        &bytes,
        vec![Edit::Move {
            pages: range("1"),
            to: count + 1,
        }],
    )
    .expect("appending is a move to one past the end");
}

#[test]
fn deleting_every_page_is_refused_rather_than_written() {
    let Some((bytes, _)) = source(FIVE_PAGES) else {
        eprintln!("skipped: {FIVE_PAGES} is not unpacked");
        return;
    };
    let refusal = edit(&bytes, vec![Edit::Delete(range("1-end"))])
        .expect_err("§7.7.3.2's /Kids has children");
    assert!(matches!(refusal, Refusal::Assembly(_)), "{refusal}");
}

#[test]
fn the_edits_compose_left_to_right_over_the_running_list() {
    let Some((bytes, before)) = source(FOURTEEN_PAGES) else {
        eprintln!("skipped: {FOURTEEN_PAGES} is not unpacked");
        return;
    };
    let count = Pages::new(&before).len();
    // RFC 0002 section 6.2: each range is read against the list the edits before it left. So
    // deleting position 1 twice takes out source pages 1 and 2.
    let (_, out) = edit(
        &bytes,
        vec![Edit::Delete(range("1")), Edit::Delete(range("1"))],
    )
    .expect("the edit");
    let after = Document::open_with_limits(out.clone(), Limits::DEFAULT).expect("re-read");
    assert_eq!(Pages::new(&after).len(), count - 2);
    assert_eq!(
        draw(&out, 1),
        draw(&bytes, 3),
        "the first two pages are gone, not the first and the third"
    );
}

#[test]
fn a_structure_tree_is_carried_and_names_only_pages_the_output_holds() {
    let Some((bytes, document)) = source("Well-Tagged-PDF-WTPDF-1.0.pdf") else {
        eprintln!("skipped: the tagged fixture is not unpacked");
        return;
    };
    let catalog = document.catalog().expect("a catalog");
    if document.get_key(&catalog, "StructTreeRoot").is_null() {
        eprintln!("skipped: the fixture states no /StructTreeRoot");
        return;
    }
    let (report, out) = edit(&bytes, vec![Edit::Delete(range("r1"))]).expect("the edit");
    assert!(
        report.warnings.iter().any(|warning| warning
            .detail
            .contains("§14.7: the structure tree is carried")),
        "the report says what the carry wrote and what it dropped: {:?}",
        report.warnings
    );
    let after = Document::open_with_limits(out, Limits::DEFAULT).expect("re-read");
    let out_catalog = after.catalog().expect("a catalog");
    let root = after.get_key(&out_catalog, "StructTreeRoot");
    let root = root.as_dict().expect("§14.7.2's structure tree root");
    assert_eq!(
        after
            .get_key(root, "Type")
            .as_name()
            .map(|name| name.as_bytes().to_vec()),
        Some(b"StructTreeRoot".to_vec()),
        "Table 354 makes /Type required and \"shall be StructTreeRoot\""
    );
    // Every carried element's `/Pg` names a page the output holds — the property that
    // distinguishes a pruned tree from ADR 0831 section 2's half-carried one, which would have pointed
    // a page's marked content at another page's structure element.
    let held: BTreeSet<ObjectId> = Pages::new(&after).indices().into_keys().collect();
    let mut checked = 0_usize;
    for number in after.xref().object_numbers() {
        let id = ObjectId::new(number, 0);
        let Object::Dictionary(dict) = after.get(id) else {
            continue;
        };
        if dict.get("S").is_none() || dict.get("P").is_none() {
            continue;
        }
        let Some(page) = dict.get("Pg").and_then(Object::as_reference) else {
            continue;
        };
        assert!(
            held.contains(&page),
            "object {number}'s /Pg names a page this document does not hold"
        );
        checked = checked.saturating_add(1);
    }
    assert!(checked > 0, "the carried tree has elements with a /Pg");
}

#[test]
fn the_same_edit_twice_writes_the_same_bytes() {
    let Some((bytes, _)) = source(FIVE_PAGES) else {
        eprintln!("skipped: {FIVE_PAGES} is not unpacked");
        return;
    };
    let edits = || {
        vec![
            Edit::Rotate {
                angle: Angle::Relative(90),
                pages: range("1-end:odd"),
            },
            Edit::Delete(range("r1")),
        ]
    };
    let (_, first) = edit(&bytes, edits()).expect("the edit");
    let (_, second) = edit(&bytes, edits()).expect("the edit again");
    assert_eq!(first, second, "RFC 0002 section 9's first layer");
}

#[test]
fn qpdf_accepts_what_this_verb_writes() {
    let Some((bytes, _)) = source(FIVE_PAGES) else {
        eprintln!("skipped: {FIVE_PAGES} is not unpacked");
        return;
    };
    let (_, out) = edit(
        &bytes,
        vec![
            Edit::Rotate {
                angle: Angle::Relative(90),
                pages: range("1"),
            },
            Edit::Delete(range("r1")),
        ],
    )
    .expect("the edit");
    match qpdf_accepts(&out) {
        Some(true) => {}
        Some(false) => panic!("qpdf --check refused the edited document"),
        None => eprintln!("skipped: qpdf is not installed"),
    }
}

#[test]
fn a_destination_to_a_deleted_page_is_the_clauses_null() {
    let Some((bytes, before)) = source(FOURTEEN_PAGES) else {
        eprintln!("skipped: {FOURTEEN_PAGES} is not unpacked");
        return;
    };
    let count = Pages::new(&before).len();
    // Every page but the first leaves, so every destination that named one of them has nothing
    // to name. §12.3.2.2 makes a destination "an indirect reference to a page object" and
    // §7.3.10 makes a reference to an object the file does not hold a null.
    let (report, out) =
        edit(&bytes, vec![Edit::Delete(range(&format!("2-{count}")))]).expect("the edit");
    assert!(
        report
            .warnings
            .iter()
            .any(|warning| warning.detail.contains("null")),
        "the dropped references are counted: {:?}",
        report.warnings
    );
    let after = Document::open_with_limits(out, Limits::DEFAULT).expect("re-read");
    assert_eq!(Pages::new(&after).len(), 1);
    // Nothing in the output names a page it does not hold: every destination either reaches the
    // one page or is null.
    let outline = after
        .catalog()
        .ok()
        .map(|catalog| after.get_key(&catalog, "Outlines"));
    assert!(
        outline.is_some(),
        "the outline is carried even where its destinations are not"
    );
}

#[test]
fn a_carried_page_states_the_key_of_its_own_entry_in_the_output_s_parent_tree() {
    // §14.7.5.4's whole point, asserted: "[t]he key for each entry shall be an integer given as
    // the value of the StructParent or StructParents entry in the object". The key is the
    // *output's*, so the producer's number does not cross — what crosses is the property that
    // the number resolves.
    let Some((bytes, document)) = source("Well-Tagged-PDF-WTPDF-1.0.pdf") else {
        eprintln!("skipped: the tagged fixture is not unpacked");
        return;
    };
    let pages = Pages::new(&document);
    let Some(page) = pages.get(0) else {
        return;
    };
    if document
        .get_key(&page.dict, "StructParents")
        .as_integer()
        .is_none()
    {
        eprintln!("skipped: page 1 states no /StructParents");
        return;
    }
    let (_, out) = edit(&bytes, vec![Edit::Delete(range("r1"))]).expect("the edit");
    let after = Document::open_with_limits(out, Limits::DEFAULT).expect("re-read");
    let carried = Pages::new(&after).get(0).expect("a first page").dict;
    let key = after
        .get_key(&carried, "StructParents")
        .as_integer()
        .expect("the carried page states a key");
    let catalog = after.catalog().expect("a catalog");
    let root = after.get_key(&catalog, "StructTreeRoot");
    let root = root.as_dict().expect("a structure tree root");
    let tree = after.get_key(root, "ParentTree");
    let tree = tree.as_dict().expect("§14.7.5.4's parent tree");
    let entry = pdf_syntax::tree::lookup_unresolved(
        tree,
        &pdf_syntax::tree::TreeKey::Number(key),
        &|value| after.resolve(value),
    )
    .expect("the page's key resolves in the output's own parent tree");
    // "For a content stream containing marked-content sequences that are content items, the
    // value shall be an array of indirect references to the sequences' parent structure
    // elements."
    let Object::Array(items) = entry else {
        panic!("a page's parent-tree value is an array: {entry:?}");
    };
    assert!(
        items.iter().any(|item| item.as_reference().is_some()),
        "and the array names structure elements: {items:?}"
    );
    // §14.7.5.4: "The ParentTreeNextKey entry in the structure tree root shall hold an integer
    // value greater than any that is currently in use as a key in the structural parent tree."
    let next = after
        .get_key(root, "ParentTreeNextKey")
        .as_integer()
        .expect("Table 354's /ParentTreeNextKey");
    assert!(
        next > key,
        "{next} is not greater than the key {key} in use"
    );
}
