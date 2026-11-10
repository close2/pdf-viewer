// Proposed sibling unit test inside `crates/pdf-font/src/truetype.rs`'s
// `truetype_encoding_tests` module. It exercises the real code-table consumer with a
// deterministic sfnt cmap and controls that yield different glyph indices.
//
// The two cmap routes intentionally disagree for PDF code 0x41:
// - Macintosh Roman (1, 0): byte 0x41 -> glyph 7, the Symbolic-font route.
// - Windows Unicode (3, 1): U+0041 -> glyph 9, the glyph-name / AGL route for WinAnsi `A`.
//
// No machine font catalogue, rasterizer, glyph outline, or platform choice is involved.

#[test]
fn a_conflicting_flags_word_follows_symbolic_with_route_controls() {
    const SYMBOLIC_GLYPH: u16 = 7;
    const LATIN_GLYPH: u16 = 9;
    const CODE_A: usize = 0x41;

    // Cmap format 6 is used for both records. The distinct subtable identities select two
    // different Table 112 / §9.6.5.4 routes; their glyph values are independent controls.
    let macintosh = [0_u16; 0x42];
    let mut macintosh = macintosh;
    macintosh[0x41] = SYMBOLIC_GLYPH;
    let unicode = [LATIN_GLYPH]; // Starts at U+0041.
    let data = sfnt(&[(
        *b"cmap",
        cmap_of(&[(1, 0, 0, &macintosh), (3, 1, 0x41, &unicode)]),
    )]);

    let code_table = |flags: u32| {
        // This is a unit fixture for the code-table decision, so the intentionally minimal
        // descriptor omits unrelated required metrics. Keep the font dictionary's subtype
        // accurate for the branch under test.
        let body = format!(
            "1 0 obj\n<< /Type /Font /Subtype /TrueType /BaseFont /Fixture \
             /Encoding /WinAnsiEncoding /FontDescriptor << /Flags {flags} >> >>\nendobj\n"
        );
        let (document, dict) = crate::fixture::document_of(&[body.as_bytes()]);
        let descriptor = document.get_key(&dict, "FontDescriptor");
        truetype_code_table(
            &document,
            &dict,
            descriptor.as_dict(),
            &data,
            &ToUnicode::default(),
            "F1",
        )
        .expect("the synthetic cmap is readable")
        .0
    };

    // Positive control: a valid Symbolic-only word uses the program's own byte code.
    assert_eq!(code_table(1 << 2)[CODE_A], Some(SYMBOLIC_GLYPH));
    // Negative-route control: Nonsymbolic-only honors WinAnsi -> A -> U+0041.
    assert_eq!(code_table(1 << 5)[CODE_A], Some(LATIN_GLYPH));
    // The invalid both-set word is nevertheless read by the processor's recommended
    // Symbolic precedence, matching the explicit Symbolic-only result.
    assert_eq!(
        code_table((1 << 2) | (1 << 5))[CODE_A],
        Some(SYMBOLIC_GLYPH)
    );
}
