//! A repair this reader made inside a content stream, and a page box it could not read, answered
//! under ISO 19005-2 and ISO 19005-4 section 5.1 (`doc/adr/3557`, `doc/adr/3558`,
//! `doc/adr/3559`).
//!
//! The expected values come from ISO 32000-2: §8.4.2 ("Occurrences of the q and Q operators shall
//! be balanced"), so a `Q` with nothing saved restores nothing and the page draws the same without
//! it; §9.4.1, whose text-showing operators belong inside `BT … ET`, so one outside that this reader
//! still draws is enclosed in a text object that starts with the matrices it was drawn by
//! (`doc/adr/3661`); and §7.7.3.3's Table 31, which requires `/MediaBox`
//! and gives it no default, so the box a page with none is drawn on is this reader's
//! (`Page::DEFAULT_MEDIA_BOX`, ADR 0389) and is what the output states.
use super::*;
use pdf_archive::Flavour;
use pdf_transform::archive::Loss;

/// ISO 19005-2 and ISO 19005-4 section 5.1's row.
const BASE_STANDARD: &str = "conformance/adheres-to-the-base-standard";

/// A simple font with no program and no descriptor, which the conversion embeds a shipped face
/// for (ISO 19005-2 section 6.2.11.4.1).
const HELVETICA: &str =
    "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>";

/// A page whose one content stream is `data`, with Helvetica as `/F1`.
fn a_page_whose_stream_is(data: &[u8]) -> Vec<u8> {
    Conforming {
        resources: "/Font << /F1 6 0 R >>".to_owned(),
        contents: Some((String::new(), data.to_vec())),
        objects: vec![HELVETICA.to_owned()],
        ..Conforming::part_two()
    }
    .build()
}

/// The plan a shipped profile makes for `target`, with the authorisations its `discard` steps
/// stand for.
fn under(profile: &str, target: Target) -> ArchivePlan {
    let text = pdf_transform::profiles::named(profile)
        .expect("a shipped profile")
        .text;
    let config = pdf_transform::archive::Configuration::read(text, target).expect("it reads");
    ArchivePlan {
        authorised: config.authorisations(target),
        ..plan_from(text, target)
    }
}

/// The sentences of every refusal the conversion reported.
fn refusals(report: &Report) -> Vec<String> {
    conversion(report)
        .decided
        .iter()
        .filter_map(|decided| match decided.decision {
            Decision::Refused(because) => Some(because.sentence().to_string()),
            _ => None,
        })
        .collect()
}

#[test]
fn a_restore_with_nothing_saved_is_written_out_where_the_page_draws_the_same() {
    let source = a_page_whose_stream_is(b"BT /F1 12 Tf 10 100 Td (Hello) Tj ET Q");
    for profile in ["as-shown", "as-if-printed"] {
        for target in [Target::Two(Level::B), Target::Four(Flavour::Plain)] {
            let (report, output) = convert_with_plan(&source, &under(profile, target));
            let Decision::Authorised { losses, .. } = decision(&report, BASE_STANDARD) else {
                panic!("{profile}: {:?}", decision(&report, BASE_STANDARD));
            };
            assert!(
                losses.iter().eq([Loss::ContentAsDrawn]),
                "{profile}: {}",
                losses.words()
            );
            let output = output.expect("a file is written");
            assert_eq!(holds(&output, target).verdict(), Verdict::Conforms);
            let conversion = conversion(&report);
            assert!(
                conversion.photographed.is_empty(),
                "{profile}: the elegant remedy comes before the photograph"
            );
            let [written] = conversion.runs_written_as_drawn.as_slice() else {
                panic!("one operator: {:?}", conversion.runs_written_as_drawn);
            };
            assert_eq!(written.run, b"Q");
            assert_eq!(written.number, [] as [u8; 0]);
            let contents = decoded_contents(&output);
            let text = String::from_utf8_lossy(&contents);
            assert!(text.contains("(Hello) Tj ET"), "{text}");
            assert!(!text.contains('Q'), "the lone Q is gone: {text}");
        }
    }
}

/// The first page's content stream, decoded, as text with each run of white space one space, so
/// that what is asserted is the tokens and their order.
fn contents_text(output: &[u8]) -> String {
    String::from_utf8_lossy(&decoded_contents(output))
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Converts `source` under `as-shown` to `target` and returns the output and the losses section
/// 5.1's row was answered with.
fn as_shown(source: &[u8], target: Target) -> (Vec<u8>, Losses) {
    let (report, output) = convert_with_plan(source, &under("as-shown", target));
    let Decision::Authorised { losses, .. } = decision(&report, BASE_STANDARD) else {
        panic!("{:?}", decision(&report, BASE_STANDARD));
    };
    assert!(
        conversion(&report).photographed.is_empty(),
        "the elegant remedy comes before the photograph"
    );
    let output = output.expect("as-shown writes a file");
    assert_eq!(holds(&output, target).verdict(), Verdict::Conforms);
    (output, losses)
}

#[test]
fn text_operators_drawn_outside_a_text_object_are_enclosed_in_one_where_the_page_draws_the_same() {
    // `Td` and `Tj` with no `BT` before them: this reader draws them by the identity matrices a
    // `BT` sets (ADR 3353), so a `BT` in front of them and an `ET` after draw the same page
    // (ADR 3661). Nothing of the producer's is removed, and the loss is `content-as-drawn`, the
    // reading being this reader's.
    let source = a_page_whose_stream_is(b"/F1 12 Tf 10 100 Td (Hello) Tj");
    for target in [Target::Two(Level::B), Target::Four(Flavour::Plain)] {
        let (output, losses) = as_shown(&source, target);
        assert!(
            losses.iter().eq([Loss::ContentAsDrawn]),
            "{}",
            losses.words()
        );
        let text = contents_text(&output);
        assert!(
            text.contains("/F1 12 Tf BT 10 100 Td (Hello) Tj ET"),
            "{text}"
        );
    }
}

#[test]
fn a_text_object_added_ends_before_an_operator_a_text_object_may_not_hold_and_another_begins() {
    // pdf.js's `issue10388_reduced` in small: two runs of text operators outside any `BT … ET`
    // with a saved state between them. §9.4.1 admits no `q` inside a text object, so each run
    // gets its own, and each starts with its own `Tm`, which sets both matrices.
    let source = a_page_whose_stream_is(
        b"/F1 1 Tf 9 0 0 9 36 48 Tm [(A)] TJ q 1 0 0 1 46 46 cm Q 9 0 0 9 51 48 Tm [(B)] TJ",
    );
    let (output, _) = as_shown(&source, Target::Two(Level::B));
    let text = contents_text(&output);
    assert!(
        text.contains("BT 9 0 0 9 36 48 Tm [(A)] TJ ET q 1 0 0 1 46 46 cm Q BT 9 0 0 9 51 48 Tm"),
        "{text}"
    );
    assert_eq!(text.matches("BT").count(), 2, "{text}");
    assert_eq!(text.matches("ET").count(), 2, "{text}");
}

#[test]
fn a_text_object_added_after_a_matrix_was_stated_states_it_again() {
    // The second `Td` reads the line matrix the `Tm` set before the `q`, so the text object added
    // around it opens with that `Tm`, the numbers respelled as §7.3.3 spells them.
    let source =
        a_page_whose_stream_is(b"/F1 12 Tf 1 0 0 1 10 100.0 Tm (A) Tj q Q 0 -20 Td (B) Tj");
    let (output, _) = as_shown(&source, Target::Two(Level::B));
    let text = contents_text(&output);
    assert!(
        text.contains("Q BT 1 0 0 1 10 100 Tm 0 -20 Td (B) Tj ET"),
        "{text}"
    );
}

#[test]
fn a_producers_stray_et_closes_the_text_object_added() {
    // `T02-05-01_001`'s shape: the `BT` is missing and the `ET` is there. This reader ignores
    // the stray `ET`; once the `BT` is added it ends the object, and no second `ET` is written.
    let source = a_page_whose_stream_is(b"/F1 12 Tf (Hello) Tj ET");
    let (output, _) = as_shown(&source, Target::Two(Level::B));
    let text = contents_text(&output);
    assert!(text.contains("BT (Hello) Tj ET"), "{text}");
    assert_eq!(text.matches("ET").count(), 1, "{text}");
}

#[test]
fn a_text_operator_whose_matrix_the_walk_cannot_state_keeps_its_refusal_or_is_photographed() {
    // After `(A) Tj` the text matrix stands past the line matrix by the glyph's width, which the
    // walk does not measure; the `Tj` after the `q … Q` is drawn there, so no text object can be
    // added that starts where it was drawn, and `as-shown` refuses by the class's name.
    let source = a_page_whose_stream_is(b"/F1 12 Tf 10 100 Td (A) Tj q Q (B) Tj");
    let target = Target::Four(Flavour::Plain);
    let (report, output) = convert_with_plan(&source, &under("as-shown", target));
    assert!(output.is_none(), "as-shown writes no file");
    let refused = refusals(&report);
    assert!(
        refused.iter().any(|sentence| sentence
            .contains("model/text-operator-outside-a-text-object")
            && sentence.contains("ADR 3661")),
        "refused by the class's name: {refused:?}"
    );
    let (report, output) = convert_with_plan(&source, &under("as-if-printed", target));
    let output = output.expect("as-if-printed photographs the page");
    assert_eq!(holds(&output, target).verdict(), Verdict::Conforms);
    let Decision::Authorised { losses, .. } = decision(&report, BASE_STANDARD) else {
        panic!("{:?}", decision(&report, BASE_STANDARD));
    };
    assert!(losses.contains(Loss::PageAsImage), "{}", losses.words());
    assert_eq!(conversion(&report).photographed.len(), 1);
}

#[test]
fn a_file_whose_cross_reference_table_was_rebuilt_is_proved_drawn_the_same_by_a_whole_file() {
    // §7.5.6's update chains to the section before it, and a table rebuilt by scanning has none,
    // so the proof's second file is written whole from the objects this reader read (ADR 3662).
    // `startxref` naming an offset that holds no table is the defect.
    let mut source = a_page_whose_stream_is(b"/F1 12 Tf 10 100 Td (Hello) Tj");
    let at = source
        .windows(b"startxref".len())
        .rposition(|window| window == b"startxref")
        .expect("the fixture ends with startxref");
    source.truncate(at);
    source.extend_from_slice(b"startxref\n1\n%%EOF\n");
    assert!(
        Document::open_with_limits(source.clone(), Limits::DEFAULT)
            .expect("it opens")
            .was_recovered(),
        "the fixture's table is rebuilt"
    );
    let (output, _) = as_shown(&source, Target::Two(Level::B));
    assert!(contents_text(&output).contains("BT 10 100 Td (Hello) Tj ET"));
}

#[test]
fn a_media_box_that_is_no_rectangle_is_written_as_the_box_this_reader_drew() {
    let built = a_page_whose_stream_is(b"BT /F1 12 Tf 10 100 Td (Hello) Tj ET");
    // Three numbers in place of four, the same length so every offset stands.
    let (was, now) = (
        &b"/MediaBox [0 0 200 200]"[..],
        &b"/MediaBox [0 0 200]    "[..],
    );
    let at = built
        .windows(was.len())
        .position(|window| window == was)
        .expect("the fixture states its media box");
    let mut source = built.clone();
    source[at..at + was.len()].copy_from_slice(now);
    for target in [Target::Two(Level::B), Target::Four(Flavour::Plain)] {
        let (report, output) = convert_with_plan(&source, &under("as-shown", target));
        let output = output.unwrap_or_else(|| panic!("{:?}", refusals(&report)));
        assert_eq!(holds(&output, target).verdict(), Verdict::Conforms);
        let document = Document::open_with_limits(output, Limits::DEFAULT).expect("it opens");
        let page = pdf_model::Pages::new(&document)
            .get(0)
            .expect("the page is there");
        assert_eq!(page.media_box, pdf_model::Page::DEFAULT_MEDIA_BOX);
        assert!(
            document
                .repairs()
                .iter()
                .all(|logged| logged.class.name != pdf_model::repair::MEDIA_BOX_SUBSTITUTED.name),
            "the output states the box, so reading it substitutes none"
        );
    }
}

/// ISO 32000-2 Table 352's `BMC` begins a sequence "terminated by a balancing EMC operator", and
/// §7.8.2's `BX` and `EX` "shall occur in pairs": a stream ending inside a section, or closing one
/// it never opened, is read as if balanced, and the checker's grammar finds it before the fence
/// asks which pages to photograph (`doc/adr/3614`). Under `as-shown` the row keeps its refusal by
/// the class's name; under `as-if-printed` the page is photographed.
#[test]
fn a_section_left_open_or_closed_twice_is_found_before_the_photographs() {
    let target = Target::Two(Level::B);
    for (stream, class) in [
        (
            &b"/Tag BMC 0 0 10 10 re f"[..],
            "model/marked-content-unbalanced",
        ),
        (
            &b"0 0 10 10 re f EX"[..],
            "model/compatibility-sections-unbalanced",
        ),
    ] {
        let source = a_page_whose_stream_is(stream);
        let (report, output) = convert_with_plan(&source, &under("as-shown", target));
        assert!(output.is_none(), "{class}: as-shown writes no file");
        let refused = refusals(&report);
        assert!(
            refused.iter().any(|sentence| sentence.contains(class)),
            "refused by the class's name: {refused:?}"
        );
        let (report, output) = convert_with_plan(&source, &under("as-if-printed", target));
        let output = output.expect("as-if-printed photographs the page");
        assert_eq!(
            holds(&output, target).verdict(),
            Verdict::Conforms,
            "{class}"
        );
        assert_eq!(conversion(&report).photographed.len(), 1, "{class}");
    }
}

/// A `BDC` whose tag is not a name (Table 352: "tag shall be a name object") and whose property
/// list names nothing the resources hold (§14.6.2) draws nothing of its own, so it is written out
/// where the page draws the same — and the `EMC` balancing it with it, or the `EMC` left would
/// balance nothing (`doc/adr/3615`).
#[test]
fn a_section_opened_by_an_operator_written_out_is_written_out_whole() {
    let source = a_page_whose_stream_is(b"123 /oc1 BDC 0 0 10 10 re f EMC");
    let target = Target::Two(Level::B);
    let (report, output) = convert_with_plan(&source, &under("as-shown", target));
    let Decision::Authorised { losses, .. } = decision(&report, BASE_STANDARD) else {
        panic!("{:?}", decision(&report, BASE_STANDARD));
    };
    assert!(
        losses.iter().eq([Loss::ContentAsDrawn]),
        "{}",
        losses.words()
    );
    let output = output.expect("a file is written");
    assert_eq!(holds(&output, target).verdict(), Verdict::Conforms);
    let written: Vec<&[u8]> = conversion(&report)
        .runs_written_as_drawn
        .iter()
        .map(|written| written.run.as_slice())
        .collect();
    assert_eq!(written, [&b"123 /oc1 BDC"[..], b"EMC"]);
    let contents = decoded_contents(&output);
    let text = String::from_utf8_lossy(&contents);
    assert!(text.contains("0 0 10 10 re f"), "{text}");
    assert!(!text.contains("BDC") && !text.contains("EMC"), "{text}");
}

/// The checker's own walk, which never interprets, logs each class the tokens decide before
/// anything draws a page (`doc/adr/3629`): ISO 32000-2 §7.8.2's "operands shall not be left over
/// when an operator finishes execution", §14.6.2's property list that "shall be defined as a named
/// resource in the Properties subdictionary", Table 352's balancing `EMC`, and §8.9.7's inline
/// image, whose `/ColorSpace` names a space the resources must define.
#[test]
fn the_checkers_walk_logs_each_class_the_tokens_decide_before_anything_draws() {
    let target = Target::Two(Level::B);
    let logged_by_the_walk = |stream: &[u8]| {
        let document = Document::open_with_limits(a_page_whose_stream_is(stream), Limits::DEFAULT)
            .expect("the fixture opens");
        let _ = pdf_archive::check(&document, target);
        document
            .repairs()
            .iter()
            .filter(|logged| matches!(logged.at, pdf_syntax::Site::Content(_)))
            .map(|logged| logged.class.name)
            .collect::<BTreeSet<_>>()
    };
    for (stream, class) in [
        (
            &b"BT /F1 12 Tf 5 10 100 Td (a) Tj ET"[..],
            "model/operands-left-over",
        ),
        (
            &b"/OC /absent BDC 0 0 10 10 re f EMC"[..],
            "model/property-list-unresolved",
        ),
        (
            &b"/Tx BMC 0 0 10 10 re f EMC EMC"[..],
            "model/marked-content-unbalanced",
        ),
        (
            &b"q 10 0 0 10 0 0 cm BI /W 1 /H 1 /BPC 8 /CS /Absent ID \x80 EI Q"[..],
            "model/inline-image-colour-space-unresolved",
        ),
    ] {
        let logged = logged_by_the_walk(stream);
        assert!(
            logged.contains(class),
            "{}: {logged:?}",
            String::from_utf8_lossy(stream)
        );
    }
    // §7.3.6: a keyword inside an array is no operator, so the operands around it are the next
    // operator's and nothing is left over (`operator-in-TJ-array.pdf`).
    let logged = logged_by_the_walk(b"BT /F1 12 Tf 10 100 Td [(a) 0 Tc (b)] TJ ET");
    assert!(!logged.contains("model/operands-left-over"), "{logged:?}");
}

/// An `EMC` with no section open balances nothing, and this reader ignores it (Table 352), so under
/// `as-shown` it is written out alone where the page draws the same — `issue15629.pdf`'s form
/// closes one `BMC` twice (`doc/adr/3631`).
#[test]
fn an_emc_closing_nothing_is_written_out_alone() {
    let source = a_page_whose_stream_is(b"/Tx BMC 0 0 10 10 re f EMC EMC");
    let target = Target::Two(Level::B);
    let (report, output) = convert_with_plan(&source, &under("as-shown", target));
    let Decision::Authorised { losses, .. } = decision(&report, BASE_STANDARD) else {
        panic!("{:?}", decision(&report, BASE_STANDARD));
    };
    assert!(
        losses.iter().eq([Loss::ContentAsDrawn]),
        "{}",
        losses.words()
    );
    let output = output.expect("a file is written");
    assert_eq!(holds(&output, target).verdict(), Verdict::Conforms);
    let written: Vec<&[u8]> = conversion(&report)
        .runs_written_as_drawn
        .iter()
        .map(|written| written.run.as_slice())
        .collect();
    assert_eq!(written, [&b"EMC"[..]]);
    let contents = decoded_contents(&output);
    let text = String::from_utf8_lossy(&contents);
    assert!(text.contains("/Tx BMC 0 0 10 10 re f EMC"), "{text}");
    assert_eq!(text.matches("EMC").count(), 1, "{text}");
}

/// §7.8.2: "all of the operands needed by an operator shall immediately precede that operator",
/// so an operator given operands in front of its own is run with the last of them and the rest
/// dropped. Only those are written out, the operator kept with its own, so the page draws the same
/// whether it draws or not (`doc/adr/3641`): `T02-05-01_008`'s `/F0 36. (…) Tj` keeps its show.
/// Where the operator also needs another repair it is written out whole, and where that changes
/// the page it is left, refused by a class's name under `as-shown` and photographed under
/// `as-if-printed` (`doc/adr/3631`).
#[test]
fn operands_left_over_are_written_out_alone_and_the_operator_kept() {
    let target = Target::Two(Level::B);
    let written_out = |report: &Report| -> Vec<Vec<u8>> {
        conversion(report)
            .runs_written_as_drawn
            .iter()
            .map(|written| written.run.clone())
            .collect()
    };
    for (stream, leftover, kept) in [
        (
            &b"5 /F1 12 Tf 0 0 10 10 re f"[..],
            &b"5"[..],
            &b"/F1 12 Tf"[..],
        ),
        (b"1 2 3 m 10 10 l S", b"1", b"2 3 m"),
        (b"BT /F1 12 Tf /F1 36. (a) Tj ET", b"/F1 36.", b"(a) Tj"),
        (b"[1] 0 0 m 10 10 l S", b"[1]", b"0 0 m"),
    ] {
        let source = a_page_whose_stream_is(stream);
        let (report, output) = convert_with_plan(&source, &under("as-shown", target));
        let output = output.unwrap_or_else(|| panic!("{:?}", refusals(&report)));
        assert_eq!(holds(&output, target).verdict(), Verdict::Conforms);
        assert_eq!(written_out(&report), [leftover.to_vec()]);
        let contents = decoded_contents(&output);
        let text = String::from_utf8_lossy(&contents);
        assert!(
            text.contains(&*String::from_utf8_lossy(kept)),
            "the operator keeps its own operands: {text}"
        );
    }

    // A show outside a text object with a leftover in front: the leftover is written out alone,
    // and a text object is added around the show, which this reader drew by the identity
    // matrices `ET` left (`doc/adr/3641`, `doc/adr/3661`).
    let drawing = a_page_whose_stream_is(b"BT /F1 12 Tf ET 1 (a) Tj");
    let (report, output) = convert_with_plan(&drawing, &under("as-shown", target));
    let output = output.unwrap_or_else(|| panic!("{:?}", refusals(&report)));
    assert_eq!(holds(&output, target).verdict(), Verdict::Conforms);
    assert_eq!(written_out(&report), [b"1".to_vec()]);
    assert!(
        conversion(&report).photographed.is_empty(),
        "the page is not photographed"
    );
    let text = contents_text(&output);
    assert!(text.ends_with("ET BT (a) Tj ET"), "{text}");
}

/// ISO 19005-2 section 6.2.10 and ISO 19005-4 section 6.2.9 require a blend mode ISO 32000-2
/// defines, and §11.6.3 gives `Normal` only to an array none of whose names is recognised. A bare
/// name outside Tables 134 and 135 is drawn by this reader as `Normal`, so under a profile granting
/// `content-as-drawn` it is written as `/Normal`, the producer's name gone and the loss named, and
/// section 5.1's row does not photograph the page for it (`doc/adr/3643`).
#[test]
fn a_bare_blend_mode_name_is_written_as_the_normal_this_reader_draws() {
    let source = Conforming {
        resources: "/Font << /F1 6 0 R >> /ExtGState << /GS0 << /Type /ExtGState /BM /Sepia >> >>"
            .to_owned(),
        contents: Some((String::new(), b"/GS0 gs 0 0 10 10 re f".to_vec())),
        objects: vec![HELVETICA.to_owned()],
        ..Conforming::part_two()
    }
    .build();
    for target in [Target::Two(Level::B), Target::Four(Flavour::Plain)] {
        let (report, output) = convert_with_plan(&source, &under("as-shown", target));
        let output = output.unwrap_or_else(|| panic!("{:?}", refusals(&report)));
        assert_eq!(holds(&output, target).verdict(), Verdict::Conforms);
        assert!(
            matches!(
                decision(&report, "graphics/graphics-state-blend-modes-are-defined"),
                Decision::Authorised {
                    rewrite: Rewrite::BlendModeNormal,
                    losses,
                } if losses.contains(Loss::ContentAsDrawn)
            ),
            "the bare name is restated at the loss content-as-drawn"
        );
        assert!(
            conversion(&report).photographed.is_empty(),
            "nothing is photographed"
        );
        let written = String::from_utf8_lossy(&output).into_owned();
        assert!(written.contains("/BM /Normal"), "{written}");
        assert!(!written.contains("Sepia"), "the producer's name is gone");
    }
}
