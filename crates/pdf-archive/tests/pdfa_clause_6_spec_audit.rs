//! The specification audit of ISO 19005-2 and ISO 19005-4 clause 6 (review 2282).
//!
//! Each test builds the smallest file on which [`pdf_archive::check`] reads a different object
//! than the clause names, beside a twin that differs only where the clause looks, and asks for
//! the verdict the clause gives. The twins are not conforming files (they state no output intent);
//! what a test compares is the one requirement under audit, or the set of requirements the twin
//! does not already fail. Every test here fails until a fix round takes the finding, and each is
//! listed in `doc/reviews/2282-specification-audit-19005/expected-failures.json`.
//!
//! ISO 19005 is cited by section and paraphrased, never quoted (`doc/third-party-data.md`);
//! ISO 32000 is quoted where its words decide.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    clippy::too_many_lines,
    reason = "test code: fixtures are small, built here, and a fixture that cannot open should fail \
              loudly; B01's test is one table of twelve sites, read top to bottom"
)]

use std::fmt::Write as _;

use pdf_archive::{Level, Outcome, Target, check};
use pdf_syntax::Document;

/// Level B: every requirement under audit here binds it.
const TWO_B: Target = Target::Two(Level::B);

/// The file identifier every fixture states, so that section 6.1.3 is met on both twins.
const ID: &str = "<00112233445566778899aabbccddeeff>";

/// A packet that declares PDF/A-2b and nothing else.
const PACKET: &str = "<?xpacket begin=\"\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?>\
<x:xmpmeta xmlns:x=\"adobe:ns:meta/\"><rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\
<rdf:Description rdf:about=\"\" xmlns:pdfaid=\"http://www.aiim.org/pdfa/ns/id/\">\
<pdfaid:part>2</pdfaid:part><pdfaid:conformance>B</pdfaid:conformance></rdf:Description>\
</rdf:RDF></x:xmpmeta><?xpacket end=\"w\"?>";

/// A whole file from object bodies, numbered from 1, with an exact cross-reference table.
fn file(objects: &[String], trailer_extra: &str) -> Vec<u8> {
    let mut bytes: Vec<u8> = b"%PDF-1.7\n%\xe2\xe3\xcf\xd3\n".to_vec();
    let mut offsets = Vec::with_capacity(objects.len());
    for (index, body) in objects.iter().enumerate() {
        offsets.push(bytes.len());
        bytes.extend_from_slice(format!("{} 0 obj\n{body}\nendobj\n", index + 1).as_bytes());
    }
    let xref = bytes.len();
    let size = objects.len() + 1;
    let mut table = format!("xref\n0 {size}\n0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(table, "{offset:010} 00000 n ");
    }
    let _ = write!(
        table,
        "trailer\n<< /Size {size} /Root 1 0 R /ID [{ID} {ID}] {trailer_extra} >>\nstartxref\n{xref}\n%%EOF\n"
    );
    bytes.extend_from_slice(table.as_bytes());
    bytes
}

/// A stream object's body, its `/Length` exact.
fn stream(dictionary: &str, data: &str) -> String {
    format!(
        "<< {dictionary} /Length {} >>\nstream\n{data}\nendstream",
        data.len()
    )
}

/// The parts of a one-page document a test varies.
#[derive(Clone, Copy)]
struct Shape<'a> {
    catalog: &'a str,
    page: &'a str,
    resources: &'a str,
    content: &'a str,
    content_dictionary: &'a str,
    trailer: &'a str,
}

impl Default for Shape<'_> {
    fn default() -> Self {
        Self {
            catalog: "",
            page: "",
            resources: "<< >>",
            content: "",
            content_dictionary: "",
            trailer: "",
        }
    }
}

/// Catalog (1), page tree (2), page (3), metadata (4), content (5), then `extra` from 6 on.
fn document(shape: Shape<'_>, extra: &[String]) -> Vec<u8> {
    let mut objects = vec![
        format!(
            "<< /Type /Catalog /Pages 2 0 R /Metadata 4 0 R {} >>",
            shape.catalog
        ),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Resources {} /Contents 5 0 R {} >>",
            shape.resources, shape.page
        ),
        stream("/Type /Metadata /Subtype /XML", PACKET),
        stream(shape.content_dictionary, shape.content),
    ];
    objects.extend_from_slice(extra);
    file(&objects, shape.trailer)
}

/// The requirements `check` fails on a file.
fn failed(bytes: Vec<u8>, target: Target) -> Vec<&'static str> {
    let document = Document::open(bytes).expect("the fixture opens");
    assert!(
        !document.was_recovered(),
        "the fixture's cross-reference table is the file's own"
    );
    check(&document, target)
        .judgements
        .into_iter()
        .filter(|judgement| matches!(judgement.outcome, Outcome::Failed { .. }))
        .map(|judgement| judgement.id)
        .collect()
}

/// What `variant` fails that `twin` does not.
fn newly_failed(twin: Vec<u8>, variant: Vec<u8>, target: Target) -> Vec<&'static str> {
    let before = failed(twin, target);
    failed(variant, target)
        .into_iter()
        .filter(|id| !before.contains(id))
        .collect()
}

/// An appearance stream every annotation fixture names as its `/N`.
fn appearance() -> String {
    stream("/Type /XObject /Subtype /Form /BBox [0 0 10 10]", "")
}

// ---------------------------------------------------------------------------------------------
// 2282-B01: a forbidden key whose value is a reference to an undefined object.

/// ISO 32000-1 section 7.3.10, and ISO 32000-2 §7.3.10 in the same words for part 4:
///
/// > An indirect reference to an undefined object shall not be considered an error by a PDF
/// > processor; it shall be treated as a reference to the null object.
///
/// and §7.3.7 makes a null entry the absent entry. `crate::table::states` is the validator's own
/// reading of both, and its comment says every rule that forbids a key asks through it. Twelve
/// rules ask `Dictionary::get(..).is_some()` instead, so `/AA 99 0 R` in a file with no object 99
/// is reported as an `AA` the clause forbids. Each case below differs from its twin only by one
/// such entry; none may fail a requirement the twin meets.
#[test]
fn a_forbidden_key_naming_no_object_states_nothing_spec_audit() {
    const NOWHERE: &str = "99 0 R";
    let mut reported = Vec::new();
    let mut case = |site: &str, twin: Vec<u8>, variant: Vec<u8>| {
        let new = newly_failed(twin, variant, TWO_B);
        if !new.is_empty() {
            reported.push(format!("{site}: {new:?}"));
        }
    };

    let plain = || document(Shape::default(), &[]);
    let content_keys = format!("/F {NOWHERE} /FFilter {NOWHERE} /FDecodeParms {NOWHERE}");
    case(
        "a stream's /F, /FFilter, /FDecodeParms (ISO 19005-2 section 6.1.7.1)",
        plain(),
        document(
            Shape {
                content_dictionary: &content_keys,
                ..Shape::default()
            },
            &[],
        ),
    );
    let catalog_keys = format!("/AA {NOWHERE} /Requirements {NOWHERE} /NeedsRendering {NOWHERE}");
    case(
        "the catalog's /AA, /Requirements, /NeedsRendering (sections 6.5.2, 6.11, 6.4.2)",
        plain(),
        document(
            Shape {
                catalog: &catalog_keys,
                ..Shape::default()
            },
            &[],
        ),
    );
    let page_keys = format!("/AA {NOWHERE} /PresSteps {NOWHERE}");
    case(
        "a page's /AA and /PresSteps (sections 6.5.2, 6.10)",
        plain(),
        document(
            Shape {
                page: &page_keys,
                ..Shape::default()
            },
            &[],
        ),
    );
    let form = format!("/AcroForm << /Fields [] /XFA {NOWHERE} >>");
    case(
        "the interactive form's /XFA (section 6.4.2)",
        document(
            Shape {
                catalog: "/AcroForm << /Fields [] >>",
                ..Shape::default()
            },
            &[],
        ),
        document(
            Shape {
                catalog: &form,
                ..Shape::default()
            },
            &[],
        ),
    );
    let names = format!("/Names << /AlternatePresentations {NOWHERE} >>");
    case(
        "the name dictionary's /AlternatePresentations (section 6.10)",
        document(
            Shape {
                catalog: "/Names << >>",
                ..Shape::default()
            },
            &[],
        ),
        document(
            Shape {
                catalog: &names,
                ..Shape::default()
            },
            &[],
        ),
    );
    let group = ["<< /Type /OCG /Name (g) >>".to_owned()];
    let automatic = format!(
        "/OCProperties << /OCGs [6 0 R] /D << /Name (d) /Order [6 0 R] /AS {NOWHERE} >> >>"
    );
    case(
        "an optional content configuration's /AS (section 6.9)",
        document(
            Shape {
                catalog: "/OCProperties << /OCGs [6 0 R] /D << /Name (d) /Order [6 0 R] >> >>",
                ..Shape::default()
            },
            &group,
        ),
        document(
            Shape {
                catalog: &automatic,
                ..Shape::default()
            },
            &group,
        ),
    );
    let image = |extra: &str| {
        stream(
            &format!(
                "/Type /XObject /Subtype /Image /Width 1 /Height 1 /ColorSpace /CalGray \
                 /BitsPerComponent 8 {extra}"
            ),
            "0",
        )
    };
    case(
        "an image's /Intent (sections 6.2.8.1 and 6.2.6)",
        document(Shape::default(), &[image("")]),
        document(Shape::default(), &[image(&format!("/Intent {NOWHERE}"))]),
    );
    let state = |extra: &str| format!("<< /Type /ExtGState {extra} >>");
    case(
        "a graphics state's /RI (sections 6.2.5 and 6.2.6)",
        document(Shape::default(), &[state("")]),
        document(Shape::default(), &[state(&format!("/RI {NOWHERE}"))]),
    );
    let widget = |extra: &str| {
        format!(
            "<< /Type /Annot /Subtype /Widget /FT /Tx /T (a) /Rect [0 0 10 10] /F 4 \
             /AP << /N 7 0 R >> {extra} >>"
        )
    };
    let with_widget = Shape {
        catalog: "/AcroForm << /Fields [6 0 R] >>",
        page: "/Annots [6 0 R]",
        ..Shape::default()
    };
    case(
        "a widget's /A and /AA (sections 6.4.1 and 6.5.2)",
        document(with_widget, &[widget(""), appearance()]),
        document(
            with_widget,
            &[widget(&format!("/A {NOWHERE} /AA {NOWHERE}")), appearance()],
        ),
    );
    let square = |extra: &str| {
        format!(
            "<< /Type /Annot /Subtype /Square /Rect [0 0 10 10] /F 4 \
             /AP << /N 7 0 R {extra} >> >>"
        )
    };
    let with_square = Shape {
        page: "/Annots [6 0 R]",
        ..Shape::default()
    };
    case(
        "an appearance dictionary's /D and /R (section 6.3.3)",
        document(with_square, &[square(""), appearance()]),
        document(
            with_square,
            &[square(&format!("/D {NOWHERE} /R {NOWHERE}")), appearance()],
        ),
    );
    let permissions = format!("/Perms << /UR {NOWHERE} >>");
    case(
        "a permissions dictionary's /UR (section 6.1.12)",
        document(
            Shape {
                catalog: "/Perms << >>",
                ..Shape::default()
            },
            &[],
        ),
        document(
            Shape {
                catalog: &permissions,
                ..Shape::default()
            },
            &[],
        ),
    );

    assert!(
        reported.is_empty(),
        "an entry naming an undefined object is the null object, and the null object is the absent \
         entry; these were reported as the forbidden key itself:\n{}",
        reported.join("\n")
    );
}

// ---------------------------------------------------------------------------------------------
// 2282-B02: a file specification written directly where it is used.

/// ISO 19005-2 section 6.8 binds the file specification of every embedded file wherever it
/// stands: its embedded file has to conform to part 1 or part 2, and it has to state both `F` and
/// `UF`. ISO 32000-1 section 12.5.6.15 lets a file attachment annotation's `/FS` hold the file
/// specification directly. The validator's population is the indirect objects plus the direct
/// leaves of the `/EmbeddedFiles` tree, so a direct `/FS` is never read: the file below embeds a
/// plain-text file, states no `UF`, and passes both rows. The converter then reports the output
/// as conforming; veraPDF 1.30 fails the same output on 6.8, test 5 (evidence, not the reading).
#[test]
fn a_file_specification_written_in_place_is_held_to_the_embedded_file_rules_spec_audit() {
    let attachment = |specification: &str| {
        format!(
            "<< /Type /Annot /Subtype /FileAttachment /Rect [0 0 10 10] /F 4 /Contents (f) \
             /AP << /N 7 0 R >> /FS {specification} >>"
        )
    };
    let text_file = stream("/Type /EmbeddedFile /Subtype /text#2Fplain", "hello");
    let shape = Shape {
        page: "/Annots [6 0 R]",
        ..Shape::default()
    };
    let rules = [
        "embedded-files/embedded-file-is-itself-pdfa",
        "embedded-files/file-and-unicode-names",
    ];

    // The control: the same specification as an indirect object is read, and fails both rules.
    let indirect = failed(
        document(
            shape,
            &[
                attachment("9 0 R"),
                appearance(),
                text_file.clone(),
                "<< /Type /Filespec /F (a.txt) /EF << /F 8 0 R >> >>".to_owned(),
            ],
        ),
        TWO_B,
    );
    for rule in rules {
        assert!(
            indirect.contains(&rule),
            "the control fails {rule} (it fails {indirect:?})"
        );
    }

    let direct = failed(
        document(
            shape,
            &[
                attachment("<< /Type /Filespec /F (a.txt) /EF << /F 8 0 R >> >>"),
                appearance(),
                text_file,
            ],
        ),
        TWO_B,
    );
    let missed: Vec<_> = rules.iter().filter(|rule| !direct.contains(rule)).collect();
    assert!(
        missed.is_empty(),
        "a file attachment annotation's direct /FS embeds a text file and states no UF, and \
         {missed:?} passed it (the file fails {direct:?})"
    );
}

// ---------------------------------------------------------------------------------------------
// 2282-B03: colour operators the base standard ignores, counted as device colour.

/// ISO 32000-1 sections 8.6.8 and 9.6.5 (ISO 32000-2 §9.6.4's Table 111 in the same words):
/// after `d1` "any use of such operators shall be ignored", and §8.7.3.3's uncoloured tiling
/// pattern is painted in the colour the pattern is selected with, its cell's colour operators
/// ignored. ISO 19005-2 section 6.2.4.3 binds the device colour a file *uses*; an operator the
/// base standard ignores uses nothing, and a cell whose colour comes from its pattern uses no
/// initial `DeviceGray`. The survey carries the `uncoloured` state for exactly this
/// (`survey.rs`'s `State::uncoloured`) and hands it to the grammar only; `set_device_colour`,
/// `set_space` and the initial-space record never read it. veraPDF 1.30 reports none of the five
/// (evidence).
#[test]
fn colour_the_base_standard_ignores_is_no_device_colour_use_spec_audit() {
    let calibrated = "[/CalRGB << /WhitePoint [0.9505 1 1.089] >>]";
    let device = [
        "graphics/device-gray-needs-a-default-or-an-output-intent",
        "graphics/device-rgb-needs-a-default-or-an-rgb-output-intent",
    ];
    let mut reported = Vec::new();

    let font = "<< /Type /Font /Subtype /Type3 /FontBBox [0 0 1 1] \
                /FontMatrix [0.001 0 0 0.001 0 0] /CharProcs << /a 7 0 R >> \
                /Encoding << /Differences [97 /a] >> /FirstChar 97 /LastChar 97 \
                /Widths [1000] /Resources << >> >>"
        .to_owned();
    let glyph_resources = format!("<< /Font << /F 6 0 R >> /ColorSpace << /C {calibrated} >> >>");
    let glyph = |procedure: &str| {
        document(
            Shape {
                resources: &glyph_resources,
                content: "/C cs 1 0 0 sc BT /F 12 Tf (a) Tj ET",
                ..Shape::default()
            },
            &[font.clone(), stream("", procedure)],
        )
    };
    for (what, procedure) in [
        (
            "rg in a d1 glyph",
            "1000 0 0 0 1000 1000 d1 1 0 0 rg 0 0 1000 1000 re f",
        ),
        (
            "cs /DeviceRGB in a d1 glyph",
            "1000 0 0 0 1000 1000 d1 /DeviceRGB cs 1 0 0 sc 0 0 1000 1000 re f",
        ),
    ] {
        let new = newly_failed(
            glyph("1000 0 0 0 1000 1000 d1 0 0 1000 1000 re f"),
            glyph(procedure),
            TWO_B,
        );
        if !new.is_empty() {
            reported.push(format!("{what}: {new:?}"));
        }
    }

    let pattern_resources =
        format!("<< /Pattern << /P 6 0 R >> /ColorSpace << /CS0 [/Pattern {calibrated}] >> >>");
    let tiled = |cell: &str| {
        document(
            Shape {
                resources: &pattern_resources,
                content: "/CS0 cs 0.5 0.5 0.5 /P scn 0 0 50 50 re f",
                ..Shape::default()
            },
            &[stream(
                "/Type /Pattern /PatternType 1 /PaintType 2 /TilingType 1 /BBox [0 0 10 10] \
                 /XStep 10 /YStep 10 /Resources << >>",
                cell,
            )],
        )
    };
    for (what, cell) in [
        ("an uncoloured cell that sets no colour", "0 0 5 5 re f"),
        ("rg in an uncoloured cell", "1 0 0 rg 0 0 5 5 re f"),
        (
            "cs /DeviceRGB in an uncoloured cell",
            "/DeviceRGB cs 1 0 0 sc 0 0 5 5 re f",
        ),
    ] {
        let fails = failed(tiled(cell), TWO_B);
        let used: Vec<_> = device.iter().filter(|rule| fails.contains(rule)).collect();
        if !used.is_empty() {
            reported.push(format!("{what}: {used:?}"));
        }
    }

    assert!(
        reported.is_empty(),
        "a colour the base standard ignores was reported as a device colour use:\n{}",
        reported.join("\n")
    );
}

// ---------------------------------------------------------------------------------------------
// 2282-B04: a metadata stream known by where it stands.

/// ISO 19005-2 section 6.6.2.1 (ISO 19005-4 section 6.7.2.1): every metadata stream in the file is
/// held to the XMP rules, among them well-formed XML and no deprecated `bytes` attribute in the
/// packet header. ISO 32000-1 Table 30 makes a page's `/Metadata` "a metadata stream containing
/// metadata for the page". The population is streams stating `/Type /Metadata`; a page's
/// `/Metadata` stream that omits that required entry is read by nothing — not by these rows, and
/// not by `conformance/adheres-to-the-base-standard` for the missing `/Type` — so a malformed
/// packet there passes every row. The control, the same stream with `/Type`, fails both.
#[test]
fn a_page_metadata_stream_is_read_whether_or_not_it_names_its_type_spec_audit() {
    let malformed = "<?xpacket begin=\"\" id=\"W5M0MpCehiHzreSzNTczkc9d\" bytes=\"10\"?>\
        <x:xmpmeta xmlns:x=\"adobe:ns:meta/\"><rdf:RDF \
        xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\"><unclosed></rdf:RDF>\
        </x:xmpmeta><?xpacket end=\"w\"?>";
    let rules = [
        "metadata/xmp-packets-well-formed",
        "metadata/xmp-packet-header-attributes",
    ];
    let with_page_metadata = |dictionary: &str| {
        failed(
            document(
                Shape {
                    page: "/Metadata 6 0 R",
                    ..Shape::default()
                },
                &[stream(dictionary, malformed)],
            ),
            TWO_B,
        )
    };

    let typed = with_page_metadata("/Type /Metadata /Subtype /XML");
    for rule in rules {
        assert!(
            typed.contains(&rule),
            "the control fails {rule} (it fails {typed:?})"
        );
    }
    let untyped = with_page_metadata("/Subtype /XML");
    assert!(
        rules.iter().any(|rule| untyped.contains(rule))
            || untyped.contains(&"conformance/adheres-to-the-base-standard"),
        "a page's /Metadata stream with a malformed packet and no /Type failed nothing that reads \
         it (the file fails {untyped:?})"
    );
}

// ---------------------------------------------------------------------------------------------
// 2282-B05: the groups an Order array has to reference.

/// ISO 19005-2 section 6.9 (ISO 19005-4 section 6.10): an `Order` array references every optional
/// content group in the file. The row's census is `/OCProperties /OCGs`, so a group the page's
/// content selects through `/OC` (ISO 32000-1 section 8.11.3.2) that the array leaves out is no
/// group the row knows — and the base standard's own rule that `/OCGs` lists every group is
/// reported by no row either. The control: the same group listed in `/OCGs` and left out of
/// `Order` fails.
#[test]
fn an_order_array_references_every_group_the_file_uses_spec_audit() {
    let rule = "optional-content/order-lists-every-group";
    let groups = [
        "<< /Type /OCG /Name (g) >>".to_owned(),
        "<< /Type /OCG /Name (h) >>".to_owned(),
    ];
    let with = |listed: &str| {
        let catalog =
            format!("/OCProperties << /OCGs {listed} /D << /Name (v) /Order [6 0 R] >> >>");
        failed(
            document(
                Shape {
                    catalog: &catalog,
                    resources: "<< /Properties << /oc1 7 0 R >> >>",
                    content: "/OC /oc1 BDC 0 0 m 10 10 l S EMC",
                    ..Shape::default()
                },
                &groups,
            ),
            TWO_B,
        )
    };

    let listed = with("[6 0 R 7 0 R]");
    assert!(
        listed.contains(&rule),
        "the control fails {rule} (it fails {listed:?})"
    );
    let unlisted = with("[6 0 R]");
    assert!(
        unlisted.contains(&rule),
        "the page's content selects optional content group 7 0, which no Order array references, \
         and {rule} passed (the file fails {unlisted:?})"
    );
}
