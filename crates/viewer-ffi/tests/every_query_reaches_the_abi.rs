//! Every `viewer_core::Query` variant against the entry points that answer it.
//!
//! **This is the instrument ADR 0509 asked for and ADR 0576 built**, and the defect it exists
//! against is one this crate actually had: `Query::Find`, `Query::Opening`, `Query::Preferences`
//! and eight more reached **no symbol at all**, for as long as three hundred sessions in one case,
//! and nothing anywhere said so. The ABI's own protection is `PDFV_EVENT_KIND_COUNT`, which is the
//! right shape for a message that *arrives* — a caller checks the number at startup and refuses —
//! and no shape at all for a *question*: a `Query` added after the last sweep leaves a C caller
//! with no symbol and no signal, which is exactly how eleven accumulated.
//!
//! What replaces it is the mechanism the rest of this crate uses one directory over. Every other
//! host on this boundary is protected by `viewer-core`'s enums being exhaustive — *"a new `Event`
//! should fail to compile in every consumer"* — and a C caller cannot fail to compile. So the
//! compiler is made to fail **here** instead: [`entry_points`] matches exhaustively over `Query`,
//! so a variant added to `viewer-core` breaks this file, and whoever fixes it has to name the
//! symbol that answers it or write down that there is none.
//!
//! Three assertions, and each catches a different way the list can open again:
//!
//! - **the samples cover the enumeration**, counted out of `viewer-core`'s own source rather than
//!   from a number written here — a hand-written count is the thing that went stale in
//!   `doc/todo/02` §2 and in this crate's own event map (`header_and_library_agree.rs`'s note on
//!   `PDFV_EVENT_SEARCHED`);
//! - **every variant names at least one entry point**, so an arm cannot be closed with an empty
//!   list;
//! - **every entry point named exists in both `abi.rs` and the header**, which is where a symbol
//!   renamed on one side of the boundary shows up.
//!
//! `header_and_library_agree.rs` is the neighbouring check and asks the opposite question: that
//! every *symbol* is declared. This one asks that every *question* has one.

#![expect(
    clippy::expect_used,
    reason = "test code: a source file that cannot be read must fail loudly rather than pass by \
              doing nothing"
)]

use std::collections::BTreeSet;
use std::path::Path;

use viewer_core::Query;

/// The entry points a C caller answers this question with.
///
/// **Exhaustive over `Query` on purpose**, and that is the whole of what this file buys: a variant
/// added to `viewer-core` fails to compile here, in a test whose name says what it is for.
///
/// Several variants name more than one symbol, and the reason differs each time — which is why
/// this is a list rather than a name. `Query::Frame` is three because C's two-call idiom needs a
/// size before a copy; `Query::Preferences` is two because Table 147 has one entry that is a list
/// and eighteen that are not; `Query::Selection` is two because a caller draws the shapes and
/// copies the text at different moments. What none of them may be is **zero**.
fn entry_points(query: &Query<'_>) -> &'static [&'static str] {
    match query {
        Query::PageCount => &["pdfv_page_count"],
        Query::CurrentPage => &["pdfv_current_page"],
        // Two, because this is the one question whose answer goes back the way it came: the
        // reader's place is read with one and restored with the other (ADR 0737).
        Query::View => &["pdfv_view", "pdfv_set_view"],
        Query::PageGeometry(_) => &["pdfv_page_geometry"],
        Query::Outline => &["pdfv_outline_read"],
        Query::Layers => &["pdfv_layers_read"],
        Query::Attachments => &["pdfv_attachments_read"],
        // §12.3.5.2's key grammar is the fifth piece and is a function of its own, because a
        // caller holding a folder tree and a file list cannot put one inside the other without it.
        Query::Collection => &[
            "pdfv_collection_read",
            "pdfv_collection_view",
            "pdfv_collection_initial",
            "pdfv_collection_columns",
            "pdfv_collection_folders",
            "pdfv_collection_folder_of",
        ],
        Query::Articles => &["pdfv_articles_read"],
        Query::PageLabel(_) => &["pdfv_page_label"],
        Query::Thumbnail(_) => &["pdfv_thumbnail_read", "pdfv_thumbnail_copy"],
        Query::LinkAt(_) => &["pdfv_link_at"],
        Query::FieldAt(_) => &["pdfv_field_at"],
        Query::Fields => &["pdfv_fields_read"],
        Query::Caret { .. } => &["pdfv_caret"],
        Query::Offset { .. } => &["pdfv_offset"],
        Query::FieldSelection { .. } => &["pdfv_field_selection"],
        Query::FreeTextAt { .. } => &["pdfv_free_text_at"],
        Query::Dirty => &["pdfv_dirty"],
        Query::Properties => &["pdfv_properties_read"],
        Query::Opening => &["pdfv_opening"],
        Query::Preferences => &["pdfv_preference", "pdfv_preference_ranges"],
        Query::Find(_) => &["pdfv_find_matches", "pdfv_matches_quads"],
        // §14.8.2.5's logical order is not a second text accessor: it is what `pdfv_selection_text`
        // is *not*, and the entry point says which of the two orders it answered in (ADR 0519).
        Query::LogicalSelection => &["pdfv_selection_copy_text"],
        Query::Focus => &["pdfv_focused_annotation"],
        Query::Highlight => &["pdfv_highlight_quads"],
        Query::Popups => &["pdfv_popups_read", "pdfv_popup_text", "pdfv_popup_quad"],
        Query::Selection => &["pdfv_selection_text", "pdfv_selection_quads"],
        Query::Frame => &["pdfv_frame_count", "pdfv_frame_info", "pdfv_frame_copy"],
        Query::AccessibilityTree => &[
            "pdfv_structure_read",
            "pdfv_structure_page",
            "pdfv_structure_node",
            "pdfv_structure_text",
            // §14.7's per-character offsets and boxes, which AT-SPI's `Text` interface is built on
            // and which this answer carried nowhere until the seven-hundred-and-twenty-sixth.
            "pdfv_structure_lines",
            "pdfv_structure_line",
            "pdfv_structure_character",
        ],
        Query::Reports => &["pdfv_reported_pages", "pdfv_reported_page", "pdfv_report"],
        Query::Readback => &[
            "pdfv_readback_pages",
            "pdfv_readback_page",
            "pdfv_readback_count",
        ],
    }
}

/// One value of every `Query` variant.
///
/// The arguments are placeholders — nothing here is asked of a viewer — because what is under test
/// is the mapping and not an answer. `session.rs`'s own tests and `c/open_a_page.c` are what
/// actually call these.
fn every_query() -> Vec<Query<'static>> {
    let at = (0.0_f32, 0.0_f32);
    vec![
        Query::PageCount,
        Query::CurrentPage,
        Query::View,
        Query::PageGeometry(0),
        Query::Outline,
        Query::Layers,
        Query::Attachments,
        Query::Collection,
        Query::Articles,
        Query::PageLabel(0),
        Query::Thumbnail(0),
        Query::LinkAt(at),
        Query::FieldAt(at),
        Query::Fields,
        Query::Caret { at, offset: 0 },
        Query::Offset { at, point: at },
        Query::FieldSelection { at, from: 0, to: 0 },
        Query::FreeTextAt { at },
        Query::Dirty,
        Query::Properties,
        Query::Opening,
        Query::Preferences,
        Query::Find(""),
        Query::LogicalSelection,
        Query::Focus,
        Query::Highlight,
        Query::Popups,
        Query::Selection,
        Query::Frame,
        Query::AccessibilityTree,
        Query::Reports,
        Query::Readback,
    ]
}

/// How many variants `viewer_core::Query` declares, read out of its own source.
///
/// **Counted rather than written down**, which is `CLAUDE.md`'s rule about derived facts applied to
/// a test: a number here would be one more thing a round could forget to move, and this project has
/// been bitten by exactly that at least four times. The shape it counts on is the tree's own — one
/// variant per line, at four spaces of indent, inside `pub enum Query`.
fn variants_declared() -> usize {
    let source = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../viewer-core/src/query.rs"),
    )
    .expect("viewer-core has a query module");
    let body = source
        .split_once("pub enum Query<'a> {")
        .expect("viewer-core declares Query")
        .1;
    let body = body.split_once("\n}").expect("the enum is closed").0;
    body.lines()
        .filter(|line| {
            // A variant is an identifier at exactly one level of indent. Doc comments start with
            // `///`, nested fields are deeper, and a variant's closing brace starts with `}`.
            let Some(rest) = line.strip_prefix("    ") else {
                return false;
            };
            rest.starts_with(|first: char| first.is_ascii_uppercase())
        })
        .count()
}

#[test]
fn every_query_variant_names_at_least_one_entry_point() {
    for query in every_query() {
        let symbols = entry_points(&query);
        assert!(
            !symbols.is_empty(),
            "{query:?} names no entry point: a C caller cannot ask it, and nothing else would say \
             so — see this file's own module comment for why an empty arm is not an option"
        );
    }
}

#[test]
fn the_samples_cover_the_whole_enumeration() {
    let queries = every_query();
    let distinct: BTreeSet<String> = queries
        .iter()
        .map(|query| format!("{:?}", core::mem::discriminant(query)))
        .collect();
    assert_eq!(
        distinct.len(),
        queries.len(),
        "two samples are the same variant"
    );
    assert_eq!(
        queries.len(),
        variants_declared(),
        "`viewer-core` declares a `Query` variant this file has no sample for; `entry_points` \
         will have failed to compile first, which is the point"
    );
}

#[test]
fn every_entry_point_named_exists_in_the_library_and_in_the_header() {
    let abi = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/abi.rs"))
        .expect("this crate has an abi module");
    let header =
        std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("include/pdf_viewer.h"))
            .expect("this crate has a header");
    for query in every_query() {
        for symbol in entry_points(&query) {
            assert!(
                abi.contains(&format!("fn {symbol}(")),
                "{query:?} names {symbol}, which the library does not export"
            );
            assert!(
                header.contains(&format!("{symbol}(")),
                "{query:?} names {symbol}, which the header does not declare"
            );
        }
    }
}
