//! A value set in a machine face whose outlines are a `CFF ` table, saved, and drawn from the file
//! alone (ADR 1438) — `machine_face_written.rs`'s claim for Table 124's `OpenType` row.
//!
//! Its own binary because it turns the machine's fonts off for the whole process and hands the
//! layout one face through `pdf_font::provider`'s port: which face `installed_covering` walks to
//! first is the catalogue's order, and a `CFF ` face is what this test is about. The port is the
//! one a confined worker is handed faces through, so the layout under test is the shipped one.
//!
//! The face is the machine's own: an Arabic `CFF ` face where `fc-list` names one, since the value
//! ADR 1425 saved is Arabic, and otherwise one covering a value in Duployan shorthand — a script no
//! compiled-in face draws, so the layout has to ask the machine, which Latin, Greek and Cyrillic
//! values do not (the compiled-in Helvetica draws all three). The write under test is the
//! program's, not the script's. Where the machine offers neither, the test says so and stops
//! (ADR 1154).

#![expect(
    clippy::expect_used,
    reason = "a test's failure is its purpose, and these helpers run outside #[test] bodies \
              where `allow-panic-in-tests` does not reach"
)]

use std::fmt::Write as _;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use pdf_model::view::{Entered, ViewState};
use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::{Document, Object};
use render_cpu::CpuRasterizer;

/// A raster budget no fixture here comes near.
const GENEROUS: u64 = 1 << 30;

/// The face the port hands out, once chosen.
static FACE: OnceLock<Vec<u8>> = OnceLock::new();

/// Whether the port still answers: switched off once the file is saved.
static OFFERING: AtomicBool = AtomicBool::new(true);

/// How many times the port was asked after it stopped answering, which a file drawing from its
/// own program never does.
static ASKED_AFTER: AtomicUsize = AtomicUsize::new(0);

/// The port's answer: the chosen face for any request naming characters, while offering.
fn ask(request: &[u8]) -> Option<(Vec<u8>, Vec<u8>)> {
    let (_, wanted, _) = pdf_font::provider::decode_request(request)?;
    if wanted.is_empty() {
        return None;
    }
    if !OFFERING.load(Ordering::Acquire) {
        ASKED_AFTER.fetch_add(1, Ordering::AcqRel);
        return None;
    }
    Some((
        pdf_font::provider::encode_identity("machine CFF face"),
        FACE.get()?.clone(),
    ))
}

/// One page with one text field, its `/DA` naming a standard font that has neither script.
fn one_field() -> Vec<u8> {
    let body = "1 0 obj\n<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [5 0 R] \
                /DR << /Font << /Helv 6 0 R >> >> >> >>\nendobj\n\
                2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
                3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] \
                /Resources << >> /Contents 4 0 R /Annots [5 0 R] >>\nendobj\n\
                4 0 obj\n<< /Length 0 >>\nstream\n\nendstream\nendobj\n\
                5 0 obj\n<< /Type /Annot /Subtype /Widget /Rect [20 40 180 70] /F 4 /FT /Tx \
                /T (field) /P 3 0 R /DA (/HeBo 12 Tf 0 g) >>\nendobj\n\
                6 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica \
                /Encoding /WinAnsiEncoding >>\nendobj\n";
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(out.len());
        out.push_str(object);
    }
    let xref_at = out.len();
    let size = offsets.len().saturating_add(1);
    let _ = writeln!(out, "xref\n0 {size}");
    out.push_str("0000000000 65535 f \n");
    for offset in &offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

/// Page one rasterised against a viewer state.
fn draw(document: &Document, view: &ViewState) -> pdf_render::Raster {
    let page = pdf_model::Pages::new(document).get(0).expect("page one");
    let list = pdf_model::content::interpret_with(document, &page, view).display_list;
    let target = TargetSpec::for_page(&list, 2.0, GENEROUS).expect("valid target");
    CpuRasterizer::new()
        .with_medium(pdf_render::Medium::NONE)
        .rasterize(&list, target)
        .expect("supported")
}

/// A `CFF ` face `fc-list` names under `pattern` that covers every character `text` displays.
fn cff_face_covering(pattern: &str, text: &str) -> Option<Vec<u8>> {
    let listed = std::process::Command::new("fc-list")
        .arg(pattern)
        .arg("file")
        .output()
        .ok()?;
    let paragraphs = pdf_font::shaping::Paragraphs::new(text);
    let wanted: Vec<char> = pdf_font::shaping::displayed_characters(text, paragraphs.as_ref())?
        .keys()
        .copied()
        .collect();
    let mut paths: Vec<String> = String::from_utf8_lossy(&listed.stdout)
        .lines()
        .filter_map(|line| line.split(':').next().map(str::to_owned))
        .collect();
    // The catalogue's order is the machine's; sorting makes the choice the same on every run.
    paths.sort();
    paths
        .into_iter()
        .filter_map(|path| std::fs::read(path).ok())
        .filter(|program| program.starts_with(b"OTTO"))
        .find(|program| {
            pdf_font::substitute::face_covers(&std::sync::Arc::from(program.as_slice()), &wanted)
        })
}

/// The one descendant font of the one `Type0` font the saved widget's appearance uses.
fn saved_descendant(saved: &Document) -> pdf_syntax::Dictionary {
    let page = pdf_model::Pages::new(saved).get(0).expect("page one");
    let annots = saved.get_key(&page.dict, "Annots");
    let widget = annots
        .as_array()
        .and_then(|annots| annots.first())
        .map(|widget| saved.resolve(widget))
        .expect("the widget");
    let normal = saved.get_key(widget.as_dict().expect("a dictionary"), "AP");
    let normal = saved.get_key(normal.as_dict().expect("an /AP"), "N");
    let stream = normal.as_stream().expect("an appearance stream");
    let resources = saved.get_key(&stream.dict, "Resources");
    let fonts = saved.get_key(resources.as_dict().expect("resources"), "Font");
    let fonts = fonts.as_dict().expect("fonts");
    let font = fonts
        .iter()
        .map(|(_, font)| saved.resolve(font))
        .find(|font| {
            font.as_dict()
                .and_then(|font| font.get("Subtype"))
                .and_then(Object::as_name)
                .is_some_and(|name| name.as_bytes() == b"Type0")
        })
        .expect("the machine face's Type0 font");
    let descendants = saved.get_key(font.as_dict().expect("a font"), "DescendantFonts");
    let descendant = descendants
        .as_array()
        .and_then(|array| array.first())
        .map(|descendant| saved.resolve(descendant))
        .expect("a descendant");
    descendant.as_dict().expect("a CIDFont").clone()
}

/// A value saved in a `CFF ` machine face is written under `/FontFile3` `/OpenType` and a
/// `CIDFontType0`, and draws from the file alone exactly as the viewer showed it.
#[test]
fn a_value_saved_in_a_cff_machine_face_draws_from_the_file_alone() {
    let arabic = "\u{633}\u{644}\u{627}\u{645} \u{633}\u{644}\u{627}\u{645}";
    let duployan = "\u{1bc02}\u{1bc03}\u{1bc04} \u{1bc00}\u{1bc01}";
    let Some((typed, face)) = cff_face_covering(":fontformat=CFF:lang=ar", arabic)
        .map(|face| (arabic, face))
        .or_else(|| cff_face_covering(":fontformat=CFF", duployan).map(|face| (duployan, face)))
    else {
        println!(
            "skipped: no CFF face on this machine covers the Arabic or the Duployan value's \
             shaped characters"
        );
        return;
    };
    FACE.set(face).expect("set once");
    pdf_font::substitute::no_machine_fonts();
    pdf_font::provider::faces_come_from(ask);

    let document = Document::open(one_field()).expect("the fixture is a valid PDF");
    let mut view = ViewState::of(&document);
    assert_eq!(
        view.set_field(&document, "field", &Entered::Text(typed.to_owned())),
        1
    );
    let on_screen = draw(&document, &view);
    assert!(
        on_screen.data.iter().any(|byte| *byte != 0),
        "the value is drawn on the screen"
    );
    let written = view.save(&document).expect("the fixture can be written");
    assert!(
        written.unconstructed.is_empty(),
        "{:?}",
        written.unconstructed
    );
    OFFERING.store(false, Ordering::Release);

    let saved_path =
        std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("machine_cff_face_written.pdf");
    std::fs::write(&saved_path, &written.bytes).expect("the target's scratch directory");
    // qpdf absent is a machine without the witness, and the file's own re-reading below still binds.
    if let Ok(output) = std::process::Command::new("qpdf")
        .arg("--check")
        .arg(&saved_path)
        .output()
    {
        let said = String::from_utf8_lossy(&output.stdout);
        assert!(
            output.status.success() && said.contains("No syntax or stream encoding errors"),
            "qpdf --check: {said}{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let saved = Document::open(written.bytes).expect("the save re-opens");
    let descendant = saved_descendant(&saved);
    let subtype = |dict: &pdf_syntax::Dictionary| {
        dict.get("Subtype")
            .and_then(Object::as_name)
            .map(|name| name.as_bytes().to_vec())
    };
    assert_eq!(subtype(&descendant), Some(b"CIDFontType0".to_vec()));
    assert!(
        descendant.get("CIDToGIDMap").is_none(),
        "Table 117: Type 2 only"
    );
    let descriptor = saved.get_key(&descendant, "FontDescriptor");
    let descriptor = descriptor.as_dict().expect("a descriptor");
    assert!(descriptor.get("FontFile2").is_none());
    let file = saved.get_key(descriptor, "FontFile3");
    let file = file.as_stream().expect("a font file stream");
    assert_eq!(subtype(&file.dict), Some(b"OpenType".to_vec()));
    assert!(file.dict.get("Length1").is_none());

    let from_the_file = draw(&saved, &ViewState::of(&saved));
    assert_eq!(
        ASKED_AFTER.load(Ordering::Acquire),
        0,
        "the file drew without asking for a face"
    );
    assert_eq!(
        (from_the_file.width, from_the_file.height),
        (on_screen.width, on_screen.height)
    );
    assert!(
        from_the_file.data == on_screen.data,
        "the page drawn from the file is the page the viewer showed"
    );
}
