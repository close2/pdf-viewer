//! ISO 32000-2 §13.2.3 to §13.2.7's viability rules, each judged through `pdf_model::media::judge`.
//!
//! The unit tests beside `media.rs` check §13.2.2's two dictionaries, Table 286's permissions and
//! the search through a selector. These check the rules of the subclauses the ledger reads as
//! executed by that judgement, one subclause to a test, so that a row's `implemented` names a test
//! of its own sentence rather than of its neighbour's (ADR 2376). Every fixture is judged for the one
//! player this program has — the hand-off, which is told nothing but the file and is named by no
//! software identifier (ADR 1960).

#![expect(
    clippy::expect_used,
    reason = "test code: a fixture that cannot exercise what the test is about is a failure"
)]

use std::fmt::Write as _;

use pdf_model::media::{Processor, Report, judge};
use pdf_syntax::{Document, Object, ObjectId};

/// Assembles a document from object bodies numbered from 1, with object 1 the catalog.
fn document(objects: &[&str]) -> Document {
    let mut out = String::from("%PDF-2.0\n");
    let mut offsets = Vec::new();
    for (index, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        let _ = write!(out, "{} 0 obj\n{body}\nendobj\n", index.saturating_add(1));
    }
    let xref_at = out.len();
    let _ = write!(
        out,
        "xref\n0 {}\n0000000000 65535 f \n",
        objects.len().saturating_add(1)
    );
    for offset in &offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n",
        objects.len().saturating_add(1)
    );
    Document::open(out.into_bytes()).expect("a valid fixture")
}

/// An embedded file specification whose stream is object 4.
const EMBEDDED: &str = "<< /Type /Filespec /F (clip.mp4) /UF (clip.mp4) /EF << /F 4 0 R >> >>";

/// The embedded file stream [`EMBEDDED`] names.
const FILE: &str = "<< /Length 4 /Type /EmbeddedFile >>\nstream\nmoov\nendstream";

/// Judges the rendition `body` as object 2, beside an embedded file as object 4.
fn judged(body: &str, processor: Processor<'_>) -> Report {
    let doc = document(&["<< /Type /Catalog >>", body, "null", FILE]);
    let rendition = Object::Reference(ObjectId {
        number: 2,
        generation: 0,
    });
    judge(&doc, &rendition, processor)
}

/// Whether the rendition `body` is viable for the hand-off.
fn viable(body: &str) -> bool {
    judged(body, Processor::default()).chosen().is_some()
}

/// A media clip data dictionary the hand-off can take: a content type and embedded data.
fn clip() -> String {
    format!("<< /S /MCD /CT (video/mp4) /D {EMBEDDED} >>")
}

/// §13.2.3.1 and Table 277: a rendition's `/S` decides its type, and a media criteria dictionary
/// binds from either dictionary.
///
/// > The criteria specified by all of its entries shall be met regardless of whether they are in
/// > an MH or a BE dictionary. The only exception is that if an entry in a BE dictionary is
/// > unrecognised by the interactive PDF processor, it shall not affect the viability of the
/// > object.
///
/// And where both dictionaries state an entry, "MH taking precedence (corresponding BE entries
/// shall be ignored)".
#[test]
fn a_media_criterion_binds_from_either_dictionary_and_an_unknown_one_only_from_mh() {
    let clip = clip();
    assert!(viable(&format!("<< /S /MR /C {clip} >>")));
    // Table 277's /S: "The rendition is non-viable if the interactive PDF processor does not
    // recognise the value of this entry."
    let report = judged(&format!("<< /S /XR /C {clip} >>"), Processor::default());
    assert!(report.chosen().is_none());
    assert!(!report.stopped.is_empty(), "{report:?}");
    // Table 279's /R needs a bandwidth this program does not measure, and binds from BE too.
    assert!(!viable(&format!(
        "<< /S /MR /BE << /C << /R 1000 >> >> /C {clip} >>"
    )));
    // An entry Table 279 does not define: ignored in BE, non-viable in MH.
    assert!(viable(&format!(
        "<< /S /MR /BE << /C << /Unknown true >> >> /C {clip} >>"
    )));
    assert!(!viable(&format!(
        "<< /S /MR /MH << /C << /Unknown true >> >> /C {clip} >>"
    )));
    // MH's /L is evaluated and BE's is ignored, whichever of the two the language meets.
    let both =
        format!("<< /S /MR /MH << /C << /L [(fr)] >> >> /BE << /C << /L [(en)] >> >> /C {clip} >>");
    let english = Processor {
        language: Some("en"),
    };
    let french = Processor {
        language: Some("fr"),
    };
    assert!(judged(&both, english).chosen().is_none());
    assert!(judged(&both, french).chosen().is_some());
    // Table 279's /V binds only "[i]f this entry is present and non-empty", and a non-empty one
    // names a processor this program, which has no software name, is not.
    assert!(viable(&format!(
        "<< /S /MR /MH << /C << /V [] >> >> /C {clip} >>"
    )));
    assert!(!viable(&format!(
        "<< /S /MR /MH << /C << /V [<< /U (vnd.adobe.swname:ADBE_Acrobat) >>] >> >> /C {clip} >>"
    )));
}

/// §13.2.3.2: "A media rendition object shall be viable if and only if the objects referenced by
/// its C , P , and SP entries are viable", and a rendition with no clip names a player that takes
/// no input, which the hand-off is not.
#[test]
fn a_media_rendition_is_viable_only_if_its_clip_and_its_parameters_are() {
    let clip = clip();
    assert!(viable(&format!(
        "<< /S /MR /C {clip} /P << /BE << /V 50 >> >> /SP << /BE << /W 0 >> >> >>"
    )));
    assert!(!viable(&format!(
        "<< /S /MR /C {clip} /SP << /MH << /W 0 >> >> >>"
    )));
    assert!(!viable(
        "<< /S /MR /P << /PL << /A [<< /PID << /U (vnd.adobe.swname:ADBE_MCI) >> >>] >> >> >>"
    ));
}

/// §13.2.3.3: "A selector rendition itself may be non-viable; in this case, none of its
/// associated media renditions shall be evaluated", and Table 283 says "An empty array is valid".
#[test]
fn a_non_viable_selector_evaluates_none_of_its_renditions() {
    let clip = clip();
    let report = judged(
        &format!("<< /S /SR /MH << /C << /R 1 >> >> /R [<< /S /MR /C {clip} >>] >>"),
        Processor::default(),
    );
    assert!(report.considered.is_empty(), "{report:?}");
    assert!(!report.stopped.is_empty());
    let report = judged("<< /S /SR /R [] >>", Processor::default());
    assert!(report.chosen().is_none());
    assert!(report.considered.is_empty());
    // A selector inside a selector is searched depth first, as the clause's tree search.
    assert!(viable(&format!(
        "<< /S /SR /R [<< /S /SR /R [<< /S /MR /C {clip} >>] >>] >>"
    )));
}

/// §13.2.4.1 and Table 284's `/S`: "The media clip is non-viable if the interactive PDF processor
/// does not recognise the value of this entry."
#[test]
fn a_media_clip_of_an_unrecognised_subtype_is_not_viable() {
    assert!(!viable(&format!(
        "<< /S /MR /C << /S /MCX /CT (video/mp4) /D {EMBEDDED} >> >>"
    )));
}

/// §13.2.4.3: "If the next-level media object is non-viable, the media clip section shall be also
/// non-viable", Table 288's `/D` is required, and an offset in `MH` is one the hand-off cannot
/// honour, because a player handed the file plays all of it.
#[test]
fn a_section_is_viable_with_its_next_level_clip_and_a_must_honour_offset_is_not() {
    let clip = clip();
    let offset = "<< /S /T /T << /S /S /V 5 >> >>";
    let report = judged(
        &format!("<< /S /MR /C << /S /MCS /D {clip} /BE << /B {offset} >> >> >>"),
        Processor::default(),
    );
    let (_, chosen) = report
        .chosen()
        .expect("a best-effort offset leaves it viable");
    assert_eq!(chosen.content_type.as_deref(), Some("video/mp4"));
    assert!(!chosen.unhonoured.is_empty(), "{chosen:?}");
    assert!(!viable(&format!(
        "<< /S /MR /C << /S /MCS /D {clip} /MH << /E {offset} >> >> >>"
    )));
    assert!(!viable(
        "<< /S /MR /C << /S /MCS /D << /S /MCD /CT (video/mp4) /D (clip.mp4) >> >> >>"
    ));
    assert!(!viable("<< /S /MR /C << /S /MCS >> >>"));
    // A section of a section ends in the data, and is the data's clip.
    assert!(viable(&format!(
        "<< /S /MR /C << /S /MCS /D << /S /MCS /D {clip} >> >> >>"
    )));
}

/// §13.2.5 and Table 290: the play parameters' `MH` options are ones the hand-off cannot pass to a
/// player, its `BE` options are ignored, and of its `/PL`: "If this object is non-viable, the media
/// play parameters dictionary shall be considered non-viable."
#[test]
fn play_parameters_are_viable_on_their_best_effort_options_and_their_players() {
    let clip = clip();
    assert!(viable(&format!(
        "<< /S /MR /C {clip} /P << /BE << /D << /S /Unknown >> /RC 2 >> >> >>"
    )));
    assert!(!viable(&format!(
        "<< /S /MR /C {clip} /P << /MH << /A false >> >> >>"
    )));
    assert!(!viable(&format!(
        "<< /S /MR /C {clip} /P << /PL << /MU [<< /PID << /U (vnd.adobe.swname:ADBE_MCI) >> >>] >> >> >>"
    )));
}

/// §13.2.6.1: the screen parameters' `MH` and `BE` "shall function as discussed in 13.2.2", and
/// every Table 294 entry is a window the hand-off cannot tell a player to open.
#[test]
fn screen_parameters_must_honour_make_the_rendition_non_viable_and_best_effort_do_not() {
    let clip = clip();
    assert!(viable(&format!(
        "<< /S /MR /C {clip} /SP << /BE << /W 1 /M 2 >> >> >>"
    )));
    assert!(!viable(&format!(
        "<< /S /MR /C {clip} /SP << /MH << /W 3 >> >> >>"
    )));
}

/// §13.2.7.2.2's four conditions for the one player this program has, whichever program this
/// machine starts for the content type: (a) cannot hold because that player is chosen by the type,
/// (b) cannot because no `/NU` can name it, (c) holds wherever a non-empty `/MU` names the players
/// that must play it, and (d) wherever the content type is unknown and no `/MU` is stated. Of
/// §13.2.7.2.1's arrays: "An empty array shall be treated as if it is not present."
#[test]
fn the_media_player_algorithm_admits_the_hand_off_only_where_no_player_is_required() {
    let player = "<< /PID << /U (vnd.adobe.swname:ADBE_MCI) >> >>";
    let with_players = |players: &str| {
        viable(&format!(
            "<< /S /MR /C << /S /MCD /CT (video/mp4) /D {EMBEDDED} /PL {players} >> >>"
        ))
    };
    assert!(with_players(&format!("<< /NU [{player}] >>")));
    assert!(with_players(&format!("<< /A [{player}] >>")));
    assert!(with_players("<< /MU [] >>"));
    assert!(!with_players(&format!("<< /MU [{player}] >>")));
    // Condition (d): no content type, no /MU, and an /A this player is not found in.
    assert!(!viable(&format!(
        "<< /S /MR /C << /S /MCD /D {EMBEDDED} /PL << /A [{player}] >> >> >>"
    )));
}
