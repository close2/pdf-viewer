//! ISO 19005-2 and ISO 19005-4 section 5.1 over the reader's log of repairs, answered by what the
//! repair was (ADR 3521, ADR 3522).
//!
//! Section 5.1 requires a conforming file to conform to the base standard. ISO 32000-2 §7.4.4
//! makes RFC 1950 and RFC 1951 the definition of `FlateDecode` data, so data that stops before its
//! final block states nothing past the cut, and this reader draws the bytes it decoded before it.
//! The stream is written as those bytes at the loss `damaged-data-as-decoded`. A literal string
//! with no closing parenthesis inside a content stream is §7.3.4.2's departure in bytes this verb
//! carries unchanged, so it is refused by name.

use super::*;
use pdf_transform::profiles::{Contribution, named};

const SITE: &str = "conformance/adheres-to-the-base-standard";

const TARGETS: [Target; 2] = [Target::Two(Level::B), Target::Four(Flavour::Plain)];

/// A content stream long enough that its compressed form has bytes to cut: one painted square,
/// then white space, which draws nothing wherever the cut falls in it.
fn content() -> Vec<u8> {
    let mut content = b"0 0 10 10 re f\n".to_vec();
    content.extend(std::iter::repeat_n(b' ', 4096));
    content
}

/// [`content`] compressed, with the last bytes of its final block and its check value cut off.
fn cut_flate() -> Vec<u8> {
    let whole = pdf_syntax::serialize::flate_encode(&content(), 0).expect("flate encodes");
    whole[..whole.len() / 2].to_vec()
}

/// A page whose one content stream is `data` under `dict`.
fn a_page_under(target: Target, dict: &str, data: &[u8]) -> Vec<u8> {
    let base = match target {
        Target::Two(_) => Conforming::part_two(),
        Target::Four(_) => Conforming::default(),
    };
    Conforming {
        contents: Some((dict.to_owned(), data.to_vec())),
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

/// The bytes this reader decodes from the source's one content stream, which is what it draws.
fn decoded_from(source: &[u8]) -> Vec<u8> {
    let document = Document::open(source.to_vec()).expect("the source opens");
    let page = pdf_model::Pages::new(&document)
        .get(0)
        .expect("the source has a page");
    let stream = document
        .get_key(&page.dict, "Contents")
        .as_stream()
        .cloned()
        .expect("a content stream");
    document
        .decoded_stream_data_reported(&stream)
        .expect("the stream decodes in part")
        .data
        .to_vec()
}

#[test]
fn both_shipped_profiles_authorise_the_loss_at_both_parts() {
    for profile in ["as-if-printed", "as-shown", "state-every-loss"] {
        for target in TARGETS {
            assert!(
                authorised_by(profile, target).grants(Loss::DamagedDataAsDecoded),
                "{profile} at {target}"
            );
        }
    }
}

#[test]
fn cut_flate_data_is_written_as_the_bytes_decoded_under_each_profile() {
    for profile in ["as-if-printed", "as-shown"] {
        for target in TARGETS {
            let source = a_page_under(target, "/Filter /FlateDecode", &cut_flate());
            assert!(
                holds(&source, target)
                    .failures()
                    .any(|judgement| judgement.id == SITE),
                "the cut fails section 5.1"
            );
            let (report, output) = convert(&source, target, authorised_by(profile, target));
            assert_eq!(
                decision(&report, SITE),
                Decision::Authorised {
                    losses: Losses::one(Loss::DamagedDataAsDecoded),
                    rewrite: Rewrite::DamagedDataAsDecoded,
                },
                "{profile} at {target}"
            );
            let output = written(&report, output);
            let held = holds(&output, target);
            assert_eq!(held.verdict(), Verdict::Conforms, "{}", held.render());
            assert_eq!(page_contents(&output), vec![decoded_from(&source)]);
            let restated = &conversion(&report).damaged_data_restated;
            assert_eq!(restated.len(), 1, "{profile} at {target}");
            assert_eq!(restated[0].stage, "FlateDecode");
            assert_eq!(restated[0].encoded, cut_flate().len());
            assert!(
                conversion(&report)
                    .render()
                    .contains("losing damaged-data-as-decoded"),
                "the report names the loss"
            );
            let packet = String::from_utf8(document_packet(&output)).expect("the packet is UTF-8");
            assert!(
                packet.contains(&restated[0].sentence()),
                "{profile} at {target}: the history names the stream:\n{packet}"
            );
        }
    }
}

#[test]
fn nothing_is_written_without_the_loss() {
    for target in TARGETS {
        let source = a_page_under(target, "/Filter /FlateDecode", &cut_flate());
        let (report, output) = convert(&source, target, Authorisations::default());
        assert!(
            matches!(
                decision(&report, SITE),
                Decision::Unauthorised {
                    losses,
                    rewrite: Rewrite::DamagedDataAsDecoded,
                } if losses == Losses::one(Loss::DamagedDataAsDecoded)
            ),
            "{target}: {:?}",
            decision(&report, SITE)
        );
        assert!(output.is_none());
    }
}

/// A repair outside content — here a `/Length` that misstates the data, read to `endstream` — is
/// left behind by the whole-file rewrite, so the row loses nothing.
#[test]
fn a_repair_outside_content_is_answered_by_the_rewrite_alone() {
    for target in TARGETS {
        let source = a_page_under(target, "", b"0 0 10 10 re f\n");
        let at = source
            .windows(b"/Length 15".len())
            .position(|window| window == b"/Length 15")
            .expect("the content states its length");
        let mut source = source;
        source[at + b"/Length ".len()..at + b"/Length 15".len()].copy_from_slice(b"99");
        if !holds(&source, target)
            .failures()
            .any(|judgement| judgement.id == SITE)
        {
            continue;
        }
        let (report, output) = convert(&source, target, Authorisations::default());
        assert_eq!(
            decision(&report, SITE),
            Decision::Mechanical(Rewrite::WholeFileRewritten),
            "{target}"
        );
        assert_eq!(
            holds(&written(&report, output), target).verdict(),
            Verdict::Conforms
        );
    }
}

#[test]
fn a_string_left_open_in_a_content_stream_is_refused_by_name() {
    for target in TARGETS {
        let source = a_page_under(target, "", b"BT (left open ET\n");
        let (report, output) = convert(&source, target, authorised_by("as-shown", target));
        let Decision::Refused(because) = decision(&report, SITE) else {
            panic!("{target}: {:?}", decision(&report, SITE));
        };
        assert_eq!(because.word(), "the-fence");
        assert!(
            because.sentence().contains("syntax/string-unterminated")
                && because.sentence().contains("ISO 32000-2 §7.3.4.2")
                && because.sentence().contains("section 5.1"),
            "{}",
            because.sentence()
        );
        assert!(output.is_none());
    }
}

/// Data no filter decodes has no bytes to write, and whether the output carries the stream is
/// another rewrite's to say, so the output's own check refuses it, naming the class (ADR 3521).
#[test]
fn data_no_filter_decodes_is_refused_by_name() {
    for target in TARGETS {
        let source = a_page_under(
            target,
            "/Filter /FlateDecode",
            b"\xff\xff not a zlib stream",
        );
        let (report, output) = convert(&source, target, authorised_by("as-shown", target));
        assert!(output.is_none(), "{target}");
        let refusals = format!("{:?}", report.refused);
        assert!(
            refusals.contains("syntax/stream-data-undecodable")
                || conversion(&report).decided.iter().any(|decided| matches!(
                    decided.decision,
                    Decision::Refused(because)
                        if because.sentence().contains("syntax/stream-data-undecodable")
                )),
            "{target}: {refusals}"
        );
    }
}
