//! `with_cids_given` on hand-built CID-keyed programs: the copies, the tables restated, the
//! bounds, and the read-back refusing a mis-written `FDSelect`.

use skrifa::GlyphId;
use skrifa::outline::OutlinePen;

use super::super::{Layout, OP_FD_SELECT, advances, dict_ints, draw, int5, open};
use super::{
    CidToGive, MOST_CID_BYTES_GIVEN, OP_CID_COUNT, fd_select_written, given_with, with_cids_given,
};
use crate::cff::CffError;

/// `500 hlineto 500 vlineto -500 hlineto return`: the square glyph 0 draws, as FD 1's local
/// subroutine 0.
const SQUARE_SUBR: &[u8] = &[248, 136, 6, 248, 136, 7, 252, 136, 6, 11];

/// `100 100 100 rmoveto -107 callsubr endchar`: width 100 past FD 1's `nominalWidthX` of 200,
/// then the square through local subroutine 0 under a small INDEX's bias.
const NOTDEF: &[u8] = &[239, 239, 239, 21, 32, 10, 14];

/// An INDEX of `items` with offset size four (Adobe Technical Note #5176, section 5).
#[expect(
    clippy::cast_possible_truncation,
    clippy::arithmetic_side_effects,
    reason = "a fixture builder: every INDEX here is far below 2^16 items and 2^32 bytes"
)]
fn index(items: &[&[u8]]) -> Vec<u8> {
    let mut out = (items.len() as u16).to_be_bytes().to_vec();
    if items.is_empty() {
        return out;
    }
    out.push(4);
    let mut offset = 1u32;
    out.extend_from_slice(&offset.to_be_bytes());
    for item in items {
        offset += item.len() as u32;
        out.extend_from_slice(&offset.to_be_bytes());
    }
    for item in items {
        out.extend_from_slice(item);
    }
    out
}

/// A three-glyph CID-keyed program: glyph 0 (CID 0) is `notdef` under FD 1, whose Private DICT
/// states `nominalWidthX` 200 and the local subroutine [`SQUARE_SUBR`]; glyph 1 (CID 5) under
/// FD 0, whose Private DICT is empty; glyph 2 (CID 6) under FD 1. The charset is format 2, the
/// `FDSelect` of `fd_select_format`, 0 or 3.
#[expect(
    clippy::cast_possible_wrap,
    clippy::arithmetic_side_effects,
    reason = "a fixture builder over a font of three glyphs: every quantity is far below 2^31, \
              and a wrong table is what the assertions exist to catch"
)]
fn cid_keyed(fd_select_format: u8, notdef: &[u8]) -> Vec<u8> {
    let header = [1u8, 0, 4, 4];
    let names = index(&[b"F"]);
    let strings = index(&[b"Adobe", b"Identity"]);
    let gsubrs = index(&[]);
    let charset = [2u8, 0, 5, 0, 1];
    let fd_select: &[u8] = if fd_select_format == 0 {
        &[0, 1, 0, 1]
    } else {
        &[3, 0, 3, 0, 0, 1, 0, 1, 0, 0, 2, 1, 0, 3]
    };
    let charstrings = index(&[notdef, &[14], &[14]]);
    let top_len = 43;
    let top_index_len = 3 + 2 * 4 + top_len;
    let strings_at = header.len() + names.len() + top_index_len;
    let charset_at = strings_at + strings.len() + gsubrs.len();
    let fd_select_at = charset_at + charset.len();
    let charstrings_at = fd_select_at + fd_select.len();
    let private_at = charstrings_at + charstrings.len();
    let mut private = Vec::new();
    private.extend_from_slice(&int5(200));
    private.push(21);
    private.extend_from_slice(&int5(12));
    private.push(19);
    let subrs = index(&[SQUARE_SUBR]);
    let fd_array_at = private_at + private.len() + subrs.len();
    let mut top = Vec::new();
    for operand in [391, 392, 0] {
        top.extend_from_slice(&int5(operand));
    }
    top.extend_from_slice(&[12, 30]);
    top.extend_from_slice(&int5(charset_at as i64));
    top.push(15);
    top.extend_from_slice(&int5(fd_select_at as i64));
    top.extend_from_slice(&[12, 37]);
    top.extend_from_slice(&int5(charstrings_at as i64));
    top.push(17);
    top.extend_from_slice(&int5(fd_array_at as i64));
    top.extend_from_slice(&[12, 36]);
    assert_eq!(top.len(), top_len);
    let font_dict = |size: usize| {
        let mut dict = int5(size as i64).to_vec();
        dict.extend_from_slice(&int5(private_at as i64));
        dict.push(18);
        dict
    };
    let (fd0, fd1) = (font_dict(0), font_dict(private.len()));

    let mut out = Vec::new();
    out.extend_from_slice(&header);
    out.extend_from_slice(&names);
    out.extend_from_slice(&index(&[&top]));
    out.extend_from_slice(&strings);
    out.extend_from_slice(&gsubrs);
    out.extend_from_slice(&charset);
    out.extend_from_slice(fd_select);
    out.extend_from_slice(&charstrings);
    out.extend_from_slice(&private);
    out.extend_from_slice(&subrs);
    out.extend_from_slice(&index(&[&fd0, &fd1]));
    assert_eq!(out.len(), fd_array_at + index(&[&fd0, &fd1]).len());
    out
}

/// Every pen call, rounded to hundredths, so two outlines compare as lists.
#[derive(Default)]
struct Recorded(Vec<String>);

impl OutlinePen for Recorded {
    fn move_to(&mut self, x: f32, y: f32) {
        self.0.push(format!("M {x:.2} {y:.2}"));
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.0.push(format!("L {x:.2} {y:.2}"));
    }
    fn quad_to(&mut self, cx: f32, cy: f32, x: f32, y: f32) {
        self.0.push(format!("Q {cx:.2} {cy:.2} {x:.2} {y:.2}"));
    }
    fn curve_to(&mut self, cx0: f32, cy0: f32, cx1: f32, cy1: f32, x: f32, y: f32) {
        self.0.push(format!(
            "C {cx0:.2} {cy0:.2} {cx1:.2} {cy1:.2} {x:.2} {y:.2}"
        ));
    }
    fn close(&mut self) {
        self.0.push("Z".to_owned());
    }
}

fn outline(data: &[u8], glyph: u16) -> Vec<String> {
    let mut pen = Recorded::default();
    draw(data, glyph, &mut pen).expect("the glyph draws");
    pen.0
}

/// The CID each glyph of `data` has, and the Font DICT `FDSelect` gives it.
fn cids_and_font_dicts(data: &[u8]) -> Vec<(u16, u16)> {
    let font = open(data).expect("the program opens");
    let charset = font.charset().expect("a charset");
    (0..font.num_glyphs())
        .map(|glyph| {
            let id = GlyphId::from(glyph);
            (
                charset.string_id(id).expect("a CID").to_u16(),
                font.subfont_index(id).expect("a Font DICT"),
            )
        })
        .collect()
}

fn give(cid: u16, advance: i64) -> CidToGive {
    CidToGive { cid, advance }
}

#[test]
fn each_cid_is_given_a_copy_of_glyph_0_under_its_font_dict_in_either_fdselect_format() {
    for format in [0u8, 3] {
        let source = cid_keyed(format, NOTDEF);
        let (rebuilt, added) =
            with_cids_given(&source, &[give(9, 500), give(7, 300)]).expect("written");
        assert_eq!(added, [3, 4]);
        assert_eq!(
            cids_and_font_dicts(&rebuilt),
            [(0, 1), (5, 0), (6, 1), (9, 1), (7, 1)],
            "format {format}: the glyphs held keep their CID and Font DICT"
        );
        // The copies draw glyph 0's square, which only FD 1's local subroutine draws.
        let square = outline(&source, 0);
        assert!(!square.is_empty());
        assert_eq!(outline(&rebuilt, 3), square);
        assert_eq!(outline(&rebuilt, 4), square);
        let widths = advances(&rebuilt, &[0, 3, 4]).expect("advances");
        let notdef = widths[0].expect("glyph 0's advance");
        assert!((widths[1].expect("a copy's advance") - notdef * 5.0 / 3.0).abs() < 1e-3);
        assert!((widths[2].expect("a copy's advance") - notdef).abs() < 1e-3);
        // The FDSelect keeps the format the program stated it in.
        let layout = Layout::read(&rebuilt).expect("a Top DICT");
        let (operands, _) = layout
            .entries
            .iter()
            .find(|(_, op)| *op == OP_FD_SELECT)
            .expect("an FDSelect entry");
        let at = usize::try_from(dict_ints(operands)[0]).expect("an offset");
        assert_eq!(rebuilt[at], format);
    }
}

#[test]
fn cid_count_is_raised_only_where_a_cid_passes_it() {
    let source = cid_keyed(3, NOTDEF);
    let cid_count = |data: &[u8]| {
        Layout::read(data)
            .expect("a Top DICT")
            .entries
            .iter()
            .find(|(_, op)| *op == OP_CID_COUNT)
            .map(|(operands, _)| dict_ints(operands))
    };
    let (below, _) = with_cids_given(&source, &[give(8719, 300)]).expect("written");
    assert_eq!(
        cid_count(&below),
        None,
        "8720, the default, already covers CID 8719"
    );
    let (past, _) = with_cids_given(&source, &[give(9000, 300)]).expect("written");
    assert_eq!(cid_count(&past), Some(vec![9001]));
}

#[test]
fn cid_0_a_held_cid_and_a_repeated_cid_are_refused() {
    let source = cid_keyed(0, NOTDEF);
    for asked in [
        vec![give(0, 300)],
        vec![give(5, 300)],
        vec![give(9, 300), give(9, 500)],
    ] {
        assert!(
            matches!(
                with_cids_given(&source, &asked),
                Err(CffError::Malformed { .. })
            ),
            "{asked:?}"
        );
    }
}

#[test]
fn the_glyph_count_and_the_appended_bytes_are_bounded_before_the_work() {
    let source = cid_keyed(3, NOTDEF);
    // Three glyphs held, so 65 532 more is the INDEX's last count and one more passes it.
    let asked: Vec<CidToGive> = (1..=65_533).map(|cid| give(cid, 300)).collect();
    assert!(matches!(
        with_cids_given(&source, &asked),
        Err(CffError::Malformed { detail }) if detail.contains("INDEX counts")
    ));

    // A `.notdef` of about 100 kB: its width, a moveto, fifty thousand `1 hlineto`, endchar.
    let mut long = vec![239, 239, 239, 21];
    for _ in 0..50_000 {
        long.extend_from_slice(&[140, 6]);
    }
    long.push(14);
    let source = cid_keyed(3, &long);
    // Each copy restates the one-byte width operand in the three-byte form.
    let copy = long.len() + 2;
    let fits = MOST_CID_BYTES_GIVEN / copy;
    let asked = |n: usize| -> Vec<CidToGive> {
        (0..n)
            .map(|at| give(u16::try_from(at + 10).expect("small"), 300))
            .collect()
    };
    assert!(with_cids_given(&source, &asked(fits)).is_ok());
    assert!(matches!(
        with_cids_given(&source, &asked(fits + 1)),
        Err(CffError::Malformed { detail }) if detail.contains("bound")
    ));
}

/// The plant the read-back exists for: an `FDSelect` in format 3 whose sentinel is one glyph
/// short, so that `read-fonts` finds no Font DICT for the last copy.
#[test]
fn an_fdselect_whose_sentinel_is_one_glyph_short_is_refused_by_the_read_back() {
    fn one_short(format: u8, font_dicts: &[u8]) -> Option<Vec<u8>> {
        let mut table = fd_select_written(format, font_dicts)?;
        let at = table.len() - 2;
        let sentinel = u16::from_be_bytes([table[at], table[at + 1]]) - 1;
        table[at..].copy_from_slice(&sentinel.to_be_bytes());
        Some(table)
    }
    let source = cid_keyed(3, NOTDEF);
    assert!(given_with(&source, &[give(9, 500)], fd_select_written).is_ok());
    assert!(matches!(
        given_with(&source, &[give(9, 500)], one_short),
        Err(CffError::Malformed { detail }) if detail.contains("read back")
    ));
}

#[test]
fn the_name_keyed_writer_still_refuses_a_cid_keyed_program() {
    let source = cid_keyed(3, NOTDEF);
    assert!(matches!(
        crate::cff::with_empty_glyphs(&source, &[300]),
        Err(CffError::Malformed { detail }) if detail.contains("CID-keyed")
    ));
}
