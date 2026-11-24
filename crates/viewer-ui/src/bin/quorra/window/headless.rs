//! This window driven without a display: an [`App`] made by [`App::new`], the launch path's own
//! constructor, its document opened by the launch path's own `open_document` on the thread the
//! launch path uses and joined as `resumed` joins it, and every key and press handed to the
//! handlers winit's events reach. Nothing is mocked; what is absent is the window, which the
//! handlers already treat as not yet made (`App::redraw`, `App::window_now`).

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use viewer_core::{Answer, Command, Query, RestrictionLevel, RestrictionPolicy};
use winit::event::ElementState;
use winit::keyboard::{Key, ModifiersState, NamedKey};

use crate::app::{App, Launching, Pending};

#[path = "../../../../../../tools/conformance/test-support/signed_under_a_policy.rs"]
mod signed_under_a_policy;

#[path = "../../../../../../tools/conformance/test-support/showing.rs"]
mod showing;

#[path = "../../../../../pdf-media-worker/tests/support/pcm_stream.rs"]
#[expect(
    dead_code,
    reason = "the fixture's other streams are `pdf-media-worker`'s own tests'"
)]
mod pcm_stream;

/// A file under the system's temporary directory, named for this process and `name`.
fn temporary(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("quorra-headless-{}-{name}", std::process::id()))
}

/// A one-revision file of `objects`, numbered from 1, with its exact cross-reference table.
fn file_of(objects: &[&str]) -> Vec<u8> {
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::with_capacity(objects.len());
    for (index, object) in objects.iter().enumerate() {
        offsets.push(out.len());
        let _ = write!(out, "{} 0 obj\n{object}\nendobj\n", index.saturating_add(1));
    }
    let xref = out.len();
    let size = objects.len().saturating_add(1);
    let _ = write!(out, "xref\n0 {size}\n0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n"
    );
    out.into_bytes()
}

/// The window on the document at `path`, with `reader`'s words, made as `main` makes it and given
/// an 800 by 1000 viewport, as `resumed` gives it the window's size.
fn window_on(path: &Path, reader: viewer_host::ReaderWords) -> App {
    let trusting = reader.trusting();
    let opening = viewer_host::Loading::start({
        let path = path.to_path_buf();
        move || {
            crate::open_document(
                &path,
                None,
                None,
                crate::Policies {
                    restrictions: RestrictionPolicy::uniform(RestrictionLevel::Off),
                    reader,
                    separations: false,
                    plays_here: true,
                },
            )
        }
    });
    let launch = crate::timing::Launch::new();
    let trace = crate::trace::Trace::off(launch.began);
    let mut app = App::new(Launching {
        path: path.to_path_buf(),
        also: Vec::new(),
        fragment: None,
        trace,
        processor: true,
        proxy_pages: 0,
        supersample: 1,
        coverage: crate::arguments::CoverageChoice::Auto,
        backend: None,
        backend_asked_for: false,
        standing: viewer_host::Restrictions::new(RestrictionPolicy::uniform(RestrictionLevel::Off)),
        links: viewer_host::Links::Refuse,
        remote_documents: viewer_host::RemoteDocuments::Refuse,
        archive: None,
        trusting,
        separations: false,
        chrome: None,
        opening: Some(opening),
        ahead: None,
        instancing: None,
        launch,
        // As the launch's when the helper is there: a sound is played as the window plays one
        // (ADRs 3301, 3417).
        hearing: viewer_host::sound::Hearing::Here,
    });
    app.take_the_opening();
    app.dispatch(Command::Resize {
        width: 800,
        height: 1000,
        scale: 1.0,
    });
    app
}

/// The core's point for page point `(x, y)` on page one, as the window's pointer arrives there.
fn at(app: &App, x: f32, y: f32) -> (f32, f32) {
    let Answer::Geometry(geometry) = app.viewer.query(Query::PageGeometry(0)) else {
        panic!("page one has a geometry once the window has a size");
    };
    (
        geometry.origin.0 + x * geometry.scale,
        geometry.origin.1 + (geometry.page.height - y) * geometry.scale,
    )
}

/// ISO 32000-2 §12.7.5.3, Table 231 bit 21: a file-select control's file is read once, at the
/// Enter that commits the typed path, never per keystroke (ADR 1216, ADR 3041) — counted here at
/// the window, through the press that aims the keyboard and the keys that type and commit.
#[test]
fn a_typed_path_is_read_once_at_enter() {
    let document = temporary("file-select.pdf");
    std::fs::write(
        &document,
        file_of(&[
            "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R] \
             /DA (/Helv 12 Tf 0 g) /DR << /Font << /Helv 5 0 R >> >> >> >>",
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 400 400] /Annots [4 0 R] >>",
            "<< /Type /Annot /Subtype /Widget /P 3 0 R /FT /Tx /T (upload) /Ff 1048576 \
             /Rect [20 300 220 330] /DA (/Helv 12 Tf 0 g) >>",
            "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>",
        ]),
    )
    .expect("write the form");
    let chosen = temporary("file-select-chosen.txt");
    std::fs::write(&chosen, b"typed").expect("write the chosen file");
    let path = chosen.to_string_lossy().into_owned();

    let mut app = window_on(&document, viewer_host::ReaderWords::default());
    let field = at(&app, 120.0, 315.0);
    app.click_page(field, ElementState::Pressed);
    app.click_page(field, ElementState::Released);
    assert!(
        app.typing.is_some(),
        "the press gives the field the keyboard"
    );

    let before = viewer_host::chosen_reads();
    for character in path.chars() {
        let mut typed = [0; 4];
        app.pressed(&Key::Character(character.encode_utf8(&mut typed)));
    }
    assert_eq!(
        viewer_host::chosen_reads(),
        before,
        "no keystroke reads a file"
    );
    app.pressed(&Key::Named(NamedKey::Enter));
    assert_eq!(
        viewer_host::chosen_reads(),
        before.saturating_add(1),
        "one read for the typed path and its Enter"
    );
    assert!(app.typing.is_none(), "Enter gives the keyboard back");

    let Answer::Fields(fields) = app.viewer.query(Query::Fields) else {
        panic!("the window's core answers its fields");
    };
    let value = fields
        .into_iter()
        .find(|one| one.name.qualified == "upload")
        .and_then(|one| one.value)
        .map(|value| value.text);
    let _ = std::fs::remove_file(&document);
    let _ = std::fs::remove_file(&chosen);
    assert_eq!(
        value.as_deref(),
        Some(path.as_str()),
        "the path is the field's text"
    );
}

/// What Shift + T leaves waiting for the person's answer: the references it will hand the
/// refresh, or `None` where it put no question.
fn shift_t(app: &mut App) -> Option<Vec<String>> {
    app.modifiers_changed(ModifiersState::SHIFT);
    app.pressed(&Key::Character("T"));
    app.modifiers_changed(ModifiersState::empty());
    match &app.asked {
        Some(Pending::TrustedLists { references }) => Some(references.clone()),
        _ => None,
    }
}

/// **A signed document's policy reference reaches the refresh through Shift + T** (ADR 2936,
/// ETSI TS 119 172-2 clause 4.22.2.1 b)): with the policy's copy bound, the question at `ask`
/// holds the list the policy references, which is what its yes hands the refresh, and the yes
/// starts it. The calibration is the same document before the copy is bound, whose question holds
/// none: only a bound policy is anybody's agreement.
#[test]
fn a_bound_policy_s_reference_goes_with_shift_t_to_the_refresh() {
    let directory = temporary("trusted-lists");
    // A directory left by an earlier run is the one thing to clear; its absence is the usual case.
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).expect("a temporary directory");
    std::fs::write(directory.join("OJEU-LOTL-Certs-Set.der"), [0x30, 0x00])
        .expect("the certificate file");
    std::fs::write(
        directory.join(pdf_signature::trusted_list::authentication::OJEU_LOTL_LOC),
        "http://127.0.0.1:9/never-answered.xml",
    )
    .expect("the location file");
    let document = temporary("signed.pdf");
    std::fs::write(&document, signed_under_a_policy::document()).expect("write the document");

    let mut app = window_on(
        &document,
        viewer_host::ReaderWords {
            trusted_lists: Some(directory.clone()),
            ..viewer_host::ReaderWords::default()
        },
    );
    app.restrictions = app.restrictions.with(viewer_host::ActLevel::Submissions(
        viewer_host::Submissions::Ask,
    ));

    assert_eq!(
        shift_t(&mut app),
        Some(Vec::new()),
        "an unbound policy is nobody's agreement"
    );
    app.pressed(&Key::Named(NamedKey::Escape));
    assert!(app.asked.is_none(), "Escape answers the question");

    let focused = app.documents.focused();
    assert_eq!(
        signed_under_a_policy::bind(&mut app.viewer, focused),
        1,
        "the signature publishes its policy"
    );
    assert_eq!(
        shift_t(&mut app),
        Some(vec![signed_under_a_policy::REFERENCE.to_owned()]),
        "the question holds the referenced list"
    );
    assert!(app.refresher.interval().is_none(), "nothing fetched yet");
    app.pressed(&Key::Named(NamedKey::Enter));
    assert!(
        app.refresher.interval().is_some(),
        "the yes starts the refresh"
    );
    let _ = std::fs::remove_file(&document);
    let _ = std::fs::remove_dir_all(&directory);
}

/// Turns the window's loop as `about_to_wait` does — the picture that is due put up — until
/// `done` holds, two minutes at most.
fn show_until(app: &mut App, what: &str, mut done: impl FnMut(&App) -> bool) {
    let began = std::time::Instant::now();
    while !done(app) {
        assert!(
            began.elapsed() < std::time::Duration::from_mins(2),
            "the window never {what} (the worker must be built: `cargo build -p pdf-media-worker \
             --bins`)"
        );
        let next = app.show_the_pictures();
        let now = std::time::Instant::now();
        let rest = next
            .map_or(std::time::Duration::from_millis(5), |at| {
                at.saturating_duration_since(now)
            })
            .min(std::time::Duration::from_millis(40));
        std::thread::sleep(rest);
    }
}

/// Which of `pictures` the processor's picture `raster` is: the one whose samples, converted by
/// the oracle, are its pixels.
fn which(
    raster: &pdf_render::Raster,
    pictures: &[pdf_media_worker::video::Picture],
) -> Option<usize> {
    let conversion =
        pdf_render::video::Conversion::for_signal(pdf_render::video::Signal::default(), 16)
            .expect("the default signal converts");
    pictures.iter().position(|samples| {
        let planes = pdf_render::video::Planes {
            width: 16,
            height: 16,
            y: &samples.y,
            cb: &samples.u,
            cr: &samples.v,
        };
        render_cpu::video::convert_picture(&planes, &conversion).is_ok_and(|ours| ours == *raster)
    })
}

/// The processor's picture composed over a white 800 by 1000 window: drawn at the centre of the
/// screen annotation's quadrilateral, and not just past its far corner.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "pixel offsets inside an 800 by 1000 window"
)]
fn drawn_only_in_its_rectangle(app: &App) {
    let quad = app
        .picture_quad()
        .expect("the screen annotation is on the window");
    let list = app.picture_list(800, 1000).expect("a picture to draw");
    let page = pdf_render::Raster {
        width: 800,
        height: 1000,
        format: pdf_render::RasterFormat::Rgba8,
        data: [255, 255, 255, 255].repeat(800 * 1000),
    };
    let composed = viewer_ui::software::compose(&page, &[&list]).expect("compose");
    let pixel = |x: f32, y: f32| {
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "a point inside an 800 by 1000 window"
        )]
        let at = (y as usize * 800 + x as usize) * 4;
        composed.data[at..at + 3].to_vec()
    };
    let (centre_x, centre_y) = (
        f32::midpoint(quad[0], quad[4]),
        f32::midpoint(quad[1], quad[5]),
    );
    assert_ne!(
        pixel(centre_x, centre_y),
        [255, 255, 255],
        "drawn in the rectangle"
    );
    assert_eq!(
        pixel(quad[4] + 4.0, quad[5] + 4.0),
        [255, 255, 255],
        "and only there"
    );
}

/// ISO 32000-2 §13.2.6.1's Table 294 `/W` 3 on this window drawn by the processor, end to end: the
/// screen annotation activated, the clip judged for this window's own player (`ShowsVideoHere`),
/// the confined worker decoding `pdf-media-worker`'s `I_PCM` clip, and each picture put up as its
/// samples converted, over the annotation's rectangle and nowhere else, in order, until the clip
/// ends and the rectangle is the page's again; then a second play paused by Table 218's `/OP` 2,
/// held still, resumed by Space and stopped by Escape before its end (ADR 3545).
#[test]
fn a_clip_plays_in_its_screen_annotation_and_stops_and_pauses() {
    let (sets, units, pictures) = pcm_stream::stream(8, 16, 16, pcm_stream::Coding::Progressive);
    let clip = pcm_stream::mp4(&sets, &units, 16, 16);
    let document = temporary("video.pdf");
    std::fs::write(&document, showing::a_video_document(&clip)).expect("write the document");
    let mut app = window_on(&document, viewer_host::ReaderWords::default());
    // The reader's level for a clip played here, `open`: the clip is shown without a question.
    app.restrictions = app.restrictions.with(viewer_host::ActLevel::MediaFiles(
        viewer_host::MediaFiles::Open,
    ));

    app.dispatch(Command::Activate(showing::SCREEN));
    let mut seen: Vec<usize> = Vec::new();
    let mut placed = false;
    show_until(&mut app, "ended the clip", |app| {
        if let Some(raster) = &app.picture {
            let index = which(raster, &pictures);
            assert!(index.is_some(), "the picture up is one of the clip's own");
            if seen.last() != index.as_ref() {
                seen.extend(index);
            }
            if !placed {
                placed = true;
                drawn_only_in_its_rectangle(app);
            }
        }
        app.video.as_ref().is_some_and(|video| !video.in_hand()) && !seen.is_empty()
    });
    assert!(
        seen.windows(2).all(|pair| pair[0] < pair[1]),
        "in order: {seen:?}"
    );
    assert!(
        app.picture.is_none(),
        "the rectangle is the page's again at the end"
    );

    app.dispatch(Command::Activate(showing::SCREEN));
    show_until(&mut app, "put a picture up again", |app| {
        app.picture.is_some()
    });
    app.dispatch(Command::Activate(showing::PAUSE));
    let still = app.picture.clone();
    std::thread::sleep(std::time::Duration::from_millis(200));
    assert!(
        app.show_the_pictures().is_none(),
        "a paused clip waits for nothing"
    );
    assert_eq!(app.picture, still, "a paused clip holds its picture, still");
    app.dispatch(Command::Activate(showing::RESUME));
    show_until(&mut app, "showed a new picture after the resume", |app| {
        app.picture.is_some() && app.picture != still
    });
    app.dispatch(Command::Activate(showing::PAUSE));
    assert!(app.resume_the_pictures(), "Space shows a paused clip again");
    assert!(
        !app.resume_the_pictures(),
        "and has nothing to resume once it shows"
    );
    app.dispatch(Command::Activate(showing::STOP));
    show_until(&mut app, "took the stopped clip's picture down", |app| {
        app.picture.is_none()
    });
    assert!(
        !app.stop_the_pictures(),
        "with none in hand, Escape is the key table's"
    );
    app.dispatch(Command::Activate(showing::SCREEN));
    show_until(&mut app, "put a picture up a third time", |app| {
        app.picture.is_some()
    });
    app.pressed(&Key::Named(NamedKey::Escape));
    assert!(
        app.picture.is_none(),
        "Escape stops a clip that shows and takes its picture down"
    );
    let _ = std::fs::remove_file(&document);
}
