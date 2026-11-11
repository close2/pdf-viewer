//! Valid optional timezone fields and their defaults under ISO 32000-2 §7.9.4.

#[test]
fn timezone_suffixes_preserve_stated_minutes_and_default_omitted_fields() {
    for (suffix, expected) in [
        ("+", 0),
        ("-", 0),
        ("Z", 0),
        ("+23", 23 * 60),
        ("-23'", -23 * 60),
        ("+23'59", 23 * 60 + 59),
        ("-23'59", -(23 * 60 + 59)),
        ("+00'30", 30),
        ("-00'30", -30),
        ("Z00'00", 0),
    ] {
        let text = format!("D:20261231235959{suffix}");
        let parsed = pdf_syntax::date::Date::parse(&text).expect("valid date spelling");
        assert_eq!(parsed.offset, Some(expected), "{text}");
        assert_eq!((parsed.year, parsed.month, parsed.day), (2026, 12, 31));
        assert_eq!((parsed.hour, parsed.minute, parsed.second), (23, 59, 59));
    }
}
