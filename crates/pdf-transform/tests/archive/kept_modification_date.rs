//! The `/ModDate` ISO 19005-4 section 6.1.3 keeps beside a `/PieceInfo`, and the packet's
//! `xmp:ModifyDate` beside it (R18 of `doc/pdfa-blocking-issues.md`, `doc/adr/2748`).
//!
//! ISO 32000-2 §14.3.4 requires the two sources to be "fully equivalent" where both are written,
//! which is a statement about the instant and not its spelling: §7.9.4's grammar and the XMP Date
//! ISO 16684-1 references spell one instant differently, and a field either leaves out reads as
//! §7.9.4's default for it. Where the two name different instants, §14.5 decides which stays:
//! a data dictionary's `LastModified` is compared with the dictionary's entry and nothing in the
//! packet, so the dictionary's stays as written and the packet's is restated from it — a loss of
//! the packet's former value, recorded in the file's own history, under the three profiles that
//! answer `disagreeing = "discard"`; the others keep the refusal.

use super::{
    ArchivePlan, Conforming, Decision, Packet, Target, Verdict, conversion, convert_with_plan,
    decision, document_packet, holds, plan_from,
};
use pdf_syntax::{Document, Limits};

const AS_SHOWN: &str = include_str!("../../../../doc/profiles/as-shown.toml");
const AS_IF_PRINTED: &str = include_str!("../../../../doc/profiles/as-if-printed.toml");
const ONLY_METADATA_LOSS: &str = include_str!("../../../../doc/profiles/only-metadata-loss.toml");
const NO_DATA_LOSS: &str = include_str!("../../../../doc/profiles/no-data-loss.toml");
const STATE_EVERY_LOSS: &str = include_str!("../../../../doc/profiles/state-every-loss.toml");
const KEEP_EVERYTHING: &str = include_str!("../../../../doc/profiles/keep-everything.toml");
const REFUSE_ANY_LOSS: &str = include_str!("../../../../doc/profiles/refuse-any-loss.toml");

/// The profiles that accept the packet's former date as a loss.
const RESTATING: [(&str, &str); 3] = [
    ("as-shown", AS_SHOWN),
    ("as-if-printed", AS_IF_PRINTED),
    ("only-metadata-loss", ONLY_METADATA_LOSS),
];

/// The profiles that do not.
const REFUSING: [(&str, &str); 4] = [
    ("no-data-loss", NO_DATA_LOSS),
    ("state-every-loss", STATE_EVERY_LOSS),
    ("keep-everything", KEEP_EVERYTHING),
    ("refuse-any-loss", REFUSE_ANY_LOSS),
];

const TARGET: Target = Target::Four(pdf_archive::Flavour::Plain);

const SITE: &str = "file-structure/document-information-dictionary-holds-only-a-modification-date";

fn planned(profile: &str) -> ArchivePlan {
    let config =
        pdf_transform::archive::Configuration::read(profile, TARGET).expect("the profile reads");
    ArchivePlan {
        authorised: config.authorisations(TARGET),
        ..plan_from(profile, TARGET)
    }
}

/// A part 4 document whose catalog states a `/PieceInfo`, whose `/Info` states `info_date` as its
/// `/ModDate` beside a title, and whose packet states `packet_date` as `xmp:ModifyDate`.
fn a_piece_info_document(info_date: &str, packet_date: &str) -> Vec<u8> {
    let mut packet = String::new();
    packet.push_str("<?xpacket begin=\"\u{feff}\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?>\n");
    packet.push_str("<x:xmpmeta xmlns:x=\"adobe:ns:meta/\">\n");
    packet.push_str("<rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\n");
    packet.push_str("<rdf:Description rdf:about=\"\" ");
    packet.push_str("xmlns:pdfaid=\"http://www.aiim.org/pdfa/ns/id/\" ");
    packet.push_str("xmlns:xmp=\"http://ns.adobe.com/xap/1.0/\">\n");
    packet.push_str("<pdfaid:part>4</pdfaid:part>\n<pdfaid:rev>2020</pdfaid:rev>\n");
    packet.push_str("<xmp:ModifyDate>");
    packet.push_str(packet_date);
    packet.push_str("</xmp:ModifyDate>\n");
    packet.push_str("</rdf:Description>\n</rdf:RDF>\n</x:xmpmeta>\n<?xpacket end=\"w\"?>");
    Conforming {
        catalog: "/PieceInfo << /Acme << /LastModified (D:20080513123528) \
                  /Private (nothing) >> >>"
            .to_owned(),
        info: Some(format!("/Title (A title) /ModDate ({info_date})")),
        metadata: Packet::Stated(packet),
        ..Conforming::default()
    }
    .build()
}

/// The output's kept `/ModDate`, as its bytes.
fn kept_date(output: &[u8]) -> Vec<u8> {
    let document =
        Document::open_with_limits(output.to_vec(), Limits::DEFAULT).expect("the output parses");
    let pdf_syntax::Object::Dictionary(info) = document.get_key(document.trailer(), "Info") else {
        panic!("the dictionary the clause keeps is still there");
    };
    assert_eq!(info.iter().count(), 1, "holding ModDate alone: {info:?}");
    document
        .get_key(&info, "ModDate")
        .as_string()
        .expect("a string")
        .to_vec()
}

/// The output packet's `xmp:ModifyDate`, and its history's parameters joined.
fn packet_dates(output: &[u8]) -> (Option<String>, String) {
    let packet = document_packet(output);
    let xmp = pdf_model::xmp::Xmp::parse(&packet).expect("the packet parses");
    let text = String::from_utf8_lossy(&packet).into_owned();
    (
        xmp.text(pdf_model::xmp::XMP, "ModifyDate")
            .map(str::to_owned),
        text,
    )
}

#[test]
fn one_instant_spelled_two_ways_is_equivalent_and_nothing_changes() {
    // §7.9.4 defaults an absent second to zero and the W3C forms ISO 16684-1 references may stop
    // after the minutes; an offset moves the clock and not the instant; a fraction of zeros
    // names the whole second. Each pair below is one instant, so no profile loses anything.
    for (info, packet) in [
        ("D:20060922094100-07'00'", "2006-09-22T09:41-07:00"),
        ("D:20140924192303Z", "2014-09-24T21:23:03+02:00"),
        ("D:20140924212303+02'00'", "2014-09-24T21:23:03.000+02:00"),
        ("D:20140924212303+02'00'", "2014-09-24T21:23:03+02:00"),
    ] {
        let source = a_piece_info_document(info, packet);
        for (name, profile) in RESTATING.iter().chain(&REFUSING) {
            let (report, output) = convert_with_plan(&source, &planned(profile));
            let output = output.unwrap_or_else(|| {
                panic!(
                    "{name}, {info} beside {packet}: {}",
                    conversion(&report).render()
                )
            });
            assert_eq!(holds(&output, TARGET).verdict(), Verdict::Conforms);
            assert_eq!(
                kept_date(&output),
                info.as_bytes(),
                "{name}: the producer's entry"
            );
            let (modify, text) = packet_dates(&output);
            assert_eq!(
                modify.as_deref(),
                Some(packet),
                "{name}: the producer's property"
            );
            assert!(
                !text.contains("modification-date"),
                "{name}: nothing recorded"
            );
        }
    }
}

#[test]
fn a_zone_stated_on_one_side_alone_or_a_fraction_is_another_instant() {
    // An absent zone is a producer saying nothing, so equating it with a stated one would choose
    // what was meant; and a fraction that is not zero names no second §7.9.4 can write.
    for (info, packet) in [
        ("D:20140924212303", "2014-09-24T21:23:03Z"),
        ("D:20140924212303Z", "2014-09-24T21:23:03"),
        ("D:20140924212303+02'00'", "2014-09-24T21:23:03.5+02:00"),
        ("D:20140924212303+02'00'", "2014-09-24T21:23:04+02:00"),
    ] {
        let source = a_piece_info_document(info, packet);
        let (report, output) = convert_with_plan(&source, &planned(REFUSE_ANY_LOSS));
        assert!(
            output.is_none(),
            "{info} beside {packet}: nothing is written"
        );
        let decided = decision(&report, SITE);
        let Decision::Refused(because) = decided else {
            panic!("{info} beside {packet}: refused by name: {decided:?}");
        };
        assert!(
            because.sentence().contains("names another instant"),
            "{}",
            because.sentence()
        );
    }
}

#[test]
fn another_instant_is_restated_from_the_dictionary_and_the_former_value_recorded() {
    // §14.5 compares the PieceInfo's LastModified with the dictionary's ModDate, so the entry
    // stays byte for byte and the packet's property takes its instant, spelled as §7.9.4's date
    // is spelled in the packet everywhere else; the value that went is in the history.
    let (info, packet) = ("D:20080515152007+05'30'", "2008-05-13T12:35:28-04:00");
    let source = a_piece_info_document(info, packet);
    for (name, profile) in RESTATING {
        let (report, output) = convert_with_plan(&source, &planned(profile));
        let output = output.unwrap_or_else(|| panic!("{name}: {}", conversion(&report).render()));
        let held = holds(&output, TARGET);
        assert_eq!(
            held.verdict(),
            Verdict::Conforms,
            "{name}: {}",
            held.render()
        );
        assert_eq!(
            kept_date(&output),
            info.as_bytes(),
            "{name}: the entry stays"
        );
        let (modify, text) = packet_dates(&output);
        assert_eq!(
            modify.as_deref(),
            Some("2008-05-15T15:20:07+05:30"),
            "{name}: the packet names the dictionary's instant"
        );
        assert_eq!(
            text.matches("<xmp:ModifyDate>").count(),
            1,
            "{name}: one property"
        );
        let entry = text
            .split("stEvt:parameters")
            .find(|part| part.contains("restated as the document information dictionary"))
            .unwrap_or_else(|| panic!("{name}: no history entry\n{text}"));
        assert!(
            entry.contains("xmp:ModifyDate 2008-05-13T12:35:28-04:00 restated")
                && entry.contains("(modification-date)"),
            "{name}: {entry}"
        );
        let moved = &conversion(&report).moved_information;
        assert!(
            moved
                .iter()
                .any(|entry| entry.key == "ModDate" && entry.fate.contains("disagreeing")),
            "{name}: the report names the answer: {moved:?}"
        );
    }
}

#[test]
fn the_profiles_that_take_no_such_loss_keep_the_refusal() {
    let source = a_piece_info_document("D:20230227192022Z00'00'", "2008-09-02T11:04:34-04:00");
    for (name, profile) in REFUSING {
        let (report, output) = convert_with_plan(&source, &planned(profile));
        assert!(output.is_none(), "{name}: nothing is written");
        let decided = decision(&report, SITE);
        let Decision::Refused(because) = decided else {
            panic!("{name}: refused by name: {decided:?}");
        };
        assert!(
            because.sentence().contains("disagreeing"),
            "{name}: {}",
            because.sentence()
        );
    }
}
