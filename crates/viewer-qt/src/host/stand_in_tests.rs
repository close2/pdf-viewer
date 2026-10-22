//! Byte ownership and metadata controls independent of any GUI event loop.
use super::describe;
use pdf_render::{Raster, RasterFormat};

thread_local! {
    static PLACEMENT_QUERIES: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

pub(super) fn placement_query() {
    PLACEMENT_QUERIES.with(|count| count.set(count.get().saturating_add(1)));
}

fn raster() -> Raster {
    Raster {
        width: 2,
        height: 2,
        format: RasterFormat::Rgba8,
        data: vec![1, 3, 5, 255, 7, 11, 13, 128, 17, 19, 23, 0, 29, 31, 37, 64],
    }
}

#[test]
fn a_descriptor_borrows_exact_rgba_bytes_and_keeps_scaled_placement() {
    let mut raster = raster();
    {
        let view = describe(&raster, (2.5, -1.25), (8, 12));
        assert!(view.frame.present);
        assert_eq!((view.frame.width, view.frame.height), (2, 2));
        assert_eq!((view.frame.shown_width, view.frame.shown_height), (8, 12));
        assert_eq!((view.frame.origin_x, view.frame.origin_y), (2.5, -1.25));
        assert_eq!(view.pixels, raster.data);
        assert_eq!(view.pixels.as_ptr(), raster.data.as_ptr());
    }
    raster.data[0] = 99;
    assert_eq!(describe(&raster, (0.0, 0.0), (2, 2)).pixels[0], 99);
}

#[test]
fn a_short_raster_is_not_handed_to_the_image_sink() {
    let mut raster = raster();
    raster.data.pop();
    let view = describe(&raster, (2.0, 3.0), (2, 2));
    assert!(!view.frame.present);
    assert!(view.pixels.is_empty());
    assert_eq!((view.frame.width, view.frame.height), (0, 0));
}

fn unsized_host() -> super::Host {
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc/PDF20_AN001-BPC.pdf");
    super::Host::open(
        &path,
        None,
        pdf_model::view::WidgetAppearances::Delegated,
        viewer_host::Settings::default(),
        viewer_host::ReaderWords::default(),
        viewer_host::trace::Trace::off(std::time::Instant::now()),
    )
    .expect("committed document opens")
}

fn host() -> super::Host {
    let mut host = unsized_host();
    host.resized(800, 1000, 1.0);
    drawn(&mut host);
    host
}

fn drawn(host: &mut super::Host) {
    let began = std::time::Instant::now();
    while host.drawing_wait() >= 0 {
        assert!(began.elapsed() < std::time::Duration::from_mins(2));
        std::thread::sleep(viewer_host::drawing::POLL);
        host.drawing_pump();
    }
}

#[test]
fn one_snapshot_queries_placements_once_and_borrows_the_completed_source() {
    let host = host();
    PLACEMENT_QUERIES.with(|count| count.set(0));
    let views = host.frame_views();
    assert!(!views.is_empty());
    assert_eq!(PLACEMENT_QUERIES.with(std::cell::Cell::get), 1);
    let viewer_core::Answer::Frame(frames) = host.state.viewer.query(viewer_core::Query::Frame)
    else {
        panic!("completed frames");
    };
    assert_eq!(views.len(), frames.len());
    for (view, frame) in views.iter().zip(frames) {
        assert_eq!(view.pixels.as_ptr(), frame.raster.data.as_ptr());
        assert_eq!(view.pixels, frame.raster.data);
        assert_eq!(
            (view.frame.width, view.frame.height),
            (frame.raster.width, frame.raster.height)
        );
        let origin = viewer_host::stand_in::whole_pixel(frame.origin);
        assert_eq!((view.frame.origin_x, view.frame.origin_y), origin);
    }
}

#[test]
fn transition_bypasses_page_queries_and_borrows_its_own_pixels() {
    let mut host = host();
    host.playing = Some(raster());
    PLACEMENT_QUERIES.with(|count| count.set(0));
    let views = host.frame_views();
    assert_eq!(PLACEMENT_QUERIES.with(std::cell::Cell::get), 0);
    assert_eq!(views.len(), 1);
    let view = &views[0];
    assert_eq!((view.frame.origin_x, view.frame.origin_y), (0.0, 0.0));
    assert_eq!((view.frame.shown_width, view.frame.shown_height), (2, 2));
    assert_eq!(
        view.pixels.as_ptr(),
        host.playing.as_ref().expect("transition").data.as_ptr()
    );
}

#[test]
fn spread_zoom_and_return_keep_order_bytes_and_one_query() {
    use pdf_model::viewer_preferences::PageLayout;
    use viewer_core::{Answer, Command, Query, Zoom};
    let mut host = host();
    host.state.presented = true;
    host.dispatch(Command::Layout(PageLayout::TwoPageLeft));
    drawn(&mut host);
    for zoom in [Zoom::Out, Zoom::In, Zoom::FitPage] {
        host.dispatch(Command::Zoom { zoom, at: None });
        drawn(&mut host);
        PLACEMENT_QUERIES.with(|count| count.set(0));
        let views = host.frame_views();
        assert_eq!(PLACEMENT_QUERIES.with(std::cell::Cell::get), 1);
        assert_eq!(
            views.len(),
            2,
            "the spread really exercises multiple images"
        );
        let Answer::Frame(frames) = host.state.viewer.query(Query::Frame) else {
            panic!("completed spread");
        };
        assert_eq!(frames.len(), views.len());
        assert!(frames[0].page < frames[1].page);
        for (view, frame) in views.iter().zip(frames) {
            assert_eq!(view.pixels.as_ptr(), frame.raster.data.as_ptr());
            assert_eq!(view.pixels, frame.raster.data);
            let Answer::Geometry(geometry) =
                host.state.viewer.query(Query::PageGeometry(frame.page))
            else {
                panic!("spread geometry");
            };
            assert_eq!(
                (view.frame.shown_width, view.frame.shown_height),
                (geometry.width, geometry.height)
            );
            assert_eq!(
                (view.frame.origin_x, view.frame.origin_y),
                viewer_host::stand_in::whole_pixel(geometry.origin)
            );
        }
    }
}

#[test]
fn a_host_without_a_viewport_has_no_frame_to_copy() {
    let host = unsized_host();
    PLACEMENT_QUERIES.with(|count| count.set(0));
    assert!(host.frame_views().is_empty());
    assert_eq!(PLACEMENT_QUERIES.with(std::cell::Cell::get), 1);
}
