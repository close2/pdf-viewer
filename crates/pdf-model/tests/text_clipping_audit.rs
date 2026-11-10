//! ISO 32000-2 §9.3.6: combined stroke/clip modes and clip activation at ET.

#![expect(clippy::expect_used, reason = "a complete fixture must open")]

use std::fmt::Write as _;

use pdf_render::Command;
use pdf_syntax::Document;

fn interpret(mode: u8) -> pdf_model::Interpretation {
    // Two separate shows distinguish clipping after the first show from clipping at ET.
    // The standard Helvetica face is compiled into this reader, independent of host fonts.
    let content =
        format!("BT /F1 24 Tf 10 10 Td {mode} Tr (A) Tj 40 0 Td (B) Tj ET 0 0 100 100 re f");
    let objects = [
        String::from("<< /Type /Catalog /Pages 2 0 R >>"),
        String::from("<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
        String::from(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>",
        ),
        format!(
            "<< /Length {} >>\nstream\n{content}\nendstream",
            content.len()
        ),
        String::from("<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>"),
    ];
    let mut bytes = String::from("%PDF-1.7\n");
    let mut offsets = vec![0];
    for (number, object) in (1..).zip(objects) {
        offsets.push(bytes.len());
        let _ = writeln!(bytes, "{number} 0 obj\n{object}\nendobj");
    }
    let xref = bytes.len();
    let _ = writeln!(bytes, "xref\n0 {}\n0000000000 65535 f ", offsets.len());
    for offset in offsets.iter().skip(1) {
        let _ = writeln!(bytes, "{offset:010} 00000 n ");
    }
    let _ = write!(
        bytes,
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
        offsets.len()
    );
    let document = Document::open(bytes.into_bytes()).expect("complete PDF 1.7");
    assert!(!document.was_recovered(), "exact xref offsets");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let result = pdf_model::interpret(&document, &page);
    assert!(result.unsupported.is_empty(), "{:?}", result.unsupported);
    result
}

#[test]
fn modes_five_and_six_paint_both_shows_before_the_text_clip_takes_effect() {
    for (mode, expected) in [
        (5, vec!["stroke", "stroke", "fill"]),
        (6, vec!["fill", "stroke", "fill", "stroke", "fill"]),
    ] {
        let result = interpret(mode);
        let commands = result.display_list.commands();
        let kinds: Vec<_> = commands
            .iter()
            .map(|command| match command {
                Command::Fill { .. } => "fill",
                Command::Stroke { .. } => "stroke",
                _ => "unexpected",
            })
            .collect();
        assert_eq!(kinds, expected, "mode {mode} emits every glyph operation");
        let (rectangle, glyphs) = commands.split_last().expect("nonempty commands");
        assert!(
            glyphs.iter().all(|command| command.clip().is_none()),
            "even the second show paints before the text clip becomes active"
        );
        let clip = rectangle.clip().expect("the rectangle after ET is clipped");
        assert!(
            !result
                .display_list
                .clip(clip)
                .expect("clip exists")
                .path
                .is_empty()
        );

        // Removing only the clip bit keeps both glyph operations but leaves the later fill free.
        let control = interpret(mode.saturating_sub(4));
        assert_eq!(control.display_list.commands().len(), commands.len());
        assert!(
            control
                .display_list
                .commands()
                .iter()
                .all(|command| command.clip().is_none())
        );
    }
}
