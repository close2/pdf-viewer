//! ISO 19005 section 6.2.2 asks every content stream's resources to be explicit and to define each
//! name the stream uses; the verb answers both without changing a byte of a content stream
//! (`doc/adr/2500`).
//!
//! Each shape is converted to both targets and the output is held to the target again by
//! `pdf_archive`, independently of the verb. A shape the verb refuses is planted beside the one it
//! carries out, so that a refusal that quietly became a partial fix would fail here.
use super::{
    Authorisations, Because, Conforming, Decision, Flavour, Level, Limits, Rewrite, Target,
    Verdict, convert, decision, holds, stream,
};
use pdf_syntax::Document;
use std::fmt::Write as _;

use pdf_syntax::object::{Dictionary, Object};

const NAMED: &str = "graphics/named-resources-are-defined";
const EXPLICIT: &str = "graphics/content-streams-have-an-explicit-resources-dictionary";
const TARGETS: [Target; 2] = [Target::Two(Level::B), Target::Four(Flavour::Plain)];

/// The fixture for `target`, with the page's resources and content stream as written.
fn page_of(target: Target, resources: &str, content: &str) -> Conforming {
    let base = if matches!(target, Target::Two(_)) {
        Conforming::part_two()
    } else {
        Conforming::default()
    };
    Conforming {
        resources: resources.to_owned(),
        contents: Some((String::new(), content.as_bytes().to_vec())),
        ..base
    }
}

/// A content stream that selects `count` graphics states, each by a name of its own.
fn graphics_states_named(count: u32) -> String {
    (0..count).fold(String::new(), |mut content, index| {
        // Writing into a `String` cannot fail.
        let _ = writeln!(content, "/N{index} gs");
        content
    })
}

/// A form that marks a square and names `/G1` in the page's graphics states.
fn form_naming_g1() -> Vec<u8> {
    stream(
        "/Type /XObject /Subtype /Form /BBox [0 0 10 10]",
        b"/G1 gs 0 0 5 5 re f",
    )
}

/// The first page of a converted file.
fn first_page(output: &[u8]) -> (Document, Dictionary) {
    let document = Document::open_with_limits(output.to_vec(), Limits::DEFAULT).expect("output");
    let page = pdf_model::Pages::new(&document).get(0).expect("page");
    (document, page.dict)
}

/// The names a resource category of a dictionary holds.
fn names_in(document: &Document, resources: &Dictionary, category: &str) -> Vec<String> {
    document
        .get_key(resources, category)
        .as_dict()
        .map(|table| {
            table
                .iter()
                .map(|(name, _)| String::from_utf8_lossy(name.as_bytes()).into_owned())
                .collect()
        })
        .unwrap_or_default()
}

/// The decoded bytes of the stream a resource name stands for.
fn content_of(document: &Document, object: &Object) -> Vec<u8> {
    let object = document.resolve(object);
    let stream = object.as_stream().expect("a stream");
    document
        .decoded_stream_data(stream)
        .expect("decodes")
        .to_vec()
}

#[test]
fn a_form_stating_no_resources_is_given_the_page_s_and_its_bytes_are_untouched() {
    for target in TARGETS {
        let source = Conforming {
            objects: vec!["<< /Type /ExtGState /CA 1 >>".to_owned()],
            binary_objects: vec![form_naming_g1()],
            ..page_of(
                target,
                "/ExtGState << /G1 6 0 R >> /XObject << /F1 7 0 R >>",
                "/F1 Do",
            )
        }
        .build();
        let (report, output) = convert(&source, target, Authorisations::default());
        assert!(matches!(
            decision(&report, EXPLICIT),
            Decision::Mechanical(Rewrite::PageResources)
        ));
        let output = output.unwrap_or_else(|| panic!("{}", super::conversion(&report).render()));
        assert_eq!(holds(&output, target).verdict(), Verdict::Conforms);
        let (document, page) = first_page(&output);
        let resources = document.get_key(&page, "Resources");
        let resources = resources.as_dict().expect("the page's resources");
        let forms = document.get_key(resources, "XObject");
        let form = document.get_key(forms.as_dict().expect("table"), "F1");
        let form_dict = form.as_stream().expect("form").dict.clone();
        let own = document.get_key(&form_dict, "Resources");
        assert_eq!(
            names_in(
                &document,
                own.as_dict().expect("the form's resources"),
                "ExtGState"
            ),
            vec!["G1".to_owned()]
        );
        assert_eq!(content_of(&document, &form), b"/G1 gs 0 0 5 5 re f");
    }
}

#[test]
fn a_graphics_state_and_a_form_nothing_defines_are_defined_empty() {
    for target in TARGETS {
        let source = Conforming {
            objects: vec!["<< /Type /ExtGState /CA 1 >>".to_owned()],
            ..page_of(
                target,
                "/ExtGState << /G1 6 0 R >>",
                "/Nope gs /Gone Do 0 0 5 5 re f",
            )
        }
        .build();
        let (report, output) = convert(&source, target, Authorisations::default());
        assert!(matches!(
            decision(&report, NAMED),
            Decision::Mechanical(Rewrite::EmptyResourceDefined)
        ));
        let output = output.expect("both names are defined");
        assert_eq!(holds(&output, target).verdict(), Verdict::Conforms);
        let (document, page) = first_page(&output);
        let resources = document.get_key(&page, "Resources");
        let resources = resources.as_dict().expect("resources");
        assert_eq!(
            names_in(&document, resources, "ExtGState"),
            vec!["G1".to_owned(), "Nope".to_owned()]
        );
        let states = document.get_key(resources, "ExtGState");
        let empty = document.get_key(states.as_dict().expect("table"), "Nope");
        assert!(
            empty
                .as_dict()
                .expect("a dictionary")
                .iter()
                .next()
                .is_none()
        );
        let forms = document.get_key(resources, "XObject");
        let form = document.get_key(forms.as_dict().expect("table"), "Gone");
        let form = form.as_stream().expect("an empty form");
        assert_eq!(
            document
                .get_key(&form.dict, "Subtype")
                .as_name()
                .map(|n| n.as_bytes().to_vec()),
            Some(b"Form".to_vec())
        );
        let area = document.get_key(&form.dict, "BBox");
        assert!(
            area.as_array()
                .expect("box")
                .iter()
                .all(|n| n.as_number() == Some(0.0)),
            "a form with no area marks nothing"
        );
        assert!(
            document
                .decoded_stream_data(form)
                .expect("decodes")
                .is_empty()
        );
    }
}

/// ISO 32000-2 §7.3.9: a reference to a nonexistent object "shall be treated the same as a null
/// object", and a null entry "shall be equivalent to omitting the entry entirely". So a name whose
/// entry is either defines nothing, the page draws nothing for it, and the name is defined empty
/// in place of the entry — `veraPDFHiResWrongObjectID`'s `/Im0 99 0 R` is the first shape
/// (`doc/adr/2550`).
#[test]
fn a_name_whose_entry_is_null_or_names_no_object_is_defined_empty_in_its_place() {
    for target in TARGETS {
        let source = page_of(
            target,
            "/XObject << /Gone 99 0 R /Null null >>",
            "/Gone Do /Null Do 0 0 5 5 re f",
        )
        .build();
        let (report, output) = convert(&source, target, Authorisations::default());
        assert!(
            matches!(
                decision(&report, NAMED),
                Decision::Mechanical(Rewrite::EmptyResourceDefined)
            ),
            "the source defines neither name"
        );
        let output = output.expect("both names are defined");
        assert_eq!(holds(&output, target).verdict(), Verdict::Conforms);
        let (document, page) = first_page(&output);
        let resources = document.get_key(&page, "Resources");
        let resources = resources.as_dict().expect("resources");
        let forms = document.get_key(resources, "XObject");
        let forms = forms.as_dict().expect("table");
        for name in ["Gone", "Null"] {
            let form = document.get_key(forms, name);
            let form = form
                .as_stream()
                .expect("an empty form in the entry's place");
            assert!(
                document
                    .decoded_stream_data(form)
                    .expect("decodes")
                    .is_empty(),
                "{name} draws nothing, as the source's entry drew nothing"
            );
        }
    }
}

#[test]
fn a_font_nothing_defines_is_an_empty_type_3_font_where_only_glyphs_were_shown() {
    // Each content stream shows text under a name no dictionary defines and moves the position by
    // nothing but the widths: the reader drew nothing and moved nothing, and so does the font.
    for content in [
        "BT /Nope 12 Tf (abc) Tj ET",
        "BT /Nope 12 Tf 0 Tc 0 Tw [(a) 0 (b)] TJ ET",
        "BT /Nope 12 Tf 3 Tc 0 Tc (abc) Tj ET",
        "BT q 3 Tc Q /Nope 12 Tf (abc) Tj ET",
    ] {
        for target in TARGETS {
            let source = page_of(target, "", content).build();
            let (report, output) = convert(&source, target, Authorisations::default());
            assert!(
                matches!(
                    decision(&report, NAMED),
                    Decision::Mechanical(Rewrite::EmptyResourceDefined)
                ),
                "{content}"
            );
            let output = output.expect("the font is defined");
            assert_eq!(
                holds(&output, target).verdict(),
                Verdict::Conforms,
                "{content}"
            );
            let (document, page) = first_page(&output);
            let resources = document.get_key(&page, "Resources");
            let fonts = document.get_key(resources.as_dict().expect("resources"), "Font");
            let font = document.get_key(fonts.as_dict().expect("table"), "Nope");
            let font = font.as_dict().expect("a font dictionary");
            assert_eq!(
                document
                    .get_key(font, "Subtype")
                    .as_name()
                    .map(|n| n.as_bytes().to_vec()),
                Some(b"Type3".to_vec())
            );
            let widths = document.get_key(font, "Widths");
            let widths = widths.as_array().expect("widths");
            assert_eq!(widths.len(), 256);
            assert!(widths.iter().all(|width| width.as_number() == Some(0.0)));
        }
    }
}

#[test]
fn a_font_nothing_defines_stays_refused_where_text_moved_more_than_its_widths() {
    // A plant for the survey's record of what a show did: each of these would move the text
    // position, or the clip, by something an empty font would do and the reader did not.
    for content in [
        "BT /Nope 12 Tf 2 Tc (abc) Tj ET",
        "BT /Nope 12 Tf 5 Tw (abc) Tj ET",
        "BT /Nope 12 Tf [(a) 120 (b)] TJ ET",
        "BT /Nope 12 Tf 14 TL (abc) ' ET",
        "BT /Nope 12 Tf 1 2 (abc) \" ET",
        "BT /Nope 12 Tf 7 Tr (abc) Tj ET",
    ] {
        let target = Target::Two(Level::B);
        let source = page_of(target, "", content).build();
        let (report, output) = convert(&source, target, Authorisations::default());
        assert!(
            matches!(
                decision(&report, NAMED),
                Decision::Refused(Because::NotBuiltYet(_))
            ),
            "{content}"
        );
        assert!(output.is_none(), "{content}");
    }
}

#[test]
fn a_name_in_a_category_with_no_empty_value_stays_refused() {
    for content in [
        "/Nope cs 0 0 5 5 re f",
        "/Nope sh",
        "/Pattern cs /Nope scn 0 0 5 5 re f",
    ] {
        let target = Target::Two(Level::B);
        let source = page_of(target, "", content).build();
        let (report, output) = convert(&source, target, Authorisations::default());
        assert!(
            matches!(
                decision(&report, NAMED),
                Decision::Refused(Because::NotBuiltYet(_))
            ),
            "{content}"
        );
        assert!(output.is_none(), "{content}");
    }
}

#[test]
fn a_walk_stopped_at_its_bound_defines_nothing_rather_than_some() {
    // More distinct missing names than the survey records (its bound is 100 000 observations):
    // the walk stops short, so the names it did record are not all of them and none is defined.
    let content = graphics_states_named(100_100);
    let target = Target::Two(Level::B);
    let source = page_of(target, "", &content).build();
    let (report, output) = convert(&source, target, Authorisations::default());
    assert!(matches!(
        decision(&report, NAMED),
        Decision::Refused(Because::NotBuiltYet(_))
    ));
    assert!(output.is_none());
}

#[test]
fn a_stream_naming_tens_of_thousands_of_missing_resources_is_defined_whole() {
    // Under the survey's bound, so every name is recorded and every one is defined, and the
    // output is read again by the validator: the plan, the rewrite and that second walk each
    // handle a table of tens of thousands of names without copying it once per name.
    let content = graphics_states_named(50_000);
    let target = Target::Two(Level::B);
    let source = page_of(target, "", &content).build();
    let (_, output) = convert(&source, target, Authorisations::default());
    let output = output.expect("every name is defined");
    assert_eq!(holds(&output, target).verdict(), Verdict::Conforms);
    let (document, page) = first_page(&output);
    let resources = document.get_key(&page, "Resources");
    assert_eq!(
        names_in(
            &document,
            resources.as_dict().expect("resources"),
            "ExtGState"
        )
        .len(),
        50_000
    );
}

#[test]
fn a_standard_font_name_nothing_defines_is_the_font_the_reader_drew_not_an_empty_one() {
    // The reader draws the fourteen names of §9.6.2.2 from its own faces without a resource,
    // so the text is on the page; an empty Type 3 font would erase it.
    for (name, target) in [
        ("Helvetica", Target::Two(Level::B)),
        ("Times-Roman", Target::Two(Level::B)),
        ("Helvetica", Target::Four(Flavour::Plain)),
        ("Times-Roman", Target::Four(Flavour::Plain)),
    ] {
        let source = page_of(
            target,
            "",
            &format!("BT /{name} 24 Tf 10 10 Td (Hello) Tj ET"),
        )
        .build();
        let (report, output) = convert(&source, target, Authorisations::default());
        assert!(
            matches!(
                decision(&report, NAMED),
                Decision::Mechanical(Rewrite::EmptyResourceDefined)
            ),
            "{name} {target}"
        );
        // The definition is embedded through the standard fourteen's own route, so the file
        // stands and the font carries the shipped face (`doc/adr/2522`).
        let output =
            output.unwrap_or_else(|| panic!("{name}: {}", super::conversion(&report).render()));
        assert_eq!(
            holds(&output, target).verdict(),
            Verdict::Conforms,
            "{name}"
        );
        let (document, page) = first_page(&output);
        let resources = document.get_key(&page, "Resources");
        let fonts = document.get_key(resources.as_dict().expect("resources"), "Font");
        let font = document.get_key(fonts.as_dict().expect("table"), name);
        let font = font.as_dict().expect("a font dictionary");
        assert_eq!(
            document
                .get_key(font, "Subtype")
                .as_name()
                .map(|n| n.as_bytes().to_vec()),
            // Liberation Sans, the Helvetica face, is a `glyf` program that goes under a
            // TrueType dictionary (§9.9, Table 124); the others are bare CFF under Type1.
            Some(if name == "Helvetica" {
                b"TrueType".to_vec()
            } else {
                b"Type1".to_vec()
            }),
            "{name}"
        );
        let descriptor = document.get_key(font, "FontDescriptor");
        let descriptor = descriptor.as_dict().expect("a descriptor");
        assert!(
            ["FontFile", "FontFile2", "FontFile3"]
                .iter()
                .any(|key| !document.get_key(descriptor, key).is_null()),
            "{name}: a program is embedded"
        );
    }
}

#[test]
fn a_standard_font_defined_for_a_name_is_one_object_whatever_number_of_streams_name_it() {
    // Sixty forms and the page all show `/Helvetica`, which no resource dictionary defines: each
    // definition names the one font object, and its face is embedded once.
    let target = Target::Two(Level::B);
    let forms = 60_usize;
    let mut xobjects = String::new();
    let mut invoked = String::from("BT /Helvetica 12 Tf 10 10 Td (Hello) Tj ET\n");
    let binary_objects: Vec<Vec<u8>> = (0..forms)
        .map(|index| {
            let _ = write!(xobjects, "/F{index} {} 0 R ", 6 + index);
            let _ = writeln!(invoked, "/F{index} Do");
            stream(
                "/Type /XObject /Subtype /Form /BBox [0 0 100 100] /Resources << >>",
                b"BT /Helvetica 12 Tf 10 20 Td (World) Tj ET",
            )
        })
        .collect();
    let source = Conforming {
        binary_objects,
        ..page_of(target, &format!("/XObject << {xobjects} >>"), &invoked)
    }
    .build();
    let (report, output) = convert(&source, target, Authorisations::default());
    let output = output.unwrap_or_else(|| panic!("{}", super::conversion(&report).render()));
    assert_eq!(holds(&output, target).verdict(), Verdict::Conforms);
    let (document, page) = first_page(&output);
    let mut fonts = std::collections::BTreeSet::new();
    let page_fonts = document.get_key(
        document
            .get_key(&page, "Resources")
            .as_dict()
            .expect("resources"),
        "Font",
    );
    fonts.insert(
        page_fonts
            .as_dict()
            .and_then(|table| table.get("Helvetica"))
            .and_then(Object::as_reference)
            .expect("the page's font is an object"),
    );
    let xobjects = document.get_key(
        document
            .get_key(&page, "Resources")
            .as_dict()
            .expect("resources"),
        "XObject",
    );
    for index in 0..forms {
        let form = document.get_key(xobjects.as_dict().expect("table"), &format!("F{index}"));
        let own = document.get_key(&form.as_stream().expect("form").dict, "Resources");
        let table = document.get_key(own.as_dict().expect("resources"), "Font");
        fonts.insert(
            table
                .as_dict()
                .and_then(|table| table.get("Helvetica"))
                .and_then(Object::as_reference)
                .expect("the form's font is an object"),
        );
    }
    assert_eq!(fonts.len(), 1, "one font object serves every definition");
    assert!(
        output.len() < 400_000,
        "{} bytes: the face was embedded more than once",
        output.len()
    );
}

#[test]
fn a_spacing_set_by_the_quote_operator_refuses_a_font_nothing_defines_for_a_later_show() {
    // `aw ac string "` sets Tw and Tc (§9.4.3, Table 107), so the show after it under an empty
    // font would have moved by the spacing the reader never applied.
    let target = Target::Two(Level::B);
    let source = Conforming {
        objects: vec!["<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_owned()],
        ..page_of(
            target,
            "/Font << /F1 6 0 R >>",
            "BT /F1 24 Tf 10 50 Td 0 30 (a) \" /Nope 24 Tf (aaa) Tj /F1 24 Tf (a) Tj ET",
        )
    }
    .build();
    let (report, output) = convert(&source, target, Authorisations::default());
    assert!(matches!(
        decision(&report, NAMED),
        Decision::Refused(Because::NotBuiltYet(_))
    ));
    assert!(output.is_none());
}

#[test]
fn forms_stating_no_resources_under_a_direct_page_dictionary_share_one_object() {
    // The page's dictionary is written directly and large; each of the forms is given a
    // reference to one object holding it, and a name only the forms used is defined in it.
    let target = Target::Two(Level::B);
    let forms = 60_usize;
    let mut states = String::new();
    let mut objects = Vec::new();
    for index in 0..2_000_usize {
        let _ = write!(states, "/G{index} 6 0 R ");
        if index == 0 {
            objects.push("<< /Type /ExtGState /CA 1 >>".to_owned());
        }
    }
    let mut xobjects = String::new();
    let mut invoked = String::new();
    let binary_objects: Vec<Vec<u8>> = (0..forms)
        .map(|index| {
            let _ = write!(xobjects, "/F{index} {} 0 R ", 7 + index);
            let _ = writeln!(invoked, "/F{index} Do");
            stream(
                "/Type /XObject /Subtype /Form /BBox [0 0 10 10]",
                b"/G0 gs /Only gs 0 0 5 5 re f",
            )
        })
        .collect();
    let source = Conforming {
        objects,
        binary_objects,
        ..page_of(
            target,
            &format!("/ExtGState << {states} >> /XObject << {xobjects} >>"),
            &invoked,
        )
    }
    .build();
    let (_, output) = convert(&source, target, Authorisations::default());
    let output = output.expect("the forms are given the page's resources");
    assert_eq!(holds(&output, target).verdict(), Verdict::Conforms);
    let (document, page) = first_page(&output);
    let resources = document.get_key(&page, "Resources");
    let forms_table = document.get_key(resources.as_dict().expect("resources"), "XObject");
    let mut shared = std::collections::BTreeSet::new();
    for index in 0..forms {
        let form = document.get_key(forms_table.as_dict().expect("table"), &format!("F{index}"));
        let reference = form
            .as_stream()
            .expect("a form")
            .dict
            .get("Resources")
            .and_then(Object::as_reference)
            .expect("the form's resources are an object");
        shared.insert(reference);
    }
    assert_eq!(shared.len(), 1, "one object serves every form");
    let held = document.get(*shared.iter().next().expect("one"));
    let held = held.as_dict().expect("a dictionary");
    assert!(names_in(&document, held, "ExtGState").contains(&"Only".to_owned()));
    assert!(
        output.len() < forms * 2_000,
        "{} bytes for {forms} forms: the dictionary was copied per form",
        output.len()
    );
}

/// A document of `copies + 1` pages that all draw `/F1`, one form stating no `/Resources`.
///
/// The first page states its resources directly; each of the others names an object of its own,
/// whose graphics state `/G1` is an object of its own too, so that the pages resolve the form's
/// one name to different things (ISO 32000-2 §7.8.3).
#[expect(
    clippy::arithmetic_side_effects,
    reason = "object numbers of a generated fixture, at most 3 * 257 + 10"
)]
fn pages_sharing_a_form(target: Target, copies: usize, form: Vec<u8>) -> Vec<u8> {
    let form_number = 7 + 3 * copies;
    let mut objects = vec!["<< /Type /ExtGState /LW 1 >>".to_owned()];
    let mut more_pages = Vec::new();
    for index in 0..copies {
        let first = 7 + 3 * index;
        objects.push(format!("<< /Type /ExtGState /LW {} >>", index + 2));
        objects.push(format!(
            "<< /ExtGState << /G1 {first} 0 R >> /XObject << /F1 {form_number} 0 R >> >>"
        ));
        objects.push(format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Resources {} 0 R \
             /Contents 4 0 R >>",
            first + 1
        ));
        more_pages.push(first + 2);
    }
    Conforming {
        objects,
        more_pages,
        binary_objects: vec![form],
        ..page_of(
            target,
            &format!("/ExtGState << /G1 6 0 R >> /XObject << /F1 {form_number} 0 R >>"),
            "/F1 Do",
        )
    }
    .build()
}

/// The form page `index` draws as `/F1`, and the graphics state it resolves `/G1` to through the
/// form's own resources and through the page's.
fn form_and_states(document: &Document, index: usize) -> (Object, Object, Object) {
    let page = pdf_model::Pages::new(document)
        .get(index)
        .expect("page")
        .dict;
    let resources = document.get_key(&page, "Resources");
    let resources = resources.as_dict().expect("the page's resources");
    let table = document.get_key(resources, "XObject");
    let form = document.get_key(table.as_dict().expect("table"), "F1");
    let own = document.get_key(&form.as_stream().expect("form").dict, "Resources");
    let through_form = document.get_key(
        document
            .get_key(own.as_dict().expect("the form's resources"), "ExtGState")
            .as_dict()
            .expect("states"),
        "G1",
    );
    let through_page = document.get_key(
        document
            .get_key(resources, "ExtGState")
            .as_dict()
            .expect("states"),
        "G1",
    );
    (form, through_form, through_page)
}

#[test]
fn a_form_reached_from_pages_with_different_resources_is_copied_for_each_dictionary() {
    for target in TARGETS {
        let source = pages_sharing_a_form(target, 2, form_naming_g1());
        let (report, output) = convert(&source, target, Authorisations::default());
        assert!(matches!(
            decision(&report, EXPLICIT),
            Decision::Mechanical(Rewrite::PageResources)
        ));
        let output = output.unwrap_or_else(|| panic!("{}", super::conversion(&report).render()));
        assert_eq!(holds(&output, target).verdict(), Verdict::Conforms);
        let document = Document::open_with_limits(output.clone(), Limits::DEFAULT).expect("output");
        let mut seen = Vec::new();
        for index in 0..3 {
            let (form, through_form, through_page) = form_and_states(&document, index);
            assert_eq!(
                through_form, through_page,
                "page {index}: the form resolves /G1 to what the page's own resources say"
            );
            assert_eq!(content_of(&document, &form), b"/G1 gs 0 0 5 5 re f");
            seen.push(through_page);
        }
        assert!(
            seen[0] != seen[1] && seen[1] != seen[2] && seen[0] != seen[2],
            "the three pages resolve /G1 to three different states"
        );
    }
}

#[test]
fn a_form_also_named_by_a_form_with_resources_of_its_own_is_not_copied() {
    // `/F2` states resources of its own and draws `/F1` through them: the original form is named
    // by a table that is not the page's, so a copy for the page's table would leave that route
    // reading the wrong resources, and the conversion refuses instead.
    let target = Target::Two(Level::B);
    let outer = stream(
        "/Type /XObject /Subtype /Form /BBox [0 0 10 10] /Resources << /XObject << /F1 10 0 R >> >>",
        b"/F1 Do",
    );
    let source = Conforming {
        objects: vec![
            "<< /Type /ExtGState /LW 1 >>".to_owned(),
            "<< /Type /ExtGState /LW 2 >>".to_owned(),
            "<< /ExtGState << /G1 7 0 R >> /XObject << /F1 10 0 R /F2 11 0 R >> >>".to_owned(),
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Resources 8 0 R \
             /Contents 4 0 R >>"
                .to_owned(),
        ],
        more_pages: vec![9],
        binary_objects: vec![form_naming_g1(), outer],
        ..page_of(
            target,
            "/ExtGState << /G1 6 0 R >> /XObject << /F1 10 0 R /F2 11 0 R >>",
            "/F1 Do /F2 Do",
        )
    }
    .build();
    let (report, output) = convert(&source, target, Authorisations::default());
    assert!(matches!(
        decision(&report, EXPLICIT),
        Decision::Refused(Because::NotBuiltYet(text)) if text.contains("differ")
    ));
    assert!(output.is_none());
}

#[test]
fn copies_of_a_form_stop_at_their_count_and_at_their_bytes() {
    let target = Target::Two(Level::B);
    // 256 is `MAX_FORM_COPIES`: the copies for 256 pages are written, one more is refused.
    let at = pages_sharing_a_form(target, 256, form_naming_g1());
    let (report, output) = convert(&at, target, Authorisations::default());
    assert!(output.is_some(), "{}", super::conversion(&report).render());
    let over = pages_sharing_a_form(target, 257, form_naming_g1());
    let (report, output) = convert(&over, target, Authorisations::default());
    assert!(matches!(
        decision(&report, EXPLICIT),
        Decision::Refused(Because::NotBuiltYet(text)) if text.contains("more copies")
    ));
    assert!(output.is_none());
    // 32 MiB is `MAX_COPIED_BYTES`: a form of one MiB is copied for 32 pages and not 33.
    let big = |copies: usize| {
        let mut data = b"/G1 gs 0 0 5 5 re f\n".repeat((1 << 20) / 20);
        data.resize(1 << 20, b' ');
        pages_sharing_a_form(
            target,
            copies,
            stream("/Type /XObject /Subtype /Form /BBox [0 0 10 10]", &data),
        )
    };
    let (report, output) = convert(&big(32), target, Authorisations::default());
    assert!(output.is_some(), "{}", super::conversion(&report).render());
    let (report, output) = convert(&big(33), target, Authorisations::default());
    assert!(matches!(
        decision(&report, EXPLICIT),
        Decision::Refused(Because::NotBuiltYet(text)) if text.contains("more copies")
    ));
    assert!(output.is_none());
}
