//! A packet kept on an appended page: its intent, its face, and the two rows a replaced packet
//! meets (`doc/adr/2676`, `doc/adr/2677`).
//!
//! ISO 32000-2 §8.4.1's Table 51 makes `DeviceGray` the initial colour space, so a page that sets
//! a packet's text paints in a device space, and ISO 19005-2 and ISO 19005-4 section 6.2.4.3
//! license one through an output intent of any family: a source stating none is given one. A
//! character `/WinAnsiEncoding` does not name is reached through a `/Differences` array of Adobe
//! Glyph List names, the route §9.6.5.4 gives a nonsymbolic TrueType font and ISO 19005-2 section
//! 6.2.11.6 admits. A packet section 6.6.2.1 replaces states no property, so it states none
//! outside its schema (section 6.6.2.3.1); and one that will not decode leaves no bytes for a
//! page, so the replacement is the remedy and no page is owed.
use super::{
    ArchivePlan, Because, Decision, Level, Rewrite, Target, Verdict, a_document_carrying,
    convert_with_plan, decision, holds, plan_from,
};
use pdf_syntax::{Document, Limits};

/// The profile these sites are answered by, as it ships.
const STATE_EVERY_LOSS: &str = include_str!("../../../../doc/profiles/state-every-loss.toml");

const PROPERTIES: &str = "metadata/properties-use-known-schemas";
const WELL_FORMED: &str = "metadata/xmp-packets-well-formed";

const TARGETS: [Target; 2] = [
    Target::Two(Level::B),
    Target::Four(pdf_archive::Flavour::Plain),
];

/// The target these fixtures fail section 6.6.2.3.1's row at.
const PART_TWO: [Target; 1] = [Target::Two(Level::B)];

fn planned(target: Target) -> ArchivePlan {
    let config =
        pdf_transform::archive::Configuration::read(STATE_EVERY_LOSS, target).expect("it reads");
    ArchivePlan {
        authorised: config.authorisations(target),
        ..plan_from(STATE_EVERY_LOSS, target)
    }
}

/// A packet whose `xmp:CreateDate` is not a date, so section 6.6.2.3.1's row fails at it, and
/// whose `pdf:Producer` is `producer`.
fn a_packet_with_a_bad_date(producer: &str) -> String {
    format!(
        "<?xpacket begin=\"\u{feff}\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?>\n\
         <x:xmpmeta xmlns:x=\"adobe:ns:meta/\">\n\
         <rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\n\
         <rdf:Description rdf:about=\"\" xmlns:xmp=\"http://ns.adobe.com/xap/1.0/\" \
         xmlns:pdf=\"http://ns.adobe.com/pdf/1.3/\">\n\
         <xmp:CreateDate>the day we shipped it</xmp:CreateDate>\n\
         <pdf:Producer>{producer}</pdf:Producer>\n\
         </rdf:Description>\n</rdf:RDF>\n</x:xmpmeta>\n<?xpacket end=\"w\"?>"
    )
}

/// The fixture with its page's content blanked to spaces of the same length, so that the source
/// paints nothing — no device colour any row asks an intent for — and states no intent.
fn painting_nothing(mut bytes: Vec<u8>) -> Vec<u8> {
    let was: &[u8] = b"0 g BT /F1 12 Tf 10 100 Td (A) Tj ET";
    let at = bytes
        .windows(was.len())
        .position(|window| window == was)
        .expect("the fixture's page shows one character");
    bytes[at..at.saturating_add(was.len())].fill(b' ');
    bytes
}

fn page_count(output: &[u8]) -> usize {
    let document = Document::open_with_limits(output.to_vec(), Limits::DEFAULT).expect("it opens");
    pdf_model::Pages::new(&document).len()
}

fn text_of_last_page(output: &[u8]) -> String {
    let document = Document::open_with_limits(output.to_vec(), Limits::DEFAULT).expect("it opens");
    let pages = pdf_model::Pages::new(&document);
    let page = pages
        .get(pages.len().saturating_sub(1))
        .expect("the page exists");
    pdf_model::interpret(&document, &page).text
}

#[test]
fn a_packet_page_in_a_file_stating_no_intent_is_given_one() {
    let source = painting_nothing(a_document_carrying(
        a_packet_with_a_bad_date("Somebody's exporter"),
        String::new(),
    ));
    assert!(
        !String::from_utf8_lossy(&source).contains("OutputIntents"),
        "the source states no intent"
    );
    for target in PART_TWO {
        let (report, output) = convert_with_plan(&source, &planned(target));
        let output = output.unwrap_or_else(|| panic!("{target:?}: it converts: {report:#?}"));
        assert_eq!(
            holds(&output, target).verdict(),
            Verdict::Conforms,
            "{target:?}"
        );
        assert!(
            String::from_utf8_lossy(&output).contains("/OutputIntents"),
            "{target:?}: the appended page's DeviceGray is licensed by an intent"
        );
        assert_eq!(page_count(&output), 2, "{target:?}: one page appended");
        assert!(text_of_last_page(&output).contains("Somebody's exporter"));
    }
}

#[test]
fn a_character_win_ansi_does_not_name_is_set_through_a_differences_array() {
    // U+0401 and U+043B are Cyrillic: no WinAnsiEncoding code names them, the Adobe Glyph List
    // lists both (`afii10023`, `afii10077`), and the shipped face draws both.
    let source = a_document_carrying(a_packet_with_a_bad_date("Ёлка exporter"), String::new());
    for target in PART_TWO {
        let (report, output) = convert_with_plan(&source, &planned(target));
        let output = output.unwrap_or_else(|| panic!("{target:?}: it converts: {report:#?}"));
        assert_eq!(
            holds(&output, target).verdict(),
            Verdict::Conforms,
            "{target:?}"
        );
        let written = String::from_utf8_lossy(&output).into_owned();
        assert!(written.contains("/Differences"), "{target:?}");
        assert!(
            written.contains("/BaseEncoding /WinAnsiEncoding"),
            "{target:?}"
        );
        assert!(
            text_of_last_page(&output).contains("Ёлка exporter"),
            "{target:?}: the page's text extracts as the packet's"
        );
    }
}

#[test]
fn a_character_the_list_does_not_name_keeps_the_refusal() {
    // U+4E00 has no name the Adobe Glyph List lists, only the computed `uni4E00`, which ISO
    // 19005-2 section 6.2.11.6 does not admit in a `/Differences` array.
    let source = a_document_carrying(a_packet_with_a_bad_date("一 exporter"), String::new());
    let target = Target::Two(Level::B);
    let (report, output) = convert_with_plan(&source, &planned(target));
    assert!(output.is_none());
    assert!(
        matches!(decision(&report, PROPERTIES), Decision::Refused(Because::NotBuiltYet(why))
            if why.contains("a Differences array naming each")),
        "{:?}",
        decision(&report, PROPERTIES)
    );
}

#[test]
fn a_property_in_a_replaced_packet_takes_the_replacements_decision() {
    // A prefix no declaration binds is section 6.6.2.1's fault (`doc/adr/2412`), and the packet's
    // bad date is section 6.6.2.3.1's: the replacement states no property, so none outside its
    // schema either.
    let packet = a_packet_with_a_bad_date("an exporter").replace(
        "</rdf:Description>",
        "<dc:format>application/pdf</dc:format>\n</rdf:Description>",
    );
    let source = a_document_carrying(packet, String::new());
    for target in PART_TWO {
        let (report, output) = convert_with_plan(&source, &planned(target));
        assert_eq!(
            decision(&report, PROPERTIES),
            decision(&report, WELL_FORMED),
            "{target:?}: the property row takes the replacement's decision"
        );
        let output = output.unwrap_or_else(|| panic!("{target:?}: it converts: {report:#?}"));
        assert_eq!(
            holds(&output, target).verdict(),
            Verdict::Conforms,
            "{target:?}"
        );
    }
}

#[test]
fn a_packet_that_will_not_decode_is_replaced_and_owes_no_page() {
    // The catalog's packet states Flate data that is not Flate: section 6.6.2.1's row fails at it
    // and nothing decodes for a page to keep.
    let broken = super::Conforming {
        metadata: super::Packet::Unnamed,
        catalog: "/Metadata 6 0 R".to_owned(),
        objects: vec![
            "<< /Type /Metadata /Subtype /XML /Filter /FlateDecode /Length 4 >>\nstream\nabcd\nendstream"
                .to_owned(),
        ],
        ..super::Conforming::part_two()
    }
    .build();
    for target in TARGETS {
        let (report, output) = convert_with_plan(&broken, &planned(target));
        assert!(
            matches!(
                decision(&report, WELL_FORMED),
                Decision::Configured {
                    rewrite: Rewrite::FreshMetadataPacket,
                    ..
                }
            ),
            "{target:?}: {:?}",
            decision(&report, WELL_FORMED)
        );
        let output = output.unwrap_or_else(|| panic!("{target:?}: it converts: {report:#?}"));
        assert_eq!(
            holds(&output, target).verdict(),
            Verdict::Conforms,
            "{target:?}"
        );
        assert_eq!(page_count(&output), 1, "{target:?}: no page appended");
    }
}
