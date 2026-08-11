//! What the interpreter's four resource bounds stop, and that each stops it *by name*.
//!
//! `CLAUDE.md` principle 3: "Memory safety is not enough. Explicit memory and time budgets
//! guard against decompression bombs, xref cycles, and pathological content — Rust does not
//! prevent resource exhaustion." The four bounds in `content.rs` are that guard, and until the
//! four-hundred-and-thirty-fifth session **nobody had opened the documents they stop**: the
//! survey of 65 944 crawled documents reported 84 refusals over 83 of them and no more.
//!
//! # Where the standard is on a nested construct
//!
//! ISO 32000-2 leaves every one of these to the processor, and Annex C is where it says so.
//! §C.1: "In general, this PDF standard does not restrict the size or quantity of things
//! described in the PDF file format" — and §C.2's Table C.1 has a *Nested objects* row that
//! anticipates a bound like these outright:
//!
//! > As described in this PDF standard, many constructs can be nested including stitching
//! > functions, q / Q operators, XObjects, article threads, etc. However PDF processors may
//! > implement recursive algorithms which may cause issues for excessively nested constructs.
//!
//! and its NOTE gives the one figure the standard prints for any of them:
//!
//! > In previous versions of PDF, a maximum depth of graphics state nesting by q and Q
//! > operators was 28.
//!
//! Annex C is **informative**, so none of that binds; what it does is settle whether 256 is a
//! mean bound, and it is nine times the only number the standard names.
//!
//! # What each test is
//!
//! Every fixture is **generated**, so `doc/todo/03`'s promotion budget pays nothing, and each
//! hostile case is paired with a control just inside the bound — without one, a change that
//! refused *everything* would pass the hostile half and be a blank page. ADR 0271.

#![expect(
    clippy::expect_used,
    reason = "test code: a malformed fixture should fail loudly"
)]

use std::fmt::Write as _;

use pdf_syntax::Document;

/// A one-page PDF whose content stream is `operators`, with `extra` objects beside it.
///
/// `resources` is the page's `/Resources` dictionary, written out verbatim so that a fixture
/// can name a form or a pattern it also supplies through `extra`.
fn page(operators: &str, resources: &str, extra: &str) -> Document {
    let length = operators.len();
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] \
         /Resources {resources} /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length {length} >>\nstream\n{operators}\nendstream\nendobj\n{extra}"
    );

    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(out.len());
        out.push_str(object);
    }
    let xref_at = out.len();
    let size = offsets.len().saturating_add(1);
    let _ = writeln!(out, "xref\n0 {size}");
    out.push_str("0000000000 65535 f \n");
    for offset in &offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    Document::open(out.into_bytes()).expect("the fixture opens")
}

/// What page one reported, as the survey and the corpus gate print it.
fn reported(document: &Document) -> String {
    let page = pdf_model::Pages::new(document)
        .get(0)
        .expect("the fixture has a page");
    format!("{:?}", pdf_model::interpret(document, &page).unsupported)
}

/// How many commands page one produced, for a control that has to actually draw.
fn commands(document: &Document) -> usize {
    let page = pdf_model::Pages::new(document)
        .get(0)
        .expect("the fixture has a page");
    pdf_model::interpret(document, &page)
        .display_list
        .commands()
        .len()
}

/// A stream nesting `q` past `MAX_STATE_DEPTH` is refused, and says which bound refused it.
///
/// Each `q` carries a fresh dash pattern, because what a saved state *costs* is the clone of
/// everything in it and `Stroke::dash_array` is the one field a content stream can make large:
/// this is the shape that turns depth into memory rather than into a counter.
///
/// **The web's one witness wants 337** — `0546285.pdf`, archive `0546` of `cc-main-2021-31`,
/// the only document of 65 944 to reach this bound — which is 12 times Table C.1's figure and
/// is why the four-hundred-and-thirty-fifth session left the bound at 256. ADR 0271.
#[test]
fn nesting_the_graphics_state_past_the_bound_is_refused_by_name() {
    let mut content = String::new();
    for index in 0..400 {
        let _ = writeln!(content, "[{} {}] 0 d q", index % 7 + 1, index % 5 + 1);
    }
    let document = page(&content, "<< >>", "");
    let reported = reported(&document);
    assert!(
        reported.contains("MAX_STATE_DEPTH"),
        "400 nested q must be refused by name, not silently truncated: {reported}"
    );
}

/// …and a stream nesting `q` to just inside the bound still draws.
///
/// The control. A guard that refused every `q` would pass the test above and turn the corpus
/// blank, which is `CLAUDE.md`'s trap 1 in one sentence.
#[test]
fn nesting_the_graphics_state_inside_the_bound_still_draws() {
    let mut content = String::new();
    for _ in 0..200 {
        content.push_str("q\n");
    }
    content.push_str("0 0 0 rg 10 10 100 100 re f\n");
    let document = page(&content, "<< >>", "");
    let reported = reported(&document);
    assert_eq!(reported, "[]", "200 nested q is inside the bound");
    assert!(
        commands(&document) > 0,
        "the square under 200 nested q is still drawn"
    );
}

/// A form `XObject` that draws itself is refused, and says which bound refused it.
///
/// **This is what the bound is for, and the web says so.** All four documents of 65 944 that
/// reach `MAX_FORM_DEPTH` are cycles: with the bound lifted sixteenfold to 256 in a scratch
/// build, every one of them reached 256 as well — `0915226.pdf`, `2268260.pdf`, `4974696.pdf`
/// and `6327929.pdf`. Nothing else stops this. The confined worker's address-space ceiling
/// cannot, because unbounded recursion exhausts the *stack* and Rust's guard page turns that
/// into an abort rather than into a report.
#[test]
fn a_form_that_draws_itself_is_refused_by_name() {
    let form = "5 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 612 792] \
                /Resources << /XObject << /Fx 5 0 R >> >> /Length 22 >>\n\
                stream\n0 0 0 rg /Fx Do\nendstream\nendobj\n";
    let document = page("/Fx Do", "<< /XObject << /Fx 5 0 R >> >>", form);
    let reported = reported(&document);
    assert!(
        reported.contains("MAX_FORM_DEPTH"),
        "a form naming itself is a cycle and must be refused by name: {reported}"
    );
}

/// A tiling pattern whose cell is **empty** is refused by `MAX_TILES` and by nothing else.
///
/// The case that makes this bound load-bearing rather than a second opinion. Every other
/// runaway a pattern can state is caught by `MAX_OPERATIONS`, because a cell's content stream
/// runs through the same interpreter and its operators are counted — but an *empty* cell
/// executes no operator at all, so the loop over `columns × rows` would run the number of
/// times the file's `/XStep` and `/YStep` say and no counter would ever move. The file states
/// the trip count directly; this is what bounds it.
///
/// **Measured rather than reasoned about.** With `MAX_TILES` lifted to 4 194 304 in a scratch
/// build, an empty cell stepped to give 1 000 000 tiles interprets in **889 ms and reports
/// nothing at all** — so the per-tile cost is 0.89 µs and `MAX_OPERATIONS` is never consulted.
/// The fixture below states a `/XStep` of 0.001 over a 600-unit fill, which is 600 000 columns
/// and as many rows: 3.6 × 10¹¹ tiles, or about **four days** at that rate.
#[test]
fn a_tiling_whose_cell_is_empty_is_refused_by_name() {
    let pattern = "5 0 obj\n<< /PatternType 1 /PaintType 1 /TilingType 1 /BBox [0 0 1 1] \
                   /XStep 0.001 /YStep 0.001 /Resources << >> /Length 0 >>\n\
                   stream\n\nendstream\nendobj\n";
    let document = page(
        "/Pattern cs /P0 scn 0 0 600 600 re f",
        "<< /Pattern << /P0 5 0 R >> >>",
        pattern,
    );
    let reported = reported(&document);
    assert!(
        reported.contains("MAX_TILES"),
        "an empty cell stepped every thousandth of a unit must be refused by name: {reported}"
    );
}

/// …and a tiling whose cell count is inside the bound still paints.
#[test]
fn a_tiling_inside_the_bound_still_paints() {
    let cell = "1 0 0 rg 0 0 10 10 re f";
    let pattern = format!(
        "5 0 obj\n<< /PatternType 1 /PaintType 1 /TilingType 1 /BBox [0 0 10 10] \
         /XStep 20 /YStep 20 /Resources << >> /Length {} >>\nstream\n{cell}\nendstream\nendobj\n",
        cell.len()
    );
    let document = page(
        "/Pattern cs /P0 scn 0 0 400 400 re f",
        "<< /Pattern << /P0 5 0 R >> >>",
        &pattern,
    );
    let reported = reported(&document);
    assert_eq!(reported, "[]", "20 x 20 cells over 400 units is 400 tiles");
    assert!(
        commands(&document) > 100,
        "four hundred cells each paint a square"
    );
}

/// A content stream longer than `MAX_OPERATIONS` is refused, and says which bound refused it.
///
/// **The one bound of the four whose population is mostly legitimate.** Of the 31 documents of
/// 65 944 that reach it, every one *terminates* with the bound lifted a hundredfold — they want
/// 4.1 to 53.6 million operators and take 0.27 s to 49.9 s — so what this bound stops is
/// slowness rather than a bomb, and the confined worker's cancel already answers slowness at
/// 0.83–1.97 ms. It is left where it is because the worst case at a *raised count* is not
/// bounded either: one operator can paint the whole page, so a count is not a cost. ADR 0271.
#[test]
fn a_content_stream_longer_than_the_bound_is_refused_by_name() {
    // `n` is §8.5.3.1's "end the path object without filling or stroking", the cheapest
    // operator there is — so this measures the bound rather than the operator.
    let content = "n\n".repeat(4_000_002);
    let document = page(&content, "<< >>", "");
    let reported = reported(&document);
    assert!(
        reported.contains("MAX_OPERATIONS"),
        "four million operators must be refused by name: {reported}"
    );
}
