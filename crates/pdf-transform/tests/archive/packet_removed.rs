//! A packet no page can lay out, removed by its row's second step and stated (`doc/adr/2678`).
//!
//! ISO 32000-2 §7.4: "The filter or filters for a stream shall be specified by the Filter entry
//! in the stream's dictionary", so a metadata stream stating no filter holds its bytes as they
//! are, and bytes that are not UTF-8 are no packet a page can set as text (ISO 16684-1 makes a
//! packet Unicode). `state-every-loss` answers the packet rows with `preserve`, `original =
//! "page"` and `on-failure = "append-statement"`: such a packet is removed, the fresh packet
//! replaces it, and the statement page lists the stream, its length and its filters. `no-data-loss`
//! names no second step, so the same document keeps its refusal.
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

/// The stream data, written as a placeholder and then overwritten byte for byte.
const PLACEHOLDER: &[u8] = b"QQQQQQQQQQQQ";

fn planned(profile: &str, target: Target) -> ArchivePlan {
    let config =
        pdf_transform::archive::Configuration::read(profile, target).expect("the profile reads");
    ArchivePlan {
        authorised: config.authorisations(target),
        ..plan_from(profile, target)
    }
}

/// A document whose catalog's metadata stream states no `/Filter` and holds twelve bytes that are
/// not UTF-8 — `0xFF` never begins a UTF-8 sequence — as `241573_ftp`'s object 67 holds Flate data
/// under no filter.
fn a_packet_that_is_not_text() -> Vec<u8> {
    let mut bytes = super::Conforming {
        metadata: super::Packet::Unnamed,
        catalog: "/Metadata 6 0 R".to_owned(),
        objects: vec![format!(
            "<< /Type /Metadata /Subtype /XML /Length {} >>\nstream\n{}\nendstream",
            PLACEHOLDER.len(),
            String::from_utf8_lossy(PLACEHOLDER)
        )],
        ..super::Conforming::part_two()
    }
    .build();
    let at = bytes
        .windows(PLACEHOLDER.len())
        .position(|window| window == PLACEHOLDER)
        .expect("the fixture holds the placeholder");
    bytes[at..at.saturating_add(PLACEHOLDER.len())].fill(0xFF);
    bytes
}

/// The page count, and the text of every page after the source's one: the statement, which the
/// fixture's small page breaks over several.
fn pages_of(output: &[u8]) -> (usize, String) {
    let document = Document::open_with_limits(output.to_vec(), Limits::DEFAULT).expect("it opens");
    let pages = pdf_model::Pages::new(&document);
    let text = (1..pages.len())
        .map(|index| {
            let page = pages.get(index).expect("the page exists");
            pdf_model::interpret(&document, &page).text
        })
        .collect::<Vec<_>>()
        .join("\n");
    (pages.len(), text)
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
fn a_packet_no_page_can_lay_out_is_removed_and_stated() {
    let source = a_packet_that_is_not_text();
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

        // The report names the stream it came from, with the facts the page states.
        let replaced = &conversion(&report).replaced_packets;
        let removed = replaced
            .iter()
            .find_map(|packet| packet.removed.as_ref())
            .unwrap_or_else(|| panic!("{target:?}: the removal is reported: {replaced:?}"));
        assert_eq!(removed.at.number, 6, "{target:?}");
        assert_eq!(removed.length, PLACEHOLDER.len(), "{target:?}");
        assert!(removed.filters.is_empty(), "{target:?}: no filter stated");
        assert_eq!(
            removed.stated_on,
            Some(1),
            "{target:?}: on the appended page"
        );

        // The statement page lists it, in facts computed from the source.
        let (count, text) = pages_of(&output);
        assert!(count > 1, "{target:?}: the statement is appended");
        for fact in [
            "Loss: metadata-packet",
            "Object: metadata stream 6 0",
            "Stream length: 12 bytes",
            "Filters stated: none",
            // The requirement's identifier is wrapped on the narrow page, so its head is asked.
            "1. Requirement: metadata/",
        ] {
            assert!(text.contains(fact), "{target:?}: {fact:?} in {text:?}");
        }

        // And `xmpMM:History` records it.
        assert!(
            the_packet_of(&output).contains("is removed (metadata-packet)"),
            "{target:?}: the history names the removal"
        );
    }
}

#[test]
fn no_data_loss_keeps_refusing_a_packet_no_page_can_lay_out() {
    let source = a_packet_that_is_not_text();
    for target in TARGETS {
        let (report, output) = convert_with_plan(&source, &planned(NO_DATA_LOSS, target));
        assert!(
            matches!(
                decision(&report, WELL_FORMED),
                Decision::Refused(Because::NotBuiltYet(why)) if why.contains("not UTF-8 text")
            ),
            "{target:?}: {:?}",
            decision(&report, WELL_FORMED)
        );
        assert!(output.is_none(), "{target:?}: nothing is written");
    }
}

#[test]
fn a_packet_kept_as_a_file_admits_no_second_step() {
    // An attachment keeps any bytes, so nothing is left for a second step to take: the row is
    // refused when the file is read rather than read and not done (`doc/adr/2678`).
    let profile = "[profile]\nname = \"t\"\nsentence = \"t\"\ndefault = \"stop\"\n\n\
                   [site.\"metadata/xmp-packets-well-formed\"]\nremedy = \"preserve\"\n\
                   fresh-packet = true\noriginal = \"attach\"\non-failure = \"append-statement\"\n";
    let target = Target::Four(pdf_archive::Flavour::F);
    assert!(
        pdf_transform::archive::Configuration::read(profile, target).is_err(),
        "the attaching row names a second step"
    );
    let paged = profile.replace("original = \"attach\"", "original = \"page\"");
    assert!(
        pdf_transform::archive::Configuration::read(&paged, target).is_ok(),
        "the paging row is the one that hands a packet on"
    );
}
