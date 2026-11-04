//! A document information dictionary key Table 349 does not define, carried into the packet
//! (`doc/adr/2680`).
//!
//! ISO 19005-4 section 6.1.3 leaves `/Info` no entry but `/ModDate`, and ISO 32000-2 §14.3.3 sends
//! "any other document level metadata" to a metadata stream. Table 349's NOTEs name a property for
//! each of its own keys and none for a key it does not define, so `unmapped = "carry"` writes such
//! a value into the packet unchanged under this converter's namespace for a dictionary key, whose
//! local name spells the key. ISO 19005-4 section 6.7.2 admits any well-formed XMP and asks for no
//! description of the schema. The value is kept, so both stating profiles take it; a value that is
//! not a text string has no carried form. `state-every-loss` then removes it by the row's second
//! step and states it on the appended page (`doc/adr/2681`); `no-data-loss` names no second step
//! and keeps the refusal; an operator's `unreadable = "discard"` drops it and the report names it.
use super::{
    ArchivePlan, Decision, Rewrite, Target, Verdict, a_document_stating_information, conversion,
    convert_with_plan, decision, document_packet, holds, plan_from,
};
use pdf_syntax::{Document, Limits};
use pdf_transform::archive::{
    CARRIED_INFORMATION_NAMESPACE, CARRIED_INFORMATION_PREFIX, carried_local_name,
};

const STATE_EVERY_LOSS: &str = include_str!("../../../../doc/profiles/state-every-loss.toml");
const NO_DATA_LOSS: &str = include_str!("../../../../doc/profiles/no-data-loss.toml");

const PROFILES: [(&str, &str); 2] = [
    ("state-every-loss", STATE_EVERY_LOSS),
    ("no-data-loss", NO_DATA_LOSS),
];

const TARGET: Target = Target::Four(pdf_archive::Flavour::Plain);

const NEEDS_PIECE_INFO: &str = "file-structure/document-information-dictionary-needs-piece-info";

fn planned(profile: &str) -> ArchivePlan {
    let config =
        pdf_transform::archive::Configuration::read(profile, TARGET).expect("the profile reads");
    ArchivePlan {
        authorised: config.authorisations(TARGET),
        ..plan_from(profile, TARGET)
    }
}

/// The inverse of `carried_local_name`'s escape, written from its documented rule alone.
fn key_of(local: &str) -> Vec<u8> {
    // `_` only delimits an escape, so the pieces between them alternate: the even ones stand for
    // themselves and the odd ones are an escape's two hexadecimal digits.
    let mut key = Vec::new();
    for (index, piece) in local.split('_').enumerate() {
        if index % 2 == 0 {
            key.extend_from_slice(piece.as_bytes());
        } else {
            assert_eq!(piece.len(), 2, "an escape is two digits: {local}");
            key.push(u8::from_str_radix(piece, 16).expect("two hexadecimal digits"));
        }
    }
    key
}

#[test]
fn a_key_is_spelled_as_an_xml_name_from_which_it_is_recovered_byte_for_byte() {
    for (key, local) in [
        (&b"WPS-ARTICLEDOI"[..], "WPS-ARTICLEDOI"),
        (b"PTEX.Fullbanner", "PTEX.Fullbanner"),
        (b"Base Target", "Base_20_Target"),
        (b"_AuthorEmail", "_5F_AuthorEmail"),
        (b"1st", "_31_st"),
        (b"-x", "_2D_x"),
        (b"a_5F_b", "a_5F_5F_5F_b"),
        (b"Gr\xC3\xB6\xC3\x9Fe", "Gr_C3__B6__C3__9F_e"),
        (b"\xFF", "_FF_"),
    ] {
        let spelled = carried_local_name(key).expect("a key that is not empty is spelled");
        assert_eq!(spelled, local);
        // An XML NCName: a letter or `_` first, then letters, digits, `-`, `.` and `_`.
        assert!(
            spelled.starts_with(|first: char| first.is_ascii_alphabetic() || first == '_')
                && spelled
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '.' | '_')),
            "{spelled}"
        );
        assert_eq!(key_of(&spelled), key, "{spelled} gives its key back");
    }
    assert_eq!(
        carried_local_name(b""),
        None,
        "no XML name spells the empty key"
    );
}

#[test]
fn both_stating_profiles_carry_a_key_table_349_does_not_define() {
    // `241573_ftp`'s three `WPS-` keys, a key with a space as the corpus's `Base Target`, and a
    // leading underscore as `_AuthorEmail`, beside a key Table 349 does define.
    let source = a_document_stating_information(
        "/Title (A title) /WPS-ARTICLEDOI (10.1107/S2053273314007578) \
         /Base#20Target (_top) /_AuthorEmail (a&b <c>)",
    );
    for (name, profile) in PROFILES {
        let (report, output) = convert_with_plan(&source, &planned(profile));
        let output = output.unwrap_or_else(|| panic!("{name}: it converts: {report:#?}"));
        assert_eq!(
            holds(&output, TARGET).verdict(),
            Verdict::Conforms,
            "{name}"
        );
        assert!(
            matches!(
                decision(&report, NEEDS_PIECE_INFO),
                Decision::Stated {
                    rewrite: Rewrite::InformationMovedIntoThePacket,
                    ..
                }
            ),
            "{name}: {:?}",
            decision(&report, NEEDS_PIECE_INFO)
        );

        let packet = document_packet(&output);
        let xmp = pdf_model::xmp::Xmp::parse(&packet).expect("the packet parses");
        for (local, value) in [
            ("WPS-ARTICLEDOI", "10.1107/S2053273314007578"),
            ("Base_20_Target", "_top"),
            ("_5F_AuthorEmail", "a&b <c>"),
        ] {
            assert_eq!(
                xmp.text(CARRIED_INFORMATION_NAMESPACE, local),
                Some(value),
                "{name}: {local} carries the dictionary's value unchanged"
            );
        }
        assert_eq!(
            xmp.text(pdf_model::xmp::DC, "title"),
            Some("A title"),
            "{name}: a Table 349 key still goes where its NOTE names"
        );

        let moved = &conversion(&report).moved_information;
        for (key, local) in [
            ("WPS-ARTICLEDOI", "WPS-ARTICLEDOI"),
            ("Base Target", "Base_20_Target"),
            ("_AuthorEmail", "_5F_AuthorEmail"),
        ] {
            let property = format!("{CARRIED_INFORMATION_PREFIX}:{local}");
            assert!(
                moved.iter().any(|entry| entry.key == key
                    && entry.property.as_deref() == Some(property.as_str())
                    && entry.fate.contains("carry")),
                "{name}: the report names {key} and where it went: {moved:?}"
            );
        }
    }
}

/// The values §14.3.3 and XML leave no carried form for, each with the type the page names.
///
/// §14.3.3: the value of a key Table 349 does not define "shall be a text string". An integer is
/// not one, nor is a string holding a byte Table D.2 leaves undefined; and a control character XML
/// 1.0 excludes, or a carriage return its end-of-line handling turns into a line feed, has no
/// carried form that is the value as stated.
const NOT_CARRIED: [(&str, &str); 4] = [
    ("/Title (A title) /Batch 4417", "integer"),
    ("/Title (A title) /Batch (a\\001b)", "string of 3 bytes"),
    (
        "/Title (A title) /Batch <FEFF006100010062>",
        "string of 8 bytes",
    ),
    ("/Title (A title) /Batch (a\\rb)", "string of 3 bytes"),
];

/// The report's sentence for its statement page, empty where it reports none.
fn stated_sentence(conversion: &pdf_transform::archive::Conversion) -> String {
    conversion
        .statement
        .as_ref()
        .map(pdf_transform::archive::StatementPage::sentence)
        .unwrap_or_default()
}

/// The page count, and the text of every page after the source's one: the statement.
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

#[test]
fn state_every_loss_removes_and_states_a_value_that_cannot_be_carried() {
    // `241573_ftp`'s `/SaveStreams` dictionary and pdf.js issue15910's `/Copies 0`, beside a key
    // the packet carries and a key Table 349 defines.
    let source = a_document_stating_information(
        "/Title (A title) /WPS-ARTICLEDOI (10.1634/x) /SaveStreams << /q 1 /Q 2 >> /Copies 0",
    );
    let (report, output) = convert_with_plan(&source, &planned(STATE_EVERY_LOSS));
    let output = output.unwrap_or_else(|| panic!("it converts: {report:#?}"));
    assert_eq!(holds(&output, TARGET).verdict(), Verdict::Conforms);
    assert!(
        matches!(
            decision(&report, NEEDS_PIECE_INFO),
            Decision::Stated {
                rewrite: Rewrite::InformationMovedIntoThePacket,
                ..
            }
        ),
        "{:?}",
        decision(&report, NEEDS_PIECE_INFO)
    );

    // The carriable key is still carried.
    let packet = document_packet(&output);
    let xmp = pdf_model::xmp::Xmp::parse(&packet).expect("the packet parses");
    assert_eq!(
        xmp.text(CARRIED_INFORMATION_NAMESPACE, "WPS-ARTICLEDOI"),
        Some("10.1634/x")
    );
    assert_eq!(xmp.text(CARRIED_INFORMATION_NAMESPACE, "SaveStreams"), None);
    assert_eq!(xmp.text(CARRIED_INFORMATION_NAMESPACE, "Copies"), None);

    // The report names each removal, with the facts the page states and the page it is on.
    let moved = &conversion(&report).moved_information;
    for (key, value) in [
        ("SaveStreams", "dictionary of 2 entries"),
        ("Copies", "integer"),
    ] {
        let entry = moved
            .iter()
            .find(|entry| entry.key == key)
            .unwrap_or_else(|| panic!("the report names {key}: {moved:?}"));
        assert!(entry.property.is_none(), "{key}");
        assert!(
            entry.fate.contains("statement page"),
            "{key}: {}",
            entry.fate
        );
        let removed = entry
            .removed
            .as_ref()
            .unwrap_or_else(|| panic!("{key}: the removal is reported"));
        assert_eq!(removed.key, format!("/{key}"));
        assert_eq!(removed.value, value);
        assert_eq!(removed.length, None);
        assert_eq!(removed.stated_on, Some(1), "{key}: on the appended page");
    }
    assert!(
        moved
            .iter()
            .any(|entry| entry.key == "WPS-ARTICLEDOI" && entry.removed.is_none()),
        "{moved:?}"
    );

    // The statement page lists both, in facts computed from the source and never the value.
    let (count, text) = pages_of(&output);
    assert!(count > 1, "the statement is appended");
    for fact in [
        "document-information-entry",
        "Key: /SaveStreams",
        "Value: dictionary of 2 entries",
        "Key: /Copies",
        "Value: integer",
    ] {
        assert!(text.contains(fact), "{fact:?} in {text:?}");
    }
    assert!(!text.contains("10.1634/x"), "a carried value is not stated");

    // And `xmpMM:History` records each removal.
    let packet = String::from_utf8_lossy(&packet);
    for key in ["/SaveStreams", "/Copies"] {
        let sentence = format!("entry {key} (value: ");
        assert!(packet.contains(&sentence), "the history names {key}");
    }
    assert!(packet.contains("is removed (document-information-entry)"));
    // The report's sentence counts them as keys, not objects.
    let said = stated_sentence(conversion(&report));
    assert!(said.contains("listing 2 removed key(s) under "), "{said}");

    // Each value without a carried form takes the same step, named by its type and size.
    for (entries, value) in NOT_CARRIED {
        let source = a_document_stating_information(entries);
        let (report, output) = convert_with_plan(&source, &planned(STATE_EVERY_LOSS));
        let output = output.unwrap_or_else(|| panic!("{entries}: it converts: {report:#?}"));
        assert_eq!(
            holds(&output, TARGET).verdict(),
            Verdict::Conforms,
            "{entries}"
        );
        let removed = conversion(&report)
            .moved_information
            .iter()
            .find_map(|entry| entry.removed.clone())
            .unwrap_or_else(|| panic!("{entries}: the removal is reported"));
        assert_eq!(removed.value, value, "{entries}");
        let (_, text) = pages_of(&output);
        assert!(
            text.contains(&format!("Value: {value}")),
            "{entries}: {text}"
        );
    }
}

#[test]
fn no_data_loss_refuses_a_value_that_cannot_be_carried_by_name() {
    // `no-data-loss` names no second step and no `unreadable`, so it keeps the refusal: the value
    // would not be kept.
    let mut documents: Vec<&str> = NOT_CARRIED.iter().map(|(entries, _)| *entries).collect();
    documents.push(
        "/Title (A title) /WPS-ARTICLEDOI (10.1634/x) /SaveStreams << /q 1 /Q 2 >> /Copies 0",
    );
    for entries in documents {
        let source = a_document_stating_information(entries);
        let (report, output) = convert_with_plan(&source, &planned(NO_DATA_LOSS));
        assert!(output.is_none(), "nothing is written for {entries}");
        let Decision::Refused(because) = decision(&report, NEEDS_PIECE_INFO) else {
            panic!(
                "refused by name for {entries}: {:?}",
                decision(&report, NEEDS_PIECE_INFO)
            );
        };
        assert!(
            because.sentence().contains("`unmapped = \"carry\"`")
                && because.sentence().contains("`unreadable`"),
            "{}",
            because.sentence()
        );
    }
}

#[test]
fn an_operator_who_accepts_the_loss_drops_a_value_that_cannot_be_carried() {
    // An operator who accepts the loss says so, and the report names the key.
    let config = "[site.\"file-structure/document-information-dictionary-needs-piece-info\"]\n\
         remedy = \"preserve\"\nunmapped = \"carry\"\nunreadable = \"discard\"\n\
         [site.\"file-structure/document-information-dictionary-holds-only-a-modification-date\"]\n\
         remedy = \"preserve\"\nunmapped = \"carry\"\nunreadable = \"discard\"\n";
    let source = a_document_stating_information("/Title (A title) /Batch 4417 /Lot (7)");
    let (report, output) = convert_with_plan(&source, &plan_from(config, TARGET));
    let output = output.expect("the operator's answer converts it");
    assert_eq!(holds(&output, TARGET).verdict(), Verdict::Conforms);
    let packet = document_packet(&output);
    let xmp = pdf_model::xmp::Xmp::parse(&packet).expect("the packet parses");
    assert_eq!(xmp.text(CARRIED_INFORMATION_NAMESPACE, "Lot"), Some("7"));
    assert_eq!(xmp.text(CARRIED_INFORMATION_NAMESPACE, "Batch"), None);
    let moved = &conversion(&report).moved_information;
    assert!(
        moved
            .iter()
            .any(|entry| entry.key == "Batch" && entry.property.is_none()),
        "the report names the key whose value left: {moved:?}"
    );
}
