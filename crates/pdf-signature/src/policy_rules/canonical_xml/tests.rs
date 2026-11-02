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

fn canonical(
    octets: &str,
    apex: &Apex<'_>,
    comments: Comments,
) -> Result<String, PolicyDocumentError> {
    let mut out = Vec::new();
    canonicalize(octets.as_bytes(), apex, comments, &mut |piece| {
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
fn the_two_inclusive_methods_are_placed_and_no_other() {
    assert_eq!(Comments::of(C14N), Some(Comments::Omitted));
    assert_eq!(Comments::of(C14N_WITH_COMMENTS), Some(Comments::Kept));
    assert_eq!(
        Comments::of("http://www.w3.org/2001/10/xml-exc-c14n#"),
        None
    );
    assert_eq!(Comments::of("http://www.w3.org/2006/12/xml-c14n11"), None);
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
/// depth budget with every level re-declaring, and one start tag of attributes filling the bytes.
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
    for (name, document) in [
        ("ancestor", &ancestor),
        ("split", &split),
        ("declarations", &declarations),
        ("deep", &deep),
        ("wide", &wide),
    ] {
        assert!(
            document.len() <= MAX_POLICY_OCTETS,
            "{name} is past the bytes"
        );
        let started = std::time::Instant::now();
        let mut written = 0_usize;
        for _ in 0..100 {
            canonicalize(document.as_bytes(), &P, Comments::Omitted, &mut |piece| {
                written += piece.len();
            })
            .unwrap_or_else(|error| panic!("{name} is refused: {error:?}"));
        }
        println!(
            "{name}: {} octets in, {} out per run, {:?} for 100 runs",
            document.len(),
            written / 100,
            started.elapsed()
        );
    }
}
