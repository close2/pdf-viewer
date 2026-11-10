//! Consumer regressions for §12.5.6.18 and its referenced flag/action conditions.
//! Fixtures use PDF 1.7, complete xrefs, stored form appearances, and a positive action
//! control. They do not implement multimedia playback.

#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "tiny self-contained valid PDF fixtures; malformed setup should fail loudly"
)]

use std::fmt::Write as _;

use pdf_model::optional_content::Purpose;
use pdf_model::view::ViewState;
use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;
use viewer_core::{
    Command, DocumentId, Event, RestrictionLevel, RestrictionPolicy, RestrictionScope, Viewer,
};

const DOCUMENT: DocumentId = DocumentId(1);

fn assemble(objects: &[String]) -> Vec<u8> {
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for (index, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        let _ = writeln!(out, "{} 0 obj\n{body}\nendobj", index.saturating_add(1));
    }
    let xref_at = out.len();
    let size = offsets.len().saturating_add(1);
    let _ = writeln!(out, "xref\n0 {size}\n0000000000 65535 f ");
    for offset in offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

fn screen_page(flags: &str, appearance: &str) -> Vec<u8> {
    let form = "1 0 0 rg 0 0 100 100 re f";
    assemble(&[
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> /Contents 4 0 R /Annots [5 0 R] >>".to_owned(),
        "<< /Length 0 >>\nstream\n\nendstream".to_owned(),
        format!("<< /Type /Annot /Subtype /Screen /Rect [0 0 100 100] {flags} {appearance} >>"),
        format!("<< /Type /XObject /Subtype /Form /BBox [0 0 100 100] /Resources << >> /Length {} >>\nstream\n{form}\nendstream", form.len().saturating_add(1)),
    ])
}

fn stored_mark(subtype: &str, appearance: &str) -> Vec<u8> {
    let form = |paint: &str| {
        format!(
            "<< /Type /XObject /Subtype /Form /BBox [0 0 100 100] /Resources << >> /Length {} >>\nstream\n{paint}\nendstream",
            paint.len()
        )
    };
    assemble(&[
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> /Contents 4 0 R /Annots [5 0 R] >>".to_owned(),
        "<< /Length 0 >>\nstream\n\nendstream".to_owned(),
        format!("<< /Type /Annot /Subtype /{subtype} /F 68 /Rect [0 0 100 100] {appearance} >>"),
        form("1 0 0 rg 0 0 100 100 re f"),
        form("0 0 1 rg 0 0 100 100 re f"),
    ])
}

fn draw(bytes: Vec<u8>, purpose: Purpose) -> [u8; 4] {
    let document = Document::open(bytes).expect("complete fixture parses");
    assert!(!document.was_recovered(), "the xref is authoritative");
    let page = pdf_model::Pages::new(&document).get(0).expect("one page");
    let mut state = ViewState::of(&document);
    state.set_purpose(purpose);
    let interpretation = pdf_model::content::interpret_with(&document, &page, &state);
    assert!(
        interpretation.is_complete(),
        "supported fixture: {:?}",
        interpretation.unsupported
    );
    let target = TargetSpec::for_page(&interpretation.display_list, 1.0, 1 << 20)
        .expect("small page target");
    let raster = CpuRasterizer::new()
        .with_medium(pdf_render::Medium::NONE)
        .rasterize(&interpretation.display_list, target)
        .expect("CPU renderer supports the Form XObject");
    let offset =
        ((50_u32.saturating_mul(raster.width).saturating_add(50)) as usize).saturating_mul(4);
    raster.data[offset..offset + 4]
        .try_into()
        .expect("one pixel")
}

#[test]
fn screen_without_ap_is_not_printed_but_stored_normal_ap_is() {
    // The AP absence is Screen's own rule. Table 167 would otherwise ignore Print where no
    // appearance stream exists, so include /F 4 and assert the subtype-specific no-print result.
    let absent = screen_page("/F 4", "");
    assert_eq!(draw(absent.clone(), Purpose::View), [0, 0, 0, 0]);
    assert_eq!(draw(absent, Purpose::Print), [0, 0, 0, 0]);

    // Positive stored appearance control: it is used for default display and printing while no
    // clip is being played. The media player is outside this test and outside this feature path.
    let stored = screen_page("/F 4", "/AP << /N 6 0 R >>");
    assert_eq!(draw(stored.clone(), Purpose::View), [255, 0, 0, 255]);
    assert_eq!(draw(stored, Purpose::Print), [255, 0, 0, 255]);
}

#[test]
fn printer_mark_and_legacy_trapnet_render_their_stored_appearance_for_view_and_print() {
    // A conforming PrinterMark carries a normal appearance and Print+ReadOnly; ReadOnly does
    // not suppress display. The same stored-form path supports a legacy TrapNet, which is
    // deprecated but remains accepted by this compatibility reader.
    let printer_mark = stored_mark("PrinterMark", "/AP << /N 6 0 R >>");
    for purpose in [Purpose::View, Purpose::Print] {
        assert_eq!(
            draw(printer_mark.clone(), purpose),
            [255, 0, 0, 255],
            "PrinterMark {purpose:?}"
        );
    }

    // TrapNet requires AS even with its normal appearance represented as named alternatives.
    let trapnet = stored_mark(
        "TrapNet",
        "/AP << /N << /Network 6 0 R >> >> /AS /Network /LastModified (D:20261110000000Z)",
    );
    for purpose in [Purpose::View, Purpose::Print] {
        assert_eq!(
            draw(trapnet.clone(), purpose),
            [255, 0, 0, 255],
            "TrapNet {purpose:?}"
        );
    }
}

fn rendition_document(screen_page_ref: u32) -> Vec<u8> {
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> /Annots [4 0 R] >>"
            .to_owned(),
        format!(
            "<< /Type /Annot /Subtype /Screen /P {screen_page_ref} 0 R /Rect [0 0 50 50] /A 5 0 R >>"
        ),
        "<< /Type /Action /S /Rendition /OP 0 /AN 4 0 R /R 6 0 R >>".to_owned(),
        "<< /Type /Rendition /S /MR /N (clip) /C 7 0 R >>".to_owned(),
        "<< /Type /MediaClip /S /MCD /CT (video/mp4) /D 8 0 R /Alt [() (clip)] >>".to_owned(),
        "<< /Type /Filespec /F (clip.mp4) /UF (clip.mp4) /EF << /F 9 0 R >> >>".to_owned(),
        "<< /Type /EmbeddedFile /Length 4 >>\nstream\nmoov\nendstream".to_owned(),
    ];
    assemble(&objects)
}

fn activate(bytes: Vec<u8>) -> Vec<Event> {
    let parsed = Document::open(bytes.clone()).expect("complete rendition fixture");
    assert!(!parsed.was_recovered());
    let mut viewer = Viewer::new(200, 200, 1.0);
    viewer
        .handle(Command::Restrict(RestrictionScope::Window(
            RestrictionPolicy::default().with(
                pdf_model::restriction::Operation::HandOverMedia,
                RestrictionLevel::Off,
            ),
        )))
        .for_each(drop);
    let opened = viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: bytes.into(),
            password: None,
            fragment: None,
        })
        .collect::<Vec<_>>();
    assert!(
        opened
            .iter()
            .any(|event| matches!(event, Event::Opened { .. }))
    );
    assert!(
        !opened
            .iter()
            .any(|event| matches!(event, Event::Media { .. }))
    );
    viewer
        .handle(Command::Activate(pdf_syntax::ObjectId::new(4, 0)))
        .collect()
}

#[test]
fn rendition_action_requires_screen_p_to_be_its_page() {
    // Positive control uses the actual page object; reaching Event::Media is hand-off, not
    // playback. The changed reference below is the only precondition difference.
    let valid = activate(rendition_document(3));
    let handed = valid
        .iter()
        .filter_map(|event| match event {
            Event::Media { media, .. } => Some(media.clone()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        handed,
        [viewer_core::Media::Embedded {
            name: "clip.mp4".to_owned(),
            bytes: b"moov".to_vec()
        }]
    );

    let invalid = activate(rendition_document(1));
    assert!(
        !invalid
            .iter()
            .any(|event| matches!(event, Event::Media { .. })),
        "{invalid:?}"
    );
    assert!(
        invalid.iter().any(|event| matches!(
            event,
            Event::Reported { notes, .. }
                if notes.iter().any(|note| note.contains("/P is not a page object"))
        )),
        "the action reports its page-reference precondition: {invalid:?}"
    );
}

#[test]
fn printer_mark_and_trapnet_as_select_the_named_normal_appearance() {
    for subtype in ["PrinterMark", "TrapNet"] {
        for (state, pixel) in [("Red", [255, 0, 0, 255]), ("Blue", [0, 0, 255, 255])] {
            let ap = format!(
                "/AP << /N << /Red 6 0 R /Blue 7 0 R >> >> /AS /{state} /LastModified (D:20261110000000Z)"
            );
            for purpose in [Purpose::View, Purpose::Print] {
                assert_eq!(
                    draw(stored_mark(subtype, &ap), purpose),
                    pixel,
                    "{subtype} {state} {purpose:?}"
                );
            }
        }
    }
}
