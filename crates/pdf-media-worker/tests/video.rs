//! H.264 in the media worker: a stream written here by hand, whose every picture is known before it
//! is decoded, through the session, the MP4 reader and the confined worker (ADR 3526, ADR 3527).
//!
//! The stream is ITU-T H.264's simplest that carries pictures: a Baseline sequence and picture
//! parameter set and IDR slices whose every macroblock is `I_PCM` (7.3.5, `mb_type` 25 of Table
//! 7-11), which carries its samples as raw bytes, with the deblocking filter disabled by the slice
//! (`disable_deblocking_filter_idc` 1, 7.4.3). So the decoded picture is the samples written, byte
//! for byte, by the standard's own construction — no other decoder is consulted for what the answer
//! is. The syntax is cited from Rec. ITU-T H.264 (V16) (06/2026), held as
//! `doc/ITU-T-H.264-202606.pdf`, not quoted.

#![expect(
    clippy::cast_possible_truncation,
    reason = "test code: a fixture built here fails loudly, and its sizes are a few kilobytes"
)]

use std::ops::ControlFlow;

use pdf_media_worker::mp4::movie_video_track;
use pdf_media_worker::video::{Session, Signal, VideoError};
use pdf_media_worker::{MediaError, MediaWorker};

const WORKER: &str = env!("CARGO_BIN_EXE_pdf-media-worker");

#[path = "support/pcm_stream.rs"]
mod pcm_stream;

use pcm_stream::{
    Coding, Tables, held_stream, movie_of, mp4, mp4_with, picture_set, sequence_with_vui, stream,
};

#[test]
fn every_pcm_picture_decodes_to_the_samples_it_carries() {
    let (sets, units, pictures) = stream(3, 32, 32, Coding::Progressive);
    let mut session = Session::for_track(4, &sets).expect("the parameter sets");
    let mut decoded = Vec::new();
    for unit in &units {
        decoded.extend(session.sample(unit).expect("a picture"));
    }
    decoded.extend(session.finish());
    assert_eq!(decoded, pictures);
}

#[test]
fn a_run_with_no_refresh_is_held_for_order_and_every_picture_output() {
    // Past `REORDER_FRAMES` pictures, the session outputs as it decodes; at the end, the rest.
    let (sets, units, picture) = held_stream(40, 32, 32);
    let mut session = Session::for_track(4, &sets).unwrap();
    let mut decoded = 0;
    let mut early = 0;
    for unit in &units {
        let out = session.sample(unit).expect("a picture");
        early += out.len();
        decoded += out.iter().filter(|out| **out == picture).count();
    }
    let rest = session.finish();
    decoded += rest.iter().filter(|out| **out == picture).count();
    assert_eq!(decoded, 41, "every picture is the IDR picture, copied");
    assert_eq!(early, 41 - pdf_media_worker::video::REORDER_FRAMES);
}

/// The media worker's address space at its peak over a 1080-line clip that holds the most pictures
/// a session holds, read from `/proc` by the host (ADR 3528).
#[test]
// not a gate: it prints the figure `Profile::Media`'s ceiling was set from; the end-to-end tests
// above are what fail if the ceiling is too small for a clip.
#[ignore = "a measurement a person reads, of a 1080-line clip through the confined worker"]
fn the_worker_s_address_space_over_a_held_1080_line_clip() {
    let (sets, units, picture) = held_stream(60, 1920, 1088);
    let file = mp4(&sets, &units, 1920, 1088);
    let mut worker = MediaWorker::with_program(WORKER.into());
    let mut decoded = 0;
    let mut peak = String::new();
    let streamed = worker
        .decode_mp4_video(&file, &mut |out| {
            assert_eq!(out, picture);
            decoded += 1;
            ControlFlow::Continue(())
        })
        .expect("decoded");
    if let Some(process) = worker.process_id() {
        let status = std::fs::read_to_string(format!("/proc/{process}/status")).unwrap();
        peak = status
            .lines()
            .filter(|line| line.starts_with("VmPeak") || line.starts_with("VmHWM"))
            .collect::<Vec<_>>()
            .join("; ");
    }
    println!("1920x1088, {} pictures: {peak}", streamed.pictures);
    assert_eq!(decoded, 61);
}

#[test]
fn an_mbaff_sequence_is_handed_off_by_name_before_any_picture() {
    let (sets, _, _) = stream(1, 32, 32, Coding::Mbaff);
    let refused = Session::for_track(4, &sets).expect_err("MBAFF");
    assert!(matches!(refused, VideoError::Interlaced(_)), "{refused:?}");
    assert!(
        refused.to_string().contains("this machine's own player"),
        "{refused}"
    );
}

#[test]
fn a_field_picture_is_handed_off_by_name() {
    let (sets, units, _) = stream(1, 32, 32, Coding::Fields);
    let mut session = Session::for_track(4, &sets).expect("a sequence that allows fields");
    let refused = session.sample(&units[0]).expect_err("a field");
    assert!(matches!(refused, VideoError::Interlaced(_)), "{refused:?}");
}

#[test]
fn a_length_that_runs_past_its_sample_is_refused() {
    let (sets, units, _) = stream(1, 16, 16, Coding::Progressive);
    let mut session = Session::for_track(4, &sets).unwrap();
    let cut = &units[0][..units[0].len() - 1];
    assert!(matches!(session.sample(cut), Err(VideoError::Refused(_))));
}

#[test]
fn the_mp4_reader_reads_the_video_track() {
    let (sets, units, _) = stream(2, 32, 16, Coding::Progressive);
    let file = mp4(&sets, &units, 32, 16);
    let track = movie_video_track(movie_of(&file), file.len() as u64).expect("the track");
    assert_eq!((track.width, track.height, track.length_size), (32, 16, 4));
    assert_eq!(track.parameter_sets, sets);
    assert_eq!(track.time_scale, 25_000);
    assert_eq!(track.access_units.len(), 2);
    let first = track.access_units[0];
    let at = first.offset as usize;
    assert_eq!(&file[at..at + first.size as usize], &units[0][..]);
}

#[test]
fn the_confined_worker_writes_every_picture_into_the_ring() {
    let (sets, units, pictures) = stream(40, 48, 32, Coding::Progressive);
    let file = mp4(&sets, &units, 48, 32);
    let mut worker = MediaWorker::with_program(WORKER.into());
    let mut decoded = Vec::new();
    let streamed = worker
        .decode_mp4_video(&file, &mut |picture| {
            decoded.push(picture);
            ControlFlow::Continue(())
        })
        .expect("decoded");
    assert_eq!(decoded, pictures);
    assert_eq!(streamed.pictures, 40);
    assert!(!streamed.stopped);
    // The same worker, its ring kept, takes the next clip.
    let again = worker
        .decode_mp4_video(&file, &mut |_| ControlFlow::Continue(()))
        .expect("decoded again");
    assert_eq!(again.pictures, 40);
    assert_eq!(worker.starts(), 1);
}

/// Each picture's time is its sample's decoding time on the track's clock: the fixture's `stts`
/// states one duration of 1000 for every sample and `mdhd` a time scale of 25 000, so picture `k`
/// is at `k × 40` ms (ISO/IEC 14496-12, 8.6.1.2; no `ctts`, so presentation is decoding order).
#[test]
fn each_picture_is_handed_on_at_its_sample_s_time() {
    let (sets, units, _) = stream(10, 16, 16, Coding::Progressive);
    let file = mp4(&sets, &units, 16, 16);
    let mut worker = MediaWorker::with_program(WORKER.into());
    let mut times = Vec::new();
    worker
        .decode_mp4_video_timed(&file, &mut |_, at| {
            times.push(at);
            ControlFlow::Continue(())
        })
        .expect("decoded");
    let expected: Vec<_> = (0..10u64)
        .map(|k| std::time::Duration::from_millis(k * 40))
        .collect();
    assert_eq!(times, expected);
}

#[test]
fn a_caller_that_stops_stops_the_track() {
    let (sets, units, _) = stream(40, 16, 16, Coding::Progressive);
    let file = mp4(&sets, &units, 16, 16);
    let mut worker = MediaWorker::with_program(WORKER.into());
    let mut seen = 0;
    let streamed = worker
        .decode_mp4_video(&file, &mut |_| {
            seen += 1;
            if seen == 5 {
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            }
        })
        .expect("stopped, not failed");
    assert!(streamed.stopped);
    assert_eq!(streamed.pictures, 5);
}

#[test]
fn an_interlaced_track_is_handed_off_by_the_worker_by_name() {
    for coding in [Coding::Mbaff, Coding::Fields] {
        let (sets, units, _) = stream(2, 32, 32, coding);
        let file = mp4(&sets, &units, 32, 32);
        let mut worker = MediaWorker::with_program(WORKER.into());
        let refused = worker
            .decode_mp4_video(&file, &mut |_| ControlFlow::Continue(()))
            .expect_err("interlaced");
        assert!(matches!(refused, MediaError::Interlaced(_)), "{refused:?}");
    }
}

#[test]
fn a_sound_file_holds_no_video_track_and_says_so() {
    let (sets, units, _) = stream(1, 16, 16, Coding::Progressive);
    let mut file = mp4(&sets, &units, 16, 16);
    let at = file
        .windows(4)
        .position(|window| window == b"vide")
        .unwrap();
    file[at..at + 4].copy_from_slice(b"soun");
    let mut worker = MediaWorker::with_program(WORKER.into());
    let refused = worker
        .decode_mp4_video(&file, &mut |_| ControlFlow::Continue(()))
        .expect_err("no video");
    assert!(
        refused.to_string().contains("no enabled video track"),
        "{refused}"
    );
}

/// The VUI's signal reaches each picture, through the session and through the confined worker's
/// ring, from a Baseline set and from a High set whose scaling lists stand before it; a set with no
/// colour description leaves Annex E's unspecified codes (E.2.1).
#[test]
fn the_vui_s_signal_reaches_every_picture() {
    let stated = Signal {
        full_range: true,
        colour_primaries: 1,
        transfer_characteristics: 1,
        matrix_coefficients: 1,
        chroma_sample_loc_type: 2,
    };
    let undescribed = Signal {
        full_range: false,
        chroma_sample_loc_type: 1,
        ..Signal::default()
    };
    for (high, signal) in [(false, stated), (true, undescribed)] {
        let (_, units, mut pictures) = stream(3, 48, 32, Coding::Progressive);
        let sets = vec![sequence_with_vui(3, 2, high, signal), picture_set()];
        for picture in &mut pictures {
            picture.signal = signal;
        }
        let mut session = Session::for_track(4, &sets).expect("the parameter sets");
        let mut decoded = Vec::new();
        for unit in &units {
            decoded.extend(session.sample(unit).expect("a picture"));
        }
        decoded.extend(session.finish());
        assert_eq!(decoded, pictures, "high profile {high}");
        let file = mp4(&sets, &units, 48, 32);
        let mut worker = MediaWorker::with_program(WORKER.into());
        let mut crossed = Vec::new();
        worker
            .decode_mp4_video(&file, &mut |picture| {
                crossed.push(picture);
                ControlFlow::Continue(())
            })
            .expect("decoded");
        assert_eq!(crossed, pictures, "high profile {high}, through the ring");
    }
}

/// A sequence parameter set cut inside its VUI is refused out loud, never read as stating nothing.
#[test]
fn a_set_cut_inside_its_vui_is_refused() {
    let mut set = sequence_with_vui(3, 2, false, Signal::default());
    set.truncate(set.len() - 3);
    let error = Session::for_track(4, &[set, picture_set()]).expect_err("refused");
    assert!(
        matches!(&error, VideoError::Refused(sentence) if sentence.contains("VUI")),
        "{error:?}"
    );
}

/// The times a decode of `file` from `from` hands its pictures on at, in milliseconds, and the
/// index in `pictures` of each picture handed on.
#[expect(
    clippy::expect_used,
    reason = "a test's helper: a stream this file wrote, decoded wrong, fails the test loudly"
)]
fn timed_from(
    file: &[u8],
    from: std::time::Duration,
    pictures: &[pdf_media_worker::video::Picture],
) -> (Vec<u128>, Vec<usize>, Option<String>) {
    let mut worker = MediaWorker::with_program(WORKER.into());
    let mut times = Vec::new();
    let mut which = Vec::new();
    let streamed = worker
        .decode_mp4_video_from(file, from, &mut |picture, at| {
            times.push(at.as_millis());
            which.push(
                pictures
                    .iter()
                    .position(|known| *known == picture)
                    .expect("a picture of the stream"),
            );
            ControlFlow::Continue(())
        })
        .expect("decoded");
    (times, which, streamed.note)
}

/// ISO/IEC 14496-12:2012 8.6.1.3, "CT(n) = DT(n) + CTTS(n)", and 8.6.6's edit list, whose media
/// time is "in composition time": four pictures 40 ms apart composed 2000 units (80 ms at 25 000 a
/// second) after they are decoded are presented from 80 ms with no edit list, and from 0 ms under
/// the standard's own shift — "Entry-count = 1, Segment-duration = 0, Media-Time = 20" with 2000 in
/// place of 20. A leading empty edit of 500 ms (the movie's 1000 a second) delays them by that
/// (ADR 3554).
#[test]
fn composition_offsets_and_the_edit_list_time_each_picture() {
    let (sets, units, pictures) = stream(4, 16, 16, Coding::Progressive);
    let offset = Tables {
        composition: Some((0, vec![(4, 2000)])),
        ..Tables::default()
    };
    let file = mp4_with(&sets, &units, 16, 16, &offset);
    let zero = std::time::Duration::ZERO;
    assert_eq!(timed_from(&file, zero, &pictures).0, [80, 120, 160, 200]);
    let shifted = Tables {
        edits: Some(vec![(0, 2000)]),
        ..offset.clone()
    };
    let file = mp4_with(&sets, &units, 16, 16, &shifted);
    let (times, which, note) = timed_from(&file, zero, &pictures);
    assert_eq!(times, [0, 40, 80, 120]);
    assert_eq!(which, [0, 1, 2, 3]);
    assert_eq!(note, None);
    let delayed = Tables {
        edits: Some(vec![(500, -1), (0, 2000)]),
        ..offset.clone()
    };
    let file = mp4_with(&sets, &units, 16, 16, &delayed);
    assert_eq!(timed_from(&file, zero, &pictures).0, [500, 540, 580, 620]);
    // A version 1 table's offsets are signed: −1000 units presents each picture 40 ms early, and
    // the one composed before the edit's start is decoded and not presented.
    let early = Tables {
        composition: Some((1, vec![(4, -1000)])),
        edits: Some(vec![(0, 0)]),
        ..Tables::default()
    };
    let file = mp4_with(&sets, &units, 16, 16, &early);
    let (times, which, _) = timed_from(&file, zero, &pictures);
    assert_eq!(times, [0, 40, 80]);
    assert_eq!(which, [1, 2, 3]);
    // Two edits of the media are not a shape this reader applies, and it says so.
    let twice = Tables {
        edits: Some(vec![(80, 0), (80, 0)]),
        ..Tables::default()
    };
    let file = mp4_with(&sets, &units, 16, 16, &twice);
    let (times, _, note) = timed_from(&file, zero, &pictures);
    assert_eq!(times, [0, 40, 80, 120]);
    assert!(note.is_some_and(|note| note.contains("video track's edit list")));
}

/// A decode from a place begins at the last sync sample presented at or before it (8.6.2's
/// `stss`), not at the track's start: samples 1 and 3 are sync samples, so a decode from 85 ms
/// begins at the third sample (80 ms) and its first picture is that sample's. With no `stss`,
/// "every sample is a sync sample" and the decode begins at the sample presented at or before the
/// place (ADR 3555).
#[test]
fn a_decode_from_a_place_begins_at_the_sync_sample_before_it() {
    let (sets, units, pictures) = stream(4, 16, 16, Coding::Progressive);
    let synced = Tables {
        sync: Some(vec![1, 3]),
        ..Tables::default()
    };
    let file = mp4_with(&sets, &units, 16, 16, &synced);
    let from = std::time::Duration::from_millis(85);
    let (times, which, _) = timed_from(&file, from, &pictures);
    assert_eq!(times, [80, 120]);
    assert_eq!(which, [2, 3]);
    let early = std::time::Duration::from_millis(79);
    assert_eq!(timed_from(&file, early, &pictures).1, [0, 1, 2, 3]);
    let file = mp4(&sets, &units, 16, 16);
    let (times, which, _) = timed_from(&file, std::time::Duration::from_millis(125), &pictures);
    assert_eq!(times, [120]);
    assert_eq!(which, [3]);
}

/// The reader's tables: `ctts` expanded to one offset a sample, signed in version 1; `stss`'s
/// numbers from 1 as indices from 0; `elst`'s entries; and a `ctts` that covers other than every
/// sample, or an `stss` out of order, refused.
#[test]
fn the_mp4_reader_reads_the_timing_tables() {
    let (sets, units, _) = stream(3, 16, 16, Coding::Progressive);
    let tables = Tables {
        composition: Some((1, vec![(1, 0), (2, -500)])),
        sync: Some(vec![1, 3]),
        edits: Some(vec![(100, -1), (0, 7)]),
    };
    let file = mp4_with(&sets, &units, 16, 16, &tables);
    let track = movie_video_track(movie_of(&file), file.len() as u64).expect("the track");
    assert_eq!(track.composition_offsets, [0, -500, -500]);
    assert_eq!(track.sync_samples, Some(vec![0, 2]));
    assert_eq!(track.movie_time_scale, Some(1000));
    assert_eq!(
        track
            .edits
            .iter()
            .map(|edit| (edit.segment_duration, edit.media_time))
            .collect::<Vec<_>>(),
        [(100, -1), (0, 7)]
    );
    let short = Tables {
        composition: Some((0, vec![(2, 0)])),
        ..Tables::default()
    };
    let file = mp4_with(&sets, &units, 16, 16, &short);
    assert!(movie_video_track(movie_of(&file), file.len() as u64).is_err());
    let unordered = Tables {
        sync: Some(vec![2, 1]),
        ..Tables::default()
    };
    let file = mp4_with(&sets, &units, 16, 16, &unordered);
    assert!(movie_video_track(movie_of(&file), file.len() as u64).is_err());
}

/// Eight pictures, each carrying its own index in its samples (`pcm_stream::samples`), in two groups
/// of four — `stss` names samples 1 and 5 — at 40 ms a picture (ISO/IEC 14496-12, 8.6.1.2, 8.6.2).
fn two_groups() -> (Vec<u8>, Vec<pdf_media_worker::video::Picture>) {
    let (sets, units, pictures) = stream(8, 32, 32, Coding::Progressive);
    let tables = Tables {
        sync: Some(vec![1, 5]),
        ..Tables::default()
    };
    (mp4_with(&sets, &units, 32, 32, &tables), pictures)
}

/// One group: `stss` names the first sample alone, so the pictures that copy it are not sync
/// samples (a track without `stss` makes every sample one, ISO/IEC 14496-12, 8.6.2).
fn one_group() -> Tables {
    Tables {
        sync: Some(vec![1]),
        ..Tables::default()
    }
}

/// Decodes `file` backward in a confined worker: each picture's index in `pictures` and its time
/// in the pass.
#[expect(
    clippy::expect_used,
    reason = "a test's helper: a fixture that does not play should fail the test loudly"
)]
fn played_backward(
    file: &[u8],
    pictures: &[pdf_media_worker::video::Picture],
    until: Option<std::time::Duration>,
    from: std::time::Duration,
) -> Vec<(usize, u128)> {
    let mut worker = MediaWorker::with_program(WORKER.into());
    let mut presented = Vec::new();
    worker
        .decode_mp4_video_backward(file, until, from, &mut |picture, at| {
            let index = pictures
                .iter()
                .position(|known| *known == picture)
                .expect("a picture of the fixture");
            presented.push((index, at.as_millis()));
            ControlFlow::Continue(())
        })
        .expect("played backward");
    presented
}

/// ISO 32000-2 §13.4, Table 307: "If the value of this entry is negative, the movie shall be
/// played backward with respect to Start and Duration." The worker decodes each group from its sync
/// sample and lets it out last picture first, the last group first, so the pictures arrive in the
/// reverse of their presentation order; picture `k` ends at `(k + 1) × 40` ms, so its time in a pass
/// from the track's end (320 ms) is `(7 − k) × 40` ms (ADR 3837).
#[test]
fn a_movie_played_backward_presents_its_pictures_last_first() {
    let (file, pictures) = two_groups();
    let presented = played_backward(&file, &pictures, None, std::time::Duration::ZERO);
    let expected: Vec<(usize, u128)> = (0..8)
        .rev()
        .map(|index| (index, (7 - index as u128) * 40))
        .collect();
    assert_eq!(presented, expected);
}

/// "With respect to Start and Duration": a pass that begins at a section's end — 200 ms, where
/// picture 5 begins — presents picture 4 first, at zero, and nothing presented at or after its
/// beginning; the group that holds pictures 4 to 7 is still decoded from its sync sample.
#[test]
fn a_backward_pass_begins_at_its_section_s_end() {
    let (file, pictures) = two_groups();
    let presented = played_backward(
        &file,
        &pictures,
        Some(std::time::Duration::from_millis(200)),
        std::time::Duration::ZERO,
    );
    assert_eq!(presented, [(4, 0), (3, 40), (2, 80), (1, 120), (0, 160)]);
}

/// A backward pass resumed 160 ms in: the last group's pictures are all before that in the pass,
/// so it is not decoded, and the first group's are presented from 160 ms down.
#[test]
fn a_resumed_backward_pass_skips_the_groups_before_it() {
    let (file, pictures) = two_groups();
    let presented = played_backward(
        &file,
        &pictures,
        None,
        std::time::Duration::from_millis(160),
    );
    assert_eq!(presented, [(3, 160), (2, 200), (1, 240), (0, 280)]);
}

/// A group too long to hold at once is let out in parts, each decoded from the group's sync sample
/// and holding its own range of places (ADR 3843): with a part bound of two 32 by 32 pictures,
/// each group of four is two halves, the later half first, and the pass is exactly the one an
/// unsplit decode gives — every picture last first at `(7 − k) × 40` ms, and resumed 160 ms in
/// from the first group alone. The wrong answers excluded: the halves in their recorded order, a
/// half's pictures timed as the other's, and a picture of the first half held twice.
#[test]
fn a_group_split_into_parts_plays_as_one() {
    let (file, pictures) = two_groups();
    let planes = 32 * 32 + 2 * 16 * 16;
    let split = |until, from| {
        let mut worker = MediaWorker::with_program(WORKER.into()).with_group_bytes(2 * planes);
        let mut presented = Vec::new();
        worker
            .decode_mp4_video_backward(&file, until, from, &mut |picture, at| {
                let index = pictures
                    .iter()
                    .position(|known| *known == picture)
                    .expect("a picture of the fixture");
                presented.push((index, at.as_millis()));
                ControlFlow::Continue(())
            })
            .expect("played backward in parts");
        presented
    };
    let expected: Vec<(usize, u128)> = (0..8)
        .rev()
        .map(|index| (index, (7 - index as u128) * 40))
        .collect();
    assert_eq!(split(None, std::time::Duration::ZERO), expected);
    assert_eq!(
        split(None, std::time::Duration::from_millis(160)),
        [(3, 160), (2, 200), (1, 240), (0, 280)]
    );
    let mut one = MediaWorker::with_program(WORKER.into()).with_group_bytes(planes);
    let mut presented = Vec::new();
    one.decode_mp4_video_backward(&file, None, std::time::Duration::ZERO, &mut |picture, _| {
        presented.push(pictures.iter().position(|known| *known == picture));
        ControlFlow::Continue(())
    })
    .expect("four parts of one picture");
    assert_eq!(presented, (0..8).rev().map(Some).collect::<Vec<_>>());
}

/// A group whose decoded planes pass `MAX_GROUP_PARTS` parts of `MAX_GROUP_BYTES` is refused by
/// name before any of it is decoded: 129 1920 by 1088 pictures — an IDR and 128 that copy it, one
/// group — are 404 213 760 bytes, past the four parts of 100 663 296 one backward pass holds it in.
#[test]
fn a_group_past_the_bound_is_refused_by_name() {
    let (sets, units, _) = held_stream(128, 1920, 1088);
    let file = mp4_with(&sets, &units, 1920, 1088, &one_group());
    let mut worker = MediaWorker::with_program(WORKER.into());
    let mut handed = 0;
    let refused = worker
        .decode_mp4_video_backward(&file, None, std::time::Duration::ZERO, &mut |_, _| {
            handed += 1;
            ControlFlow::Continue(())
        })
        .expect_err("past the bound");
    assert!(matches!(refused, MediaError::Refused(_)), "{refused:?}");
    assert!(
        refused
            .to_string()
            .contains("a movie played backward holds"),
        "{refused}"
    );
    assert_eq!(handed, 0);
}

/// The media worker's address space at its peak over a 1080-line group of thirty-two pictures played
/// backward — the most `MAX_GROUP_BYTES` admits in one part — and over a two-second group of sixty,
/// let out in two halves, and of 128 in the four parts `MAX_GROUP_PARTS` admits (ADR 3843); and
/// the time from the request to the first picture, which is the last part's decode from the sync
/// sample, and to the last (ADRs 3837, 3843).
#[test]
// not a gate: it prints the figures ADR 3837 records; the tests above are what fail on behaviour.
#[ignore = "a measurement a person reads, of a 1080-line group through the confined worker"]
fn the_worker_s_address_space_over_a_backward_1080_line_group() {
    for (width, height, skipped) in [
        (32, 32, 7),
        (1920, 1088, 31),
        (1920, 1088, 59),
        (1920, 1088, 127),
    ] {
        let (sets, units, picture) = held_stream(skipped, width, height);
        let file = mp4_with(&sets, &units, width as u16, height as u16, &one_group());
        let mut worker = MediaWorker::with_program(WORKER.into());
        // Started, and its ring attached, before the clock starts: what a second clip costs.
        worker
            .decode_mp4_video(&file, &mut |_| ControlFlow::Break(()))
            .expect("started");
        let asked = std::time::Instant::now();
        let mut first = None;
        let mut decoded = 0;
        worker
            .decode_mp4_video_backward(&file, None, std::time::Duration::ZERO, &mut |out, _| {
                first.get_or_insert_with(|| asked.elapsed());
                assert_eq!(out, picture);
                decoded += 1;
                ControlFlow::Continue(())
            })
            .expect("played backward");
        let mut peak = String::new();
        if let Some(process) = worker.process_id() {
            let status = std::fs::read_to_string(format!("/proc/{process}/status")).unwrap();
            peak = status
                .lines()
                .filter(|line| line.starts_with("VmPeak") || line.starts_with("VmHWM"))
                .collect::<Vec<_>>()
                .join("; ");
        }
        let planes = decoded * (width * height + 2 * (width / 2) * (height / 2));
        println!(
            "{width}x{height}, a group of {decoded} pictures, {planes} bytes of planes: first \
             backward picture after {:?}, last after {:?}; {peak}",
            first.unwrap_or_default(),
            asked.elapsed()
        );
        assert_eq!(decoded, skipped as usize + 1);
    }
}
