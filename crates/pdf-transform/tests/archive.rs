//! `archive`: the PDF/A converter, held to the requirements it claims to meet.
//!
//! # The fixture is built here, and that is the point
//!
//! Every test below starts from [`Conforming`], a five-object document this file constructs and
//! `pdf_archive::check` finds conforming to PDF/A-4. **Nothing about it is read off a corpus**:
//! its header, its cross-reference section, its XMP identification packet and its one empty page
//! are written from the clauses, so a test's expected value is derivable from the standard
//! rather than from what some other producer happened to emit. `crates/pdf-archive/tests/corpus.rs`
//! is where the veraPDF corpus is read, and it is read there as a population of witnesses.
//!
//! One violation is then added at a time, so that each test asks one question: does the
//! converter fix *this* requirement, does it say so, and is the file it wrote still everything
//! the source was?
//!
//! # Where the expected values come from
//!
//! ISO 19005-2:2011 and ISO 19005-4:2020 are licensed to a single reader, so this file cites
//! their clauses and does not quote them; ISO 32000-2 is quoted where a rule turns on its words.
//! A section sign marks a clause of ISO 32000-2 and nothing else.

#![expect(
    clippy::expect_used,
    clippy::panic,
    reason = "test code: a fixture that cannot be built, and a requirement the converter was \
              expected to decide about and did not, must both fail loudly"
)]

use std::collections::BTreeSet;
use std::fmt::Write as _;

use pdf_archive::{Flavour, Level, Outcome, Target, Verdict};
use pdf_syntax::object::ObjectId;
use pdf_syntax::{Document, Limits};
use pdf_transform::archive::{ArchivePlan, Authorisations, Because, Decision, Loss, Rewrite};
use pdf_transform::{Budget, Exit, MemorySinks, Plan, Policy, Report, Source, apply};

/// The XMP identification packet each part requires of a conforming file.
///
/// ISO 19005-4 section 6.7.3 asks for `pdfaid:part` 4 and `pdfaid:rev`, the four-digit year of
/// the revision, with no conformance property for the plain profile; ISO 19005-2 section 6.6.4
/// asks for `pdfaid:part` 2 and `pdfaid:conformance`, one of A, B or U. The two parts differ in
/// this and in the header's version, which is why the fixture is built per target rather than
/// once.
fn identification(target: Target) -> String {
    let claim = match target.part() {
        pdf_archive::Part::Two => {
            "<pdfaid:part>2</pdfaid:part>\n<pdfaid:conformance>B</pdfaid:conformance>"
        }
        pdf_archive::Part::Four => "<pdfaid:part>4</pdfaid:part>\n<pdfaid:rev>2020</pdfaid:rev>",
    };
    let mut packet = String::new();
    packet.push_str("<?xpacket begin=\"\u{feff}\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?>\n");
    packet.push_str("<x:xmpmeta xmlns:x=\"adobe:ns:meta/\">\n");
    packet.push_str("<rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\n");
    packet.push_str("<rdf:Description rdf:about=\"\" ");
    packet.push_str("xmlns:pdfaid=\"http://www.aiim.org/pdfa/ns/id/\">\n");
    packet.push_str(claim);
    packet.push_str("\n</rdf:Description>\n</rdf:RDF>\n</x:xmpmeta>\n<?xpacket end=\"w\"?>");
    packet
}

/// A document that conforms to PDF/A-4, with a place to put one violation.
///
/// Five objects: §7.7.2's catalog, §7.7.3's page tree, one page, its content stream and
/// §14.3.2's metadata stream. The header is §7.5.2's two lines — "The PDF file begins with the
/// 5 characters `%PDF-`" and the four bytes "whose codes are 128 or greater" — and the trailer
/// carries §14.4's `/ID`, which ISO 19005-4 section 6.1.3 requires.
#[derive(Debug)]
struct Conforming {
    /// The part and level the fixture is built to conform to.
    ///
    /// It decides the XMP identification packet and the header's version, and nothing else:
    /// every other entry below satisfies both parts.
    target: Target,
    /// Entries added to the catalog dictionary.
    catalog: String,
    /// Entries added to the page dictionary.
    page: String,
    /// The page's §7.8.3 resource dictionary, written between its angle brackets.
    ///
    /// Separate from [`Self::page`] because the page always states one — ISO 19005 section 6.2.2
    /// requires a content stream that references anything to have a resource dictionary
    /// explicitly associated with it — and a second `/Resources` appended after the first would
    /// be a duplicate key rather than an addition.
    resources: String,
    /// Objects 6 and up, each written as its body.
    objects: Vec<String>,
    /// Objects after those, each written as its body, for one that is not text.
    ///
    /// An embedded font program is bytes rather than a string, and splicing it into the built
    /// file afterwards would leave every cross-reference offset past it wrong. So it is a body
    /// like any other and the offsets are taken once, after everything is in place.
    binary_objects: Vec<Vec<u8>>,
    /// The page's content stream, dictionary and data.
    contents: Option<(String, Vec<u8>)>,
    /// What §14.3.2's metadata stream holds, and whether the catalog names it.
    metadata: Packet,
    /// The header line, where a test wants a wrong one.
    header: Option<&'static str>,
}

/// What the fixture's §14.3.2 metadata stream holds.
///
/// Three cases rather than a string, because a conversion has to be tested against all three: the
/// packet that already conforms, a producer's packet claiming something else, and a catalog that
/// names no metadata stream at all.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Packet {
    /// The identification packet the fixture's own target requires.
    Identification,
    /// A packet the test wrote.
    Stated(String),
    /// The stream is written and the catalog does not name it, so the document states none.
    Unnamed,
}

impl Default for Conforming {
    /// PDF/A-4, which `doc/questions/A46` puts first.
    fn default() -> Self {
        Self {
            target: Target::Four(Flavour::Plain),
            catalog: String::new(),
            page: String::new(),
            resources: String::new(),
            objects: Vec::new(),
            binary_objects: Vec::new(),
            contents: None,
            header: None,
            metadata: Packet::Identification,
        }
    }
}

impl Conforming {
    /// The fixture built to a part 2 target, which needs its own identification and header.
    fn part_two() -> Self {
        Self {
            target: Target::Two(Level::B),
            header: Some("%PDF-1.7"),
            ..Self::default()
        }
    }

    /// The bytes.
    fn build(&self) -> Vec<u8> {
        let (content_dict, content_data) = self
            .contents
            .clone()
            .unwrap_or_else(|| (String::new(), Vec::new()));
        let names_metadata = if self.metadata == Packet::Unnamed {
            ""
        } else {
            "/Metadata 5 0 R"
        };
        let mut bodies: Vec<Vec<u8>> = vec![
            format!(
                "<< /Type /Catalog /Pages 2 0 R {names_metadata} {} >>",
                self.catalog
            )
            .into_bytes(),
            b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
            format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Resources << {} >> \
                 /Contents 4 0 R {} >>",
                self.resources, self.page
            )
            .into_bytes(),
            stream(
                &format!("{content_dict} /Length {}", content_data.len()),
                &content_data,
            ),
            {
                let packet = match &self.metadata {
                    Packet::Stated(packet) => packet.clone(),
                    Packet::Identification | Packet::Unnamed => identification(self.target),
                };
                stream(
                    &format!("/Type /Metadata /Subtype /XML /Length {}", packet.len()),
                    packet.as_bytes(),
                )
            },
        ];
        for object in &self.objects {
            bodies.push(object.clone().into_bytes());
        }
        bodies.extend(self.binary_objects.iter().cloned());

        let mut out: Vec<u8> = Vec::new();
        out.extend_from_slice(self.header.unwrap_or("%PDF-2.0").as_bytes());
        out.extend_from_slice(b"\n%\xe2\xe3\xcf\xd3\n");
        let mut offsets = Vec::new();
        for (index, body) in bodies.iter().enumerate() {
            offsets.push(out.len());
            out.extend_from_slice(format!("{} 0 obj\n", index.saturating_add(1)).as_bytes());
            out.extend_from_slice(body);
            out.extend_from_slice(b"\nendobj\n");
        }
        let start = out.len();
        let size = bodies.len().saturating_add(1);
        out.extend_from_slice(format!("xref\n0 {size}\n").as_bytes());
        out.extend_from_slice(b"0000000000 65535 f \n");
        for offset in &offsets {
            out.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
        }
        // §14.4: the identifier is "[a] permanent identifier based on the contents of the
        // file at the time it was originally created". Any two equal strings satisfy the clause for
        // a file created here; what matters to these tests is that the entry is present.
        let identifier = "1D3A4B5C6D7E8F90A1B2C3D4E5F60718";
        out.extend_from_slice(
            format!(
                "trailer\n<< /Size {size} /Root 1 0 R /ID [<{identifier}> <{identifier}>] >>\n\
                 startxref\n{start}\n%%EOF\n"
            )
            .as_bytes(),
        );
        out
    }
}

/// One stream object's body: §7.3.8.1's dictionary, keyword, data and `endstream`.
fn stream(dict: &str, data: &[u8]) -> Vec<u8> {
    let mut out = format!("<< {dict} >>\nstream\n").into_bytes();
    out.extend_from_slice(data);
    out.extend_from_slice(b"\nendstream");
    out
}

/// §7.4.4.2's LZW encoding of `data`, using literal codes only.
///
/// > Data encoded using the LZW compression method shall consist of a sequence of codes that are
/// > 9 to 12 bits long. Each code shall represent a single character of input data (0 -255), a
/// > clear-table marker (256), an EOD marker (257), or a table entry representing a
/// > multiple-character sequence that has been encountered previously in the input (258 or
/// > greater).
///
/// So a clear-table marker, one nine-bit code per byte, and an EOD marker is a valid encoding of
/// any input — it simply exploits none of the compression. It stays nine bits wide for as long
/// as the decoder's next table entry is below 511, which §7.4.4.3's `EarlyChange` default makes
/// the threshold, so this refuses anything long enough to reach it rather than widening codes it
/// would then have to test.
fn lzw_literals(data: &[u8]) -> Vec<u8> {
    assert!(
        data.len() < 200,
        "a literal-code encoding stays nine bits wide only while the decoder's next entry is \
         below §7.4.4.3's early-change threshold"
    );
    let mut bits = String::new();
    let mut push = |code: u16| {
        for shift in (0..9).rev() {
            bits.push(if code >> shift & 1 == 1 { '1' } else { '0' });
        }
    };
    push(256);
    for byte in data {
        push(u16::from(*byte));
    }
    push(257);
    while !bits.len().is_multiple_of(8) {
        bits.push('0');
    }
    bits.as_bytes()
        .chunks(8)
        .map(|chunk| {
            chunk
                .iter()
                .fold(0_u8, |value, bit| (value << 1) | u8::from(*bit == b'1'))
        })
        .collect()
}

/// Converts `bytes` to `target`, answering the report and the output where one was written.
fn convert(bytes: &[u8], target: Target, authorised: Authorisations) -> (Report, Option<Vec<u8>>) {
    let sinks = MemorySinks::new();
    let report = apply(
        &Plan::Archive(ArchivePlan {
            source: 0,
            names: "out.pdf".parse().expect("a pattern"),
            target,
            authorised,
            profile: None,
            substitute_fonts: true,
        }),
        &[Source::new(bytes.to_vec())],
        &sinks,
        &Policy::default(),
        &Budget::default(),
    )
    .expect("the conversion applies");
    let output = sinks.into_outputs().pop().map(|(_, bytes)| bytes);
    (report, output)
}

/// The same, to PDF/A-4 with nothing authorised.
fn to_part_four(bytes: &[u8]) -> (Report, Option<Vec<u8>>) {
    convert(
        bytes,
        Target::Four(Flavour::Plain),
        Authorisations::default(),
    )
}

/// The conversion's own report, which every test reads.
fn conversion(report: &Report) -> &pdf_transform::archive::Conversion {
    report
        .archive
        .as_ref()
        .expect("every archive plan reports a conversion, written or not")
}

/// The decision taken about one requirement.
fn decision(report: &Report, requirement: &str) -> Decision {
    conversion(report)
        .decided
        .iter()
        .find(|decided| decided.requirement == requirement)
        .unwrap_or_else(|| panic!("{requirement} was not among the requirements that failed"))
        .decision
}

/// Opens a converted file and holds it to the target again, independently of the verb.
fn holds(bytes: &[u8], target: Target) -> pdf_archive::Report {
    let document = Document::open_with_limits(bytes.to_vec(), Limits::DEFAULT)
        .expect("a converted document opens");
    pdf_archive::check(&document, target)
}

#[test]
fn the_fixture_conforms_to_part_four_before_anything_is_converted() {
    let report = holds(&Conforming::default().build(), Target::Four(Flavour::Plain));
    assert_eq!(
        report.verdict(),
        Verdict::Conforms,
        "the fixture every other test starts from:\n{}",
        report.render()
    );
}

#[test]
fn a_document_that_already_conforms_is_not_rewritten() {
    let (report, output) = to_part_four(&Conforming::default().build());
    let conversion = conversion(&report);
    assert!(
        conversion.decided.is_empty(),
        "nothing failed, so nothing was decided: {:?}",
        conversion.decided
    );
    assert_eq!(report.exit(false, false), Exit::Success);
    let output = output.expect("a conforming document still gets a file");
    assert_eq!(
        holds(&output, Target::Four(Flavour::Plain)).verdict(),
        Verdict::Conforms
    );
    let achieved = conversion
        .achieved
        .as_ref()
        .expect("the output was held to the target");
    assert!(achieved.conforms, "and the report says so");
    assert!(achieved.still_failing.is_empty());
}

#[test]
fn the_same_input_twice_produces_identical_bytes() {
    // RFC 0002 section 9: "the same inputs produce the same bytes, with no flag needed". The
    // conversion reads a path or an environment nowhere. It reads a *clock* in exactly one
    // place — the `stEvt:when` of the `xmpMM:History` entry ISO 19005-2 section 6.6.6 asks of a
    // recorded action, which is a fact about when the action happened and cannot be anything
    // else — so this fixture is one no action is recorded for, and
    // `a_device_cmyk_page_under_part_two_gets_the_devicen_default` is where that entry is
    // checked instead.
    let source = Conforming {
        catalog: "/Requirements [<< /Type /Requirement /S /EnableJavaScripts >>]".to_owned(),
        ..Conforming::default()
    }
    .build();
    let (_, first) = to_part_four(&source);
    let (_, second) = to_part_four(&source);
    assert_eq!(first, second, "the same document converted twice");
    assert!(first.is_some(), "and it was converted");
}

#[test]
fn a_requirements_dictionary_is_removed_and_the_removal_is_reported() {
    // ISO 19005-4 section 6.12 forbids the catalog's Requirements key outright.
    let source = Conforming {
        catalog: "/Requirements [<< /Type /Requirement /S /EnableJavaScripts >>]".to_owned(),
        ..Conforming::default()
    }
    .build();
    let (report, output) = to_part_four(&source);
    assert_eq!(
        decision(&report, "document-requirements/no-requirements-dictionary"),
        Decision::Mechanical(Rewrite::RequirementsDictionary)
    );
    let output = output.expect("the document converts");
    let held = holds(&output, Target::Four(Flavour::Plain));
    assert_eq!(held.verdict(), Verdict::Conforms, "{}", held.render());
    assert!(
        !String::from_utf8_lossy(&output).contains("Requirements"),
        "the key is gone from the file, not only from the verdict"
    );
}

#[test]
fn a_pages_presentation_steps_are_removed() {
    // ISO 19005-4 section 6.11.
    let source = Conforming {
        page: "/PresSteps << /Type /NavNode >>".to_owned(),
        ..Conforming::default()
    }
    .build();
    let (report, output) = to_part_four(&source);
    assert_eq!(
        decision(&report, "alternate-presentations/no-presentation-steps"),
        Decision::Mechanical(Rewrite::PresentationSteps)
    );
    let output = output.expect("the document converts");
    assert_eq!(
        holds(&output, Target::Four(Flavour::Plain)).verdict(),
        Verdict::Conforms
    );
}

#[test]
fn the_name_dictionarys_alternate_presentations_are_removed() {
    // ISO 19005-4 section 6.11, the other half of the same subclause. The name dictionary is
    // reached through §7.7.2's `/Names`, and this states it *indirectly* on purpose: an
    // indirect one is a separate object, which is the harder of the two paths through the
    // rewriter.
    let source = Conforming {
        catalog: "/Names 6 0 R".to_owned(),
        objects: vec!["<< /AlternatePresentations << >> >>".to_owned()],
        ..Conforming::default()
    }
    .build();
    let (report, output) = to_part_four(&source);
    assert_eq!(
        decision(
            &report,
            "alternate-presentations/none-in-the-name-dictionary"
        ),
        Decision::Mechanical(Rewrite::AlternatePresentations)
    );
    let output = output.expect("the document converts");
    assert_eq!(
        holds(&output, Target::Four(Flavour::Plain)).verdict(),
        Verdict::Conforms
    );
}

#[test]
fn an_lzw_stream_becomes_flate_and_its_decoded_bytes_are_unchanged() {
    // ISO 19005-2 section 6.1.7.2 and ISO 19005-4 section 6.1.6.2 forbid the LZWDecode filter.
    // What the rewrite promises is §7.4.4's own claim about the two filters — they "decode data
    // that has been encoded using the LZW or Flate data compression method, respectively" — so
    // the decoded bytes are the test, and the encoded ones are expected to differ.
    let marks = b"0 0 200 200 re W n\n";
    let source = Conforming {
        contents: Some(("/Filter /LZWDecode".to_owned(), lzw_literals(marks))),
        ..Conforming::default()
    }
    .build();
    let before = decoded_contents(&source);
    assert_eq!(
        before, marks,
        "the fixture's own stream decodes to its marks"
    );

    let (report, output) = to_part_four(&source);
    assert_eq!(
        decision(&report, "file-structure/no-lzw-filter"),
        Decision::Mechanical(Rewrite::FlateInsteadOfLzw)
    );
    let output = output.expect("the document converts");
    assert_eq!(
        holds(&output, Target::Four(Flavour::Plain)).verdict(),
        Verdict::Conforms
    );
    assert_eq!(
        decoded_contents(&output),
        marks,
        "every mark the producer specified is the same mark"
    );
    assert!(
        String::from_utf8_lossy(&output).contains("FlateDecode"),
        "and the filter it is stated under is the one the clause admits"
    );
}

/// The first page's content stream, decoded.
fn decoded_contents(bytes: &[u8]) -> Vec<u8> {
    let document =
        Document::open_with_limits(bytes.to_vec(), Limits::DEFAULT).expect("the document opens");
    let pages = pdf_model::Pages::new(&document);
    let page = pages.get(0).expect("one page");
    let contents = document.get_key(&page.dict, "Contents");
    let stream = contents.as_stream().expect("one content stream");
    document
        .decoded_stream_data(stream)
        .expect("the content stream decodes")
        .to_vec()
}

#[test]
fn the_catalogs_version_is_restated_in_the_shape_part_four_requires() {
    // ISO 19005-4 section 6.1.12 fixes the shape of the catalog's Version: a 2, a full stop and
    // one decimal digit. §7.7.2's Table 28 gives the entry its meaning, "[t]he version of the
    // PDF specification to which this document conforms", so restating it as the header's own
    // version says what the file already asserts.
    let source = Conforming {
        catalog: "/Version /2".to_owned(),
        ..Conforming::default()
    }
    .build();
    let (report, output) = to_part_four(&source);
    assert_eq!(
        decision(&report, "file-structure/catalog-version-key"),
        Decision::Mechanical(Rewrite::CatalogVersion)
    );
    let output = output.expect("the document converts");
    assert_eq!(
        holds(&output, Target::Four(Flavour::Plain)).verdict(),
        Verdict::Conforms
    );
    assert!(
        String::from_utf8_lossy(&output).contains("/Version /2.0"),
        "the value is three characters, as the clause requires"
    );
}

#[test]
fn a_header_stating_the_wrong_part_is_rewritten() {
    // ISO 19005-4 section 6.1.2 admits 2.0 to 2.9; a 1.7 header is a failure rather than a
    // curiosity. Raising the version is allowed and lowering it is not — the other half is
    // `a_pdf_two_source_cannot_be_lowered_to_part_two`.
    let source = Conforming {
        header: Some("%PDF-1.7"),
        ..Conforming::default()
    }
    .build();
    let (report, output) = to_part_four(&source);
    assert_eq!(
        decision(&report, "file-structure/file-header"),
        Decision::Mechanical(Rewrite::FileHeader)
    );
    let output = output.expect("the document converts");
    assert_eq!(
        holds(&output, Target::Four(Flavour::Plain)).verdict(),
        Verdict::Conforms
    );
    assert!(output.starts_with(b"%PDF-2.0"));
}

#[test]
fn a_pdf_two_source_cannot_be_lowered_to_part_two() {
    // ISO 19005-2 section 6.1.2 requires a %PDF-1.n header, so a PDF 2.0 source would have every
    // construct PDF 2.0 added translated or refused one at a time
    // (doc/pdf-a-conversion-limits.md section 6). This converter translates none of them, so it
    // says so rather than writing a file whose header disowns its contents.
    let (report, output) = convert(
        &Conforming::default().build(),
        Target::Two(Level::B),
        Authorisations::default(),
    );
    assert!(output.is_none(), "no file is written");
    assert_eq!(report.exit(false, false), Exit::Refused);
    assert!(
        matches!(
            decision(&report, "file-structure/file-header"),
            Decision::Refused(Because::NotThisTarget(_))
        ),
        "and the refusal says the target is the problem rather than the tool"
    );
}

#[test]
fn an_images_alternates_and_opi_are_removed() {
    // ISO 19005-4 section 6.2.7.1 forbids both keys on an image XObject.
    let source = Conforming {
        objects: vec![
            String::from_utf8_lossy(&stream(
                "/Type /XObject /Subtype /Image /Width 1 /Height 1 /ColorSpace /DeviceGray \
             /BitsPerComponent 8 /OPI << >> /Alternates [] /Length 1",
                b"\x00",
            ))
            .into_owned(),
        ],
        resources: "/XObject << /Im0 6 0 R >>".to_owned(),
        ..Conforming::default()
    }
    .build();
    let (report, output) = to_part_four(&source);
    assert_eq!(
        decision(&report, "graphics/no-image-alternates-or-opi"),
        Decision::Mechanical(Rewrite::ImageAlternatesAndOpi)
    );
    let output = output.expect("the document converts");
    let held = holds(&output, Target::Four(Flavour::Plain));
    assert_eq!(held.verdict(), Verdict::Conforms, "{}", held.render());
    let text = String::from_utf8_lossy(&output);
    assert!(!text.contains("/OPI") && !text.contains("/Alternates"));
}

#[test]
fn image_interpolation_is_a_loss_and_needs_authorising() {
    // doc/pdf-a-conversion-limits.md section 4.7 marks this row **not quite mechanical**: turning
    // interpolation off is what ISO 19005-4 section 6.2.7.1 requires, and a low-resolution image
    // will look blockier for it. So the conversion stops until somebody says it may.
    let source = Conforming {
        objects: vec![
            String::from_utf8_lossy(&stream(
                "/Type /XObject /Subtype /Image /Width 1 /Height 1 /ColorSpace /DeviceGray \
             /BitsPerComponent 8 /Interpolate true /Length 1",
                b"\x00",
            ))
            .into_owned(),
        ],
        resources: "/XObject << /Im0 6 0 R >>".to_owned(),
        ..Conforming::default()
    }
    .build();

    let (report, output) = to_part_four(&source);
    assert!(output.is_none(), "unauthorised, so nothing is written");
    assert_eq!(report.exit(false, false), Exit::Refused);
    assert_eq!(
        decision(&report, "graphics/image-interpolation-is-off"),
        Decision::Unauthorised {
            loss: Loss::ImageSmoothing,
            rewrite: Rewrite::InterpolationOff,
        }
    );

    let authorised = Authorisations {
        image_smoothing: true,
        metadata_property: false,
        annotation_printing: false,
        jpeg2000_colour_fallback: false,
    };
    let (report, output) = convert(&source, Target::Four(Flavour::Plain), authorised);
    assert_eq!(
        decision(&report, "graphics/image-interpolation-is-off"),
        Decision::Authorised {
            loss: Loss::ImageSmoothing,
            rewrite: Rewrite::InterpolationOff,
        }
    );
    let output = output.expect("authorised, so it converts");
    assert_eq!(
        holds(&output, Target::Four(Flavour::Plain)).verdict(),
        Verdict::Conforms
    );
    assert!(String::from_utf8_lossy(&output).contains("/Interpolate false"));
}

#[test]
fn an_annotation_with_no_appearance_is_given_the_one_its_clause_states() {
    // ISO 19005-2 section 6.3.3 and ISO 19005-4 section 6.3.3 require an appearance dictionary of
    // every annotation but three subtypes and the degenerate rectangle. §12.5.2's Table 166
    // requires the same of a writer, and §12.5.6.8 says what a square's marks are — its /IC
    // interior and its border — so the construction is the standard's rather than this program's
    // invention. doc/questions/A21 allows it on condition every one written is reported.
    //
    // The page paints in a device colour of its own, because the constructed stream paints in
    // the annotation's: ISO 19005-4 section 6.2.4.3 licenses a device colour space through an
    // output intent, and a document that had none of its own would need one added for marks the
    // conversion itself wrote. Nothing is changed that no failed requirement asked for, so this
    // conversion does not add one on its own account — the output's own verdict refuses such a
    // file instead, which is the net doc/adr/0947 built.
    let source = Conforming {
        page: "/Annots [6 0 R]".to_owned(),
        objects: vec![
            "<< /Type /Annot /Subtype /Square /Rect [10 10 90 90] /F 4 /IC [0 0 1] >>".to_owned(),
        ],
        contents: Some(paints_in_device_rgb()),
        ..Conforming::default()
    }
    .build();
    let target = Target::Four(Flavour::Plain);

    let (report, output) = to_part_four(&source);
    assert!(matches!(
        decision(
            &report,
            "annotations/appearance-dictionary-present-from-base-standard"
        ),
        Decision::Stated {
            rewrite: Rewrite::AppearanceDictionary,
            ..
        }
    ));
    let output = output.expect("nothing is lost, so no authorisation is asked for");
    assert_eq!(holds(&output, target).verdict(), Verdict::Conforms);

    let appearances = &conversion(&report).appearances;
    assert_eq!(appearances.len(), 1, "one appearance: {appearances:?}");
    let written = appearances.first().expect("the appearance written");
    assert_eq!(written.subtype, "Square");
    assert_eq!(written.page, 0);

    let text = String::from_utf8_lossy(&output);
    assert!(text.contains("/AP"), "the annotation names its appearance");
    assert!(
        text.contains("/Subtype /Form"),
        "and §12.5.5 makes the thing it names a form XObject"
    );
    assert!(
        text.contains("/BBox [10 10 90 90]"),
        "whose box is the annotation rectangle §12.5.5 renders it inside"
    );
}

#[test]
fn an_annotation_whose_clause_states_no_artwork_refuses_rather_than_inventing_one() {
    // §12.5.6.12's stamp displays an icon whose artwork no clause states: Table 184's names are
    // legends rather than symbols, and the clause **recommends** rather than requires a
    // predefined appearance. So there is nothing to construct, and constructing something would
    // put a mark on the page the document never described — doc/questions/A48's line.
    let source = Conforming {
        page: "/Annots [6 0 R]".to_owned(),
        objects: vec![
            "<< /Type /Annot /Subtype /Stamp /Rect [10 10 90 90] /F 4 /Name /Approved >>"
                .to_owned(),
        ],
        ..Conforming::default()
    }
    .build();

    let (report, output) = to_part_four(&source);
    assert!(output.is_none(), "nothing derivable, so nothing is written");
    assert_eq!(report.exit(false, false), Exit::Refused);
    assert!(matches!(
        decision(
            &report,
            "annotations/appearance-dictionary-present-from-base-standard"
        ),
        Decision::Refused(Because::TheFence(_))
    ));
}

#[test]
fn an_annotation_stating_no_flags_is_made_printable_only_with_authorisation() {
    // ISO 19005-2 section 6.3.2 and ISO 19005-4 section 6.3.2 require every annotation but a
    // Popup to state an /F, and require its Print bit set. §12.5.2's Table 166 defaults /F to 0,
    // and §12.5.3's Table 167 says a clear Print bit means the annotation is never printed — so
    // the entry the requirement asks for says the opposite of what the file said, which is why
    // doc/pdf-a-conversion-limits.md section 3.7 makes this an Ask.
    //
    // A Link, because Table 166 exempts that subtype from needing an appearance dictionary: the
    // fixture then fails this requirement and no other.
    let source = Conforming {
        page: "/Annots [6 0 R]".to_owned(),
        objects: vec!["<< /Type /Annot /Subtype /Link /Rect [0 0 10 10] >>".to_owned()],
        ..Conforming::default()
    }
    .build();
    let target = Target::Four(Flavour::Plain);

    let (report, output) = to_part_four(&source);
    assert!(output.is_none(), "unauthorised, so nothing is written");
    assert_eq!(
        decision(&report, "annotations/flags-entry-present"),
        Decision::Unauthorised {
            loss: Loss::AnnotationPrinting,
            rewrite: Rewrite::AnnotationFlags,
        }
    );

    let authorised = Authorisations {
        image_smoothing: false,
        metadata_property: false,
        annotation_printing: true,
        jpeg2000_colour_fallback: false,
    };
    let (report, output) = convert(&source, target, authorised);
    assert_eq!(
        decision(&report, "annotations/flags-entry-present"),
        Decision::Authorised {
            loss: Loss::AnnotationPrinting,
            rewrite: Rewrite::AnnotationFlags,
        }
    );
    let output = output.expect("authorised, so it converts");
    assert_eq!(holds(&output, target).verdict(), Verdict::Conforms);
    assert!(
        String::from_utf8_lossy(&output).contains("/F 4"),
        "Table 167 numbers Print at bit position 3, so the value is 4 and every other flag is \
         the default it already had"
    );
}

#[test]
fn a_property_its_own_schema_does_not_define_is_removed_only_with_authorisation() {
    // ISO 19005-2 section 6.6.2.3.1 requires every property to *use* the schema whose namespace
    // it names, and a packet can name a predefined namespace without using it: the XMP basic
    // schema defines `xmp:CreateDate` as a date, and this one states a sentence.
    // doc/pdf-a-conversion-limits.md section 3.9 is the reading, and it closes two of the three
    // routes out — correcting the value would be inventing content, and describing a predefined
    // schema in section 6.6.2.3.2's extension container would misrepresent it in the file. What
    // is left is removing the property, which is a loss and therefore an Ask.
    let mut packet = String::new();
    packet.push_str("<?xpacket begin=\"\u{feff}\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?>\n");
    packet.push_str("<x:xmpmeta xmlns:x=\"adobe:ns:meta/\">\n");
    packet.push_str("<rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\n");
    packet.push_str("<rdf:Description rdf:about=\"\" ");
    packet.push_str("xmlns:pdfaid=\"http://www.aiim.org/pdfa/ns/id/\" ");
    packet.push_str("xmlns:xmp=\"http://ns.adobe.com/xap/1.0/\" ");
    packet.push_str("xmlns:pdf=\"http://ns.adobe.com/pdf/1.3/\">\n");
    packet.push_str("<pdfaid:part>2</pdfaid:part>\n<pdfaid:conformance>B</pdfaid:conformance>\n");
    packet.push_str("<xmp:CreateDate>the day we shipped it</xmp:CreateDate>\n");
    packet.push_str("<pdf:Producer>Somebody's exporter</pdf:Producer>\n");
    packet.push_str("</rdf:Description>\n</rdf:RDF>\n</x:xmpmeta>\n<?xpacket end=\"w\"?>");
    let source = Conforming {
        metadata: Packet::Stated(packet),
        ..Conforming::part_two()
    }
    .build();
    let target = Target::Two(Level::B);

    let (report, output) = convert(&source, target, Authorisations::default());
    assert!(output.is_none(), "unauthorised, so nothing is written");
    assert_eq!(report.exit(false, false), Exit::Refused);
    assert_eq!(
        decision(&report, "metadata/properties-use-known-schemas"),
        Decision::Unauthorised {
            loss: Loss::MetadataProperty,
            rewrite: Rewrite::PropertyOutsideItsSchema,
        }
    );

    let authorised = Authorisations {
        image_smoothing: false,
        metadata_property: true,
        annotation_printing: false,
        jpeg2000_colour_fallback: false,
    };
    let (report, output) = convert(&source, target, authorised);
    assert_eq!(
        decision(&report, "metadata/properties-use-known-schemas"),
        Decision::Authorised {
            loss: Loss::MetadataProperty,
            rewrite: Rewrite::PropertyOutsideItsSchema,
        }
    );
    let output = output.expect("authorised, so it converts");
    assert_eq!(holds(&output, target).verdict(), Verdict::Conforms);

    // section 3.9's condition: the report names the property, its namespace and what was there,
    // because a removed property leaves nothing in the output to find it by.
    let removed = &conversion(&report).removed;
    assert_eq!(removed.len(), 1, "one property went: {removed:?}");
    let gone = removed.first().expect("the property that went");
    assert_eq!(gone.spelled, "xmp:CreateDate");
    assert_eq!(gone.name.namespace, "http://ns.adobe.com/xap/1.0/");
    assert_eq!(gone.stated, "the day we shipped it");

    let written = String::from_utf8_lossy(&output);
    assert!(
        !written.contains("the day we shipped it"),
        "the property is out of the packet, value and all"
    );
    assert!(
        written.contains("Somebody's exporter"),
        "and every other byte of the producer's packet crosses unchanged"
    );
    assert!(
        written.contains("<stEvt:action>converted</stEvt:action>"),
        "section 4.2: every Ask writes one xmpMM:History entry, which is the audit trail that \
         makes it defensible"
    );
}

#[test]
fn a_form_xobjects_opi_and_postscript_passthrough_are_removed() {
    // ISO 19005-2 section 6.2.9.1 forbids /OPI on a form XObject, and forbids the two keys PDF
    // 2.0 does not define at all: a /PS stream and a /Subtype2 of PS. ISO 32000-1's 8.8.2 says
    // a PostScript fragment has no effect when the document is viewed on screen or printed to a
    // non-PostScript device, so removing it takes nothing off the page.
    //
    // A part 2 target, because part 4 states only the /OPI half — the target is what decides
    // which requirements bind, and this is the case where they differ.
    let source = Conforming {
        objects: vec![
            String::from_utf8_lossy(&stream(
                "/Type /XObject /Subtype /Form /BBox [0 0 1 1] /Resources << >> /OPI << >> \
                 /Subtype2 /PS /PS 7 0 R /Length 0",
                b"",
            ))
            .into_owned(),
            String::from_utf8_lossy(&stream("/Length 0", b"")).into_owned(),
        ],
        resources: "/XObject << /Fm0 6 0 R >>".to_owned(),
        ..Conforming::part_two()
    }
    .build();
    let (report, output) = convert(&source, Target::Two(Level::B), Authorisations::default());
    let conversion = conversion(&report);
    assert_eq!(
        decision(&report, "graphics/no-form-xobject-opi"),
        Decision::Mechanical(Rewrite::FormOpi),
        "{:?}",
        conversion.decided
    );
    assert_eq!(
        decision(
            &report,
            "graphics/no-postscript-passthrough-in-a-form-xobject"
        ),
        Decision::Mechanical(Rewrite::FormPostScript)
    );
    let output = output.expect("the document converts");
    let text = String::from_utf8_lossy(&output);
    assert!(!text.contains("/OPI") && !text.contains("/Subtype2") && !text.contains("/PS "));
}

#[test]
fn a_postscript_xobject_is_dropped_with_the_resource_entry_that_named_it() {
    // ISO 19005-2 section 6.2.9.3. The object is dropped, and §7.3.10 does the rest: "an
    // indirect reference to an undefined object shall not be considered an error by a PDF
    // processor; it shall be treated as a reference to the null object", and §7.3.7 makes "[a]
    // dictionary entry whose value is null ... the same as if the entry does not exist". So the
    // resource entry naming it disappears without the rewriter having to find it.
    let source = Conforming {
        objects: vec![
            String::from_utf8_lossy(&stream("/Type /XObject /Subtype /PS /Length 0", b""))
                .into_owned(),
        ],
        resources: "/XObject << /Ps0 6 0 R >>".to_owned(),
        ..Conforming::part_two()
    }
    .build();
    let (report, output) = convert(&source, Target::Two(Level::B), Authorisations::default());
    assert_eq!(
        decision(&report, "graphics/no-postscript-xobjects"),
        Decision::Mechanical(Rewrite::PostScriptXObject)
    );
    let output = output.expect("the document converts");
    let text = String::from_utf8_lossy(&output);
    assert!(!text.contains("/Ps0"), "the resource entry went with it");
    assert!(!text.contains("/Subtype /PS"));
}

#[test]
fn a_requirement_this_converter_cannot_meet_refuses_by_name_and_writes_nothing() {
    // ISO 19005-4 section 6.2.5 forbids a transfer function in a graphics state.
    // doc/pdf-a-conversion-limits.md section 4.8 says why removing one is not mechanical — a
    // transfer function decides what a *screen* shows, and an inverting one is a photographic
    // negative — so this converter does not do it yet, and says which requirement it could not
    // meet rather than writing a file that claims to have met it.
    let source = Conforming {
        objects: vec!["<< /Type /ExtGState /TR /Identity >>".to_owned()],
        resources: "/ExtGState << /Gs0 6 0 R >>".to_owned(),
        ..Conforming::default()
    }
    .build();
    let (report, output) = to_part_four(&source);
    assert!(output.is_none(), "no file is written");
    assert_eq!(report.exit(false, false), Exit::Refused);
    assert!(matches!(
        decision(&report, "graphics/no-transfer-function-in-a-graphics-state"),
        Decision::Refused(Because::NotBuiltYet(_))
    ));
    let declined = report
        .refused
        .first()
        .expect("the refusal is named beside the report");
    assert!(
        declined
            .subject
            .contains("graphics/no-transfer-function-in-a-graphics-state"),
        "and it names the requirement and its clause: {}",
        declined.subject
    );
    assert!(declined.subject.contains("ISO 19005-4 section 6.2.5"));
}

#[test]
fn the_report_carries_what_the_verdict_does_not_cover() {
    // doc/pdf-a-conversion-limits.md section 7: "A pass is relative to a stated list", and the
    // list is printed with the verdict. `doc/questions/A20` adds that the reason may never be
    // softened, so the conversion carries the validator's own sentence rather than a summary of it.
    let (report, _) = to_part_four(&Conforming::default().build());
    let conversion = conversion(&report);
    assert!(
        !conversion.not_checked.is_empty(),
        "this crate does not check everything, and says which"
    );
    let held = holds(&Conforming::default().build(), Target::Four(Flavour::Plain));
    for judgement in held.unchecked() {
        let carried = conversion
            .not_checked
            .iter()
            .find(|row| row.requirement == judgement.id)
            .expect("every unchecked requirement reaches the conversion's report");
        let Outcome::Unchecked(why) = judgement.outcome else {
            unreachable!("`unchecked` yields only unchecked rows");
        };
        assert_eq!(carried.because, why, "word for word");
        assert_eq!(carried.citation, judgement.citation);
    }
}

#[test]
fn every_requirement_the_decision_table_answers_is_one_the_validator_states() {
    // The decision table is keyed by `pdf_archive`'s identifiers, and a typo in one would not
    // fail to compile: it would quietly become a requirement nobody answers, which this verb
    // then refuses. So the two tables are compared here, where a typo is a failing test rather
    // than a document that cannot be converted for a reason nobody can see.
    let mut stated: Vec<&'static str> = Vec::new();
    for target in Target::ALL {
        for requirement in pdf_archive::table::binding(target) {
            stated.push(requirement.id);
        }
    }
    for answered in pdf_transform::archive::answered() {
        assert!(
            stated.contains(&answered),
            "{answered} is answered by the converter and stated by no requirement"
        );
    }
    // The same trap, one table over: a refusal keyed by a name no requirement carries is a
    // refusal nobody ever reads, and the document gets the generic "a later slice owes this"
    // instead of the argument that was written for it.
    for refused in pdf_transform::archive::refused_by_name() {
        assert!(
            stated.contains(&refused),
            "{refused} is refused by name and stated by no requirement"
        );
    }
}

#[test]
fn the_unconsidered_requirements_are_the_ones_this_file_names() {
    // The coverage ratchet, in `doc/todo/00`'s shape: equality in both directions rather than a
    // ceiling. A requirement that reaches none of `decision.rs`'s three tables is answered with
    // the catch-all sentence — "this converter does not yet meet this requirement" — which is
    // true of every gap and informative about none, so the set of requirements in that state is
    // held where it is. Arriving fails the build on arrival; leaving means striking the line in
    // the commit that answered it.
    //
    // The corpus cannot do this job: it ranks requirements by how many documents exercise them,
    // and a requirement no document in the corpus exercises is invisible to it. `CLAUDE.md`'s
    // two denominators, and this is the one whose denominator is the specification.
    let named: BTreeSet<&str> = include_str!("archive_unconsidered.txt")
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect();
    let found: BTreeSet<&str> = pdf_transform::archive::unconsidered()
        .into_iter()
        .map(|requirement| requirement.id)
        .collect();
    let arrived: Vec<&&str> = found.difference(&named).collect();
    let left: Vec<&&str> = named.difference(&found).collect();
    assert!(
        arrived.is_empty(),
        "these requirements have no considered answer and archive_unconsidered.txt does not \
         name them: {arrived:?}. Give each one a row in decision.rs — a remedy, a refusal with \
         its own argument, or a not-built-yet that names what it waits on — or add it here."
    );
    assert!(
        left.is_empty(),
        "archive_unconsidered.txt names requirements that now have a considered answer: \
         {left:?}. Strike them from the file."
    );
}

#[test]
fn the_json_report_names_the_target_the_decisions_and_the_outcome() {
    // RFC 0002 section 4.5's report, and `doc/adr/0927`'s condition: what was written is *named
    // in the report the conversion produces*, per document, rather than inferable from a diff.
    let source = Conforming {
        catalog: "/Requirements [<< /Type /Requirement /S /EnableJavaScripts >>]".to_owned(),
        ..Conforming::default()
    }
    .build();
    let (report, _) = to_part_four(&source);
    let json = report.to_json().render();
    for expected in [
        "\"archive\"",
        "\"target\": \"PDF/A-4\"",
        "\"requirement\": \"document-requirements/no-requirements-dictionary\"",
        "\"clause\": \"ISO 19005-4 section 6.12\"",
        "\"decision\": \"mechanical\"",
        "\"rewrite\": \"requirements-dictionary\"",
        "\"conforms\": true",
        "\"not_checked\"",
    ] {
        assert!(
            json.contains(expected),
            "the report is missing {expected}:\n{json}"
        );
    }
}

#[test]
fn a_conversion_that_would_break_what_the_source_met_writes_nothing() {
    // The property that makes the report's claim about the output a measurement rather than a
    // promise. A PostScript XObject a content stream *invokes* is the case that reaches it: the
    // object has to go (ISO 19005-2 section 6.2.9.3) and the `Do` that names it may not be
    // edited (doc/pdf-a-conversion-limits.md section 5.2), so the removal leaves the named resource
    // undefined, which ISO 19005-2 section 6.2.2 forbids.
    let source = Conforming {
        objects: vec![
            String::from_utf8_lossy(&stream("/Type /XObject /Subtype /PS /Length 0", b""))
                .into_owned(),
        ],
        resources: "/XObject << /Ps0 6 0 R >>".to_owned(),
        contents: Some((String::new(), b"/Ps0 Do".to_vec())),
        ..Conforming::part_two()
    }
    .build();
    let (report, output) = convert(&source, Target::Two(Level::B), Authorisations::default());
    assert!(output.is_none(), "nothing is written");
    assert_eq!(report.exit(false, false), Exit::Refused);
    let achieved = conversion(&report)
        .achieved
        .as_ref()
        .expect("the output was assembled and then held to the target");
    assert!(!achieved.conforms);
    assert!(
        achieved
            .regressions
            .contains(&"graphics/named-resources-are-defined"),
        "and the report names the requirement the conversion would have broken: {achieved:?}"
    );
}

#[test]
fn an_object_the_catalog_cannot_reach_is_not_carried() {
    // §7.5.5's Table 15 makes `/Root` "( Required; shall be an indirect reference ) The catalog
    // dictionary for the PDF file" and §7.7.2's Table 28 makes the catalog "the root of a
    // document's object hierarchy". An object no path from that root reaches is one no reader of
    // the output can ask for, so the walk does not carry it — and this is written down because
    // it is the one way the output holds *less* than the input without a decision saying so.
    let source = Conforming {
        objects: vec!["<< /Unreachable true >>".to_owned()],
        ..Conforming::default()
    }
    .build();
    let (_, output) = to_part_four(&source);
    let output = output.expect("the document converts");
    assert!(!String::from_utf8_lossy(&output).contains("Unreachable"));
}

#[test]
fn the_targets_are_all_six_and_the_verb_takes_each_of_them() {
    // `doc/pdf-a-conversion-limits.md` section 9: a user whose deposit rule names one target
    // cannot be answered with "use another one", so every target `pdf_archive` knows is selectable
    // here. What each *does* about a given document differs, and that is the report's business;
    // what this asserts is that none of them is unreachable.
    for target in Target::ALL {
        let (report, _) = convert(
            &Conforming::default().build(),
            target,
            Authorisations::default(),
        );
        assert_eq!(conversion(&report).target, target);
    }
}

#[test]
fn every_loss_has_a_word_a_caller_can_authorise_it_by() {
    for loss in Loss::ALL {
        assert_eq!(Loss::parse(loss.word()), Some(loss));
        let mut authorisations = Authorisations::default();
        assert!(!authorisations.grants(loss));
        authorisations.authorise(loss);
        assert!(authorisations.grants(loss));
    }
    assert_eq!(Loss::parse("everything"), None);
}

/// The `desc` and `cprt` tags `data/icc/PROVENANCE.md` read out of the shipped profile by hand.
///
/// Repeated here rather than imported so that a change to the shipped file fails a test in the
/// crate that embeds it as well as in the one that reads it.
const SHIPPED_PROFILE: (&str, &str) =
    ("sRGB2014", "Copyright International Color Consortium, 2015");

/// A page that paints in `DeviceRGB`, which is the commonest reason a document needs an output
/// intent at all.
///
/// §8.6.8's `rg` "shall set the colour space to `DeviceRGB` … and set the colour to use for
/// filling operations", so this content selects the space the colour subclauses restrict.
fn paints_in_device_rgb() -> (String, Vec<u8>) {
    (String::new(), b"1 0 0 rg 0 0 10 10 re f".to_vec())
}

/// A page that paints in `DeviceCMYK`: §8.6.8's `k`, the operator the sRGB default cannot license.
fn paints_in_device_cmyk() -> (String, Vec<u8>) {
    (String::new(), b"0 0 0 1 k 0 0 10 10 re f".to_vec())
}

/// The output intent dictionary a converted file states, where it states one.
fn output_intent(bytes: &[u8]) -> pdf_syntax::object::Dictionary {
    let document = Document::open_with_limits(bytes.to_vec(), Limits::DEFAULT)
        .expect("a converted document opens");
    let catalog = document.catalog().expect("a catalog");
    let intents = document.get_key(&catalog, "OutputIntents");
    let entries = intents.as_array().expect("an OutputIntents array");
    let entry = entries.first().expect("one entry");
    document
        .resolve(entry)
        .as_dict()
        .cloned()
        .expect("an output intent dictionary")
}

#[test]
fn a_device_rgb_page_gains_the_shipped_output_intent_and_the_report_says_what_that_asserts() {
    // ISO 19005-4 section 6.2.4.3 licenses DeviceRGB through a PDF/A output intent holding an RGB
    // destination profile, and section 6.2.3 requires that intent's /S to be GTS_PDFA1.
    // `doc/questions/A18` allows shipping the profile *and attaches the condition this test is
    // about*: the report says that adding one reinterprets the marks already in the file.
    let source = Conforming {
        contents: Some(paints_in_device_rgb()),
        ..Conforming::default()
    }
    .build();
    let (report, output) = to_part_four(&source);
    let decided = decision(
        &report,
        "graphics/device-rgb-needs-a-default-a-blending-space-or-an-rgb-output-intent",
    );
    let Decision::Stated {
        rewrite,
        reinterprets,
    } = decided
    else {
        panic!("an output intent states an interpretation rather than losing nothing: {decided:?}");
    };
    assert_eq!(rewrite, Rewrite::OutputIntent);
    assert!(
        reinterprets.contains("colour-manages"),
        "the sentence a reader is owed: {reinterprets}"
    );

    let output = output.expect("the document converts");
    assert_eq!(
        holds(&output, Target::Four(Flavour::Plain)).verdict(),
        Verdict::Conforms
    );
    let intent = output_intent(&output);
    assert_eq!(
        intent
            .get("S")
            .and_then(|value| value.as_name())
            .map(|name| name.as_bytes().to_vec()),
        Some(b"GTS_PDFA1".to_vec()),
        "the value both parts' section 6.2.3 makes a PDF/A output intent"
    );
    assert_eq!(
        intent
            .get("OutputConditionIdentifier")
            .and_then(pdf_syntax::Object::as_string),
        Some(&b"Custom"[..]),
        "§14.11.5's own word for a production condition that is not a recognised standard"
    );

    let profile = conversion(&report)
        .profile
        .as_ref()
        .expect("the report names the profile the intent embeds");
    assert_eq!(profile.describes.as_deref(), Some(SHIPPED_PROFILE.0));
    assert_eq!(
        profile.copyright.as_deref(),
        Some(SHIPPED_PROFILE.1),
        "doc/pdf-a-conversion-limits.md section 10.1: whose profile this is, read out of the \
         profile"
    );
    assert_eq!(profile.space, "RGB");
    let rendered = conversion(&report).render();
    assert!(
        rendered.contains(SHIPPED_PROFILE.1),
        "the copyright tag is in the report a person reads:\n{rendered}"
    );
}

#[test]
fn a_device_cmyk_page_is_refused_by_name_because_srgb_does_not_license_it() {
    // ISO 19005-4 section 6.2.4.3 licenses DeviceCMYK through a **CMYK** destination profile, and
    // the profile this program ships is RGB. **And part 4 has no second licence**: it states the
    // sentence requiring a *device independent* DefaultCMYK where ISO 19005-2 states it admitting
    // a DeviceN-based one, so the construction the test below writes for part 2 is not open here
    // and the answer is the press's own profile. That is the standard's difference between the
    // two parts rather than a gap in this converter.
    let source = Conforming {
        contents: Some(paints_in_device_cmyk()),
        ..Conforming::default()
    }
    .build();
    let (report, output) = to_part_four(&source);
    let decided = decision(
        &report,
        "graphics/device-cmyk-needs-a-default-a-blending-space-or-a-cmyk-output-intent",
    );
    let Decision::Refused(because) = decided else {
        panic!("an sRGB intent does not license DeviceCMYK: {decided:?}");
    };
    assert!(
        because.sentence().contains("--output-intent-profile"),
        "the refusal says what would answer it: {}",
        because.sentence()
    );
    assert!(output.is_none(), "no file is written");
}

#[test]
fn a_supplied_profile_is_the_one_embedded_and_its_copyright_tag_is_reported() {
    // doc/pdf-a-conversion-limits.md section 10.1: the ICC's guidance is that a profile's terms of
    // use live in its `cprt` tag, so a user embedding somebody else's profile is told whose it is.
    // The profile supplied here is the shipped one, which is the only ICC profile this tree may
    // redistribute — what the test establishes is that a *supplied* profile is read the same way
    // and reported as supplied.
    let source = Conforming {
        contents: Some(paints_in_device_rgb()),
        ..Conforming::default()
    }
    .build();
    let sinks = MemorySinks::new();
    let profile: std::sync::Arc<[u8]> = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/icc/sRGB2014.icc"
    ))
    .expect("the shipped profile is in the tree")
    .into();
    let report = apply(
        &Plan::Archive(ArchivePlan {
            source: 0,
            names: "out.pdf".parse().expect("a pattern"),
            target: Target::Four(Flavour::Plain),
            authorised: Authorisations::default(),
            profile: Some(profile),
            substitute_fonts: true,
        }),
        &[Source::new(source)],
        &sinks,
        &Policy::default(),
        &Budget::default(),
    )
    .expect("the conversion applies");
    let stated = conversion(&report)
        .profile
        .as_ref()
        .expect("a profile is reported");
    assert_eq!(stated.source.word(), "supplied");
    assert_eq!(stated.copyright.as_deref(), Some(SHIPPED_PROFILE.1));
    assert!(
        sinks.into_outputs().pop().is_some(),
        "the document converts"
    );
}

#[test]
fn a_destination_profile_the_file_already_holds_is_shared_rather_than_doubled() {
    // ISO 19005-2 section 6.2.3 and ISO 19005-4 section 6.2.3: where an OutputIntents array holds
    // more than one entry, every entry that states a DestOutputProfile states the same indirect
    // object. So a file holding a PDF/X intent over an RGB profile gets a PDF/A entry naming
    // *that* object, and a converter that embedded a second profile beside it would break the
    // requirement it was trying to meet.
    let icc = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/icc/sRGB2014.icc"
    ))
    .expect("the shipped profile is in the tree");
    // §7.4.2's ASCIIHexDecode, so that the fixture's object bodies stay text — which is also how
    // §14.11.5's own EXAMPLE writes a profile stream.
    let mut hex = String::new();
    for byte in &icc {
        let _ = write!(hex, "{byte:02X}");
    }
    hex.push('>');
    let source = Conforming {
        contents: Some(paints_in_device_rgb()),
        catalog: "/OutputIntents [ << /Type /OutputIntent /S /GTS_PDFX \
                  /OutputConditionIdentifier (Custom) /DestOutputProfile 6 0 R >> ]"
            .to_owned(),
        objects: vec![
            String::from_utf8_lossy(&stream(
                &format!("/N 3 /Filter /ASCIIHexDecode /Length {}", hex.len()),
                hex.as_bytes(),
            ))
            .into_owned(),
        ],
        ..Conforming::default()
    }
    .build();
    let (report, output) = to_part_four(&source);
    let output = output.unwrap_or_else(|| panic!("{}", conversion(&report).render()));
    assert_eq!(
        holds(&output, Target::Four(Flavour::Plain)).verdict(),
        Verdict::Conforms
    );
    assert_eq!(
        conversion(&report)
            .profile
            .as_ref()
            .map(|profile| profile.source.word()),
        Some("already-in-the-file"),
        "the report says the profile was not this conversion's choice"
    );
    let document = Document::open_with_limits(output, Limits::DEFAULT).expect("it opens");
    let catalog = document.catalog().expect("a catalog");
    let entries = document.get_key(&catalog, "OutputIntents");
    let entries = entries.as_array().expect("an array").to_vec();
    assert_eq!(entries.len(), 2, "the producer's entry and the one added");
    let named: Vec<_> = entries
        .iter()
        .filter_map(|entry| document.resolve(entry).as_dict().cloned())
        .filter_map(|entry| {
            entry
                .get("DestOutputProfile")
                .and_then(pdf_syntax::Object::as_reference)
        })
        .collect();
    assert_eq!(named.len(), 2, "both entries state a destination profile");
    assert_eq!(
        named.first(),
        named.get(1),
        "and they state the same object, which is what section 6.2.3 requires"
    );
}

#[test]
fn a_packet_claiming_another_part_is_edited_in_place_and_keeps_its_other_properties() {
    // ISO 19005-4 section 6.7.3 requires a part number of 4 and a revision year; the fixture's
    // part 2 packet states neither. What this test is really about is the *other* property: a
    // producer's `pdf:Producer` is metadata somebody wrote deliberately, and a conversion that
    // rebuilt the packet from this tree's own reading of it would drop what that reading drops.
    let source = Conforming {
        metadata: Packet::Stated(
            "<?xpacket begin=\"\u{feff}\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?>\n\
             <x:xmpmeta xmlns:x=\"adobe:ns:meta/\">\n\
             <rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\n\
             <rdf:Description rdf:about=\"\" \
             xmlns:pdf=\"http://ns.adobe.com/pdf/1.3/\" \
             xmlns:pdfaid=\"http://www.aiim.org/pdfa/ns/id/\">\n\
             <pdf:Producer>Somebody's exporter</pdf:Producer>\n\
             <pdfaid:part>2</pdfaid:part>\n\
             <pdfaid:conformance>B</pdfaid:conformance>\n\
             </rdf:Description>\n</rdf:RDF>\n</x:xmpmeta>\n<?xpacket end=\"w\"?>"
                .to_owned(),
        ),
        ..Conforming::default()
    }
    .build();
    let (report, output) = to_part_four(&source);
    assert_eq!(
        decision(&report, "metadata/identification-part-number-four"),
        Decision::Mechanical(Rewrite::IdentificationSchema)
    );
    let output = output.expect("the document converts");
    assert_eq!(
        holds(&output, Target::Four(Flavour::Plain)).verdict(),
        Verdict::Conforms
    );
    let packet = document_packet(&output);
    let read = pdf_model::xmp::Xmp::parse(&packet).expect("the written packet parses");
    assert_eq!(
        read.text("http://ns.adobe.com/pdf/1.3/", "Producer"),
        Some("Somebody's exporter"),
        "the producer's own property crosses the conversion"
    );
    assert_eq!(
        read.text("https://www.aiim.org/pdfa/ns/id/", "part"),
        Some("4")
    );
    assert_eq!(
        read.text("https://www.aiim.org/pdfa/ns/id/", "rev"),
        Some("2020")
    );
    assert_eq!(
        read.text("http://www.aiim.org/pdfa/ns/id/", "conformance"),
        None,
        "ISO 19005-4 section 6.7.3 reserves the conformance property for its two annexes, so the \
         old claim's is removed under both spellings of the namespace"
    );
}

#[test]
fn a_document_with_no_metadata_stream_is_given_one() {
    // ISO 19005-2 section 6.6.2.1 and ISO 19005-4 section 6.7.2.1 require the catalog to state a
    // metadata stream; section 4.2 of doc/pdf-a-conversion-limits.md makes synthesising one the
    // default where the document has none. §14.3.2's Table 347 decides the two entries it carries.
    let source = Conforming {
        metadata: Packet::Unnamed,
        ..Conforming::default()
    }
    .build();
    let (report, output) = to_part_four(&source);
    assert_eq!(
        decision(&report, "metadata/catalog-metadata-stream"),
        Decision::Mechanical(Rewrite::IdentificationSchema)
    );
    let output = output.expect("the document converts");
    assert_eq!(
        holds(&output, Target::Four(Flavour::Plain)).verdict(),
        Verdict::Conforms
    );
    let read = pdf_model::xmp::Xmp::parse(&document_packet(&output)).expect("it parses");
    assert_eq!(
        read.text("https://www.aiim.org/pdfa/ns/id/", "part"),
        Some("4")
    );
}

/// The bytes of a converted document's catalog metadata stream.
fn document_packet(bytes: &[u8]) -> Vec<u8> {
    let document = Document::open_with_limits(bytes.to_vec(), Limits::DEFAULT).expect("it opens");
    let catalog = document.catalog().expect("a catalog");
    let stream = document.get_key(&catalog, "Metadata");
    let stream = stream.as_stream().expect("a metadata stream");
    document
        .decoded_stream_data(stream)
        .expect("the packet decodes")
        .to_vec()
}

/// The `/DefaultCMYK` a converted part-2 document states, resolved to its array.
fn default_cmyk(document: &Document) -> Vec<pdf_syntax::Object> {
    let page = pdf_model::Pages::new(document).get(0).expect("one page");
    let resources = document
        .get_key(&page.dict, "Resources")
        .as_dict()
        .cloned()
        .expect("the page's own resources");
    let spaces = document
        .get_key(&resources, "ColorSpace")
        .as_dict()
        .cloned()
        .expect("a ColorSpace subdictionary");
    document
        .get_key(&spaces, "DefaultCMYK")
        .as_array()
        .map(<[pdf_syntax::Object]>::to_vec)
        .expect("a DefaultCMYK colour space array")
}

#[test]
fn a_device_cmyk_page_under_part_two_gets_the_devicen_default() {
    // ISO 19005-2 section 6.2.4.3 admits a **DeviceN-based** DefaultCMYK beside a device
    // independent one, and its NOTE 2 says why: such a space is subject to section 6.2.4.4 and is
    // thereby device independent. `doc/questions/A48` allows writing one, on two conditions —
    // that it is reported per document, and that it is recorded in xmpMM:History naming the
    // clause. Both are asserted here, because the permission is the two of them together.
    let (report, output) = converted_cmyk_page();
    let decided = decision(
        &report,
        "graphics/device-cmyk-needs-a-default-or-a-cmyk-output-intent",
    );
    let Decision::Stated {
        rewrite,
        reinterprets,
    } = decided
    else {
        panic!("the DeviceN default states an interpretation the standard defines: {decided:?}");
    };
    assert_eq!(rewrite, Rewrite::DefaultCmyk);
    assert!(
        reinterprets.contains("crude approximation"),
        "§10.4.2.1's own word for what this costs is in the sentence: {reinterprets}"
    );

    assert_eq!(
        holds(&output, Target::Two(Level::B)).verdict(),
        Verdict::Conforms,
        "the output is held to the target again:\n{}",
        holds(&output, Target::Two(Level::B)).render()
    );
    let document = Document::open_with_limits(output, Limits::DEFAULT).expect("it opens");
    let space = default_cmyk(&document);

    // §8.6.6.5's DeviceN: the family, the names, the alternate space and the tint transform.
    assert_eq!(
        space
            .first()
            .and_then(|first| first.as_name())
            .map(|name| name.as_bytes().to_vec()),
        Some(b"DeviceN".to_vec())
    );
    let names: Vec<Vec<u8>> = document
        .resolve(space.get(1).expect("a names array"))
        .as_array()
        .expect("an array")
        .iter()
        .filter_map(|name| name.as_name().map(|name| name.as_bytes().to_vec()))
        .collect();
    assert_eq!(
        names,
        vec![
            b"Cyan".to_vec(),
            b"Magenta".to_vec(),
            b"Yellow".to_vec(),
            b"Black".to_vec()
        ],
        "the four names §8.6.6.5 reserves to a CMYK device's process colourants, in order"
    );
    let alternate = document.resolve(space.get(2).expect("an alternate space"));
    let alternate = alternate.as_array().expect("an ICCBased array");
    assert_eq!(
        alternate
            .first()
            .and_then(|first| first.as_name())
            .map(|name| name.as_bytes().to_vec()),
        Some(b"ICCBased".to_vec()),
        "the alternate is a device independent space, which is what section 6.2.4.4 requires"
    );

    // §10.4.2.5: "red = 1.0 - min(1.0, cyan + black)", and the same for green from magenta and
    // blue from yellow. The tint transform is that arithmetic and this is the check of it —
    // written out here rather than trusted, because the operators §7.10.5.2 defines have no min.
    let tint =
        pdf_model::function::Function::parse(&document, space.get(3).expect("a tint transform"))
            .expect("a type 4 function");
    for cmyk in [
        [0.0, 0.0, 0.0, 1.0],
        [0.25, 0.0, 0.76, 0.0],
        [1.0, 1.0, 1.0, 1.0],
        [0.6, 0.2, 0.1, 0.5],
        [0.0, 0.0, 0.0, 0.0],
    ] {
        let [cyan, magenta, yellow, black] = cmyk;
        let clause =
            [cyan, magenta, yellow].map(|component| 1.0_f32 - 1.0_f32.min(component + black));
        let evaluated = tint.eval(&cmyk);
        for (index, expected) in clause.iter().enumerate() {
            let got = evaluated.get(index).copied().unwrap_or(f32::NAN);
            assert!(
                (got - expected).abs() < 1e-6,
                "{cmyk:?} component {index}: the clause says {expected}, the function says {got}"
            );
        }
    }
}

/// A part-2 document whose one page paints in `DeviceCMYK`, converted.
fn converted_cmyk_page() -> (Report, Vec<u8>) {
    let source = Conforming {
        contents: Some(paints_in_device_cmyk()),
        ..Conforming::part_two()
    }
    .build();
    let (report, output) = convert(&source, Target::Two(Level::B), Authorisations::default());
    let output = output.unwrap_or_else(|| panic!("{}", conversion(&report).render()));
    (report, output)
}

#[test]
fn the_devicen_default_is_reported_and_recorded_in_the_files_own_history() {
    // `doc/questions/A48` allows the construction on two conditions and this is both of them:
    // reported per document, and recorded in the file's own xmpMM:History naming the clause.
    // `doc/adr/0927`: what makes these permissions rather than a licence is that a reader can see
    // what was done, so a converter that met the first and not the second would have taken a
    // licence nobody granted.
    let (report, output) = converted_cmyk_page();
    let recorded = conversion(&report)
        .recorded
        .as_ref()
        .expect("the report names what was written into the history");
    assert!(
        recorded.contains("10.4.2.5"),
        "naming the clause: {recorded}"
    );
    let rendered = conversion(&report).render();
    assert!(
        rendered.contains("xmpMM:History"),
        "and the report a person reads says where it went:\n{rendered}"
    );

    // Its second: the file's own provenance carries it, naming the clause.
    let packet = String::from_utf8(document_packet(&output)).expect("the packet is UTF-8");
    assert!(
        packet.contains("xmpMM:History"),
        "the packet states a history:\n{packet}"
    );
    assert!(
        packet.contains("clause 10.4.2.5"),
        "and the entry names the clause that licensed the transform:\n{packet}"
    );
    let history = pdf_model::xmp::Xmp::parse_detail(packet.as_bytes()).expect("it parses");
    let entry = history
        .iter()
        .find(|property| {
            property.name.namespace == pdf_model::xmp::XMP_MM && property.name.local == "History"
        })
        .and_then(|property| property.value.array().and_then(<[_]>::first))
        .expect("one recorded action");
    for field in ["action", "parameters", "when"] {
        assert!(
            entry.field(pdf_model::xmp::RESOURCE_EVENT, field).is_some(),
            "ISO 19005-2 section 6.6.6 asks a recorded action for its {field}"
        );
    }
}

/// An ICC profile header naming a CMYK output profile, and nothing else.
///
/// ISO 15076-1's header is 128 bytes at the front of every profile, and three of its fields are
/// what ISO 19005-2 section 6.2.3 reads of a destination profile: the profile class at offset 12,
/// the data colour space at offset 16, and the `acsp` signature at offset 36 that says the bytes
/// are a profile at all. A tag count of zero follows it. That is a *fixture* rather than a usable
/// profile — no press is described by it — and it is enough for the question this test asks,
/// which is which of section 6.2.4.3's two licences a CMYK profile makes the converter take.
fn cmyk_header() -> std::sync::Arc<[u8]> {
    let mut out = vec![0u8; 132];
    out.splice(12..16, *b"prtr");
    out.splice(16..20, *b"CMYK");
    out.splice(20..24, *b"XYZ ");
    out.splice(36..40, *b"acsp");
    // The size field the header opens with, which a reader uses to bound the tag table.
    let size = u32::try_from(out.len()).unwrap_or(u32::MAX).to_be_bytes();
    out.splice(0..4, size);
    out.into()
}

/// Converts with a caller-supplied output intent profile.
fn convert_with_profile(
    bytes: &[u8],
    target: Target,
    profile: &std::sync::Arc<[u8]>,
) -> (Report, Option<Vec<u8>>) {
    let sinks = MemorySinks::new();
    let report = apply(
        &Plan::Archive(ArchivePlan {
            source: 0,
            names: "out.pdf".parse().expect("a pattern"),
            target,
            authorised: Authorisations::default(),
            profile: Some(std::sync::Arc::clone(profile)),
            substitute_fonts: true,
        }),
        &[Source::new(bytes.to_vec())],
        &sinks,
        &Policy::default(),
        &Budget::default(),
    )
    .expect("the conversion applies");
    let output = sinks.into_outputs().pop().map(|(_, bytes)| bytes);
    (report, output)
}

#[test]
fn a_cmyk_profile_answers_the_clause_and_no_default_is_written() {
    // The two licences ISO 19005-2 section 6.2.4.3 offers, and the ranking between them:
    // `doc/pdf-a-conversion-limits.md` section 10.1 says the correct answer is the owner's own
    // profile, so a supplied CMYK profile takes the output-intent route and the DeviceN default
    // — an approximation §10.4.2.1 calls crude — is not written at all.
    let source = Conforming {
        contents: Some(paints_in_device_cmyk()),
        ..Conforming::part_two()
    }
    .build();
    let (report, output) = convert_with_profile(&source, Target::Two(Level::B), &cmyk_header());
    let decided = decision(
        &report,
        "graphics/device-cmyk-needs-a-default-or-a-cmyk-output-intent",
    );
    assert_eq!(
        decided.rewrite(),
        Some(Rewrite::OutputIntent),
        "a CMYK destination profile answers the clause outright: {decided:?}"
    );
    let output = output.expect("the document converts");
    let document = Document::open_with_limits(output, Limits::DEFAULT).expect("it opens");
    let page = pdf_model::Pages::new(&document).get(0).expect("one page");
    let resources = document
        .get_key(&page.dict, "Resources")
        .as_dict()
        .cloned()
        .expect("the page's resources");
    assert!(
        document.get_key(&resources, "ColorSpace").is_null(),
        "nothing is changed that no failed requirement asked for"
    );
    assert!(
        conversion(&report).recorded.is_none(),
        "and nothing is recorded in the history, because nothing was interpreted"
    );
}

/// A PDF/A-2 fixture whose one font states `widths` and embeds its program only if `embedded`.
///
/// The same shape as [`a_font_whose_cmap_states`] one clause over, and the two differences are
/// what the font tests turn on: the `/Widths` array is the test's, and the descriptor's
/// `/FontFile2` is present or absent. The font is non-symbolic `/WinAnsiEncoding` and the page
/// draws the single code 0x41, which that encoding names `A` — so which glyph is meant is the
/// standard's answer rather than this fixture's.
///
/// 667 is the width Liberation Sans itself states for that glyph: 1366 units of a 2048-unit em
/// is 0.66699, which is inside the thousandth ISO 19005-2 section 6.2.11.5 allows. So a fixture
/// stating 667 asks for section 4.9's first metric route and one stating anything else asks for
/// the second.
fn a_font_stating(widths: &str, embedded: bool) -> Vec<u8> {
    let program = if embedded { "/FontFile2 8 0 R" } else { "" };
    Conforming {
        resources: "/Font << /F1 6 0 R >> /ColorSpace << /CS0 [/CalGray << /WhitePoint \
                    [0.9505 1.0 1.089] >>] >>"
            .to_owned(),
        contents: Some((
            String::new(),
            b"/CS0 cs 0 sc BT /F1 12 Tf 10 100 Td (A) Tj ET".to_vec(),
        )),
        objects: vec![
            format!(
                "<< /Type /Font /Subtype /TrueType /BaseFont /LiberationSans /FirstChar 65 \
                 /LastChar 65 /Widths [{widths}] /FontDescriptor 7 0 R \
                 /Encoding /WinAnsiEncoding >>"
            ),
            format!(
                "<< /Type /FontDescriptor /FontName /LiberationSans /Flags 32 \
                 /FontBBox [-543 -303 1300 980] /ItalicAngle 0 /Ascent 905 /Descent -212 \
                 /CapHeight 716 /StemV 80 {program} >>"
            ),
        ],
        binary_objects: vec![stream(
            &format!("/Length {}", LIBERATION_SANS.len()),
            LIBERATION_SANS,
        )],
        ..Conforming::part_two()
    }
    .build()
}

/// One font dictionary's `/Widths` array, read out of a converted file.
fn stated_widths(bytes: &[u8]) -> Vec<f64> {
    let held = Document::open_with_limits(bytes.to_vec(), Limits::DEFAULT).expect("it opens");
    let font = held
        .get(ObjectId::new(6, 0))
        .as_dict()
        .cloned()
        .expect("the font object");
    held.resolve(&held.get_key(&font, "Widths"))
        .as_array()
        .expect("a /Widths array")
        .iter()
        .filter_map(|entry| held.resolve(entry).as_number())
        .collect()
}

#[test]
fn a_font_the_file_never_embedded_is_given_a_face_the_report_names() {
    // ISO 19005-2 section 6.2.11.4.1 requires the program of every rendered font to be in the
    // file, and `doc/pdf-a-conversion-limits.md` section 4.9 makes embedding one of the faces
    // this program ships the **default** rather than a refusal — `doc/questions/A47` settled
    // that. The width the fixture states is the face's own, so this is section 4.9's *first*
    // metric route and nothing inside the program is touched.
    let source = a_font_stating("667", false);
    let (report, output) = convert(&source, Target::Two(Level::B), Authorisations::default());
    let decided = decision(&report, "fonts/font-programs-embedded");
    assert!(
        matches!(decided, Decision::Stated { .. }),
        "embedding a face states an interpretation rather than losing something: {decided:?}"
    );
    let output = output.expect("the document converts");
    assert_eq!(
        holds(&output, Target::Two(Level::B)).verdict(),
        Verdict::Conforms,
        "and what was written is held to the target again"
    );
    let substituted = &conversion(&report).substituted;
    assert_eq!(
        substituted.len(),
        1,
        "section 4.9 asks for one report line per font: {substituted:?}"
    );
    let font = substituted.first().expect("the one font");
    assert_eq!(
        (font.requested.as_str(), font.face, font.route),
        (
            "LiberationSans",
            "Liberation Sans Regular",
            pdf_transform::archive::MetricRoute::FaceMetrics
        ),
        "the face requested, the face used and which metric route was taken"
    );
    assert!(
        conversion(&report)
            .recorded
            .as_ref()
            .is_some_and(|recorded| recorded.contains("LiberationSans")),
        "ISO 19005-2 section 6.6.6's NOTE 1 names font substitution as an action to record in \
         the file's own xmpMM:History, and A47's permission is conditional on it"
    );
}

#[test]
fn an_embedded_face_whose_metrics_differ_has_the_program_restated_and_not_the_widths() {
    // `doc/pdf-a-conversion-limits.md` section 4.9's second metric route, and the third — "never"
    // — checked in the same breath. The fixture states a width the shipped face does not have, so
    // the face's advances are restated to it; what must *not* happen is the reverse, because
    // ISO 32000-2 §9.2.4 makes `/Widths` what positions the glyphs and rewriting it would move
    // every line of the page.
    let source = a_font_stating("500", false);
    let (report, output) = convert(&source, Target::Two(Level::B), Authorisations::default());
    let output = output.expect("the document converts");
    assert_eq!(
        holds(&output, Target::Two(Level::B)).verdict(),
        Verdict::Conforms,
        "the two statements of the advance now agree, which is what section 6.2.11.5 asks"
    );
    assert_eq!(
        conversion(&report)
            .substituted
            .first()
            .map(|font| font.route),
        Some(pdf_transform::archive::MetricRoute::RestatedProgram),
        "and the report says which of the two routes it was"
    );
    assert_eq!(
        stated_widths(&output),
        stated_widths(&source),
        "the widths the file states cross the conversion unchanged: section 4.9 calls restating \
         them **never**, because they are what positions the glyphs"
    );
}

#[test]
fn a_program_whose_advances_disagree_with_its_dictionary_is_restated_in_place() {
    // ISO 19005-2 section 6.2.11.5: the widths the font dictionary states and the ones the
    // embedded program states shall agree. This fixture embeds the real Liberation Sans and then
    // states 500 for a glyph the program makes 667 wide — the shape the corpus's own
    // `6.2.11.5` witnesses have — and the remedy is the program's number, never the file's.
    let source = a_font_stating("500", true);
    let (report, output) = convert(&source, Target::Two(Level::B), Authorisations::default());
    let decided = decision(&report, "fonts/widths-agree-with-the-program");
    assert_eq!(
        decided.rewrite(),
        Some(Rewrite::RestateFontMetrics),
        "restating a program's advances loses nothing and moves no mark: {decided:?}"
    );
    let output = output.expect("the document converts");
    assert_eq!(
        holds(&output, Target::Two(Level::B)).verdict(),
        Verdict::Conforms,
        "and what was written is held to the target again"
    );
    assert_eq!(
        stated_widths(&output),
        stated_widths(&source),
        "the dictionary's widths are untouched"
    );
    let restated = &conversion(&report).restated;
    assert_eq!(
        restated
            .first()
            .map(|font| (font.requested.as_str(), font.glyphs)),
        Some(("LiberationSans", 1)),
        "and the report names the font and how many glyphs were restated: {restated:?}"
    );
}

#[test]
fn no_substitute_turns_the_font_back_into_a_refusal_the_caller_can_take_back() {
    // `doc/pdf-a-conversion-limits.md` section 4.9: "--no-substitute for the user who wants the
    // other behaviour, which turns every such font back into a refusal." The reason has to say
    // which of the four kinds of *no* it is — this one is the caller's own, and it goes away the
    // moment the flag does, which is what separates it from a gap and from a fence.
    let source = a_font_stating("667", false);
    let sinks = MemorySinks::default();
    let report = apply(
        &Plan::Archive(ArchivePlan {
            source: 0,
            names: "out.pdf".parse().expect("a pattern"),
            target: Target::Two(Level::B),
            authorised: Authorisations::default(),
            profile: None,
            substitute_fonts: false,
        }),
        &[Source::new(source)],
        &sinks,
        &Policy::default(),
        &Budget::default(),
    )
    .expect("the conversion applies");
    let decided = conversion(&report)
        .decided
        .iter()
        .find(|decided| decided.requirement == "fonts/font-programs-embedded")
        .map(|decided| decided.decision)
        .expect("the embedding requirement is decided about");
    assert!(
        matches!(decided, Decision::Refused(Because::Declined(_))),
        "the caller asked for this, so it is not a gap and not a fence: {decided:?}"
    );
    assert!(
        sinks.into_outputs().is_empty(),
        "and no file is written, because the document does not conform without the face"
    );
}

/// The TrueType program the Unicode tests embed.
///
/// A real font is needed rather than a plausible one, because the derivation refuses a font
/// `pdf-font` had to substitute for: the glyph names would then be the substitute's, and what
/// ISO 32000-2 §9.10.2's second method asks for is the name *this file's* font selects.
/// `data/standard-fonts/PROVENANCE.md` and `/NOTICE` record the licence, which is the SIL OFL
/// 1.1 and permits embedding.
const LIBERATION_SANS: &[u8] =
    include_bytes!("../../../data/standard-fonts/LiberationSans-Regular.ttf");

/// A PDF/A-2 fixture whose one font states the `/Encoding` and the `/ToUnicode` `CMap` given.
///
/// Non-symbolic, and drawing the single code 0x41. That flag is ISO 19005-2 section
/// 6.2.11.7.2's fourth exemption, so `fonts/to-unicode-present` does not bind the font — which
/// leaves the *values* rule, the one sentence of the subclause stated with no exemption at all,
/// as the only thing the fixture fails.
///
/// The page selects a `CalGray` before it shows the text: §8.6.5.2 makes that colour space
/// device independent, so the fixture does not trip section 6.2.4.3's rule about `DeviceGray`
/// and the conversion has exactly one requirement to answer.
fn a_font_whose_cmap_states(encoding: &str, mapping: &str) -> Vec<u8> {
    let cmap = format!(
        "/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n\
         /CMapName /Test def\n/CMapType 2 def\n\
         1 begincodespacerange\n<00> <FF>\nendcodespacerange\n\
         1 beginbfchar\n{mapping}\nendbfchar\n\
         endcmap\nCMapName currentdict /CMap defineresource pop\nend\nend\n"
    );
    Conforming {
        resources: "/Font << /F1 6 0 R >> /ColorSpace << /CS0 [/CalGray << /WhitePoint \
                    [0.9505 1.0 1.089] >>] >>"
            .to_owned(),
        contents: Some((
            String::new(),
            b"/CS0 cs 0 sc BT /F1 12 Tf 10 100 Td (A) Tj ET".to_vec(),
        )),
        objects: vec![
            format!(
                "<< /Type /Font /Subtype /TrueType /BaseFont /LiberationSans /FirstChar 65 \
                 /LastChar 65 /Widths [667] /FontDescriptor 7 0 R /Encoding {encoding} \
                 /ToUnicode 8 0 R >>"
            ),
            "<< /Type /FontDescriptor /FontName /LiberationSans /Flags 32 \
             /FontBBox [-543 -303 1300 980] /ItalicAngle 0 /Ascent 905 /Descent -212 \
             /CapHeight 716 /StemV 80 /FontFile2 9 0 R >>"
                .to_owned(),
        ],
        binary_objects: vec![
            stream(&format!("/Length {}", cmap.len()), cmap.as_bytes()),
            stream(
                &format!("/Length {}", LIBERATION_SANS.len()),
                LIBERATION_SANS,
            ),
        ],
        ..Conforming::part_two()
    }
    .build()
}

#[test]
fn a_placeholder_unicode_value_is_replaced_by_the_one_the_encoding_derives() {
    // ISO 19005-2 section 6.2.11.7.2's last sentence: the values a `/ToUnicode` CMap states
    // "shall all be greater than zero (0), but not equal to either U+FEFF or U+FFFE". This
    // fixture writes U+0000 for the one code it draws, and the derivation answers it the way
    // §9.10.2's second method does — `/WinAnsiEncoding` names code 0x41 `A`, and the Adobe Glyph
    // List makes that U+0041. Nothing is guessed: both halves are tables the standard prints.
    let source = a_font_whose_cmap_states("/WinAnsiEncoding", "<41> <0000>");
    let (report, output) = convert(&source, Target::Two(Level::U), Authorisations::default());
    let decided = decision(&report, "fonts/to-unicode-values-are-usable");
    assert_eq!(
        decided.rewrite(),
        Some(Rewrite::ToUnicode),
        "a code the font's own encoding names is a code the CMap can be derived for: {decided:?}"
    );
    let output = output.expect("the document converts");
    assert_eq!(
        holds(&output, Target::Two(Level::U)).verdict(),
        Verdict::Conforms,
        "and what was written is held to the target again"
    );
    let derived = derived_cmap(&output);
    assert!(
        derived.contains("<41>") && derived.contains("<0041>"),
        "the derived CMap maps the drawn code to the character its glyph name stands for: \
         {derived}"
    );
}

#[test]
fn a_usable_value_the_producer_wrote_is_kept_rather_than_re_derived() {
    // The producer's own statement about their own file, which this converter has no better
    // evidence than. `/WinAnsiEncoding` would derive U+0041 for code 0x41; the fixture says the
    // code means U+00C4 instead, which is a usable value, so the derived CMap carries it across
    // unchanged and only the *second* code — the one whose value is a placeholder — is answered
    // from the encoding.
    let source = a_font_whose_cmap_states("/WinAnsiEncoding", "<41> <00C4>");
    let (report, output) = convert(&source, Target::Two(Level::U), Authorisations::default());
    assert!(
        conversion(&report)
            .decided
            .iter()
            .all(|decided| decided.requirement != "fonts/to-unicode-values-are-usable"),
        "a usable value fails nothing, so nothing is decided about it"
    );
    let output = output.expect("the document converts");
    let held = Document::open_with_limits(output, Limits::DEFAULT).expect("it opens");
    let font = held
        .get(ObjectId::new(6, 0))
        .as_dict()
        .cloned()
        .expect("the font object");
    assert!(
        matches!(
            held.get_key(&font, "ToUnicode"),
            pdf_syntax::Object::Stream(_)
        ),
        "the producer's own CMap is still the one the font names"
    );
}

#[test]
fn a_code_whose_glyph_name_is_nobodys_refuses_rather_than_inventing_a_meaning() {
    // `doc/pdf-a-conversion-limits.md` section 4.3: a code whose meaning is not derivable is a
    // refusal, not an invention. The fixture's `/Differences` gives code 0x41 a subsetter's
    // private label, which is in neither list ISO 19005-2 section 6.2.11.7.2's second exemption
    // names — so the name says which glyph was drawn and not which character it stands for.
    let source = a_font_whose_cmap_states(
        "<< /Type /Encoding /BaseEncoding /WinAnsiEncoding /Differences [65 /g4711] >>",
        "<41> <0000>",
    );
    let (report, output) = convert(&source, Target::Two(Level::U), Authorisations::default());
    let decided = decision(&report, "fonts/to-unicode-values-are-usable");
    assert!(
        matches!(decided, Decision::Refused(Because::NotBuiltYet(_))),
        "a private glyph name derives nothing: {decided:?}"
    );
    assert!(
        output.is_none(),
        "and no file is written wearing a claim it has not earned"
    );
}

/// The `/ToUnicode` `CMap` the font in a converted output names, as text.
fn derived_cmap(output: &[u8]) -> String {
    let held = Document::open_with_limits(output.to_vec(), Limits::DEFAULT).expect("it opens");
    let mut fonts = Vec::new();
    for id in held.xref().object_numbers() {
        let object = held.get(ObjectId::new(id, 0));
        let Some(dict) = object.as_dict() else {
            continue;
        };
        if held
            .get_key(dict, "Type")
            .as_name()
            .map(pdf_syntax::Name::as_bytes)
            == Some(b"Font")
            && let pdf_syntax::Object::Stream(stream) = held.get_key(dict, "ToUnicode")
            && let Some(data) = held.decoded_stream_data(&stream)
        {
            fonts.push(String::from_utf8_lossy(&data).into_owned());
        }
    }
    fonts
        .pop()
        .expect("the converted font names a ToUnicode CMap")
}

#[test]
fn a_tagged_source_is_declared_level_a_and_an_untagged_one_is_refused() {
    // `doc/pdf-a-conversion-limits.md` section 5.1: the converter will not invent a structure
    // tree, **and that is not the same as refusing PDF/A-2a**. ISO 19005-2 section 6.7.2.2 asks
    // the catalog for `/MarkInfo` with `/Marked true`, and a file that already states a
    // `/StructTreeRoot` has demonstrated the conventions the flag claims — so the flag is
    // written down rather than made up. A file with no tree gets the sentence instead.
    let tagged = Conforming {
        catalog: "/StructTreeRoot 6 0 R".to_owned(),
        objects: vec!["<< /Type /StructTreeRoot /K [] >>".to_owned()],
        ..Conforming::part_two()
    }
    .build();
    let (report, output) = convert(&tagged, Target::Two(Level::A), Authorisations::default());
    assert_eq!(
        decision(&report, "logical-structure/mark-info-marked").rewrite(),
        Some(Rewrite::MarkInfo),
        "the tree is there and the flag is not"
    );
    let output = output.expect("a tagged document converts to Level A");
    assert_eq!(
        holds(&output, Target::Two(Level::A)).verdict(),
        Verdict::Conforms,
        "and what was written is held to Level A again"
    );

    let untagged = Conforming::part_two().build();
    let (report, output) = convert(&untagged, Target::Two(Level::A), Authorisations::default());
    assert!(
        matches!(
            decision(&report, "logical-structure/mark-info-marked"),
            Decision::Refused(Because::TheFence(_))
        ),
        "and a file with no tree is not told the flag will arrive in a later slice"
    );
    assert!(
        output.is_none(),
        "no file wearing a claim it has not earned"
    );
}

#[test]
fn an_embedded_files_specification_gains_the_two_keys_its_own_entries_derive() {
    // ISO 19005-4 section 6.9 requires `/F`, `/UF` and `/AFRelationship` of every embedded
    // file's specification, and Annex A keeps all three for PDF/A-4f. Both rewrites write down
    // something already stated: §7.11.3's Table 43 makes `/F` and `/UF` the same file name in
    // two types, and gives `/AFRelationship` the default `Unspecified` — so a specification
    // without the entry already relates to the document in exactly the way the name records.
    let source = Conforming {
        catalog: "/Names << /EmbeddedFiles << /Names [(note.txt) 6 0 R] >> >>".to_owned(),
        objects: vec!["<< /Type /Filespec /F (note.txt) /EF << /F 7 0 R >> >>".to_owned()],
        binary_objects: vec![stream(
            "/Type /EmbeddedFile /Subtype /text#2Fplain /Length 5",
            b"hello",
        )],
        ..Conforming::default()
    }
    .build();
    let (report, output) = convert(&source, Target::Four(Flavour::F), Authorisations::default());
    assert_eq!(
        decision(&report, "embedded-files/file-and-unicode-names").rewrite(),
        Some(Rewrite::AssociatedFileNames),
        "the name is in one key and the other's value is that same name"
    );
    assert_eq!(
        decision(&report, "embedded-files/relationship-stated").rewrite(),
        Some(Rewrite::AssociatedFileRelationship),
        "and Table 43's own default is what the absent entry already meant"
    );
    let output = output.expect("the document converts");
    assert_eq!(
        holds(&output, Target::Four(Flavour::F)).verdict(),
        Verdict::Conforms,
        "and what was written is held to PDF/A-4f again"
    );
    let held = Document::open_with_limits(output, Limits::DEFAULT).expect("it opens");
    // The output is a fresh file, so the specification is found by what it is rather than by the
    // number the source gave it.
    let spec = held
        .xref()
        .object_numbers()
        .filter_map(|number| held.get(ObjectId::new(number, 0)).as_dict().cloned())
        .find(|dict| !held.get_key(dict, "EF").is_null())
        .expect("the file specification");
    assert_eq!(
        held.get_key(&spec, "UF").as_string().map(<[u8]>::to_vec),
        Some(b"note.txt".to_vec()),
        "the Unicode name is the one the byte string already held"
    );
    assert_eq!(
        held.get_key(&spec, "AFRelationship")
            .as_name()
            .map(|name| name.as_bytes().to_vec()),
        Some(b"Unspecified".to_vec()),
        "and the relationship is the entry's own default rather than a guess"
    );
}

#[test]
fn a_document_with_nothing_embedded_cannot_be_told_it_will_be_pdfa_four_f_later() {
    // ISO 19005-4 Annex A.2 makes the `/EmbeddedFiles` key required, which is the one
    // requirement in either part a document can fail by holding nothing at all. Attaching a file
    // would be adding content no source states, so the answer is `NotThisTarget` — the class
    // `doc/pdf-a-conversion-limits.md` section 2.0 exists to keep apart from "not yet", because
    // a user told the second comes back tomorrow for the same answer.
    let source = Conforming::default().build();
    let (report, output) = convert(&source, Target::Four(Flavour::F), Authorisations::default());
    assert!(
        matches!(
            decision(&report, "embedded-files/pdfa-4f-carries-embedded-files"),
            Decision::Refused(Because::NotThisTarget(_))
        ),
        "the target is what is wrong, and PDF/A-4 is what this document is for"
    );
    assert!(output.is_none(), "and no file is written");
}

// --------------------------------------------------------------------------------------------
// `doc/pdf-a-mitigations.md` section 13.3 — *owed, not optional*: the refusals whose right answer
// loses nothing. Each test below asks the same three questions of one of them: was the decision
// the class `doc/adr/0948` says it is, does the output conform when it is carried out, and is
// what the file now says the sentence the standard states rather than a choice this program made.
// --------------------------------------------------------------------------------------------

/// Converts to PDF/A-2b with nothing authorised, which is where part 2's own rules are asked.
fn to_part_two(bytes: &[u8]) -> (Report, Option<Vec<u8>>) {
    convert(bytes, Target::Two(Level::B), Authorisations::default())
}

#[test]
fn a_rendering_intent_the_base_standard_does_not_define_is_restated_as_the_one_it_names() {
    // ISO 19005-2 section 6.2.6 admits only the four intents ISO 32000-2 §8.6.5.8 defines, and
    // that subclause says what a processor does with any other name: "If a PDF processor does
    // not recognise the specified name, it shall use the RelativeColorimetric intent by
    // default." So the value written is the standard's own answer rather than a choice, which is
    // what puts the decision in `doc/adr/0948`'s `Stated` class.
    //
    // The graphics state is written *inside* the page's resource dictionary, which is the shape
    // that matters: `pdf_archive` reports a dictionary at the object it is written in, so the
    // finding names the page and the entry is two levels down inside it.
    let source = Conforming {
        resources: "/ExtGState << /GS0 << /Type /ExtGState /RI /Cheerful >> >>".to_owned(),
        ..Conforming::part_two()
    }
    .build();
    let (report, output) = to_part_two(&source);
    assert!(matches!(
        decision(
            &report,
            "graphics/rendering-intent-entries-name-one-of-four"
        ),
        Decision::Stated {
            rewrite: Rewrite::RenderingIntent,
            ..
        }
    ));
    let output = output.expect("nothing is lost, so nothing is authorised");
    let held = holds(&output, Target::Two(Level::B));
    assert_eq!(held.verdict(), Verdict::Conforms, "{}", held.render());
    let written = String::from_utf8_lossy(&output).into_owned();
    assert!(
        written.contains("/RI /RelativeColorimetric"),
        "the entry names the intent §8.6.5.8 supplies: {written}"
    );
    assert!(!written.contains("Cheerful"), "and the old name is gone");
}

#[test]
fn an_optional_content_groups_intent_is_left_alone_by_the_rendering_intent_rewrite() {
    // §8.11.2.3 gives an optional content group an `/Intent` whose values are `View` and
    // `Design`, and §8.9.5.1's Table 87 gives an image `XObject` an `/Intent` that is a rendering
    // intent. One key, two meanings — so the rewrite is guarded by `/Subtype /Image`, which is
    // the validator's own test, and a group's entry is none of its business.
    let source = Conforming {
        catalog: "/OCProperties << /OCGs [6 0 R] /D << /Name (Default) /Order [6 0 R] >> >>"
            .to_owned(),
        resources: "/ExtGState << /GS0 << /Type /ExtGState /RI /Cheerful >> >>".to_owned(),
        objects: vec!["<< /Type /OCG /Name (Layer) /Intent /Design >>".to_owned()],
        ..Conforming::part_two()
    }
    .build();
    let (_, output) = to_part_two(&source);
    let output = output.expect("the document converts");
    let written = String::from_utf8_lossy(&output).into_owned();
    assert!(
        written.contains("/Intent /Design"),
        "the group's own intent is untouched: {written}"
    );
}

#[test]
fn a_blend_mode_array_naming_nothing_the_standard_defines_becomes_normal() {
    // ISO 19005-2 section 6.2.10 and ISO 19005-4 section 6.2.9 require a defined blend mode.
    // §8.4.1's Table 57 says what a reader takes from an array: "In the latter case, the PDF
    // reader shall use the first blend mode in the array that it recognises (or Normal if it
    // recognises none of them)." So an array with nothing recognised in it already *is* Normal
    // to every conforming reader, and writing the name down composites nothing differently.
    let source = Conforming {
        resources: "/ExtGState << /GS0 << /Type /ExtGState /BM [/Sepia /Woodcut] >> >>".to_owned(),
        ..Conforming::default()
    }
    .build();
    let (report, output) = to_part_four(&source);
    assert!(matches!(
        decision(&report, "graphics/graphics-state-blend-modes-are-defined"),
        Decision::Stated {
            rewrite: Rewrite::BlendModeNormal,
            ..
        }
    ));
    let output = output.expect("nothing is lost, so nothing is authorised");
    let held = holds(&output, Target::Four(Flavour::Plain));
    assert_eq!(held.verdict(), Verdict::Conforms, "{}", held.render());
    let written = String::from_utf8_lossy(&output).into_owned();
    assert!(written.contains("/BM /Normal"), "{written}");
    assert!(!written.contains("Sepia"), "and the array is gone");
}

#[test]
fn a_blend_mode_written_as_a_bare_name_is_refused_because_no_clause_says_what_it_means() {
    // The other half of the same requirement, and `doc/pdf-a-mitigations.md` section 4.6 is why
    // it stays refused: Table 57 says what a reader does with an unrecognised name *in an array*
    // and says nothing at all about one on its own. Writing a mode in its place would decide how
    // these marks composite with what is under them, which `doc/questions/A48` forbids.
    let source = Conforming {
        resources: "/ExtGState << /GS0 << /Type /ExtGState /BM /Sepia >> >>".to_owned(),
        ..Conforming::default()
    }
    .build();
    let (report, output) = to_part_four(&source);
    assert!(
        matches!(
            decision(&report, "graphics/graphics-state-blend-modes-are-defined"),
            Decision::Refused(Because::NotBuiltYet(_))
        ),
        "the shape with no answer in the standard keeps the refusal"
    );
    assert!(output.is_none(), "and no file is written");
}

#[test]
fn an_appearance_subdictionary_collapses_to_the_state_the_annotation_selects() {
    // ISO 19005-2 section 6.3.3 and ISO 19005-4 section 6.3.3 require an annotation that is not
    // a button widget to state its normal appearance as a stream. §12.5.2's Table 166 makes
    // `/AS` "[t]he annotation's appearance state , which selects the applicable appearance
    // stream from an appearance subdictionary", so the stream written is the one the reader was
    // drawing already.
    let source = Conforming {
        page: "/Annots [6 0 R]".to_owned(),
        objects: vec![
            "<< /Type /Annot /Subtype /Stamp /Rect [10 10 90 90] /F 4 /AS /On \
             /AP << /N << /On 7 0 R /Off 8 0 R >> >> >>"
                .to_owned(),
            "<< /Type /XObject /Subtype /Form /BBox [0 0 80 80] /Length 0 >>\nstream\n\nendstream"
                .to_owned(),
            "<< /Type /XObject /Subtype /Form /BBox [0 0 80 80] /Length 0 >>\nstream\n\nendstream"
                .to_owned(),
        ],
        ..Conforming::default()
    }
    .build();
    let (report, output) = to_part_four(&source);
    assert!(matches!(
        decision(&report, "annotations/normal-appearance-shape"),
        Decision::Stated {
            rewrite: Rewrite::NormalAppearanceFromState,
            ..
        }
    ));
    let output = output.expect("nothing is lost, so nothing is authorised");
    let held = holds(&output, Target::Four(Flavour::Plain));
    assert_eq!(held.verdict(), Verdict::Conforms, "{}", held.render());
}

#[test]
fn an_appearance_subdictionary_with_no_state_stays_refused() {
    // The half `doc/pdf-a-mitigations.md` section 6 leaves without an answer: with no `/AS`,
    // nothing in the file says which state the document is in, and choosing one is the half of
    // `doc/questions/A48`'s line this converter does not cross.
    let source = Conforming {
        page: "/Annots [6 0 R]".to_owned(),
        objects: vec![
            "<< /Type /Annot /Subtype /Stamp /Rect [10 10 90 90] /F 4 \
             /AP << /N << /On 7 0 R >> >> >>"
                .to_owned(),
            "<< /Type /XObject /Subtype /Form /BBox [0 0 80 80] /Length 0 >>\nstream\n\nendstream"
                .to_owned(),
        ],
        ..Conforming::default()
    }
    .build();
    let (report, output) = to_part_four(&source);
    assert!(matches!(
        decision(&report, "annotations/normal-appearance-shape"),
        Decision::Refused(Because::NotBuiltYet(_))
    ));
    assert!(output.is_none(), "and no file is written");
}

#[test]
fn an_order_array_gains_the_groups_the_producer_left_out_of_it() {
    // ISO 19005-2 section 6.9 and ISO 19005-4 section 6.10 require the array to reference every
    // optional content group the file states. §8.11.4.3's Table 99 says what the entry decides,
    // and it is a panel rather than a page: "Any groups not listed in this array shall not be
    // presented in any user interface that uses the configuration." The groups and their order
    // are both the file's own, so nothing is invented.
    let source = Conforming {
        catalog: "/OCProperties << /OCGs [6 0 R 7 0 R] \
                   /D << /Name (Default) /Order [6 0 R] >> >>"
            .to_owned(),
        objects: vec![
            "<< /Type /OCG /Name (First) >>".to_owned(),
            "<< /Type /OCG /Name (Second) >>".to_owned(),
        ],
        ..Conforming::default()
    }
    .build();
    let (report, output) = to_part_four(&source);
    assert_eq!(
        decision(&report, "optional-content/order-lists-every-group"),
        Decision::Mechanical(Rewrite::OptionalContentOrder),
        "nothing is lost and nothing is reinterpreted: the groups are already in the file"
    );
    let output = output.expect("the document converts");
    let held = holds(&output, Target::Four(Flavour::Plain));
    assert_eq!(held.verdict(), Verdict::Conforms, "{}", held.render());
}

#[test]
fn a_signature_widgets_missing_flags_are_answered_by_the_annotation_rule_that_states_them() {
    // ISO 19005-2 section 6.4.3 and ISO 19005-4 section 6.5.1 ask the annotation flag and
    // appearance rules again of a signature field's widget. `doc/pdf-a-mitigations.md` section 7
    // records that a document refused on this row was one the conversion would in fact have
    // fixed, because the rules it names are rows of the same table — so the answer is theirs,
    // which for a widget stating no `/F` is section 3.7's authorised loss.
    let widget = "<< /Type /Annot /Subtype /Widget /FT /Sig /T (Signature) \
                  /Rect [10 10 90 90] /AP << /N 7 0 R >> >>";
    let appearance =
        "<< /Type /XObject /Subtype /Form /BBox [0 0 80 80] /Length 0 >>\nstream\n\nendstream";
    let source = Conforming {
        catalog: "/AcroForm << /Fields [6 0 R] >>".to_owned(),
        page: "/Annots [6 0 R]".to_owned(),
        objects: vec![widget.to_owned(), appearance.to_owned()],
        ..Conforming::default()
    }
    .build();
    let (report, output) = to_part_four(&source);
    assert_eq!(
        decision(
            &report,
            "signatures/signature-widgets-meet-the-annotation-rules"
        ),
        Decision::Unauthorised {
            loss: Loss::AnnotationPrinting,
            rewrite: Rewrite::AnnotationFlags,
        },
        "the compound row takes the most constraining of the rules it names"
    );
    assert!(output.is_none(), "unauthorised, so nothing is written");

    let authorised = Authorisations {
        image_smoothing: false,
        metadata_property: false,
        annotation_printing: true,
        jpeg2000_colour_fallback: false,
    };
    let (report, output) = convert(&source, Target::Four(Flavour::Plain), authorised);
    assert_eq!(
        decision(
            &report,
            "signatures/signature-widgets-meet-the-annotation-rules"
        ),
        Decision::Authorised {
            loss: Loss::AnnotationPrinting,
            rewrite: Rewrite::AnnotationFlags,
        }
    );
    let output = output.expect("authorised, so it converts");
    let held = holds(&output, Target::Four(Flavour::Plain));
    assert_eq!(held.verdict(), Verdict::Conforms, "{}", held.render());
}

// ---------------------------------------------------------------------------------------------
// `doc/pdf-a-mitigations.md` section 13.3's *owed, not optional*, session 962's four.
// ---------------------------------------------------------------------------------------------

#[test]
fn the_catalogs_needs_rendering_is_removed() {
    // ISO 19005-2 section 6.4.2 and ISO 19005-4 section 6.4.2 forbid the key. §7.7.2's Table 29
    // is why removing it loses nothing: the entry is deprecated in PDF 2.0, its subject is the
    // XFA form, and its "Default value: false" is what an absent entry states. This fixture
    // states no /XFA, which is the other half of the same subclause and is still refused — so
    // there is no form for any reader to regenerate.
    let source = Conforming {
        catalog: "/NeedsRendering true".to_owned(),
        ..Conforming::default()
    }
    .build();
    let (report, output) = to_part_four(&source);
    assert_eq!(
        decision(&report, "forms/no-needs-rendering"),
        Decision::Mechanical(Rewrite::NeedsRendering)
    );
    let output = output.expect("the document converts");
    let held = holds(&output, Target::Four(Flavour::Plain));
    assert_eq!(held.verdict(), Verdict::Conforms, "{}", held.render());
    assert!(
        !String::from_utf8_lossy(&output).contains("NeedsRendering"),
        "the key is gone from the file and not only from the verdict"
    );
}

#[test]
fn a_packet_header_loses_its_deprecated_attributes_and_every_other_byte_crosses() {
    // ISO 19005-2 section 6.6.2.1 and ISO 19005-4 section 6.7.2.1 forbid the `bytes` and
    // `encoding` attributes of the packet's own processing instruction. Each describes the
    // packet's framing rather than anything inside it, so what has to be true afterwards is
    // both that they are gone and that the RDF the producer wrote is byte for byte what it was.
    let packet = identification(Target::Four(Flavour::Plain)).replace(
        "id=\"W5M0MpCehiHzreSzNTczkc9d\"?>",
        "id=\"W5M0MpCehiHzreSzNTczkc9d\" bytes=\"1234\" encoding=\"UTF-8\"?>",
    );
    let source = Conforming {
        metadata: Packet::Stated(packet),
        ..Conforming::default()
    }
    .build();
    let (report, output) = to_part_four(&source);
    assert_eq!(
        decision(&report, "metadata/xmp-packet-header-attributes"),
        Decision::Mechanical(Rewrite::PacketHeaderAttributes)
    );
    let output = output.expect("the document converts");
    let held = holds(&output, Target::Four(Flavour::Plain));
    assert_eq!(held.verdict(), Verdict::Conforms, "{}", held.render());
    let written = String::from_utf8_lossy(&output).into_owned();
    assert!(
        !written.contains("bytes=") && !written.contains("encoding="),
        "both attributes are cut out of the processing instruction"
    );
    assert!(
        written.contains("<?xpacket begin=\"\u{feff}\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?>"),
        "and the rest of the header is the producer's, with the white space in front of each \
         cut attribute taken with it"
    );
    assert!(
        written.contains("<pdfaid:part>4</pdfaid:part>"),
        "the RDF the header wrapped is untouched"
    );
}

/// A PDF/A-2 fixture whose one embedded TrueType font states `flags` and `encoding`, drawing
/// `text`.
///
/// The same shape as [`a_font_stating`], with the two entries the encoding rules turn on made
/// the test's: §9.8.1's Table 122 bit 3 is the symbolic flag — 4 set, 32 for non-symbolic — and
/// §9.6.5.1's `/Encoding` is the entry ISO 19005 restricts. The program is the real Liberation
/// Sans, because a substituted face is one the preparation declines to reason about.
///
/// `draws` is `None` for a font the file carries and no content stream shows anything with,
/// which is a shape both encoding rules still bind: they walk the *font objects*, and a font
/// nothing draws with is one whose encoding no mark depends on.
fn a_truetype_font(flags: &str, encoding: &str, draws: Option<&str>) -> Vec<u8> {
    Conforming {
        resources: "/Font << /F1 6 0 R >> /ColorSpace << /CS0 [/CalGray << /WhitePoint \
                    [0.9505 1.0 1.089] >>] >>"
            .to_owned(),
        contents: Some((
            String::new(),
            draws.map_or_else(Vec::new, |text| {
                format!("/CS0 cs 0 sc BT /F1 12 Tf 10 100 Td ({text}) Tj ET").into_bytes()
            }),
        )),
        objects: vec![
            format!(
                "<< /Type /Font /Subtype /TrueType /BaseFont /LiberationSans /FirstChar 0 \
                 /LastChar 255 /Widths [{}] /FontDescriptor 7 0 R {encoding} >>",
                vec!["667"; 256].join(" ")
            ),
            format!(
                "<< /Type /FontDescriptor /FontName /LiberationSans /Flags {flags} \
                 /FontBBox [-543 -303 1300 980] /ItalicAngle 0 /Ascent 905 /Descent -212 \
                 /CapHeight 716 /StemV 80 /FontFile2 8 0 R >>"
            ),
        ],
        binary_objects: vec![stream(
            &format!("/Length {}", LIBERATION_SANS.len()),
            LIBERATION_SANS,
        )],
        ..Conforming::part_two()
    }
    .build()
}

/// One font dictionary's `/Encoding`, read out of a converted file as it is written.
fn stated_encoding(bytes: &[u8]) -> Option<String> {
    let held = Document::open_with_limits(bytes.to_vec(), Limits::DEFAULT).expect("it opens");
    let font = held
        .get(ObjectId::new(6, 0))
        .as_dict()
        .cloned()
        .expect("the font object");
    held.resolve(font.get("Encoding")?)
        .as_name()
        .map(|name| String::from_utf8_lossy(name.as_bytes()).into_owned())
}

#[test]
fn a_symbolic_truetype_fonts_encoding_is_removed_where_no_shown_code_moves() {
    // ISO 19005-2 section 6.2.11.6 and ISO 19005-4 section 6.2.10.6 forbid a symbolic TrueType
    // font from stating an `/Encoding` at all. §9.6.5.4 makes the entry decide which `cmap`
    // subtable a code is looked up through, so the removal is mechanical only once the glyph
    // every shown code reaches has been shown to be the same without it.
    //
    // **The fixture's font draws nothing**, and that is the face's doing rather than the
    // rewrite's: the same subclause requires a *rendered* symbolic font's program to carry a
    // `cmap` subtable of the kind the part names, Liberation Sans carries the Microsoft Unicode
    // one and not that, and that rule is behind ADR 0816's fence — so a rendered symbolic
    // fixture built from this face is refused before this row is reached. What is left is the
    // case the proof answers vacuously and correctly: a font the file carries whose encoding no
    // mark on any page depends on.
    let source = a_truetype_font("4", "/Encoding /WinAnsiEncoding", None);
    let (report, output) = convert(&source, Target::Two(Level::B), Authorisations::default());
    assert_eq!(
        decision(&report, "fonts/symbolic-truetype-states-no-encoding"),
        Decision::Mechanical(Rewrite::SymbolicTrueTypeEncodingRemoved)
    );
    let output = output.expect("the document converts");
    let held = holds(&output, Target::Two(Level::B));
    assert_eq!(held.verdict(), Verdict::Conforms, "{}", held.render());
    assert_eq!(
        stated_encoding(&output),
        None,
        "the entry is gone from the font dictionary"
    );
}

#[test]
fn a_non_symbolic_truetype_font_is_given_a_name_the_part_admits() {
    // The same subclause's other row: a non-symbolic TrueType font's encoding shall be
    // `MacRomanEncoding` or `WinAnsiEncoding`. This one states `StandardEncoding` and draws the
    // single code 0x41, which all three encodings name `A` — so the proof passes and the first
    // of the two admitted names is written.
    let source = a_truetype_font("32", "/Encoding /StandardEncoding", Some("A"));
    let (report, output) = convert(&source, Target::Two(Level::B), Authorisations::default());
    assert_eq!(
        decision(
            &report,
            "fonts/non-symbolic-truetype-uses-a-standard-encoding"
        ),
        Decision::Mechanical(Rewrite::StandardTrueTypeEncoding)
    );
    let output = output.expect("the document converts");
    let held = holds(&output, Target::Two(Level::B));
    assert_eq!(held.verdict(), Verdict::Conforms, "{}", held.render());
    assert_eq!(
        stated_encoding(&output).as_deref(),
        Some("WinAnsiEncoding"),
        "the name written is one of the two the part admits"
    );
}

#[test]
fn a_non_symbolic_truetype_font_whose_shown_code_would_move_is_refused() {
    // The proof is what makes the row above mechanical, so it has to bite. Code 0xE9 is `eacute`
    // under `WinAnsiEncoding` and `Odieresis` under `StandardEncoding`, and Liberation Sans has
    // both glyphs — so writing either admitted name over this file's encoding would change what
    // the page draws, and `doc/adr/0947`'s first rule makes that a refusal rather than a plan.
    let source = a_truetype_font("32", "/Encoding /StandardEncoding", Some("\\351"));
    let (report, output) = convert(&source, Target::Two(Level::B), Authorisations::default());
    let decided = decision(
        &report,
        "fonts/non-symbolic-truetype-uses-a-standard-encoding",
    );
    assert!(
        matches!(decided, Decision::Refused(Because::NotBuiltYet(_))),
        "a rewrite that would move a mark is refused with the preparation's own sentence: \
         {decided:?}"
    );
    assert!(output.is_none(), "and no file claims a conformance");
}

// --------------------------------------------------------------------------------------------
// ISO 19005-2 section 6.2.3 and ISO 19005-4 section 6.2.3: one destination profile per array.
//
// The clause's own note says where this arises — a file conforming to ISO 19005 and to PDF/X or
// PDF/E at the same time carries one output intent per standard — and the two entries usually
// carry the same profile twice. That is the case these two tests separate: the same profile in
// two objects is one object's worth of information, and two different profiles are two
// destinations of which only one can survive.
// --------------------------------------------------------------------------------------------

/// The ICC profile this program ships, for a fixture that needs a real one.
///
/// A synthetic header is enough where a test asks only which colour family a profile is, and it
/// is not enough here: the output has to be held to every one of section 6.2.3's other rows, and
/// those read the profile's device class, its colour space, its required tags and its own
/// identifier.
const SRGB: &[u8] = include_bytes!("../../../data/icc/sRGB2014.icc");

/// A fixture whose `OutputIntents` array holds a PDF/A entry and a PDF/X one, each naming its
/// own copy of a destination profile.
///
/// §14.11.5's Table 400 and Table 401 shape both entries. The profile streams are objects 8 and
/// 9, and `second` is what object 9 carries — the same bytes for the lossless case, and
/// something else for the case that is not.
fn two_output_intents(second: &[u8]) -> Vec<u8> {
    let profile = |bytes: &[u8]| stream(&format!("/N 3 /Length {}", bytes.len()), bytes);
    Conforming {
        catalog: "/OutputIntents [6 0 R 7 0 R]".to_owned(),
        objects: vec![
            "<< /Type /OutputIntent /S /GTS_PDFA1 /OutputConditionIdentifier (Custom) \
             /DestOutputProfile 8 0 R >>"
                .to_owned(),
            "<< /Type /OutputIntent /S /GTS_PDFX /OutputConditionIdentifier (Custom) \
             /DestOutputProfile 9 0 R >>"
                .to_owned(),
        ],
        binary_objects: vec![profile(SRGB), profile(second)],
        ..Conforming::default()
    }
    .build()
}

#[test]
fn two_entries_carrying_one_profile_are_pointed_at_one_object() {
    // ISO 19005-4 section 6.2.3 requires every entry that states a `DestOutputProfile` to state
    // the same indirect object. Where the two objects hold the same bytes, naming one of them
    // says exactly what the file said: each entry still refers its colours to the profile it
    // already referred them to, and the object that goes was a copy. That is what puts this in
    // `doc/adr/0948`'s mechanical class rather than among the choices.
    let source = two_output_intents(SRGB);
    let (report, output) = to_part_four(&source);
    assert_eq!(
        decision(
            &report,
            "graphics/one-destination-profile-per-output-intents-array"
        ),
        Decision::Mechanical(Rewrite::SharedDestinationProfile)
    );
    let output = output.expect("nothing is lost, so nothing is authorised");
    let held = holds(&output, Target::Four(Flavour::Plain));
    assert_eq!(held.verdict(), Verdict::Conforms, "{}", held.render());
    let document =
        Document::open_with_limits(output.clone(), Limits::DEFAULT).expect("the output opens");
    let named = destination_profiles(&document);
    assert_eq!(
        named.len(),
        2,
        "both entries still state a destination profile: {named:?}"
    );
    assert_eq!(
        named.iter().collect::<BTreeSet<_>>().len(),
        1,
        "and they state one object: {named:?}"
    );
}

#[test]
fn two_entries_naming_two_different_profiles_stay_refused() {
    // The other half of the same sentence, and it is a discard rather than a restatement: the
    // two entries name two destinations, the clause admits one, and nothing in the file says
    // which of them its producer meant. `doc/pdf-a-mitigations.md` section 4.1 puts that
    // decision in a configuration an operator writes, which is why the row is refused here
    // rather than answered.
    let mut other = SRGB.to_vec();
    let last = other.len().saturating_sub(1);
    other[last] ^= 0xff;
    let source = two_output_intents(&other);
    let (report, output) = to_part_four(&source);
    assert!(
        matches!(
            decision(
                &report,
                "graphics/one-destination-profile-per-output-intents-array"
            ),
            Decision::Refused(Because::TheFence(_))
        ),
        "the profiles differ, so one of the two destinations would be thrown away"
    );
    assert!(
        output.is_none(),
        "and a refusal writes no file rather than a file that does not conform"
    );
}

/// Every `DestOutputProfile` object the catalog's `OutputIntents` array names.
fn destination_profiles(document: &Document) -> Vec<ObjectId> {
    let catalog = document.catalog().expect("the output has a catalog");
    let intents = document.get_key(&catalog, "OutputIntents");
    let intents = intents.as_array().expect("the array survives").to_vec();
    intents
        .iter()
        .filter_map(|entry| document.resolve(entry).as_dict().cloned())
        .filter_map(|intent| {
            intent
                .get("DestOutputProfile")
                .and_then(pdf_syntax::object::Object::as_reference)
        })
        .collect()
}

// --------------------------------------------------------------------------------------------
// ISO 19005-2 section 6.2.4.4 and ISO 19005-4 section 6.2.4.4: a spot colourant's own entry.
//
// The subclause states two rules about one array and the second decides the first. Every spot
// colour a `DeviceN` space uses needs an entry in that space's `Colorants` dictionary; and every
// `Separation` array in the file naming one colourant, the arrays written inside a `Colorants`
// dictionary expressly included, has to state the same alternate space and tint transform, as
// PDF objects. So where the file already defines the ink, the entry is not a choice.
// --------------------------------------------------------------------------------------------

/// A fixture drawing through a `DeviceN` over one spot colourant and `Black`, with no
/// `/Colorants` entry for the spot.
///
/// §8.6.6.5 shapes the `DeviceN` array and §8.6.6.4 the `Separation`; both use the same
/// `CalGray` alternate and the same §7.10.2 sampled tint transform, so nothing about the fixture
/// depends on a function this test would have to invent. `separately` decides whether the file
/// also states a `Separation` array for the spot colourant of its own, which is what makes the
/// entry derivable rather than a choice.
fn a_devicen_over(separately: bool) -> Vec<u8> {
    // A one-input, one-output sampled function over two samples: §7.10.2's `/Size`, `/Domain`,
    // `/Range` and `/BitsPerSample`, with the samples as two bytes.
    let tint = "<< /FunctionType 0 /Domain [0 1] /Range [0 1] /Size [2] /BitsPerSample 8 \
                /Length 2 >>";
    let gray = "[/CalGray << /WhitePoint [0.9505 1.0 1.089] >>]";
    let separation = format!("[/Separation /Spot {gray} 8 0 R]");
    let devicen = format!("[/DeviceN [/Spot /Black] {gray} 8 0 R]");
    let mut objects = vec![devicen];
    if separately {
        objects.push(separation);
    } else {
        objects.push("<< /Type /Dummy >>".to_owned());
    }
    Conforming {
        resources: "/ColorSpace << /CS0 6 0 R /CS1 7 0 R >>".to_owned(),
        objects,
        binary_objects: vec![stream(tint, &[0x00, 0xff])],
        ..Conforming::default()
    }
    .build()
}

#[test]
fn a_spot_colourant_the_file_already_defines_gains_the_producers_own_entry() {
    let source = a_devicen_over(true);
    let (report, output) = to_part_four(&source);
    assert_eq!(
        decision(
            &report,
            "graphics/spot-colourants-appear-in-the-colorants-dictionary"
        ),
        Decision::Mechanical(Rewrite::SpotColorantEntry)
    );
    let output = output.expect("nothing is lost, so nothing is authorised");
    let held = holds(&output, Target::Four(Flavour::Plain));
    assert_eq!(held.verdict(), Verdict::Conforms, "{}", held.render());
    let written = String::from_utf8_lossy(&output).into_owned();
    assert!(
        written.contains("/Colorants"),
        "the attributes dictionary the space did not have is written: {written}"
    );
    assert!(
        written.contains("/Spot [/Separation /Spot"),
        "and the entry is the Separation array the file already stated: {written}"
    );
}

#[test]
fn a_spot_colourant_the_file_never_defines_stays_refused() {
    // The general case, and section 13.3.1's correction of this catalogue entry: deriving the
    // entry means deriving a one-input tint transform from the space's two-input one, which
    // §7.10 gives a PDF function no way to express — so the derived function could only sample
    // the producer's, and an archive would carry the sample as though it were the definition.
    let source = a_devicen_over(false);
    let (report, output) = to_part_four(&source);
    assert!(
        matches!(
            decision(
                &report,
                "graphics/spot-colourants-appear-in-the-colorants-dictionary"
            ),
            Decision::Refused(Because::NotBuiltYet(_))
        ),
        "nothing in the file states what this ink is on its own"
    );
    assert!(output.is_none(), "and a refusal writes no file");
}

// ----------------------------------------------------------------------------------------------
// ISO 19005-2 section 6.2.8.3, ISO 19005-4 section 6.2.7.3: the two JP2 colour-box requirements.
// ----------------------------------------------------------------------------------------------

/// A JP2 box: `LBox`, `TBox`, payload. ISO/IEC 15444-1:2000 I.4, Table I-1.
fn jp2_box(kind: [u8; 4], payload: &[u8]) -> Vec<u8> {
    let mut bytes = u32::try_from(payload.len().saturating_add(8))
        .expect("a fixture box fits in LBox")
        .to_be_bytes()
        .to_vec();
    bytes.extend_from_slice(&kind);
    bytes.extend_from_slice(payload);
    bytes
}

/// A `colr` box, I.5.3.3: `METH`, `PREC`, `APPROX`, and whatever the method puts after them.
fn jp2_colour(method: u8, approximation: u8, tail: &[u8]) -> Vec<u8> {
    let mut payload = vec![method, 0, approximation];
    payload.extend_from_slice(tail);
    jp2_box(*b"colr", &payload)
}

/// A `colr` box stating `METH` 1 and an enumerated colour space, Table I-9.
fn jp2_enumerated(approximation: u8, space: u32) -> Vec<u8> {
    jp2_colour(1, approximation, &space.to_be_bytes())
}

/// A whole JP2 file over three eight-bit components, carrying the given colour specifications.
///
/// Signature, file type, a `jp2h` holding `ihdr` and the boxes, and a codestream whose `SIZ`
/// marker segment states the same three components: I.5, I.5.3.1 and A.5.1.
fn jp2_file(colour: &[Vec<u8>]) -> Vec<u8> {
    let mut header = 1u32.to_be_bytes().to_vec();
    header.extend_from_slice(&1u32.to_be_bytes());
    header.extend_from_slice(&3u16.to_be_bytes());
    header.extend_from_slice(&[7, 7, 0, 0]);
    let mut inner = jp2_box(*b"ihdr", &header);
    for box_bytes in colour {
        inner.extend_from_slice(box_bytes);
    }

    let mut parameters = 0u16.to_be_bytes().to_vec();
    for value in [1u32, 1, 0, 0, 1, 1, 0, 0] {
        parameters.extend_from_slice(&value.to_be_bytes());
    }
    parameters.extend_from_slice(&3u16.to_be_bytes());
    for _ in 0..3 {
        parameters.extend_from_slice(&[7, 1, 1]);
    }
    let mut codestream = vec![0xFF, 0x4F, 0xFF, 0x51];
    codestream.extend_from_slice(
        &u16::try_from(parameters.len().saturating_add(2))
            .expect("a fixture SIZ segment fits")
            .to_be_bytes(),
    );
    codestream.extend_from_slice(&parameters);

    let mut bytes = jp2_box(*b"jP  ", &[0x0D, 0x0A, 0x87, 0x0A]);
    bytes.extend_from_slice(&jp2_box(*b"ftyp", b"jp2 \0\0\0\0jp2 "));
    bytes.extend_from_slice(&jp2_box(*b"jp2h", &inner));
    bytes.extend_from_slice(&jp2_box(*b"jp2c", &codestream));
    bytes
}

/// A PDF/A-4 fixture whose only offence is the colour specifications of one `JPXDecode` image.
fn a_jpx_image_with(colour: &[Vec<u8>]) -> Vec<u8> {
    let data = jp2_file(colour);
    Conforming {
        resources: "/XObject << /Im0 6 0 R >>".to_owned(),
        binary_objects: vec![stream(
            &format!(
                "/Type /XObject /Subtype /Image /Width 1 /Height 1 /Filter /JPXDecode /Length {}",
                data.len()
            ),
            &data,
        )],
        ..Conforming::default()
    }
    .build()
}

/// Every `colr` box the JPEG 2000 data in a converted file states, as `(METH, APPROX)`.
fn colour_specifications(bytes: &[u8]) -> Vec<(u8, u8)> {
    let document =
        Document::open_with_limits(bytes.to_vec(), Limits::DEFAULT).expect("the output opens");
    for number in document.xref().object_numbers() {
        let pdf_syntax::object::Object::Stream(stream) = document.get(ObjectId::new(number, 0))
        else {
            continue;
        };
        if document
            .get_key(&stream.dict, "Filter")
            .as_name()
            .and_then(|name| name.as_str().map(str::to_owned))
            != Some("JPXDecode".to_owned())
        {
            continue;
        }
        let headers =
            pdf_model::jpeg2000::Headers::parse(&stream.data).expect("the data still parses");
        return headers
            .colour
            .iter()
            .map(|colour| (colour.method, colour.approximation))
            .collect();
    }
    panic!("the converted file still holds the JPXDecode image");
}

#[test]
fn a_colour_specification_the_part_ignores_is_removed_only_with_authorisation() {
    // ISO 19005-2 section 6.2.8.3 and ISO 19005-4 section 6.2.7.3 admit a METH of 0x01, 0x02 or
    // 0x03 and no other, and the sentence beside it says a conforming processor shall use only
    // the selected colour space and shall ignore all the other specifications. So the box that
    // fails here is one the part itself directs a processor to ignore, and removing it is the
    // route that writes no value of this converter's — which is what doc/pdf-a-mitigations.md
    // section 4.5 had not asked.
    //
    // Exactly one specification carries the APPROX of 0x01 both parts' NOTE 2 makes the mark of
    // the best available, so the one-best rule passes and this fixture fails the method rule
    // alone.
    let source = a_jpx_image_with(&[
        jp2_enumerated(1, 16),
        jp2_colour(4, 2, b"a vendor colour method"),
    ]);
    let target = Target::Four(Flavour::Plain);

    let (report, output) = to_part_four(&source);
    assert_eq!(
        decision(&report, "graphics/jpeg2000-colour-specification-method"),
        Decision::Unauthorised {
            loss: Loss::Jpeg2000ColourFallback,
            rewrite: Rewrite::Jpeg2000ColourSpecifications,
        }
    );
    assert!(output.is_none(), "unauthorised, so nothing is written");

    let authorised = Authorisations {
        image_smoothing: false,
        metadata_property: false,
        annotation_printing: false,
        jpeg2000_colour_fallback: true,
    };
    let (report, output) = convert(&source, target, authorised);
    assert_eq!(
        decision(&report, "graphics/jpeg2000-colour-specification-method"),
        Decision::Authorised {
            loss: Loss::Jpeg2000ColourFallback,
            rewrite: Rewrite::Jpeg2000ColourSpecifications,
        }
    );
    let output = output.expect("authorised, so it converts");
    assert_eq!(holds(&output, target).verdict(), Verdict::Conforms);
    assert_eq!(
        colour_specifications(&output),
        vec![(1, 1)],
        "the specification the part makes the used one stays, and it alone"
    );
}

#[test]
fn a_file_marking_no_specification_best_keeps_the_one_a_jp2_reader_uses() {
    // ISO/IEC 15444-1:2000 I.5.3.3 reserves APPROX, requires it to be zero and has a conforming
    // reader ignore its value — and says in the same clause that a conforming JP2 reader ignores
    // every colour specification box after the first. So a file written to that edition marks
    // none of its specifications best available, fails ISO 19005's one-best rule for exactly
    // that reason, and the box a reader uses is nevertheless settled: the first.
    let source = a_jpx_image_with(&[
        jp2_enumerated(0, 16),
        jp2_colour(4, 0, b"a vendor colour method"),
    ]);
    let target = Target::Four(Flavour::Plain);
    let authorised = Authorisations {
        image_smoothing: false,
        metadata_property: false,
        annotation_printing: false,
        jpeg2000_colour_fallback: true,
    };
    let (report, output) = convert(&source, target, authorised);
    assert_eq!(
        decision(
            &report,
            "graphics/jpeg2000-one-best-colour-space-specification"
        ),
        Decision::Authorised {
            loss: Loss::Jpeg2000ColourFallback,
            rewrite: Rewrite::Jpeg2000ColourSpecifications,
        }
    );
    let output = output.expect("authorised, so it converts");
    assert_eq!(holds(&output, target).verdict(), Verdict::Conforms);
    assert_eq!(
        colour_specifications(&output),
        vec![(1, 0)],
        "one specification left, so the rule whose condition is more than one no longer applies"
    );
}

#[test]
fn two_specifications_marked_best_stay_refused() {
    // The half of doc/pdf-a-mitigations.md section 4.5 that is unchanged: where the file ranks
    // its own specifications ambiguously, choosing between them ranks two of the producer's
    // statements on evidence the file does not carry. Neither ISO 19005's APPROX sentence nor
    // ISO/IEC 15444-1:2000 I.5.3.3's first-box rule selects one here, so the refusal stands.
    let source = a_jpx_image_with(&[jp2_enumerated(1, 16), jp2_enumerated(1, 17)]);
    let authorised = Authorisations {
        image_smoothing: false,
        metadata_property: false,
        annotation_printing: false,
        jpeg2000_colour_fallback: true,
    };
    let (report, output) = convert(&source, Target::Four(Flavour::Plain), authorised);
    assert!(
        matches!(
            decision(
                &report,
                "graphics/jpeg2000-one-best-colour-space-specification"
            ),
            Decision::Refused(Because::NotBuiltYet(_))
        ),
        "two boxes marked best available, and nothing in either standard says which wins"
    );
    assert!(output.is_none(), "and a refusal writes no file");
}

#[test]
fn one_specification_with_a_method_the_part_forbids_stays_refused() {
    // The other half that is unchanged, and the sharper one: with a single colour specification
    // there is nothing to remove, and the only route left writes one of the three admitted
    // methods in its place — which states a colour space the box did not.
    let source = a_jpx_image_with(&[jp2_colour(4, 1, b"a vendor colour method")]);
    let authorised = Authorisations {
        image_smoothing: false,
        metadata_property: false,
        annotation_printing: false,
        jpeg2000_colour_fallback: true,
    };
    let (report, output) = convert(&source, Target::Four(Flavour::Plain), authorised);
    assert!(
        matches!(
            decision(&report, "graphics/jpeg2000-colour-specification-method"),
            Decision::Refused(Because::NotBuiltYet(_))
        ),
        "there is no other box for a processor to fall back to and none to remove"
    );
    assert!(output.is_none(), "and a refusal writes no file");
}
