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

/// §7.8.2's `BX` and `EX` "shall occur in pairs": a stream closing a compatibility section it never
/// opened is read as if balanced, and the checker's grammar finds it before the fence asks which
/// pages to photograph (`doc/adr/3614`). No write answers it: under `as-shown` the row keeps its
/// refusal by the class's name; under `as-if-printed` the page is photographed.
#[test]
fn a_compatibility_section_closed_twice_is_found_before_the_photographs() {
    let target = Target::Two(Level::B);
    let class = "model/compatibility-sections-unbalanced";
    let source = a_page_whose_stream_is(b"0 0 10 10 re f EX");
    let (report, output) = convert_with_plan(&source, &under("as-shown", target));
    assert!(output.is_none(), "as-shown writes no file");
    let refused = refusals(&report);
    assert!(
        refused.iter().any(|sentence| sentence.contains(class)),
        "refused by the class's name: {refused:?}"
    );
    let (report, output) = convert_with_plan(&source, &under("as-if-printed", target));
    let output = output.expect("as-if-printed photographs the page");
    assert_eq!(holds(&output, target).verdict(), Verdict::Conforms);
    assert_eq!(conversion(&report).photographed.len(), 1);
}

/// ISO 32000-2 Table 352's `BMC` begins a sequence "terminated by a balancing EMC operator", and
/// §14.6.1 has each sequence "entirely contained within a single content stream". This reader
/// ends a section left open where its stream ends, so an `EMC` written there draws the same page,
/// and both profiles write it rather than photograph the page (`doc/adr/3734`). The loss is
/// `content-as-drawn`: where the producer meant the section to end is this reader's reading.
#[test]
fn a_section_left_open_is_closed_where_its_stream_ends() {
    let source = a_page_whose_stream_is(b"/Tag BMC 0 0 10 10 re f /Inner BMC 0 0 5 5 re f % ends");
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
            assert!(conversion(&report).photographed.is_empty(), "{profile}");
            let output = output.expect("a file is written");
            assert_eq!(holds(&output, target).verdict(), Verdict::Conforms);
            let text = contents_text(&output);
            // The comment the stream ends in is ended by the line before each `EMC`.
            assert!(text.ends_with("0 0 5 5 re f % ends EMC EMC"), "{text}");
        }
    }
}

/// **A write that answers one class on a page answers none it left** (`doc/adr/3733`). The page's
/// `/Contents` is two streams, which §7.8.2 reads as one: the first opens a section and ends with
/// an operand the `Q` opening the second takes, so that `Q` restores nothing (§8.4.2) and has an
/// operand left over (§7.8.2). Its operand stands in the other stream, so the walk cannot write it
/// out; it writes the section's `EMC` where the second stream ends (`doc/adr/3734`) and the
/// exponent's decimal in the first (`doc/adr/3677`), and the page draws the same. Read again with
/// those written, the page still needs both of the `Q`'s repairs: `as-shown` refuses by one of
/// their names, and `as-if-printed` photographs the page rather than count it answered.
#[test]
fn a_page_holding_a_class_the_write_left_is_photographed_though_another_was_written() {
    let second = b"Q";
    let built = Conforming {
        resources: "/Font << /F1 6 0 R >>".to_owned(),
        contents: Some((String::new(), b"/Tag BMC 1e1 0 10 10 re f 5".to_vec())),
        objects: vec![HELVETICA.to_owned()],
        binary_objects: vec![stream(&format!("/Length {}", second.len()), second)],
        // Room for the array below, so that no offset in the file moves.
        page: " ".repeat(8),
        ..Conforming::part_two()
    }
    .build();
    let (was, now) = (
        &b"/Contents 4 0 R          "[..],
        &b"/Contents [4 0 R 7 0 R]  "[..],
    );
    let at = built
        .windows(was.len())
        .position(|window| window == was)
        .expect("the fixture names its content stream");
    let mut source = built.clone();
    source[at..at + was.len()].copy_from_slice(now);
    let target = Target::Two(Level::B);
    let (report, output) = convert_with_plan(&source, &under("as-shown", target));
    assert!(output.is_none(), "as-shown writes no file");
    let refused = refusals(&report);
    assert!(
        refused.iter().any(|sentence| {
            sentence.contains("model/restore-with-nothing-saved")
                || sentence.contains("model/operands-left-over")
        }),
        "refused by a class the write left: {refused:?}"
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

/// §8.6.8 has `sc` take as many operands as the current space has components; three in the initial
/// `DeviceGray` (§8.4.1) are drawn by this reader in `DeviceRGB`, as `rg` draws them, so the operator
/// is written as `rg` with its operands kept where the page draws the same (`doc/adr/3679`).
#[test]
fn colour_operands_that_do_not_fit_are_written_as_the_device_operator_this_reader_drew() {
    let source = a_page_whose_stream_is(b"0.5 0.25 1 sc 0 0 10 10 re f");
    for target in [Target::Two(Level::B), Target::Four(Flavour::Plain)] {
        let (output, losses) = as_shown(&source, target);
        assert!(
            losses.iter().eq([Loss::ContentAsDrawn]),
            "{}",
            losses.words()
        );
        assert_eq!(contents_text(&output), "0.5 0.25 1 rg 0 0 10 10 re f");
    }
}

/// A page stating `selects` (nothing where empty) and then painting a form whose `scn` gives three
/// operands, with the CIE-based `Lab` space (§8.6.5.4), three components, as `/CS0`.
fn a_page_painting_a_three_operand_form(selects: &str) -> Vec<u8> {
    let form = b"1 0 0 scn 0 0 10 10 re f";
    Conforming {
        resources: "/ColorSpace << /CS0 [/Lab << /WhitePoint [0.9505 1 1.089] >>] >> \
                    /XObject << /Fm0 6 0 R >>"
            .to_owned(),
        contents: Some((String::new(), format!("{selects} /Fm0 Do").into_bytes())),
        binary_objects: vec![stream(
            &format!(
                "/Type /XObject /Subtype /Form /BBox [0 0 20 20] /Length {}",
                form.len()
            ),
            form,
        )],
        ..Conforming::part_two()
    }
    .build()
}

/// §8.10.1 after `Do`'s steps: "the initial graphics state for the form shall be inherited from
/// the graphics state that is in effect at the time Do is invoked". So three operands fit a form
/// painted where the page selected a three-component space, and the validator finds no repair;
/// painted in §8.4.1's initial `DeviceGray`, the same form needs §8.6.8's (`doc/adr/3697`).
#[test]
fn a_form_is_judged_in_the_colour_space_its_do_left_in_force() {
    let target = Target::Two(Level::B);
    let inherits = holds(&a_page_painting_a_three_operand_form("/CS0 cs"), target);
    assert!(
        !inherits
            .failures()
            .any(|judgement| judgement.id == BASE_STANDARD),
        "the form inherits Lab"
    );
    let initial = holds(&a_page_painting_a_three_operand_form(""), target);
    assert!(
        initial
            .failures()
            .any(|judgement| judgement.id == BASE_STANDARD),
        "the form starts in DeviceGray"
    );
}

/// §11.6.5.1 makes a soft mask's `/G` a transparency group `XObject`, run in the initial state
/// (the interpreter's `GraphicsState::initial`), so three `scn` operands in it are drawn in
/// `DeviceRGB`. The repair is logged at the group's own object, which the write respells as `rg`;
/// logged at the page, whose stream no write touches, it stayed refused (`doc/adr/3697`).
#[test]
fn a_repair_inside_a_soft_mask_group_is_answered_at_the_group() {
    let group = b"0 0 0 scn 0 0 20 20 re f";
    let source = Conforming {
        resources: "/ExtGState << /GS0 << /Type /ExtGState /SMask << /Type /Mask /S /Alpha \
                    /G 6 0 R >> >> >>"
            .to_owned(),
        contents: Some((String::new(), b"/GS0 gs 0 0 10 10 re f".to_vec())),
        binary_objects: vec![stream(
            &format!(
                "/Type /XObject /Subtype /Form /BBox [0 0 20 20] /Group << /Type /Group \
                 /S /Transparency >> /Length {}",
                group.len()
            ),
            group,
        )],
        ..Conforming::part_two()
    }
    .build();
    let target = Target::Two(Level::B);
    assert!(
        holds(&source, target)
            .failures()
            .any(|judgement| judgement.id == BASE_STANDARD),
        "the group's scn needs the repair"
    );
    let (_, losses) = as_shown(&source, target);
    assert!(losses.contains(Loss::ContentAsDrawn), "{}", losses.words());
}

/// §7.3.3 forbids a writer the exponential form ("such as 6.02E23") and states nothing for a
/// reader; this reader reads `1.2e1` as twelve, so the run is written as `12` (`doc/adr/3677`).
#[test]
fn a_number_in_exponential_form_is_written_as_the_value_it_was_read_as() {
    let source = a_page_whose_stream_is(b"BT /F1 1.2e1 Tf 10 100 Td (Hello) Tj ET");
    for target in [Target::Two(Level::B), Target::Four(Flavour::Plain)] {
        let (output, losses) = as_shown(&source, target);
        assert!(
            losses.iter().eq([Loss::ContentAsDrawn]),
            "{}",
            losses.words()
        );
        assert_eq!(
            contents_text(&output),
            "BT /F1 12 Tf 10 100 Td (Hello) Tj ET"
        );
    }
}

/// §8.9.7's Table 91 abbreviation and its full name are one key, and Errata Collection 3 (Issue #3)
/// makes the abbreviated one take precedence: `/W 1` is the width this reader read, and the
/// contradicting `/Width 2` is written out where the page draws the same (`doc/adr/3678`).
#[test]
fn an_inline_image_key_stated_twice_keeps_the_entry_this_reader_read() {
    let source = a_page_whose_stream_is(
        b"q 10 0 0 10 0 0 cm BI /W 1 /H 1 /BPC 8 /CS /G /Width 2 ID \x80 EI Q",
    );
    for target in [Target::Two(Level::B), Target::Four(Flavour::Plain)] {
        let (output, losses) = as_shown(&source, target);
        assert!(
            losses.iter().eq([Loss::ContentAsDrawn]),
            "{}",
            losses.words()
        );
        let text = contents_text(&output);
        assert!(text.contains("BI /W 1 /H 1 /BPC 8 /CS /G ID"), "{text}");
        assert!(!text.contains("/Width"), "{text}");
    }
}

/// Table 106's `Td` sets the text matrix and the line matrix to a translation by its operands
/// times the line matrix held, so a move from a matrix the walk can state is one it states too
/// (`doc/adr/3736`). The second `Td` stands outside a text object after the `q Q` that ended the
/// one added around the first, and reads the line matrix the first `Td` set; the text object added
/// around it opens with a `Tm` stating that matrix.
#[test]
fn a_move_from_a_matrix_the_walk_can_state_is_restated_across_a_cut() {
    let source = a_page_whose_stream_is(b"/F1 12 Tf 10 100 Td (A) Tj q Q 0 -20 Td (B) Tj");
    for target in [Target::Two(Level::B), Target::Four(Flavour::Plain)] {
        let (output, losses) = as_shown(&source, target);
        assert!(
            losses.iter().eq([Loss::ContentAsDrawn]),
            "{}",
            losses.words()
        );
        let text = contents_text(&output);
        assert!(
            text.contains("BT 10 100 Td (A) Tj ET q Q BT 1 0 0 1 10 100 Tm 0 -20 Td (B) Tj ET"),
            "{text}"
        );
    }
}

/// ISO 32000-2 §7.3.4.3 gives a hexadecimal string's bytes as digits and white space, and gives a
/// byte that is neither no meaning; this reader skips it and reads the digits around it. Written
/// without it, the string is the one the reader read and the page draws the same, so both section
/// 5.1's repair and section 6.1.6's row are answered at `content-as-drawn`, the reading being this
/// reader's (`doc/adr/3735`).
#[test]
fn a_byte_a_hexadecimal_string_holds_that_is_no_digit_is_written_out_as_this_reader_skipped_it() {
    let source = a_page_whose_stream_is(b"BT /F1 12 Tf 10 100 Td <48 65 6Cz6C 6F> Tj ET");
    for profile in ["as-shown", "as-if-printed"] {
        let target = Target::Two(Level::B);
        let (report, output) = convert_with_plan(&source, &under(profile, target));
        let output = output.unwrap_or_else(|| panic!("{profile}: {:?}", refusals(&report)));
        assert_eq!(holds(&output, target).verdict(), Verdict::Conforms);
        assert!(conversion(&report).photographed.is_empty(), "{profile}");
        for row in [
            BASE_STANDARD,
            "file-structure/hexadecimal-string-holds-only-digits",
        ] {
            let Decision::Authorised { losses, .. } = decision(&report, row) else {
                panic!("{profile} {row}: {:?}", decision(&report, row));
            };
            assert!(
                losses.iter().eq([Loss::ContentAsDrawn]),
                "{profile} {row}: {}",
                losses.words()
            );
        }
        let text = contents_text(&output);
        assert!(text.contains("<48 65 6C6C 6F> Tj"), "{profile}: {text}");
    }
}

/// §8.9.7: unfiltered inline image data is exactly as long as §8.9.3's arithmetic says, here one
/// byte. Three more stand before `EI`, which this reader found by searching and never decoded; cut,
/// the data ends where the arithmetic predicts, the page draws the same, and section 5.1's repair
/// is answered at `content-as-drawn` (`doc/adr/3735`).
#[test]
fn inline_image_data_past_the_extent_its_decoding_reads_is_cut() {
    let source =
        a_page_whose_stream_is(b"q 10 0 0 10 0 0 cm BI /W 1 /H 1 /BPC 8 /CS /G ID \x80XYZ EI Q");
    let target = Target::Two(Level::B);
    let (output, losses) = as_shown(&source, target);
    assert!(
        losses.iter().eq([Loss::ContentAsDrawn]),
        "{}",
        losses.words()
    );
    let contents = decoded_contents(&output);
    assert!(
        contents.windows(5).any(|window| window == b"\x80 EI "),
        "{}",
        String::from_utf8_lossy(&contents)
    );
    assert!(!contents.windows(3).any(|window| window == b"XYZ"));
}

/// §8.10.1 starts a form in the graphics state in force at its `Do`, so the form here is drawn
/// twice in two spaces: in §8.4.1's initial `DeviceGray`, where its three `scn` operands do not
/// fit (§8.6.8) and this reader draws them as `rg`, and under `Lab`, where they do. Its one stream
/// is judged in neither (`doc/adr/3697`): written as `rg` it would draw the `Lab` occurrence as
/// RGB, so no write of it draws both the same. `as-shown` refuses by the class's own sentence,
/// which says so (`doc/adr/3802`), and `as-if-printed` photographs the page.
#[test]
fn a_form_drawn_in_two_colour_spaces_is_refused_by_name() {
    let source = a_page_painting_a_three_operand_form_twice();
    let target = Target::Two(Level::B);
    let (report, output) = convert_with_plan(&source, &under("as-shown", target));
    assert!(output.is_none(), "as-shown writes no file");
    let refused = refusals(&report);
    assert!(
        refused.iter().any(|sentence| {
            sentence.contains("model/colour-operands-do-not-fit-the-space")
                && sentence.contains("ADR 3802")
        }),
        "{refused:?}"
    );
    let (report, output) = convert_with_plan(&source, &under("as-if-printed", target));
    let output = output.expect("as-if-printed photographs the page");
    assert_eq!(holds(&output, target).verdict(), Verdict::Conforms);
    assert_eq!(conversion(&report).photographed.len(), 1);
}

/// [`a_page_painting_a_three_operand_form`]'s form, painted first in the initial `DeviceGray` and
/// then after the page selected `Lab`.
fn a_page_painting_a_three_operand_form_twice() -> Vec<u8> {
    let form = b"1 0 0 scn 0 0 10 10 re f";
    Conforming {
        resources: "/ColorSpace << /CS0 [/Lab << /WhitePoint [0.9505 1 1.089] >>] >> \
                    /XObject << /Fm0 6 0 R >>"
            .to_owned(),
        contents: Some((String::new(), b"q /Fm0 Do Q /CS0 cs /Fm0 Do".to_vec())),
        binary_objects: vec![stream(
            &format!(
                "/Type /XObject /Subtype /Form /BBox [0 0 20 20] /Length {}",
                form.len()
            ),
            form,
        )],
        ..Conforming::part_two()
    }
    .build()
}

/// The text of `source`'s output under `as-shown`, for both parts, each asserted to lose only
/// `content-as-drawn`.
fn written_as_shown(source: &[u8]) -> Vec<String> {
    [Target::Two(Level::B), Target::Four(Flavour::Plain)]
        .into_iter()
        .map(|target| {
            let (output, losses) = as_shown(source, target);
            assert!(
                losses.iter().eq([Loss::ContentAsDrawn]),
                "{}",
                losses.words()
            );
            contents_text(&output)
        })
        .collect()
}

/// Table 106's `T*` "has the same effect as the code 0 - T l TD where T l denotes the current
/// leading parameter in the text state", and Table 107's `'` and `"` begin with that move, so each
/// moves the line matrix by the leading the last `TL` set (`doc/adr/3801`). After a cut, the text
/// object added around the next `Td` opens with the matrix each move stated: `10 100` moved down by
/// the leading 14 is `10 86`.
#[test]
fn a_move_by_the_leading_is_restated_across_a_cut() {
    for shown in [&b"T* (A) Tj"[..], b"(A) '", b"1 2 (A) \""] {
        let mut source = b"/F1 12 Tf 14 TL 10 100 Td ".to_vec();
        source.extend_from_slice(shown);
        source.extend_from_slice(b" q Q 0 -20 Td (B) Tj");
        for text in written_as_shown(&a_page_whose_stream_is(&source)) {
            assert!(
                text.contains("q Q BT 1 0 0 1 10 86 Tm 0 -20 Td (B) Tj ET"),
                "{}: {text}",
                String::from_utf8_lossy(shown)
            );
        }
    }
}

/// Table 106's `TD` "shall have the same effect as this code: - t y TL t x t y Td", so it sets the
/// leading to the negative of its `ty`, here −100, and the `T*` after it moves by `0 100` to
/// `10 200`; and §8.4.2's `Q` restores the leading the `q` saved with the graphics state, so the
/// `30 TL` between them is undone and the `T*` moves by the 14 before it (`doc/adr/3801`).
#[test]
fn the_leading_a_td_sets_and_a_restore_brings_back_moves_the_line() {
    let cases: [(&[u8], &str); 2] = [
        (b"10 100 TD T* (A) Tj", "BT 1 0 0 1 10 200 Tm 0 -20 Td"),
        (
            b"14 TL q 30 TL Q 10 100 Td T* (A) Tj",
            "BT 1 0 0 1 10 86 Tm 0 -20 Td",
        ),
    ];
    for (moves, expected) in cases {
        let mut source = b"/F1 12 Tf ".to_vec();
        source.extend_from_slice(moves);
        source.extend_from_slice(b" q Q 0 -20 Td (B) Tj");
        for text in written_as_shown(&a_page_whose_stream_is(&source)) {
            assert!(text.contains(expected), "{expected}: {text}");
        }
    }
}

/// A `Td` composed onto a matrix a `Tm` stated (`doc/adr/3736`): Table 106 makes it the
/// translation by `5 5` times `2 0 0 2 10 100`, whose origin is `10 + 2 × 5` and `100 + 2 × 5`.
#[test]
fn a_move_from_a_matrix_a_tm_stated_is_restated_across_a_cut() {
    let source =
        a_page_whose_stream_is(b"/F1 12 Tf 2 0 0 2 10 100 Tm 5 5 Td (A) Tj q Q 0 -20 Td (B) Tj");
    for text in written_as_shown(&source) {
        assert!(
            text.contains("q Q BT 2 0 0 2 20 110 Tm 0 -20 Td (B) Tj ET"),
            "{text}"
        );
    }
}

/// §8.9.7 reads an inline image's data through its filters, and §7.4.2's `ASCIIHexDecode` ends at
/// its EOD marker `>`: the three bytes after it stand before `EI`, which this reader found by
/// searching and never decoded. Cut, the data ends at the marker, the white space §8.9.7 asks for
/// before `EI` stays, and the page draws the same (`doc/adr/3735`).
#[test]
fn inline_image_data_past_its_filters_end_of_data_is_cut() {
    let source = a_page_whose_stream_is(
        b"q 10 0 0 10 0 0 cm BI /W 1 /H 1 /BPC 8 /CS /G /F /AHx ID 80>XYZ EI Q",
    );
    for target in [Target::Two(Level::B), Target::Four(Flavour::Plain)] {
        let (output, losses) = as_shown(&source, target);
        assert!(
            losses.iter().eq([Loss::ContentAsDrawn]),
            "{}",
            losses.words()
        );
        let contents = decoded_contents(&output);
        let text = String::from_utf8_lossy(&contents);
        assert!(text.contains("ID 80> EI Q"), "{text}");
        assert!(!text.contains("XYZ"), "{text}");
    }
}
