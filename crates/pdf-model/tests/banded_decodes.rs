//! `pdf_model::image::banded_decodes`, the seam the `jpeg_bands` fuzz target reads (ADR 1495),
//! calibrated: on the band modules' own fixtures it cuts every frame its plans admit and answers
//! the whole frame's bytes. A seam that declined everything would leave the target comparing
//! nothing with nothing and passing.

use pdf_model::image::banded_decodes;

/// The fixtures `image/restart.rs` and `image/cut.rs` test against: 48 × 200 frames from
/// `libjpeg-turbo`'s `cjpeg`, with restart intervals and without.
const FIXTURES: [(&str, &[u8], bool); 7] = [
    ("restart/hv_row", include_bytes!("restart/hv_row.jpg"), true),
    ("restart/h_row", include_bytes!("restart/h_row.jpg"), true),
    (
        "restart/grey_row",
        include_bytes!("restart/grey_row.jpg"),
        true,
    ),
    ("cut/hv", include_bytes!("cut/hv.jpg"), false),
    ("cut/v", include_bytes!("cut/v.jpg"), false),
    ("cut/none", include_bytes!("cut/none.jpg"), false),
    ("cut/grey", include_bytes!("cut/grey.jpg"), false),
];

#[test]
fn every_fixture_is_cut_below_the_floor_and_decodes_to_the_whole_frame() {
    for (name, data, restarts) in FIXTURES {
        for lines in [8, 16, 64] {
            let decodes = banded_decodes(data, lines);
            let whole = decodes.whole.as_ref().expect("the whole frame decodes");
            let planned = if restarts {
                decodes.at_restarts.as_ref()
            } else {
                decodes.at_rows.as_ref()
            };
            let banded = planned.unwrap_or_else(|| panic!("{name} is cut in bands of {lines}"));
            assert!(
                banded == whole,
                "{name} in bands of {lines} lines moved a byte"
            );
        }
    }
}

#[test]
fn a_codestream_that_is_no_frame_is_declined_by_all_three() {
    let decodes = banded_decodes(b"\xFF\xD8\xFF\xD9", 8);
    assert_eq!(
        (decodes.whole, decodes.at_restarts, decodes.at_rows),
        (None, None, None)
    );
}

/// A one-component frame of fourteen restart intervals whose data an encoder never wrote — the
/// `jpeg_bands` fuzz target's finding (ADR 1495). The whole decoder reads past the damage one
/// way and a band starting at a later interval reads it another, so bands of 16 lines once wrote
/// black where the whole frame is grey from line 96 down. A band the decoder cannot read without
/// recovering from an error is refused, and the frame is the whole decoder's.
#[test]
fn a_damaged_interval_leaves_the_frame_to_the_whole_decoder() {
    let data = include_bytes!("restart/damaged_interval.jpg");
    for lines in [8, 16, 24, 64] {
        let decodes = banded_decodes(data, lines);
        assert!(decodes.whole.is_some(), "the whole decoder reads it");
        assert_eq!(
            decodes.at_restarts, None,
            "bands of {lines} lines would have moved bytes, so they are refused"
        );
    }
}

/// A frame header stating 65535 lines of 65291 samples is past `MAX_SAMPLES`, and the seam
/// refuses it before any decoder sizes a raster for it, as `decode_jpeg` does.
#[test]
fn a_grid_past_the_sample_budget_is_decoded_by_none_of_the_three() {
    let mut data = include_bytes!("restart/damaged_interval.jpg").to_vec();
    let sof = data
        .windows(2)
        .position(|pair| pair == [0xFF, 0xC0])
        .expect("a baseline frame header");
    data[sof + 5..sof + 9].copy_from_slice(&[0xFF, 0xFF, 0xFF, 0x0B]);
    assert_eq!(
        banded_decodes(&data, 16),
        pdf_model::image::BandedDecodes {
            whole: None,
            at_restarts: None,
            at_rows: None,
        }
    );
}

/// A frame of restart intervals whose closing `EOI` was removed — `restart`'s own fixture of a
/// scan the data ends without one, its last interval complete. ISO/IEC 10918-1 section E.2.3
/// ends a scan when its intervals are decoded, so the scan is whole without the marker, and
/// the restart plan's last band reads the tail as the whole decoder does: the frame is cut and
/// is the whole frame (ADR 1513).
#[test]
fn a_scan_of_restart_intervals_without_its_eoi_is_cut_and_is_the_whole_frame() {
    let data = include_bytes!("restart/hv_truncated.jpg");
    assert_ne!(
        data.get(data.len() - 2..),
        Some(&[0xFF, 0xD9][..]),
        "no EOI"
    );
    for lines in [8, 16, 64] {
        let decodes = banded_decodes(data, lines);
        let whole = decodes.whole.as_ref().expect("the whole frame decodes");
        let banded = decodes
            .at_restarts
            .as_ref()
            .unwrap_or_else(|| panic!("cut in bands of {lines} lines"));
        assert!(banded == whole, "bands of {lines} lines moved a byte");
    }
}

/// The same frame with its data ended inside its last interval, at every byte of that interval:
/// a scan whose data does not hold its last MCU is not complete by section E.2.3's count, and
/// the restart plan never answers other than the whole decoder — refused, wherever the decoder
/// ran out of data before a row of the band began (ADR 1513).
#[test]
fn a_scan_the_data_ends_inside_its_last_interval_is_never_cut_differently() {
    let data = include_bytes!("restart/hv_truncated.jpg");
    let last = data
        .windows(2)
        .rposition(|pair| pair[0] == 0xFF && (0xD0..=0xD7).contains(&pair[1]))
        .expect("a restart marker");
    let mut refused = 0;
    for end in last + 2..data.len() {
        let decodes = banded_decodes(&data[..end], 16);
        match (&decodes.at_restarts, &decodes.whole) {
            (None, _) => refused += 1,
            (Some(banded), Some(whole)) => assert!(banded == whole, "ended at {end}"),
            (Some(_), None) => panic!("cut where the whole decoder refuses, ended at {end}"),
        }
    }
    assert!(refused > 0, "some truncation is refused");
}

/// A grey frame of 100 × 107 whose entropy-coded data runs to the end of the stream with no `EOI`
/// — the `jpeg_bands` fuzz target's second finding (ADR 1495). Section F.2.2.3's decoding finds all
/// 182 blocks inside the data, which is what the entropy pass found; the whole decoder stops short
/// of the last MCU row and leaves it at a DC of zero, so a cut at that row wrote 0 where the frame
/// is 128. A scan with no `EOI` after it is the whole decoder's; the same frame ended by its `EOI`
/// is cut, and is the whole frame.
#[test]
fn a_scan_the_data_ends_inside_is_left_to_the_whole_decoder() {
    let data = grey_frame_without_its_eoi();
    for lines in [8, 16, 64] {
        let decodes = banded_decodes(&data, lines);
        assert!(decodes.whole.is_some(), "the whole decoder reads it");
        assert_eq!(decodes.at_rows, None, "no EOI, so no cut at {lines} lines");
    }
    let mut ended = data.clone();
    ended.extend_from_slice(&[0xFF, 0xD9]);
    let decodes = banded_decodes(&ended, 16);
    let banded = decodes.at_rows.expect("ended by its EOI, the frame is cut");
    assert!(
        Some(&banded) == decodes.whole.as_ref(),
        "and the cut frame is the whole frame"
    );
}

/// The same frame decoded whole with and without the `EOI` its data lacks. Section E.2.3 ends the
/// scan on its MCU count and all 182 blocks are in the data, so the two are one frame; `zune-jpeg`
/// 0.5.15 instead fills the last MCU row — lines 104 to 106 — with 128, because its lookahead
/// reached the end of the data while that row's bits were still unconsumed and it stops at the next
/// row on having reached it. `doc/patches/zune-jpeg-scan-complete-without-eoi.patch` makes it stop
/// only once it has consumed past the end, and `doc/questions/Q227` asks whether the tree carries
/// it. **This holds the current bytes by name and waits on that patch: when the fork takes it the
/// first assertion fails, and the guard is deleted for the frame's equality** (ADR 1520).
#[test]
fn a_complete_last_row_without_its_eoi_is_grey_until_the_fork_takes_the_patch() {
    let data = grey_frame_without_its_eoi();
    let mut ended = data.clone();
    ended.extend_from_slice(&[0xFF, 0xD9]);
    let bare = banded_decodes(&data, 16)
        .whole
        .expect("the whole decoder reads it");
    let whole = banded_decodes(&ended, 16)
        .whole
        .expect("and reads it with its EOI");
    let row = 100 * 4;
    assert_eq!(
        (bare.len(), whole.len()),
        (107 * row, 107 * row),
        "100 × 107, RGBA"
    );
    let first_of_the_last_row = 104 * row;
    assert!(
        bare[first_of_the_last_row..]
            .chunks_exact(4)
            .all(|pixel| pixel[..3] == [128, 128, 128]),
        "the patch is in: delete this guard and assert the two decodes equal"
    );
    assert_eq!(
        bare[..first_of_the_last_row],
        whole[..first_of_the_last_row],
        "every row above the last MCU row is the frame's"
    );
    assert_ne!(
        bare[first_of_the_last_row..],
        whole[first_of_the_last_row..],
        "the frame's last row is not grey"
    );
}

/// A grey frame of 100 × 107 whose entropy-coded data runs to the end of the stream with no `EOI`
/// — the `jpeg_bands` fuzz target's second finding (ADR 1495), the codestream in hexadecimal.
#[expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "a fixture written out by hand: a pair of hexadecimal digits that is not one is a \
              broken test, and an offset two past one inside a string of even length cannot wrap"
)]
fn grey_frame_without_its_eoi() -> Vec<u8> {
    let hex = [
    "ffd8ffe000104a46494600010100000100010000ffdb004300100b0c0e0c0a100e0d0e1211101318281a181616183123",
    "251d283a333d3c3933383740485c4e404457453738506d51575f626768673e4d71797064785c656763ffc0000b08006b",
    "006401011100ffc400190001000301010000000000000000000000000102030407ffc400191001010101010100000000",
    "000000000000000102111203ffda0008010100003f00f3f0000000048701000253c4f0e1c388e23480129916917994f9",
    "4f856e55b956c56a0131791a672d6656984f856e19eb2cf519d54168d331b6236ce5acc26e14d658ef2c3719695a8168",
    "d72dfe6e8c46d989b19ea30dc73ed8e94a8168d32e8f9d7462b6cd5ad67bae7fa5736eb2d29502634cd6d8adf1a6b9da",
    "6ed4d6986f4c3759d56a04c5a5699d34ceda4da7dabadb2d699eaa955013169569a5a693ed5ba56e95b508004a7a74e9",
    "d47440000000000000037fd9",
    ]
    .concat();
    (0..hex.len())
        .step_by(2)
        .map(|at| u8::from_str_radix(&hex[at..at + 2], 16).expect("hexadecimal"))
        .collect()
}
