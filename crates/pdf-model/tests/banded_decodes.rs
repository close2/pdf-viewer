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
/// — the `jpeg_bands` fuzz target's second finding (ADR 1495). The row plan re-codes its last band
/// as a codestream of its own, ended by an `EOI` the data does not have, so a scan with no `EOI`
/// after it is left to the whole decoder (`image::cut`, ADR 1513); the same frame ended by its
/// `EOI` is cut, and is the whole frame.
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

/// The same frame decoded whole with and without the `EOI` its data lacks. ITU-T T.81 section
/// E.2.3 ends the scan on its MCU count and all 182 blocks are in the data, so the two are one
/// frame, its last MCU row — lines 104 to 106 — included. A decoder that stops at the row after its
/// lookahead *reached* the end of the data, rather than after it consumed past it, fills that row
/// with 128 instead; the fork the manifest pins carries
/// `doc/patches/zune-jpeg-scan-complete-without-eoi.patch`, which is that difference (ADRs 1520,
/// 1730).
#[test]
fn a_complete_last_row_without_its_eoi_is_decoded_as_the_frame() {
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
    assert_eq!(bare.len(), 107 * row, "100 × 107, RGBA");
    let first_of_the_last_row = 104 * row;
    assert!(
        whole[first_of_the_last_row..]
            .chunks_exact(4)
            .any(|pixel| pixel[..3] != [128, 128, 128]),
        "the frame's last MCU row is not grey, so a grey one would be seen below"
    );
    assert!(bare == whole, "the scan without its EOI is the whole frame");
}

/// A frame whose DC prediction leaves `i32` once it is multiplied by the quantiser's DC entry is
/// decoded whole, and the decode returns. A DC category bounds each difference the scan codes
/// (ITU-T T.81 Table F.1), not the sum of the differences a hostile frame accumulates, so a
/// decoder meets such a sum and must not abort on it. A prediction updated by a wrapping add and
/// then multiplied unchecked panics wherever overflow checks are on (the dev, test and fuzz
/// profiles); the fork the manifest pins carries
/// `doc/patches/zune-jpeg-dc-prediction-overflow.patch`, which wraps the multiply as the add is
/// wrapped (ADRs 1589, 1730). The frame header states 509 lines of 2122 samples, so the decode is
/// that many RGBA pixels.
#[test]
fn a_dc_prediction_past_i32_is_decoded_rather_than_aborting() {
    let decodes = banded_decodes(&dc_prediction_past_i32(), 16);
    let whole = decodes.whole.expect("the whole decoder reads it");
    assert_eq!(whole.len(), 2122 * 509 * 4, "2122 × 509, RGBA");
}

/// A grey frame of 100 × 107 whose entropy-coded data runs to the end of the stream with no `EOI`
/// — the `jpeg_bands` fuzz target's second finding (ADR 1495), the codestream in hexadecimal.
fn grey_frame_without_its_eoi() -> Vec<u8> {
    from_hex(&[
        "ffd8ffe000104a46494600010100000100010000ffdb004300100b0c0e0c0a100e0d0e1211101318281a181616183123",
        "251d283a333d3c3933383740485c4e404457453738506d51575f626768673e4d71797064785c656763ffc0000b08006b",
        "006401011100ffc400190001000301010000000000000000000000000102030407ffc400191001010101010100000000",
        "000000000000000102111203ffda0008010100003f00f3f0000000048701000253c4f0e1c388e23480129916917994f9",
        "4f856e55b956c56a0131791a672d6656984f856e19eb2cf519d54168d331b6236ce5acc26e14d658ef2c3719695a8168",
        "d72dfe6e8c46d989b19ea30dc73ed8e94a8168d32e8f9d7462b6cd5ad67bae7fa5736eb2d29502634cd6d8adf1a6b9da",
        "6ed4d6986f4c3759d56a04c5a5699d34ceda4da7dabadb2d699eaa955013169569a5a693ed5ba56e95b508004a7a74e9",
        "d47440000000000000037fd9",
    ])
}

/// A one-component frame of 2122 × 509 whose DC differences accumulate past `i32` once
/// dequantised — the `jpeg_bands` fuzz target's third finding, the codestream of
/// `doc/patches/zune-jpeg-dc-prediction-overflow.patch` in hexadecimal.
fn dc_prediction_past_i32() -> Vec<u8> {
    from_hex(&[
        "ffd8ffe000104a46494600010100000100010000ffdb004300100b0c0e0c0a100e0d0e1211101318281a181616183123",
        "251d283a333d3c3933383740485c4e404457453738506d51575f626768673e4d71797064785c656763ffc0000b0801fd",
        "084a01011100ffc4001a00010003010101000000000000000000000f00000000010405030207ffc4001f100100020202",
        "0301010000000000000000001361111403120102044131ffda0008010100003f00f9f800000000000000000000000000",
        "000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
        "000000000000024c183060c183060c183060c183060c183060c183060c183060c183060c183060c183060c183060c183",
        "060c183060c183060c183060c183060c183060c183060c183060c183060c183060c19d4ea753a9d4ea753a9d4ea753a9",
        "d4ea753a9d4ea753a9d4ea3a759d4ea753a9d4ea753a9d4ea753a9d4ea753a9d4ea7533a9d4ea753a9d4ea753a9d4ea7",
        "53a9d4ea753a9d4ea753a9d4ea753a9d4ea753a9d4ea753a9d4ea753a9d4ea753a9d4e7553d4a9a7ea3a9d4ea753a9d4",
        "ea753a9d4ea753a9d4ea753a9d4ea753a9d4ea753a9d4ea753a9d4ea753a9d4ea753a9d4ea753a1ad4866b51ad46b51a",
        "d46b51ad46b51ad46b51ad46b51ad46b51ad46b51ad46b51ad46b51affffffffffffffffffffffffffffffffffffffff",
        "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffd46b51ad46b5",
        "1ad46b51ad46b51ad46b51ad46b51ad46b51ad46b51ad46b51ad46b51ad46b51ad46b51ad46b51adffb51a2b6b51ad46",
        "b51ad46b510a0a0a0ac18308f3e1cfdbc38727850e7f0cbfa7c7f593f4f8feb27e8f1fd67731a7bb9797904f875f4581",
        "818181818181818181818181818181818181818181818181818181818181818181818181818181818181818181818181",
        "818181818181818181818181818181818181818181818181818181818181818181818181818181818181818181818181",
        "81998181818181818181819e2687cff8d5f97f1aff002fe35be75fe25af474f090000000000000000000000000000000",
        "000000000000000000000a00000000000000000000000a0a0a0a0a0a0a00",
    ])
}

/// The bytes a codestream written out in hexadecimal digits stands for.
#[expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "a fixture written out by hand: a pair of hexadecimal digits that is not one is a \
              broken test, and an offset two past one inside a string of even length cannot wrap"
)]
fn from_hex(lines: &[&str]) -> Vec<u8> {
    let hex = lines.concat();
    (0..hex.len())
        .step_by(2)
        .map(|at| u8::from_str_radix(&hex[at..at + 2], 16).expect("hexadecimal"))
        .collect()
}
