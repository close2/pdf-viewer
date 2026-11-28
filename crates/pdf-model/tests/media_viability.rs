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

use pdf_model::media::{Players, Processor, Report, Screens, judge};
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
    let document = Document::open(out.into_bytes()).expect("a valid fixture");
    assert!(!document.was_recovered(), "exact xref offsets");
    document
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
        players: Players::HandOff,
        screens: Screens::default(),
    };
    let french = Processor {
        language: Some("fr"),
        players: Players::HandOff,
        screens: Screens::default(),
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
    assert_ne!(report.stopped, [] as [String; 0]);
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

/// §13.2.6.2 and §13.2.6.3 as far as the hand-off executes them: whether a must-honour `/B` names
/// the start of its clip, read through Tables 296 to 300. Table 298: "Frame numbers begin at 0;
/// negative frame numbers are out of range". Table 300: "Non-integral values shall be allowed", so
/// half a second is read as a number, and is a seek. An offset or a timespan with no `/S`, which
/// both tables require, names no subtype and so no start.
#[test]
fn an_offset_names_its_clips_start_only_as_zero_seconds_or_frame_zero() {
    let clip = clip();
    let must_begin =
        |offset: &str| format!("<< /S /MR /C << /S /MCS /D {clip} /MH << /B {offset} >> >> >>");
    assert!(viable(&must_begin("<< /S /T /T << /S /S /V 0.0 >> >>")));
    assert!(viable(&must_begin("<< /S /F /F 0 >>")));
    for seek in [
        "<< /S /F /F -1 >>",
        "<< /S /T /T << /S /S /V 0.5 >> >>",
        "<< /T << /S /S /V 0 >> >>",
        "<< /S /T /T << /V 0 >> >>",
    ] {
        assert!(!viable(&must_begin(seek)), "{seek}");
    }
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

/// §13.2.4.3 under §13.2.6.2: a `/B` "specified by time or frames" is "relative to the start of the
/// next-level media clip", and a player handed the file starts at its beginning, so a must-honour
/// `/B` of zero seconds (Table 297 over a Table 300 simple timespan) or of frame 0 (Table 298) is
/// honoured and the section stays viable. **The controls are the offsets the hand-off cannot honour**:
/// a later start, a marker only the container locates, a subtype Table 296 or Table 300 does not
/// recognise, an `/E` whose clip end only the container states, and a start of zero relative to a
/// next-level section that itself begins later, best effort or not.
#[test]
fn a_must_honour_begin_at_the_clips_start_is_honoured_by_the_hand_off() {
    let clip = clip();
    let section = |options: &str| format!("<< /S /MR /C << /S /MCS /D {clip} {options} >> >>");
    let zero_seconds = "<< /S /T /T << /S /S /V 0 >> >>";
    for start in [
        zero_seconds,
        "<< /Type /MediaOffset /S /T /T << /Type /Timespan /S /S /V 0.0 >> >>",
        "<< /S /F /F 0 >>",
    ] {
        let report = judged(
            &section(&format!("/MH << /B {start} >>")),
            Processor::default(),
        );
        let (_, chosen) = report.chosen().expect("a begin at the clip's start");
        assert!(chosen.unhonoured.is_empty(), "{start}: {chosen:?}");
        let report = judged(
            &section(&format!("/BE << /B {start} >>")),
            Processor::default(),
        );
        let (_, chosen) = report
            .chosen()
            .expect("a best-effort begin at the clip's start");
        assert!(chosen.unhonoured.is_empty(), "{start}: {chosen:?}");
    }
    for later in [
        "<< /S /T /T << /S /S /V 5 >> >>",
        "<< /S /T /T << /S /S /V -1 >> >>",
        "<< /S /F /F 1 >>",
        "<< /S /M /M (Chapter One) >>",
        "<< /S /X /F 0 >>",
        "<< /S /T /T << /S /Q /V 0 >> >>",
    ] {
        assert!(
            !viable(&section(&format!("/MH << /B {later} >>"))),
            "{later}"
        );
    }
    assert!(
        !viable(&section(&format!("/MH << /E {zero_seconds} >>"))),
        "an /E's clip end is the container's to state"
    );
    let nested = |inner: &str| {
        format!(
            "<< /S /MR /C << /S /MCS /MH << /B {zero_seconds} >> /D << /S /MCS {inner} /D {clip} \
             >> >> >>"
        )
    };
    assert!(viable(&nested(&format!("/MH << /B {zero_seconds} >>"))));
    assert!(viable(&nested("")));
    assert!(
        !viable(&nested("/BE << /B << /S /F /F 20 >> >>")),
        "a start relative to a next-level section that begins at frame 20"
    );
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

/// Whether a clip of type `video/mp4` whose own `/PL` is `players` is viable, and what the
/// rendition's report says it set aside.
fn with_players(players: &str) -> (bool, Vec<String>) {
    let report = judged(
        &format!("<< /S /MR /C << /S /MCD /CT (video/mp4) /D {EMBEDDED} /PL {players} >> >>"),
        Processor::default(),
    );
    let set_aside = report
        .considered
        .iter()
        .flat_map(|considered| considered.set_aside.iter().cloned())
        .collect();
    (report.chosen().is_some(), set_aside)
}

/// §13.2.4.3: "For media types where an offset makes no sense (such as JPEG images), B and E shall
/// be ignored, with no effect on viability." An offset over an image keeps a must-honour section
/// viable and is not reported as played without; an entry Table 289 does not define is still a
/// reason, and a type that can animate keeps its offsets.
#[test]
fn an_offset_over_a_still_image_is_ignored_with_no_effect_on_viability() {
    let offset = "<< /S /T /T << /S /S /V 5 >> >>";
    let section = |content_type: &str, options: &str| {
        judged(
            &format!(
                "<< /S /MR /C << /S /MCS /D << /S /MCD /CT ({content_type}) /D {EMBEDDED} >> \
                 {options} >> >>"
            ),
            Processor::default(),
        )
    };
    let report = section("image/jpeg", &format!("/MH << /B {offset} /E {offset} >>"));
    assert!(report.chosen().is_some(), "{report:?}");
    let report = section("image/jpeg", &format!("/BE << /B {offset} >>"));
    let (_, chosen) = report.chosen().expect("a best-effort offset over an image");
    assert!(chosen.unhonoured.is_empty(), "{chosen:?}");
    // The type is matched case-insensitively and without its parameters, and a section of a
    // section takes the data's type.
    assert!(
        section("IMAGE/PNG; x=1", &format!("/MH << /E {offset} >>"))
            .chosen()
            .is_some()
    );
    assert!(viable(&format!(
        "<< /S /MR /C << /S /MCS /MH << /B {offset} >> /D << /S /MCS /MH << /E {offset} >> /D \
         << /S /MCD /CT (image/jpeg) /D {EMBEDDED} >> >> >> >>"
    )));
    assert!(
        section("image/jpeg", "/MH << /X 1 >>").chosen().is_none(),
        "an entry Table 289 does not define"
    );
    assert!(
        section("image/gif", &format!("/MH << /B {offset} >>"))
            .chosen()
            .is_none(),
        "a type whose format can animate keeps its offsets"
    );
}

/// §13.2.7.2.1: "If a non-viable media player info dictionary is referenced by MU , NU , or A , it
/// shall be treated as if it were not present in its original array", so an `/MU` of non-viable
/// entries is an empty one, which "shall be treated as if it is not present", and the removal is
/// said. One viable entry beside them still names the players that must play it.
#[test]
fn a_non_viable_media_player_info_dictionary_is_not_present_in_its_array() {
    let viable_player = "<< /PID << /U (vnd.adobe.swname:ADBE_MCI) >> >>";
    let non_viable = "<< /PID << /U (vnd.adobe.swname:ADBE_MCI) /L [-1] >> >>";
    let (played, set_aside) = with_players(&format!("<< /MU [{non_viable} {non_viable}] >>"));
    assert!(played, "{set_aside:?}");
    assert_eq!(set_aside.len(), 2, "{set_aside:?}");
    assert!(set_aside[0].contains("/MU entry 1"), "{set_aside:?}");
    let (played, _) = with_players(&format!("<< /MU [{non_viable} {viable_player}] >>"));
    assert!(!played);
    // The play parameters' /PL is read by the same rule.
    assert!(viable(&format!(
        "<< /S /MR /C {} /P << /PL << /MU [{non_viable}] >> >> >>",
        clip()
    )));
    // An emptied /MU leaves condition (d) to a clip with no content type.
    assert!(!viable(&format!(
        "<< /S /MR /C << /S /MCD /D {EMBEDDED} /PL << /MU [{non_viable}] >> >> >>"
    )));
    // An entry that is no media player info dictionary names no player.
    assert!(with_players("<< /MU [null 7] >>").0);
}

/// §13.2.7.3 and Table 302: `/PID` is required, and its `MH` holds entries that "shall be honoured
/// for this dictionary to be considered viable" where "Currently, there are no defined entries for
/// this dictionary", so by §13.2.2 any entry there makes the dictionary non-viable and an empty one
/// does not.
#[test]
fn a_media_player_info_dictionary_is_viable_only_with_its_pid_and_nothing_it_must_honour() {
    let identifier = "/PID << /U (vnd.adobe.swname:ADBE_MCI) >>";
    assert!(with_players("<< /MU [<< /Type /MediaPlayerInfo >>] >>").0);
    assert!(with_players(&format!("<< /MU [<< {identifier} /MH << /X 1 >> >>] >>")).0);
    assert!(!with_players(&format!("<< /MU [<< {identifier} /MH << >> >>] >>")).0);
    assert!(!with_players(&format!("<< /MU [<< {identifier} /BE << /X 1 >> >>] >>")).0);
}

/// §13.2.7.4.1: a software identifier holding a key Table 303 does not define "shall be considered
/// to be partially recognised", and §13.2.7.2.1: "The same rule shall apply to a media player info
/// dictionary that contains a partially unrecognised software identifier dictionary." Table 303's
/// `/U` is required.
#[test]
fn a_partially_recognised_software_identifier_takes_its_player_out_of_the_array() {
    let entry = |identifier: &str| format!("<< /MU [<< /PID << {identifier} >> >>] >>");
    let (played, set_aside) = with_players(&entry("/U (vnd.adobe.swname:ADBE_MCI) /Future 1"));
    assert!(played);
    assert!(set_aside[0].contains("/Future"), "{set_aside:?}");
    assert!(with_players(&entry("/L [1] /H [2]")).0, "no /U");
    assert!(
        !with_players(&entry(
            "/Type /SoftwareIdentifier /U (vnd.adobe.swname:ADBE_MCI) /L [1] /LI true /H [2] \
             /HI false /OS [(Windows)]"
        ))
        .0,
        "every Table 303 key is recognised"
    );
}

/// §13.2.7.4.4: "If a version array contains negative numbers, it is non-viable, as is the
/// enclosing software identifier", in either bound, and a media player info dictionary holding it
/// is then not present in its array (§13.2.7.2.1).
#[test]
fn a_negative_version_number_makes_the_software_identifier_non_viable() {
    let entry = |versions: &str| {
        format!("<< /MU [<< /PID << /U (vnd.adobe.swname:ADBE_MCI) {versions} >> >>] >>")
    };
    assert!(with_players(&entry("/L [1 -2]")).0);
    assert!(with_players(&entry("/H [3 0 -0.5]")).0);
    assert!(!with_players(&entry("/L [0] /H []")).0);
}

/// Judges the rendition `body` as object 2 beside an embedded file whose stream is object 4 and
/// holds `file`, written in `ASCIIHexDecode` so that the fixture stays text.
fn judged_over(body: &str, file: &[u8]) -> Report {
    let mut hex = String::new();
    for byte in file {
        let _ = write!(hex, "{byte:02X}");
    }
    hex.push('>');
    let stream = format!(
        "<< /Length {} /Type /EmbeddedFile /Filter /ASCIIHexDecode >>\nstream\n{hex}\nendstream",
        hex.len()
    );
    let doc = document(&["<< /Type /Catalog >>", body, "null", &stream]);
    let rendition = Object::Reference(ObjectId {
        number: 2,
        generation: 0,
    });
    judge(&doc, &rendition, Processor::default())
}

/// A rendition of one section whose options are `options`, over media clip data of `content_type`
/// embedded as [`EMBEDDED`].
fn section_over(content_type: &str, options: &str) -> String {
    format!(
        "<< /S /MR /C << /S /MCS /D << /S /MCD /CT ({content_type}) /D {EMBEDDED} >> {options} >> \
         >>"
    )
}

fn riff_chunk(id: [u8; 4], data: &[u8]) -> Vec<u8> {
    let mut chunk = id.to_vec();
    chunk.extend_from_slice(&u32::try_from(data.len()).expect("small").to_le_bytes());
    chunk.extend_from_slice(data);
    if data.len() % 2 == 1 {
        chunk.push(0);
    }
    chunk
}

/// A PCM WAVE file, 8 000 16-bit mono samples a second for two seconds, with a labelled cue point
/// for each of `cues` (a name and a sample position).
fn wave_file(cues: &[(&str, u32)]) -> Vec<u8> {
    let mut format = Vec::new();
    for field in [1u16, 1] {
        format.extend_from_slice(&field.to_le_bytes());
    }
    format.extend_from_slice(&8000u32.to_le_bytes());
    format.extend_from_slice(&16000u32.to_le_bytes());
    for field in [2u16, 16] {
        format.extend_from_slice(&field.to_le_bytes());
    }
    let mut body = b"WAVE".to_vec();
    body.extend(riff_chunk(*b"fmt ", &format));
    body.extend(riff_chunk(*b"data", &vec![0; 32000]));
    let mut cue = u32::try_from(cues.len())
        .expect("few")
        .to_le_bytes()
        .to_vec();
    let mut list = b"adtl".to_vec();
    for (index, (name, position)) in cues.iter().enumerate() {
        let id = u32::try_from(index.saturating_add(1)).expect("few");
        cue.extend_from_slice(&id.to_le_bytes());
        cue.extend_from_slice(&position.to_le_bytes());
        cue.extend_from_slice(b"data");
        cue.extend_from_slice(&[0; 8]);
        cue.extend_from_slice(&position.to_le_bytes());
        let mut label = id.to_le_bytes().to_vec();
        label.extend_from_slice(name.as_bytes());
        label.push(0);
        list.extend(riff_chunk(*b"labl", &label));
    }
    body.extend(riff_chunk(*b"cue ", &cue));
    body.extend(riff_chunk(*b"LIST", &list));
    let mut file = b"RIFF".to_vec();
    file.extend_from_slice(&u32::try_from(body.len()).expect("small").to_le_bytes());
    file.extend(body);
    file
}

fn mp4_box(kind: [u8; 4], payload: &[u8]) -> Vec<u8> {
    let mut item = u32::try_from(payload.len().saturating_add(8))
        .expect("small")
        .to_be_bytes()
        .to_vec();
    item.extend_from_slice(&kind);
    item.extend_from_slice(payload);
    item
}

fn full_box(kind: [u8; 4], fields: &[u8]) -> Vec<u8> {
    let mut payload = vec![0; 4];
    payload.extend_from_slice(fields);
    mp4_box(kind, &payload)
}

/// An MP4 file whose movie runs ten seconds (6 000 units at 600 a second) over one picture track of
/// 250 samples, and no chapter list.
fn mp4_file() -> Vec<u8> {
    let mut mvhd = vec![0; 8];
    mvhd.extend_from_slice(&600u32.to_be_bytes());
    mvhd.extend_from_slice(&6000u32.to_be_bytes());
    let mut tkhd = vec![0; 8];
    tkhd.extend_from_slice(&1u32.to_be_bytes());
    let mut hdlr = vec![0; 4];
    hdlr.extend_from_slice(b"vide");
    let mut stsz = 0u32.to_be_bytes().to_vec();
    stsz.extend_from_slice(&250u32.to_be_bytes());
    let stbl = mp4_box(*b"stbl", &full_box(*b"stsz", &stsz));
    let mdia = mp4_box(
        *b"mdia",
        &[
            full_box(*b"mdhd", &mvhd),
            full_box(*b"hdlr", &hdlr),
            mp4_box(*b"minf", &stbl),
        ]
        .concat(),
    );
    let trak = mp4_box(*b"trak", &[full_box(*b"tkhd", &tkhd), mdia].concat());
    let moov = mp4_box(*b"moov", &[full_box(*b"mvhd", &mvhd), trak].concat());
    [mp4_box(*b"ftyp", b"isom\0\0\0\0"), moov].concat()
}

fn seconds(value: &str) -> String {
    format!("<< /S /T /T << /S /S /V {value} >> >>")
}

/// §13.2.4.3: "However, if E specifies an offset beyond the end of the next-level media clip, the
/// end value shall be used instead, and there is no effect on viability." A player handed the file
/// stops at its end, so a must-honour `/E` at or past the clip's own length — read from its
/// container, in seconds or in frames as the offset is — is honoured, and an `/E` inside the clip
/// is a stop the player would have to be told. **The controls**: an end one unit inside the clip;
/// a frame offset over a sound, which has no frames; a next-level section that ends earlier, best
/// effort or not, under §13.2.4.3's "all B or E entries, respectively, at deeper levels (closer to
/// the media clip data), shall be evaluated as if they were in an MH dictionary"; a type whose
/// container this program does not read; and the same file named by URL, which it does not fetch.
#[test]
fn an_end_past_the_clips_own_length_is_the_end_and_has_no_effect_on_viability() {
    let mp4 = mp4_file();
    let ends = |options: &str| {
        judged_over(&section_over("video/mp4", options), &mp4)
            .chosen()
            .is_some()
    };
    for end in [seconds("10"), seconds("12.5"), "<< /S /F /F 250 >>".into()] {
        assert!(ends(&format!("/MH << /E {end} >>")), "{end}");
        let report = judged_over(
            &section_over("video/mp4", &format!("/BE << /E {end} >>")),
            &mp4,
        );
        let (_, chosen) = report
            .chosen()
            .expect("a best-effort end at the clip's end");
        assert!(chosen.unhonoured.is_empty(), "{end}: {chosen:?}");
    }
    for inside in [seconds("9.5"), "<< /S /F /F 249 >>".into()] {
        assert!(!ends(&format!("/MH << /E {inside} >>")), "{inside}");
    }
    let report = judged_over(
        &section_over("video/mp4", &format!("/MH << /E {} >>", seconds("9.5"))),
        &mp4,
    );
    let reasons = format!("{report:?}");
    assert!(reasons.contains("runs 10 s"), "{reasons}");

    let wave = wave_file(&[]);
    let sound = |options: &str| {
        judged_over(&section_over("audio/wav", options), &wave)
            .chosen()
            .is_some()
    };
    assert!(sound(&format!("/MH << /E {} >>", seconds("2"))));
    assert!(!sound(&format!("/MH << /E {} >>", seconds("1.9"))));
    assert!(
        !sound("/MH << /E << /S /F /F 100000 >> >>"),
        "a sound has no frames"
    );

    let nested = |inner: &str| {
        let body = format!(
            "<< /S /MR /C << /S /MCS /MH << /E {} >> /D << /S /MCS {inner} /D << /S /MCD /CT \
             (video/mp4) /D {EMBEDDED} >> >> >> >>",
            seconds("20")
        );
        judged_over(&body, &mp4).chosen().is_some()
    };
    assert!(nested(""));
    assert!(nested(&format!("/BE << /E {} >>", seconds("30"))));
    assert!(
        !nested(&format!("/BE << /E {} >>", seconds("5"))),
        "a next-level section that ends at 5 s"
    );

    let unread = judged_over(
        &section_over("video/x-flv", &format!("/MH << /E {} >>", seconds("20"))),
        &mp4,
    );
    assert!(unread.chosen().is_none());
    assert!(format!("{unread:?}").contains("video/x-flv"), "{unread:?}");
    let url = format!(
        "<< /S /MR /C << /S /MCS /D << /S /MCD /CT (video/mp4) /D << /Type /Filespec /FS /URL /F \
         (https://example.org/clip.mp4) >> >> /MH << /E {} >> >> >>",
        seconds("20")
    );
    let report = judged_over(&url, &mp4);
    assert!(report.chosen().is_none());
    assert!(format!("{report:?}").contains("URL"), "{report:?}");
}

/// §13.2.4.3: "When B or E are specified by markers, there shall be a corresponding absolute offset
/// into the underlying media clip data object. If this offset is not within the range defined by the
/// next-level media clip (if any), or if the marker is not present in the underlying media clip, the
/// existence of the entry shall be ignored, and there is no effect on viability." A marker the
/// container's table does not name is ignored, in `MH` as in `BE`; one it names inside the clip is a
/// place the player would have to be told, and one at the clip's start is where a player handed the
/// file begins. **The controls**: the named marker inside the clip, and a container this program
/// cannot read, where whether a marker is present is not known.
#[test]
fn a_marker_the_clip_lacks_is_ignored_and_one_it_names_inside_the_clip_is_not() {
    let wave = wave_file(&[("Start", 0), ("Chapter One", 4000)]);
    let marker = |name: &str| format!("<< /S /M /M ({name}) >>");
    let judged = |options: &str| judged_over(&section_over("audio/wav", options), &wave);
    for options in [
        format!("/MH << /B {} >>", marker("Intro")),
        format!("/MH << /E {} >>", marker("Intro")),
        format!("/BE << /B {} >>", marker("Intro")),
        format!("/MH << /B {} >>", marker("Start")),
    ] {
        let report = judged(&options);
        let (_, chosen) = report.chosen().expect("an absent marker is ignored");
        assert!(chosen.unhonoured.is_empty(), "{options}: {chosen:?}");
    }
    for options in [
        format!("/MH << /B {} >>", marker("Chapter One")),
        format!("/MH << /E {} >>", marker("Chapter One")),
        format!("/MH << /E {} >>", marker("Start")),
    ] {
        let report = judged(&options);
        assert!(report.chosen().is_none(), "{options}");
        assert!(
            format!("{report:?}").contains("places the marker"),
            "{report:?}"
        );
    }
    assert!(
        judged_over(
            &section_over("video/mp4", &format!("/MH << /B {} >>", marker("Intro"))),
            &mp4_file()
        )
        .chosen()
        .is_some(),
        "an MP4 file with no chapter list names no marker"
    );
    assert!(
        judged_over(
            &section_over("video/mp4", &format!("/MH << /B {} >>", marker("Intro"))),
            b"moov"
        )
        .chosen()
        .is_none(),
        "a container cut short cannot say whether a marker is present"
    );
}

/// Tables 296 and 300: "The rendition is non-viable if the interactive PDF processor does not
/// recognise the value of this entry". The sentence names the rendition, so a section's `/B` or `/E`
/// whose offset or timespan states an unrecognised `/S` makes it non-viable from a `BE` as from an
/// `MH` (ADR 2447). **The control** is an absent `/S`, which is no value at all: in a `BE` it stays
/// an offset not honoured, and the rendition is chosen.
#[test]
fn an_unrecognised_offset_subtype_makes_the_rendition_non_viable_from_either_dictionary() {
    let clip = clip();
    let section = |options: &str| format!("<< /S /MR /C << /S /MCS /D {clip} {options} >> >>");
    for unrecognised in [
        "<< /S /X /F 0 >>",
        "<< /S 7 /F 0 >>",
        "<< /S /T /T << /S /Q /V 0 >> >>",
    ] {
        for dictionary in ["MH", "BE"] {
            for key in ["B", "E"] {
                let report = judged(
                    &section(&format!("/{dictionary} << /{key} {unrecognised} >>")),
                    Processor::default(),
                );
                assert!(
                    report.chosen().is_none(),
                    "{dictionary} {key} {unrecognised}"
                );
                assert!(
                    format!("{report:?}").contains("does not recognise"),
                    "{report:?}"
                );
            }
        }
    }
    let report = judged(&section("/BE << /B << /F 3 >> >>"), Processor::default());
    let (_, chosen) = report
        .chosen()
        .expect("an absent /S is no unrecognised value");
    assert!(!chosen.unhonoured.is_empty(), "{chosen:?}");
}

/// Table 300: "The rendition is non-viable if the interactive PDF processor does not recognise the
/// value of this entry." A media duration's explicit `/T` (Table 292) is a Table 300 timespan, and
/// Table 291 asks the play parameters' `BE` to be honoured in a best-effort sense only, so every
/// other fault in a `BE` duration leaves the rendition chosen and this sentence, which names the
/// rendition, does not (ADR 2472). **The controls**: Table 292 reaches `/T` only "if the value of S
/// is T"; a duration whose own `/S` is not recognised is "non-viable" as a dictionary, which a `BE`
/// sets aside; a timespan with a negative or a missing `/V`, or no `/S`, is not an unrecognised
/// value; and the same timespan in an `MH` makes the rendition non-viable as every `MH` entry does.
#[test]
fn an_unrecognised_timespan_in_a_best_effort_duration_makes_the_rendition_non_viable() {
    let clip = clip();
    let parameters = |options: &str| format!("<< /S /MR /C {clip} /P << {options} >> >>");
    let duration = |entries: &str| parameters(&format!("/BE << /D << {entries} >> >>"));
    for unrecognised in [
        "/S /T /T << /S /Q /V 3 >>",
        "/S /T /T << /S 7 /V 3 >>",
        "/S /T /T << /S /Q >>",
    ] {
        let report = judged(&duration(unrecognised), Processor::default());
        assert!(report.chosen().is_none(), "{unrecognised}");
        let reasons = format!("{report:?}");
        assert!(
            reasons.contains("Table 300") && reasons.contains("does not recognise"),
            "{reasons}"
        );
    }
    for honoured in [
        "/S /T /T << /S /S /V 3 >>",
        "/S /T /T << /S /S /V 2.5 >>",
        "/S /T /T << /S /S /V -1 >>",
        "/S /T /T << /S /S >>",
        "/S /T /T << /V 3 >>",
        "/S /T",
        "/S /T /T 3",
        "/S /I /T << /S /Q /V 3 >>",
        "/S /F /T << /S /Q /V 3 >>",
        "/S /Unknown /T << /S /Q /V 3 >>",
        "/T << /S /Q /V 3 >>",
    ] {
        assert!(viable(&duration(honoured)), "{honoured}");
    }
    assert!(
        !viable(&parameters("/MH << /D << /S /T /T << /S /Q /V 3 >> >> >>")),
        "an MH duration is a must-honour entry"
    );
}

/// §13.2.6.2: "Different media types support different types of offsets." An offset a media type
/// does not carry is one the player "did not support" (§13.2.4.3 NOTE 2), so a section's must-honour
/// offset of that type cannot be honoured and the rendition is non-viable, with the reason said.
/// **The cases**: a frame offset over a sound, which has no frames; a time offset over a container
/// the reader does not read, where whether an end reaches the clip's own end is not known; the file
/// named by URL is `an_end_past_the_clips_own_length_is_the_end_and_has_no_effect_on_viability`'s.
/// **The controls**: the same offsets in a `BE`, which a player may
/// ignore, leave the rendition chosen; and a still image's offsets make no sense and are ignored
/// (§13.2.4.3).
#[test]
fn an_offset_a_media_type_does_not_carry_is_a_value_the_player_cannot_honour() {
    let wave = wave_file(&[]);
    let frames = "<< /S /F /F 100000 >>";
    let sound = judged_over(
        &section_over("audio/wav", &format!("/MH << /E {frames} >>")),
        &wave,
    );
    assert!(sound.chosen().is_none());
    assert!(
        format!("{sound:?}").contains("states no frames"),
        "{sound:?}"
    );

    let unread = judged_over(
        &section_over("audio/mpeg", &format!("/MH << /E {} >>", seconds("2"))),
        &wave,
    );
    assert!(unread.chosen().is_none());
    assert!(format!("{unread:?}").contains("audio/mpeg"), "{unread:?}");

    for (content_type, entry) in [
        ("audio/wav", frames.to_owned()),
        ("audio/mpeg", seconds("2")),
    ] {
        let report = judged_over(
            &section_over(content_type, &format!("/BE << /E {entry} >>")),
            &wave,
        );
        let (_, chosen) = report
            .chosen()
            .expect("a best-effort offset may be ignored");
        assert!(!chosen.unhonoured.is_empty(), "{content_type}: {chosen:?}");
    }
    assert!(
        judged_over(
            &section_over("image/jpeg", &format!("/MH << /E {frames} >>")),
            &wave
        )
        .chosen()
        .is_some(),
        "a still image has no offsets to carry"
    );
}

/// A SWF file of `version` running `frames` frames at 10 frames a second with one frame label,
/// `label` on the second frame: the header's rate and count are the clip's length, and the label is
/// its one named offset (ADR 2488).
fn swf_file(frames: u16, label: &str) -> Vec<u8> {
    let tag = |kind: u16, body: &[u8]| {
        let mut out = ((kind << 6) | u16::try_from(body.len()).expect("short"))
            .to_le_bytes()
            .to_vec();
        out.extend_from_slice(body);
        out
    };
    // A RECT of Nbits 1 holding four zero fields: 5 + 4 bits, padded to two bytes.
    let mut file = b"FWS".to_vec();
    file.push(9);
    let mut body = vec![0b0000_1000, 0b0000_0000];
    body.extend_from_slice(&0x0A00u16.to_le_bytes());
    body.extend_from_slice(&frames.to_le_bytes());
    body.extend(tag(1, &[]));
    let mut name = label.as_bytes().to_vec();
    name.push(0);
    body.extend(tag(43, &name));
    body.extend(tag(1, &[]));
    body.extend(tag(0, &[]));
    file.extend_from_slice(
        &u32::try_from(body.len().saturating_add(8))
            .expect("small")
            .to_le_bytes(),
    );
    file.extend(body);
    file
}

/// An ASF file whose File Properties Object states `play` in 100 ns units and no preroll.
fn asf_file(play: u64) -> Vec<u8> {
    // GUIDs as the Windows fields their text spells: ASF_Header_Object and
    // ASF_File_Properties_Object.
    let header_guid = [
        0x30, 0x26, 0xB2, 0x75, 0x8E, 0x66, 0xCF, 0x11, 0xA6, 0xD9, 0x00, 0xAA, 0x00, 0x62, 0xCE,
        0x6C,
    ];
    let properties_guid = [
        0xA1, 0xDC, 0xAB, 0x8C, 0x47, 0xA9, 0xCF, 0x11, 0x8E, 0xE4, 0x00, 0xC0, 0x0C, 0x20, 0x53,
        0x65,
    ];
    let mut data = vec![0; 40];
    data.extend_from_slice(&play.to_le_bytes());
    data.extend_from_slice(&play.to_le_bytes());
    data.extend_from_slice(&[0; 8]);
    data.extend_from_slice(&2u32.to_le_bytes());
    data.extend_from_slice(&[0; 12]);
    let mut properties = properties_guid.to_vec();
    properties.extend_from_slice(
        &u64::try_from(data.len().saturating_add(24))
            .expect("small")
            .to_le_bytes(),
    );
    properties.extend(data);
    let mut file = header_guid.to_vec();
    file.extend_from_slice(
        &u64::try_from(properties.len().saturating_add(30))
            .expect("small")
            .to_le_bytes(),
    );
    file.extend_from_slice(&1u32.to_le_bytes());
    file.extend_from_slice(&[1, 2]);
    file.extend(properties);
    file
}

/// §13.2.4.3's end and marker rules over the three containers ADR 2488 and ADR 2489 add — a SWF
/// file, an ASF file and an MPEG-2 program stream — each judged by its own header.
///
/// > However, if E specifies an offset beyond the end of the next-level media clip, the end value
/// > shall be used instead, and there is no effect on viability.
///
/// **The controls**: an end one unit inside the clip; a marker the file names inside the clip; a
/// time over a program stream, whose clause states no length; and a type whose bytes are another
/// container.
#[test]
fn a_flash_a_windows_media_and_a_program_stream_clip_are_judged_by_their_own_headers() {
    let marker = |name: &str| format!("<< /S /M /M ({name}) >>");
    let end = |value: &str| format!("/MH << /E {} >>", seconds(value));
    let swf = swf_file(30, "intro");
    let flash = |options: &str| {
        judged_over(
            &section_over("application/x-shockwave-flash", options),
            &swf,
        )
    };
    // 30 frames at 10 a second: three seconds.
    assert!(flash(&end("3")).chosen().is_some());
    assert!(flash(&end("4")).chosen().is_some());
    assert!(flash(&end("2")).chosen().is_none());
    assert!(
        flash(&format!("/MH << /B {} >>", marker("outro")))
            .chosen()
            .is_some()
    );
    let named = flash(&format!("/MH << /E {} >>", marker("intro")));
    assert!(named.chosen().is_none());
    assert!(
        format!("{named:?}").contains("places the marker"),
        "{named:?}"
    );
    let frames = flash("/MH << /E << /S /F /F 30 >> >>");
    assert!(frames.chosen().is_some(), "{frames:?}");
    assert!(flash("/MH << /E << /S /F /F 29 >> >>").chosen().is_none());

    let asf = asf_file(300_000_000);
    let media = |options: &str| judged_over(&section_over("video/x-ms-wmv", options), &asf);
    assert!(media(&end("30")).chosen().is_some());
    assert!(media(&end("29")).chosen().is_none());
    assert!(
        media(&format!("/MH << /B {} >>", marker("Start")))
            .chosen()
            .is_some()
    );
    assert!(
        media("/MH << /E << /S /F /F 100 >> >>").chosen().is_none(),
        "an ASF file states no frames"
    );

    // An MPEG-2 pack header: 0x000001BA, '01' and the marker bits, a mux rate of 1.
    let program = [
        0x00, 0x00, 0x01, 0xBA, 0x44, 0x00, 0x04, 0x00, 0x04, 0x01, 0x00, 0x00, 0x07, 0xF8,
    ];
    let mpeg = |options: &str| judged_over(&section_over("video/mpeg", options), &program);
    assert!(
        mpeg(&format!("/MH << /B {} >>", marker("Intro")))
            .chosen()
            .is_some()
    );
    let timed = mpeg(&end("20"));
    assert!(timed.chosen().is_none());
    assert!(
        format!("{timed:?}").contains("does not state how long"),
        "{timed:?}"
    );

    // Another container's bytes under the type are not the container the type names.
    let wrong = judged_over(&section_over("video/x-ms-wmv", &end("30")), &swf);
    assert!(wrong.chosen().is_none());
    assert!(
        format!("{wrong:?}").contains("not the ASF file"),
        "{wrong:?}"
    );
}

// §13.2.4.2 and Table 287: the handoff cannot carry a base URL. This tests
// the public viability judgement, not actual playback of the fixture bytes.
#[test]
fn a_data_base_url_is_required_for_must_honour_and_reported_for_best_effort() {
    let rendition = |options: &str| {
        format!("<< /S /MR /C << /S /MCD /CT (video/mp4) /D {EMBEDDED} {options} >> >>")
    };
    let base = "(https://media.example.test/catalog/)";

    let mandatory = judged(
        &rendition(&format!("/MH << /BU {base} >>")),
        Processor::default(),
    );
    assert!(mandatory.chosen().is_none(), "{mandatory:?}");
    assert!(
        mandatory.notes().iter().any(|line| {
            line.contains("Table 287's base URL /BU") && line.contains("not viable")
        }),
        "{mandatory:?}"
    );

    let best_effort = judged(
        &rendition(&format!("/BE << /BU {base} >>")),
        Processor::default(),
    );
    let (_, clip) = best_effort.chosen().expect("BE does not veto handoff");
    assert!(
        clip.unhonoured
            .iter()
            .any(|entry| entry.contains("base URL /BU")),
        "{clip:?}"
    );

    let ordinary = judged(&rendition(""), Processor::default());
    let (_, clip) = ordinary.chosen().expect("no URL preference is viable");
    assert!(clip.unhonoured.is_empty(), "{clip:?}");
}

/// §13.2.2's viability is relative to the player, and Table 291's `MH` is judged against the player
/// a clip reaches (ADR 3350).
///
/// > MH ("must honour"): The options specified by this entry shall be honoured; otherwise, the
/// > containing object shall be considered non-viable.
///
/// An embedded clip whose `/CT` names a RIFF WAVE file, in a host that plays it itself, is judged
/// for that player: `/V`, `/D`, `/RC`, `/A` and an aural clip's `/F` are honoured, so an `MH`
/// stating them leaves the rendition viable, carries its parameters and is marked played here; the
/// same rendition is non-viable for the hand-off, which is told nothing. `/C` true — "a
/// player-specific controller user interface" — and an entry Table 291 does not define stay
/// non-viable for both, and a `BE` instance of an entry `MH` states is ignored.
#[test]
fn table_291_must_honour_is_judged_for_the_player_the_clip_reaches() {
    let wave = format!("<< /S /MCD /CT (audio/wav) /D {EMBEDDED} >>");
    let here = Processor {
        language: None,
        players: Players::AlsoHere,
        screens: Screens::default(),
    };
    let parameters = "/P << /MH << /V 50 /RC 2 /A false /F 3 /D << /S /T /T << /S /S /V 1.5 >> >> >> \
                      /BE << /V 80 /C true >> >>";
    let body = format!("<< /S /MR {parameters} /C {wave} >>");
    assert!(!viable(&body), "the hand-off is told none of Table 291");
    let report = judged(&body, here);
    let (_, chosen) = report.chosen().expect("viable for this program's player");
    assert!(chosen.played_here);
    assert!(
        chosen.temporary_file.is_none(),
        "this player writes no file"
    );
    assert!(
        (chosen.playing.level - 0.5).abs() < f64::EPSILON,
        "MH's /V over BE's"
    );
    assert!((chosen.playing.repeat - 2.0).abs() < f64::EPSILON);
    assert!(!chosen.playing.auto_play);
    assert_eq!(
        chosen.playing.duration,
        pdf_model::media::SimpleDuration::Seconds(1.5)
    );
    assert!(
        chosen.unhonoured.iter().any(|entry| entry.contains("/C")),
        "BE's /C true is set aside and said: {:?}",
        chosen.unhonoured
    );
    // A controller in MH, and an entry Table 291 does not define, are honoured by neither player.
    for must in ["/C true", "/Unknown 1", "/V -5"] {
        let body = format!("<< /S /MR /P << /MH << {must} >> >> /C {wave} >>");
        assert!(judged(&body, here).chosen().is_none(), "{must}");
    }
    // A clip of another type is the hand-off's, whatever the host plays.
    let body = format!("<< /S /MR /P << /MH << /V 50 >> >> /C {} >>", clip());
    assert!(judged(&body, here).chosen().is_none());
    let report = judged(&format!("<< /S /MR /C {} >>", clip()), here);
    assert!(!report.chosen().expect("viable").1.played_here);
}

/// A clip whose `/CT` is MP4's `video/` type is this program's player's only where the host shows
/// video too (`Players::AlsoVideoHere`, ADR 3538): a host that plays sound alone judges it for the
/// hand-off, as before, and an `audio/` MP4 clip is played here by both.
#[test]
fn an_mp4_video_clip_is_played_here_only_by_a_host_that_shows_video() {
    let video = format!("<< /S /MCD /CT (video/mp4) /D {EMBEDDED} >>");
    let body = format!("<< /S /MR /P << /MH << /V 50 >> >> /C {video} >>");
    let sound_only = Processor {
        language: None,
        players: Players::AlsoHere,
        screens: Screens::default(),
    };
    let shows_video = Processor {
        language: None,
        players: Players::AlsoVideoHere,
        screens: Screens::default(),
    };
    assert!(
        judged(&body, sound_only).chosen().is_none(),
        "Table 291 is the hand-off's to ignore, so MH makes it non-viable there"
    );
    let report = judged(&body, shows_video);
    let (_, chosen) = report.chosen().expect("viable for this program's player");
    assert!(chosen.played_here);
    assert!((chosen.playing.level - 0.5).abs() < f64::EPSILON);
    let sound = format!("<< /S /MR /C << /S /MCD /CT (audio/mp4) /D {EMBEDDED} >> >>");
    for players in [sound_only, shows_video] {
        assert!(
            judged(&sound, players)
                .chosen()
                .expect("viable")
                .1
                .played_here
        );
    }
}

/// Table 291's `/C`, "A flag specifying whether to display a player-specific controller user
/// interface when playing", is honoured over a clip with pictures by the controller bar the window
/// draws (ADR 3610) — in `MH` and `BE` alike — and a sound, which has no pictures to draw it over,
/// is judged as before: `MH` non-viable, `BE` set aside and said.
#[test]
fn table_291_controls_are_the_bar_over_a_clip_with_pictures() {
    let video = format!("<< /S /MCD /CT (video/mp4) /D {EMBEDDED} >>");
    let shows_video = Processor {
        language: None,
        players: Players::AlsoVideoHere,
        screens: Screens::default(),
    };
    for options in ["/MH << /C true >>", "/BE << /C true >>"] {
        let body = format!("<< /S /MR /P << {options} >> /C {video} >>");
        let report = judged(&body, shows_video);
        let (_, chosen) = report.chosen().expect("viable with its bar");
        assert!(chosen.playing.controls, "{options}");
        assert!(
            !chosen.unhonoured.iter().any(|entry| entry.contains("/C")),
            "{options}: nothing set aside: {:?}",
            chosen.unhonoured
        );
    }
    let body = format!("<< /S /MR /P << /BE << /C false >> >> /C {video} >>");
    assert!(
        !judged(&body, shows_video)
            .chosen()
            .expect("viable")
            .1
            .playing
            .controls
    );
    let sound = format!("<< /S /MCD /CT (audio/mp4) /D {EMBEDDED} >>");
    let body = format!("<< /S /MR /P << /BE << /C true >> >> /C {sound} >>");
    let report = judged(&body, shows_video);
    let (_, chosen) = report
        .chosen()
        .expect("a best-effort controller is set aside");
    assert!(!chosen.playing.controls);
    assert!(
        chosen
            .unhonoured
            .iter()
            .any(|entry| entry.contains("best-effort /C"))
    );
    let body = format!("<< /S /MR /P << /MH << /C true >> >> /C {sound} >>");
    assert!(
        judged(&body, shows_video).chosen().is_none(),
        "a must-honour one is non-viable"
    );
}

/// §13.2.3.3's search falls to the next rendition when the first is non-viable for the player
/// that would play it: a selector of a WAVE clip whose `MH` asks for a controller, then one whose
/// `MH` asks only for a volume, chooses the second here and neither for the hand-off.
#[test]
fn a_rendition_non_viable_for_this_player_falls_to_the_next() {
    let wave = format!("<< /S /MCD /CT (audio/vnd.wave) /D {EMBEDDED} >>");
    let first = format!("<< /S /MR /N (controlled) /P << /MH << /C true >> >> /C {wave} >>");
    let second = format!("<< /S /MR /N (quiet) /P << /MH << /V 25 >> >> /C {wave} >>");
    let body = format!("<< /S /SR /R [{first} {second}] >>");
    let here = Processor {
        language: None,
        players: Players::AlsoHere,
        screens: Screens::default(),
    };
    let report = judged(&body, here);
    let (considered, chosen) = report.chosen().expect("the second is viable here");
    assert_eq!(considered.name.as_deref(), Some("quiet"));
    assert!((chosen.playing.level - 0.25).abs() < f64::EPSILON);
    assert!(judged(&body, Processor::default()).chosen().is_none());
}

/// §13.2.4.3's media clip sections over a clip this program plays itself (ADR 3398): time offsets
/// are placed on the next-level clip's own section, an `/E` past the next level is held to it, a
/// beginning past the end is "an empty range" and valid, a frame offset — which a sound's
/// container does not count — makes an `MH` section non-viable and a `BE` one set aside and said,
/// and an `MH` `/B` above makes a deeper `BE` `/B` must-honour.
#[test]
fn a_media_clip_section_is_played_here_as_its_offsets_place_it() {
    use pdf_model::media::Section;
    let here = Processor {
        language: None,
        players: Players::AlsoHere,
        screens: Screens::default(),
    };
    let wave = format!("<< /S /MCD /CT (audio/wav) /D {EMBEDDED} >>");
    let time = |seconds: f64| format!("<< /S /T /T << /S /S /V {seconds} >> >>");
    let section = |options: &str, next: &str| format!("<< /S /MCS {options} /D {next} >>");
    let chosen = |clip: &str| {
        judged(&format!("<< /S /MR /C {clip} >>"), here)
            .chosen()
            .map(|(_, clip)| clip.clone())
    };
    let inner = section(
        &format!("/MH << /B {} /E {} >>", time(1.0), time(5.0)),
        &wave,
    );
    let outer = section(
        &format!("/BE << /B {} /E {} >>", time(0.5), time(9.0)),
        &inner,
    );
    let clip = chosen(&outer).expect("a section of a WAVE clip is played here");
    assert!(clip.played_here);
    assert_eq!(
        clip.playing.section,
        Section {
            begin: 1.5,
            end: Some(5.0)
        },
        "B relative to the next level's start; E past its end held to it"
    );
    let empty = section(
        &format!("/MH << /B {} /E {} >>", time(4.0), time(2.0)),
        &wave,
    );
    let clip = chosen(&empty).expect("an empty range is valid");
    assert_eq!(
        clip.playing.section,
        Section {
            begin: 4.0,
            end: Some(2.0)
        }
    );
    let frame = "<< /S /F /F 10 >>";
    assert!(chosen(&section(&format!("/MH << /B {frame} >>"), &wave)).is_none());
    let clip = chosen(&section(&format!("/BE << /B {frame} >>"), &wave)).expect("BE");
    assert_eq!(clip.playing.section, Section::WHOLE);
    assert!(
        clip.unhonoured
            .iter()
            .any(|entry| entry.contains("/B (Table 289)")),
        "{:?}",
        clip.unhonoured
    );
    let deeper = section(&format!("/BE << /B {frame} >>"), &wave);
    let forced = section(&format!("/MH << /B {} >>", time(1.0)), &deeper);
    assert!(
        chosen(&forced).is_none(),
        "an MH /B above makes the deeper /B must-honour"
    );
    // The hand-off still plays a file from its start, so the same section is not viable there.
    assert!(!viable(&format!("<< /S /MR /C {inner} >>")));
}

/// Table 287's `/BU` and Table 294's screen parameters for a clip this program plays itself
/// (ADRs 3398, 3399): the player resolves no URL, so an `MH` base URL is honoured; it shows a
/// sound in no window of its own, so an `MH` `/W 2` — Table 294's hidden window — is honoured with
/// the entries the table ignores for hidden windows and paints nothing, and `/W 3` — the screen
/// annotation's rectangle — is honoured by painting `/B` there (ADR 3407), while a floating or
/// full-screen window, an unrecognised type, or a `/B` or `/O` outside the table's range in `MH` is
/// non-viable and in `BE` is set aside and said.
#[test]
fn a_base_url_and_a_hidden_window_are_honoured_by_this_player() {
    let here = Processor {
        language: None,
        players: Players::AlsoHere,
        screens: Screens::default(),
    };
    let wave = |options: &str| format!("<< /S /MCD /CT (audio/wav) {options} /D {EMBEDDED} >>");
    let chosen = |screen: &str, clip: &str| {
        judged(&format!("<< /S /MR {screen} /C {clip} >>"), here)
            .chosen()
            .map(|(_, clip)| clip.clone())
    };
    let clip = chosen("", &wave("/MH << /BU (http://example.org/) >>")).expect("BU honoured");
    assert!(clip.played_here && clip.unhonoured.is_empty(), "{clip:?}");
    let plain = wave("");
    let hidden = "/SP << /MH << /W 2 /B [1 0 0] /O 0.5 /M 1 >> >>";
    let clip = chosen(hidden, &plain).expect("a hidden window is this player's");
    assert!(clip.unhonoured.is_empty(), "{:?}", clip.unhonoured);
    assert_eq!(clip.background, None, "a hidden window paints nothing");
    for must in [
        "/W 0",
        "/W 7",
        "/B [1 0]",
        "/B [1 0 2]",
        "/O 1.5",
        "/W 3 /O (half)",
    ] {
        let screen = format!("/SP << /MH << {must} >> >>");
        assert!(chosen(&screen, &plain).is_none(), "{must}");
    }
    let clip = chosen("/SP << /BE << /W 1 >> >>", &plain).expect("BE");
    assert!(
        clip.unhonoured
            .iter()
            .any(|entry| entry.contains("/W (Table 294)")),
        "{:?}",
        clip.unhonoured
    );
    let clip = chosen("/SP << /BE << /W 2 /B [0 0 1] >> >>", &plain).expect("BE hidden");
    assert!(clip.unhonoured.is_empty(), "{:?}", clip.unhonoured);
}

/// ISO 32000-2 Table 294's `/W` 3 for a sound this program plays (ADR 3407): "\[t]he rectangle
/// occupied by the screen annotation", whose `/B` "shall be used if the media object does not
/// entirely cover the rectangle" — all of it, for a sound — at `/O`'s opacity. `MH`'s entries come
/// before `BE`'s; with neither, and with no screen parameters at all (the table's default `/W` is
/// 3), the background is this program's stated default, opaque black; `/M` and `/F` are ignored
/// for the type, as the table says; and a `BE` `/B` that is not a colour is set aside, said, and
/// the default painted. **The control** is the hand-off, which is told no window and paints none.
#[test]
fn a_sound_played_in_its_screen_annotations_rectangle_paints_the_background() {
    use pdf_model::media::Background;
    let here = Processor {
        language: None,
        players: Players::AlsoHere,
        screens: Screens::default(),
    };
    let plain = format!("<< /S /MCD /CT (audio/wav) /D {EMBEDDED} >>");
    let chosen = |screen: &str| {
        judged(&format!("<< /S /MR {screen} /C {plain} >>"), here)
            .chosen()
            .map(|(_, clip)| clip.clone())
    };
    let background = |screen: &str| chosen(screen).and_then(|clip| clip.background);
    assert_eq!(background(""), Some(Background::default()));
    assert_eq!(Background::default().colour, [0.0, 0.0, 0.0]);
    assert_eq!(
        background("/SP << /MH << /W 3 /B [1 0.5 0] /O 0.25 /M 2 /F << /D [1 1] >> >> >>"),
        Some(Background {
            colour: [1.0, 0.5, 0.0],
            opacity: 0.25,
        })
    );
    assert_eq!(
        background("/SP << /MH << /B [0 0 1] >> /BE << /B [1 0 0] /O 0.5 >> >>"),
        Some(Background {
            colour: [0.0, 0.0, 1.0],
            opacity: 0.5,
        }),
        "MH's /B before BE's, BE's /O where MH states none"
    );
    let clip = chosen("/SP << /BE << /B (red) >> >>").expect("a BE /B is best effort");
    assert_eq!(clip.background, Some(Background::default()));
    assert!(
        clip.unhonoured
            .iter()
            .any(|entry| entry.contains("/B (Table 294)")),
        "{:?}",
        clip.unhonoured
    );
    let handed = judged(&format!("<< /S /MR /C {plain} >>"), Processor::default())
        .chosen()
        .map(|(_, clip)| clip.background);
    assert_eq!(handed, Some(None), "the hand-off paints nothing");
}

/// Table 291's `/F` for a clip with pictures, read for a host that shows video (ADR 3553): each
/// of the six values is the clip's fit, in `MH` and in `BE` alike — 3's "scrolling user interface"
/// is provided by every window that shows pictures wherever the picture overflows (ADR 3901), so
/// it neither makes an `MH` rendition non-viable nor is set aside from `BE`; and an
/// unrecognised value "shall be treated as the default value if the entry is in a BE dictionary.
/// If the entry is in an MH dictionary and it has an unrecognised value, the object shall be
/// considered non-viable." A sound is fitted to nothing, so `MH /F 3` leaves an `audio/` clip
/// viable.
#[test]
fn a_clips_fit_is_read_from_its_play_parameters_and_scrolling_is_provided() {
    use pdf_model::media::Fit;
    let shows_video = Processor {
        language: None,
        players: Players::AlsoVideoHere,
        screens: Screens::default(),
    };
    let video = format!("<< /S /MCD /CT (video/mp4) /D {EMBEDDED} >>");
    let with = |options: &str, clip: &str| {
        judged(
            &format!("<< /S /MR /P << {options} >> /C {clip} >>"),
            shows_video,
        )
    };
    for (value, fit) in [
        (0, Fit::Meet),
        (1, Fit::Slice),
        (2, Fit::Fill),
        (3, Fit::Scroll),
        (4, Fit::Hidden),
        (5, Fit::PlayersDefault),
    ] {
        for dictionary in ["MH", "BE"] {
            let report = with(&format!("/{dictionary} << /F {value} >>"), &video);
            let (_, chosen) = report.chosen().expect("viable");
            assert_eq!(chosen.playing.fit, fit, "/{dictionary} /F {value}");
            assert_eq!(chosen.unhonoured, Vec::<String>::new());
        }
    }
    assert_eq!(
        with("", &video).chosen().expect("viable").1.playing.fit,
        Fit::PlayersDefault,
        "Default value: 5"
    );
    assert!(with("/MH << /F 9 >>", &video).chosen().is_none());
    let report = with("/BE << /F 9 >>", &video);
    assert_eq!(
        report.chosen().expect("viable").1.playing.fit,
        Fit::PlayersDefault
    );
    let sound = format!("<< /S /MCD /CT (audio/mp4) /D {EMBEDDED} >>");
    let report = with("/MH << /F 3 >>", &sound);
    assert!(
        report
            .chosen()
            .expect("a sound honours every fit")
            .1
            .played_here
    );
}

/// A host whose windows open, place and find every monitor (ADR 3585).
fn every_window() -> Processor<'static> {
    Processor {
        language: Some("en"),
        players: Players::AlsoVideoHere,
        screens: Screens {
            windows: true,
            placed: true,
            monitors: [true; 7],
        },
    }
}

/// The rendition of an embedded `video/mp4` clip with screen parameters `screen`.
fn video_in(screen: &str) -> String {
    format!("<< /S /MR /SP << {screen} >> /C << /S /MCD /CT (video/mp4) /D {EMBEDDED} >> >>")
}

/// ISO 32000-2 Table 294's `/W` 0 and 1 for a clip with pictures and a host whose windows open
/// (ADR 3585): the floating window's Table 295 entries are read, `MH`'s `/F` before `BE`'s, each
/// with its default where absent; the full-screen window ignores `/F`; and a host that opens no
/// window still finds `/W` 0 in `MH` non-viable.
#[test]
fn a_floating_window_is_read_from_table_295_and_a_full_screen_window_ignores_it() {
    use pdf_model::media::{
        Floating, MonitorSpecifier, Offscreen, Position, Relative, Resize, WindowKind,
    };
    let report = judged(
        &video_in(
            "/MH << /W 0 /F << /D [320 240] /P 2 >> >> \
             /BE << /F << /D [10 10] /RT 2 /O 0 /R 2 /T false /TT [(en) (A clip) () (Ein Clip)] >> \
             /B [0 0 1] >>",
        ),
        every_window(),
    );
    let (_, clip) = report.chosen().expect("viable");
    let window = clip.window.as_ref().expect("a floating window");
    assert_eq!(
        window.kind,
        WindowKind::Floating(Floating {
            size: (320, 240),
            relative: Relative::Desktop,
            position: Position::of_value(2).expect("2"),
            offscreen: Offscreen::Ignore,
            title_bar: false,
            user_close: true,
            resize: Resize::Free,
            title: Some("A clip".to_owned()),
            held_within: None,
        }),
        "MH's /D wins over BE's; the rest come from BE or their defaults"
    );
    assert_eq!(window.monitor, MonitorSpecifier::Largest, "/M's default, 0");
    assert_eq!(window.colour, [0.0, 0.0, 1.0]);
    assert_eq!(clip.background, None, "no rectangle on the page is painted");

    let defaults = judged(
        &video_in("/BE << /W 0 /F << /D [8 6] >> >>"),
        every_window(),
    );
    let (_, clip) = defaults.chosen().expect("viable");
    let Some(WindowKind::Floating(floating)) = clip.window.as_ref().map(|window| &window.kind)
    else {
        panic!("a floating window");
    };
    assert_eq!(floating.relative, Relative::Document, "/RT's default, 0");
    assert_eq!(floating.position, Position::CENTRE, "/P's default, 4");
    assert_eq!(floating.offscreen, Offscreen::OnScreen, "/O's default, 1");
    assert_eq!(
        floating.resize,
        Resize::Fixed,
        "/R 0, ISO 32000-1's default"
    );
    assert!(
        floating.title_bar && floating.user_close,
        "/T and /UC: true"
    );
    assert_eq!(floating.title, None);

    let full = judged(
        &video_in("/MH << /W 1 /M 5 /F << /D [-1 0] >> >>"),
        every_window(),
    );
    let (_, clip) = full
        .chosen()
        .expect("viable: /F is ignored for a full-screen window");
    let window = clip.window.as_ref().expect("a full-screen window");
    assert_eq!(window.kind, WindowKind::FullScreen);
    assert_eq!(window.monitor, MonitorSpecifier::Height);

    let no_windows = Processor {
        language: None,
        players: Players::AlsoVideoHere,
        screens: Screens::default(),
    };
    assert!(
        judged(&video_in("/MH << /W 0 /F << /D [8 6] >> >>"), no_windows)
            .chosen()
            .is_none(),
        "a host that opens no window cannot honour /W 0 in MH"
    );
}

/// Table 295's rule for its entries — "if they are nested within an MH dictionary, the floating
/// window parameters object (and hence the media screen parameters object) shall be considered
/// non-viable; if they are nested within a BE dictionary, they shall be considered to have their
/// default values" — and `/D`, "(Required)": with no usable size the window `MH` asks for is
/// non-viable and the one `BE` asks for leaves the clip in its annotation's rectangle.
#[test]
fn an_unrecognised_floating_window_entry_is_non_viable_in_mh_and_default_in_be() {
    use pdf_model::media::{Relative, WindowKind};
    for (entry, value) in [("RT", "4"), ("P", "9"), ("O", "3"), ("R", "3"), ("T", "1")] {
        let must = video_in(&format!(
            "/MH << /W 0 /F << /D [8 6] /{entry} {value} >> >>"
        ));
        assert!(
            judged(&must, every_window()).chosen().is_none(),
            "MH /{entry} {value}"
        );
        let best = video_in(&format!(
            "/MH << /W 0 /F << /D [8 6] >> >> /BE << /F << /{entry} {value} >> >>"
        ));
        assert!(
            judged(&best, every_window()).chosen().is_some(),
            "BE /{entry} {value}"
        );
    }
    let report = judged(
        &video_in("/MH << /W 0 /F << /D [8 6] >> >> /BE << /F << /RT 7 >> >>"),
        every_window(),
    );
    let Some(WindowKind::Floating(floating)) = report
        .chosen()
        .and_then(|(_, clip)| clip.window.clone())
        .map(|window| window.kind)
    else {
        panic!("a floating window");
    };
    assert_eq!(floating.relative, Relative::Document);
    for screen in ["/MH << /W 0 >>", "/MH << /W 0 /F << /D [0 6] >> >>"] {
        assert!(
            judged(&video_in(screen), every_window()).chosen().is_none(),
            "{screen}"
        );
    }
    let report = judged(&video_in("/BE << /W 0 >>"), every_window());
    let (_, clip) = report.chosen().expect("viable in the rectangle");
    assert_eq!(clip.window, None);
    assert!(clip.background.is_some(), "it plays in the rectangle");
    assert!(
        clip.unhonoured.iter().any(|note| note.contains("/D")),
        "{:?}",
        clip.unhonoured
    );
}

/// ISO 32000-2 §13.2.7.5's monitor specifier, read from Table 294's `/M` for a window (ADR 3586):
/// each of Table 304's seven values is read; an unrecognised one — "Unrecognised value in MH :
/// object is non-viable; in BE : treat as default value"; a value the host's toolkit cannot find
/// (a colour depth GDK does not report) and a floating window's monitor for a host that places no
/// window are non-viable in `MH` and set aside, with the default 0, in `BE`.
#[test]
fn a_monitor_specifier_is_read_for_a_window_and_judged_against_what_the_host_finds() {
    use pdf_model::media::MonitorSpecifier;
    for value in 0..=6 {
        let report = judged(
            &video_in(&format!("/MH << /W 1 /M {value} >>")),
            every_window(),
        );
        let (_, clip) = report.chosen().expect("viable");
        let monitor = clip.window.as_ref().expect("a window").monitor;
        assert_eq!(i64::from(monitor.value()), value);
        assert_eq!(MonitorSpecifier::of_value(value), Some(monitor));
    }
    assert!(
        judged(&video_in("/MH << /W 1 /M 7 >>"), every_window())
            .chosen()
            .is_none()
    );
    let report = judged(&video_in("/MH << /W 1 >> /BE << /M 7 >>"), every_window());
    assert_eq!(
        report
            .chosen()
            .expect("viable")
            .1
            .window
            .as_ref()
            .map(|window| window.monitor),
        Some(MonitorSpecifier::Largest)
    );
    let no_depth = Processor {
        screens: Screens {
            windows: true,
            placed: false,
            monitors: [true, false, true, false, true, true, true],
        },
        ..every_window()
    };
    assert!(
        judged(&video_in("/MH << /W 1 /M 3 >>"), no_depth)
            .chosen()
            .is_none(),
        "a colour depth nobody reports"
    );
    let report = judged(&video_in("/MH << /W 1 >> /BE << /M 3 >>"), no_depth);
    let (_, clip) = report.chosen().expect("viable");
    assert_eq!(
        clip.window.as_ref().map(|window| window.monitor),
        Some(MonitorSpecifier::Largest)
    );
    assert!(clip.unhonoured.iter().any(|note| note.contains("/M 3")));
    assert!(
        judged(&video_in("/MH << /W 0 /M 4 /F << /D [8 6] >> >>"), no_depth)
            .chosen()
            .is_none(),
        "a floating window's monitor, for a host that places none"
    );
    assert!(
        judged(&video_in("/MH << /W 0 /F << /D [8 6] /P 0 >> >>"), no_depth)
            .chosen()
            .is_none(),
        "a floating window's position, for a host that places none"
    );
    assert!(
        judged(
            &video_in("/MH << /W 0 /F << /D [8 6] >> /O 0.5 >>"),
            every_window()
        )
        .chosen()
        .is_none(),
        "a transparent floating window"
    );
}
