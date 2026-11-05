//! Canonical XML 1.0 over hand-built documents, each expected form derived from the section of
//! the Recommendation its test names.

use super::*;

/// The root's child `P` in the namespace `urn:d`.
const P: Apex<'static> = Apex {
    depth: 1,
    namespaces: &["urn:d"],
    local: "P",
};

/// The document element itself, which makes the subset the whole element tree.
const ROOT: Apex<'static> = Apex {
    depth: 0,
    namespaces: &[""],
    local: "doc",
};

/// The inclusive Canonical XML 1.0 form, which every test before the two later
/// Recommendations' is about.
fn canonical(
    octets: &str,
    apex: &Apex<'_>,
    comments: Comments,
) -> Result<String, PolicyDocumentError> {
    canonical_by(
        octets,
        apex,
        &Method::plain(Algorithm::Canonical10, comments),
    )
}

/// The form `method` computes.
fn canonical_by(
    octets: &str,
    apex: &Apex<'_>,
    method: &Method<'_>,
) -> Result<String, PolicyDocumentError> {
    let mut out = Vec::new();
    canonicalize(octets.as_bytes(), apex, method, &mut |piece| {
        out.extend_from_slice(piece);
    })?;
    Ok(String::from_utf8(out).expect("the canonical form is UTF-8"))
}

/// One document that meets every rendering rule a subset can: namespaces declared on an ancestor
/// outside the subset, one re-declared, an `xml` attribute inherited, an undeclared default, an
/// empty element, a comment, a processing instruction, a CDATA section and references.
const SUBSET: &str = concat!(
    r#"<r xmlns="urn:d" xmlns:b="urn:b" xmlns:a="urn:a" xml:lang="en"><x/>"#,
    r#"<P b:z="1" a:y="2" z="3" a="&#x9;&quot;&lt;" xmlns:a="urn:a">"#,
    r#"<e xmlns=""><f xmlns="urn:d"/></e><!-- c --><?pi  data ?><![CDATA[<&>]]>t&#xD;&gt;</P></r>"#,
);

#[test]
fn a_subset_renders_its_inherited_namespaces_and_xml_attributes_in_section_2_order() {
    // Section 2.3: the apex renders every namespace in scope, `a` once although it is declared
    // twice; section 2.2: the default first, then by prefix; attributes unqualified first, then
    // by namespace URI, which puts the XML namespace's `http:` before `urn:`; section 2.4: the
    // root's `xml:lang` is copied onto the apex. Below it, `xmlns=""` is rendered where the
    // parent had a default and `urn:d` again where it is restored; the empty element becomes a
    // pair, the comment is dropped, the processing instruction keeps its data after one space,
    // and the CDATA section and the references become escaped characters.
    assert_eq!(
        canonical(SUBSET, &P, Comments::Omitted).expect("well-formed"),
        concat!(
            r#"<P xmlns="urn:d" xmlns:a="urn:a" xmlns:b="urn:b" a="&#x9;&quot;&lt;" z="3" "#,
            r#"xml:lang="en" a:y="2" b:z="1"><e xmlns=""><f xmlns="urn:d"></f></e>"#,
            r#"<?pi data ?>&lt;&amp;&gt;t&#xD;&gt;</P>"#,
        )
    );
}

#[test]
fn the_with_comments_form_keeps_the_comment_where_it_stands() {
    let kept = canonical(SUBSET, &P, Comments::Kept).expect("well-formed");
    assert!(kept.contains("</e><!-- c --><?pi data ?>"), "{kept}");
}

#[test]
fn an_attribute_of_the_xml_namespace_on_the_apex_itself_is_not_inherited_over() {
    // Section 2.4 copies only the names the apex does not carry.
    let octets = r#"<r xmlns="urn:d" xml:lang="en" xml:space="preserve"><P xml:lang="de"/></r>"#;
    assert_eq!(
        canonical(octets, &P, Comments::Omitted).expect("well-formed"),
        r#"<P xmlns="urn:d" xml:lang="de" xml:space="preserve"></P>"#
    );
}

#[test]
fn whitespace_in_content_is_kept_as_example_3_2_keeps_it() {
    // Section 3.2's input, whose canonical form is itself: no DTD, so every white space
    // character is content.
    let octets = "<doc>\n   <clean>   </clean>\n   <dirty>   A   B   </dirty>\n   <mixed>\n      A\n      <clean>   </clean>\n      B\n      <dirty>   A   B   </dirty>\n      C\n   </mixed>\n</doc>";
    assert_eq!(
        canonical(octets, &ROOT, Comments::Omitted).expect("well-formed"),
        octets
    );
}

#[test]
fn line_breaks_and_attribute_white_space_are_normalised_before_rendering() {
    // Section 2.1 takes XML 1.0's line-end handling and attribute-value normalisation: a CR LF
    // pair and a lone CR are LF in content, a literal break or tab in a value is a space, and a
    // character reference keeps its character, which section 2.3 then escapes.
    let octets = "<doc a=\"x\r\ny\tz&#xA;w\">a\r\nb\rc</doc>";
    assert_eq!(
        canonical(octets, &ROOT, Comments::Omitted).expect("well-formed"),
        "<doc a=\"x y z&#xA;w\">a\nb\nc</doc>"
    );
}

#[test]
fn a_character_outside_ascii_is_written_as_its_utf_8() {
    // Section 3.6: a character reference becomes the character's UTF-8 octets.
    assert_eq!(
        canonical("<doc>&#169;</doc>", &ROOT, Comments::Omitted).expect("well-formed"),
        "<doc>\u{a9}</doc>"
    );
}

#[test]
fn the_six_identifiers_of_the_three_recommendations_are_placed_and_no_other() {
    // XML Signature 1.1 sections 6.5.1 to 6.5.3, and Exclusive XML Canonicalization's section 4.
    for (uri, algorithm, comments) in [
        (C14N, Algorithm::Canonical10, Comments::Omitted),
        (C14N_WITH_COMMENTS, Algorithm::Canonical10, Comments::Kept),
        (
            "http://www.w3.org/2001/10/xml-exc-c14n#",
            Algorithm::Exclusive10,
            Comments::Omitted,
        ),
        (
            "http://www.w3.org/2001/10/xml-exc-c14n#WithComments",
            Algorithm::Exclusive10,
            Comments::Kept,
        ),
        (
            "http://www.w3.org/2006/12/xml-c14n11",
            Algorithm::Canonical11,
            Comments::Omitted,
        ),
        (
            "http://www.w3.org/2006/12/xml-c14n11#WithComments",
            Algorithm::Canonical11,
            Comments::Kept,
        ),
    ] {
        assert_eq!(
            Method::of(uri, Some("a")),
            Some(Method {
                algorithm,
                comments,
                prefix_list: Some("a"),
            }),
            "{uri}"
        );
    }
    assert_eq!(
        Method::of("http://www.w3.org/2006/12/xml-c14n11#", None),
        None
    );
    assert_eq!(
        Method::of("http://www.w3.org/TR/1999/REC-xpath-19991116", None),
        None
    );
}

#[test]
fn what_the_module_names_as_refused_is_refused() {
    let refusals = [
        // Section 2.1's note.
        r#"<doc xmlns:a="relative"/>"#,
        // XML 1.0 section 4.1: no declaration defines the entity.
        "<doc>&nbsp;</doc>",
        "<doc>&#xFFFE;</doc>",
        r#"<!DOCTYPE doc [<!ATTLIST doc a CDATA "d">]><doc/>"#,
        r#"<?xml version="1.0" encoding="ISO-8859-1"?><doc/>"#,
        // XML Namespaces 1.0 section 6.3: one expanded name twice.
        r#"<doc xmlns:a="urn:x" xmlns:b="urn:x" a:k="1" b:k="2"/>"#,
        r#"<doc a:k="1"/>"#,
        r#"<doc xmlns:a=""/>"#,
        "<doc><a></b></doc>",
        // XML 1.0 section 2.6, productions [16] and [17]: the fuzz target's finding, whose form
        // `<?xml <?a?>` read back as a misplaced declaration, and a target run into its data.
        "<doc><?xml<?a?></doc>",
        "<doc><?XmL data?></doc>",
        "<doc><?pi<data?></doc>",
    ];
    for octets in refusals {
        assert!(
            matches!(
                canonical(octets, &ROOT, Comments::Omitted),
                Err(PolicyDocumentError::Xml(_))
            ),
            "{octets}"
        );
    }
    assert!(matches!(
        canonical("<doc/>", &P, Comments::Omitted),
        Err(PolicyDocumentError::ElementMalformed("PolicyComponents"))
    ));
}

#[test]
fn the_three_budgets_hold_at_their_bounds_and_refuse_past_them() {
    let nested = |depth: usize| {
        let mut octets = "<doc>".to_owned();
        octets.push_str(&"<a>".repeat(depth.saturating_sub(1)));
        octets.push_str(&"</a>".repeat(depth.saturating_sub(1)));
        octets.push_str("</doc>");
        octets
    };
    let deepest = usize::from(der::MAX_DEPTH);
    assert!(canonical(&nested(deepest), &ROOT, Comments::Omitted).is_ok());
    assert_eq!(
        canonical(&nested(deepest + 1), &ROOT, Comments::Omitted),
        Err(PolicyDocumentError::NestedTooDeep)
    );
    let wide = |count: usize| format!("<doc>{}</doc>", "<a/>".repeat(count.saturating_sub(1)));
    assert!(canonical(&wide(MAX_ENTRIES), &ROOT, Comments::Omitted).is_ok());
    assert!(matches!(
        canonical(&wide(MAX_ENTRIES + 1), &ROOT, Comments::Omitted),
        Err(PolicyDocumentError::TooManyEntries)
    ));
    let large = format!("<doc>{}</doc>", " ".repeat(MAX_POLICY_OCTETS));
    assert!(matches!(
        canonical(&large, &ROOT, Comments::Omitted),
        Err(PolicyDocumentError::TooLarge { .. })
    ));
}

/// The memory probe's population, ignored by the ordinary run: documents the reader's budgets
/// admit, each canonicalised whole a hundred times — `xml` attributes filling an ancestor, the
/// same split between the ancestor and the apex, the entry budget spent on declarations, the
/// depth budget with every level re-declaring, one start tag of attributes filling the bytes,
/// the depth budget with every level using each prefix it declares, and an `xml:base` on every
/// ancestor of an apex at the depth budget — each under all three methods.
#[test]
#[ignore = "the memory probe's population, run under getrusage"]
fn hostile_documents_are_canonicalised_within_their_budgets() {
    // Writing into a `String` cannot fail, which is why each `write!` result is discarded.
    use std::fmt::Write as _;
    let room = MAX_POLICY_OCTETS.saturating_sub(64);
    let repeated = |count: usize, item: &dyn Fn(&mut String, usize)| -> String {
        let mut out = String::new();
        for n in 0..count {
            item(&mut out, n);
        }
        out
    };
    let xml_attributes = |prefix: &str, count: usize| -> String {
        repeated(count, &|out, n| {
            let _ = write!(out, " xml:{prefix}{n}=\"\"");
        })
    };
    let ancestor = format!(
        "<r xmlns=\"urn:d\"{}><P/></r>",
        xml_attributes("a", room / 15)
    );
    let split = format!(
        "<r xmlns=\"urn:d\"{}><P{}/></r>",
        xml_attributes("a", room / 30),
        xml_attributes("b", room / 30)
    );
    let declared = repeated(MAX_ENTRIES - 3, &|out, n| {
        let _ = write!(out, " xmlns:p{n}=\"urn:{n}\"");
    });
    let declarations = format!("<r xmlns=\"urn:d\"{declared}><P/></r>");
    let per_level = (MAX_ENTRIES / usize::from(der::MAX_DEPTH)).saturating_sub(2);
    let mut deep = "<r xmlns=\"urn:d\">".to_owned();
    for level in 1..usize::from(der::MAX_DEPTH) {
        deep.push_str("<P");
        for n in 0..per_level {
            let _ = write!(deep, " xmlns:p{n}=\"urn:{level}.{n}\"");
        }
        deep.push('>');
    }
    deep.push_str(&"</P>".repeat(usize::from(der::MAX_DEPTH) - 1));
    deep.push_str("</r>");
    let attributes = repeated(room / 12, &|out, n| {
        let _ = write!(out, " a{n}=\"\"");
    });
    let wide = format!("<r xmlns=\"urn:d\"><P{attributes}/></r>");
    // Exclusive canonicalisation's visibly-utilized prefixes: every level re-declares and uses
    // each of its prefixes, so each is looked up and pushed at every level.
    let mut utilized = "<r xmlns=\"urn:d\">".to_owned();
    for level in 1..usize::from(der::MAX_DEPTH) {
        utilized.push_str("<P");
        for n in 0..per_level {
            let _ = write!(utilized, " xmlns:p{n}=\"urn:{level}.{n}\" p{n}:a=\"\"");
        }
        utilized.push('>');
    }
    utilized.push_str(&"</P>".repeat(usize::from(der::MAX_DEPTH) - 1));
    utilized.push_str("</r>");
    // Canonical XML 1.1's `xml:base` fix-up: every ancestor of the apex carries a value of
    // `../` segments, which the join keeps and so carries to the next, the longest it can be.
    let levels = usize::from(der::MAX_DEPTH) - 1;
    let climb = "../".repeat(room / (levels + 1) / 3 - 16);
    let mut bases = String::new();
    for _ in 0..levels {
        let _ = write!(bases, "<P xmlns=\"urn:d\" xml:base=\"{climb}\">");
    }
    bases.push_str("<P xml:base=\"x\"/>");
    bases.push_str(&"</P>".repeat(levels));
    let deepest = Apex {
        depth: levels,
        namespaces: &["urn:d"],
        local: "P",
    };
    for (name, document, apex) in [
        ("ancestor", &ancestor, &P),
        ("split", &split, &P),
        ("declarations", &declarations, &P),
        ("deep", &deep, &P),
        ("wide", &wide, &P),
        ("utilized", &utilized, &P),
        ("bases", &bases, &deepest),
    ] {
        assert!(
            document.len() <= MAX_POLICY_OCTETS,
            "{name} is past the bytes"
        );
        for algorithm in [
            Algorithm::Canonical10,
            Algorithm::Exclusive10,
            Algorithm::Canonical11,
        ] {
            let started = std::time::Instant::now();
            let mut written = 0_usize;
            for _ in 0..100 {
                let method = Method::plain(algorithm, Comments::Omitted);
                canonicalize(document.as_bytes(), apex, &method, &mut |piece| {
                    written += piece.len();
                })
                .unwrap_or_else(|error| panic!("{name} is refused: {error:?}"));
            }
            println!(
                "{name} under {algorithm:?}: {} octets in, {} out per run, {:?} for 100 runs",
                document.len(),
                written / 100,
                started.elapsed()
            );
        }
    }
}

/// Exclusive XML Canonicalization 1.0 without comments and without a `PrefixList`.
const EXCLUSIVE: Method<'static> = Method::plain(Algorithm::Exclusive10, Comments::Omitted);

/// Canonical XML 1.1 without comments.
const VERSION_11: Method<'static> = Method::plain(Algorithm::Canonical11, Comments::Omitted);

/// `algorithm` without comments with `prefix_list` as its parameter.
const fn listing(algorithm: Algorithm, prefix_list: &str) -> Method<'_> {
    Method {
        algorithm,
        comments: Comments::Omitted,
        prefix_list: Some(prefix_list),
    }
}

/// Exclusive XML Canonicalization section 2.1's element `n1:elem1`, below the root.
const ELEM1: Apex<'static> = Apex {
    depth: 1,
    namespaces: &["http://b.example"],
    local: "elem1",
};

#[test]
fn exclusive_section_2_1_s_enveloped_element_canonicalises_as_it_did_alone() {
    // Section 2.1: the first document is in canonical form; enveloped, inclusive Canonical XML
    // renders the envelope's `n0` on the element, and the exclusive form is the first document's.
    let alone = "<n1:elem1 xmlns:n1=\"http://b.example\">\n       content\n   </n1:elem1>";
    let enveloped = concat!(
        "<n0:pdu xmlns:n0=\"http://a.example\">\n      ",
        "<n1:elem1 xmlns:n1=\"http://b.example\">\n       content\n   </n1:elem1>\n</n0:pdu>",
    );
    let at_root = Apex { depth: 0, ..ELEM1 };
    assert_eq!(
        canonical_by(alone, &at_root, &EXCLUSIVE).as_deref(),
        Ok(alone)
    );
    assert_eq!(
        canonical(alone, &at_root, Comments::Omitted).as_deref(),
        Ok(alone)
    );
    assert_eq!(
        canonical(enveloped, &ELEM1, Comments::Omitted).as_deref(),
        Ok(concat!(
            "<n1:elem1 xmlns:n0=\"http://a.example\" xmlns:n1=\"http://b.example\">",
            "\n       content\n   </n1:elem1>",
        ))
    );
    assert_eq!(
        canonical_by(enveloped, &ELEM1, &EXCLUSIVE).as_deref(),
        Ok(alone)
    );
}

/// Exclusive XML Canonicalization section 2.2's two envelopes of one `n1:elem2`.
const LOCAL: &str = concat!(
    r#"<n0:local xmlns:n0="foo:bar" xmlns:n3="ftp://example.org">"#,
    r#"<n1:elem2 xmlns:n1="http://example.net" xml:lang="en">"#,
    r#"<n3:stuff xmlns:n3="ftp://example.org"/></n1:elem2></n0:local>"#,
);
const PDU: &str = concat!(
    r#"<n2:pdu xmlns:n1="http://example.com" xmlns:n2="http://foo.example" xml:lang="fr" "#,
    r#"xml:space="retain"><n1:elem2 xmlns:n1="http://example.net" xml:lang="en">"#,
    r#"<n3:stuff xmlns:n3="ftp://example.org"/></n1:elem2></n2:pdu>"#,
);
const ELEM2: Apex<'static> = Apex {
    depth: 1,
    namespaces: &["http://example.net"],
    local: "elem2",
};

#[test]
fn exclusive_section_2_2_s_two_envelopes_give_one_form_where_inclusive_gives_two() {
    // Section 2.2's three serialisations, its line wrapping undone. Inclusive Canonical XML 1.0
    // elevates `n3` to the apex in the first envelope and copies `xml:space` in the second;
    // Canonical XML 1.1 section 2.4 copies `xml:space` too, it being simple inheritable.
    let first = concat!(
        r#"<n1:elem2 xmlns:n0="foo:bar" xmlns:n1="http://example.net" "#,
        r#"xmlns:n3="ftp://example.org" xml:lang="en"><n3:stuff></n3:stuff></n1:elem2>"#,
    );
    let second = concat!(
        r#"<n1:elem2 xmlns:n1="http://example.net" xmlns:n2="http://foo.example" "#,
        r#"xml:lang="en" xml:space="retain">"#,
        r#"<n3:stuff xmlns:n3="ftp://example.org"></n3:stuff></n1:elem2>"#,
    );
    let exclusive = concat!(
        r#"<n1:elem2 xmlns:n1="http://example.net" xml:lang="en">"#,
        r#"<n3:stuff xmlns:n3="ftp://example.org"></n3:stuff></n1:elem2>"#,
    );
    assert_eq!(
        canonical(LOCAL, &ELEM2, Comments::Omitted).as_deref(),
        Ok(first)
    );
    assert_eq!(
        canonical(PDU, &ELEM2, Comments::Omitted).as_deref(),
        Ok(second)
    );
    assert_eq!(
        canonical_by(LOCAL, &ELEM2, &VERSION_11).as_deref(),
        Ok(first)
    );
    assert_eq!(
        canonical_by(PDU, &ELEM2, &VERSION_11).as_deref(),
        Ok(second)
    );
    assert_eq!(
        canonical_by(LOCAL, &ELEM2, &EXCLUSIVE).as_deref(),
        Ok(exclusive)
    );
    assert_eq!(
        canonical_by(PDU, &ELEM2, &EXCLUSIVE).as_deref(),
        Ok(exclusive)
    );
}

#[test]
fn a_prefix_list_hands_its_prefixes_to_the_inclusive_rules() {
    // Section 4: a listed prefix is handled as Canonical XML 1.0 handles it, so the apex renders
    // it from the envelope wherever it is in scope, and a descendant re-declaring it with the
    // same value does not; an unlisted one keeps section 3's rule.
    let method = listing(Algorithm::Exclusive10, "n3");
    assert_eq!(
        canonical_by(LOCAL, &ELEM2, &method).as_deref(),
        Ok(concat!(
            r#"<n1:elem2 xmlns:n1="http://example.net" xmlns:n3="ftp://example.org" "#,
            r#"xml:lang="en"><n3:stuff></n3:stuff></n1:elem2>"#,
        ))
    );
    let method = listing(Algorithm::Exclusive10, " n0\tabsent ");
    assert_eq!(
        canonical_by(LOCAL, &ELEM2, &method).as_deref(),
        Ok(concat!(
            r#"<n1:elem2 xmlns:n0="foo:bar" xmlns:n1="http://example.net" xml:lang="en">"#,
            r#"<n3:stuff xmlns:n3="ftp://example.org"></n3:stuff></n1:elem2>"#,
        ))
    );
    // `#default` lists the default namespace: the apex renders it though it uses only `a`.
    let octets = r#"<r xmlns="urn:d" xmlns:a="urn:a"><a:P><e/></a:P></r>"#;
    let apex = Apex {
        depth: 1,
        namespaces: &["urn:a"],
        local: "P",
    };
    assert_eq!(
        canonical_by(octets, &apex, &EXCLUSIVE).as_deref(),
        Ok(r#"<a:P xmlns:a="urn:a"><e xmlns="urn:d"></e></a:P>"#)
    );
    assert_eq!(
        canonical_by(octets, &apex, &listing(Algorithm::Exclusive10, "#default")).as_deref(),
        Ok(r#"<a:P xmlns="urn:d" xmlns:a="urn:a"><e></e></a:P>"#)
    );
    // The parameter is exclusive canonicalisation's alone.
    assert_eq!(
        canonical_by(LOCAL, &ELEM2, &listing(Algorithm::Canonical10, "")),
        canonical(LOCAL, &ELEM2, Comments::Omitted)
    );
}

#[test]
fn exclusive_renders_a_namespace_where_it_is_visibly_utilized_and_not_in_effect_above() {
    // Section 1.1's visible utilization and section 3's three conditions: `b` is declared on
    // the apex and used by nobody, so it is never rendered; `a` is not used by the apex, so it is
    // rendered on each sibling that uses it, by its element name or by an attribute only; the
    // apex's default is rendered once and is in effect below; `xmlns=""` is rendered on the one
    // element that leaves a rendered default, and not again on its child; an unprefixed
    // attribute utilizes nothing.
    let octets = concat!(
        r#"<r xmlns="urn:d"><P xmlns:a="urn:a" xmlns:b="urn:b" z="1">"#,
        r#"<a:x><y/></a:x><w a:k="2"/><e xmlns=""><f/></e></P></r>"#,
    );
    assert_eq!(
        canonical_by(octets, &P, &EXCLUSIVE).as_deref(),
        Ok(concat!(
            r#"<P xmlns="urn:d" z="1"><a:x xmlns:a="urn:a"><y></y></a:x>"#,
            r#"<w xmlns:a="urn:a" a:k="2"></w><e xmlns=""><f></f></e></P>"#,
        ))
    );
}

/// Canonical XML 1.1 section 3.8's input, its document type declaration left out and the
/// `xml:space` default it declared on `e2` written there instead, since this reader refuses a
/// document type declaration (the module comment says why).
const SECTION_3_8: &str = concat!(
    r#"<doc xmlns="http://www.ietf.org" xmlns:w3c="http://www.w3.org" xml:base="something/else">"#,
    r#"<e1><e2 xmlns="" xml:id="abc" xml:base="bar/" xml:space="preserve">"#,
    r#"<e3 id="E3" xml:base="foo"/></e2></e1></doc>"#,
);

#[test]
fn section_3_8_s_apex_start_tag_is_rendered_as_the_recommendation_renders_it() {
    // Section 3.8's subset has `e1` as its apex, under the omitted `doc`: the canonical form
    // opens `<e1 xmlns="http://www.ietf.org" xmlns:w3c="http://www.w3.org"
    // xml:base="something/else">`, the root's `xml:base` fixed up onto it. This subset keeps
    // `e1`'s descendants, which are no orphans and are rendered as written.
    let e1 = Apex {
        depth: 1,
        namespaces: &["http://www.ietf.org"],
        local: "e1",
    };
    assert_eq!(
        canonical_by(SECTION_3_8, &e1, &VERSION_11).as_deref(),
        Ok(concat!(
            r#"<e1 xmlns="http://www.ietf.org" xmlns:w3c="http://www.w3.org" "#,
            r#"xml:base="something/else"><e2 xmlns="" xml:base="bar/" xml:id="abc" "#,
            r#"xml:space="preserve"><e3 id="E3" xml:base="foo"></e3></e2></e1>"#,
        ))
    );
}

#[test]
fn section_3_8_s_e3_takes_xml_space_and_the_joined_base_and_not_xml_id() {
    // `e3` as the apex: every ancestor is omitted, so the fix-up folds "something/else", "bar/"
    // and "foo" into "something/bar/foo" — the value section 3.8 gives `e3` — `xml:space` is
    // copied, and `xml:id` is not (section 3.8's "xml:id not inherited"). Canonical XML 1.0
    // copies `xml:id` and keeps `e3`'s own `xml:base`; exclusive canonicalisation copies nothing
    // and renders no namespace `e3` does not use.
    let e3 = Apex {
        depth: 3,
        namespaces: &[""],
        local: "e3",
    };
    assert_eq!(
        canonical_by(SECTION_3_8, &e3, &VERSION_11).as_deref(),
        Ok(concat!(
            r#"<e3 xmlns:w3c="http://www.w3.org" id="E3" xml:base="something/bar/foo" "#,
            r#"xml:space="preserve"></e3>"#,
        ))
    );
    assert_eq!(
        canonical(SECTION_3_8, &e3, Comments::Omitted).as_deref(),
        Ok(concat!(
            r#"<e3 xmlns:w3c="http://www.w3.org" id="E3" xml:base="foo" xml:id="abc" "#,
            r#"xml:space="preserve"></e3>"#,
        ))
    );
    assert_eq!(
        canonical_by(SECTION_3_8, &e3, &EXCLUSIVE).as_deref(),
        Ok(r#"<e3 id="E3" xml:base="foo"></e3>"#)
    );
}

#[test]
fn version_1_1_copies_only_the_simple_inheritable_attributes_and_drops_an_empty_base() {
    // Section 2.4: of the `xml` attributes an ancestor carries, `xml:lang` and `xml:space` are
    // copied, `xml:id` is not, and any other is an ordinary attribute and stays where it is; a
    // fix-up that comes out empty — section 2.4's "abc/" and "../" — is not rendered, the apex's
    // own value included; with no `xml:base` on an ancestor the apex's own is left as it is.
    let octets = concat!(
        r#"<r xmlns="urn:d" xml:foo="1" xml:id="i" xml:lang="en" xml:base="abc/">"#,
        r#"<P xml:space="default" xml:base="../"/></r>"#,
    );
    assert_eq!(
        canonical_by(octets, &P, &VERSION_11).as_deref(),
        Ok(r#"<P xmlns="urn:d" xml:lang="en" xml:space="default"></P>"#)
    );
    assert_eq!(
        canonical(octets, &P, Comments::Omitted).as_deref(),
        Ok(concat!(
            r#"<P xmlns="urn:d" xml:base="../" xml:foo="1" xml:id="i" xml:lang="en" "#,
            r#"xml:space="default"></P>"#,
        ))
    );
    let unbased = r#"<r xmlns="urn:d"><P xml:base="../x"/></r>"#;
    assert_eq!(
        canonical_by(unbased, &P, &VERSION_11).as_deref(),
        Ok(r#"<P xmlns="urn:d" xml:base="../x"></P>"#)
    );
}

#[test]
fn each_later_method_keeps_comments_only_with_comments() {
    let octets = r#"<r xmlns="urn:d"><!--out--><P><!--in--></P></r>"#;
    for algorithm in [Algorithm::Exclusive10, Algorithm::Canonical11] {
        assert_eq!(
            canonical_by(octets, &P, &Method::plain(algorithm, Comments::Kept)).as_deref(),
            Ok(r#"<P xmlns="urn:d"><!--in--></P>"#)
        );
        assert_eq!(
            canonical_by(octets, &P, &Method::plain(algorithm, Comments::Omitted)).as_deref(),
            Ok(r#"<P xmlns="urn:d"></P>"#)
        );
    }
}

#[test]
fn the_later_methods_refuse_what_the_first_refuses_and_charge_the_prefix_list() {
    // Both define themselves on Canonical XML 1.0's data model, and Canonical XML 1.1 section
    // 2.1 repeats the requirement to fail on a relative namespace URI.
    let relative = r#"<r xmlns="urn:d"><P xmlns:a="rel/a"/></r>"#;
    for method in [EXCLUSIVE, VERSION_11] {
        assert!(matches!(
            canonical_by(relative, &P, &method),
            Err(PolicyDocumentError::Xml(_))
        ));
    }
    // Each listed prefix is one entry of the budget, charged before the work: one past it is
    // refused before the document is read.
    let full = "p ".repeat(MAX_ENTRIES + 1);
    assert_eq!(
        canonical_by("<doc/>", &ROOT, &listing(Algorithm::Exclusive10, &full)),
        Err(PolicyDocumentError::TooManyEntries)
    );
}
