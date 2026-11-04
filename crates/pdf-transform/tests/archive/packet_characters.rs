//! A C0 control character a packet holds, left off the page that keeps the packet by its row's
//! second step and stated (`doc/adr/2682`, `doc/questions/A778`).
//!
//! A control character has no glyph in any face, and setting one would show `.notdef`, which
//! ISO 19005-2 section 6.2.11.8 forbids. `state-every-loss` answers the packet rows with
//! `preserve`, `original = "page"` and `on-failure = "append-statement"`, and that second step
//! leaves the character off the page, lays the rest of the packet out, and lists the character on
//! the statement page by its code point, its count and its byte offsets. ISO 32000-2 §7.2.3's line
//! breaks are not control characters to it: the layout reads them as end-of-line markers.
//! `no-data-loss` names no second step and keeps the face search's refusal, and a character
//! outside C0 keeps it under every profile.
use super::{
    ArchivePlan, Because, Decision, Level, Rewrite, Target, Verdict, conversion, convert_with_plan,
    decision, holds, plan_from,
};
use pdf_syntax::{Document, Limits};

const STATE_EVERY_LOSS: &str = include_str!("../../../../doc/profiles/state-every-loss.toml");
const NO_DATA_LOSS: &str = include_str!("../../../../doc/profiles/no-data-loss.toml");

const WELL_FORMED: &str = "metadata/xmp-packets-well-formed";

const TARGETS: [Target; 2] = [
    Target::Two(Level::B),
    Target::Four(pdf_archive::Flavour::Plain),
];

fn planned(profile: &str, target: Target) -> ArchivePlan {
    let config =
        pdf_transform::archive::Configuration::read(profile, target).expect("the profile reads");
    ArchivePlan {
        authorised: config.authorisations(target),
        ..plan_from(profile, target)
    }
}

/// A packet whose header's `begin` holds U+0000 where ISO 16684-1 puts the byte-order mark, as
/// `balloon_a1b_jp2k`'s does, with `control` inside one value and a CRLF line break before its
/// trailer. Its `rdf:RDF` is never closed, so section 6.6.2.1's row fails at it and the packet
/// is replaced whatever the checker makes of the characters.
fn a_packet_holding(control: char) -> String {
    format!(
        "<?xpacket begin=\"\u{0}\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?>\n\
         <x:xmpmeta xmlns:x=\"adobe:ns:meta/\">\n\
         <rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\n\
         <rdf:Description rdf:about=\"\" xmlns:dc=\"http://purl.org/dc/elements/1.1/\">\n\
         <dc:format>image/{control}jpeg</dc:format>\n\
         </rdf:Description>\n</x:xmpmeta>\r\n<?xpacket end=\"w\"?>"
    )
}

/// The byte offset of the first character `begin`'s value holds: `<?xpacket begin="` is 17 bytes.
const BEGIN_OFFSET: usize = 17;

fn a_document_holding(control: char) -> Vec<u8> {
    let packet = a_packet_holding(control);
    super::Conforming {
        metadata: super::Packet::Unnamed,
        catalog: "/Metadata 6 0 R".to_owned(),
        objects: vec![format!(
            "<< /Type /Metadata /Subtype /XML /Length {} >>\nstream\n{packet}\nendstream",
            packet.len()
        )],
        ..super::Conforming::part_two()
    }
    .build()
}

/// The text of every page after the source's one: the packet's pages, then the statement's.
fn appended_text(output: &[u8]) -> String {
    let document = Document::open_with_limits(output.to_vec(), Limits::DEFAULT).expect("it opens");
    let pages = pdf_model::Pages::new(&document);
    (1..pages.len())
        .map(|index| {
            let page = pages.get(index).expect("the page exists");
            pdf_model::interpret(&document, &page).text
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn the_packet_of(output: &[u8]) -> String {
    let held = Document::open_with_limits(output.to_vec(), Limits::DEFAULT).expect("it opens");
    let catalog = held.catalog().expect("a catalog");
    let metadata = held.get_key(&catalog, "Metadata");
    let metadata = metadata.as_stream().expect("a metadata stream");
    let bytes = held.decoded_stream_data(metadata).expect("the packet");
    String::from_utf8_lossy(&bytes).into_owned()
}

#[test]
fn a_control_character_is_left_off_the_packet_page_and_stated() {
    let source = a_document_holding('\u{b}');
    let vertical_tab_at = a_packet_holding('\u{b}')
        .find('\u{b}')
        .expect("the packet holds it");
    for target in TARGETS {
        let (report, output) = convert_with_plan(&source, &planned(STATE_EVERY_LOSS, target));
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

        // The report names each character on the stream it came from, in code-point order.
        let replaced = &conversion(&report).replaced_packets;
        let omitted: Vec<_> = replaced
            .iter()
            .flat_map(|packet| packet.omitted.iter())
            .collect();
        assert_eq!(omitted.len(), 2, "{target:?}: {replaced:?}");
        assert!(
            replaced.iter().all(|packet| packet.removed.is_none()),
            "{target:?}: the packet is kept, not removed"
        );
        let (nul, tab) = (omitted[0], omitted[1]);
        assert_eq!(
            (nul.character, nul.count, nul.offsets.as_slice()),
            ('\u{0}', 1, [BEGIN_OFFSET].as_slice()),
            "{target:?}"
        );
        assert_eq!(
            (tab.character, tab.count, tab.offsets.as_slice()),
            ('\u{b}', 1, [vertical_tab_at].as_slice()),
            "{target:?}"
        );
        assert!(
            omitted
                .iter()
                .all(|it| it.at.number == 6 && it.stated_on.is_some()),
            "{target:?}: {omitted:?}"
        );

        // The packet's page sets the rest of the value, and the line break still divides lines.
        let text = appended_text(&output);
        assert!(text.contains("image/jpeg"), "{target:?}: {text:?}");
        assert!(
            text.contains("</x:xmpmeta>") && !text.contains("</x:xmpmeta><?xpacket"),
            "{target:?}: the CRLF still ends a line: {text:?}"
        );
        assert!(
            !text.contains('\u{0}') && !text.contains('\u{b}'),
            "{target:?}: no control character is set"
        );

        // The statement lists each, by its code point and never as itself. The fixture's page
        // is narrow, so the measure breaks lines inside a fact; the breaks are taken out.
        let unbroken = text.replace('\n', "");
        for fact in [
            "Loss: metadata-packet-character",
            "Object: metadata stream 6 0",
            "Character: U+0000",
            "Character: U+000B",
            "Occurrences: 1",
            &format!("Byte offsets in the decoded packet: {BEGIN_OFFSET}"),
            &format!("Byte offsets in the decoded packet: {vertical_tab_at}"),
        ] {
            assert!(unbroken.contains(fact), "{target:?}: {fact:?} in {text:?}");
        }

        // And `xmpMM:History` records it.
        assert!(
            the_packet_of(&output).contains("(metadata-packet-character)"),
            "{target:?}: the history names the omission"
        );
    }
}

#[test]
fn no_data_loss_keeps_refusing_a_control_character() {
    let source = a_document_holding('\u{b}');
    for target in TARGETS {
        let (report, output) = convert_with_plan(&source, &planned(NO_DATA_LOSS, target));
        assert!(
            matches!(
                decision(&report, WELL_FORMED),
                Decision::Refused(Because::NotBuiltYet(why)) if why.contains("typeface")
            ),
            "{target:?}: {:?}",
            decision(&report, WELL_FORMED)
        );
        assert!(output.is_none(), "{target:?}: nothing is written");
    }
}

#[test]
fn a_character_outside_c0_keeps_the_face_refusal() {
    // U+007F is a control character, but not one of C0's: the second step takes C0 only (A778),
    // so DELETE still asks the face search for a glyph and is refused there.
    let source = a_document_holding('\u{7f}');
    for target in TARGETS {
        let (report, output) = convert_with_plan(&source, &planned(STATE_EVERY_LOSS, target));
        assert!(
            matches!(
                decision(&report, WELL_FORMED),
                Decision::Refused(Because::NotBuiltYet(why)) if why.contains("typeface")
            ),
            "{target:?}: {:?}",
            decision(&report, WELL_FORMED)
        );
        assert!(output.is_none(), "{target:?}: nothing is written");
    }
}
