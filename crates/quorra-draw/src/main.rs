//! Draws one page of a PDF to a PNG on the graphics device, through `render-raster`.
//!
//! ```sh
//! quorra-draw document.pdf --page 3 --dpi 144 -o page-3.png
//! ```
//!
//! The window draws its pages on the device (CLAUDE.md principle 2), and `quorra-transform render`
//! draws them on the processor's oracle, `render-cpu`. This command is the device's half of that
//! pair: the same interpretation, the same target arithmetic and the same encoder, with the device
//! in the rasteriser's place. So a page drawn by both commands differs only where the two backends
//! do, and the portable conformance suite's `quorra-gpu` adapter judges the device by the clause
//! exactly as its `quorra` adapter judges the oracle (RFC 0016 section 5, ADR 3314).
//!
//! - **The page** is one-based, as `quorra-transform`'s `--pages` is; 1 when not named.
//! - **The size** is ISO 32000-2 §8.3.2.3's: "\[t]he default for the size of the unit in default
//!   user space (1 / 72 inch)", so a page *w* units wide at *d* dots per inch is *w* · *d* / 72
//!   pixels wide, and [`pdf_render::TargetSpec::for_page`] makes that a whole raster as it does for
//!   every backend. 150 dpi when not named, `quorra-transform`'s default.
//! - **The state** is the window's: the document's own defaults ([`ViewState::of`]), §6.3.2.2's
//!   annotation pass included, under §8.11.4.4's `View` event, since what is judged is what a
//!   person sees on the screen.
//! - **The device** is the one the environment selects, as for the window (`WGPU_BACKEND`,
//!   `VK_ICD_FILENAMES`); the adapter is printed on standard error, because the adapter a run
//!   drew on is the record of which device it was, never the environment it was given. A page the
//!   device refuses is refused out loud, with exit 4, and never drawn on the processor instead:
//!   the command exists to show what the device draws.
//!
//! Exit statuses: 0 written; 1 the file could not be read or written, or no device came up; 2 the
//! arguments do not form a request; 4 the page was declined (no such page, a target past the
//! budget, a refusal by the device), the reason on standard error.
//!
//! **The shared adapter's lock is this program's to hold** for the device's whole life (ADRs 2664,
//! 2675, 3093): the device-section hook is installed before the device exists. Where the machine
//! has no lock file the hook takes nothing, and on the server it queues behind every earlier
//! waiter. The lock's own failures — a wait past `GPU_LOCK_TIMEOUT`, a hang marker standing —
//! stop the program by panicking with the lock's name, the cost ADR 3314 states.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use std::fmt;
use std::path::PathBuf;
use std::process::ExitCode;

use pdf_model::Pages;
use pdf_model::content::FontCache;
use pdf_model::view::ViewState;
use pdf_render::{Raster, Rasterizer as _, TargetSpec};
use pdf_syntax::Document;
use render_raster::QuorraRasterizer;

#[path = "../../../tools/conformance/test-support/gpu_lock.rs"]
mod gpu_lock;
#[path = "../../../tools/conformance/test-support/gpu_lock_hook.rs"]
mod gpu_lock_hook;

/// The dots per inch when `--dpi` is not given: `quorra-transform render`'s default.
const DEFAULT_DPI: f32 = 150.0;

/// The most pixels one page may have: `quorra-transform`'s budget (`pdf_transform::Budget`), a
/// gibibyte of RGBA. A page's size comes from the document, so the product of size and scale is
/// bounded before anything is allocated (CLAUDE.md principle 3).
const MAX_PIXELS: u64 = 1 << 28;

/// The command's usage line, printed with a usage failure.
const USAGE: &str = "quorra-draw <file.pdf> [--page <n>] [--dpi <dots per inch>] -o <out.png>";

/// One request, as the arguments state it.
#[derive(Debug, Clone, PartialEq)]
struct Request {
    /// The document.
    pdf: PathBuf,
    /// The page, one-based.
    page: usize,
    /// Dots per inch.
    dpi: f32,
    /// Where the PNG goes.
    out: PathBuf,
}

/// Why no page was written, each with its exit status.
#[derive(Debug)]
enum Failure {
    /// The arguments do not form a request.
    Usage(String),
    /// The document could not be read or opened.
    Open(String),
    /// No graphics device came up.
    Device(String),
    /// The page was declined, by name.
    Declined(String),
    /// The PNG could not be encoded or written.
    Write(String),
}

impl fmt::Display for Failure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usage(what) => write!(formatter, "{what}\nusage: {USAGE}"),
            Self::Open(what) => write!(formatter, "cannot open the document: {what}"),
            Self::Device(what) => write!(formatter, "no graphics device: {what}"),
            Self::Declined(what) => write!(formatter, "the page is declined: {what}"),
            Self::Write(what) => write!(formatter, "cannot write the page: {what}"),
        }
    }
}

impl Failure {
    /// The exit status the module comment gives each failure.
    fn status(&self) -> u8 {
        match self {
            Self::Usage(_) => 2,
            Self::Declined(_) => 4,
            Self::Open(_) | Self::Device(_) | Self::Write(_) => 1,
        }
    }
}

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    match parse(&arguments).and_then(|request| draw(&request)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(failure) => {
            eprintln!("quorra-draw: {failure}");
            ExitCode::from(failure.status())
        }
    }
}

/// Reads the arguments into a request.
fn parse(arguments: &[String]) -> Result<Request, Failure> {
    let mut pdf = None;
    let mut page = 1;
    let mut dpi = DEFAULT_DPI;
    let mut out = None;
    let mut words = arguments.iter();
    while let Some(word) = words.next() {
        let mut value = |option: &str| {
            words
                .next()
                .ok_or_else(|| Failure::Usage(format!("{option} needs a value")))
        };
        match word.as_str() {
            "--page" => {
                let given = value("--page")?;
                page = given
                    .parse::<usize>()
                    .ok()
                    .filter(|&page| page >= 1)
                    .ok_or_else(|| {
                        Failure::Usage(format!("--page {given}: a page is a whole number from 1"))
                    })?;
            }
            "--dpi" => {
                let given = value("--dpi")?;
                dpi = given
                    .parse::<f32>()
                    .ok()
                    .filter(|&dpi| dpi.is_finite() && dpi > 0.0)
                    .ok_or_else(|| {
                        Failure::Usage(format!("--dpi {given}: a resolution is a positive number"))
                    })?;
            }
            "-o" | "--output" => out = Some(PathBuf::from(value("-o")?)),
            option if option.starts_with('-') && option.len() > 1 => {
                return Err(Failure::Usage(format!("unknown option {option}")));
            }
            file => {
                if pdf.replace(PathBuf::from(file)).is_some() {
                    return Err(Failure::Usage("one document at a time".to_owned()));
                }
            }
        }
    }
    Ok(Request {
        pdf: pdf.ok_or_else(|| Failure::Usage("no document named".to_owned()))?,
        page,
        dpi,
        out: out.ok_or_else(|| Failure::Usage("no output named (-o)".to_owned()))?,
    })
}

/// Interprets the page, draws it on the device and writes the PNG.
fn draw(request: &Request) -> Result<(), Failure> {
    let bytes = std::fs::read(&request.pdf)
        .map_err(|error| Failure::Open(format!("{}: {error}", request.pdf.display())))?;
    let document = Document::open(bytes)
        .map_err(|error| Failure::Open(format!("{}: {error}", request.pdf.display())))?;
    let pages = Pages::new(&document);
    // `parse` admits no page below 1, so the subtraction never saturates.
    let page = pages.get(request.page.saturating_sub(1)).ok_or_else(|| {
        Failure::Declined(format!(
            "page {} of a document of {}",
            request.page,
            pages.len()
        ))
    })?;
    let view = ViewState::of(&document);
    let list = pdf_model::content::interpret_with_fonts(&document, &page, &view, &FontCache::new())
        .display_list;
    let target = TargetSpec::for_page(&list, request.dpi / 72.0, MAX_PIXELS)
        .map_err(|refusal| Failure::Declined(refusal.to_string()))?;

    gpu_lock_hook::install();
    let mut device =
        QuorraRasterizer::new_headless().map_err(|error| Failure::Device(error.to_string()))?;
    eprintln!("quorra-draw: adapter {}", device.adapter_description());
    let raster = device
        .rasterize(&list, target)
        .map_err(|refusal| Failure::Declined(format!("the device refuses it: {refusal}")))?;
    // The device is dropped, and the shared adapter's lock with it, before the file is written:
    // encoding is the processor's work and holds nobody else back (ADR 2675).
    drop(device);

    let png = encode(&raster).map_err(|error| Failure::Write(error.to_string()))?;
    std::fs::write(&request.out, png)
        .map_err(|error| Failure::Write(format!("{}: {error}", request.out.display())))
}

/// The raster as an 8-bit RGBA PNG, as `quorra-transform render` writes one: the backend has
/// imposed the page on §11.4.7's white already, so every alpha is 255 and nothing is lost by
/// keeping the channel.
fn encode(raster: &Raster) -> Result<Vec<u8>, png::EncodingError> {
    let mut bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut bytes, raster.width, raster.height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header()?;
        writer.write_image_data(&raster.data)?;
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::{DEFAULT_DPI, Failure, Request, parse};
    use std::path::PathBuf;

    fn words(line: &str) -> Vec<String> {
        line.split_whitespace().map(str::to_owned).collect()
    }

    #[test]
    fn a_request_names_its_page_resolution_and_output() {
        let request = parse(&words("in.pdf --page 3 --dpi 144 -o out.png")).expect("a request");
        assert_eq!(
            request,
            Request {
                pdf: PathBuf::from("in.pdf"),
                page: 3,
                dpi: 144.0,
                out: PathBuf::from("out.png"),
            }
        );
    }

    #[test]
    fn the_first_page_at_the_transform_s_resolution_is_the_default() {
        let request = parse(&words("in.pdf -o out.png")).expect("a request");
        assert_eq!((request.page, request.dpi), (1, DEFAULT_DPI));
    }

    #[test]
    fn a_request_without_a_page_number_or_output_is_a_usage_failure() {
        for line in [
            "in.pdf",
            "-o out.png",
            "in.pdf --page 0 -o out.png",
            "in.pdf --dpi -1 -o out.png",
            "in.pdf --dpi NaN -o out.png",
            "in.pdf other.pdf -o out.png",
            "in.pdf --zoom 2 -o out.png",
            "in.pdf -o",
        ] {
            let failure = parse(&words(line)).expect_err(line);
            assert!(matches!(failure, Failure::Usage(_)), "{line}: {failure:?}");
            assert_eq!(failure.status(), 2, "{line}");
        }
    }
}
