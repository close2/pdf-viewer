//! `quorra-draw` as a person runs it: a file in, a PNG out, and an exit status that says why not.
//!
//! The page is written here with its offsets counted, so that what is tested is drawing and not
//! §7.5.4's recovery. It is 72 × 36 units with the left half filled red: ISO 32000-2 §8.3.2.3's
//! unit of 1/72 inch makes it 2 × 1 inches, so 144 × 72 pixels at 144 dpi, and §8.6.4.2's
//! `DeviceRGB` `1 0 0` is red, which the device writes as 255, 0, 0 wherever coverage is whole.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "test code: a fixture that cannot be written or a binary that cannot be run is the \
              failure, and the offsets of a four-object file are far below any integer's range"
)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// The binary under test.
const DRAW: &str = env!("CARGO_BIN_EXE_quorra-draw");

/// A one-page PDF 72 × 36 units whose left half is filled with `DeviceRGB` red.
fn half_red_page() -> Vec<u8> {
    let content = b"1 0 0 rg 0 0 36 36 re f\n";
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 72 36] /Contents 4 0 R /Resources << >> >>"
            .to_owned(),
        format!(
            "<< /Length {} >>\nstream\n{}endstream",
            content.len(),
            String::from_utf8_lossy(content)
        ),
    ];
    let mut file = b"%PDF-2.0\n".to_vec();
    let mut offsets = Vec::new();
    for (index, body) in objects.iter().enumerate() {
        offsets.push(file.len());
        file.extend_from_slice(format!("{} 0 obj\n{body}\nendobj\n", index + 1).as_bytes());
    }
    let xref = file.len();
    file.extend_from_slice(
        format!("xref\n0 {}\n0000000000 65535 f\r\n", objects.len() + 1).as_bytes(),
    );
    for offset in offsets {
        file.extend_from_slice(format!("{offset:010} 00000 n\r\n").as_bytes());
    }
    file.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    file
}

/// A directory of the test's own, holding the page.
fn fixture(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("quorra-draw-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("the temporary directory is writable");
    std::fs::write(dir.join("page.pdf"), half_red_page()).expect("the fixture is written");
    dir
}

fn run(arguments: &[&str]) -> Output {
    Command::new(DRAW)
        .args(arguments)
        .output()
        .expect("the binary runs")
}

fn path(dir: &Path, name: &str) -> String {
    dir.join(name).to_string_lossy().into_owned()
}

#[test]
fn a_page_is_drawn_at_its_size_in_dots_and_its_colour() {
    let dir = fixture("draws");
    let out = path(&dir, "page.png");
    let done = run(&[&path(&dir, "page.pdf"), "--dpi", "144", "-o", &out]);
    let stderr = String::from_utf8_lossy(&done.stderr);
    if done.status.code() == Some(1) && stderr.contains("no graphics device") {
        println!("skipped: no adapter on this machine ({stderr})");
        return;
    }
    assert_eq!(done.status.code(), Some(0), "{stderr}");
    assert!(
        stderr.contains("adapter "),
        "the adapter is named: {stderr}"
    );

    let decoder = png::Decoder::new(std::io::BufReader::new(
        std::fs::File::open(&out).expect("the PNG was written"),
    ));
    let mut reader = decoder.read_info().expect("a PNG");
    let mut pixels = vec![0; reader.output_buffer_size().expect("a bounded image")];
    let frame = reader.next_frame(&mut pixels).expect("one frame");
    assert_eq!(
        (frame.width, frame.height),
        (144, 72),
        "§8.3.2.3: 72 × 36 units at 144 dpi"
    );
    assert_eq!(frame.color_type, png::ColorType::Rgba);
    let at = |x: usize, y: usize| {
        let start = (y * 144 + x) * 4;
        <[u8; 4]>::try_from(&pixels[start..start + 4]).expect("four samples")
    };
    assert_eq!(at(36, 36), [255, 0, 0, 255], "inside the fill");
    assert_eq!(
        at(108, 36),
        [255, 255, 255, 255],
        "the page's white beside it"
    );
    std::fs::remove_dir_all(&dir).expect("the temporary directory is removed");
}

#[test]
fn a_page_the_document_does_not_have_is_declined_with_status_four() {
    let dir = fixture("declined");
    let done = run(&[
        &path(&dir, "page.pdf"),
        "--page",
        "2",
        "-o",
        &path(&dir, "page.png"),
    ]);
    assert_eq!(done.status.code(), Some(4));
    assert!(
        String::from_utf8_lossy(&done.stderr).contains("page 2 of a document of 1"),
        "{}",
        String::from_utf8_lossy(&done.stderr)
    );
    assert!(
        !dir.join("page.png").exists(),
        "nothing is written for a declined page"
    );
    std::fs::remove_dir_all(&dir).expect("the temporary directory is removed");
}

#[test]
fn a_file_that_does_not_exist_is_status_one_and_bad_arguments_status_two() {
    let dir = fixture("statuses");
    assert_eq!(
        run(&[&path(&dir, "absent.pdf"), "-o", &path(&dir, "x.png")])
            .status
            .code(),
        Some(1)
    );
    assert_eq!(run(&[&path(&dir, "page.pdf")]).status.code(), Some(2));
    std::fs::remove_dir_all(&dir).expect("the temporary directory is removed");
}
