//! What `doc/todo/59`'s resource port costs and what it buys, measured on the documents that
//! named it.
//!
//! ```text
//! cargo run --release -p pdf-vfs --example faces_on_the_port
//! cargo run --release -p pdf-vfs --example faces_on_the_port -- doc/pdf.js/test/pdfs/other.pdf
//! ```
//!
//! # What it asks
//!
//! ADR 0870 bought a live confined worker at a stated fidelity cost: a document that names a face
//! and does not embed it is drawn from the compiled-in Latin faces, because the worker cannot walk
//! `/usr/share/fonts` and is *killed* rather than told no for trying. Four documents in the first
//! sixty of `doc/pdf.js` are in that population and each of them is a whole page lost, not a
//! glyph. That is the measurement session 914 traded away, and this takes it again.
//!
//! Three renders of page one, at 150 dpi, through `pdf_vfs::Vfs`:
//!
//! | column | transport | faces |
//! |---|---|---|
//! | `here` | this process, unconfined | the machine's, read directly — **the reference** |
//! | `withheld` | a confined worker | the compiled-in fourteen only (ADR 0870's posture) |
//! | `offered` | a confined worker | the machine's, through the port (ADR 0880) |
//!
//! **`here` is the reference and the comparison is byte identity**, which is the strongest form
//! the question has: `read_corpus.rs` holds the two transports to exactly that on every document
//! whose fonts are embedded, and calls `no_machine_fonts()` in its own process so that the
//! *confined* column is not compared against a different machine. What this example measures is
//! precisely the population that comparison had to exclude — so a `withheld` column that differs
//! from `here` is ADR 0870's cost, and an `offered` column that does not differ is the port
//! paying it back.
//!
//! Ink is printed beside it because a byte difference says *that* two pages differ and not *how
//! much*: it is the mean of `255 − luminance` over the page, in the oracle's own vocabulary, so a
//! page drawn with no glyphs at all is near zero and a page drawn with them is not.

#![forbid(unsafe_code)]
#![expect(
    clippy::print_stdout,
    reason = "an example whose whole output is a table for a person"
)]

use std::path::{Path, PathBuf};

use pdf_syntax::FileBytes;
use pdf_transform::{Budget, Policy, Secret, Source};
use pdf_vfs::worker::{InProcess, Worker, WorkerError, Workers};
use pdf_vfs::{Config, ConfinedWorkers, FileBacking, MachineFaces, Vfs};

/// The documents ADR 0870 named, which are the ones the read side's walk found being killed.
const NAMED: &[&str] = &[
    "XiaoBiaoSong.pdf",
    "SimFang-variant.pdf",
    "90ms_rksj_h_sample.pdf",
    "ThuluthFeatures.pdf",
];

/// Where those documents live.
const CORPUS: &str = "doc/pdf.js/test/pdfs";

/// How a column is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Column {
    /// This process, unconfined, with the machine's own fonts.
    Here,
    /// A confined worker offered nothing.
    Withheld,
    /// A confined worker offered the machine's faces through the port.
    Offered,
}

/// Workers of one column.
#[derive(Debug)]
struct OneColumn(Column);

impl Workers for OneColumn {
    fn spawn(
        &self,
        bytes: FileBytes,
        password: Option<Secret>,
        policy: Policy,
        budget: Budget,
    ) -> Result<Box<dyn Worker>, WorkerError> {
        match self.0 {
            Column::Here => Ok(Box::new(InProcess::new(
                match password {
                    Some(secret) => Source::with_password(bytes, secret),
                    None => Source::new(bytes),
                },
                policy,
                budget,
                Some(1),
            ))),
            Column::Withheld => {
                ConfinedWorkers::start(&bytes, None, policy, budget, MachineFaces::Withheld)
                    .map(|worker| Box::new(worker) as Box<dyn Worker>)
            }
            Column::Offered => {
                ConfinedWorkers::start(&bytes, None, policy, budget, MachineFaces::Offered)
                    .map(|worker| Box::new(worker) as Box<dyn Worker>)
            }
        }
    }
}

/// Page one of a document at 150 dpi, as the mount's own PNG.
fn drawn(path: &Path, column: Column) -> Result<Vec<u8>, String> {
    let vfs = Vfs::new(
        Box::new(FileBacking::new(path)),
        Box::new(OneColumn(column)),
        Config::default(),
    );
    vfs.open("/renders/150dpi/0001.png")
        .map(|handle| handle.bytes().to_vec())
        .map_err(|error| error.to_string())
}

/// The mean of `255 − luminance` over a decoded page, which is the oracle's own measure.
///
/// Luminance from the sRGB weights, on whatever the mount's PNG turned out to be: the layout says
/// PNG and not which colour type, so the decoder is asked rather than assumed.
fn ink(encoded: &[u8]) -> f64 {
    let Ok(mut reader) = png::Decoder::new(std::io::Cursor::new(encoded)).read_info() else {
        return f64::NAN;
    };
    let mut buffer = vec![0u8; reader.output_buffer_size().unwrap_or(0)];
    let Ok(frame) = reader.next_frame(&mut buffer) else {
        return f64::NAN;
    };
    let channels = match frame.color_type {
        png::ColorType::Grayscale => 1,
        png::ColorType::GrayscaleAlpha => 2,
        png::ColorType::Rgb => 3,
        png::ColorType::Rgba => 4,
        png::ColorType::Indexed => return f64::NAN,
    };
    let pixels = &buffer[..frame.buffer_size()];
    let mut total = 0.0f64;
    let mut count = 0usize;
    for pixel in pixels.chunks_exact(channels) {
        let luminance = if channels <= 2 {
            f64::from(pixel[0])
        } else {
            0.2126 * f64::from(pixel[0])
                + 0.7152 * f64::from(pixel[1])
                + 0.0722 * f64::from(pixel[2])
        };
        total += 255.0 - luminance;
        count = count.saturating_add(1);
    }
    if count == 0 {
        return 0.0;
    }
    total / f64::from(u32::try_from(count).unwrap_or(u32::MAX))
}

fn main() {
    // **Trap 10, and it cost this example its first whole run.** `Column::Here` decodes §7.4.6's,
    // §7.4.7's and §7.4.9's images by spawning `pdf-sandbox-worker`; the *confined* columns decode
    // them in-process, because a confined process cannot spawn (ADR 0218). So without the worker
    // beside this binary, `here` draws nothing on every JBIG2 document and the two confined columns
    // draw the image — and the run reported **124 documents "still short"**, of which 119 had a
    // reference page with no ink at all. With the worker in place the same run reports **0**. A
    // difference attributed to fonts that is really a missing decoder is exactly the shape trap 16
    // is about, so this refuses rather than measuring.
    if let Err(missing) = confined_transport::program_beside_executable(
        pdf_sandbox::WORKER_PROGRAM,
        "PDF_SANDBOX_WORKER",
    ) {
        println!(
            "refusing to measure: {} {missing} — this example's reference column decodes images by \
             spawning it and the confined columns decode them in-process, so without it every \
             difference in an image is reported as a difference in a font (trap 10). Build it with \
             `cargo build --release -p pdf-sandbox --bins` and put it beside this binary.",
            pdf_sandbox::WORKER_PROGRAM
        );
        return;
    }

    let named: Vec<PathBuf> = {
        let given: Vec<PathBuf> = std::env::args_os().skip(1).map(PathBuf::from).collect();
        if given.is_empty() {
            NAMED
                .iter()
                .map(|name| Path::new(CORPUS).join(name))
                .collect()
        } else {
            given
        }
    };

    println!(
        "{:<26} {:>10} {:>10} {:>10}   verdict",
        "document", "here", "withheld", "offered"
    );
    let mut paid_back = 0usize;
    let mut still_short = 0usize;
    for path in &named {
        let name = path.file_name().map_or_else(
            || path.display().to_string(),
            |name| name.to_string_lossy().into_owned(),
        );
        if !path.is_file() {
            println!("{name:<26} {:>10}", "absent");
            continue;
        }
        let here = drawn(path, Column::Here);
        let withheld = drawn(path, Column::Withheld);
        let offered = drawn(path, Column::Offered);
        let verdict = match (&here, &withheld, &offered) {
            (Ok(here), Ok(withheld), Ok(offered)) => {
                let short = here != withheld;
                let paid = here == offered;
                if short && paid {
                    paid_back = paid_back.saturating_add(1);
                    "the port pays ADR 0870's cost back in full"
                } else if short {
                    still_short = still_short.saturating_add(1);
                    "still short: the offered face is not the one read here"
                } else if paid {
                    "nothing was owed: withheld already matched"
                } else {
                    still_short = still_short.saturating_add(1);
                    "neither column matches"
                }
            }
            _ => "a column refused",
        };
        let show = |what: &Result<Vec<u8>, String>| match what {
            Ok(bytes) => format!("{:.2}", ink(bytes)),
            Err(_) => String::from("refused"),
        };
        println!(
            "{name:<26} {:>10} {:>10} {:>10}   {verdict}",
            show(&here),
            show(&withheld),
            show(&offered)
        );
        for (column, what) in [
            ("here", &here),
            ("withheld", &withheld),
            ("offered", &offered),
        ] {
            if let Err(why) = what {
                println!("    {column}: {why}");
            }
        }
    }
    println!(
        "\n{paid_back} of {} paid back in full, {still_short} still short",
        named.len()
    );
}
