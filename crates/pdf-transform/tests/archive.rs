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

use std::fmt::Write as _;

use pdf_archive::{Flavour, Level, Outcome, Target, Verdict};
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
    // conversion reads a clock, a path or an environment nowhere, so this is a test rather than
    // a demo.
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
    // the profile this program ships is RGB. `doc/pdf-a-conversion-limits.md` section 10.1 is the
    // whole of the answer: supply the press's profile, or wait for the DeviceN /DefaultCMYK
    // construction that section states.
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
