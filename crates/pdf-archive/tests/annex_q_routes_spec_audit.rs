//! Annex Q routes the survey's walk does not take (audit 2271).
//!
//! ISO 32000-2 Annex Q.3: "the content stream of the form `XObject` shall be processed according to
//! Q.2, \"Page content\"", and Q.5 (Errata Collection 3, Issue #295): "For any graphical element
//! that represents text drawing, its text state shall be checked to determine the type of font
//! being used for rendering. If the Subtype of the font dictionary is Type3, then each object in
//! its `CharProc` array shall be processed as if it was a form `XObject` according to Q.3". Each
//! test pairs the failing route with a control that takes the route the walk already follows.

#![expect(clippy::expect_used, reason = "standalone exact-xref fixture")]

use std::fmt::Write as _;

use pdf_archive::survey::Survey;
use pdf_syntax::Document;

/// One page whose `/Resources` and content are the caller's, then the caller's objects from 5 on.
fn document(resources: &str, content: &str, extra: &[String]) -> Document {
    let mut objects = vec![
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /Contents 4 0 R \
             /Resources {resources} >>"
        ),
        stream("", content),
    ];
    objects.extend_from_slice(extra);

    let mut bytes = String::from("%PDF-2.0\n");
    let mut offsets = Vec::new();
    for (index, body) in objects.iter().enumerate() {
        offsets.push(bytes.len());
        let _ = writeln!(bytes, "{} 0 obj\n{body}\nendobj", index.saturating_add(1));
    }
    let xref = bytes.len();
    let size = objects.len().saturating_add(1);
    let _ = write!(bytes, "xref\n0 {size}\n0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(bytes, "{offset:010} 00000 n ");
    }
    let _ = write!(
        bytes,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n"
    );
    let document = Document::open(bytes.into_bytes()).expect("the exact-xref PDF opens");
    assert!(
        !document.was_recovered(),
        "the fixture xref is authoritative"
    );
    document
}

fn stream(dictionary: &str, content: &str) -> String {
    format!(
        "<< {dictionary} /Length {} >>\nstream\n{content}\nendstream",
        content.len().saturating_add(1)
    )
}

/// Object 6: a Type 3 font whose one glyph paints under a graphics state with `ca` 0.5 (object 8),
/// and object 7, that glyph's procedure.
fn type3_font_and_glyph() -> Vec<String> {
    vec![
        "<< /Type /Font /Subtype /Type3 /FontBBox [0 0 1 1] /FontMatrix [1 0 0 1 0 0] \
         /CharProcs << /a 7 0 R >> /Encoding << /Type /Encoding /Differences [97 /a] >> \
         /FirstChar 97 /LastChar 97 /Widths [1] \
         /Resources << /ExtGState << /T 8 0 R >> >> >>"
            .to_owned(),
        stream("", "1 0 0 0 1 1 d1 /T gs 0 0 1 1 re f"),
        "<< /Type /ExtGState /ca 0.5 >>".to_owned(),
    ]
}

/// 2271-B01. ISO 32000-2 §8.4.5, Table 57's `/Font` selects the font a later text-showing operator
/// draws with, exactly as `Tf` does, so the "font being used for rendering" Annex Q.5 checks is
/// that font whichever operator selected it. The survey's `show` returns before `type3` where a
/// `gs` set the font, so a Type 3 glyph procedure painting at `ca` 0.5 leaves the page opaque.
#[test]
fn a_type3_font_a_graphics_state_selects_has_its_glyphs_processed_spec_audit() {
    // The control: the same font selected by `Tf` is found transparent through its glyph.
    let mut by_tf = vec!["<< /Type /ExtGState >>".to_owned()];
    by_tf.extend(type3_font_and_glyph());
    let control = document(
        "<< /Font << /F 6 0 R >> >>",
        "BT /F 12 Tf 0 0 Td (a) Tj ET",
        &by_tf,
    );
    let survey = Survey::of(&control);
    assert!(!survey.stopped_short());
    assert!(
        survey.page_is_transparent(0),
        "control: Q.5 walks the CharProcs of a Type 3 font that Tf selected"
    );

    let mut by_gs = vec!["<< /Type /ExtGState /Font [6 0 R 12] >>".to_owned()];
    by_gs.extend(type3_font_and_glyph());
    let doc = document(
        "<< /ExtGState << /GF 5 0 R >> >>",
        "/GF gs BT 0 0 Td (a) Tj ET",
        &by_gs,
    );
    let survey = Survey::of(&doc);
    assert!(!survey.stopped_short());
    assert!(
        survey.page_is_transparent(0),
        "Annex Q.5: the Type 3 font a gs /Font selected is the font used for rendering, and its \
         glyph paints at ca 0.5"
    );
}

/// Objects 6 to 8: a soft mask's group (6) that draws the form `Fm` (7), whose own content paints
/// at `ca` 0.5 (object 8). Object 5 is the graphics state that sets the mask.
fn mask_group_drawing_the_form() -> Vec<String> {
    vec![
        "<< /Type /ExtGState /SMask << /Type /Mask /S /Luminosity /G 6 0 R >> >>".to_owned(),
        stream(
            "/Type /XObject /Subtype /Form /BBox [0 0 10 10] \
             /Group << /S /Transparency /CS /DeviceGray >> \
             /Resources << /XObject << /Fm 7 0 R >> >>",
            "/Fm Do",
        ),
        stream(
            "/Type /XObject /Subtype /Form /BBox [0 0 10 10] \
             /Resources << /ExtGState << /T 8 0 R >> >>",
            "/T gs 0 0 10 10 re f",
        ),
        "<< /Type /ExtGState /ca 0.5 >>".to_owned(),
    ]
}

/// 2271-B02. Annex Q.3 processes every form a page draws "according to Q.2". The survey walks a
/// soft mask's group where the `gs` sets it, then takes back what that walk found (ADR 2994's
/// "the group is not rendered on the page"), but the `(page, object)` it marked visited stays
/// marked. A form the group draws, drawn again by the page itself, is then never opened for the
/// page: its own `ca` 0.5 fill is an element rendered on the page that the walk does not see.
#[test]
fn a_form_a_soft_mask_group_also_draws_is_still_processed_for_the_page_spec_audit() {
    let resources = "<< /ExtGState << /M 5 0 R >> /XObject << /Fm 7 0 R >> >>";
    // The control: the page draws the form and never sets the mask.
    let control = document(resources, "/Fm Do", &mask_group_drawing_the_form());
    let survey = Survey::of(&control);
    assert!(!survey.stopped_short());
    assert!(
        survey.page_is_transparent(0),
        "control: Q.3 finds the form's ca 0.5 fill"
    );
    // The second control isolates the cause: the group draws a copy of the form (object 9), so
    // the page's own `Do` opens an object the mask's walk never visited, and is found.
    let mut copied = mask_group_drawing_the_form();
    copied[1] = copied[1].replace("/Fm 7 0 R", "/Fm 9 0 R");
    copied.push(copied[2].clone());
    let control = document(resources, "q /M gs Q /Fm Do", &copied);
    assert!(
        Survey::of(&control).page_is_transparent(0),
        "control: a mask group drawing a different object leaves the page's form to be walked"
    );
    // The mask is set and restored before anything is painted under it, so by ADR 2994 the mask
    // marks nothing; the form drawn afterwards still paints at ca 0.5.
    let doc = document(
        resources,
        "q /M gs Q /Fm Do",
        &mask_group_drawing_the_form(),
    );
    let survey = Survey::of(&doc);
    assert!(!survey.stopped_short());
    assert!(
        survey.page_is_transparent(0),
        "Annex Q.3: the form the page draws is processed according to Q.2 even where a soft \
         mask's group drew it first"
    );
}

/// Objects 5 to 7: graphics states at `ca` 0.5 (`/T`) and 1 (`/O`), and a coloured tiling pattern
/// whose cell paints one opaque rectangle under its own state.
fn opaque_cell_pattern() -> Vec<String> {
    vec![
        "<< /Type /ExtGState /ca 0.5 >>".to_owned(),
        "<< /Type /ExtGState /ca 1 >>".to_owned(),
        stream(
            "/Type /Pattern /PatternType 1 /PaintType 1 /TilingType 1 /BBox [0 0 1 1] \
             /XStep 1 /YStep 1 /Resources << >>",
            "0 0 1 1 re f",
        ),
    ]
}

/// 2271-B03. Annex Q.2 checks the graphics state of "each graphical element to be rendered on
/// the page", and a Type 1 pattern's cell is processed "as a form `XObject`". ISO 32000-2
/// §8.7.3.1's step b) gives the cell its state: the processor "\[i]nstalls the graphics state
/// that was in effect at the beginning of the pattern's parent content stream", not the one in
/// force where `scn` selected the pattern. The survey walks the cell at the `scn` with that
/// operator's state, so a `ca` 0.5 restored to 1 before the fill marks a page whose every
/// element — the fill at `ca` 1 and the cell's opaque rectangle — is opaque.
#[test]
fn a_pattern_cell_is_judged_in_its_own_graphics_state_spec_audit() {
    let resources = "<< /ExtGState << /T 5 0 R /O 6 0 R >> /Pattern << /P 7 0 R >> >>";
    // The control: the same pattern selected and painted at `ca` 1 throughout.
    let control = document(
        resources,
        "/Pattern cs /P scn 0 0 10 10 re f",
        &opaque_cell_pattern(),
    );
    let survey = Survey::of(&control);
    assert!(!survey.stopped_short());
    assert!(
        !survey.page_is_transparent(0),
        "control: an opaque cell painted at ca 1 is no transparency"
    );
    // The second control: the fill itself at `ca` 0.5 is transparency, wherever the pattern was
    // selected.
    let control = document(
        resources,
        "/Pattern cs /P scn /T gs 0 0 10 10 re f",
        &opaque_cell_pattern(),
    );
    assert!(Survey::of(&control).page_is_transparent(0));

    let doc = document(
        resources,
        "/T gs /Pattern cs /P scn /O gs 0 0 10 10 re f",
        &opaque_cell_pattern(),
    );
    let survey = Survey::of(&doc);
    assert!(!survey.stopped_short());
    assert!(
        !survey.page_is_transparent(0),
        "Annex Q.2 with §8.7.3.1: the fill is painted at ca 1 and the cell's rectangle in the \
         pattern's own initial graphics state, so no element on the page is transparent"
    );
}
