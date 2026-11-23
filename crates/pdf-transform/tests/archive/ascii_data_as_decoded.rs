//! A stream whose ASCII-armoured data stops before its end-of-data marker is written as the bytes
//! this reader decoded, behind the loss `ascii-data-as-decoded`, under both shipped profiles at
//! both parts (`doc/adr/3462`).
//!
//! ISO 32000-2 §7.4.3 makes a final partial group the encoding of the last bytes only where the
//! `~>` end-of-data marker follows it, and §7.4.2 ends hexadecimal data at `>`; ISO 19005-2 and ISO
//! 19005-4 section 5.1 bind both through the base standard. The expected bytes are the complete
//! groups' decoding, which is what this reader draws (`doc/adr/3281`): the group the cut ended
//! states none.

use super::*;
use pdf_transform::profiles::{Contribution, named};

const SITE: &str = "file-structure/ascii-data-ends-at-its-end-of-data-marker";

const TARGETS: [Target; 2] = [Target::Two(Level::B), Target::Four(Flavour::Plain)];

/// `0 0 10 10 re f\n\n%%` in base-85: four complete groups and a partial one of three
/// characters, with no `~>` after it.
const CUT_BASE_85: &[u8] = b"0H`%l0ea_)0HbCI+D(rf,p`";

/// The bytes the four complete groups encode, which is all §7.4.3 lets the data state.
const DECODED: &[u8] = b"0 0 10 10 re f\n\n";

/// A page whose one content stream is `data` under `filter`.
fn a_page_under(target: Target, filter: &str, data: &[u8]) -> Vec<u8> {
    let base = match target {
        Target::Two(_) => Conforming::part_two(),
        Target::Four(_) => Conforming::default(),
    };
    Conforming {
        contents: Some((format!("/Filter /{filter}"), data.to_vec())),
        ..base
    }
    .build()
}

/// What a shipped profile contributes at `target`.
fn authorised_by(profile: &str, target: Target) -> Authorisations {
    Contribution::read(named(profile).expect("a shipped profile").text, target)
        .expect("a shipped profile reads")
        .authorised
}

fn fails_the_row(bytes: &[u8], target: Target) -> bool {
    holds(bytes, target)
        .failures()
        .any(|judgement| judgement.id == SITE)
}

#[test]
fn data_ending_at_its_marker_passes_and_data_cut_before_it_fails() {
    for target in TARGETS {
        let mut whole = CUT_BASE_85.to_vec();
        whole.extend_from_slice(b"~>");
        assert!(!fails_the_row(
            &a_page_under(target, "ASCII85Decode", &whole),
            target
        ));
        assert!(fails_the_row(
            &a_page_under(target, "ASCII85Decode", CUT_BASE_85),
            target
        ));
        assert!(!fails_the_row(
            &a_page_under(target, "ASCIIHexDecode", b"3020>"),
            target
        ));
        assert!(fails_the_row(
            &a_page_under(target, "ASCIIHexDecode", b"3020"),
            target
        ));
    }
}

#[test]
fn both_shipped_profiles_authorise_the_loss_at_both_parts() {
    for profile in ["as-if-printed", "as-shown"] {
        for target in TARGETS {
            assert!(
                authorised_by(profile, target).grants(Loss::AsciiDataAsDecoded),
                "{profile} at {target}"
            );
        }
    }
}

#[test]
fn cut_base_85_data_is_written_as_the_bytes_decoded_under_each_profile() {
    for profile in ["as-if-printed", "as-shown"] {
        for target in TARGETS {
            let source = a_page_under(target, "ASCII85Decode", CUT_BASE_85);
            let (report, output) = convert(&source, target, authorised_by(profile, target));
            assert_eq!(
                decision(&report, SITE),
                Decision::Authorised {
                    losses: Losses::one(Loss::AsciiDataAsDecoded),
                    rewrite: Rewrite::AsciiDataAsDecoded,
                },
                "{profile} at {target}"
            );
            let output = written(&report, output);
            let held = holds(&output, target);
            assert_eq!(held.verdict(), Verdict::Conforms, "{}", held.render());
            assert_eq!(page_contents(&output), vec![DECODED.to_vec()]);
            let restated = &conversion(&report).ascii_data_restated;
            assert_eq!(restated.len(), 1, "{profile} at {target}");
            assert_eq!(restated[0].stage, "ASCII85Decode");
            assert_eq!(restated[0].encoded, CUT_BASE_85.len());
            assert!(
                conversion(&report)
                    .render()
                    .contains("losing ascii-data-as-decoded"),
                "the report names the loss"
            );
        }
    }
}

/// The data stops a digit into its fourteenth pair: the thirteen whole pairs are what is written.
#[test]
fn cut_hexadecimal_data_is_written_as_the_pairs_decoded() {
    for target in TARGETS {
        let source = a_page_under(target, "ASCIIHexDecode", b"302030203130203130207265206");
        let (report, output) = convert(&source, target, authorised_by("as-shown", target));
        let output = written(&report, output);
        assert_eq!(holds(&output, target).verdict(), Verdict::Conforms);
        assert_eq!(page_contents(&output), vec![b"0 0 10 10 re ".to_vec()]);
        assert_eq!(
            conversion(&report).ascii_data_restated[0].stage,
            "ASCIIHexDecode"
        );
    }
}

#[test]
fn nothing_is_written_without_the_loss() {
    for target in TARGETS {
        let source = a_page_under(target, "ASCII85Decode", CUT_BASE_85);
        let (report, output) = convert(&source, target, Authorisations::default());
        assert!(
            matches!(
                decision(&report, SITE),
                Decision::Unauthorised {
                    losses,
                    rewrite: Rewrite::AsciiDataAsDecoded,
                } if losses == Losses::one(Loss::AsciiDataAsDecoded)
            ),
            "{target}: {:?}",
            decision(&report, SITE)
        );
        assert!(output.is_none());
    }
}

/// `/DecodeParms []` beside a chain of one: the reader decodes the stage with no parameters, so the
/// stream is restated as for no entry at all (pdf.js's `xobject-image`, whose image states
/// `/DecodeParms [ ]` beside `[/ASCIIHexDecode /DCTDecode]`).
#[test]
fn a_parameter_array_shorter_than_the_chain_is_read_as_no_parameters() {
    for target in TARGETS {
        let base = match target {
            Target::Two(_) => Conforming::part_two(),
            Target::Four(_) => Conforming::default(),
        };
        let source = Conforming {
            contents: Some((
                "/Filter [/ASCII85Decode] /DecodeParms []".to_owned(),
                CUT_BASE_85.to_vec(),
            )),
            ..base
        }
        .build();
        let (report, output) = convert(&source, target, authorised_by("as-shown", target));
        let output = written(&report, output);
        assert_eq!(holds(&output, target).verdict(), Verdict::Conforms);
        assert_eq!(page_contents(&output), vec![DECODED.to_vec()]);
    }
}

/// Each stream restated is named in the output's own `xmpMM:History`, by the sentence the report
/// gives it, as every other loss's construction is: ISO 19005-2 section 6.6.6 and ISO 19005-4
/// section 6.7.5 ask a converter to record what it did (`doc/adr/3494`).
#[test]
fn each_stream_restated_is_named_in_the_files_own_history() {
    for profile in ["as-if-printed", "as-shown"] {
        for target in TARGETS {
            let source = a_page_under(target, "ASCII85Decode", CUT_BASE_85);
            let (report, output) = convert(&source, target, authorised_by(profile, target));
            let output = written(&report, output);
            let restated = &conversion(&report).ascii_data_restated;
            let packet = String::from_utf8(document_packet(&output)).expect("the packet is UTF-8");
            assert!(
                packet.contains("xmpMM:History"),
                "{profile} at {target}:\n{packet}\n{}",
                conversion(&report).render()
            );
            assert!(
                packet.contains(&restated[0].sentence()),
                "{profile} at {target}: the entry names the stream:\n{packet}"
            );
        }
    }
}
