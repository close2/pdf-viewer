//! What one nested form `XObject` costs in stack, measured rather than assumed.
//!
//! `cargo run --release -p pdf-model --example form_depth_cost -- [KIND] [DEPTH_A DEPTH_B]`
//!
//! where `KIND` is one of `--group`, `--type3`, `--pattern` or `--mask`, and none means plain
//! form `XObject`s. `--write DEPTH PATH` writes the chain document of one depth instead, for
//! `examples/open_one` or a reference renderer to read.
//!
//! `MAX_FORM_DEPTH` is a guard on the interpreter's own recursion: a form that draws a form is
//! one more frame of `run_reader` on the thread that is drawing the page, and a thread has a
//! fixed stack. So the bound is a number about *this interpreter* — how many bytes one level
//! costs, against how many bytes a thread has — and not about the deepest document anybody has
//! seen (ADR 0793). This example prints the first of those two numbers.
//!
//! # Method
//!
//! A chain of forms `F1 → F2 → … → Fn` is generated, each drawing the next and the last filling
//! a square. For each of two depths the smallest thread stack on which `interpret` finishes is
//! found by bisection, and the per-level cost is the difference of the two thresholds over the
//! difference of the depths — which cancels whatever the page, the document and `interpret`'s
//! own entry cost, leaving one level's frames. Each probe runs in a **child process**, because a
//! stack overflow is an abort and not an error: Rust's guard page turns it into `SIGSEGV` and
//! the process dies, which is exactly why the bound exists and why the cost cannot be measured
//! in-process.
//!
//! A depth past the bound is *refused* rather than run — `MAX_FORM_DEPTH` for four of the
//! kinds, `MAX_SOFT_MASK_DEPTH` for the fifth — so the chain reports a limit, and the probe
//! reports that as a third outcome distinct from overflow; a measurement taken at such a depth
//! would be of the refusal and not of the stack, and the run says so instead of printing a
//! number.
//!
//! The five kinds are §7.8.2's five nested content streams less the annotation appearance,
//! which only ever sits at the top of a chain. `--group` makes every form a transparency group
//! (`/Group << /S /Transparency >>`), which is the deeper of the two paths a `Do` can take;
//! `--type3` chains Type 3 glyph descriptions, each showing a glyph of the next font;
//! `--pattern` chains tiling patterns, each cell filled with the next; `--mask` chains soft
//! masks, each group's content setting the next as its own mask. All of them share one bound,
//! for the reason `content.rs` gives above `MAX_FORM_DEPTH`. The figure depends on the profile
//! the example was built with: `[profile.dev]` is `opt-level = 1`, so the two are close here,
//! but the number that decides the bound is the release one.
#![expect(
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "a measuring binary whose output is its purpose; its arithmetic is over depths \
              and stack sizes it chose itself"
)]

use std::fmt::Write as _;
use std::process::{Command, ExitCode};

use pdf_syntax::{Document, Limits};

/// What the chain's forms are.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    /// Plain form `XObject`s (§8.10).
    Form,
    /// Transparency group `XObject`s (§11.6.6).
    Group,
    /// Type 3 glyph descriptions (§9.6.4), one font per level.
    Type3,
    /// Tiling pattern cells (§8.7.3), one pattern per level.
    Pattern,
    /// Soft-mask groups (§11.6.5.1), one `ExtGState` and one group per level.
    Mask,
}

impl Kind {
    fn name(self) -> &'static str {
        match self {
            Self::Form => "form XObject",
            Self::Group => "transparency group XObject",
            Self::Type3 => "Type 3 glyph description",
            Self::Pattern => "tiling pattern cell",
            Self::Mask => "soft-mask group",
        }
    }

    /// The flag that names this kind on the command line, if any.
    fn flag(self) -> Option<&'static str> {
        match self {
            Self::Form => None,
            Self::Group => Some("--group"),
            Self::Type3 => Some("--type3"),
            Self::Pattern => Some("--pattern"),
            Self::Mask => Some("--mask"),
        }
    }
}

/// The smallest stack a probe may ask for; below this `std` rounds up anyway.
const FLOOR: usize = 16 * 1024;
/// A stack every depth this example is asked for fits in.
const CEILING: usize = 256 * 1024 * 1024;
/// The resolution the bisection stops at. Thread stacks are page-granular.
const STEP: usize = 4 * 1024;

/// The probe's exit status when the chain was refused rather than drawn.
const REFUSED: u8 = 3;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut kind = Kind::Form;
    let mut numbers = Vec::new();
    let mut probe = None;
    let mut write = None;
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--write" => {
                let depth = iter.next().and_then(|s| s.parse().ok());
                let path = iter.next().cloned();
                write = Some((depth, path));
            }
            "--group" => kind = Kind::Group,
            "--type3" => kind = Kind::Type3,
            "--pattern" => kind = Kind::Pattern,
            "--mask" => kind = Kind::Mask,
            "--probe" => {
                let depth = iter.next().and_then(|s| s.parse().ok());
                let stack = iter.next().and_then(|s| s.parse().ok());
                probe = Some((depth, stack));
            }
            other => numbers.push(other.parse::<usize>().ok()),
        }
    }
    if let Some((depth, path)) = write {
        let (Some(depth), Some(path)) = (depth, path) else {
            eprintln!("--write DEPTH PATH");
            return ExitCode::from(2);
        };
        return match std::fs::write(&path, chain(kind, depth)) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("{path}: {error}");
                ExitCode::FAILURE
            }
        };
    }
    if let Some((depth, stack)) = probe {
        let (Some(depth), Some(stack)) = (depth, stack) else {
            eprintln!("--probe DEPTH STACK");
            return ExitCode::from(2);
        };
        return run_probe(kind, depth, stack);
    }
    let (shallow, deep) = match numbers.as_slice() {
        [] => (1, 16),
        [Some(a), Some(b)] if a < b => (*a, *b),
        _ => {
            eprintln!(
                "usage: form_depth_cost [--group|--type3|--pattern|--mask] [DEPTH_A DEPTH_B] \
                 with A < B, or --write DEPTH PATH"
            );
            return ExitCode::from(2);
        }
    };

    println!("{}s, {shallow} deep and {deep} deep:", kind.name());
    let Some(at_shallow) = threshold(kind, shallow) else {
        return ExitCode::from(REFUSED);
    };
    let Some(at_deep) = threshold(kind, deep) else {
        return ExitCode::from(REFUSED);
    };
    let per_level = (at_deep.saturating_sub(at_shallow)) / (deep - shallow);
    println!(
        "  smallest stack that draws {shallow} deep: {} KiB",
        at_shallow / 1024
    );
    println!(
        "  smallest stack that draws {deep} deep: {} KiB",
        at_deep / 1024
    );
    println!(
        "  one level of {} costs about {per_level} bytes of stack ({}.{} KiB), to within {} KiB \
         over {} levels",
        kind.name(),
        per_level / 1024,
        (per_level % 1024) * 10 / 1024,
        STEP / 1024,
        deep - shallow
    );
    ExitCode::SUCCESS
}

/// The smallest stack on which a chain `depth` deep draws, or `None` where the bound refuses it.
fn threshold(kind: Kind, depth: usize) -> Option<usize> {
    // A probe that draws nowhere in range is a run that cannot measure anything, and says so.
    match probe(kind, depth, CEILING) {
        Outcome::Drew => {}
        Outcome::Refused => {
            println!(
                "  {depth} deep is refused by the bound rather than drawn — nothing to measure"
            );
            return None;
        }
        Outcome::Overflowed => {
            println!(
                "  {depth} deep overflows even {} MiB",
                CEILING / 1024 / 1024
            );
            return None;
        }
    }
    let (mut low, mut high) = (FLOOR, CEILING);
    while high - low > STEP {
        let middle = usize::midpoint(low, high) / STEP * STEP;
        match probe(kind, depth, middle) {
            Outcome::Drew => high = middle,
            Outcome::Overflowed => low = middle,
            Outcome::Refused => return None,
        }
    }
    Some(high)
}

/// What one child process reported.
enum Outcome {
    Drew,
    Overflowed,
    Refused,
}

/// Runs this program again as a child, on one depth and one stack size.
fn probe(kind: Kind, depth: usize, stack: usize) -> Outcome {
    let this = std::env::current_exe().expect("this program has a path");
    let mut command = Command::new(this);
    if let Some(flag) = kind.flag() {
        command.arg(flag);
    }
    let status = command
        .arg("--probe")
        .arg(depth.to_string())
        .arg(stack.to_string())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("the child runs");
    match status.code() {
        Some(0) => Outcome::Drew,
        Some(code) if code == i32::from(REFUSED) => Outcome::Refused,
        // A stack overflow is `SIGSEGV` from the guard page, then an abort: no code at all.
        _ => Outcome::Overflowed,
    }
}

/// Interprets the chain on a thread of exactly `stack` bytes.
fn run_probe(kind: Kind, depth: usize, stack: usize) -> ExitCode {
    let bytes = chain(kind, depth);
    let drew = std::thread::Builder::new()
        .stack_size(stack)
        .spawn(move || {
            let document =
                Document::open_with_limits(bytes, Limits::DEFAULT).expect("the chain opens");
            let page = pdf_model::Pages::new(&document)
                .get(0)
                .expect("the chain has a page");
            let interpretation = pdf_model::interpret(&document, &page);
            // The square at the bottom is the only mark, so its presence is the whole chain
            // having run — and a bound reached on the way is a report, which is checked as
            // well because a soft mask refused at `MAX_SOFT_MASK_DEPTH` still lets the page's
            // own square through.
            let refused = interpretation
                .unsupported
                .iter()
                .any(|item| matches!(item, pdf_model::Unsupported::LimitReached { .. }));
            !refused && !interpretation.display_list.commands().is_empty()
        })
        .expect("a thread of that stack")
        .join()
        .expect("the probe did not panic");
    if drew {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(REFUSED)
    }
}

/// A one-page document whose content reaches a filled square through `depth` nested streams.
fn chain(kind: Kind, depth: usize) -> Vec<u8> {
    let mut body = String::new();
    body.push_str("1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    body.push_str("2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
    let first = 5;
    match kind {
        Kind::Form | Kind::Group => chain_of_forms(&mut body, kind, depth, first),
        Kind::Type3 => chain_of_glyphs(&mut body, depth, first),
        Kind::Pattern => chain_of_cells(&mut body, depth, first),
        Kind::Mask => chain_of_masks(&mut body, depth, first),
    }
    assemble(&body)
}

/// The page and `depth` forms, each drawing the next; transparency groups where `kind` says so.
fn chain_of_forms(body: &mut String, kind: Kind, depth: usize, first: usize) {
    let _ = write!(
        body,
        "3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] \
                 /Resources << /XObject << /F {first} 0 R >> >> /Contents 4 0 R >>\nendobj\n"
    );
    stream(body, 4, "", "/F Do");
    for level in 0..depth {
        let number = first + level;
        let next = number + 1;
        let group = if kind == Kind::Group {
            " /Group << /S /Transparency >>"
        } else {
            ""
        };
        let (resources, content) = if level + 1 == depth {
            (String::new(), "0 0 0 rg 10 10 100 100 re f".to_owned())
        } else {
            (
                format!(" /Resources << /XObject << /F {next} 0 R >> >>"),
                "/F Do".to_owned(),
            )
        };
        stream(
            body,
            number,
            &format!("/Type /XObject /Subtype /Form /BBox [0 0 612 792]{group}{resources}"),
            &content,
        );
    }
}

/// The page and `depth` Type 3 fonts, each glyph showing the next font's.
fn chain_of_glyphs(body: &mut String, depth: usize, first: usize) {
    // One font per level: font `n` has one glyph whose description shows glyph `a`
    // of font `n + 1`, and the last one paints the square. Each level is two
    // objects, the font dictionary and its glyph description.
    let _ = write!(
        body,
        "3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] \
                 /Resources << /Font << /T {first} 0 R >> >> /Contents 4 0 R >>\nendobj\n"
    );
    stream(body, 4, "", "BT /T 100 Tf 10 10 Td (a) Tj ET");
    for level in 0..depth {
        let font = first + 2 * level;
        let glyph = font + 1;
        let next = font + 2;
        let (resources, content) = if level + 1 == depth {
            (
                String::new(),
                "1000 0 d0\n0 0 0 rg 0 0 750 750 re f".to_owned(),
            )
        } else {
            (
                format!(" /Resources << /Font << /T {next} 0 R >> >>"),
                "1000 0 d0\nBT /T 1000 Tf 0 0 Td (a) Tj ET".to_owned(),
            )
        };
        let _ = write!(
            body,
            "{font} 0 obj\n<< /Type /Font /Subtype /Type3 /FontBBox [0 0 750 750] \
                     /FontMatrix [0.001 0 0 0.001 0 0] /CharProcs << /square {glyph} 0 R >> \
                     /Encoding << /Type /Encoding /Differences [97 /square] >> \
                     /FirstChar 97 /LastChar 97 /Widths [1000]{resources} >>\nendobj\n"
        );
        stream(body, glyph, "", &content);
    }
}

/// The page and `depth` tiling patterns, each cell filling with the next.
fn chain_of_cells(body: &mut String, depth: usize, first: usize) {
    // One pattern per level, its cell the whole page and the fill a square strictly
    // inside it; the last cell paints the square. **This kind cannot be measured past
    // about six deep**: the tiling's span takes a neighbouring cell on each side even
    // for a fill inside one, so every level multiplies the one below it ninefold, and
    // eight deep is 8.5 million commands and 2 GiB before `MAX_OPERATIONS` stops it —
    // the display list's memory rather than the stack, which the probe cannot tell
    // from an overflow. ADR 0793 records the figure and `doc/todo/49` carries it.
    let _ = write!(
        body,
        "3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] \
                 /Resources << /Pattern << /P {first} 0 R >> >> /Contents 4 0 R >>\nendobj\n"
    );
    stream(body, 4, "", "/Pattern cs /P scn 10 10 100 100 re f");
    for level in 0..depth {
        let number = first + level;
        let next = number + 1;
        let (resources, content) = if level + 1 == depth {
            (String::new(), "0 0 0 rg 10 10 100 100 re f".to_owned())
        } else {
            (
                format!(" /Resources << /Pattern << /P {next} 0 R >> >>"),
                "/Pattern cs /P scn 10 10 100 100 re f".to_owned(),
            )
        };
        stream(
            body,
            number,
            &format!(
                "/PatternType 1 /PaintType 1 /TilingType 1 /BBox [0 0 612 792] \
                         /XStep 612 /YStep 792{resources}"
            ),
            &content,
        );
    }
}

/// The page and `depth` soft masks, each group's content setting the next as its own mask.
fn chain_of_masks(body: &mut String, depth: usize, first: usize) {
    // One level is an `ExtGState` whose `/SMask` names a group form, and that group's
    // content sets the next level's `ExtGState` before it fills; the last group fills
    // white so that every mask above it lets the page's own square through.
    let _ = write!(
        body,
        "3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] \
                 /Resources << /ExtGState << /M {first} 0 R >> >> /Contents 4 0 R >>\nendobj\n"
    );
    stream(body, 4, "", "/M gs 0 0 0 rg 10 10 100 100 re f");
    for level in 0..depth {
        let state = first + 2 * level;
        let group = state + 1;
        let next = state + 2;
        let _ = write!(
            body,
            "{state} 0 obj\n<< /Type /ExtGState /SMask << /S /Luminosity /G {group} 0 R \
                     /BC [1] >> >>\nendobj\n"
        );
        let (resources, content) = if level + 1 == depth {
            (String::new(), "1 g 0 0 612 792 re f".to_owned())
        } else {
            (
                format!(" /Resources << /ExtGState << /M {next} 0 R >> >>"),
                "/M gs 1 g 0 0 612 792 re f".to_owned(),
            )
        };
        stream(
            body,
            group,
            &format!(
                "/Type /XObject /Subtype /Form /BBox [0 0 612 792] \
                         /Group << /S /Transparency /CS /DeviceGray >>{resources}"
            ),
            &content,
        );
    }
}

/// Writes one stream object.
fn stream(body: &mut String, number: usize, dict: &str, content: &str) {
    let _ = write!(
        body,
        "{number} 0 obj\n<< {dict} /Length {} >>\nstream\n{content}\nendstream\nendobj\n",
        content.len()
    );
}

/// Wraps a body of objects in a header, a cross-reference table and a trailer.
fn assemble(body: &str) -> Vec<u8> {
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(out.len());
        out.push_str(object);
    }
    let xref_at = out.len();
    let size = offsets.len() + 1;
    let _ = write!(out, "xref\n0 {size}\n0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = writeln!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF"
    );
    out.into_bytes()
}
