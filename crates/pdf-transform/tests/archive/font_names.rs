//! A font name that is not UTF-8, restated where the conversion embeds the program it names, and a
//! colourant name that is not, kept refused (ADR 2846).
//!
//! The expected values come from ISO 19005-2 section 6.1.8 (font names, colourant names and
//! structure type names "shall be valid UTF-8 character sequences"), from ISO 32000-2 Table 117 (a
//! `CIDFontType0`'s `/BaseFont` is its program's own name) and Table 120 (`/FontName` is the same
//! as that `/BaseFont`), and from Table 119, whose Type 0 `/BaseFont` is the descendant's name, a
//! hyphen and the `/Encoding` entry's `CMap` name. The bytes `BA DA CC E5` are a two-character face
//! name in GBK, and no UTF-8 sequence.
use std::sync::Arc;

use super::{Conforming, Decision, Level, Target, a_supplied_font, conversion, convert_with_plan};
use super::{decision, font_resource, holds};
use pdf_archive::Outcome;
use pdf_syntax::{Document, Limits};
use pdf_transform::archive::{Authorisations, Because, CjkPack, Loss, PackFiles, Rewrite};

/// The row both tests are about.
const ROW: &str = "file-structure/bound-names-are-valid-utf8";

/// `cjk_pack`'s Japanese fixture with the three font names written in GBK's bytes.
fn a_font_named_in_gbk() -> Vec<u8> {
    Conforming {
        resources: "/Font << /F1 6 0 R >> /ColorSpace << /CS0 [/CalGray << /WhitePoint \
                    [0.9505 1.0 1.089] >>] >>"
            .to_owned(),
        contents: Some((
            String::new(),
            b"/CS0 cs 0 sc BT /F1 12 Tf 10 100 Td <30423044> Tj ET".to_vec(),
        )),
        objects: vec![
            "<< /Type /Font /Subtype /Type0 /BaseFont /#ba#da#cc#e5-UniJIS-UCS2-H \
             /Encoding /UniJIS-UCS2-H /DescendantFonts [8 0 R] >>"
                .to_owned(),
            "<< /Type /FontDescriptor /FontName /#ba#da#cc#e5 /Flags 4 \
             /FontBBox [-92 -250 1010 922] /ItalicAngle 0 /Ascent 752 /Descent -221 \
             /CapHeight 737 /StemV 114 >>"
                .to_owned(),
            "<< /Type /Font /Subtype /CIDFontType2 /BaseFont /#ba#da#cc#e5 \
             /CIDSystemInfo << /Registry (Adobe) /Ordering (Japan1) /Supplement 2 >> \
             /FontDescriptor 7 0 R /DW 1000 >>"
                .to_owned(),
        ],
        ..Conforming::part_two()
    }
    .build()
}

/// A reader of the pack directory `QUORRA_CJK_PACK` names.
#[derive(Debug)]
struct Directory(std::path::PathBuf);

impl PackFiles for Directory {
    fn read(&self, file: &str) -> Option<Vec<u8>> {
        std::fs::read(self.0.join(file)).ok()
    }
}

#[test]
#[ignore = "needs the CJK pack, a second download that is not part of a checkout: \
            QUORRA_CJK_PACK=<dir> names it (doc/cjk-pack.md)"]
fn a_font_name_in_gbk_is_restated_as_the_name_of_the_program_embedded_for_it() {
    let source = a_font_named_in_gbk();
    assert!(
        matches!(
            holds(&source, Target::Two(Level::B))
                .judgements
                .iter()
                .find(|judgement| judgement.id == ROW)
                .map(|judgement| &judgement.outcome),
            Some(Outcome::Failed { total: 3, .. })
        ),
        "the source fails section 6.1.8 at the Type 0 font, its descendant and the descriptor"
    );
    let directory = std::env::var_os("QUORRA_CJK_PACK")
        .expect("QUORRA_CJK_PACK names the directory the pack was unpacked into");
    let mut plan = a_supplied_font(Target::Two(Level::B), "NotThisFont", &[]);
    plan.cjk_pack = Some(CjkPack::new(Arc::new(Directory(directory.into()))));
    let mut authorised = Authorisations::default();
    authorised.authorise(Loss::MachineFace);
    plan.authorised = authorised;
    let (report, output) = convert_with_plan(&source, &plan);
    assert!(
        conversion(&report).settled.is_empty(),
        "the first pass answers it, not ADR 2534's second"
    );
    assert!(
        matches!(
            decision(&report, ROW),
            Decision::Authorised {
                rewrite: Rewrite::SubstituteFontProgram,
                ..
            }
        ),
        "the names go with the embedding, at its losses: {:?}",
        decision(&report, ROW)
    );
    let output = output.expect("it converts");
    assert!(
        holds(&output, Target::Two(Level::B))
            .judgements
            .iter()
            .any(|judgement| judgement.id == ROW && judgement.outcome == Outcome::Met),
        "the output meets section 6.1.8"
    );
    let held = Document::open_with_limits(output, Limits::DEFAULT).expect("it opens");
    let name = |dict: &pdf_syntax::object::Dictionary, key: &str| {
        held.get_key(dict, key)
            .as_name()
            .map(|name| String::from_utf8_lossy(name.as_bytes()).into_owned())
            .expect("a name")
    };
    let font = font_resource(&held);
    let descendant = held
        .resolve(
            held.get_key(&font, "DescendantFonts")
                .as_array()
                .and_then(|array| array.first())
                .expect("§9.7.6.2's one-element array"),
        )
        .as_dict()
        .cloned()
        .expect("the descendant CIDFont");
    let descriptor = held
        .get_key(&descendant, "FontDescriptor")
        .as_dict()
        .cloned()
        .expect("the descriptor");
    let program_name = name(&descendant, "BaseFont");
    assert!(
        program_name.ends_with("+QuorraCJK-Japan1"),
        "Table 117: the program's own name, under §9.9.2's tag: {program_name}"
    );
    assert_eq!(name(&descendant, "Subtype"), "CIDFontType0");
    assert_eq!(name(&descriptor, "FontName"), program_name, "Table 120");
    assert_eq!(
        name(&font, "BaseFont"),
        format!("{program_name}-UniJIS-UCS2-H"),
        "Table 119: the descendant's name, a hyphen and the Encoding's CMap name"
    );
}

#[test]
fn a_colourant_name_in_gbk_keeps_the_refusal() {
    // A colourant name is what every colour space naming the same ink finds it by (§8.6.6.4), so
    // no conversion names it anew, and section 6.1.8 keeps its refusal — beside a font the
    // conversion does embed a face for, so that the embedding's names are not what decides it.
    let source = Conforming {
        resources: "/Font << /F1 6 0 R >> /ColorSpace << /CS0 [/Separation /#ba#da#cc#e5 \
                    /DeviceGray << /FunctionType 2 /Domain [0 1] /C0 [1] /C1 [0] /N 1 >>] >>"
            .to_owned(),
        contents: Some((
            String::new(),
            b"/CS0 cs 1 sc 0 0 10 10 re f BT /F1 12 Tf 10 100 Td (a) Tj ET".to_vec(),
        )),
        objects: vec!["<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_owned()],
        ..Conforming::part_two()
    }
    .build();
    let mut plan = a_supplied_font(Target::Two(Level::B), "NotThisFont", &[]);
    let mut authorised = Authorisations::default();
    authorised.authorise(Loss::MachineFace);
    authorised.authorise(Loss::PageAsImage);
    plan.authorised = authorised;
    let (report, output) = convert_with_plan(&source, &plan);
    assert!(output.is_none(), "the file is refused");
    assert!(
        matches!(
            decision(&report, ROW),
            Decision::Refused(Because::TheFence(_))
        ),
        "{:?}",
        decision(&report, ROW)
    );
}
