//! A rendered TrueType font whose codes reach their glyphs by a route this reader chose is
//! converted by restating the route in the program (`doc/adr/2520`).
//!
//! ISO 32000-2 §9.6.5.4 lets a processor "supply a mapping of its choosing" for a code the
//! font's encoding and program leave open, and ISO 19005-2 section 6.2.11.6 and ISO 19005-4
//! section 6.2.10.6 withdraw that licence. The conversion therefore writes the glyph this reader
//! drew into the program's `cmap` as the symbolic route §9.6.5.4 describes, and the tests hold it
//! to the three things that make that a restatement and not a change: the glyph every code
//! reaches is the glyph it reached, the output conforms, and the content stream is the
//! producer's bytes.
use std::fmt::Write as _;

use pdf_font::{Code, LoadedFont};
use pdf_syntax::object::{Dictionary, Object};
use pdf_syntax::{Document, Limits};

use super::{
    Authorisations, Conforming, Decision, LIBERATION_SANS, Level, Loss, Rewrite, Target, Verdict,
    conversion, convert, decision, holds, stream,
};

const DIFFERENCES: &str = "fonts/non-symbolic-truetype-differences-are-listed-names";
const NEEDS_UNICODE: &str = "fonts/non-symbolic-truetype-differences-need-the-unicode-cmap";
const MAPS_EVERY_CODE: &str = "fonts/non-symbolic-truetype-program-maps-every-code";

/// Table 121's bit position 3, the Symbolic flag, and bit position 6, the Nonsymbolic one.
const SYMBOLIC: i64 = 4;
const NONSYMBOLIC: i64 = 32;

/// The font object the fixtures draw with.
fn font_of(held: &Document) -> Dictionary {
    held.get(pdf_syntax::object::ObjectId::new(6, 0))
        .as_dict()
        .cloned()
        .expect("the font object")
}

/// The glyph each of the 256 codes reaches in the one font of `bytes`, as this reader loads it.
fn glyphs_of(bytes: &[u8]) -> Vec<Option<u16>> {
    let held = Document::open_with_limits(bytes.to_vec(), Limits::DEFAULT).expect("it opens");
    let font = LoadedFont::load(&held, &font_of(&held), "").expect("the font loads");
    assert!(!font.is_substituted(), "the program is the document's own");
    (0..=u8::MAX)
        .map(|byte| font.glyph_index(Code::single_byte(byte)))
        .collect()
}

/// The (platform, encoding) record of each subtable the program of the font in `bytes` carries.
fn cmap_records(bytes: &[u8]) -> Vec<(u16, u16)> {
    let held = Document::open_with_limits(bytes.to_vec(), Limits::DEFAULT).expect("it opens");
    let font = font_of(&held);
    let descriptor = held
        .get_key(&font, "FontDescriptor")
        .as_dict()
        .cloned()
        .expect("the descriptor");
    let Object::Stream(program) = held.get_key(&descriptor, "FontFile2") else {
        panic!("the program is a stream");
    };
    let program = held.decoded_stream_data(&program).expect("it decodes");
    let byte = |at: usize| program[at];
    let word = |at: usize| u16::from_be_bytes([byte(at), byte(at.saturating_add(1))]);
    let long =
        |at: usize| u32::from_be_bytes([0, 1, 2, 3].map(|step| byte(at.saturating_add(step))));
    // The sfnt directory's records are sixteen bytes from the twelfth; a `cmap` table's records
    // are eight bytes from the fourth.
    let record =
        |first: usize, size: usize, index: usize| first.saturating_add(size.saturating_mul(index));
    let cmap = (0..usize::from(word(4)))
        .map(|index| record(12, 16, index))
        .find(|at| {
            (0..4)
                .map(|step| byte(at.saturating_add(step)))
                .eq(*b"cmap")
        })
        .map(|at| usize::try_from(long(at.saturating_add(8))).expect("an offset"))
        .expect("the program has a cmap");
    (0..usize::from(word(cmap.saturating_add(2))))
        .map(|index| {
            let at = record(cmap.saturating_add(4), 8, index);
            (word(at), word(at.saturating_add(2)))
        })
        .collect()
}

/// The descriptor flags and the dictionary's `/Encoding` presence, as written.
fn flags_and_encoding(bytes: &[u8]) -> (i64, bool) {
    let held = Document::open_with_limits(bytes.to_vec(), Limits::DEFAULT).expect("it opens");
    let font = font_of(&held);
    let descriptor = held
        .get_key(&font, "FontDescriptor")
        .as_dict()
        .cloned()
        .expect("the descriptor");
    (
        held.get_key(&descriptor, "Flags")
            .as_integer()
            .expect("flags"),
        font.get("Encoding").is_some(),
    )
}

/// A page showing `AB` in one TrueType font whose dictionary states `first` to `last` widths,
/// `encoding`, and whose descriptor states `flags` and the program `program`.
fn a_page_in_a_truetype_font(
    flags: &str,
    encoding: &str,
    program: &[u8],
    codes: (u8, u8),
) -> Vec<u8> {
    let (first, last) = codes;
    let widths = vec!["667"; usize::from(last.saturating_sub(first)).saturating_add(1)].join(" ");
    Conforming {
        resources: "/Font << /F1 6 0 R >> /ColorSpace << /CS0 [/CalGray << /WhitePoint \
                    [0.9505 1.0 1.089] >>] >>"
            .to_owned(),
        contents: Some((
            String::new(),
            b"/CS0 cs 0 sc BT /F1 12 Tf 10 100 Td (AB) Tj ET".to_vec(),
        )),
        objects: vec![
            format!(
                "<< /Type /Font /Subtype /TrueType /BaseFont /LiberationSans /FirstChar {first} \
                 /LastChar {last} /Widths [{widths}] /FontDescriptor 7 0 R {encoding} >>"
            ),
            format!(
                "<< /Type /FontDescriptor /FontName /LiberationSans /Flags {flags} /FontBBox \
                 [-543 -303 1300 980] /ItalicAngle 0 /Ascent 905 /Descent -212 /CapHeight 716 \
                 /StemV 80 /FontFile2 8 0 R >>"
            ),
        ],
        binary_objects: vec![stream(&format!("/Length {}", program.len()), program)],
        ..Conforming::part_two()
    }
    .build()
}

/// The glyph name the fixtures give the code of `A`: the program's `post` table names a glyph so,
/// the Adobe Glyph List does not, and the reader reaches that glyph by the program's own names,
/// which ISO 19005-2 section 6.2.11.6 withdraws.
const NAME_OUTSIDE_THE_LIST: &str = "nonmarkingreturn";

/// A font whose `Differences` names, for the code of `A`, a glyph outside the Adobe Glyph List.
fn a_font_naming_a_glyph_outside_the_list() -> Vec<u8> {
    a_page_in_a_truetype_font(
        "32",
        &format!(
            "/Encoding << /Type /Encoding /BaseEncoding /WinAnsiEncoding /Differences [65 \
             /{NAME_OUTSIDE_THE_LIST}] >>"
        ),
        LIBERATION_SANS,
        (65, 66),
    )
}

#[test]
fn a_difference_naming_no_glyph_in_the_list_is_restated_in_the_program() {
    let source = a_font_naming_a_glyph_outside_the_list();
    let before = glyphs_of(&source);
    assert!(
        before[65].is_some(),
        "the name reaches a glyph by the program's post table"
    );
    assert!(before[65].is_some());
    let (report, output) = convert(&source, Target::Two(Level::B), Authorisations::default());
    let failed: Vec<&str> = holds(&source, Target::Two(Level::B))
        .failures()
        .map(|failed| failed.id)
        .collect();
    assert!(failed.contains(&DIFFERENCES), "{failed:?}");
    assert!(matches!(
        decision(&report, DIFFERENCES),
        Decision::Stated {
            rewrite: Rewrite::TrueTypeCmapRestated,
            ..
        }
    ));
    let output = output.unwrap_or_else(|| {
        panic!(
            "{:?}\n{:?}\n{:?}",
            conversion(&report).achieved,
            report.refused,
            report.warnings
        )
    });
    let held = holds(&output, Target::Two(Level::B));
    assert_eq!(held.verdict(), Verdict::Conforms, "{}", held.render());
    assert_eq!(
        glyphs_of(&output),
        before,
        "every code reaches the glyph it reached"
    );
    // `doc/questions/A48`'s condition: the file's own history names the clause and the font.
    let packet = String::from_utf8(super::document_packet(&output)).expect("the packet is UTF-8");
    assert!(
        packet.contains("ISO 19005-2 section 6.2.11.6")
            && packet.contains("TrueType font(s) now carries the Macintosh Roman (1, 0)")
            && packet.contains("fonts, in the source's object numbering, 6 0 R"),
        "{packet}"
    );
    let (flags, encoding) = flags_and_encoding(&output);
    assert_eq!(flags & SYMBOLIC, SYMBOLIC, "the font is the symbolic one");
    assert_eq!(flags & NONSYMBOLIC, 0);
    assert!(!encoding, "a symbolic font states no /Encoding");
    assert_eq!(
        cmap_records(&output),
        vec![(1, 0), (3, 0)],
        "the Macintosh Roman and the Microsoft Symbol subtables, in the order the Manual sorts them"
    );
}

#[test]
fn the_content_stream_is_the_producers_bytes() {
    let source = a_font_naming_a_glyph_outside_the_list();
    let (_, output) = convert(
        &source,
        Target::Four(pdf_archive::Flavour::Plain),
        Authorisations::default(),
    );
    let output = output.expect("part 4 converts the same font");
    let text = |bytes: &[u8]| {
        String::from_utf8_lossy(bytes).contains("/CS0 cs 0 sc BT /F1 12 Tf 10 100 Td (AB) Tj ET")
    };
    assert!(
        text(&source) && text(&output),
        "the page's marks are unchanged"
    );
    assert_eq!(glyphs_of(&output), glyphs_of(&source));
}

/// The program of Liberation Sans with a `cmap` of the Macintosh Roman subtable alone, so that
/// it carries no Microsoft Unicode (3, 1) subtable, mapping `A` and `B` as the original does.
fn liberation_without_a_unicode_subtable() -> Vec<u8> {
    let held =
        Document::open_with_limits(a_font_naming_a_glyph_outside_the_list(), Limits::DEFAULT)
            .expect("it opens");
    let font = LoadedFont::load(&held, &font_of(&held), "").expect("loads");
    let glyph = |byte: u8| {
        font.glyph_index(Code::single_byte(byte))
            .expect("the program has the glyph")
    };
    // Format 6: format, length, language, first code, count, then a glyph index each.
    let mut subtable = Vec::new();
    for word in [6u16, 14, 0, 65, 2, glyph(65), glyph(66)] {
        subtable.extend_from_slice(&word.to_be_bytes());
    }
    let mut table = Vec::new();
    for word in [0u16, 1, 1, 0] {
        table.extend_from_slice(&word.to_be_bytes());
    }
    table.extend_from_slice(&12u32.to_be_bytes());
    table.extend_from_slice(&subtable);
    pdf_font::embedding::with_tables(LIBERATION_SANS, &[(*b"cmap", table)])
        .expect("Liberation Sans is an sfnt")
}

#[test]
fn a_program_with_no_unicode_subtable_gets_the_two_subtables_too() {
    let program = liberation_without_a_unicode_subtable();
    let source = a_page_in_a_truetype_font(
        "32",
        "/Encoding << /Type /Encoding /BaseEncoding /WinAnsiEncoding /Differences [66 \
         /nonmarkingreturn] >>",
        &program,
        (65, 66),
    );
    let (report, output) = convert(&source, Target::Two(Level::B), Authorisations::default());
    let failed: Vec<&str> = holds(&source, Target::Two(Level::B))
        .failures()
        .map(|failed| failed.id)
        .collect();
    assert!(
        failed.contains(&NEEDS_UNICODE) || failed.contains(&MAPS_EVERY_CODE),
        "{failed:?}"
    );
    let output = output.unwrap_or_else(|| {
        panic!(
            "{:?}\n{:?}\n{:?}",
            conversion(&report).achieved,
            report.refused,
            report.warnings
        )
    });
    assert_eq!(cmap_records(&source), vec![(1, 0)]);
    assert_eq!(cmap_records(&output), vec![(1, 0), (3, 0)]);
    let held = holds(&output, Target::Two(Level::B));
    assert_eq!(held.verdict(), Verdict::Conforms, "{}", held.render());
    assert_eq!(glyphs_of(&output)[65..=66], glyphs_of(&source)[65..=66]);
}

/// How many places of one requirement `pdf_archive::Findings` keeps before it counts instead.
const MOST_PLACES_THE_CHECKER_KEEPS: usize = 32;

/// `count` fonts of one page, each with a descriptor of its own and every descriptor naming one
/// program, which is the document a hostile producer writes to have the program copied once for
/// each font.
fn fonts_sharing_one_program(count: usize) -> Vec<u8> {
    fonts_sharing(count, LIBERATION_SANS, false)
}

/// [`fonts_sharing_one_program`] with a program of the caller's choosing.
fn fonts_sharing(count: usize, program_bytes: &[u8], null_descendant: bool) -> Vec<u8> {
    let mut objects = Vec::new();
    let mut resources = String::from("/Font << ");
    let mut marks = String::from("/CS0 cs 0 sc BT ");
    // Objects 6 onward: a font and its descriptor for each, then the one program.
    let program = count
        .saturating_mul(2)
        .saturating_add(6)
        .saturating_add(usize::from(null_descendant));
    for index in 0..count {
        let font = index.saturating_mul(2).saturating_add(6);
        write!(resources, "/F{index} {font} 0 R ").expect("a string takes it");
        write!(marks, "/F{index} 12 Tf 10 100 Td (AB) Tj ").expect("a string takes it");
        objects.push(format!(
            "<< /Type /Font /Subtype /TrueType /BaseFont /LiberationSans /FirstChar 65 \
             /LastChar 66 /Widths [667 667] /FontDescriptor {} 0 R /Encoding << /Type /Encoding \
             /BaseEncoding /WinAnsiEncoding /Differences [65 /nonmarkingreturn] >> >>",
            font.saturating_add(1)
        ));
        objects.push(format!(
            "<< /Type /FontDescriptor /FontName /LiberationSans /Flags 32 /FontBBox [-543 -303 \
             1300 980] /ItalicAngle 0 /Ascent 905 /Descent -212 /CapHeight 716 /StemV 80 \
             /FontFile2 {program} 0 R >>"
        ));
    }
    if null_descendant {
        let font = count.saturating_mul(2).saturating_add(6);
        write!(resources, "/FN {font} 0 R ").expect("a string takes it");
        write!(marks, "/FN 12 Tf 10 100 Td (AB) Tj ").expect("a string takes it");
        // A composite font whose descendant is `null` fails a row the table refuses by name
        // (`fonts/descendant-fonts-holds-one-cidfont`), so the file is refused whatever the
        // TrueType fonts say.
        objects.push(
            "<< /Type /Font /Subtype /Type0 /BaseFont /LiberationSans /Encoding /Identity-H \
             /DescendantFonts [null] >>"
                .to_owned(),
        );
    }
    resources.push_str(">> /ColorSpace << /CS0 [/CalGray << /WhitePoint [0.9505 1.0 1.089] >>] >>");
    marks.push_str("ET");
    Conforming {
        resources,
        contents: Some((String::new(), marks.into_bytes())),
        objects,
        binary_objects: vec![stream(
            &format!("/Length {}", program_bytes.len()),
            program_bytes,
        )],
        ..Conforming::part_two()
    }
    .build()
}

#[test]
fn fonts_past_the_places_the_checker_keeps_are_restated_too() {
    // The fonts restated are every object the requirement named, not the places a report keeps
    // (`doc/adr/2602`): one font past those places converts like the rest.
    let target = Target::Two(Level::B);
    for count in [
        MOST_PLACES_THE_CHECKER_KEEPS,
        MOST_PLACES_THE_CHECKER_KEEPS.saturating_add(1),
    ] {
        let source = fonts_sharing_one_program(count);
        let (report, output) = convert(&source, target, Authorisations::default());
        let output =
            output.unwrap_or_else(|| panic!("{count}: {:?}", conversion(&report).achieved));
        let held = holds(&output, target);
        assert_eq!(
            held.verdict(),
            Verdict::Conforms,
            "{count}: {}",
            held.render()
        );
    }
}

#[test]
fn programs_past_the_byte_budget_are_refused_with_their_own_sentence() {
    // The same Liberation program followed by 1.2 MiB of zeros (trailing bytes no table names),
    // shared by every font: the count of fonts is far under the checker's 32 places, and the
    // programs a conversion would copy are 1.2 MiB each, so 30 of them pass the 32 MiB budget
    // and 20 do not (`MAX_COPIED_BYTES`, ADR 2520 section 5).
    let mut padded = LIBERATION_SANS.to_vec();
    padded.resize(padded.len().saturating_add(1_258_291), 0);
    let target = Target::Two(Level::B);
    let within = fonts_sharing(20, &padded, false);
    let (report, output) = convert(&within, target, Authorisations::default());
    let output = output.unwrap_or_else(|| panic!("{:?}", conversion(&report).achieved));
    let held = holds(&output, target);
    assert_eq!(held.verdict(), Verdict::Conforms, "{}", held.render());

    let past = fonts_sharing(30, &padded, false);
    let (report, output) = convert(&past, target, Authorisations::default());
    let Decision::Refused(because) = decision(&report, DIFFERENCES) else {
        panic!("programs past the budget keep the refusal");
    };
    assert!(
        because.sentence().contains("32 MiB of font program"),
        "{because:?}"
    );
    assert!(output.is_none());
}

#[test]
fn the_restatement_is_not_made_for_a_file_another_row_refuses() {
    // A file failing a row the table refuses by name is refused whatever its fonts say, and
    // copying every font's program first would be the conversion's largest cost spent on a
    // refusal. The control is the same fonts without the failing one.
    let target = Target::Two(Level::B);
    let control = fonts_sharing(4, LIBERATION_SANS, false);
    let (report, _) = convert(&control, target, Authorisations::default());
    assert!(matches!(
        decision(&report, DIFFERENCES),
        Decision::Stated { .. }
    ));
    let refused_anyway = fonts_sharing(4, LIBERATION_SANS, true);
    let (report, output) = convert(&refused_anyway, target, Authorisations::default());
    let Decision::Refused(because) = decision(&report, DIFFERENCES) else {
        panic!("the font row is refused with the document");
    };
    assert!(
        because.sentence().contains("no program was copied"),
        "{because:?}"
    );
    assert!(output.is_none());
}

/// The row's whole case, in one place: its decision is the restatement, the output conforms, and
/// every code reaches the glyph it reached.
fn restated_and_unmoved(source: &[u8], row: &str) {
    let before = glyphs_of(source);
    let (report, output) = convert(source, Target::Two(Level::B), Authorisations::default());
    let decided = decision(&report, row);
    assert!(
        matches!(
            decided,
            Decision::Stated {
                rewrite: Rewrite::TrueTypeCmapRestated,
                ..
            }
        ),
        "{decided:?}"
    );
    let output = output.unwrap_or_else(|| panic!("{:?}\n{:?}", report.refused, report.warnings));
    let held = holds(&output, Target::Two(Level::B));
    assert_eq!(held.verdict(), Verdict::Conforms, "{}", held.render());
    for (code, (after, before)) in glyphs_of(&output).iter().zip(&before).enumerate() {
        // A code on glyph 0 is left out of the table and held only to not reaching another glyph.
        if before.is_some_and(|glyph| glyph != 0) {
            assert_eq!(after, before, "code {code} keeps its glyph");
        } else {
            assert!(
                after.is_none() || *after == Some(0),
                "code {code} reached no glyph and reaches none now: {after:?}"
            );
        }
    }
}

#[test]
fn a_code_whose_standard_name_would_move_it_is_restated_where_the_narrow_rewrite_refuses() {
    // Code 0xE9 is `eacute` under `WinAnsiEncoding` and `Odieresis` under `StandardEncoding`, so
    // writing either admitted name over this file's encoding would move the mark and the narrow
    // rewrite refuses; the restated `cmap` states the glyph this reader drew instead
    // (`Answer::OrStated`).
    let source = super::a_truetype_font("32", "/Encoding /StandardEncoding", Some("\\351"));
    restated_and_unmoved(
        &source,
        "fonts/non-symbolic-truetype-uses-a-standard-encoding",
    );
}

#[test]
fn a_code_the_file_names_no_character_for_is_restated_where_the_names_cannot_be_written() {
    // With no `/ToUnicode` the file does not say which character code 0 is, so no name can be
    // written for it and the standard-route row's own rewrite refuses; the restated `cmap` needs
    // no name.
    let source = super::a_truetype_code_no_step_reaches(false);
    restated_and_unmoved(
        &source,
        "fonts/truetype-codes-reach-glyphs-by-the-standard-route",
    );
}

/// A page showing `text` in one TrueType font the file does not embed: `/BaseFont` `base`,
/// descriptor flags `flags`, `encoding`, and the one width `width` for every code from `first`.
fn a_page_in_an_unembedded_truetype_font(
    (base, flags, encoding): (&str, &str, &str),
    (first, last, width): (u8, u8, &str),
    text: &str,
) -> Vec<u8> {
    let widths = vec![width; usize::from(last.saturating_sub(first)).saturating_add(1)].join(" ");
    Conforming {
        resources: "/Font << /F1 6 0 R >> /ColorSpace << /CS0 [/CalGray << /WhitePoint \
                    [0.9505 1.0 1.089] >>] >>"
            .to_owned(),
        contents: Some((
            String::new(),
            format!("/CS0 cs 0 sc BT /F1 12 Tf 10 100 Td ({text}) Tj ET").into_bytes(),
        )),
        objects: vec![
            format!(
                "<< /Type /Font /Subtype /TrueType /BaseFont /{base} /FirstChar {first} \
                 /LastChar {last} /Widths [{widths}] /FontDescriptor 7 0 R {encoding} >>"
            ),
            format!(
                "<< /Type /FontDescriptor /FontName /{base} /Flags {flags} /FontBBox \
                 [-543 -303 1300 980] /ItalicAngle 0 /Ascent 905 /Descent -212 /CapHeight 716 \
                 /StemV 80 >>"
            ),
        ],
        ..Conforming::part_two()
    }
    .build()
}

/// The losses embedding a face this program ships may incur, authorised as `as-if-printed`
/// authorises them.
fn faces_authorised() -> Authorisations {
    let mut authorised = Authorisations::default();
    authorised.authorise(Loss::MachineFace);
    authorised.authorise(Loss::SubstituteStretch);
    authorised
}

#[test]
fn a_font_the_file_did_not_embed_is_restated_over_the_face_the_conversion_embeds() {
    // ISO 32000-2 §9.6.5.4: a font whose Nonsymbolic flag is set reads a code through a table of
    // glyph names, and with no `/Encoding` entry code 0o361 is the base encoding's `ae`. The
    // conversion embeds the face this program ships for `Arial`, and ISO 19005-2 section
    // 6.2.11.6 then holds that program to the encoding rows the source failed: a non-symbolic
    // TrueType font states `MacRomanEncoding` or `WinAnsiEncoding`, and neither names `ae` at
    // 0o361. The restatement makes the font symbolic, its program's (1, 0) and (3, 0) subtables
    // sending the code to the glyph the shipped face's own code table gives it
    // (`doc/adr/2744`).
    let source =
        a_page_in_an_unembedded_truetype_font(("Arial", "32", ""), (241, 241, "889"), "\\361");
    let held = Document::open_with_limits(source.clone(), Limits::DEFAULT).expect("it opens");
    let shipped = pdf_font::standard::shipped_face(&held, &font_of(&held), "")
        .expect("this program ships a face for Arial");
    let drawn = shipped.glyph(241).expect("the face has the code's glyph");
    let row = "fonts/non-symbolic-truetype-uses-a-standard-encoding";
    let (report, output) = convert(&source, Target::Two(Level::B), faces_authorised());
    let decided = decision(&report, row);
    assert!(
        matches!(
            decided,
            Decision::Stated {
                rewrite: Rewrite::TrueTypeCmapRestated,
                ..
            }
        ),
        "{decided:?}"
    );
    let output = output.unwrap_or_else(|| panic!("{:?}\n{:?}", report.refused, report.warnings));
    let checked = holds(&output, Target::Two(Level::B));
    assert_eq!(checked.verdict(), Verdict::Conforms, "{}", checked.render());
    assert!(conversion(&report).photographed.is_empty());
    let (flags, encoding) = flags_and_encoding(&output);
    assert_eq!(flags & (SYMBOLIC | NONSYMBOLIC), SYMBOLIC);
    assert!(!encoding);
    assert_eq!(cmap_records(&output), vec![(1, 0), (3, 0)]);
    let glyphs = glyphs_of(&output);
    assert_eq!(
        glyphs[241],
        Some(drawn),
        "the code reaches the face's glyph"
    );
}

/// The same restatement over the face with the viewer's fitting written into it, which is what
/// a profile withholding `substitute-stretch` embeds (`doc/adr/2769`): the proof holds each glyph
/// of the embedded program to the shipped face's glyph scaled by the same writer, and the code
/// still reaches the face's glyph.
#[test]
fn a_font_the_file_did_not_embed_is_restated_over_the_face_carrying_its_fitting() {
    let source =
        a_page_in_an_unembedded_truetype_font(("Arial", "32", ""), (241, 241, "889"), "\\361");
    let held = Document::open_with_limits(source.clone(), Limits::DEFAULT).expect("it opens");
    let shipped = pdf_font::standard::shipped_face(&held, &font_of(&held), "")
        .expect("this program ships a face for Arial");
    let drawn = shipped.glyph(241).expect("the face has the code's glyph");
    let mut authorised = Authorisations::default();
    authorised.authorise(Loss::MachineFace);
    let (report, output) = convert(&source, Target::Two(Level::B), authorised);
    let decided = decision(
        &report,
        "fonts/non-symbolic-truetype-uses-a-standard-encoding",
    );
    assert!(
        matches!(
            decided,
            Decision::Stated {
                rewrite: Rewrite::TrueTypeCmapRestated,
                ..
            }
        ),
        "{decided:?}"
    );
    let output = output.unwrap_or_else(|| panic!("{:?}\n{:?}", report.refused, report.warnings));
    let checked = holds(&output, Target::Two(Level::B));
    assert_eq!(checked.verdict(), Verdict::Conforms, "{}", checked.render());
    assert!(matches!(
        conversion(&report)
            .substituted
            .first()
            .and_then(|font| font.fitting),
        Some(pdf_transform::archive::Fitting::Written(_))
    ));
    assert_eq!(glyphs_of(&output)[241], Some(drawn));
}

#[test]
fn a_font_the_substitution_makes_a_type1_one_has_no_truetype_route_left_to_state() {
    // The face this program ships for `TimesNewRoman` is a CFF one, so the substitution writes
    // the dictionary as a `Type1` one (ISO 32000-2 §9.9's Table 124 files the program under
    // `/FontFile3`), and §9.6.5.4's TrueType route is no longer how any code is read. The
    // encoding row the source failed is answered with no program copied for the font
    // (`doc/adr/2744`); the control is that the source's font is a TrueType one.
    let source =
        a_page_in_an_unembedded_truetype_font(("TimesNewRoman", "32", ""), (65, 66, "667"), "AB");
    let held = Document::open_with_limits(source.clone(), Limits::DEFAULT).expect("it opens");
    assert_eq!(
        held.get_key(&font_of(&held), "Subtype")
            .as_name()
            .map(|name| name.as_bytes().to_vec()),
        Some(b"TrueType".to_vec())
    );
    let row = "fonts/non-symbolic-truetype-uses-a-standard-encoding";
    let (report, output) = convert(&source, Target::Two(Level::B), faces_authorised());
    let decided = decision(&report, row);
    assert!(
        matches!(
            decided,
            Decision::Stated {
                rewrite: Rewrite::TrueTypeCmapRestated,
                ..
            }
        ),
        "{decided:?}"
    );
    let output = output.unwrap_or_else(|| panic!("{:?}\n{:?}", report.refused, report.warnings));
    let checked = holds(&output, Target::Two(Level::B));
    assert_eq!(checked.verdict(), Verdict::Conforms, "{}", checked.render());
    let written = Document::open_with_limits(output, Limits::DEFAULT).expect("it opens");
    let font = font_of(&written);
    assert_eq!(
        written
            .get_key(&font, "Subtype")
            .as_name()
            .map(|name| name.as_bytes().to_vec()),
        Some(b"Type1".to_vec())
    );
    let descriptor = written
        .get_key(&font, "FontDescriptor")
        .as_dict()
        .cloned()
        .expect("the descriptor");
    assert!(descriptor.get("FontFile3").is_some() && descriptor.get("FontFile2").is_none());
}
