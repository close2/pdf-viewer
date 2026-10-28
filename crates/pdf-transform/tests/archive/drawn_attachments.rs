//! An embedded PDF or image drawn on pages appended to the archive (`doc/adr/2536`, R15).
//!
//! ISO 19005-2 section 6.8 and ISO 19005-4 section 6.9 admit an embedded file only where it
//! conforms to a part of ISO 19005, so a plain PDF, a PNG and a JPEG all fail the row. A
//! `preserve` row lays each out under `doc/adr/1014`'s permission: the PDF converted by this
//! converter and its pages imported, each image one page at one unit per sample, and the device
//! colour both paint in licensed by the archive's own output intent.

use super::attachments_as_pages::{a_document_holding, pages_of};
use super::*;
use pdf_transform::archive::AttachmentAnswer;

/// The row the tests answer the site with, at the part's own identifier.
fn appended(target: Target) -> String {
    let site = match target {
        Target::Two(_) => "embedded-files/embedded-file-is-itself-pdfa",
        Target::Four(_) => "embedded-files/embedded-file-is-itself-pdfa-in-the-plain-profile",
    };
    format!(
        "[site.\"{site}\"]\nremedy = \"preserve\"\nplacement = \"append\"\non-failure = \"discard\"\n"
    )
}

/// A one-page PDF that is not PDF/A — no metadata, no output intent, device RGB — with the
/// cross-reference table its offsets state, and §7.5.2's comment of bytes above 127 after the
/// header, so that no reader takes the file for text.
fn a_plain_pdf() -> Vec<u8> {
    a_plain_pdf_of(1)
}

/// [`a_plain_pdf`] with `count` pages, every page drawing the one content stream.
fn a_plain_pdf_of(count: usize) -> Vec<u8> {
    let content = b"0 0 1 rg 10 10 80 40 re f";
    let kids: Vec<String> = (0..count)
        .map(|index| format!("{} 0 R", index.saturating_add(4)))
        .collect();
    let mut objects = vec![
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        format!(
            "<< /Type /Pages /Kids [{}] /Count {count} >>",
            kids.join(" ")
        ),
        format!(
            "<< /Length {} >>\nstream\n{}\nendstream",
            content.len(),
            String::from_utf8_lossy(content)
        ),
    ];
    objects.extend((0..count).map(|_| {
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Contents 3 0 R >>".to_owned()
    }));
    let mut out = b"%PDF-1.7\n%\xe2\xe3\xcf\xd3\n".to_vec();
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(
            format!("{} 0 obj\n{object}\nendobj\n", index.saturating_add(1)).as_bytes(),
        );
    }
    let size = objects.len().saturating_add(1);
    let xref = out.len();
    out.extend_from_slice(format!("xref\n0 {size}\n0000000000 65535 f \n").as_bytes());
    for offset in offsets {
        out.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!("trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes(),
    );
    out
}

/// A four-by-three RGBA PNG, half its pixels transparent.
fn a_png() -> Vec<u8> {
    let mut bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut bytes, 4, 3);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().expect("the header is written");
        let pixels: Vec<u8> = (0..12u8)
            .flat_map(|index| {
                [
                    index.saturating_mul(20),
                    100,
                    200,
                    if index & 1 == 0 { 255 } else { 0 },
                ]
            })
            .collect();
        writer
            .write_image_data(&pixels)
            .expect("the samples are written");
    }
    bytes
}

/// A one-component baseline JPEG from the decoder's own test files.
const GREY_JPEG: &[u8] = include_bytes!("../../../pdf-model/tests/cut/grey.jpg");

#[test]
fn a_pdf_and_two_images_are_drawn_on_pages_and_a_broken_one_is_named() {
    let pdf = a_plain_pdf();
    let png = a_png();
    // The plant: a PNG cut after its signature, which no page can carry.
    let broken = &png[..12];
    let files: [(&str, &[u8]); 4] = [
        ("drawing.pdf", &pdf),
        ("picture.png", &png),
        ("photo.jpg", GREY_JPEG),
        ("cut.png", broken),
    ];
    let source = a_document_holding(&files);
    for target in [Target::Two(Level::B), Target::Four(Flavour::Plain)] {
        let (report, output) = convert_with_plan(&source, &plan_from(&appended(target), target));
        let conversion = conversion(&report);
        let output = output.unwrap_or_else(|| {
            panic!(
                "every file is answered at {target}, so the conversion writes: {:?} {:?}",
                conversion.decided, conversion.attachments
            )
        });
        assert_eq!(
            holds(&output, target).verdict(),
            Verdict::Conforms,
            "{target}: {:?}",
            conversion.decided
        );
        if let Some(directory) = std::env::var_os("DRAWN_ATTACHMENT_ARTIFACTS") {
            let path = std::path::Path::new(&directory)
                .join(format!("drawn-{}.pdf", target.to_string().replace('/', "")));
            std::fs::write(path, &output).expect("the artifact is written");
        }
        let outcomes = &conversion.attachments;
        assert_eq!(outcomes.len(), 4, "one outcome per file: {outcomes:?}");
        let pages_of_file = |number: u32| {
            let outcome = outcomes
                .iter()
                .find(|outcome| outcome.object.number == number)
                .expect("the file's outcome");
            match &outcome.answer {
                AttachmentAnswer::AppendedAsPages(pages) => {
                    assert_eq!(outcome.loss(), "attachment-as-pages");
                    pages.clone()
                }
                AttachmentAnswer::Removed(because) => {
                    panic!("{target}: file {number} is drawn, not removed: {because}")
                }
            }
        };
        // Files are specifications 6, 8, 10 and 12, laid out after the source's one page.
        assert_eq!(pages_of_file(6), vec![1], "the PDF's one page");
        assert_eq!(pages_of_file(8), vec![2], "the PNG's page");
        assert_eq!(pages_of_file(10), vec![3], "the JPEG's page");
        let cut = outcomes
            .iter()
            .find(|outcome| outcome.object.number == 12)
            .expect("the cut file's outcome");
        assert!(
            matches!(&cut.answer, AttachmentAnswer::Removed(because) if because.contains("does not decode")),
            "{target}: the cut PNG is removed and says why: {cut:?}"
        );
        // The pages the source states and the pages each drawn file brings, against the pages
        // written: one, plus one each for the PDF, the PNG and the JPEG.
        assert_eq!(pages_of(&source), 1);
        assert_eq!(pages_of(&output), 4, "{target}");
        let document = Document::open(output.clone()).expect("the output opens");
        let tree = pdf_model::Pages::new(&document);
        let png_page = tree.get(2).expect("the PNG's page");
        assert!(
            png_page
                .media_box
                .iter()
                .zip([0.0, 0.0, 4.0, 3.0])
                .all(|(a, b)| (a - b).abs() < 1e-3),
            "one unit per sample: {:?}",
            png_page.media_box
        );
        let text = String::from_utf8_lossy(&output);
        assert!(
            text.contains("the embedded file picture.png (object 8 0) is laid out on page(s) 3"),
            "{target}: xmpMM:History names the drawn file"
        );
    }
}

#[test]
fn a_pdf_holding_a_pdf_is_drawn_one_level_deep() {
    // The attachment's own attachment is the inner conversion's to answer without drawing it:
    // the inner plan draws nothing, so the plain PDF inside the PDF is removed there, as a file
    // neither set nor drawn.
    let inner = a_plain_pdf();
    let middle = a_document_holding(&[("inner.pdf", &inner)]);
    let source = a_document_holding(&[("middle.pdf", &middle)]);
    let target = Target::Two(Level::B);
    let (report, output) = convert_with_plan(&source, &plan_from(&appended(target), target));
    let conversion = conversion(&report);
    let output = output.expect("the middle file is drawn");
    assert_eq!(holds(&output, target).verdict(), Verdict::Conforms);
    // The source's page and the middle file's one page; the inner file brings none.
    assert_eq!(pages_of(&output), 2, "{:?}", conversion.attachments);
}

#[test]
fn drawn_files_whose_pages_cannot_be_composed_take_the_rows_discard() {
    // Two PDFs of forty pages each: each is imported, and together they pass the 64 appended
    // pages `compose` lays out, so the composition fails. The row says `on-failure = "discard"`,
    // and that answer is taken — each drawn file removed and named with the composition's own
    // sentence — rather than the conversion refused.
    let long = a_plain_pdf_of(40);
    let source = a_document_holding(&[("first.pdf", &long), ("second.pdf", &long)]);
    for target in [Target::Two(Level::B), Target::Four(Flavour::Plain)] {
        let (report, output) = convert_with_plan(&source, &plan_from(&appended(target), target));
        let conversion = conversion(&report);
        let output = output.unwrap_or_else(|| {
            panic!(
                "{target}: the row's discard answers both files, so the conversion writes: {:?} \
                 {:?}",
                conversion.decided, conversion.attachments
            )
        });
        assert_eq!(
            holds(&output, target).verdict(),
            Verdict::Conforms,
            "{target}"
        );
        let outcomes = &conversion.attachments;
        assert_eq!(
            outcomes.len(),
            2,
            "{target}: one outcome per file: {outcomes:?}"
        );
        for outcome in outcomes {
            assert!(
                matches!(&outcome.answer, AttachmentAnswer::Removed(because)
                    if because.starts_with("no page carries it, because preserving this content \
                        would append more pages")),
                "{target}: each drawn file is removed and says why: {outcome:?}"
            );
        }
        // The source's one page, and no page of either file.
        assert_eq!(pages_of(&output), 1, "{target}");
    }
}
