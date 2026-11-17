//! Literal reverse mappings from ISO 32000-2 Table 113 (physical342, printed327).
//! This is an encoding-helper assertion, not a glyph-selection or raster test.
// Expected values transcribed from ISO 32000-2:2020(E) Table 113,
// physical p342 / printed p327, and independently compared with the static table.
#[test]
fn every_table_113_code_has_its_literal_mac_os_roman_name() {
    let expected = [
        (173, "notequal"),
        (176, "infinity"),
        (178, "lessequal"),
        (179, "greaterequal"),
        (182, "partialdiff"),
        (183, "summation"),
        (184, "product"),
        (185, "pi"),
        (186, "integral"),
        (189, "Omega"),
        (195, "radical"),
        (197, "approxequal"),
        (198, "Delta"),
        (215, "lozenge"),
        (219, "Euro"),
        (240, "apple"),
    ];
    for (code, glyph_name) in expected {
        assert_eq!(
            pdf_font::encoding::mac_os_roman_code(glyph_name),
            Some(code)
        );
    }
    assert_eq!(pdf_font::encoding::mac_os_roman_code("currency"), None);
}
