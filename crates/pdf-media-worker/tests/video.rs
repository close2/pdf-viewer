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

use pcm_stream::{Coding, held_stream, movie_of, mp4, picture_set, sequence_with_vui, stream};

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
