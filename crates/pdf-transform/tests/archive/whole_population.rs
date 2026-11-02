//! A remedy is handed the whole population a requirement failed over, not the places a report
//! keeps (`doc/adr/2602`).
//!
//! `pdf_archive`'s findings keep the first thirty-two places of a requirement and count the rest,
//! which is what a report should hold; a converter site that read that list as the set to rewrite
//! gave thirty-two pages their resources and left the thirty-third failing the requirement it
//! answered. Each shape here is past the list's bound and is converted at both targets, the output
//! held to the target again by `pdf_archive`.

use std::fmt::Write as _;

use pdf_font::{Code, LoadedFont};
use pdf_syntax::object::{Object, ObjectId};
use pdf_syntax::{Document, Limits};
use pdf_transform::profiles::{Contribution, named};

use super::{
    Authorisations, Because, Conforming, Decision, LIBERATION_SANS, Level, Rewrite, Target,
    Verdict, convert, decision, holds, stream, written,
};

const EXPLICIT: &str = "graphics/content-streams-have-an-explicit-resources-dictionary";
const NOTDEF_SHOWN: &str = "fonts/no-notdef-glyph-shown";
const TARGETS: [Target; 2] = [
    Target::Two(Level::B),
    Target::Four(pdf_archive::Flavour::Plain),
];

/// What the shipped profile `profile` authorises at `target`.
fn authorised_by(profile: &str, target: Target) -> Authorisations {
    Contribution::read(named(profile).expect("a shipped profile").text, target)
        .expect("a shipped profile reads")
        .authorised
}

/// A file of `objects`, numbered from 1, each its bytes, with a classic cross-reference table.
fn file<T: AsRef<[u8]>>(objects: &[T]) -> Vec<u8> {
    let mut bytes = b"%PDF-1.7\n%\xe2\xe3\xcf\xd3\n".to_vec();
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(bytes.len());
        bytes.extend_from_slice(format!("{} 0 obj\n", index.saturating_add(1)).as_bytes());
        bytes.extend_from_slice(object.as_ref());
        bytes.extend_from_slice(b"\nendobj\n");
    }
    let xref = bytes.len();
    let size = objects.len().saturating_add(1);
    bytes.extend_from_slice(format!("xref\n0 {size}\n0000000000 65535 f \n").as_bytes());
    for offset in offsets {
        bytes.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    bytes.extend_from_slice(
        format!("trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes(),
    );
    bytes
}

/// `pages` pages that state no `/Resources` and draw one content stream selecting `/G0`, under a
/// root whose `/Resources` is object 3 where `by_reference` and written directly otherwise.
fn pages_inheriting(pages: usize, by_reference: bool) -> Vec<u8> {
    pages_inheriting_with(pages, by_reference, 0)
}

/// [`pages_inheriting`], the dictionary holding `extra` further graphics states besides `/G0`.
fn pages_inheriting_with(pages: usize, by_reference: bool, extra: usize) -> Vec<u8> {
    let states = (0..extra).fold(String::new(), |mut states, index| {
        let _ = write!(states, "/K{index} 4 0 R ");
        states
    });
    let resources = format!(
        "<< /ExtGState << /G0 4 0 R {states}>> /ColorSpace << /CS0 [/CalGray << /WhitePoint \
         [0.9505 1.0 1.089] >>] >> >>"
    );
    let content = b"/G0 gs /CS0 cs 0 sc 0 0 10 10 re f";
    let mut kids = String::new();
    for index in 0..pages {
        let _ = write!(kids, "{} 0 R ", index.saturating_add(6));
    }
    let inherited = if by_reference { "3 0 R" } else { &resources };
    let mut objects = vec![
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        format!("<< /Type /Pages /Kids [{kids}] /Count {pages} /Resources {inherited} >>"),
        resources.clone(),
        "<< /Type /ExtGState /CA 1 >>".to_owned(),
        String::from_utf8_lossy(&stream(&format!("/Length {}", content.len()), content))
            .into_owned(),
    ];
    objects.extend((0..pages).map(|_| {
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Contents 5 0 R >>".to_owned()
    }));
    file(&objects)
}

#[test]
fn a_thousand_pages_inheriting_one_dictionary_are_each_given_it() {
    let pages = 1_000;
    for by_reference in [true, false] {
        let source = pages_inheriting(pages, by_reference);
        let failed = holds(&source, TARGETS[0]);
        let total = failed
            .failures()
            .find(|judgement| judgement.id == EXPLICIT)
            .map(|judgement| match &judgement.outcome {
                pdf_archive::Outcome::Failed { places, total } => (places.len(), *total),
                _ => (0, 0),
            });
        assert!(
            total.is_some_and(|(kept, total)| total == pages && kept < total),
            "the report keeps a prefix of the {pages} pages: {total:?}"
        );
        for target in TARGETS {
            let (report, output) = convert(&source, target, authorised_by("as-if-printed", target));
            let output = output.unwrap_or_else(|| {
                panic!(
                    "{target}, by reference {by_reference}: {:?}\n{:?}",
                    report.refused, report.warnings
                )
            });
            let held = holds(&output, target);
            assert_eq!(held.verdict(), Verdict::Conforms, "{}", held.render());
            let document =
                Document::open_with_limits(output.clone(), Limits::DEFAULT).expect("it opens");
            let tree = pdf_model::Pages::new(&document);
            let every: Vec<_> = tree.every().into_iter().flatten().collect();
            assert_eq!(
                every.len(),
                pages,
                "every page the source states is written"
            );
            for page in &every {
                let resources = page.dict.get("Resources").expect("a page states its own");
                if by_reference {
                    assert!(
                        matches!(resources, Object::Reference(_)),
                        "the one dictionary is named, not copied"
                    );
                }
            }
        }
    }
}

#[test]
fn copies_of_a_direct_dictionary_stop_at_the_budget_and_say_so() {
    // Four thousand graphics states make one copy weigh about 220 KB against the 32 MiB budget
    // (`VALUE_WEIGHT` and the names in `sites`): 140 pages are 31 MB of copies, 170 are 37.
    let target = TARGETS[0];
    let under = pages_inheriting_with(140, false, 4_000);
    let (report, output) = convert(&under, target, authorised_by("as-if-printed", target));
    let output = written(&report, output);
    assert_eq!(holds(&output, target).verdict(), Verdict::Conforms);
    let over = pages_inheriting_with(170, false, 4_000);
    let (report, output) = convert(&over, target, authorised_by("as-if-printed", target));
    assert!(output.is_none(), "past the budget nothing is written");
    let Decision::Refused(Because::NotBuiltYet(because)) = decision(&report, EXPLICIT) else {
        panic!("{:?}", decision(&report, EXPLICIT));
    };
    assert!(because.contains("32 MiB"), "{because}");
}

/// The codes from 1 to 31 that reach glyph 0 of the Liberation Sans program once `/Differences`
/// names a character it has no glyph for: 8, 9, 13 and 29 reach a glyph of their own through the
/// program's (1, 0) subtable, which the reader also consults.
const ON_NOTDEF: [u8; 27] = [
    1, 2, 3, 4, 5, 6, 7, 10, 11, 12, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28,
    30, 31,
];

/// A page showing every code of [`ON_NOTDEF`] in each of two TrueType fonts, objects 6 and 7,
/// each under a descriptor of its own over one program, whose `/Differences` names, for each code, a character the program has no
/// glyph for, so that every one reaches glyph 0: fifty-four places of one requirement.
fn a_page_showing_notdef_in_two_fonts() -> Vec<u8> {
    let (names, shown) = ON_NOTDEF.iter().fold(
        (String::new(), String::new()),
        |(mut names, mut shown), code| {
            let _ = write!(names, "{code} /uni4E{code:02X} ");
            let _ = write!(shown, "\\{code:03o}");
            (names, shown)
        },
    );
    let widths = vec!["667"; 66].join(" ");
    let font = |descriptor: u32| {
        format!(
            "<< /Type /Font /Subtype /TrueType /BaseFont /LiberationSans /FirstChar 1 \
             /LastChar 66 /Widths [{widths}] /FontDescriptor {descriptor} 0 R /Encoding << /Type \
             /Encoding /BaseEncoding /WinAnsiEncoding /Differences [{names}] >> >>"
        )
    };
    let descriptor = "<< /Type /FontDescriptor /FontName /LiberationSans /Flags 32 /FontBBox \
                      [-543 -303 1300 980] /ItalicAngle 0 /Ascent 905 /Descent -212 /CapHeight \
                      716 /StemV 80 /FontFile2 10 0 R >>";
    Conforming {
        resources: "/Font << /F1 6 0 R /F2 7 0 R >> /ColorSpace << /CS0 [/CalGray << \
                    /WhitePoint [0.9505 1.0 1.089] >>] >>"
            .to_owned(),
        contents: Some((
            String::new(),
            format!(
                "/CS0 cs 0 sc BT /F1 12 Tf 10 100 Td ({shown}AB) Tj /F2 12 Tf 0 20 Td ({shown}AB) \
                 Tj ET"
            )
            .into_bytes(),
        )),
        objects: vec![
            font(8),
            font(9),
            descriptor.to_owned(),
            descriptor.to_owned(),
        ],
        binary_objects: vec![stream(
            &format!("/Length {}", LIBERATION_SANS.len()),
            LIBERATION_SANS,
        )],
        ..Conforming::part_two()
    }
    .build()
}

/// The glyph each code of [`ON_NOTDEF`] reaches in the font object `font` of `bytes`.
fn glyphs_of(bytes: &[u8], font: u32) -> Vec<Option<u16>> {
    let held = Document::open_with_limits(bytes.to_vec(), Limits::DEFAULT).expect("it opens");
    let dict = held
        .get(ObjectId::new(font, 0))
        .as_dict()
        .cloned()
        .expect("the font object");
    let font = LoadedFont::load(&held, &dict, "").expect("the font loads");
    ON_NOTDEF
        .iter()
        .map(|byte| font.glyph_index(Code::single_byte(*byte)))
        .collect()
}

#[test]
fn codes_on_notdef_past_the_report_s_bound_are_each_given_the_copy() {
    let source = a_page_showing_notdef_in_two_fonts();
    let before = [glyphs_of(&source, 6), glyphs_of(&source, 7)].concat();
    assert!(
        before.iter().all(|glyph| *glyph == Some(0)),
        "every code reaches .notdef in the source: {before:?}"
    );
    let failed = holds(&source, TARGETS[0]);
    let judgement = failed
        .failures()
        .find(|judgement| judgement.id == NOTDEF_SHOWN)
        .expect("the row fails");
    assert!(
        matches!(&judgement.outcome,
            pdf_archive::Outcome::Failed { places, total }
                if places.len() < *total),
        "the report keeps a prefix of the codes shown: {:?}",
        judgement.outcome
    );
    for target in TARGETS {
        let (report, output) = convert(&source, target, Authorisations::default());
        assert!(
            matches!(
                decision(&report, NOTDEF_SHOWN),
                Decision::Stated {
                    rewrite: Rewrite::NotdefGlyphGiven,
                    ..
                }
            ),
            "{target}: {:?}",
            decision(&report, NOTDEF_SHOWN)
        );
        let output = written(&report, output);
        let held = holds(&output, target);
        assert_eq!(held.verdict(), Verdict::Conforms, "{}", held.render());
        for font in [6, 7] {
            let after = glyphs_of(&output, font);
            let given = after[0].expect("code 1 reaches a glyph");
            assert_ne!(given, 0, "no code reaches .notdef");
            assert!(
                after.iter().all(|glyph| *glyph == Some(given)),
                "{target}, font {font}: every code shares the one copy: {after:?}"
            );
        }
    }
}

const BOUNDARIES: &str = "implementation-limits/page-boundary-sizes";
const EXTERNAL: &str = "file-structure/no-external-stream-data";
const WIDTHS: &str = "fonts/widths-array-covers-character-range";
const TO_UNICODE: &str = "fonts/to-unicode-present";

/// Past the thirty-two places a report keeps, so that a site reading those places as the
/// population leaves eight failing.
const PAST_THE_KEPT: usize = 40;

/// How many places `requirement`'s failure keeps and how many it counts, where `bytes` fail it.
fn failed_at(bytes: &[u8], target: Target, requirement: &str) -> Option<(usize, usize)> {
    holds(bytes, target)
        .failures()
        .find(|judgement| judgement.id == requirement)
        .and_then(|judgement| match &judgement.outcome {
            pdf_archive::Outcome::Failed { places, total } => Some((places.len(), *total)),
            _ => None,
        })
}

/// Asserts that the report holds a prefix of `requirement`'s `population` findings.
fn the_report_keeps_a_prefix(bytes: &[u8], target: Target, requirement: &str, population: usize) {
    let failed = failed_at(bytes, target, requirement);
    assert!(
        failed.is_some_and(|(kept, total)| total == population && kept < total),
        "{target}: the report keeps a prefix of the {population} places of {requirement}: \
         {failed:?}"
    );
}

/// `source` converted at `target` under the shipped `as-if-printed` profile, held to the target
/// again and conforming.
fn converted_whole(source: &[u8], target: Target) -> Vec<u8> {
    let (report, output) = convert(source, target, authorised_by("as-if-printed", target));
    let output = written(&report, output);
    let held = holds(&output, target);
    assert_eq!(
        held.verdict(),
        Verdict::Conforms,
        "{target}: {}",
        held.render()
    );
    output
}

/// `pages` pages, each stating a crop box of 14 402 units over a media box of 200: section
/// 6.1.13's limit fails at every page, and §14.11.2.1 makes the entry the media box to a reader.
fn pages_stating_crop_boxes(pages: usize) -> Vec<u8> {
    let kids = (0..pages).fold(String::new(), |mut kids, index| {
        let _ = write!(kids, "{} 0 R ", index.saturating_add(3));
        kids
    });
    let mut objects = vec![
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        format!("<< /Type /Pages /Kids [{kids}] /Count {pages} >>"),
    ];
    objects.extend((0..pages).map(|_| {
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /CropBox [0 0 14402 14402] \
         /Resources << >> >>"
            .to_owned()
    }));
    file(&objects)
}

#[test]
fn forty_pages_stating_an_over_sized_crop_box_each_lose_it() {
    let source = pages_stating_crop_boxes(PAST_THE_KEPT);
    // One finding for each of a box's two dimensions, both past the limit.
    the_report_keeps_a_prefix(
        &source,
        TARGETS[0],
        BOUNDARIES,
        PAST_THE_KEPT.saturating_mul(2),
    );
    for target in TARGETS {
        let failed = failed_at(&source, target, BOUNDARIES).is_some();
        let output = converted_whole(&source, target);
        let document = Document::open_with_limits(output, Limits::DEFAULT).expect("it opens");
        let tree = pdf_model::Pages::new(&document);
        let every: Vec<_> = tree.every().into_iter().flatten().collect();
        assert_eq!(every.len(), PAST_THE_KEPT, "every page is written");
        if failed {
            assert!(
                every.iter().all(|page| page.dict.get("CropBox").is_none()),
                "{target}: every page's entry is gone, the thirty-third's too"
            );
        }
    }
}

/// One page whose resources name `forms` form `XObject`s, each stating `/FFilter` and no `/F`:
/// Table 5's key describes the filters of an external file that is not there.
fn forms_naming_filters_for_no_file(forms: usize) -> Vec<u8> {
    let names = (0..forms).fold(String::new(), |mut names, index| {
        let _ = write!(names, "/Fm{index} {} 0 R ", index.saturating_add(4));
        names
    });
    let data = b"% the producer's\n";
    let mut objects = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Resources << /XObject \
             << {names}>> >> >>"
        )
        .into_bytes(),
    ];
    objects.extend((0..forms).map(|_| {
        stream(
            &format!(
                "/Type /XObject /Subtype /Form /BBox [0 0 100 100] /Resources << >> \
                 /FFilter /FlateDecode /Length {}",
                data.len()
            ),
            data,
        )
    }));
    file(&objects)
}

#[test]
fn forty_streams_naming_filters_for_no_file_each_lose_the_key() {
    let source = forms_naming_filters_for_no_file(PAST_THE_KEPT);
    for target in TARGETS {
        the_report_keeps_a_prefix(&source, target, EXTERNAL, PAST_THE_KEPT);
        let (report, _) = convert(&source, target, authorised_by("as-if-printed", target));
        assert_eq!(
            decision(&report, EXTERNAL),
            Decision::Mechanical(Rewrite::ExternalDataEmbedded),
            "{target}"
        );
        let output = converted_whole(&source, target);
        let document = Document::open_with_limits(output, Limits::DEFAULT).expect("it opens");
        let forms = forms_of_page_one(&document);
        assert_eq!(
            forms.len(),
            PAST_THE_KEPT,
            "{target}: every form is written"
        );
        assert!(
            forms.iter().all(|form| form.get("FFilter").is_none()),
            "{target}: no stream keeps the key, the thirty-third's neither"
        );
    }
}

/// The dictionaries of every form page one's resources name.
fn forms_of_page_one(document: &Document) -> Vec<pdf_syntax::object::Dictionary> {
    named_on_page_one(document, "XObject")
        .into_iter()
        .filter_map(|object| object.as_stream().map(|stream| stream.dict.clone()))
        .collect()
}

/// The fonts page one's resources name.
fn fonts_of_page_one(document: &Document) -> Vec<pdf_syntax::object::Dictionary> {
    named_on_page_one(document, "Font")
        .into_iter()
        .filter_map(|object| object.as_dict().cloned())
        .collect()
}

/// Every object page one's resources name under `category`, resolved.
fn named_on_page_one(document: &Document, category: &str) -> Vec<Object> {
    let tree = pdf_model::Pages::new(document);
    let every: Vec<_> = tree.every().into_iter().flatten().collect();
    let page = every.first().expect("a page");
    let resources = document.get_key(&page.dict, "Resources");
    let resources = resources.as_dict().expect("page one states its resources");
    let named = document.get_key(resources, category);
    named
        .as_dict()
        .map(|named| {
            named
                .iter()
                .map(|(_, value)| document.resolve(value))
                .collect()
        })
        .unwrap_or_default()
}

/// A simple `TrueType` font dictionary over the embedded Liberation Sans of
/// [`a_page_drawing_in_fonts`], stating `/FirstChar 65 /LastChar 65` and `entries`.
fn truetype(entries: &str) -> String {
    format!(
        "<< /Type /Font /Subtype /TrueType /BaseFont /LiberationSans /FirstChar 65 /LastChar 65 \
         /FontDescriptor 5 0 R /Encoding /WinAnsiEncoding {entries} >>"
    )
}

/// One page drawing code 65 in each of `fonts` fonts, every one the dictionary `font`; objects 5
/// and 6 are a descriptor and an embedded Liberation Sans a font may name, and object 7 is
/// `seventh`.
fn a_page_drawing_in_fonts(fonts: usize, font: &str, seventh: &[u8]) -> Vec<u8> {
    let first_font = 8_usize;
    let (named, shown) = (0..fonts).fold(
        (String::new(), String::new()),
        |(mut named, mut shown), index| {
            let _ = write!(named, "/F{index} {} 0 R ", index.saturating_add(first_font));
            let _ = write!(shown, "/F{index} 12 Tf (A) Tj ");
            (named, shown)
        },
    );
    let content = format!("/CS0 cs 0 sc BT 10 100 Td {shown}ET");
    let mut objects = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Contents 4 0 R /Resources \
             << /Font << {named}>> /ColorSpace << /CS0 [/CalGray << /WhitePoint [0.9505 1.0 \
             1.089] >>] >> >> >>"
        )
        .into_bytes(),
        stream(&format!("/Length {}", content.len()), content.as_bytes()),
        b"<< /Type /FontDescriptor /FontName /LiberationSans /Flags 32 /FontBBox [-543 -303 1300 \
          980] /ItalicAngle 0 /Ascent 905 /Descent -212 /CapHeight 716 /StemV 80 /FontFile2 \
          6 0 R >>"
            .to_vec(),
        stream(
            &format!("/Length {}", LIBERATION_SANS.len()),
            LIBERATION_SANS,
        ),
        seventh.to_vec(),
    ];
    objects.extend((0..fonts).map(|_| font.as_bytes().to_vec()));
    file(&objects)
}

#[test]
fn forty_fonts_stating_a_widths_array_too_long_are_each_restated() {
    // Table 109: one entry for each code from `FirstChar` to `LastChar`, so `[667 600]` over the
    // one code 65 states an entry past the range, and narrowing it moves no advance.
    let source = a_page_drawing_in_fonts(PAST_THE_KEPT, &truetype("/Widths [667 600]"), b"null");
    for target in TARGETS {
        the_report_keeps_a_prefix(&source, target, WIDTHS, PAST_THE_KEPT);
        let output = converted_whole(&source, target);
        let document = Document::open_with_limits(output, Limits::DEFAULT).expect("it opens");
        let fonts = fonts_of_page_one(&document);
        assert_eq!(
            fonts.len(),
            PAST_THE_KEPT,
            "{target}: every font is written"
        );
        for font in &fonts {
            let widths = document.get_key(font, "Widths");
            let widths = widths.as_array().expect("a /Widths array");
            assert_eq!(
                widths.len(),
                1,
                "{target}: every font's array is restated, the thirty-third's too"
            );
        }
    }
}

/// One `/Widths` array of `entries` entries, the first 667, as object 7's bytes.
fn a_long_widths_array(entries: usize) -> Vec<u8> {
    let mut array = b"[667".to_vec();
    for _ in 1..entries {
        array.extend_from_slice(b" 0");
    }
    array.push(b']');
    array
}

#[test]
fn fonts_sharing_one_long_widths_array_are_read_to_a_mebi_entry_and_no_further() {
    // `font_widths::MAX_READ_WIDTHS` is 4096 fonts of 256 entries, 1 048 576 entries read: two
    // fonts naming one array of half that are read to the bound exactly, a third font is past it.
    let half = 524_288;
    let target = TARGETS[0];
    let at_the_bound =
        a_page_drawing_in_fonts(2, &truetype("/Widths 7 0 R"), &a_long_widths_array(half));
    converted_whole(&at_the_bound, target);
    let past = a_page_drawing_in_fonts(3, &truetype("/Widths 7 0 R"), &a_long_widths_array(half));
    let (report, output) = convert(&past, target, authorised_by("as-if-printed", target));
    assert!(output.is_none(), "past the bound nothing is written");
    let Decision::Refused(Because::NotBuiltYet(because)) = decision(&report, WIDTHS) else {
        panic!("{:?}", decision(&report, WIDTHS));
    };
    assert!(because.contains("1 048 576"), "{because}");
}

#[test]
fn forty_fonts_naming_no_characters_are_each_given_a_to_unicode_map() {
    // ISO 19005-2 section 6.2.11.7.2 binds level U (part 4 states it as a `should`). A Type 3
    // font drawing `uni0041` is outside its second exemption, the name being in neither list, and
    // §9.10.2's second method derives U+0041 from the name by the Adobe Glyph List
    // Specification's algorithm.
    let glyph = b"1000 0 0 0 1000 1000 d1 0 0 1000 1000 re f";
    let charproc = stream(&format!("/Length {}", glyph.len()), glyph);
    let font = "<< /Type /Font /Subtype /Type3 /FontBBox [0 0 1000 1000] /FontMatrix [0.001 0 0 \
                0.001 0 0] /CharProcs << /uni0041 7 0 R >> /Encoding << /Type /Encoding \
                /Differences [65 /uni0041] >> /FirstChar 65 /LastChar 65 /Widths [1000] \
                /Resources << >> >>";
    let source = a_page_drawing_in_fonts(PAST_THE_KEPT, font, &charproc);
    let target = Target::Two(Level::U);
    the_report_keeps_a_prefix(&source, target, TO_UNICODE, PAST_THE_KEPT);
    let (report, _) = convert(&source, target, authorised_by("as-if-printed", target));
    assert_eq!(
        decision(&report, TO_UNICODE).rewrite(),
        Some(Rewrite::ToUnicode),
        "{target}"
    );
    let output = converted_whole(&source, target);
    let document = Document::open_with_limits(output, Limits::DEFAULT).expect("it opens");
    let fonts = fonts_of_page_one(&document);
    assert_eq!(
        fonts.len(),
        PAST_THE_KEPT,
        "{target}: every font is written"
    );
    assert!(
        fonts.iter().all(|font| font.get("ToUnicode").is_some()),
        "{target}: every font states a map, the thirty-third too"
    );
}
