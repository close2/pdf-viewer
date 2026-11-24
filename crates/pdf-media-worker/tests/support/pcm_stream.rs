//! The hand-built H.264 stream of `video.rs` (ADR 3526): a Baseline sequence and picture parameter
//! set and IDR slices whose every macroblock is `I_PCM` (Rec. ITU-T H.264 7.3.5), so that each
//! decoded picture is the samples written, byte for byte, by the standard's own construction; and
//! the MP4 file that holds them (ISO/IEC 14496-12, 14496-15). Shared with `viewer-host`'s
//! `played_video.rs`, which plays the same clip through a window's video player (ADR 3545).

#![expect(
    clippy::arithmetic_side_effects,
    clippy::cast_possible_truncation,
    reason = "test code: a fixture built here fails loudly, and its sizes are a few kilobytes"
)]

use pdf_media_worker::video::{Picture, Signal};

/// Writes bits most significant first, and Exp-Golomb codes (9.1).
#[derive(Default)]
pub(crate) struct Bits {
    bytes: Vec<u8>,
    used: u8,
}

impl Bits {
    pub(crate) fn bit(&mut self, bit: bool) {
        if self.used == 0 {
            self.bytes.push(0);
        }
        // The byte this bit lands in was pushed above at its first bit.
        if bit && let Some(last) = self.bytes.last_mut() {
            *last |= 0x80 >> self.used;
        }
        self.used = (self.used + 1) % 8;
    }

    pub(crate) fn u(&mut self, count: u32, value: u32) {
        for shift in (0..count).rev() {
            self.bit(value >> shift & 1 == 1);
        }
    }

    pub(crate) fn ue(&mut self, value: u32) {
        let coded = value + 1;
        let length = 32 - coded.leading_zeros();
        self.u(length - 1, 0);
        self.u(length, coded);
    }

    pub(crate) fn align_zero(&mut self) {
        while self.used != 0 {
            self.bit(false);
        }
    }

    /// `rbsp_trailing_bits` (7.3.2.11): a one, then zeros to the byte.
    pub(crate) fn trailing(mut self) -> Vec<u8> {
        self.bit(true);
        self.align_zero();
        self.bytes
    }
}

/// A NAL unit: its header byte and its payload with emulation prevention (7.4.1).
pub(crate) fn nal(header: u8, rbsp: &[u8]) -> Vec<u8> {
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
pub(crate) enum Coding {
    Progressive,
    /// `frame_mbs_only_flag` 0 and `mb_adaptive_frame_field_flag` 1: MBAFF.
    Mbaff,
    /// `frame_mbs_only_flag` 0, and every slice a field (`field_pic_flag` 1).
    Fields,
}

/// The sequence parameter set (7.3.2.1.1): Baseline at level 5.1, `pic_order_cnt_type` 2, and
/// `references` reference frames.
pub(crate) fn sequence(
    width_mbs: u32,
    height_mbs: u32,
    coding: Coding,
    references: u32,
) -> Vec<u8> {
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

/// A progressive sequence parameter set that carries a VUI (E.1.1) stating `signal`: Baseline, or
/// High with a scaling matrix whose first list is present and asks for the default (7.3.2.1.1.1),
/// so that the reader walks the lists to reach the VUI. The VUI states an extended sample aspect
/// ratio and overscan before the signal, and nothing after the chroma siting.
pub(crate) fn sequence_with_vui(
    width_mbs: u32,
    height_mbs: u32,
    high: bool,
    signal: Signal,
) -> Vec<u8> {
    let mut bits = Bits::default();
    bits.u(8, if high { 100 } else { 66 });
    bits.u(8, 0);
    bits.u(8, 51);
    bits.ue(0);
    if high {
        bits.ue(1);
        bits.ue(0);
        bits.ue(0);
        bits.bit(false);
        bits.bit(true);
        for list in 0..8 {
            bits.bit(list == 0);
            if list == 0 {
                // delta_scale −8, se(v) code 16: nextScale becomes 0, the default list.
                bits.ue(16);
            }
        }
    }
    bits.ue(0);
    bits.ue(2);
    bits.ue(1);
    bits.bit(false);
    bits.ue(width_mbs - 1);
    bits.ue(height_mbs - 1);
    bits.bit(true);
    bits.bit(true);
    bits.bit(false);
    bits.bit(true);
    // aspect_ratio_info_present_flag, Extended_SAR and its two sixteen-bit fields.
    bits.bit(true);
    bits.u(8, 255);
    bits.u(16, 4);
    bits.u(16, 3);
    // overscan_info_present_flag and overscan_appropriate_flag.
    bits.bit(true);
    bits.bit(false);
    // video_signal_type_present_flag, video_format 5, the range, and the colour description
    // where any of its three codes is stated.
    bits.bit(true);
    bits.u(3, 5);
    bits.bit(signal.full_range);
    let described = (
        signal.colour_primaries,
        signal.transfer_characteristics,
        signal.matrix_coefficients,
    ) != (2, 2, 2);
    bits.bit(described);
    if described {
        bits.u(8, u32::from(signal.colour_primaries));
        bits.u(8, u32::from(signal.transfer_characteristics));
        bits.u(8, u32::from(signal.matrix_coefficients));
    }
    // chroma_loc_info_present_flag and the two fields' sitings.
    bits.bit(true);
    bits.ue(u32::from(signal.chroma_sample_loc_type));
    bits.ue(u32::from(signal.chroma_sample_loc_type));
    // No timing, no HRD, no picture structure, no bitstream restriction.
    for _ in 0..5 {
        bits.bit(false);
    }
    nal(0x67, &bits.trailing())
}

/// The picture parameter set (7.3.2.2): CAVLC, one slice group, the deblocking control present.
pub(crate) fn picture_set() -> Vec<u8> {
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
pub(crate) fn samples(index: u32, width: usize, height: usize) -> Picture {
    let value = |plane: u32, at: usize| (1 + (at as u32 * 7 + plane * 50 + index * 31) % 254) as u8;
    let chroma = (width / 2) * (height / 2);
    Picture {
        width,
        height,
        y: (0..width * height).map(|at| value(0, at)).collect(),
        u: (0..chroma).map(|at| value(1, at)).collect(),
        v: (0..chroma).map(|at| value(2, at)).collect(),
        signal: Signal::default(),
    }
}

/// An IDR slice (7.3.3) of `picture`'s samples, every macroblock `I_PCM` (7.3.5).
pub(crate) fn idr_slice(index: u32, picture: &Picture, coding: Coding) -> Vec<u8> {
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
pub(crate) fn skipped_slice(frame_num: u32, macroblocks: u32) -> Vec<u8> {
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
pub(crate) fn held_stream(
    skipped: u32,
    width: usize,
    height: usize,
) -> (Vec<Vec<u8>>, Vec<Vec<u8>>, Picture) {
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
pub(crate) fn stream(
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

pub(crate) fn bx(kind: [u8; 4], payload: &[u8]) -> Vec<u8> {
    let mut bytes = ((payload.len() + 8) as u32).to_be_bytes().to_vec();
    bytes.extend_from_slice(&kind);
    bytes.extend_from_slice(payload);
    bytes
}

pub(crate) fn full(kind: [u8; 4], flags: u32, fields: &[u8]) -> Vec<u8> {
    let mut payload = flags.to_be_bytes().to_vec();
    payload.extend_from_slice(fields);
    bx(kind, &payload)
}

pub(crate) fn words(values: &[u32]) -> Vec<u8> {
    values
        .iter()
        .flat_map(|value| value.to_be_bytes())
        .collect()
}

/// An MP4 file of one video track holding `units`, its movie before its media data (14496-12;
/// ISO/IEC 14496-15 for `avc1` and `avcC`).
pub(crate) fn mp4(sets: &[Vec<u8>], units: &[Vec<u8>], width: u16, height: u16) -> Vec<u8> {
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
pub(crate) fn movie_of(file: &[u8]) -> &[u8] {
    let word = |at: usize| u32::from_be_bytes([file[at], file[at + 1], file[at + 2], file[at + 3]]);
    let at = word(0) as usize;
    let size = word(at) as usize;
    &file[at + 8..at + size]
}
