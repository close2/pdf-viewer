//! Count pages whose moved view record-replays actual line-lane instances.
//!
//! Arguments are PDF paths, or `@paths.txt` with one path per line. Each first page is drawn
//! at scales 1 and 1.25 with Compute coverage. The second frame's `EncodeSource` and line
//! counter decide membership; PDF operators and display-list shapes are not proxies for it.
//! A generated line and an empty page calibrate the positive and negative answers first.
//! Every refused page is printed and counted separately from a successful negative answer.

#![expect(
    clippy::print_stdout,
    reason = "the census prints its population and every refusal"
)]

use std::path::{Path as FilePath, PathBuf};
use std::sync::Arc;

use pdf_render::{
    BlendMode, Color, Command, DisplayList, Paint, Path, PathCommand, Point, Size, Stroke,
    TargetSpec, Transform,
};
use pdf_syntax::Document;
use render_raster::{PresentFrame, QuorraRasterizer};

/// An instrument stops on setup failure and names per-document failures in its output.
type Failure = Box<dyn std::error::Error>;

/// Draw the real moved frame and read its observed road and lane count.
fn moved(list: &Arc<DisplayList>) -> Result<(bool, u32), Failure> {
    let mut backend = QuorraRasterizer::new_headless()?;
    backend.set_coverage(raster_gpu::Coverage::Compute);
    for scale in [1.0, 1.25] {
        let target = TargetSpec::for_page(list, scale, 1 << 28)?;
        let pages = [(list, target)];
        backend.rasterize_frame(&PresentFrame {
            width: target.width,
            height: target.height,
            pages: &pages,
            raster: None,
            overlays: &[],
        })?;
    }
    Ok((
        backend.last_frame().encode_source == Some(raster_gpu::EncodeSource::RecordReplayed),
        backend.last_counters().lanes.line,
    ))
}

/// A positive page and a negative page distinguish the predicate from a universal zero.
fn calibrate() -> Result<(), Failure> {
    let mut list = DisplayList::new(Size {
        width: 100.0,
        height: 100.0,
    });
    let empty = moved(&Arc::new(list.clone()))?;
    let mut path = Path::new();
    path.push(PathCommand::MoveTo(Point::new(10.0, 20.0)));
    path.push(PathCommand::LineTo(Point::new(80.0, 60.0)));
    list.push(Command::Stroke {
        path: Arc::new(path),
        transform: Transform::IDENTITY,
        stroke: Stroke {
            width: 2.0,
            ..Stroke::default()
        },
        paint: Paint::Solid(Color::BLACK),
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    });
    let line = moved(&Arc::new(list))?;
    println!("calibration: line {line:?}, empty {empty:?}");
    if !line.0 || line.1 != 1 || empty.1 != 0 {
        return Err(
            "the calibration did not distinguish a replayed line from an empty page".into(),
        );
    }
    Ok(())
}

/// Interpret exactly the first page, as the six arms do.
fn page(path: &FilePath) -> Result<Arc<DisplayList>, Failure> {
    let document = Document::open(std::fs::read(path)?)?;
    let page = pdf_model::Pages::new(&document)
        .get(0)
        .ok_or("no first page")?;
    Ok(Arc::new(
        pdf_model::interpret(&document, &page).display_list,
    ))
}

fn main() -> Result<(), Failure> {
    let mut paths = Vec::new();
    for argument in std::env::args().skip(1) {
        if let Some(path) = argument.strip_prefix('@') {
            paths.extend(std::fs::read_to_string(path)?.lines().map(PathBuf::from));
        } else {
            paths.push(PathBuf::from(argument));
        }
    }
    calibrate()?;
    let (mut drawn, mut refused, mut replayed, mut with_lines) = (0_u32, 0_u32, 0_u32, 0_u32);
    for path in &paths {
        match page(path).and_then(|list| moved(&list)) {
            Ok((record_replayed, lines)) => {
                drawn = drawn.saturating_add(1);
                if record_replayed {
                    replayed = replayed.saturating_add(1);
                    if lines > 0 {
                        with_lines = with_lines.saturating_add(1);
                        println!("replayed-lines\t{lines}\t{}", path.display());
                    }
                }
            }
            Err(error) => {
                refused = refused.saturating_add(1);
                println!("refused\t{}\t{error}", path.display());
            }
        }
    }
    println!(
        "{} requested, {drawn} drawn, {refused} refused; {replayed} record-replayed, {with_lines} with lines",
        paths.len()
    );
    Ok(())
}
