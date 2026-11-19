//! ITU-T X.520 clauses 8.2.10, 8.9.1 to 8.9.4 and 8.10.1 over TR 102 272's
//! `attributeValueConstraints`, each expected value read off the clause or the RFC it names, and
//! their own examples where they print them: RFC 3492 section 7.1's samples, RFC 3986 sections
//! 5.2.4, 6.2.2 and 6.2.3, RFC 5892's derivation, X.520 clause 8.9.2's note, and RFC 7622 section
//! 3.5's tables of legal and illegal JIDs.

use super::code_points::{self, Class, Property};
use super::*;
use crate::der::Reader;
use crate::policy_rules::tests::{seq, tlv};

const UTF8: u8 = 0x0C;
const OCTETS: u8 = 0x04;

/// `id-at-uri`, 2.5.4.83.
const URI: &[u8] = &[0x55, 0x04, 83];
/// `id-at-dnsName`, 2.5.4.100.
const DNS_NAME: &[u8] = &[0x55, 0x04, 100];
/// `id-at-intEmail`, 2.5.4.104.
const INT_EMAIL: &[u8] = &[0x55, 0x04, 104];
/// `id-at-jid`, 2.5.4.105.
const JID: &[u8] = &[0x55, 0x04, 105];
/// `id-at-presentationAddress`, 2.5.4.29.
const PRESENTATION_ADDRESS: &[u8] = &[0x55, 0x04, 29];
/// `id-at-userPwd`, 2.5.4.85.
const USER_PWD: &[u8] = &[0x55, 0x04, 85];

fn value(bytes: &[u8]) -> Value<'_> {
    Reader::new(bytes)
        .expect("small")
        .next_value()
        .expect("well formed")
        .expect("one value")
}

/// The answer for one carried value of `kind` against one stated value, both encoded.
fn compare(kind: &[u8], carried: &[u8], stated: &[u8]) -> Match {
    let set = tlv(SET, carried);
    within(
        kind,
        &value(&set),
        &[Listed {
            kind,
            value: value(stated),
        }],
    )
}

/// The answer for two `UTF8String` values of `kind`.
fn text(kind: &[u8], carried: &str, stated: &str) -> Match {
    compare(
        kind,
        &tlv(UTF8, carried.as_bytes()),
        &tlv(UTF8, stated.as_bytes()),
    )
}

/// RFC 3492 section 7.1's samples (B) and (R), and (L) and (M), whose upper-case basic code
/// points the encoding keeps.
const CHINESE: &str = "\u{4ED6}\u{4EEC}\u{4E3A}\u{4EC0}\u{4E48}\u{4E0D}\u{8BF4}\u{4E2D}\u{6587}";
const JAPANESE: &str = "\u{305D}\u{306E}\u{30B9}\u{30D4}\u{30FC}\u{30C9}\u{3067}";

#[test]
fn punycode_encodes_and_decodes_rfc_3492_s_samples() {
    let samples = [
        (
            "\u{644}\u{64A}\u{647}\u{645}\u{627}\u{628}\u{62A}\u{643}\u{644}\u{645}\u{648}\u{634}\u{639}\u{631}\u{628}\u{64A}\u{61F}",
            "egbpdaj6bu4bxfgehfvwxn",
        ),
        (CHINESE, "ihqwcrb4cv8a8dqg056pqjye"),
        (
            "3\u{5E74}B\u{7D44}\u{91D1}\u{516B}\u{5148}\u{751F}",
            "3B-ww4c5e180e575a65lsy2b",
        ),
        (
            "\u{5B89}\u{5BA4}\u{5948}\u{7F8E}\u{6075}-with-SUPER-MONKEYS",
            "-with-SUPER-MONKEYS-pc58ag80a8qai00g7n9n",
        ),
        (JAPANESE, "d9juau41awczczp"),
    ];
    for (unicode, encoded) in samples {
        assert_eq!(punycode::encode(unicode).as_deref(), Some(encoded));
        assert_eq!(punycode::decode(encoded).as_deref(), Some(unicode));
        // Section 5: the digits are read in either case.
        assert_eq!(
            punycode::decode(&encoded.to_ascii_uppercase()).map(|text| text.to_lowercase()),
            Some(unicode.to_lowercase())
        );
    }
    // Section 6.2: a character that is no digit and a code point outside ASCII fail, and so does
    // section 6.4's overflow.
    assert_eq!(punycode::decode("ab!c"), None);
    assert_eq!(punycode::decode("\u{E9}"), None);
    assert_eq!(punycode::decode("zzzzzzzzzzzzzzzzzzzz"), None);
}

#[test]
fn rfc_5892_derives_each_code_point_s_property() {
    let cases = [
        // LDH (E), and an upper-case letter, which case folding changes (B).
        ('a', Property::Valid),
        ('-', Property::Valid),
        ('A', Property::Disallowed),
        // Exceptions (F): each of its three values.
        ('\u{DF}', Property::Valid),
        ('\u{B7}', Property::ContextO),
        ('\u{660}', Property::ContextO),
        ('\u{640}', Property::Disallowed),
        // JoinControl (H).
        ('\u{200C}', Property::ContextJ),
        // IgnorableProperties (C): a space; IgnorableBlocks (D); OldHangulJamo (I).
        (' ', Property::Disallowed),
        ('\u{20D0}', Property::Disallowed),
        ('\u{1100}', Property::Disallowed),
        // LetterDigits (A): a precomposed Hangul syllable, a Han ideograph.
        ('\u{AC00}', Property::Valid),
        ('\u{4E2D}', Property::Valid),
        // Appendix B's illustrative rows that no later Unicode version changes: a lower-case
        // letter and a symbol.
        ('\u{2184}', Property::Valid),
        ('\u{2190}', Property::Disallowed),
        // Unassigned (J).
        ('\u{378}', Property::Unassigned),
    ];
    for (c, property) in cases {
        assert_eq!(code_points::idna(c), property, "U+{:04X}", u32::from(c));
    }
}

#[test]
fn rfc_8264_derives_each_code_point_s_property_by_class() {
    let cases = [
        // ASCII7 (K) in both classes; SPACE is Spaces (N), ID_DIS or FREE_PVAL.
        ('A', Property::Valid, Property::Valid),
        (' ', Property::Disallowed, Property::Valid),
        // HasCompat (Q): ROMAN NUMERAL FOUR, RFC 7622 example 20's.
        ('\u{2163}', Property::Disallowed, Property::Valid),
        // Symbols (O): BLACK CHESS KING, examples 12 and 21.
        ('\u{265A}', Property::Disallowed, Property::Valid),
        // LetterDigits (A).
        ('\u{3C0}', Property::Valid, Property::Valid),
        // Controls (L) and PrecisIgnorableProperties (M).
        ('\u{7}', Property::Disallowed, Property::Disallowed),
        ('\u{FFFE}', Property::Disallowed, Property::Disallowed),
        ('\u{200D}', Property::ContextJ, Property::ContextJ),
    ];
    for (c, identifier, freeform) in cases {
        assert_eq!(code_points::precis(c, Class::Identifier), identifier);
        assert_eq!(code_points::precis(c, Class::Freeform), freeform);
    }
}

#[test]
fn rfc_5892_s_contextual_rules_decide_their_code_points() {
    let allowed = |text: &str| {
        let chars: Vec<char> = text.chars().collect();
        code_points::all_allowed(&chars, code_points::idna)
    };
    // A.1: ZERO WIDTH NON-JOINER after a virama (DEVANAGARI SIGN VIRAMA), and not at the start.
    assert!(allowed("\u{915}\u{94D}\u{200C}\u{937}"));
    assert!(!allowed("\u{200C}a"));
    // A.1's second clause: between a dual-joining and a right-joining Arabic letter.
    assert!(allowed("\u{628}\u{200C}\u{627}"));
    // A.3: MIDDLE DOT between two 'l's only.
    assert!(allowed("l\u{B7}l"));
    assert!(!allowed("a\u{B7}b"));
    // A.4: KERAIA before a Greek letter.
    assert!(allowed("\u{375}\u{3B1}"));
    assert!(!allowed("\u{375}a"));
    // A.5: GERESH after a Hebrew letter.
    assert!(allowed("\u{5D0}\u{5F3}"));
    assert!(!allowed("a\u{5F3}"));
    // A.7: KATAKANA MIDDLE DOT with Katakana in the label.
    assert!(allowed("\u{30A2}\u{30FB}\u{30A4}"));
    assert!(!allowed("a\u{30FB}b"));
    // A.8 and A.9: the two sets of Arabic-Indic digits never mixed.
    assert!(allowed("\u{660}\u{661}"));
    assert!(!allowed("\u{660}\u{6F1}"));
}

#[test]
fn the_bidi_rule_holds_each_label_to_one_direction() {
    let rule = |text: &str| code_points::bidi_rule(&text.chars().collect::<Vec<_>>());
    // Conditions 1 and 6: a left-to-right label begins and ends left to right.
    assert!(rule("example"));
    assert!(!rule("1example"));
    // Conditions 1 to 3: a Hebrew label; condition 5: Hebrew inside a left-to-right label.
    assert!(rule("\u{5E9}\u{5DC}\u{5D5}\u{5DD}"));
    assert!(!rule("a\u{5E9}"));
    // Condition 4: European and Arabic-Indic digits in one right-to-left label.
    assert!(!rule("\u{627}1\u{661}"));
}

#[test]
fn dns_name_match_compares_label_for_label() {
    // Item b): two labels of the first form compare by caseIgnoreMatch.
    assert_eq!(
        text(DNS_NAME, "WWW.Example.COM", "www.example.com"),
        Match::Yes
    );
    assert_eq!(
        text(DNS_NAME, "www.example.com", "www.example.org"),
        Match::No
    );
    // A different number of labels.
    assert_eq!(text(DNS_NAME, "example.com", "www.example.com"), Match::No);
    // Item c): a U-label against the A-label RFC 3492 section 7.1 encodes it to, in either case.
    let u_label = format!("{CHINESE}.example");
    for a_label in [
        "xn--ihqwcrb4cv8a8dqg056pqjye.example",
        "XN--IHQWCRB4CV8A8DQG056PQJYE.example",
    ] {
        assert_eq!(text(DNS_NAME, &u_label, a_label), Match::Yes);
        assert_eq!(text(DNS_NAME, a_label, &u_label), Match::Yes);
    }
    assert_eq!(
        text(
            DNS_NAME,
            &format!("{JAPANESE}.example"),
            "xn--d9juau41awczczp.example"
        ),
        Match::Yes
    );
    assert_eq!(
        text(DNS_NAME, &u_label, "xn--d9juau41awczczp.example"),
        Match::No
    );
    // Two U-labels by caseIgnoreMatch.
    assert_eq!(text(DNS_NAME, &u_label, &u_label), Match::Yes);
    // Item a): a label of the first form against an A-label, though the characters agree.
    assert_eq!(
        text(
            DNS_NAME,
            "ihqwcrb4cv8a8dqg056pqjye.example",
            "xn--ihqwcrb4cv8a8dqg056pqjye.example"
        ),
        Match::No
    );
}

#[test]
fn dns_name_match_s_wildcard_is_one_leftmost_label() {
    // Clause 8.9.2's note, both ways round.
    for name in ["a.example.com", "b.example.com"] {
        assert_eq!(text(DNS_NAME, name, "*.example.com"), Match::Yes);
        assert_eq!(text(DNS_NAME, "*.example.com", name), Match::Yes);
    }
    for name in ["a.b.example.com", "example.com"] {
        assert_eq!(text(DNS_NAME, name, "*.example.com"), Match::No);
    }
    assert_eq!(text(DNS_NAME, "*.example.com", "*.example.com"), Match::Yes);
    // Item d): two wildcards, or one not leftmost.
    assert_eq!(
        text(DNS_NAME, "a.b.example.com", "*.*.example.com"),
        Match::No
    );
    assert_eq!(
        text(DNS_NAME, "a.b.example.com", "a.*.example.com"),
        Match::No
    );
}

#[test]
fn a_name_outside_clause_6_2_15_s_syntax_is_undecided() {
    for outside in [
        // RFC 3492 sample (A) holds ARABIC QUESTION MARK, which RFC 5892 disallows, so its
        // A-label converts to no valid U-label.
        "xn--egbpdaj6bu4bxfgehfvwxn.example",
        // An upper-case letter in a U-label; a leading hyphen; a leading combining mark.
        "M\u{FC}nchen.example",
        "-\u{FC}.example",
        "\u{301}a\u{FC}.example",
        // A label that is not Normalization Form C.
        "mu\u{308}nchen.example",
        // MIDDLE DOT outside Catalan's 'l·l'.
        "a\u{B7}b.example",
        // An empty label, and a character no label holds.
        "example..com",
        "exa_mple.com",
        // A right-to-left label that breaks the Bidi Rule.
        "a\u{5E9}.example",
    ] {
        assert_eq!(
            text(DNS_NAME, outside, "example.example"),
            Match::Undecided,
            "{outside}"
        );
    }
    // A Hebrew label is right to left and keeps the rule; the name's other labels keep it too.
    assert_eq!(
        text(
            DNS_NAME,
            "\u{5E9}\u{5DC}\u{5D5}\u{5DD}.example",
            "\u{5E9}\u{5DC}\u{5D5}\u{5DD}.example"
        ),
        Match::Yes
    );
    // A value that is no UTF8String.
    assert_eq!(
        compare(
            DNS_NAME,
            &tlv(0x16, b"example.com"),
            &tlv(UTF8, b"example.com")
        ),
        Match::Undecided
    );
}

#[test]
fn int_email_match_compares_the_local_part_and_the_domain() {
    assert_eq!(
        text(INT_EMAIL, "Juliet@Example.COM", "juliet@example.com"),
        Match::Yes
    );
    assert_eq!(
        text(
            INT_EMAIL,
            &format!("juliet@{CHINESE}.example"),
            "JULIET@xn--ihqwcrb4cv8a8dqg056pqjye.example"
        ),
        Match::Yes
    );
    assert_eq!(
        text(INT_EMAIL, "romeo@example.com", "juliet@example.com"),
        Match::No
    );
    assert_eq!(
        text(INT_EMAIL, "juliet@example.org", "juliet@example.com"),
        Match::No
    );
    // Clause 6.2.16: no asterisk as a label; and an address needs an '@' and a local part.
    for outside in ["juliet@*.example.com", "juliet.example.com", "@example.com"] {
        assert_eq!(
            text(INT_EMAIL, outside, "juliet@example.com"),
            Match::Undecided,
            "{outside}"
        );
    }
}

#[test]
fn jid_match_compares_rfc_7622_s_enforced_parts() {
    // Table 1's examples 9 and 10 match once the capital sigma is lowered; 11's final sigma
    // matches neither; 6 and 7 differ, the sharp s not being "ss".
    assert_eq!(
        text(JID, "\u{3A3}@example.com/foo", "\u{3C3}@example.com/foo"),
        Match::Yes
    );
    assert_eq!(
        text(JID, "\u{3C2}@example.com/foo", "\u{3C3}@example.com/foo"),
        Match::No
    );
    assert_eq!(
        text(JID, "fussball@example.com", "fu\u{DF}ball@example.com"),
        Match::No
    );
    // Clause 8.9.4 items c) and e): a part one holds and the other does not (examples 1, 2, 13).
    assert_eq!(
        text(JID, "juliet@example.com", "juliet@example.com/foo"),
        Match::No
    );
    assert_eq!(text(JID, "example.com", "juliet@example.com"), Match::No);
    // Example 15: the '@' after the first '/' is the resourcepart's.
    assert_eq!(
        text(
            JID,
            "a.example.com/b@example.net",
            "a.example.com/b@example.net"
        ),
        Match::Yes
    );
    assert_eq!(
        text(JID, "a.example.com/b@example.net", "b@a.example.com"),
        Match::No
    );
    // Examples 3 and 12: a space and a symbol in a resourcepart; a non-ASCII space is SPACE.
    assert_eq!(
        text(
            JID,
            "juliet@example.com/foo\u{A0}bar",
            "juliet@example.com/foo bar"
        ),
        Match::Yes
    );
    assert_eq!(
        text(
            JID,
            "king@example.com/\u{265A}",
            "king@example.com/\u{265A}"
        ),
        Match::Yes
    );
    // The resourcepart keeps its case; the localpart is width-mapped and lowered; the domainpart
    // loses its final dot, is lowered, and an A-label is its U-label.
    assert_eq!(
        text(JID, "juliet@example.com/Foo", "juliet@example.com/foo"),
        Match::No
    );
    assert_eq!(
        text(
            JID,
            "\u{FF2A}\u{FF55}\u{FF4C}\u{FF49}\u{FF45}\u{FF54}@example.com",
            "juliet@example.com"
        ),
        Match::Yes
    );
    assert_eq!(
        text(JID, "juliet@EXAMPLE.com.", "juliet@example.com"),
        Match::Yes
    );
    assert_eq!(
        text(
            JID,
            "juliet@xn--ihqwcrb4cv8a8dqg056pqjye.example",
            &format!("juliet@{CHINESE}.example")
        ),
        Match::Yes
    );
}

#[test]
fn a_string_rfc_7622_does_not_admit_as_a_jid_is_undecided() {
    // Table 2's examples 16, 17, 19, 20, 21, 22 and 23.
    for illegal in [
        "\"juliet\"@example.com",
        "foo bar@example.com",
        "@example.com/",
        "henry\u{2163}@example.com",
        "\u{265A}@example.com",
        "juliet@",
        "/foobar",
    ] {
        assert_eq!(
            text(JID, illegal, "juliet@example.com"),
            Match::Undecided,
            "{illegal}"
        );
    }
}

#[test]
fn uri_match_normalises_as_rfc_3986_section_6_2_2_says() {
    // Section 6.2.2's example, every normalisation at once.
    assert_eq!(
        text(
            URI,
            "example://a/b/c/%7Bfoo%7D",
            "eXAMPLE://a/./b/../b/%63/%7bfoo%7d"
        ),
        Match::Yes
    );
    // Section 6.2.2.1's: scheme and host case.
    assert_eq!(
        text(URI, "HTTP://www.EXAMPLE.com/", "http://www.example.com/"),
        Match::Yes
    );
    // Section 6.2.3's four equivalent forms.
    let forms = [
        "http://example.com",
        "http://example.com/",
        "http://example.com:/",
        "http://example.com:80/",
    ];
    for left in forms {
        for right in forms {
            assert_eq!(text(URI, left, right), Match::Yes, "{left} {right}");
        }
    }
    // Section 5.2.4's examples of remove_dot_segments.
    assert_eq!(uri::remove_dot_segments("/a/b/c/./../../g"), "/a/g");
    assert_eq!(uri::remove_dot_segments("mid/content=5/../6"), "mid/6");
    // The path's case is significant; a reserved character's encoding is not decoded.
    assert_eq!(
        text(URI, "http://example.com/A", "http://example.com/a"),
        Match::No
    );
    assert_eq!(text(URI, "http://a/b%2fc", "http://a/b/c"), Match::No);
    assert_eq!(text(URI, "http://a/b%2fc", "http://a/b%2Fc"), Match::Yes);
    // Defaults are the scheme's own; an empty query is removed (clause 8.9.1 item d)).
    assert_eq!(
        text(URI, "https://example.com:443/x", "https://example.com/x"),
        Match::Yes
    );
    assert_eq!(
        text(URI, "http://example.com:443/", "http://example.com/"),
        Match::No
    );
    assert_eq!(text(URI, "http://a/b?", "http://a/b"), Match::Yes);
    // No URI: outside ASCII, a broken triplet, no scheme.
    for outside in ["http://ex\u{E4}mple.com/", "http://a/%G1", "//a/b"] {
        assert_eq!(
            text(URI, outside, "http://a/"),
            Match::Undecided,
            "{outside}"
        );
    }
}

/// A `PresentationAddress` under explicit tags: the selectors given, and the network addresses.
fn presentation(selectors: [Option<&[u8]>; 3], addresses: &[&[u8]]) -> Vec<u8> {
    let mut members: Vec<Vec<u8>> = selectors
        .iter()
        .enumerate()
        .filter_map(|(number, selector)| {
            let tag = 0xA0 | u8::try_from(number).expect("three selectors");
            selector.map(|octets| tlv(tag, &tlv(OCTETS, octets)))
        })
        .collect();
    let set: Vec<u8> = addresses
        .iter()
        .flat_map(|address| tlv(OCTETS, address))
        .collect();
    members.push(tlv(0xA3, &tlv(SET, &set)));
    seq(&members)
}

#[test]
fn presentation_address_match_wants_equal_selectors_and_a_subset() {
    let stored = presentation([Some(b"p"), None, Some(b"t")], &[b"one", b"two"]);
    let compare_to = |presented: Vec<u8>| compare(PRESENTATION_ADDRESS, &stored, &presented);
    assert_eq!(
        compare_to(presentation([Some(b"p"), None, Some(b"t")], &[b"two"])),
        Match::Yes
    );
    assert_eq!(
        compare_to(presentation(
            [Some(b"p"), None, Some(b"t")],
            &[b"one", b"two"]
        )),
        Match::Yes
    );
    // An address the stored value lacks; a selector that differs, or that only one states.
    assert_eq!(
        compare_to(presentation([Some(b"p"), None, Some(b"t")], &[b"three"])),
        Match::No
    );
    assert_eq!(
        compare_to(presentation([Some(b"q"), None, Some(b"t")], &[b"one"])),
        Match::No
    );
    assert_eq!(
        compare_to(presentation([Some(b"p"), None, None], &[b"one"])),
        Match::No
    );
    // No network address is outside the type, which asks for at least one.
    assert_eq!(
        compare_to(presentation([Some(b"p"), None, Some(b"t")], &[])),
        Match::Undecided
    );
}

#[test]
fn user_pwd_match_compares_clear_and_encrypted_passwords() {
    let clear = |text: &str| tlv(UTF8, text.as_bytes());
    let algorithm = |arc: u8| seq(&[tlv(0x06, &[0x2A, 0x03, arc])]);
    let encrypted = |arc: u8, octets: &[u8]| seq(&[algorithm(arc), tlv(OCTETS, octets)]);
    // Clear against clear, by caseExactMatch.
    assert_eq!(
        compare(USER_PWD, &clear("Secret"), &clear("Secret")),
        Match::Yes
    );
    assert_eq!(
        compare(USER_PWD, &clear("Secret"), &clear("secret")),
        Match::No
    );
    // Encrypted against encrypted: the algorithms first, then the octets.
    assert_eq!(
        compare(USER_PWD, &encrypted(1, b"ab"), &encrypted(1, b"ab")),
        Match::Yes
    );
    assert_eq!(
        compare(USER_PWD, &encrypted(1, b"ab"), &encrypted(1, b"ac")),
        Match::No
    );
    assert_eq!(
        compare(USER_PWD, &encrypted(1, b"ab"), &encrypted(2, b"ab")),
        Match::No
    );
    // Clear against encrypted needs an encryption no Recommendation defines.
    assert_eq!(
        compare(USER_PWD, &encrypted(1, b"ab"), &clear("Secret")),
        Match::Undecided
    );
    assert_eq!(
        compare(USER_PWD, &clear("Secret"), &encrypted(1, b"ab")),
        Match::Undecided
    );
}
