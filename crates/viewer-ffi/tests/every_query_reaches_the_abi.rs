//! Every `viewer_core::Query` variant against the entry points that answer it.
//!
//! **This is the instrument ADR 0509 asked for and ADR 0576 built**, and the defect it exists
//! against is one this crate actually had: `Query::Find`, `Query::Opening`, `Query::Preferences`
//! and eight more reached **no symbol at all**, for as long as three hundred sessions in one case,
//! and nothing anywhere said so. The ABI's own protection is `QUORRA_EVENT_KIND_COUNT`, which is the
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
//!   `QUORRA_EVENT_SEARCHED`);
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
        Query::PageCount => &["quorra_page_count"],
        Query::CurrentPage => &["quorra_current_page"],
        // Two, because this is the one question whose answer goes back the way it came: the
        // reader's place is read with one and restored with the other (ADR 0737).
        Query::View => &["quorra_view", "quorra_set_view"],
        Query::PageGeometry(_) => &["quorra_page_geometry"],
        Query::Outline => &["quorra_outline_read"],
        Query::Layers => &["quorra_layers_read"],
        Query::Attachments => &["quorra_attachments_read"],
        // §12.3.5.2's key grammar is the fifth piece and is a function of its own, because a
        // caller holding a folder tree and a file list cannot put one inside the other without it.
        Query::Collection => &[
            "quorra_collection_read",
            "quorra_collection_view",
            "quorra_collection_initial",
            "quorra_collection_columns",
            "quorra_collection_folders",
            "quorra_collection_folder_of",
        ],
        Query::Articles => &["quorra_articles_read"],
        Query::PageLabel(_) => &["quorra_page_label"],
        Query::Thumbnail(_) => &["quorra_thumbnail_read", "quorra_thumbnail_copy"],
        Query::LinkAt(_) => &["quorra_link_at"],
        Query::FieldAt(_) => &["quorra_field_at"],
        Query::Fields => &["quorra_fields_read"],
        Query::Caret { .. } => &["quorra_caret"],
        Query::Offset { .. } => &["quorra_offset"],
        Query::FieldSelection { .. } => &["quorra_field_selection"],
        Query::FreeTextAt { .. } => &["quorra_free_text_at"],
        Query::Dirty => &["quorra_dirty"],
        Query::Properties => &["quorra_properties_read"],
        Query::Opening => &["quorra_opening"],
        Query::Preferences => &["quorra_preference", "quorra_preference_ranges"],
        Query::Find(_) => &["quorra_find_matches", "quorra_matches_quads"],
        // §14.8.2.5's logical order is not a second text accessor: it is what `quorra_selection_text`
        // is *not*, and the entry point says which of the two orders it answered in (ADR 0519).
        Query::LogicalSelection => &["quorra_selection_copy_text"],
        Query::Focus => &["quorra_focused_annotation"],
        Query::Highlight => &["quorra_highlight_quads"],
        Query::Popups => &[
            "quorra_popups_read",
            "quorra_popup_text",
            "quorra_popup_quad",
        ],
        Query::Selection => &["quorra_selection_text", "quorra_selection_quads"],
        Query::Frame => &[
            "quorra_frame_count",
            "quorra_frame_info",
            "quorra_frame_copy",
        ],
        Query::AccessibilityTree => &[
            "quorra_structure_read",
            "quorra_structure_page",
            "quorra_structure_node",
            "quorra_structure_text",
            // §14.7's per-character offsets and boxes, which AT-SPI's `Text` interface is built on
            // and which this answer carried nowhere until the seven-hundred-and-twenty-sixth.
            "quorra_structure_lines",
            "quorra_structure_line",
            "quorra_structure_character",
        ],
        Query::Reports => &[
            "quorra_reported_pages",
            "quorra_reported_page",
            "quorra_report",
        ],
        Query::Readback => &[
            "quorra_readback_pages",
            "quorra_readback_page",
            "quorra_readback_count",
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
        std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("include/quorra.h"))
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
