//! A simple font outside the standard 14 that states neither a descriptor nor `/Widths` converts
//! behind the loss `layout-from-this-viewer` (`doc/questions/A687`, `doc/adr/2504`).
//!
//! ISO 32000-2 §9.6.2.1's Table 109 makes `/Widths` and `/FontDescriptor` required of every
//! simple font but the fourteen; the layout of any other name is stated by nothing, so what the
//! output states is this viewer's. `as-if-printed` discards the loss and `as-shown` stops on it.

use super::*;

/// A page showing `Hello` in a simple font stated by name alone: the subtype and `/BaseFont` the
/// caller writes (`base_font` is the whole entry, or empty), no `/FontDescriptor`, no `/Widths`.
fn a_font_stating_nothing(subtype: &str, base_font: &str, extra: &str) -> Vec<u8> {
    Conforming {
        resources: "/Font << /F1 6 0 R >> /ColorSpace << /CS0 [/CalGray << /WhitePoint \
                    [0.9505 1.0 1.089] >>] >>"
            .to_owned(),
        contents: Some((
            String::new(),
            b"/CS0 cs 0 sc BT /F1 12 Tf 10 100 Td (Hello) Tj ET".to_vec(),
        )),
        objects: vec![format!(
            "<< /Type /Font /Subtype /{subtype} {base_font} /Encoding /WinAnsiEncoding {extra} >>"
        )],
        ..Conforming::part_two()
    }
    .build()
}

/// Every loss a face embedded over a font the file did not describe can incur, authorised: which
/// of the face losses a fixture incurs depends on the faces the machine has installed.
fn every_font_loss() -> Authorisations {
    let mut authorised = the_face_losses_authorised();
    authorised.authorise(Loss::LayoutFromThisViewer);
    authorised
}

/// The converted font's entry, converted to `target` with every font loss authorised.
fn converted(source: &[u8], target: Target) -> (Report, Vec<u8>) {
    let (report, output) = convert(source, target, every_font_loss());
    let output = written(&report, output);
    assert_eq!(
        holds(&output, target).verdict(),
        Verdict::Conforms,
        "{target}: what was written is held to the target again"
    );
    (report, output)
}

#[test]
fn a_named_face_outside_the_fourteen_converts_under_the_loss_at_both_parts() {
    for subtype in ["Type1", "MMType1", "TrueType"] {
        for target in [Target::Two(Level::B), Target::Four(Flavour::Plain)] {
            let source = a_font_stating_nothing(subtype, "/BaseFont /Arial", "");
            let (report, output) = converted(&source, target);
            let font = conversion(&report).substituted.first().expect("one font");
            assert!(
                font.losses.contains(Loss::LayoutFromThisViewer),
                "{subtype} {target}: {}",
                font.losses.words()
            );
            let (held, dict) = described_font(&output);
            assert_eq!(name_entry(&held, &dict, "BaseFont"), "Arial", "{subtype}");
            let descriptor = held.get_key(&dict, "FontDescriptor");
            let descriptor = descriptor.as_dict().expect("a descriptor");
            assert_eq!(name_entry(&held, descriptor, "FontName"), "Arial");
            assert!(
                held.get_key(&dict, "Widths").as_array().is_some(),
                "{subtype}: the widths the page is laid out by are stated"
            );
            // The face's program sits under a key of the descriptor, so the font is drawn from
            // its own program and from no substitute.
            assert!(
                ["FontFile2", "FontFile3"]
                    .iter()
                    .any(|key| !held.get_key(descriptor, key).is_null()),
                "{subtype}: a program is embedded"
            );
        }
    }
}

#[test]
fn the_widths_written_are_the_layout_the_viewer_drew_to_a_thousandth_of_an_em() {
    // ADR 2477's proof: the converted font read back through this reader's own font loader
    // advances every one of the 256 codes as the source font did, and is drawn from its own
    // program rather than from a substitute.
    let source = a_font_stating_nothing("TrueType", "/BaseFont /Arial", "");
    let target = Target::Two(Level::B);
    let (_, output) = converted(&source, target);
    let drawn = Document::open_with_limits(source.clone(), Limits::DEFAULT).expect("it opens");
    let before = drawn
        .get(ObjectId::new(6, 0))
        .as_dict()
        .cloned()
        .expect("font");
    let before = pdf_font::LoadedFont::load(&drawn, &before, "F1").expect("the source loads");
    let (held, dict) = described_font(&output);
    let after = pdf_font::LoadedFont::load(&held, &dict, "F1").expect("the output loads");
    assert!(
        after.substitute_program().is_none(),
        "the output is drawn from the program it embeds"
    );
    for code in 0..=u8::MAX {
        let code = pdf_font::Code::single_byte(code);
        let (was, is) = (before.advance(code), after.advance(code));
        assert!(
            (was - is).abs() < 0.001,
            "code {}: {was} em before, {is} em after",
            code.value()
        );
    }
}

#[test]
fn a_font_naming_no_base_font_is_given_one_equal_to_its_descriptors_font_name() {
    // Table 120's `/FontName`: "This name shall be the same as the value of BaseFont in the font
    // or CIDFont dictionary that refers to this font descriptor." veraPDF 6.2.11.2 test 3.
    let source = a_font_stating_nothing("TrueType", "", "");
    for target in [Target::Two(Level::B), Target::Four(Flavour::Plain)] {
        let (report, output) = converted(&source, target);
        let (held, dict) = described_font(&output);
        let base_font = name_entry(&held, &dict, "BaseFont");
        assert!(!base_font.is_empty(), "{target}: a BaseFont is supplied");
        assert!(
            !base_font.contains(' '),
            "{base_font:?} is a PostScript name"
        );
        let descriptor = held.get_key(&dict, "FontDescriptor");
        let descriptor = descriptor.as_dict().expect("a descriptor");
        assert_eq!(name_entry(&held, descriptor, "FontName"), base_font);
        assert!(
            conversion(&report)
                .substituted
                .first()
                .is_some_and(|font| font.losses.contains(Loss::LayoutFromThisViewer)),
            "{target}"
        );
    }
}

#[test]
fn the_name_supplied_is_the_postscript_name_the_embedded_program_states() {
    // ISO 32000-2 §9.6.3: "If the TrueType or OpenType font program's "name" table contains a
    // PostScript language name, it should be used." The name table's name ID 6 is read from the
    // program the output carries, not from anything the converter wrote beside it.
    use skrifa::MetadataProvider as _;
    let source = a_font_stating_nothing("TrueType", "", "");
    for target in [Target::Two(Level::B), Target::Four(Flavour::Plain)] {
        let (_, output) = converted(&source, target);
        let (held, dict) = described_font(&output);
        let descriptor = held.get_key(&dict, "FontDescriptor");
        let descriptor = descriptor.as_dict().expect("a descriptor");
        let program = held.get_key(descriptor, "FontFile2");
        let program = program
            .as_stream()
            .expect("an sfnt program under FontFile2");
        let bytes = held.decoded_stream_data(program).expect("it decodes");
        let font = skrifa::FontRef::new(&bytes).expect("the program is an sfnt");
        let stated: String = font
            .localized_strings(skrifa::string::StringId::POSTSCRIPT_NAME)
            .english_or_first()
            .expect("the program states a PostScript name")
            .chars()
            .collect();
        assert_eq!(name_entry(&held, &dict, "BaseFont"), stated, "{target}");
        assert_eq!(
            name_entry(&held, descriptor, "FontName"),
            stated,
            "{target}"
        );
    }
}

#[test]
fn a_base_font_the_file_states_is_kept_byte_for_byte() {
    // A name with bytes outside ASCII (UTF-8, which ISO 19005 binds names to) must survive as the
    // same bytes in both entries, because Table 120 asks for equality of the names.
    let source = a_font_stating_nothing("TrueType", "/BaseFont /Ari#c3#a9l", "");
    let (_, output) = converted(&source, Target::Two(Level::B));
    let (held, dict) = described_font(&output);
    let bytes = |dict: &pdf_syntax::object::Dictionary, key: &str| {
        held.get_key(dict, key)
            .as_name()
            .map(|name| name.as_bytes().to_vec())
    };
    let descriptor = held.get_key(&dict, "FontDescriptor");
    let descriptor = descriptor.as_dict().expect("a descriptor");
    assert_eq!(
        bytes(&dict, "BaseFont"),
        Some("Ari\u{e9}l".as_bytes().to_vec())
    );
    assert_eq!(
        bytes(descriptor, "FontName"),
        Some("Ari\u{e9}l".as_bytes().to_vec())
    );
}

#[test]
fn without_the_loss_the_conversion_names_it_and_writes_nothing() {
    let source = a_font_stating_nothing("TrueType", "/BaseFont /Tahoma", "");
    let mut partial = every_font_loss();
    partial.layout_from_this_viewer = false;
    let (report, output) = convert(&source, Target::Two(Level::B), partial);
    assert!(output.is_none(), "nothing is written");
    let Decision::Unauthorised { losses, .. } = decision(&report, "fonts/font-programs-embedded")
    else {
        panic!(
            "the font requirement waits on the loss: {:?}",
            decision(&report, "fonts/font-programs-embedded")
        );
    };
    assert!(
        losses.contains(Loss::LayoutFromThisViewer),
        "{}",
        losses.words()
    );
}

#[test]
fn the_standard_fourteen_by_name_do_not_incur_the_loss() {
    // Annex D publishes their metrics, so the layout is the standard's and not this viewer's.
    let source = a_font_stating_nothing("Type1", "/BaseFont /Helvetica", "");
    let (report, _) = converted(&source, Target::Two(Level::B));
    let font = conversion(&report).substituted.first().expect("one font");
    assert!(!font.losses.contains(Loss::LayoutFromThisViewer));
}

#[test]
fn a_font_stating_widths_but_no_descriptor_outside_the_fourteen_stays_refused() {
    let source = a_font_stating_nothing(
        "TrueType",
        "/BaseFont /Arial",
        "/FirstChar 72 /LastChar 72 /Widths [722]",
    );
    let (report, output) = convert(&source, Target::Two(Level::B), every_font_loss());
    assert!(output.is_none(), "nothing is written");
    let Decision::Refused(because) = decision(&report, "fonts/font-programs-embedded") else {
        panic!("the font requirement is refused");
    };
    assert!(
        because
            .sentence()
            .contains("states /Widths and no FontDescriptor"),
        "{}",
        because.sentence()
    );
}

#[test]
fn a_font_of_a_subtype_no_simple_font_has_stays_refused() {
    let source = a_font_stating_nothing("Type7", "/BaseFont /Times-Italic", "");
    let (report, output) = convert(&source, Target::Two(Level::B), every_font_loss());
    assert!(output.is_none(), "nothing is written");
    let Decision::Refused(because) = decision(&report, "fonts/font-programs-embedded") else {
        panic!("the font requirement is refused");
    };
    assert!(
        because
            .sentence()
            .contains("none of Type1, MMType1 and TrueType"),
        "{}",
        because.sentence()
    );
}

#[test]
fn a_base_font_that_is_no_name_stays_refused() {
    let source = a_font_stating_nothing("TrueType", "/BaseFont (Arial)", "");
    let (report, output) = convert(&source, Target::Two(Level::B), every_font_loss());
    assert!(output.is_none(), "nothing is written");
    let Decision::Refused(because) = decision(&report, "fonts/font-programs-embedded") else {
        panic!("the font requirement is refused");
    };
    assert!(
        because.sentence().contains("a BaseFont that is not a name"),
        "{}",
        because.sentence()
    );
}

#[test]
fn the_printing_profile_discards_the_loss_and_the_window_profile_stops_on_it() {
    for (name, discards) in [("as-if-printed", true), ("as-shown", false)] {
        let profile = pdf_transform::profiles::named(name).expect("a shipped profile");
        let configuration =
            pdf_transform::archive::Configuration::read(profile.text, Target::Two(Level::B))
                .expect("the shipped profile reads");
        let authorised = configuration.authorisations(Target::Two(Level::B));
        assert_eq!(
            authorised.grants(Loss::LayoutFromThisViewer),
            discards,
            "{name}"
        );
    }
}

#[test]
fn the_files_own_history_says_the_widths_are_this_viewers() {
    let source = a_font_stating_nothing("TrueType", "/BaseFont /Arial", "");
    let (report, _) = converted(&source, Target::Two(Level::B));
    let recorded = conversion(&report).recorded.as_deref().expect("a history");
    assert!(recorded.contains("layout-from-this-viewer"), "{recorded}");
    assert!(
        recorded.contains("the widths written are the layout this viewer drew it with"),
        "{recorded}"
    );
}

#[test]
fn the_report_does_not_say_the_face_agrees_with_widths_the_file_never_stated() {
    let source = a_font_stating_nothing("TrueType", "/BaseFont /Arial", "");
    let (report, _) = converted(&source, Target::Two(Level::B));
    let text = conversion(&report).render();
    assert!(
        text.contains(
            "the file states no widths for it, so the face's own advances are written as its widths"
        ),
        "{text}"
    );
    assert!(
        !text.contains("the face's own advances are the widths the file states"),
        "{text}"
    );
}
