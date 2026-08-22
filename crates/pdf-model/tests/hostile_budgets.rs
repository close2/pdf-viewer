//! What this program's resource bounds stop, and that each stops it *by name*.
//!
//! `CLAUDE.md` principle 3: "Memory safety is not enough. Explicit memory and time budgets
//! guard against decompression bombs, xref cycles, and pathological content — Rust does not
//! prevent resource exhaustion." The four bounds in `content.rs` are that guard, and until the
//! four-hundred-and-thirty-fifth session **nobody had opened the documents they stop**: the
//! survey of 65 944 crawled documents reported 84 refusals over 83 of them and no more.
//!
//! Two of the fixtures below are about a bound on *bytes* rather than on a count —
//! `Limits::max_stream_len`, and the total of a page's `/Contents` parts, which had no bound at
//! all until ADR 0306. They are here rather than beside `pdf-syntax`'s own filter tests because
//! what they assert is what the *page* reports, which is the thing a person sees.
//!
//! # A fixture whose two numbers agree measures neither
//!
//! `MAX_OPERATIONS` counted lexer tokens for its whole life while its name and its comment said
//! operators, and the reason no test saw it is in this file: the fixture below was
//! `"n\n".repeat(4_000_002)` — a *zero-operand* operator, chosen "so this measures the bound
//! rather than the operator", which is the one input shape where tokens and operators are the
//! same number. §7.8.2 puts an operator after its operands, so a `c` is seven tokens and one
//! operator and a real drawing was refused at a seventh of the advertised bound. Every fixture
//! here now states operands, and the control that would have caught it — many tokens, few
//! operators — is `a_stream_of_many_tokens_and_few_operators_still_draws`. ADR 0306.
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
    clippy::arithmetic_side_effects,
    reason = "test code: a malformed fixture should fail loudly, and the one sum here is an \
              index into a fixture of three objects"
)]

use std::fmt::Write as _;

use pdf_syntax::{Document, Limits};

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
    assemble(&body, Limits::DEFAULT)
}

/// A one-page PDF whose `/Contents` is an array of `parts`, opened under `limits`.
///
/// Table 31 makes the array one stream, which is what gives the concatenation a bound.
fn page_of_parts(parts: &[String], limits: Limits) -> Document {
    let mut names = String::new();
    let mut objects = String::new();
    for (index, part) in parts.iter().enumerate() {
        let number = index + 4;
        let _ = write!(names, "{number} 0 R ");
        let _ = write!(
            objects,
            "{number} 0 obj\n<< /Length {} >>\nstream\n{part}\nendstream\nendobj\n",
            part.len()
        );
    }
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] \
         /Resources << >> /Contents [{names}] >>\nendobj\n{objects}"
    );
    assemble(&body, limits)
}

/// Wraps a body of objects in a header, a cross-reference table and a trailer.
fn assemble(body: &str, limits: Limits) -> Document {
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
    Document::open_with_limits(out.into_bytes(), limits).expect("the fixture opens")
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
///
/// **What "refused" means here is the trip count and not the paint**, since the
/// six-hundred-and-forty-seventh session: the bound is reported by name and the sites it affords
/// are laid down, which for an *empty* cell is four thousand copies of nothing. The work this
/// test bounds is unchanged — 4096 sites is what the check admitted before it and what it spends
/// now — and the assertion is on the name for that reason. ADR 0477.
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
/// 65 944 that reach it under the *token* count it used to keep, every one *terminates* with the
/// bound lifted a hundredfold, and they are maps, plans and charts rather than bombs. It is left
/// at four million because the worst case at a raised count is not bounded either: one operator
/// can paint the whole page, so a count is not a cost. ADRs 0271 and 0306.
///
/// The fixture states one operand per operator on purpose. `0 g` is §8.6.8's "set the colour
/// space to `DeviceGray` … and set the gray level", two lexer tokens and **one** operator, so a
/// counter reading the lexer would refuse this stream at half its length and this assertion
/// would pass for the wrong reason.
#[test]
fn a_content_stream_longer_than_the_bound_is_refused_by_name() {
    let content = "0 g\n".repeat(4_000_002);
    let document = page(&content, "<< >>", "");
    let reported = reported(&document);
    assert!(
        reported.contains("MAX_OPERATIONS"),
        "four million operators must be refused by name: {reported}"
    );
}

/// …and a stream of *many tokens and few operators* is not refused at all.
///
/// **The control that was missing for the whole life of the bound**, and the shape of the
/// document that found it: a hand-traced drawing, all cubic Béziers and no text. §7.8.2 puts an
/// operator after its operands, so `x1 y1 x2 y2 x3 y3 c` is seven tokens and one operator, and a
/// counter reading the loop's turns charges a curve seven times over. The fixture below states
/// **4.4 million lexer tokens and 1.1 million operators**: it was refused before ADR 0306 and
/// draws after it, and no other assertion in this file can tell the two counters apart.
#[test]
fn a_stream_of_many_tokens_and_few_operators_still_draws() {
    // `c` appends a cubic Bézier — six operands, seven tokens — and `n` ends the path so that
    // the fixture does not accumulate half a million segments in one path object.
    let mut content = "0 0 0 rg 10 10 100 100 re f\n".to_owned();
    content.push_str(&"0 0 0 0 0 0 c\nn\n".repeat(550_000));
    let document = page(&content, "<< >>", "");
    let reported = reported(&document);
    assert_eq!(
        reported, "[]",
        "1.1 million operators are inside a four-million-operator bound, whatever the token \
         count is"
    );
    assert!(
        commands(&document) > 0,
        "and the square stated before the curves is still drawn"
    );
}

/// A page whose `/Contents` parts add up past `max_stream_len` says so, and names the bound.
///
/// **There was no total at all until ADR 0306.** One part was bounded and the concatenation was
/// not, and `/Contents` may hold `max_array_len` = 2²⁰ entries. ISO 32000-2 §7.7.3.3's Table 31
/// is what gives the concatenation a bound without inventing a second number:
///
/// > If the value is an array, the effect shall be as if all of the streams in the array were
/// > concatenated with at least one white-space character added between the streams' data, in
/// > order, to form a single stream.
///
/// So the array *is* one stream, and the bound one stream gets is the bound it gets. The
/// fixture moves the bound rather than building a gibibyte to reach it.
#[test]
fn contents_parts_adding_up_past_the_bound_are_refused_by_name() {
    let part = "0 g\n".repeat(100);
    let limits = Limits {
        max_stream_len: 1000,
        ..Limits::DEFAULT
    };
    let document = page_of_parts(&[part.clone(), part.clone(), part], limits);
    let reported = reported(&document);
    assert!(
        reported.contains("TooLarge"),
        "three parts of 400 bytes against a bound of 1000 must be refused by name: {reported}"
    );
}

/// …and parts adding up to less than the bound are one stream, drawn.
#[test]
fn contents_parts_inside_the_bound_are_one_stream() {
    let limits = Limits {
        max_stream_len: 1000,
        ..Limits::DEFAULT
    };
    let document = page_of_parts(
        &[
            "0 0 0 rg\n".to_owned(),
            "10 10 100 100 re\n".to_owned(),
            "f\n".to_owned(),
        ],
        limits,
    );
    let reported = reported(&document);
    assert_eq!(reported, "[]", "three short parts are inside the bound");
    assert!(
        commands(&document) > 0,
        "and the square, whose operator is in the third part and whose operands are in the \
         second, is drawn — which is what Table 31's concatenation means"
    );
}

/// **A `/Mask` that names an image mask carrying a mask of its own is refused, not followed.**
///
/// Found by the `page` fuzz target in the five-hundred-and-sixty-fourth session, and it is not a
/// budget: `decode_parts` → `apply_explicit_mask` → `decode` → `decode_parts` had **no bound at
/// all**, so a stencil naming itself overflowed the stack. §C.2's *Nested objects* row above
/// anticipates exactly this — "PDF processors may implement recursive algorithms which may cause
/// issues for excessively nested constructs" — but the bound here needs no constant, because
/// Table 87 gives an image mask no `/Mask` and there is no depth beyond one to allow. ADR 0399.
///
/// The fixture is generated for the reason this file's header gives, and for a second one: the
/// artefact libFuzzer wrote is a mutation of a `SafeDocs` member of a Common Crawl archive, which
/// `.gitignore` and `doc/third-party-data.md` keep out of this history.
#[test]
fn an_image_mask_that_masks_itself_is_refused_by_name() {
    let document = page(
        "q 100 0 0 100 0 0 cm /Im Do Q",
        "<< /XObject << /Im 5 0 R >> >>",
        "5 0 obj\n<< /Type /XObject /Subtype /Image /Width 2 /Height 2 \
         /BitsPerComponent 8 /ColorSpace /DeviceGray /Mask 6 0 R /Length 4 >>\n\
         stream\n\x01\x02\x03\x04\nendstream\nendobj\n\
         6 0 obj\n<< /Type /XObject /Subtype /Image /Width 2 /Height 2 \
         /ImageMask true /Mask 6 0 R /Length 2 >>\nstream\n\x08\x40\nendstream\nendobj\n",
    );
    let reported = reported(&document);
    assert!(
        reported.contains("carrying a /Mask of its own"),
        "a stencil whose own /Mask is itself must be refused by name: {reported}"
    );
    assert!(
        commands(&document) > 0,
        "and the base image is still drawn, unmasked — Table 87's rule is about the mask"
    );
}

/// **The same door on §11.6.5.2's side**: an `/SMask` carrying a `/Mask` is refused, not followed.
///
/// Table 143 says of a soft-mask image's `/Mask` entry "Shall be absent", and the `/SMask`-inside-
/// `/SMask` half of that row was already guarded while this half was not — so `apply_soft_mask`
/// reached `apply_explicit_mask` and the descent was unbounded from there. ADR 0399.
#[test]
fn a_soft_mask_carrying_an_explicit_mask_is_refused_by_name() {
    let document = page(
        "q 100 0 0 100 0 0 cm /Im Do Q",
        "<< /XObject << /Im 5 0 R >> >>",
        "5 0 obj\n<< /Type /XObject /Subtype /Image /Width 2 /Height 2 \
         /BitsPerComponent 8 /ColorSpace /DeviceGray /SMask 6 0 R /Length 4 >>\n\
         stream\n\x01\x02\x03\x04\nendstream\nendobj\n\
         6 0 obj\n<< /Type /XObject /Subtype /Image /Width 2 /Height 2 \
         /BitsPerComponent 8 /ColorSpace /DeviceGray /Mask 6 0 R /Length 4 >>\n\
         stream\n\x10\x20\x30\x40\nendstream\nendobj\n",
    );
    let reported = reported(&document);
    assert!(
        reported.contains("carries a /Mask of its own"),
        "a soft mask whose own /Mask is itself must be refused by name: {reported}"
    );
    assert!(
        commands(&document) > 0,
        "and the base image is still drawn, opaque — Table 143's rule is about the mask"
    );
}
