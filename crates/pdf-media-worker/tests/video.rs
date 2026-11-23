//! H.264 in the media worker: a stream written here by hand, whose every picture is known before it
//! is decoded, through the session, the MP4 reader and the confined worker (ADR 3526, ADR 3527).
//!
//! The stream is ITU-T H.264's simplest that carries pictures: a Baseline sequence and picture
//! parameter set and IDR slices whose every macroblock is `I_PCM` (7.3.5, `mb_type` 25 of Table
//! 7-11), which carries its samples as raw bytes, with the deblocking filter disabled by the slice
//! (`disable_deblocking_filter_idc` 1, 7.4.3). So the decoded picture is the samples written, byte
//! for byte, by the standard's own construction — no other decoder is consulted for what the answer
//! is. ITU-T H.264 is not held in `doc/`; the syntax is cited, not quoted.

#![expect(
    clippy::unwrap_used,
    clippy::arithmetic_side_effects,
    clippy::cast_possible_truncation,
    reason = "test code: a fixture built here fails loudly, and its sizes are a few kilobytes"
)]

use std::ops::ControlFlow;

use pdf_media_worker::mp4::movie_video_track;
use pdf_media_worker::video::{Picture, Session, VideoError};
use pdf_media_worker::{MediaError, MediaWorker};

const WORKER: &str = env!("CARGO_BIN_EXE_pdf-media-worker");

/// Writes bits most significant first, and Exp-Golomb codes (9.1).
#[derive(Default)]
struct Bits {
    bytes: Vec<u8>,
    used: u8,
}

impl Bits {
    fn bit(&mut self, bit: bool) {
        if self.used == 0 {
            self.bytes.push(0);
        }
        if bit {
            *self.bytes.last_mut().unwrap() |= 0x80 >> self.used;
        }
        self.used = (self.used + 1) % 8;
    }

    fn u(&mut self, count: u32, value: u32) {
        for shift in (0..count).rev() {
            self.bit(value >> shift & 1 == 1);
        }
    }

    fn ue(&mut self, value: u32) {
        let coded = value + 1;
        let length = 32 - coded.leading_zeros();
        self.u(length - 1, 0);
        self.u(length, coded);
    }

    fn align_zero(&mut self) {
        while self.used != 0 {
            self.bit(false);
        }
    }

    /// `rbsp_trailing_bits` (7.3.2.11): a one, then zeros to the byte.
    fn trailing(mut self) -> Vec<u8> {
        self.bit(true);
        self.align_zero();
        self.bytes
    }
}

/// A NAL unit: its header byte and its payload with emulation prevention (7.4.1).
fn nal(header: u8, rbsp: &[u8]) -> Vec<u8> {
    let mut unit = vec![header];
    let mut zeros = 0;
    for &byte in rbsp {
        if zeros >= 2 && byte <= 3 {
            unit.push(3);
            zeros = 0;
        }
        unit.push(byte);
        zeros = if byte == 0 { zeros + 1 } else { 0 };
    }
    unit
}

/// How the sequence codes its pictures.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Coding {
    Progressive,
    /// `frame_mbs_only_flag` 0 and `mb_adaptive_frame_field_flag` 1: MBAFF.
    Mbaff,
    /// `frame_mbs_only_flag` 0, and every slice a field (`field_pic_flag` 1).
    Fields,
}

/// The sequence parameter set (7.3.2.1.1): Baseline at level 5.1, `pic_order_cnt_type` 2, and
/// `references` reference frames.
fn sequence(width_mbs: u32, height_mbs: u32, coding: Coding, references: u32) -> Vec<u8> {
    let mut bits = Bits::default();
    bits.u(8, 66);
    bits.u(8, 0);
    bits.u(8, 51);
    bits.ue(0);
    bits.ue(0);
    bits.ue(2);
    bits.ue(references);
    bits.bit(false);
    bits.ue(width_mbs - 1);
    if coding == Coding::Progressive {
        bits.ue(height_mbs - 1);
        bits.bit(true);
    } else {
        bits.ue(height_mbs / 2 - 1);
        bits.bit(false);
        bits.bit(coding == Coding::Mbaff);
    }
    bits.bit(true);
    bits.bit(false);
    bits.bit(false);
    nal(0x67, &bits.trailing())
}

/// The picture parameter set (7.3.2.2): CAVLC, one slice group, the deblocking control present.
fn picture_set() -> Vec<u8> {
    let mut bits = Bits::default();
    bits.ue(0);
    bits.ue(0);
    bits.bit(false);
    bits.bit(false);
    bits.ue(0);
    bits.ue(0);
    bits.ue(0);
    bits.bit(false);
    bits.u(2, 0);
    bits.ue(0);
    bits.ue(0);
    bits.ue(0);
    bits.bit(true);
    bits.bit(false);
    bits.bit(false);
    nal(0x68, &bits.trailing())
}

/// The samples of picture `index`: never zero, different in every picture, plane and position.
fn samples(index: u32, width: usize, height: usize) -> Picture {
    let value = |plane: u32, at: usize| (1 + (at as u32 * 7 + plane * 50 + index * 31) % 254) as u8;
    let chroma = (width / 2) * (height / 2);
    Picture {
        width,
        height,
        y: (0..width * height).map(|at| value(0, at)).collect(),
        u: (0..chroma).map(|at| value(1, at)).collect(),
        v: (0..chroma).map(|at| value(2, at)).collect(),
    }
}

/// An IDR slice (7.3.3) of `picture`'s samples, every macroblock `I_PCM` (7.3.5).
fn idr_slice(index: u32, picture: &Picture, coding: Coding) -> Vec<u8> {
    let mut bits = Bits::default();
    bits.ue(0);
    bits.ue(7);
    bits.ue(0);
    bits.u(4, 0);
    if coding != Coding::Progressive {
        bits.bit(coding == Coding::Fields);
        if coding == Coding::Fields {
            bits.bit(false);
        }
    }
    bits.ue(index % 2);
    bits.bit(false);
    bits.bit(false);
    bits.ue(0);
    bits.ue(1);
    let (width, height) = (picture.width, picture.height);
    for mb_y in 0..height / 16 {
        for mb_x in 0..width / 16 {
            bits.ue(25);
            bits.align_zero();
            for row in 0..16 {
                let at = (mb_y * 16 + row) * width + mb_x * 16;
                bits.bytes.extend_from_slice(&picture.y[at..at + 16]);
            }
            for plane in [&picture.u, &picture.v] {
                for row in 0..8 {
                    let at = (mb_y * 8 + row) * (width / 2) + mb_x * 8;
                    bits.bytes.extend_from_slice(&plane[at..at + 8]);
                }
            }
        }
    }
    nal(0x65, &bits.trailing())
}

/// A P slice (7.3.3, 7.3.4) of frame `frame_num` in which every macroblock is skipped
/// (`mb_skip_run` the picture's count, 7.4.4): the picture is its reference, copied.
fn skipped_slice(frame_num: u32, macroblocks: u32) -> Vec<u8> {
    let mut bits = Bits::default();
    bits.ue(0);
    bits.ue(5);
    bits.ue(0);
    bits.u(4, frame_num % 16);
    bits.bit(false);
    bits.bit(false);
    bits.bit(false);
    bits.ue(0);
    bits.ue(1);
    bits.ue(macroblocks);
    nal(0x41, &bits.trailing())
}

/// An IDR picture of `width` by `height` then `skipped` pictures that copy it, each kept as a
/// reference — sixteen of them at once, the most Annex A allows any level — as MP4 samples, with the
/// parameter sets and the picture every one of them decodes to.
fn held_stream(skipped: u32, width: usize, height: usize) -> (Vec<Vec<u8>>, Vec<Vec<u8>>, Picture) {
    let (mut sets, mut units, mut pictures) = stream(1, width, height, Coding::Progressive);
    sets[0] = sequence(
        width as u32 / 16,
        height as u32 / 16,
        Coding::Progressive,
        16,
    );
    let macroblocks = (width / 16 * (height / 16)) as u32;
    for frame_num in 1..=skipped {
        let slice = skipped_slice(frame_num, macroblocks);
        let mut unit = (slice.len() as u32).to_be_bytes().to_vec();
        unit.extend_from_slice(&slice);
        units.push(unit);
    }
    (sets, units, pictures.remove(0))
}

/// `count` pictures of `width` by `height` as MP4 samples (lengths of four bytes), with the
/// parameter sets beside them and the pictures they must decode to.
fn stream(
    count: u32,
    width: usize,
    height: usize,
    coding: Coding,
) -> (Vec<Vec<u8>>, Vec<Vec<u8>>, Vec<Picture>) {
    let sets = vec![
        sequence(width as u32 / 16, height as u32 / 16, coding, 1),
        picture_set(),
    ];
    let pictures: Vec<Picture> = (0..count)
        .map(|index| samples(index, width, height))
        .collect();
    let units = pictures
        .iter()
        .enumerate()
        .map(|(index, picture)| {
            let slice = idr_slice(index as u32, picture, coding);
            let mut unit = (slice.len() as u32).to_be_bytes().to_vec();
            unit.extend_from_slice(&slice);
            unit
        })
        .collect();
    (sets, units, pictures)
}

fn bx(kind: [u8; 4], payload: &[u8]) -> Vec<u8> {
    let mut bytes = ((payload.len() + 8) as u32).to_be_bytes().to_vec();
    bytes.extend_from_slice(&kind);
    bytes.extend_from_slice(payload);
    bytes
}

fn full(kind: [u8; 4], flags: u32, fields: &[u8]) -> Vec<u8> {
    let mut payload = flags.to_be_bytes().to_vec();
    payload.extend_from_slice(fields);
    bx(kind, &payload)
}

fn words(values: &[u32]) -> Vec<u8> {
    values
        .iter()
        .flat_map(|value| value.to_be_bytes())
        .collect()
}

/// An MP4 file of one video track holding `units`, its movie before its media data (14496-12;
/// ISO/IEC 14496-15 for `avc1` and `avcC`).
fn mp4(sets: &[Vec<u8>], units: &[Vec<u8>], width: u16, height: u16) -> Vec<u8> {
    let mut record = vec![1, 66, 0, 30, 0xFF, 0xE1];
    record.extend_from_slice(&(sets[0].len() as u16).to_be_bytes());
    record.extend_from_slice(&sets[0]);
    record.push(1);
    record.extend_from_slice(&(sets[1].len() as u16).to_be_bytes());
    record.extend_from_slice(&sets[1]);
    let mut visual = vec![0; 6];
    visual.extend_from_slice(&1u16.to_be_bytes());
    visual.extend_from_slice(&[0; 16]);
    visual.extend_from_slice(&width.to_be_bytes());
    visual.extend_from_slice(&height.to_be_bytes());
    visual.extend_from_slice(&words(&[0x0048_0000, 0x0048_0000, 0]));
    visual.extend_from_slice(&1u16.to_be_bytes());
    visual.extend_from_slice(&[0; 32]);
    visual.extend_from_slice(&[0, 0x18, 0xFF, 0xFF]);
    visual.extend_from_slice(&bx(*b"avcC", &record));
    let mut descriptions = words(&[1]);
    descriptions.extend_from_slice(&bx(*b"avc1", &visual));
    let sizes: Vec<u32> = units.iter().map(|unit| unit.len() as u32).collect();
    let movie_for = |data_at: u32| {
        let mut table = full(*b"stsd", 0, &descriptions);
        table.extend_from_slice(&full(*b"stts", 0, &words(&[1, units.len() as u32, 1000])));
        table.extend_from_slice(&full(*b"stsc", 0, &words(&[1, 1, units.len() as u32, 1])));
        let mut stsz = words(&[0, units.len() as u32]);
        stsz.extend_from_slice(&words(&sizes));
        table.extend_from_slice(&full(*b"stsz", 0, &stsz));
        table.extend_from_slice(&full(*b"stco", 0, &words(&[1, data_at])));
        let mut info = bx(
            *b"dinf",
            &full(*b"dref", 0, &[words(&[1]), full(*b"url ", 1, &[])].concat()),
        );
        info.extend_from_slice(&bx(*b"stbl", &table));
        let mut media = full(*b"mdhd", 0, &words(&[0, 0, 25_000, 0, 0]));
        media.extend_from_slice(&full(
            *b"hdlr",
            0,
            &[words(&[0]), b"vide".to_vec(), vec![0; 13]].concat(),
        ));
        media.extend_from_slice(&bx(*b"minf", &info));
        let mut track = full(*b"tkhd", 1, &[0; 80]);
        track.extend_from_slice(&bx(*b"mdia", &media));
        let mut header = words(&[0, 0, 1000, 0, 0x0001_0000]);
        header.extend_from_slice(&[0; 76]);
        let mut movie = full(*b"mvhd", 0, &header);
        movie.extend_from_slice(&bx(*b"trak", &track));
        bx(*b"moov", &movie)
    };
    let kind = bx(*b"ftyp", b"isom\0\0\0\0isomavc1");
    let length = kind.len() + movie_for(0).len() + 8;
    let mut file = kind;
    file.extend_from_slice(&movie_for(length as u32));
    file.extend_from_slice(&bx(*b"mdat", &units.concat()));
    file
}

/// The movie box's payload: the box after `ftyp`, which [`mp4`] writes first.
fn movie_of(file: &[u8]) -> &[u8] {
    let at = u32::from_be_bytes(file[..4].try_into().unwrap()) as usize;
    let size = u32::from_be_bytes(file[at..at + 4].try_into().unwrap()) as usize;
    &file[at + 8..at + size]
}

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
