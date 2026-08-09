//! The viewer: opens a PDF and shows it.
//!
//! ``text
//! cargo run --release -p viewer-ui --bin pdf-viewer -- document.pdf
//! ``
//!
//! `--page N` opens at a page, and so does ISO 32000-2 Annex O's fragment identifier —
//! `document.pdf#page=5`, `#nameddest=Chapter3`, `#zoom=150,0,792` — which this program splits off
//! the argument and `viewer-core` carries out, saying by name what it cannot.
//! Arrows, Page Up and Down or Space turn pages, Home and End jump,
//! `+` and `-` zoom and **Ctrl + the wheel zooms about the pointer**, the wheel alone scrolls
//! whatever is under it, `a` selects the whole page and dragging selects part of it, `o` shows
//! the sidebar — §12.3.3's outline, §8.11's layers and §7.11.4's embedded files — `s` saves what
//! was changed beside the document, `?` shows the third-party notices this binary carries,
//! Escape quits. The window title shows the page's own label where the document states one
//! (§12.4.2), the page number, and how many things on the page could not be drawn; the things
//! themselves are printed.
//!
//! # What is here and what is not
//!
//! This is **consumer #1 of `viewer-core`** and a **tier-2 host**: everything about documents,
//! pages, zoom, links and actions is in that crate, and what is left here is a window, a
//! keyboard, a GPU and the two decisions a host owns — which files a document may name, and what
//! to do when it asks for a password.
//!
//! And, since the hundred-and-sixty-sixth session, **chrome**: `viewer_ui::chrome` draws a
//! sidebar of this program's own — §12.3.3's outline, §8.11.4.3's layers with their switches,
//! and §7.11.4's embedded files — because winit is a window and an event loop and there is no
//! toolkit here to ask for a tree view. A native host would use its platform's, from the same
//! three queries; what is host-specific is the drawing, not the data.
//!
//! Tier 2 means the pixels never cross the boundary: `viewer-core` hands over a display list and
//! a target, this draws it onto the surface with `render-quorra`, and answers `Rendered::Presented`.
//! A tier-1 host would take the raster back instead; the protocol is otherwise identical, which
//! is the property that makes the interface worth having.
//!
//! # Reporting incomplete pages
//!
//! When the interpreter cannot draw something — an unsupported font, a shading — it is said out
//! loud. A viewer that renders a page missing half its content and looks confident about it is
//! worse than one that admits the gap, and the person looking at it is the only one in a
//! position to judge whether what is missing matters.

#![expect(
    clippy::expect_used,
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "a command-line application: stdout is a reporting channel and a panic on a \
              missing display is the intended failure"
)]

use std::collections::VecDeque;
use std::io::Write as _;
use std::path::PathBuf;
use std::sync::Arc;

use pdf_model::form::Control;
use pdf_render::{
    BlendMode, Color, Command as DrawCommand, FillRule, Image, Paint, Path, PathCommand, Point,
    Raster, Rasterizer as _, Rect, Size, TargetSpec, Transform,
};
use pdf_syntax::ObjectId;
use render_cpu::CpuRasterizer;
use render_quorra::{PresentFrame, QuorraPresenter};
use viewer_core::{
    Answer, Command, DocumentId, Edit, Entered, Event, Find, FindDirection, FocusMove, PageTarget,
    PointerAction, Purpose, Query, RenderRequest, Rendered, RestrictionLevel, Selection, Viewer,
    Zoom,
};
use viewer_ui::chrome::{About, Chrome, Content, FindBar, Hit, Sidebar, Tab};
use viewer_ui::software::SoftwareSurface;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, KeyEvent, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

/// The third-party notices this binary is obliged to carry, printed by `--licences`.
///
/// Both licences covering the compiled-in standard 14 fonts require a *binary* distribution to
/// reproduce their notices "in the documentation and/or other materials provided with the
/// distribution", and until the hundred-and-forty-eighth session this program had nowhere to put
/// them. `include_str!` rather than a path, for the same reason the fonts themselves are
/// `include_bytes!`d: a notice that can go missing between the binary and the file system is not
/// carried by the binary.
///
/// `--licenses` is accepted too. The project spells it the other way and a person typing the
/// other spelling wants the same thing.
const NOTICE: &str = include_str!("../../../../NOTICE");

/// The one document this program opens.
///
/// `viewer-core` keeps a set of them because §12.6.4.4's embedded go-to and a tabbed host both
/// need one; this window shows one at a time, so it names one.
const DOCUMENT: DocumentId = DocumentId(0);

/// A driver stack `--backend` can name: what talks to the GPU, not which GPU.
///
/// **The distinction is the whole reason this exists.** One GPU is enumerated once per backend
/// that can drive it, under the *device's* name each time, so a name filter selects hardware and
/// cannot express "this card, through DX12". The set of backends is an instance-level choice and
/// the instance is made before anything else, which is why this is decided on the command line
/// and carried to `QuorraPresenter::instance_with` (quorra's ADR 0017; ours is 0221).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Backend {
    /// Vulkan: Linux's and Android's, and one of Windows' two.
    Vulkan,
    /// Direct3D 12, Windows only.
    Dx12,
    /// Metal, macOS and iOS only.
    Metal,
    /// OpenGL / OpenGL ES, everywhere and last.
    Gl,
}

impl Backend {
    /// Every value `--backend` accepts, in the order the usage message lists them.
    ///
    /// Two of wgpu's backends are deliberately absent. `BROWSER_WEBGPU` needs a `wasm32` target,
    /// which this program has none of; `NOOP` is compiled only under a wgpu feature this build
    /// does not enable and refuses to initialise without a second opt-in, so naming it would
    /// offer a choice that cannot be honoured — which is the trap `Device::adapter_names_on`
    /// exists to close one level down.
    const ALL: [Self; 4] = [Self::Vulkan, Self::Dx12, Self::Metal, Self::Gl];

    /// What a person types after `--backend`.
    fn name(self) -> &'static str {
        match self {
            Self::Vulkan => "vulkan",
            Self::Dx12 => "dx12",
            Self::Metal => "metal",
            Self::Gl => "gl",
        }
    }

    /// The value `--backend` names, or `None` for a word that is not one of them.
    fn parse(word: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|backend| backend.name() == word)
    }

    /// The one-backend set this restricts an instance to.
    fn backends(self) -> quorra_gpu::wgpu::Backends {
        match self {
            Self::Vulkan => quorra_gpu::wgpu::Backends::VULKAN,
            Self::Dx12 => quorra_gpu::wgpu::Backends::DX12,
            Self::Metal => quorra_gpu::wgpu::Backends::METAL,
            Self::Gl => quorra_gpu::wgpu::Backends::GL,
        }
    }
}

/// The backend this program asks for when nobody said, on Windows: **DX12**.
///
/// A choice this project now makes, where it used to be wgpu's hub order making it — which puts
/// Vulkan ahead of DX12 among adapters of equal rank, and is how the project owner's machine
/// reached an Intel Vulkan driver that crashed it. The argument is in ADR 0221, and the honest
/// part of it is that **no machine in this project runs Windows**, so this default is reasoned
/// rather than measured. `--backend vulkan` overrides it, and a machine that has no DX12 adapter
/// falls back to every backend with a note rather than refusing to start.
#[cfg(windows)]
const DEFAULT_BACKEND: Option<Backend> = Some(Backend::Dx12);

/// No default anywhere else: one driver stack is the norm, and where there are two the platform's
/// own ranking is the one this project has evidence for. See the Windows arm above.
#[cfg(not(windows))]
const DEFAULT_BACKEND: Option<Backend> = None;

/// How far a touchpad must be dragged under Ctrl for one zoom step.
///
/// A choice, not a derivation: a notch of a mouse wheel is one step by construction and a
/// touchpad reports a stream of pixels instead, so something has to say how many of them a notch
/// is worth. Fifty is about a finger's width on this machine's touchpad and gives roughly the
/// same number of steps per gesture as the wheel does per flick.
const WHEEL_ZOOM_PIXELS: f32 = 50.0;

/// When each milestone on the launch path was reached, from this process's own first instruction.
///
/// **`CLAUDE.md` makes this a first-class number.** Page one goes to the graphics device by the
/// project owner's decision, so creating that device and compiling its pipelines is *part of*
/// time-to-first-page — "a number to measure and to keep small". The per-step durations `--trace`
/// already printed cannot say that: each says what one step cost, and a launch is the sum of the
/// steps plus everything nobody thought to time. One `Instant`, taken before the arguments are
/// parsed, and one mark per step, so the difference between two marks is a step nobody named.
struct Launch {
    /// Taken as the first statement of `main`, so nothing before it is invisible.
    began: std::time::Instant,
    /// Each milestone and how long after `began` it was reached.
    marks: Vec<(&'static str, std::time::Duration)>,
    /// Whether the timeline has been printed, which happens once, at the first present.
    reported: bool,
}

impl Launch {
    /// Starts the clock. The first statement of `main`.
    fn new() -> Self {
        Self {
            began: std::time::Instant::now(),
            marks: Vec::new(),
            reported: false,
        }
    }

    /// Records that `step` has just finished.
    fn mark(&mut self, step: &'static str) {
        self.marks.push((step, self.began.elapsed()));
    }

    /// The first frame has reached the window: closes the timeline and, under `--trace`, prints it.
    ///
    /// Two columns because two questions are asked of a launch: *when did this step finish*,
    /// which is what a person waiting sees, and *what did it cost*, which is what a regression
    /// shows up in. Neither is derivable from the other once steps are added or reordered.
    fn arrived(&mut self, trace: Trace) {
        if self.reported {
            return;
        }
        self.reported = true;
        self.marks.push(("first present", self.began.elapsed()));
        if !trace.on(Topic::Launch) {
            return;
        }
        trace.say(
            Topic::Launch,
            format_args!("launch path, process start to first present:"),
        );
        let mut previous = std::time::Duration::ZERO;
        for (step, at) in &self.marks {
            trace.more(
                Topic::Launch,
                format_args!(
                    "{step:<22} {:8.3} ms  (+{:.3})",
                    at.as_secs_f64() * 1e3,
                    at.saturating_sub(previous).as_secs_f64() * 1e3
                ),
            );
            previous = *at;
        }
    }
}

/// What a trace line is *about*.
///
/// **The answer to "a verbosity that is not all-or-nothing", and it is a set rather than a
/// level** — ADR 0227 has the argument. The short of it: these seven are not ordered. A person
/// chasing a slow frame wants `frames` and nothing else; a person chasing a window that never
/// appears wants `launch` and `window`; a level would have to decide which of those is "more
/// verbose" than the other, and there is no such fact. 285 of the 1490 lines of the trace that
/// raised that ADR were pointer moves, and `--trace=-pointer` is the whole of what that person
/// needed to type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Topic {
    /// The launch timeline, the graphics device's bring-up, and the line that closes pipeline
    /// compilation.
    Launch,
    /// One line per frame with its stages, and the summary at exit.
    Frames,
    /// Commands handed to `viewer-core` and the events that came back — pointer moves excepted,
    /// because they are the flood.
    Events,
    /// Window events from winit, the pointer's excepted for the same reason.
    Window,
    /// Pointer movement, both the window event and the command it becomes.
    Pointer,
    /// The accessibility bridge and what it publishes.
    Access,
    /// How many shapes a selection is, which is the number `doc/todo/13` turned on.
    Selection,
}

impl Topic {
    /// Every topic, in the order `--trace=?` lists them.
    const ALL: [Self; 7] = [
        Self::Launch,
        Self::Frames,
        Self::Events,
        Self::Window,
        Self::Pointer,
        Self::Access,
        Self::Selection,
    ];

    /// What a person types after `--trace=`.
    fn name(self) -> &'static str {
        match self {
            Self::Launch => "launch",
            Self::Frames => "frames",
            Self::Events => "events",
            Self::Window => "window",
            Self::Pointer => "pointer",
            Self::Access => "access",
            Self::Selection => "selection",
        }
    }

    /// This topic's place in [`Trace::topics`].
    fn bit(self) -> u16 {
        match self {
            Self::Launch => 1,
            Self::Frames => 1 << 1,
            Self::Events => 1 << 2,
            Self::Window => 1 << 3,
            Self::Pointer => 1 << 4,
            Self::Access => 1 << 5,
            Self::Selection => 1 << 6,
        }
    }

    /// The topic a word names, or `None` for a word that is not one.
    fn parse(word: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|topic| topic.name() == word)
    }
}

/// Every topic at once: what a bare `--trace` asks for, and what `all` names.
const EVERY_TOPIC: u16 = 0x7f;

/// Whether to say what is happening, about what, and since when.
///
/// `Copy`, deliberately: this is read from inside methods that already hold `&mut self`, and a
/// borrowing instrument would have to be argued with at every call site.
#[derive(Debug, Clone, Copy)]
struct Trace {
    /// The set of [`Topic`] bits `--trace` asked for; zero when it was not given at all.
    topics: u16,
    /// `main`'s first `Instant`, so that every line can carry a clock.
    ///
    /// **ADR 0227**: every line of the trace that raised it carried a *duration* and
    /// none carried a time, so the interval a person actually waited could not be recovered and
    /// a gap in the log could not be told from a run of cheap work. One `Instant::elapsed` per
    /// line — tens of nanoseconds against a `println!` that costs microseconds.
    began: std::time::Instant,
}

impl Trace {
    /// Nothing is traced, which is what a run without the flag gets — and what every run starts
    /// as, since the flag may appear anywhere on the command line.
    fn off(began: std::time::Instant) -> Self {
        Self { topics: 0, began }
    }

    /// Whether anything at all is traced, which is what decides the graphics stack's own voice.
    fn any(self) -> bool {
        self.topics != 0
    }

    /// Whether lines about `topic` are wanted.
    fn on(self, topic: Topic) -> bool {
        self.topics & topic.bit() != 0
    }

    /// One line, stamped with the seconds since this process started.
    ///
    /// The check is repeated here even where the caller has already made it, because the caller
    /// makes it to avoid *formatting* the arguments and this makes it to avoid printing them —
    /// a call site that forgot the first is quiet rather than wrong.
    fn say(self, topic: Topic, what: std::fmt::Arguments<'_>) {
        if !self.on(topic) {
            return;
        }
        println!("trace: {:9.3} {what}", self.began.elapsed().as_secs_f64());
    }

    /// A continuation of the line above: indented under the message column, and carrying no
    /// clock of its own because it happened at the same moment.
    fn more(self, topic: Topic, what: std::fmt::Arguments<'_>) {
        if !self.on(topic) {
            return;
        }
        println!("trace: {:9}   {what}", "");
    }
}

/// The set of topics a `--trace` argument asks for, or the word that named none of them.
///
/// Empty asks for everything, which is what a bare `--trace` has always meant and what the
/// project owner's own invocation types. A list that *starts* with a subtraction is read as
/// "everything except", because `--trace=-pointer` can mean nothing else and demanding
/// `--trace=all,-pointer` would be a rule with no purpose but to be remembered.
fn parse_topics(list: &str) -> Result<u16, String> {
    if list.is_empty() {
        return Ok(EVERY_TOPIC);
    }
    let mut topics = if list.starts_with('-') {
        EVERY_TOPIC
    } else {
        0
    };
    for word in list.split(',') {
        let (word, remove) = match word.strip_prefix('-') {
            Some(rest) => (rest, true),
            None => (word, false),
        };
        if word == "all" {
            topics = if remove { 0 } else { EVERY_TOPIC };
            continue;
        }
        let Some(topic) = Topic::parse(word) else {
            return Err(word.to_owned());
        };
        if remove {
            topics &= !topic.bit();
        } else {
            topics |= topic.bit();
        }
    }
    Ok(topics)
}

/// The topics `--trace=` accepts, for a message that has to list them.
fn topic_names() -> String {
    Topic::ALL
        .iter()
        .map(|topic| topic.name())
        .collect::<Vec<_>>()
        .join(", ")
}

/// How many frames the summary keeps samples of.
///
/// A bound rather than a `Vec` that grows for as long as the program runs: `CLAUDE.md`'s third
/// principle asks for explicit memory budgets, and this one is 16 384 × 128 bytes ≈ 2 MB. The
/// *count* is not bounded, and the summary says outright when its percentiles came from the
/// first 16 384 of more frames than that rather than quietly reporting a prefix as the whole.
const FRAME_SAMPLES: usize = 16_384;

/// What one frame spent, in the stages a host can separate.
///
/// **ADR 0227**: `present -> presented in T` was one number over four different
/// things. These are the four, plus the two the same item's second point took *out* of the
/// frame — the accessibility publication and the launch timeline are not the frame's cost and
/// were being reported as though they were.
#[derive(Debug, Clone, Copy, Default)]
struct Stages {
    /// The page this frame drew, counting from one.
    page: usize,
    /// How many display-list commands the page carried.
    commands: usize,
    /// This host's own work before anything is handed over: the page's geometry, the selection,
    /// the focus ring, the caret, the popup windows and the panel, each a query into the core.
    host: std::time::Duration,
    /// What `render-quorra` reported for the same frame, which is where the device's own
    /// accounting arrives from.
    gpu: render_quorra::FrameCost,
    /// `render-cpu` rasterising the page and presenting it, on a frame the device refused.
    fallback: std::time::Duration,
    /// The accessibility publication — measured *beside* the frame rather than inside it.
    attend: std::time::Duration,
    /// The whole of `redraw_requested`, which is what the old single number was.
    total: std::time::Duration,
}

/// Every frame this run has drawn, for the summary at exit.
#[derive(Debug, Default)]
struct FrameLog {
    /// The samples, capped at [`FRAME_SAMPLES`].
    samples: Vec<Stages>,
    /// How many frames there were, which is not `samples.len()` past the cap.
    count: usize,
    /// Whether the line explaining the frame line's columns has been printed.
    legend: bool,
    /// Whether the closing line for pipeline compilation has been printed.
    pipelines: bool,
}

/// How to read one stage out of one frame.
type StageOf = fn(&Stages) -> std::time::Duration;

/// The summary's rows: what to call each stage and how to read it.
///
/// One table rather than nine `format!`s, so that a stage added later cannot appear in the
/// per-frame line and go missing from the percentiles.
const SUMMARY_ROWS: [(&str, StageOf); 10] = [
    ("frame", |frame| frame.total),
    ("host", |frame| frame.host),
    ("scene", |frame| frame.gpu.scene),
    ("device", |frame| frame.gpu.device),
    ("  encode", |frame| frame.gpu.encode),
    ("  transfer", |frame| frame.gpu.upload),
    ("  execute", |frame| frame.gpu.execute),
    // What is inside `Device::render` and outside the three phases it names: acquiring the
    // swapchain texture, presenting it, and reading the timestamp queries back. Printed as its
    // own row rather than left for a reader to subtract, because an unnamed remainder is where
    // a cost hides — and on this machine it is a sixth of the frame.
    //
    // **A bound rather than a duration**, and the summary says so: where `execute` came from the
    // adapter's timestamp queries this subtracts a device clock from a host one, so the
    // remainder also carries whatever the two disagree by (ADR 0228, `QUORRA_FEEDBACK.md` section 13).
    ("  elsewhere", |frame| {
        frame.gpu.device.saturating_sub(
            frame
                .gpu
                .encode
                .saturating_add(frame.gpu.upload)
                .saturating_add(frame.gpu.execute),
        )
    }),
    ("settle", |frame| frame.gpu.settle),
    ("fallback", |frame| frame.fallback),
];

impl FrameLog {
    /// Says what one frame cost, and keeps it for the summary.
    fn frame(&mut self, trace: Trace, stages: &Stages, outcome: &str) {
        // Nothing is *kept* unless somebody asked: the durations are gathered either way, at the
        // few hundred nanoseconds `Stages` documents, but a run with no `--trace` must not also
        // be carrying two megabytes of samples for a summary it will never print.
        if !trace.on(Topic::Frames) {
            return;
        }
        self.count = self.count.saturating_add(1);
        if self.samples.len() < FRAME_SAMPLES {
            self.samples.push(*stages);
        }
        if !self.legend {
            self.legend = true;
            Self::legend(trace);
        }
        // One line per frame, and it stays one line: the stages are *in* it, and the two that
        // only a strange frame has — the processor drawing what the device refused, and the
        // accessibility publication that happens on a page turn — are appended only when they
        // are not zero. An ordinary frame is therefore no longer than the two lines this
        // replaced (ADR 0227).
        let mut unusual = String::new();
        for (name, spent) in [("fallback", stages.fallback), ("attend", stages.attend)] {
            if spent > std::time::Duration::ZERO {
                unusual = format!("{unusual} {name} {:.1}", ms(spent));
            }
        }
        trace.say(
            Topic::Frames,
            format_args!(
                "frame p{} {}cmd {outcome} {:.1} | host {:.1} scene {:.1} device {:.1} \
                 settle {:.1}{unusual} | {} up, {} culled",
                stages.page,
                stages.commands,
                ms(stages.total),
                ms(stages.host),
                ms(stages.gpu.scene),
                ms(stages.gpu.device),
                ms(stages.gpu.settle),
                stages.gpu.uploads,
                stages.gpu.commands_culled,
            ),
        );
    }

    /// What the frame line's columns mean, once, before the first of them.
    fn legend(trace: Trace) {
        trace.say(
            Topic::Frames,
            format_args!(
                "frame lines: page, display-list commands, outcome, the whole frame in ms, then"
            ),
        );
        for (column, meaning) in [
            (
                "host",
                "this host's queries — page geometry, selection, focus, caret, popups, panel",
            ),
            (
                "scene",
                "display lists translated into a GPU scene, this frame's uploads included",
            ),
            (
                "device",
                "quorra's render: encoding, transfers, and the passes it already waits on",
            ),
            (
                "settle",
                "the frame's transient resources released and settled cache entries evicted",
            ),
            (
                "fallback",
                "render-cpu drawing a frame the device refused — absent when there was none",
            ),
            (
                "attend",
                "the accessibility publication, beside the frame and not inside its total",
            ),
            (
                "up",
                "resources handed to the device; culled: commands that reached no pixel",
            ),
        ] {
            trace.more(Topic::Frames, format_args!("{column:<9} {meaning}"));
        }
    }

    /// The percentiles, once, when the program exits.
    ///
    /// **This is where percentiles belong** (ADR 0227): "why did it feel slow" is a question
    /// about a distribution, and a distribution costs nothing per frame but cannot be read off
    /// one. Nearest rank rather than an interpolated percentile, so every figure printed is a
    /// frame that actually happened.
    fn summary(&self, trace: Trace) {
        if !trace.on(Topic::Frames) || self.count == 0 {
            return;
        }
        let truncated = if self.count > self.samples.len() {
            format!(", percentiles over the first {}", self.samples.len())
        } else {
            String::new()
        };
        trace.say(
            Topic::Frames,
            format_args!("{} frame(s){truncated}, milliseconds:", self.count),
        );
        trace.more(
            Topic::Frames,
            format_args!(
                "{:<11}{:>9}{:>9}{:>9}{:>10}",
                "", "median", "p90", "max", "sum"
            ),
        );
        for (name, read) in SUMMARY_ROWS {
            let mut values: Vec<std::time::Duration> = self.samples.iter().map(read).collect();
            values.sort_unstable();
            let sum = values.iter().fold(std::time::Duration::ZERO, |total, one| {
                total.saturating_add(*one)
            });
            trace.more(
                Topic::Frames,
                format_args!(
                    "{name:<11}{:>9.1}{:>9.1}{:>9.1}{:>10.1}",
                    ms(at_rank(&values, 1, 2)),
                    ms(at_rank(&values, 9, 10)),
                    ms(values.last().copied().unwrap_or_default()),
                    ms(sum)
                ),
            );
        }
        let uploads: u64 = self
            .samples
            .iter()
            .map(|frame| u64::from(frame.gpu.uploads))
            .sum();
        let most = self
            .samples
            .iter()
            .map(|frame| frame.gpu.uploads)
            .max()
            .unwrap_or(0);
        let measured = self.samples.iter().any(|frame| frame.gpu.execute_measured);
        trace.more(
            Topic::Frames,
            format_args!(
                "{uploads} resource upload(s), {most} in the busiest frame; execute is {}",
                if measured {
                    "the device's own timestamps"
                } else {
                    "a wall clock — this adapter offers no timestamp queries"
                }
            ),
        );
        // **Said out loud because the row is arithmetic on two clocks** (ADR 0228). `elsewhere`
        // is `device` minus the three phases quorra names, and where `execute` came from
        // timestamp queries it is a *device* duration being subtracted from a *host* wall clock:
        // the host's wait for a submitted frame is not the GPU's own measure of the passes in
        // it, so the remainder carries whatever the two clocks disagree by as well as the
        // acquire, the present and the readback. It is a bound on the unnamed cost rather than a
        // measurement of it, and it is quorra's to subdivide — `doc/QUORRA_FEEDBACK.md` section 13.
        if measured {
            trace.more(
                Topic::Frames,
                format_args!(
                    "elsewhere is device minus the three phases, so it also carries whatever \
                     the host's clock and the adapter's disagree by — a bound, not a duration"
                ),
            );
        }
    }
}

/// A duration in milliseconds, which is the unit every trace line is in.
fn ms(duration: std::time::Duration) -> f64 {
    duration.as_secs_f64() * 1e3
}

/// The value `numerator/denominator` of the way through a sorted sample, by nearest rank.
///
/// Integer arithmetic throughout: the workspace forbids unchecked arithmetic, and a percentile
/// computed in floating point would have to justify its rounding as well as its rank.
fn at_rank(
    sorted: &[std::time::Duration],
    numerator: usize,
    denominator: usize,
) -> std::time::Duration {
    let rank = sorted
        .len()
        .saturating_mul(numerator)
        .saturating_add(denominator.saturating_sub(1))
        .checked_div(denominator)
        .unwrap_or(0);
    let index = rank
        .max(1)
        .saturating_sub(1)
        .min(sorted.len().saturating_sub(1));
    sorted.get(index).copied().unwrap_or_default()
}

/// What the command line asked for.
struct Arguments {
    /// The document to open.
    path: PathBuf,
    /// What to say about what is happening, from `--trace` and `--trace=<topics>`.
    trace: Trace,
    /// Whether to draw with `render-cpu` rather than the graphics device, from `--cpu`.
    ///
    /// **And therefore whether a graphics device is created at all**, since the
    /// three-hundred-and-eighty-fourth session: this flag now decides the presenter as well as
    /// the rasteriser, so a run that asks for the processor opens no driver. See ADR 0221.
    processor: bool,
    /// The driver stack `--backend` named, or [`DEFAULT_BACKEND`] where it did not.
    backend: Option<Backend>,
    /// Whether [`Arguments::backend`] is a person's answer or this program's default, which is
    /// what decides between a refusal and a fallback when no adapter matches it.
    backend_asked_for: bool,
    /// The page `--page` named, counting from one.
    opens_at: Option<usize>,
    /// Annex O's fragment identifier, where the argument carried one after a `#`.
    fragment: Option<String>,
    /// What this reader does with the restrictions a document asserts, from
    /// `--ignore-restrictions`.
    ///
    /// **Not a user interface for them**, which `CLAUDE.md` says is not to be built yet: it is
    /// the one policy value `viewer-core` asks for, supplied the way this host supplies every
    /// other one it has — the sandbox, the backend, the page to open at. The four levels the
    /// project owner named, and the menu that will offer them, are later.
    restrictions: RestrictionLevel,
}

/// Reads the command line, applies the two settings that must be applied before anything opens a
/// document, and exits where it cannot.
///
/// Separate from `main` because the sandbox decision is one of them: it decides *where* this
/// document's images are decoded, and a policy applied halfway through is not a policy.
fn arguments(began: std::time::Instant) -> Arguments {
    let mut path = None;
    let mut sandbox = true;
    let mut trace = Trace::off(began);
    let mut processor = false;
    let mut backend = DEFAULT_BACKEND;
    let mut backend_asked_for = false;
    let mut opens_at = None;
    let mut restrictions = RestrictionLevel::On;
    let mut arguments = std::env::args_os().skip(1);
    while let Some(argument) = arguments.next() {
        if argument == "--licences" || argument == "--licenses" {
            print!("{NOTICE}");
            std::process::exit(0);
        } else if argument == "--no-sandbox" {
            sandbox = false;
        } else if argument == "--trace" || argument.to_string_lossy().starts_with("--trace=") {
            // `--trace=<topics>` rather than `--trace <topics>`, because the flag has taken no
            // value for a hundred sessions and a document is what follows it: `--trace doc.pdf`
            // must keep meaning what it always meant, and only the equals form can promise that.
            let list = argument.to_string_lossy();
            let list = list.split_once('=').map_or("", |(_, rest)| rest);
            match parse_topics(list) {
                Ok(topics) => trace.topics = topics,
                Err(word) => {
                    eprintln!(
                        "--trace={word}: not a topic. One of: {}, or all — each optionally \
                         prefixed with - to leave it out.",
                        topic_names()
                    );
                    std::process::exit(2);
                }
            }
            // The graphics stack's own voice, which is silent until something receives it.
            if trace.any() {
                speak_up();
            }
        } else if argument == "--cpu" {
            processor = true;
        } else if argument == "--backend" {
            // Refused here rather than carried as a string and refused later: a word that names
            // no backend is a typing mistake, and the list of what would have worked is a better
            // answer than a launch that quietly ignored the flag.
            let Some(word) = arguments.next() else {
                eprintln!("--backend wants one of: {}", backend_names());
                std::process::exit(2);
            };
            let Some(named) = Backend::parse(&word.to_string_lossy()) else {
                eprintln!(
                    "--backend {}: not a backend. One of: {}",
                    word.to_string_lossy(),
                    backend_names()
                );
                std::process::exit(2);
            };
            backend = Some(named);
            backend_asked_for = true;
        } else if argument == "--ignore-restrictions" {
            restrictions = RestrictionLevel::Off;
        } else if argument == "--page" {
            // A page number as the title bar shows it, which is one-based. §12.3.2.1's
            // `/OpenAction` is the document's own answer to the same question and wins where
            // this is absent; where both are stated, the person asking now wins.
            opens_at = arguments
                .next()
                .and_then(|value| value.to_string_lossy().parse::<usize>().ok())
                .filter(|page| *page > 0);
            if opens_at.is_none() {
                eprintln!("--page wants a page number, counting from 1");
                std::process::exit(2);
            }
        } else if path.is_none() {
            path = Some(argument);
        } else {
            eprintln!("unexpected argument: {}", argument.to_string_lossy());
            std::process::exit(2);
        }
    }
    let Some(argument) = path else {
        usage();
        std::process::exit(2);
    };
    let (path, fragment) = split_fragment(&argument);

    // What this *build* can confine, said before anything is opened, because it is a fact
    // about the executable rather than about a document and a person choosing a viewer for
    // untrusted files deserves it in the first line rather than in a release note. Linux has
    // seccomp-BPF and Landlock; the other two platforms get the worker process and no kernel
    // confinement, which is a decision with an argument (ADR 0194) rather than an omission.
    if !pdf_sandbox::lockdown::ENFORCED_BY_THIS_BUILD {
        println!(
            "note: this build has no kernel confinement for the image decoder — seccomp-BPF \
             and Landlock are Linux interfaces. JBIG2 and JPEG 2000 are still decoded in a \
             separate process, so a decoder failure costs one image rather than the viewer, \
             and there is no address-space ceiling on that process."
        );
    }

    // And what this build can hand a screen reader, said in the same breath and for the same
    // reason: AT-SPI is Linux's, AccessKit's other two adapters are not wired in here, and a
    // build that quietly exposed nothing would look exactly like one whose bridge is broken.
    // ADR 0194's precedent, one interface over. ADR 0214.
    if let Some(missing) = viewer_accessibility::Bridge::shortfall() {
        println!("note: {missing}");
    }

    if !sandbox {
        pdf_sandbox::set_isolation(pdf_sandbox::Isolation::InProcess);
        // Said out loud, once, on the way past. Turning the sandbox off is a reasonable choice
        // for documents you produced yourself and a bad one for documents that arrived by
        // email, and the difference is not visible from inside the program.
        println!(
            "note: --no-sandbox — JBIG2 and JPEG 2000 will be decoded in this process, with no \
             memory ceiling, and a decoder failure will take the viewer down with it"
        );
    }

    Arguments {
        path,
        trace,
        processor,
        backend,
        backend_asked_for,
        opens_at,
        fragment,
        restrictions,
    }
}

/// The thread that creates the graphics instance, or `None` where this run will not have one.
///
/// **A function rather than three lines in `main`, so that the promise `--cpu` makes has a
/// test.** Creating a `wgpu::Instance` *is* loading the driver — it is where quorra measured
/// roughly 80% of what bring-up blocks for, and it is where the project owner's machine crashed —
/// so a flag that means "no graphics device" has to mean this thread is not spawned. The
/// difference between a flag that chooses a rasteriser and a flag that avoids a driver is exactly
/// the `None` below, and `cpu_creates_no_graphics_instance` is what keeps it there.
fn spawn_instancing(
    processor: bool,
    backend: Option<Backend>,
) -> Option<std::thread::JoinHandle<quorra_gpu::wgpu::Instance>> {
    (!processor).then(|| {
        std::thread::spawn(move || match backend {
            Some(named) => QuorraPresenter::instance_with(named.backends()),
            None => QuorraPresenter::instance(),
        })
    })
}

/// The backends `--backend` accepts on this build, for a message that has to list them.
fn backend_names() -> String {
    Backend::ALL
        .iter()
        .map(|backend| backend.name())
        .collect::<Vec<_>>()
        .join(", ")
}

/// Splits `document.pdf#page=5` into the file and ISO 32000-2 Annex O's fragment identifier.
///
/// **The filesystem decides, not the punctuation**, and that is this host's choice rather than
/// anything the annex says. A `#` is an ordinary character in a file name on every system this
/// program runs on, so an argument that names an existing file is taken whole; only when it does
/// not is it read as a URI-shaped reference and split at its first `#`, which is where RFC 3986
/// puts the boundary. The cost is one `stat` on the launch path and a file called `a#b.pdf` that
/// still opens; the alternative — splitting first — makes that file unopenable and says nothing.
///
/// `viewer-core` never sees this decision: what crosses is the fragment alone, undecoded, because
/// splitting a URI is the host's job and percent-decoding belongs to whoever knows which component
/// it is decoding.
fn split_fragment(argument: &std::ffi::OsStr) -> (PathBuf, Option<String>) {
    let whole = PathBuf::from(argument);
    if whole.exists() {
        return (whole, None);
    }
    let text = argument.to_string_lossy();
    match text.split_once('#') {
        Some((path, fragment)) if !path.is_empty() => {
            (PathBuf::from(path), Some(fragment.to_owned()))
        }
        // No `#`, or nothing before it. Hand the whole thing on and let the read fail by name:
        // a path that does not exist is a better message than a fragment nobody asked for.
        _ => (whole, None),
    }
}

/// Reads the file and opens it, wherever this is called from.
///
/// Split out of `main` because it is called on a thread of its own — see the comment at the call
/// site — so it returns the viewer it made and the events it produced rather than touching an
/// `App` that is being built on another thread at the same time.
///
/// **Rule 2 lives here**: the host owns the filesystem, and this is the only place a path becomes
/// bytes.
fn open_document(
    path: &std::path::Path,
    opens_at: Option<usize>,
    fragment: Option<&str>,
    restrictions: RestrictionLevel,
) -> (Viewer, Vec<Event>) {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("cannot read {}: {error}", path.to_string_lossy());
            std::process::exit(1);
        }
    };
    // No viewport yet: the window does not exist and may not for another 50 ms. The core renders
    // nothing into one with no extent, which is exactly right — there is nothing to render into.
    let mut viewer = Viewer::new(0, 0, 1.0);
    // Before the document rather than after it, for the same reason the sandbox decision is made
    // before anything opens: a policy applied halfway through is not a policy. Nothing an open
    // does is restricted, so this is about where the value *belongs* rather than about an
    // operation it would otherwise miss.
    drop(viewer.handle(Command::Restrict(restrictions)));
    let mut events: Vec<Event> = viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes,
            password: None,
            fragment: fragment.map(str::to_owned),
        })
        .collect();
    // After the fragment rather than before it: Annex O's parameters are what the *URI* asked
    // for and `--page` is what the person at the keyboard asked for a moment later, so where
    // both name a page the second one wins.
    if let Some(page) = opens_at {
        let turned: Vec<Event> = viewer
            .handle(Command::GoTo(PageTarget::Index(page.saturating_sub(1))))
            .collect();
        events.extend(turned);
    }
    (viewer, events)
}

fn main() {
    let mut launch = Launch::new();
    let Arguments {
        path,
        trace,
        processor,
        backend,
        backend_asked_for,
        opens_at,
        fragment,
        restrictions,
    } = arguments(launch.began);
    launch.mark("arguments");

    // **The document opens on a thread of its own, and this is the launch path's one lever that
    // is ours.** What the window needs is a chain of three steps that must happen in order and on
    // this thread — an event loop, a window, a graphics device — and on this machine that chain
    // costs 50 to 90 ms (ADR 0179, ADR 0182). Reading a document depends on none of it, and costs
    // 21 ms on ISO 32000-2's 101 318 objects. Done one after the other those are 110 ms; done side
    // by side they are the longer of the two.
    //
    // `viewer-core`'s rule 4 is "no threads the core was not handed", and this keeps it: the core
    // is *made* on that thread and moved back, so it is still single-threaded and still owns none
    // of its own scheduling. What crosses is a `Viewer` and a `Vec<Event>`, both `Send`.
    let opening = std::thread::spawn({
        let path = path.clone();
        let fragment = fragment.clone();
        move || open_document(&path, opens_at, fragment.as_deref(), restrictions)
    });

    // **And the graphics instance on a second thread**, since the two-hundred-and-eighty-eighth:
    // a `wgpu::Instance` is the driver loader, it needs no window either, and quorra measured it
    // at roughly 80% of what bringing a device up blocks for (their ADR 0014, answering
    // `doc/QUORRA_FEEDBACK.md` section 8.2). Its own thread rather than the document's, and the
    // difference is not style: the device *needs* the instance and does not need the document, so
    // the instance is joined before the presenter is built and the document after it — one thread
    // for both would make the first join wait for the second's work.
    //
    // **Not under `--cpu`, and that is the whole of the flag's new meaning.** Creating the
    // instance *is* loading the driver, so a run that will not draw on the device must not make
    // one: the thread is not spawned, `resumed` builds no presenter, and nothing in the process
    // opens an ICD. Before the three-hundred-and-eighty-fourth session this line ran regardless
    // and a driver that faulted while loading took `--cpu` down with it (ADR 0221).
    let instancing = spawn_instancing(processor, backend);

    let chrome = match Chrome::new() {
        Ok(chrome) => Some(chrome),
        Err(problem) => {
            eprintln!("note: no panel: {problem}");
            None
        }
    };
    launch.mark("chrome fonts");

    let mut app = App {
        // No viewport until the window exists. The core renders nothing into one with no
        // extent, which is exactly right: there is nothing to render into yet.
        viewer: Viewer::new(0, 0, 1.0),
        title: path.to_string_lossy().into_owned(),
        path: PathBuf::from(&path),
        fragment,
        // §12.7.6.4's import-data action names a file, and this is the only place that name is
        // allowed to mean anything: a *sibling of the document being shown*. See `supply`.
        directory: std::path::Path::new(&path)
            .parent()
            .map(std::path::Path::to_path_buf),
        caption: String::new(),
        request: None,
        acknowledged: true,
        presented: None,
        presentation: None,
        arming: None,
        trace,
        processor,
        backend,
        backend_asked_for,
        cursor: (0.0, 0.0),
        dragging: false,
        control: false,
        shift: false,
        pinch: 0.0,
        dirty: false,
        attempts: 0,
        chrome,
        panel: Sidebar::default(),
        about: About::default(),
        find: FindBar::default(),
        pages_left: 0,
        outline: pdf_model::outline::Outline::default(),
        attachments: Vec::new(),
        articles: Vec::new(),
        collection: None,
        typing: None,
        clipboard: String::new(),
        drawing: None,
        pages: Vec::new(),
        information: pdf_model::metadata::Information::default(),
        metadata: None,
        state: None,
        opening: Some(opening),
        instancing,
        launch,
        frames: FrameLog::default(),
        accessibility: None,
        spoken: None,
    };

    let event_loop = EventLoop::new().expect("an event loop requires a display server");
    app.launch.mark("event loop");
    // Redraw on request rather than continuously: a document viewer is idle almost all the time,
    // and a spinning loop would drain a battery for nothing.
    event_loop.set_control_flow(ControlFlow::Wait);
    event_loop.run_app(&mut app).expect("event loop failed");
}

/// What the program does when it is given nothing to open.
fn usage() {
    eprintln!("usage: pdf-viewer [--no-sandbox] <document.pdf>");
    eprintln!("       pdf-viewer --licences");
    eprintln!();
    eprintln!("Arrows, Page Up/Down or Space turn pages; Home and End jump; + and - zoom;");
    eprintln!("o shows the sidebar — the outline, the layers and the embedded files;");
    eprintln!("? shows the third-party notices; drag to select text, a selects the page,");
    eprintln!("s saves, Escape quits.");
    eprintln!();
    eprintln!("  --no-sandbox  decode JBIG2 and JPEG 2000 images in this process rather than");
    eprintln!("                in a confined worker. Faster by a process spawn and a pipe");
    eprintln!("                round trip; appropriate only for documents you trust.");
    eprintln!("  --page N      open at page N, counting from 1 as the title bar does.");
    eprintln!("  doc.pdf#...   ISO 32000-2 Annex O's fragment identifier, which says where to");
    eprintln!("                open: page=5, nameddest=Chapter3, zoom=150,0,792, view=FitH,700,");
    eprintln!("                viewrect=..., comment=..., structelem=.... Parameters are");
    eprintln!("                separated by & and carried out left to right; whatever this");
    eprintln!("                program cannot do is named rather than ignored.");
    eprintln!("  --cpu         draw with the processor rather than the graphics device, and open");
    eprintln!("                no graphics driver at all: no instance, no adapter, no device. The");
    eprintln!("                page reaches the window through a software surface instead.");
    eprintln!("                Slower, and the same rasteriser the reference comparison is built");
    eprintln!("                on: a page that appears with this and not without it is the");
    eprintln!("                device's, and so is a launch that only works with it.");
    eprintln!("  --backend B   which driver stack talks to the GPU, not which GPU: vulkan, dx12,");
    eprintln!("                metal or gl. What to reach for when one stack on this machine is");
    eprintln!("                broken and another is not. Refused, rather than quietly ignored,");
    eprintln!("                where this machine has no adapter behind the one named.");
    eprintln!("  --trace       print every command, every event and every frame, each line");
    eprintln!("                stamped with the seconds since this process started, and");
    eprintln!("                whatever the graphics stack has to say. What to run when a page");
    eprintln!("                will not appear: the last line printed is the step that did not");
    eprintln!("                finish. A frame's line names its stages — host, scene, device,");
    eprintln!("                settle — and the percentiles over every frame are printed on the");
    eprintln!("                way out. PDFVIEWER_LOG=error|warn|info|debug sets how much of");
    eprintln!(
        "                the graphics stack's own logging comes with it, defaulting to warn."
    );
    eprintln!("  --trace=T,U   only the topics named, of: launch, frames, events, window,");
    eprintln!("                pointer, access, selection — or all. A topic prefixed with -");
    eprintln!("                is left out, and a list that starts with one means everything");
    eprintln!("                else: --trace=frames to chase a slow page, --trace=-pointer for");
    eprintln!("                everything but the flood a moving mouse makes.");
    eprintln!("  --ignore-restrictions");
    eprintln!("                perform an operation a document says its reader may not — filling");
    eprintln!("                in a field under §7.6.4.2's permission flags or an author's");
    eprintln!("                §12.8.2.2 certification. The default is to obey and say so.");
    eprintln!("  --licences    print the third-party notices this binary carries, and exit.");
}

/// How many passwords a person is asked for before the program gives up.
///
/// §7.6.4.1 states no limit — it says a processor tries the empty password and then prompts —
/// so this is a choice about a terminal rather than about the clause, and an empty line cancels
/// before it is reached.
const PASSWORD_ATTEMPTS: usize = 3;

#[expect(
    clippy::struct_excessive_bools,
    reason = "independent facts about a window, each read in one place: whether a button is \
              down, which modifiers are held, whether the core has been told about the last \
              frame, whether anything is unsaved, and what this run asked of the graphics stack"
)]
struct App {
    /// Everything about documents, pages and clicks.
    viewer: Viewer,
    /// The file's name, for the title bar.
    title: String,
    /// The file itself, kept because §7.6.4.1's prompt has to open it again with a password.
    ///
    /// The path rather than the bytes: a host that held a copy of every document it had failed
    /// to open would be holding a copy of every document.
    path: PathBuf,
    /// Annex O's fragment identifier, kept for the same reason as the path.
    ///
    /// A document that asked for a password and got one is opened a second time, and the URI that
    /// named it said `#page=5` both times.
    fragment: Option<String>,
    /// The directory the open document is in, where one can be named.
    ///
    /// The whole of this program's answer to "which files may a document ask for". §12.7.6.4's
    /// import-data action carries a file specification the *document* wrote, so honouring it
    /// unrestricted would let a PDF read any path this process can — and the clause states no
    /// policy, because a policy is a property of the processor. See [`App::supply`].
    directory: Option<PathBuf>,
    /// What the title bar says about the page, from the last `PageChanged`.
    caption: String,
    /// The render `viewer-core` last asked for, kept so an expose can redraw it.
    request: Option<RenderRequest>,
    /// Whether the core has been told that request was drawn.
    acknowledged: bool,
    /// The page and placement last put on the screen, for §12.4.4's transition to draw from.
    ///
    /// **The page being left, exactly where it was.** A transition is a picture between two
    /// pages, and by the time `Event::Transition` arrives the core has already moved on — so the
    /// outgoing page is this, kept at the moment it was presented rather than reconstructed
    /// afterwards from a geometry that now describes the page arriving.
    presented: Option<(Arc<pdf_render::DisplayList>, TargetSpec)>,
    /// §12.4.4's presentation, while `p` has one running. `None` is a window reading a document.
    presentation: Option<Presentation>,
    /// A transition named but not yet begun, waiting for the arriving page's own display list.
    ///
    /// See [`App::arm_transition`]: the core settles after the page turn, so the render request
    /// for the page being moved to arrives one event *after* the transition that names it.
    arming: Option<pdf_model::navigation::Transition>,
    /// Where the pointer last was, in device pixels.
    ///
    /// `winit` reports movement and clicks as separate events, so a click needs the position
    /// remembered from the last `CursorMoved` — the click itself carries none.
    cursor: (f64, f64),
    /// What to say about what is happening, from `--trace`.
    ///
    /// A viewer that will not draw a page has to be able to say how far it got, and the four
    /// steps between a key press and a frame — command, interpretation, draw, present — are
    /// invisible from outside the process. This makes them visible, in order, with a duration
    /// apiece and a clock; the *last line printed* is the step that did not finish.
    trace: Trace,
    /// Every frame this run has drawn, for the summary printed when it exits.
    frames: FrameLog,
    /// Whether to draw with `render-cpu` rather than the graphics device, from `--cpu`.
    ///
    /// The same rasteriser the reference oracle is built on, and the same one that draws a page
    /// the device refuses. As a *flag* it is a diagnostic a person can pull without a debugger:
    /// if a page appears under `--cpu` and not without it, the difference is the device.
    ///
    /// **And it now decides whether there is a device at all.** A run with this flag creates no
    /// instance, selects no adapter and makes no device, and presents through
    /// [`SoftwareSurface`] instead — so it is also the answer to a driver that will not load,
    /// which is what the project owner needed it to be and what it was not. ADR 0221.
    processor: bool,
    /// The driver stack asked for, from `--backend` or from [`DEFAULT_BACKEND`].
    ///
    /// Kept past `main` for one job: `resumed` needs it to say what was asked for when no adapter
    /// matched, and to decide between refusing and falling back.
    backend: Option<Backend>,
    /// Whether a person named [`App::backend`] or this program defaulted to it.
    ///
    /// **A backend a person named is honoured or refused; one this program chose gives way.** A
    /// machine that has no adapter for the flag is a question with an answer — "this machine has
    /// no such adapter, here is what it has" — and answering it beats starting on a stack the
    /// person was trying to avoid. A machine that has no adapter for the *default* is a machine
    /// this project guessed wrong about, and it gets a note and every backend.
    backend_asked_for: bool,
    /// Whether the button is down, which is what separates a move from a drag.
    dragging: bool,
    /// Whether Ctrl is held, which is what separates a wheel scroll from a wheel zoom.
    ///
    /// `winit` reports a modifier change as its own event and puts nothing in the wheel's, so a
    /// host that wants to know has to remember. Ctrl + wheel is a convention rather than a
    /// clause, and it is the one every desktop viewer has converged on. ADR 0166.
    control: bool,
    /// Whether shift is held, which is the only thing that distinguishes §12.5.1's tab from
    /// its shift-tab: winit reports one key for both.
    shift: bool,
    /// A touchpad's accumulated pixels, spent one zoom step at a time.
    ///
    /// A wheel notch arrives as a line and a touchpad's pinch as a stream of pixels; sixteen
    /// pixels is one of this program's own text rows and means nothing to a magnification, so
    /// the pixels are counted up and a step taken per `WHEEL_ZOOM_PIXELS` rather than per event.
    pinch: f32,
    /// Whether anything a person did is unsaved.
    dirty: bool,
    /// How many passwords have been asked for.
    attempts: usize,
    /// The fonts this program draws its own text with, or why it cannot.
    ///
    /// An `Option` because a build whose compiled-in faces will not parse must still show the
    /// document: the panel is chrome and the page is the point. The refusal is printed once.
    chrome: Option<Chrome>,
    /// The three lists a document keeps about itself, as this program draws them.
    panel: Sidebar,
    /// `/NOTICE`, over the page, which is the About panel the owner asked for.
    about: About,
    /// The find bar, and what a search has said. Opened with `/`.
    find: FindBar,
    /// How many pages the search in progress still has to read, zero when there is none.
    ///
    /// What makes the next frame ask for another step: `viewer-core` reads one page per
    /// `Find::Continue` and this host pumps them from its own event loop, so the window keeps
    /// drawing while a thousand-page document is searched. A count rather than a flag because it
    /// is also what the bar says.
    pages_left: usize,
    /// §12.3.3's outline and §7.11.4's embedded files, taken once when the document opened.
    ///
    /// Copied out of the queries rather than asked for per frame, and not for speed: both are
    /// properties of an immutable document that no edit reaches, so a copy taken at open cannot
    /// go stale — which is exactly not true of §8.11's layers, whose whole point is that a click
    /// changes them, so those are asked for every time.
    ///
    /// (This note read "`Answer::Outline` borrows the viewer, and a panel that is about to send
    /// it a command cannot be holding a borrow of it" until ADR 0247 made that answer owned. The
    /// reason above is the one that survives, and it is the one that was always the stronger.)
    outline: pdf_model::outline::Outline,
    /// §7.11.4's embedded files, likewise.
    attachments: Vec<pdf_model::attachment::Attachment>,
    /// §12.4.3's article threads, likewise: `Query::Articles` reads them on demand and the list
    /// belongs to a document that no edit reaches.
    articles: Vec<pdf_model::article::Thread>,
    /// §12.3.5's collection, where the catalog states one — read once, like the rest.
    ///
    /// `None` for every document anyone has opened. Where it is `Some`, the files tab draws
    /// §12.3.5.2's folder tree and the schema's columns instead of a flat list, and §12.3.5.1's
    /// resolved `/D` decides which of its rows is the document the file says to open on.
    collection: Option<(
        pdf_model::collection::Collection,
        pdf_model::collection::Initial,
    )>,
    /// §14.3.3's Table 349, likewise.
    information: pdf_model::metadata::Information,
    /// §14.3.2's metadata stream, read — `None` where the catalog names none.
    ///
    /// **Was `metadata_stream: bool` until the two-hundred-and-ninety-fourth session**, when
    /// `pdf_model::xmp` gave `viewer-core` something to answer with. The three states matter to a
    /// host and only to a host: a document that states no metadata and one whose metadata this
    /// program could not read get two different sentences in the properties tab.
    metadata: Option<Result<pdf_model::xmp::Xmp, pdf_model::xmp::XmpError>>,
    /// The field a person is typing into: the point on the page that named it, and where in its
    /// value the next character goes.
    ///
    /// **The host keeps the point, not the text.** §12.7.5.3's `DoNotScroll` makes a field take
    /// only as much of a value as fits its rectangle (ADR 0197), so a buffer of what had been
    /// typed would diverge from the field on the first character past the edge — while a point is
    /// a place, and the field does not move. Every keystroke re-asks `Query::FieldAt` for the
    /// value the *document* now holds and sends that value plus one character back, which makes
    /// divergence impossible rather than unlikely.
    ///
    /// **The caret is an offset and nothing more.** Where it *is* on the screen is
    /// `Query::Caret`'s answer, because the place the next character will be drawn is
    /// §12.7.4.3's arithmetic and not this host's — the same division `Query::Selection` and
    /// `Query::Focus` already draw. The offset is clamped to the value after every edit, which is
    /// what keeps it inside a value the field truncated.
    ///
    /// `None` is a host that is not typing, which is every host until somebody clicks a field.
    typing: Option<Typing>,
    /// What Ctrl + C and Ctrl + X took out of a field — or `c` took off the page — for Ctrl + V
    /// to put back.
    ///
    /// **This program's own clipboard and not the system's.** Reaching the platform's is a
    /// platform's business — X11's `CLIPBOARD` selection, Wayland's data device, `NSPasteboard`,
    /// `OpenClipboard` — and a native host embedding `viewer-core` owns that end by construction,
    /// exactly as it owns the colour a selection is drawn in. What this host demonstrates is the
    /// part that is the *viewer's*: which characters those three keys mean, which is the range
    /// `Query::FieldSelection` draws and `Edit::SetField` replaces. Nothing about it crosses the
    /// boundary, and that is ADR 0225's finding rather than a shortcut.
    clipboard: String,
    /// §12.5.6.6's rectangle while a person is drawing one, in the page viewport's pixels.
    ///
    /// **`Some` only between `f` and the release that ends the drag**, which is a mode a highlight
    /// does not need: a free text annotation's geometry is a box a person drew rather than text
    /// they swept, so there is nothing on the page for a press to mean until they have drawn it.
    /// `f` arms it, the next press records the first corner, and the release sends
    /// `Edit::FreeText` with both. Which key, and that there is a mode at all, are this host's —
    /// the standard describes the annotation and says nothing about how a person comes to make
    /// one.
    drawing: Option<Drawing>,
    /// §12.3.4's tab: one entry per page, with its label and its decoded thumbnail.
    ///
    /// **Empty until that tab is first shown**, which is principle 2 with a clause behind it:
    /// §12.3.4's NOTE says thumbnails "are not required, and can be included for some pages and
    /// not for others", so building this list means decoding every miniature a document carries,
    /// and a document opens at a page rather than at a contact sheet. Filled once and kept,
    /// because a thumbnail is a property of an immutable document — the same argument the
    /// outline and the attachments are cached under, and exactly not the layers'.
    pages: Vec<viewer_ui::chrome::Page>,
    state: Option<State>,
    /// The thread opening the document, until the window and the device have been brought up.
    ///
    /// `None` from the moment it is joined, which is the first thing `resumed` does after the
    /// presenter exists — so every later event, command and query sees an ordinary `Viewer` and
    /// nothing else in this file knows a thread was involved.
    opening: Option<std::thread::JoinHandle<(Viewer, Vec<Event>)>>,
    /// The thread creating the graphics instance, which is 80% of what bring-up blocks for.
    ///
    /// **`None` under `--cpu`, where it was never spawned** — the one place in this program that
    /// distinguishes "the instance is not ready yet" from "there will not be one", and both read
    /// as `None` here only because `resumed` asks `processor` first.
    instancing: Option<std::thread::JoinHandle<quorra_gpu::wgpu::Instance>>,
    /// The launch path's milestones, printed once under `--trace` when the first frame lands.
    launch: Launch,
    /// §14.7's structure on AT-SPI, once there is a page to put there.
    ///
    /// **`None` until the first frame has been presented**, and that is `CLAUDE.md`'s startup
    /// rule rather than laziness for its own sake: bringing this up creates a thread and a D-Bus
    /// connection, and page one may not wait behind either. Once it exists it costs nothing per
    /// page — `accesskit_unix` keeps every adapter inactive until an assistive technology is
    /// actually present, so publishing a page a screen reader is not reading is a lock and a
    /// clone. ADR 0214.
    accessibility: Option<viewer_accessibility::Bridge>,
    /// The page and viewport last handed to it, so that one is not rebuilt per frame.
    ///
    /// A scroll and a zoom leave the structure alone; a page turn replaces it, and a resize moves
    /// every rectangle in it. Those are the two things this remembers, and everything else a
    /// person does costs the bridge nothing.
    spoken: Option<(usize, u32, u32)>,
}

/// The magnification past which quorra's GPU coverage lane is the cheaper one.
///
/// **Derived, not tuned.** quorra keeps a glyph's rasterised coverage in an atlas until
/// the glyph exceeds 128 device pixels; past that it rasterises the glyph again on
/// every frame, which is where its cost stops being flat (its ADR 0016). The
/// magnification that happens at is `128 ÷ the height of the text`, so body text of 10
/// to 12 points crosses it between 10.7× and 13×. Ten is the low end of that band,
/// chosen because being early costs a fraction of a millisecond and being late costs
/// ten — measured on this machine at 0.44 ms per frame at 8× against 4.4 ms at 12×
/// (`doc/quorra-gpu-coverage.md`).
///
/// A page whose text is much larger or much smaller than a book's crosses it somewhere
/// else, and the honest way to do better would be to ask the display list what size its
/// text is rather than to move this number.
const GPU_COVERAGE_MAGNIFICATION: f32 = 10.0;

/// Which coverage lane the next frame should be drawn with.
///
/// Per frame, because the crossover is a magnification and a person zooming crosses it;
/// decided *here*, because this is the only crate that knows what magnification the
/// frame is at. The transform's determinant is the magnification squared — the page
/// transform is a scale, a y flip and a translation, and §7.7.3.3's page rotation puts
/// the same factor into `b` and `c` instead of `a` and `d` — so its square root is the
/// number to compare, and it is right for a rotated page as well.
fn coverage_for(transform: Transform) -> quorra_gpu::Coverage {
    let magnification = transform
        .a
        .mul_add(transform.d, -(transform.b * transform.c))
        .abs()
        .sqrt();
    if magnification >= GPU_COVERAGE_MAGNIFICATION {
        quorra_gpu::Coverage::Gpu
    } else {
        quorra_gpu::Coverage::Cpu
    }
}

/// Draws the page with [`CpuRasterizer`] and puts it on the window, whichever surface it has.
///
/// **This is one of the two jobs `CLAUDE.md` keeps the CPU backend for**: the correctness oracle,
/// and the frame the graphics device refuses. (It was three until the two-hundred-and-seventy-third
/// session, where the project owner decided page one goes to the device.) So a page the device
/// refuses is a page this program can still show — more slowly, which is a cost a person can see
/// past, where a page that never appears is not.
///
/// **Two ways back to the window, and the difference is where the overlays are composited.** With
/// a device, the raster is handed over as one image and quorra draws the overlays over it as
/// geometry, because its surface is the only path pixels take. Without one, `SoftwareSurface`
/// composites them on the processor and copies the result. `--cpu` takes the second, and so does
/// a machine whose device would not come up.
///
/// The error is a sentence rather than a type because both of its sources are already strings by
/// the time they reach the caller, which formats them into one report.
fn on_the_processor(
    surface: &mut Surface,
    list: &pdf_render::DisplayList,
    target: TargetSpec,
    overlays: &[&pdf_render::DisplayList],
) -> Result<(), String> {
    let raster = CpuRasterizer::new()
        .rasterize(list, target)
        .map_err(|problem| format!("the processor {problem}"))?;
    match surface {
        Surface::Device(presenter) => presenter
            .present(PresentFrame {
                width: target.width,
                height: target.height,
                page: None,
                raster: Some(&raster),
                overlays,
            })
            .map_err(|problem| format!("presenting the processor's page {problem}")),
        Surface::Processor(surface) => surface
            .present(&raster, overlays)
            .map_err(|problem| format!("presenting the processor's page {problem}")),
    }
}

/// One page of a §12.4.4 transition: the display list drawn to pixels, ready to be drawn again.
///
/// [`CpuRasterizer`] and not the graphics device, because this asks for *pixels* and a presenter
/// answers only by presenting. It happens twice per transition rather than per frame.
fn face(list: &pdf_render::DisplayList, target: TargetSpec) -> Option<Image> {
    let raster: Raster = CpuRasterizer::new().rasterize(list, target).ok()?;
    viewer_core::transition::drawable(&raster)
}

/// The far corner of a window, as a point in its own device pixels.
#[expect(
    clippy::cast_precision_loss,
    reason = "window dimensions are far below f32's exact integer range"
)]
fn whole(width: u32, height: u32) -> Point {
    Point::new(width as f32, height as f32)
}

/// Whether a quadrilateral this crate was handed covers a point in the viewport.
///
/// The **bounding box** of the four corners, and that is exact rather than an approximation for
/// the shape this is asked about: `viewer_core` builds a widget's quadrilateral out of Table 166's
/// `/Rect`, which "shall be two opposite corners" of an upright rectangle, through §7.7.3.3's
/// `/Rotate` — and that entry's value "shall be a multiple of 90", so the rectangle is still
/// upright on the screen. A quadrilateral this test would get wrong is one no page can state.
fn covers(quad: [f32; 8], (x, y): (f32, f32)) -> bool {
    let xs = [quad[0], quad[2], quad[4], quad[6]];
    let ys = [quad[1], quad[3], quad[5], quad[7]];
    let bound = |values: [f32; 4]| {
        values.iter().copied().fold(f32::INFINITY, f32::min)
            ..=values.iter().copied().fold(f32::NEG_INFINITY, f32::max)
    };
    bound(xs).contains(&x) && bound(ys).contains(&y)
}

/// The nearest character boundary at or before `offset`, clamped to the value's length.
///
/// A caret is a place *between* characters, and the value it indexes changes under it: a field
/// that truncated what was typed (§12.7.5.3) is shorter than what was sent, and a value read back
/// from the document may be shorter still. Every use of the offset goes through this, so a
/// multi-byte character can never be cut in half by an index.
fn caret_boundary(value: &str, offset: usize) -> usize {
    // The clamp comes first, and the test below is why: `is_char_boundary` answers *false* for an
    // offset past the end, so a search for the boundary before it would land on the last
    // character's rather than on the end of the value — one character short after every
    // truncation, which is the case this function exists for.
    let offset = offset.min(value.len());
    if value.is_char_boundary(offset) {
        return offset;
    }
    value
        .char_indices()
        .map(|(at, _)| at)
        .take_while(|at| *at < offset)
        .last()
        .unwrap_or(value.len())
}

/// The offset one character before `caret`, or the start of the value.
fn before(value: &str, caret: usize) -> usize {
    value
        .get(..caret)
        .and_then(|prefix| prefix.char_indices().next_back().map(|(at, _)| at))
        .unwrap_or(0)
}

/// The offset one character after `caret`, or the end of the value.
fn after(value: &str, caret: usize) -> usize {
    value
        .get(caret..)
        .and_then(|rest| rest.chars().next())
        .map_or(value.len(), |character| {
            caret.saturating_add(character.len_utf8())
        })
}

/// The value with `from..to` replaced by `insert`.
///
/// The whole of what a keystroke does to a field, and it is a whole *value* rather than an edit
/// because that is what `Edit::SetField` carries: the core is told what the field says now, and
/// §12.7.5.3 decides how much of it the widget accepts.
fn spliced(value: &str, from: usize, to: usize, insert: &str) -> String {
    let mut out = String::with_capacity(value.len().saturating_add(insert.len()));
    out.push_str(value.get(..from).unwrap_or_default());
    out.push_str(insert);
    out.push_str(value.get(to..).unwrap_or_default());
    out
}

/// Which of §14.8.2.5's two orders a copy takes, and the name to say it under.
///
/// `logical` is [`Query::LogicalSelection`]'s answer and `page_order` is [`Query::Selection`]'s
/// text. Three cases and each is a decision rather than a fallback:
///
/// - **Nothing selected is nothing copied**, and the clipboard is left holding whatever a field
///   put there. A copy that quietly emptied it would lose a person's paste.
/// - **A logical order, where the core could give one**, which is the point of the query: the
///   clause makes the two orders differ on 5 of the corpus's 77 tagged first pages.
/// - **Page content order otherwise**, said out loud. The core answers nothing for a document
///   with no structure tree *and* for a tree that does not reach every byte of the selection,
///   and this host cannot tell those apart — what it can do is not claim the order it did not
///   get.
fn copied(logical: Option<String>, page_order: &str) -> Option<(String, &'static str)> {
    if page_order.is_empty() {
        return None;
    }
    Some(match logical {
        Some(logical) => (logical, "§14.8.2.5's logical content order"),
        None => (page_order.to_owned(), "page content order"),
    })
}

/// Where the two ends of a selection go when an arrow key moves the caret.
///
/// Three cases, and only the first is anybody's convention but this host's: **shift** holds the
/// anchor and moves the caret by one character, so the selection grows or shrinks; a move with
/// something selected and no shift lands on the *edge* of it, which is what a person means by
/// pressing Left with a word swept; and a move with nothing selected steps one character and takes
/// the anchor with it. The standard states none of this — it describes no cursor at all — so it is
/// recorded as the choice it is (ADR 0225).
fn stepped(current: &str, ends: (usize, usize), shift: bool, forward: bool) -> (usize, usize) {
    let (caret, anchor) = ends;
    let (low, high) = (caret.min(anchor), caret.max(anchor));
    let step = if forward {
        after(current, caret)
    } else {
        before(current, caret)
    };
    if shift {
        (step, anchor)
    } else if low < high {
        let edge = if forward { high } else { low };
        (edge, edge)
    } else {
        (step, step)
    }
}

/// A person typing into a form field: which field, and where in its value.
///
/// Three numbers and no text, which is ADR 0201's decision with ADR 0211's caret and ADR 0225's
/// selection added to it. The point names the field because a field does not move; the two offsets
/// say where the next character goes and how much of the value a person has swept, and they are
/// the one thing about typing that the core cannot know — nothing in a document says where a
/// person's cursor is.
///
/// **A caret is a collapsed selection here, and two questions in the core.** This host holds one
/// pair of offsets and calls them equal when nothing is selected; `viewer-core` answers
/// `Query::Caret` with a segment and `Query::FieldSelection` with boxes, because those are two
/// shapes and not one (ADR 0225).
#[derive(Debug, Clone, Copy)]
struct Typing {
    /// Which of the two kinds of thing on a page takes characters.
    target: Target,
    /// The point on the page that named the field, in the page viewport's device pixels.
    at: (f32, f32),
    /// How far into the field's value the caret is, in bytes.
    caret: usize,
    /// The other end of the selection, in bytes — equal to [`Self::caret`] when nothing is
    /// selected, which is the ordinary state.
    anchor: usize,
}

/// How one keystroke's edit is addressed, resolved when the key is pressed.
///
/// [`Target`] says which *kind* of thing has the keyboard and is cheap enough to keep in
/// [`Typing`]; this carries the name, which is a `String` for one of the two and would cost
/// `Typing` its `Copy`.
enum Aim {
    /// §12.7.4.2's fully qualified name.
    Field(String),
    /// The annotation's object.
    FreeText(ObjectId),
}

/// A free text annotation being drawn: armed, or with its first corner down.
#[derive(Debug, Clone, Copy)]
enum Drawing {
    /// `f` was pressed and the next press on the page starts the rectangle.
    Armed,
    /// The pointer went down here, in the page viewport's device pixels.
    From((f32, f32)),
}

/// What a person is typing into.
///
/// **Two, and the standard is why**: §12.7.4.3 lays out the value of a field, and §12.5.6.6 sends
/// its own annotation to that same subclause — "[s]ubclause 12.7.4.3, 'Variable text', describes
/// the process of using these entries to generate the appearance of the text in these
/// annotations". The caret, the selection and every key below are the *same* for both, because the
/// core answers `Query::Caret`, `Query::Offset` and `Query::FieldSelection` for both; what differs
/// is the one question that reads the text back and the one edit that puts it there.
#[derive(Debug, Clone, Copy)]
enum Target {
    /// §12.7's field at [`Typing::at`], named by §12.7.4.2's qualified name when an edit is sent.
    Field,
    /// §12.5.6.6's annotation, named by its object because it has no other name.
    FreeText(ObjectId),
}

impl Typing {
    /// Nothing selected, at one offset.
    fn at_offset(target: Target, at: (f32, f32), offset: usize) -> Self {
        Self {
            target,
            at,
            caret: offset,
            anchor: offset,
        }
    }

    /// The selected range, low end first — empty where the two offsets are the same.
    fn range(self) -> (usize, usize) {
        (self.caret.min(self.anchor), self.caret.max(self.anchor))
    }
}

/// How often the clock is offered to `viewer-core` while a presentation is running and nothing
/// is being animated.
///
/// §12.4.4.1's `/Dur` is "in seconds", and `Command::Tick` carries the milliseconds that actually
/// passed rather than an assumed step, so the only thing this decides is *when the advance is
/// noticed* — a tenth of a second against a duration a document states in whole ones. Ten times a
/// second rather than every frame, because between transitions a slide show has nothing to draw
/// and a window that redrew anyway would spend a processor on a still page.
const PRESENTATION_TICK: std::time::Duration = std::time::Duration::from_millis(100);

/// §12.4.4's presentation, while this window is driving one.
///
/// **The existence of this value is what "presentation mode" means here**, which is ADR 0135's
/// answer rather than this round's: `viewer-core` has no clock and no mode, so "is a presentation
/// running" is answered by whether a host is ticking. `p` starts and stops it.
struct Presentation {
    /// When the clock was last read, so that a tick carries the milliseconds that really passed.
    ticked: std::time::Instant,
    /// When the next tick is due, which is what the event loop waits until.
    wake: std::time::Instant,
    /// The transition being drawn, where one is in flight.
    playing: Option<Playing>,
}

/// One transition, mid-flight: what it is, when it began, and the two pages it is between.
struct Playing {
    /// Table 164's style, duration and direction, as the page arrived at states them.
    transition: pdf_model::navigation::Transition,
    /// When the first frame of it was drawn.
    began: std::time::Instant,
    /// The page being left, as it was last presented.
    outgoing: Image,
    /// The page being moved to, at the same size.
    incoming: Image,
}

/// How this window's pixels reach it: with a graphics device, or without one.
///
/// **Never both, and that is the point.** A process holding [`Surface::Processor`] has created no
/// `wgpu::Instance`, selected no adapter and made no device, so a driver that faults while it
/// loads cannot reach it. Before the three-hundred-and-eighty-fourth session there was one
/// variant and `--cpu` chose only which rasteriser drew into it (ADR 0221).
enum Surface {
    /// quorra's device holds the surface; one call draws and presents a frame,
    /// and a refused frame is a typed error naming what refused — the banding
    /// machinery, the owned intermediate texture and the blitter of the Vello
    /// host all fell away with the backend that needed them.
    Device(Box<QuorraPresenter>),
    /// The processor's raster copied onto the window, with the overlays composited into it
    /// first. `--cpu`, and a device that would not come up.
    Processor(SoftwareSurface),
}

impl std::fmt::Debug for Surface {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Device(presenter) => formatter.debug_tuple("Device").field(presenter).finish(),
            Self::Processor(surface) => formatter.debug_tuple("Processor").field(surface).finish(),
        }
    }
}

/// The window, and whatever puts pixels on it.
struct State {
    window: Arc<Window>,
    /// The graphics device's surface, or the software one. See [`Surface`].
    surface: Surface,
    /// The surface size in device pixels, updated on `WindowEvent::Resized`.
    size: (u32, u32),
}

impl App {
    /// The window's extent in device pixels and its scale factor, once there is a window.
    fn window(&self) -> Option<(u32, u32, f32)> {
        let state = self.state.as_ref()?;
        #[expect(
            clippy::cast_possible_truncation,
            reason = "a display's scale factor is a small ratio"
        )]
        let scale = state.window.scale_factor() as f32;
        Some((state.size.0, state.size.1, scale))
    }

    /// How many device pixels down the left edge the panel occupies.
    ///
    /// **The page's viewport is the window less this.** A panel drawn *over* the page would
    /// hide part of it and leave the core centring the page behind the panel; telling the core
    /// about the smaller viewport instead is what makes a fitted page fit what is visible. It
    /// also means every coordinate crossing the boundary — a pointer going in, a selection quad
    /// coming out — is offset by exactly this and nothing else.
    fn inset(&self) -> u32 {
        self.window()
            .map_or(0, |(_, _, scale)| self.panel.inset(scale))
    }

    /// Tells the core how much of the window is the page's, after the panel appeared or went.
    fn resize_page(&mut self) {
        let Some((width, height, scale)) = self.window() else {
            return;
        };
        self.dispatch(Command::Resize {
            width: width.saturating_sub(self.inset()).max(1),
            height,
            scale,
        });
        self.redraw();
    }

    /// The panel's own display list for this frame, or `None` when there is nothing to draw.
    ///
    /// Rebuilt per frame rather than kept: it is a few hundred glyph fills against the page's
    /// tens of thousands, and a cache would be one more thing that can disagree with the scroll
    /// position.
    fn panel_list(&self, height: u32) -> Option<pdf_render::DisplayList> {
        let chrome = self.chrome.as_ref()?;
        if !self.panel.shown {
            return None;
        }
        let scale = self.window().map_or(1.0, |(_, _, scale)| scale);
        let layers = self.layers();
        Some(
            self.panel
                .draw(chrome, self.content(&layers), height, scale),
        )
    }

    /// Takes the lists a document cannot change, once, when it opens.
    ///
    /// §12.3.3's outline, §7.11.4's embedded files and §12.4.3's article threads — all three
    /// properties of an immutable document, so what the panel holds is a copy that cannot go
    /// stale. §8.11's layers are *not* here for exactly that reason.
    fn gather(&mut self) {
        if let Answer::Outline(outline) = self.viewer.query(Query::Outline) {
            self.outline = outline;
        }
        if let Answer::Attachments(files) = self.viewer.query(Query::Attachments) {
            self.attachments = files;
        }
        if let Answer::Articles(threads) = self.viewer.query(Query::Articles) {
            self.articles = threads;
        }
        self.collection = match self.viewer.query(Query::Collection) {
            Answer::Collection {
                collection,
                initial,
            } => Some((collection, initial)),
            _ => None,
        };
        if let Answer::Properties {
            information,
            metadata,
        } = self.viewer.query(Query::Properties)
        {
            self.information = information;
            self.metadata = metadata;
        }
        // §12.2 names XMP's `dc:title` and this program now reads it; see `named`. What is left
        // to say out loud is the case where it *could not* — a document that asks for its title
        // and whose metadata stream this reader refused, which is the one situation where the
        // fallback to §14.3.3's `/Info /Title` is still a substitution rather than the clause.
        if let Some(Err(error)) = self.metadata.as_ref() {
            println!("note: this document's §14.3.2 metadata stream could not be read: {error}");
            if self.display_doc_title() {
                println!(
                    "note: it also asks for its title in the title bar (§12.2's \
                     /DisplayDocTitle), which names XMP's dc:title; §14.3.3's /Info /Title is \
                     shown instead"
                );
            }
        }
        self.retitle();
        self.obey_page_mode();
        let layers = self.layers().len();
        if !self.outline.items.is_empty()
            || !self.attachments.is_empty()
            || !self.articles.is_empty()
            || layers > 0
        {
            println!(
                "{}: {} outline item(s), {layers} layer entr(ies), {} embedded file(s), {} \
                 article thread(s) — press o for the panel",
                self.title,
                self.outline.visible_count(),
                self.attachments.len(),
                self.articles.len()
            );
        }
    }

    /// Writes an extracted embedded file beside the document.
    ///
    /// **Rule 2 in the other direction**: the core produced the bytes and the host decides where
    /// they go. Beside the open document and nowhere else, which is the mirror of the policy
    /// §12.7.6.4's import takes — and the file's own name is a string *the document wrote*, so
    /// only its last component is used and a name that is a path, is empty, or is `..` is
    /// refused rather than followed. §7.11.4 states no policy at all, because a policy is a
    /// property of the processor.
    fn write_extracted(&self, name: &str, bytes: &[u8]) {
        let stem = std::path::Path::new(name).file_name();
        let Some(stem) = stem.filter(|stem| !stem.is_empty()) else {
            println!("note: the embedded file's name {name:?} is not a file name");
            return;
        };
        let path = self.directory.clone().unwrap_or_default().join(stem);
        match std::fs::write(&path, bytes) {
            Ok(()) => println!("extracted {} bytes to {}", bytes.len(), path.display()),
            Err(error) => println!("note: cannot write {}: {error}", path.display()),
        }
    }

    /// Table 29: opens the panel the document asks for, and says what it cannot do.
    ///
    /// §7.7.2's `/PageMode` is "how the document shall be displayed when opened", and until the
    /// hundred-and-seventieth session this program had no panel for any of its answers to name.
    /// **Four of the six it can now obey** — `UseThumbs` joined the other three in the
    /// two-hundred-and-sixty-sixth session, when §12.3.4's panel arrived — and what is left is
    /// `UseNone`, which asks for nothing, and `FullScreen`, which names chrome that does not
    /// exist here and is said once rather than ignored: a document asking for something and
    /// getting silence is trap 5 in an interface.
    ///
    /// `/PageLayout` likewise. This window shows one page at a time, which is Table 29's own
    /// default, so a document stating `SinglePage` — 24 of the corpus's 43 — is answered exactly
    /// and says nothing.
    fn obey_page_mode(&mut self) {
        use pdf_model::viewer_preferences::{PageLayout, PageMode};
        let Answer::Opening(opening) = self.viewer.query(Query::Opening) else {
            return;
        };
        match opening.mode {
            PageMode::UseNone => {}
            PageMode::UseOutlines => self.panel.show(Tab::Contents),
            PageMode::UseOptionalContent => self.panel.show(Tab::Layers),
            PageMode::UseAttachments => self.panel.show(Tab::Files),
            PageMode::UseThumbs => self.panel.show(Tab::Pages),
            PageMode::FullScreen => println!(
                "note: this document asks to open full screen (§7.7.2), which is chrome this \
                 program does not have"
            ),
        }
        if opening.layout != PageLayout::SinglePage {
            println!(
                "note: this document asks for the {:?} page layout (§7.7.2); this window shows \
                 one page at a time",
                opening.layout
            );
        }
    }

    /// The About card's display list for this frame, or `None` when it is not shown.
    fn about_list(&self, width: u32, height: u32) -> Option<pdf_render::DisplayList> {
        let chrome = self.chrome.as_ref()?;
        if !self.about.shown {
            return None;
        }
        let scale = self.window().map_or(1.0, |(_, _, scale)| scale);
        Some(self.about.draw(chrome, NOTICE, width, height, scale))
    }

    /// The find bar, where it is open.
    ///
    /// Across the whole window rather than only the page area, which is where both native hosts
    /// put theirs: a search is about the document and not about the page pane.
    fn find_list(&self, width: u32) -> Option<pdf_render::DisplayList> {
        let chrome = self.chrome.as_ref()?;
        let scale = self.window().map_or(1.0, |(_, _, scale)| scale);
        self.find.draw(chrome, width, scale)
    }

    /// §8.11.4.3's `/Order`, asked for fresh.
    ///
    /// Unlike the outline and the attachments this is *not* cached: a click on a layer's switch
    /// changes it, so a copy taken when the document opened would be the one thing on the panel
    /// that lies.
    fn layers(&self) -> Vec<viewer_core::Layer> {
        match self.viewer.query(Query::Layers) {
            Answer::Layers(layers) => layers,
            _ => Vec::new(),
        }
    }

    /// The three lists, gathered for one call into the sidebar.
    fn content<'a>(&'a self, layers: &'a [viewer_core::Layer]) -> Content<'a> {
        Content {
            outline: &self.outline,
            layers,
            attachments: &self.attachments,
            articles: &self.articles,
            collection: self.collection.as_ref().map(|(collection, initial)| {
                viewer_ui::chrome::Presentation {
                    collection,
                    initial,
                }
            }),
            information: &self.information,
            metadata: self.metadata.as_ref(),
            pages: &self.pages,
        }
    }

    /// Builds §12.3.4's page list, once, the first time its tab is shown.
    ///
    /// Called from `present`, which is the one place that runs before the panel is drawn and
    /// holds `&mut self`. A document with no thumbnails at all still gets a list — the rows are
    /// its pages, and §12.3.4's NOTE is why a page without one is still a page.
    fn ensure_pages(&mut self) {
        if !self.panel.shows_pages() || !self.pages.is_empty() {
            return;
        }
        let Answer::Count(count) = self.viewer.query(Query::PageCount) else {
            return;
        };
        self.pages = (0..count)
            .map(|index| {
                let label = match self.viewer.query(Query::PageLabel(index)) {
                    Answer::Label(label) => label,
                    _ => format!("Page {}", index.saturating_add(1)),
                };
                let thumbnail = match self.viewer.query(Query::Thumbnail(index)) {
                    Answer::Thumbnail(thumbnail) => Some(thumbnail.image),
                    _ => None,
                };
                viewer_ui::chrome::Page { label, thumbnail }
            })
            .collect();
    }

    /// What the pointer moving does: the panel's highlight, or the page's §12.5.5 appearance.
    ///
    /// Only one of the two, and never both: a hover highlight in the panel and a rollover
    /// appearance on the page are both answers to "what is under the pointer", and answering
    /// both would leave an annotation lit up behind a panel.
    fn pointer_moved(&mut self) {
        if self.about.shown {
            return;
        }
        let scale = self.window().map_or(1.0, |(_, _, scale)| scale);
        let layers = self.layers();
        // The struct is written out here rather than built by `content`: `self.panel` is
        // borrowed mutably, and only a *field* borrow of the other three is disjoint from it.
        let moved = self.panel.hover(
            at(self.cursor),
            Content {
                outline: &self.outline,
                layers: &layers,
                attachments: &self.attachments,
                articles: &self.articles,
                collection: self.collection.as_ref().map(|(collection, initial)| {
                    viewer_ui::chrome::Presentation {
                        collection,
                        initial,
                    }
                }),
                information: &self.information,
                metadata: self.metadata.as_ref(),
                pages: &self.pages,
            },
            scale,
        );
        drop(layers);
        if moved {
            self.redraw();
        }
        if self.over_panel() {
            if let Some(state) = self.state.as_ref() {
                state.window.set_cursor(winit::window::CursorIcon::Default);
            }
            return;
        }
        let point = self.on_page(self.cursor);
        // **A drag that began inside a field belongs to that field's value**, and the page's own
        // selection is not asked to extend: two highlights over one gesture would say the person
        // had swept the page as well. The anchor stays where the press put it and the caret
        // follows the pointer, which is what makes the pair a selection — `Query::Offset` is asked
        // with the field's point and the pointer's, because a drag that leaves the widget's
        // rectangle is still a drag inside its value (ADR 0225).
        if self.dragging
            && let Some(typing) = self.typing
        {
            if let Answer::Offset(offset) = self.viewer.query(Query::Offset {
                at: typing.at,
                point,
            }) && offset != typing.caret
            {
                self.typing = Some(Typing {
                    caret: offset,
                    ..typing
                });
                self.redraw();
            }
            return;
        }
        self.dispatch(Command::Pointer {
            at: point,
            action: if self.dragging {
                PointerAction::Dragged
            } else {
                PointerAction::Moved
            },
        });
        // §12.5.6.5's activation region, asked at pointer speed — which is why it is a query
        // rather than a command with an event coming back.
        if let (Answer::Link(over), Some(state)) =
            (self.viewer.query(Query::LinkAt(point)), self.state.as_ref())
        {
            state.window.set_cursor(if over {
                winit::window::CursorIcon::Pointer
            } else {
                winit::window::CursorIcon::Default
            });
        }
    }

    /// Puts what is selected on the page into this program's clipboard.
    ///
    /// **The first consumer [`Query::LogicalSelection`] has ever had.** `doc/todo/01`'s fifth
    /// sweep — "the model implements this, who calls it?" — found in the four-hundred-and-
    /// thirteenth session that the query had answered since the three-hundred-and-eighty-eighth
    /// and that no host of the four asked it, while §14.8.2.5's ledger row read `implemented` on
    /// the strength of the query alone. A clause reaches a person through a program or it does
    /// not reach one.
    ///
    /// ISO 32000-2 §14.8.2.5 defines the two orders and this host chooses between them:
    ///
    /// > Page content order shall be defined by the sequencing of graphics objects within a
    /// > page's content stream.
    ///
    /// > Logical content order -the ordering for semantic purposes -shall be defined by a
    /// > depth-first traversal of the document's logical structure
    ///
    /// A selection is taken in page content order, because that is the order the shapes are in;
    /// a *copy* off a page whose producer wrote its columns out of order wants the other one.
    /// The core answers [`Answer::None`] where the document states no structure tree or where
    /// the tree does not reach every byte of the selection, and [`copied`] is what that means
    /// here: the order it could give, named out loud, rather than a silent rearrangement.
    fn copy_selection(&mut self) {
        // Owned before the second question, because both answers borrow the viewer.
        let page_order = match self.viewer.query(Query::Selection) {
            Answer::Selected(selection) => selection.text.to_owned(),
            _ => String::new(),
        };
        let logical = match self.viewer.query(Query::LogicalSelection) {
            Answer::LogicalSelection(text) => Some(text),
            _ => None,
        };
        match copied(logical, &page_order) {
            Some((text, order)) => {
                println!(
                    "note: copied {} characters in {order}",
                    text.chars().count()
                );
                self.clipboard = text;
            }
            None => println!("note: nothing on the page is selected to copy"),
        }
    }

    /// Whether anything on the page is selected.
    ///
    /// Asked before §12.5.6.10's markup, which is defined over selected text: the core does
    /// nothing when there is nothing to mark up, and a person who pressed a key and saw no change
    /// has been told nothing at all.
    fn has_selection(&self) -> bool {
        matches!(self.viewer.query(Query::Selection),
            Answer::Selected(selection) if !selection.quads.is_empty())
    }

    /// Starts or stops typing, from where the pointer just went down.
    ///
    /// A press inside a field somebody can type into aims the keyboard at it; a press anywhere
    /// else puts the keyboard back on the page. §12.7.5.1's four field types are not equal here —
    /// a button has no text and a signature field's value is a dictionary — and the *core* is what
    /// draws that line: `Answer::Field`'s value is `None` for a field whose value is not text and
    /// `Some("")` for an empty one, which is the same distinction §12.7.4.3 makes when it decides
    /// what to lay out.
    fn aim_at_field(&mut self) {
        let at = self.on_page(self.cursor);
        let was = self.typing.is_some();
        // §12.5.6.6 first, because the core hit-tests it first: an annotation a person added is
        // drawn after the page's own `/Annots` and the thing on top is the thing under the
        // pointer. Asking in the other order would put the keyboard in a field underneath a note.
        if let Some(typing) = self.aim_at_free_text(at) {
            self.typing = Some(typing);
            return;
        }
        self.typing = match self.viewer.query(Query::FieldAt(at)) {
            // Table 231 bit 14, refused rather than mishandled — and it was mishandled until the
            // four-hundred-and-eleventh session, which is what ADR 0247's third amendment made
            // visible. This host reads a field's value back after every keystroke (ADR 0201) and a
            // password field answers with bullets, so what it sent as the next value was those
            // bullets with a character appended. Refusing is trap 5: `viewer-gtk` and `viewer-qt`
            // type into a `GtkPasswordEntry` and a `QLineEdit` in `Password` echo mode, which are
            // the platform's own secure controls, and this host draws its own page and has none.
            Answer::Field {
                name,
                value: Some(shown),
            } if shown.obscured => {
                println!(
                    "note: {} is a Table 231 bit 14 password field, which this host does not type \
                     into: its value answers as bullets and cannot be read back (ADR 0247)",
                    name.shown()
                );
                None
            }
            Answer::Field {
                name,
                value: Some(value),
            } => {
                println!("note: typing into the field {}", name.shown());
                // **The caret goes where the click went**, which is `Query::Offset` — the inverse
                // of `Query::Caret`, and the piece `doc/todo/33` said was missing until the
                // three-hundred-and-eighty-eighth session. The point names the field and is also
                // the point measured, because a press is one place; a drag then asks the same
                // question with the pointer's place instead. The end of the value is what a field
                // whose layout could not answer falls back to, which is where the caret used to
                // start every time.
                let caret = match self.viewer.query(Query::Offset { at, point: at }) {
                    Answer::Offset(offset) => offset,
                    _ => value.text.len(),
                };
                Some(Typing::at_offset(Target::Field, at, caret))
            }
            _ => None,
        };
        if was && self.typing.is_none() {
            println!("note: the keyboard is back on the page");
        }
    }

    /// Aims the keyboard at §12.5.6.6's annotation, where one a person added is under the point.
    ///
    /// The same two questions a field takes, in the same order and for the same reasons:
    /// `Query::FreeTextAt` names the annotation and hands back Table 166's `/Contents` as the log
    /// has it, and `Query::Offset` says where inside that text the press landed. Neither is a
    /// second implementation of anything — the core answers both from §12.7.4.3's own layout,
    /// which is the subclause §12.5.6.6 sends this subtype to.
    fn aim_at_free_text(&mut self, at: (f32, f32)) -> Option<Typing> {
        let Answer::FreeText { annotation, text } = self.viewer.query(Query::FreeTextAt { at })
        else {
            return None;
        };
        println!(
            "note: typing into the free text annotation {} {}",
            annotation.number, annotation.generation
        );
        let caret = match self.viewer.query(Query::Offset { at, point: at }) {
            Answer::Offset(offset) => offset,
            _ => text.len(),
        };
        Some(Typing::at_offset(Target::FreeText(annotation), at, caret))
    }

    /// One end of the drag that draws §12.5.6.6's rectangle.
    ///
    /// The press records a corner and the release sends `Edit::FreeText` with both, then aims the
    /// keyboard at what it made — which needs no event, because `Query::FreeTextAt` at a point
    /// inside the rectangle answers with the annotation the core just added. Asking rather than
    /// being told is the rule this vocabulary has grown by: a message is added for a question a
    /// host cannot answer for itself, and this is not one.
    ///
    /// **The mode ends with the release, whether or not anything was made.** A drag that never
    /// moved has drawn no box and the core answers it by doing nothing (`add_free_text` refuses a
    /// rectangle with no area); leaving the mode armed after that would be a program that seemed
    /// stuck.
    fn draw_free_text(&mut self, drawing: Drawing, element: ElementState) {
        let at = self.on_page(self.cursor);
        match (drawing, element) {
            (Drawing::Armed, ElementState::Pressed) => self.drawing = Some(Drawing::From(at)),
            (Drawing::From(from), ElementState::Released) => {
                self.drawing = None;
                // The colour of the *text*, which Table 177's `/DA` carries — Table 166's `/C` is
                // an icon's background, a popup's title bar and a link's border, none of which
                // this subtype has. A dark red is this host's choice and no clause's, the same
                // footing the highlight's yellow stands on.
                self.dispatch(Command::Edit(Edit::FreeText {
                    from,
                    to: at,
                    colour: FREE_TEXT_INK,
                }));
                let middle = ((from.0 + at.0) * 0.5, (from.1 + at.1) * 0.5);
                self.typing = self.aim_at_free_text(middle);
                if self.typing.is_none() {
                    println!("note: that rectangle has no area, so there is nothing to type in");
                }
                self.redraw();
            }
            (Drawing::Armed, ElementState::Released)
            | (Drawing::From(_), ElementState::Pressed) => {}
        }
    }

    /// §12.7.5.2's two toggling kinds, clicked.
    ///
    /// **What `Query::Fields` made possible and nothing else could.** `Edit::SetField` takes a
    /// string, and for a check box or a radio button the only strings that mean anything are the
    /// names Table 170's appearance dictionary is keyed by — §12.7.5.2.3 makes `/V` "a name object
    /// representing the check box's appearance state, which shall be used to select the
    /// appropriate appearance from the appearance dictionary" — and those names are the file's own
    /// invention. This host had no way to learn one, so it could type into a form and not tick a
    /// box in it. `FormWidget::on_state` is the name to send. ADR 0235.
    ///
    /// Which widget: `Query::FieldAt` is the hit test, because it is the model's own and because a
    /// second one here could disagree with it; the quadrilaterals then say *which of the field's
    /// widgets* the point was in, which is the question §12.7.5.2.4's set makes real — "at most
    /// one button in a set may be on at any given time", and the one that goes on is the one under
    /// the pointer.
    ///
    /// What to send: §12.7.5.2.4 states the rule for turning one off, and it is a flag rather than
    /// a convention — "[i]f set, exactly one radio button shall be selected at all times; selecting
    /// the currently selected button has no effect. If clear, clicking the selected button
    /// deselects it, leaving no button selected."
    ///
    /// Table 227 bit 1 is checked here as well as in the core, and neither is redundant: the core
    /// refuses the edit, and a host that sent it anyway would be a program that looks broken
    /// rather than one that obeys the document.
    fn toggle_button(&mut self) {
        let at = self.on_page(self.cursor);
        let Answer::Field {
            name, value: None, ..
        } = self.viewer.query(Query::FieldAt(at))
        else {
            // A field whose value is text is one to type into, which `aim_at_field` has already
            // decided about; no field at all is a press on the page.
            return;
        };
        let qualified = name.qualified.clone();
        let Answer::Fields(fields) = self.viewer.query(Query::Fields) else {
            return;
        };
        let Some(field) = fields
            .iter()
            .find(|field| field.name.qualified == qualified)
        else {
            return;
        };
        let no_toggle_to_off = match field.control {
            Control::CheckBox { .. } => false,
            Control::RadioButton {
                no_toggle_to_off, ..
            } => no_toggle_to_off,
            // §12.7.5.2.2's push-button "retains no permanent value", a signature field's is a
            // dictionary, and neither is a control a click gives a value to.
            _ => return,
        };
        if field.read_only {
            println!("note: the field {} is read-only (Table 227)", name.shown());
            return;
        }
        // The *last* widget covering the point, because §12.5.2 draws them in `/Annots` order and
        // the one on top is the one under the pointer — the same rule `pdf_model::view::field_at`
        // applies one level down.
        let Some(widget) = field
            .widgets
            .iter()
            .rev()
            .find(|widget| covers(widget.quad, at))
        else {
            return;
        };
        let value = if widget.on {
            if no_toggle_to_off {
                return;
            }
            // §12.7.5.2.3 names the off state; §12.7.5.2.4 gives it as the default value.
            "Off".to_owned()
        } else {
            let Some(state) = widget.on_state.clone() else {
                println!(
                    "note: the field {} states no appearance for an on state (§12.7.5.2.3)",
                    name.shown()
                );
                return;
            };
            state
        };
        println!("note: setting the field {} to {value}", name.shown());
        self.dispatch(Command::Edit(Edit::SetField {
            field: qualified,
            value: Entered::Text(value),
        }));
    }

    /// Aims the keyboard at whatever §12.5.1's tab walk just landed on, where it takes text.
    ///
    /// **The decision `doc/todo/33` left open, and it needed no new message.** The worry it
    /// recorded was that a focus ring on a *button* means something else — a press activates it,
    /// it does not take characters — and the answer is the one this host already uses for a
    /// click: `Answer::Field`'s value is `Some` only for a field §12.7.4.3 lays text out for, so
    /// the same question decides both. What was missing was only the *point*, and `Query::Focus`
    /// answers with the annotation's quadrilateral in the same device pixels `Query::FieldAt`
    /// takes, so the centre of the ring is the point to ask about.
    ///
    /// A walk onto anything else takes the keyboard back to the page, which is what makes Tab out
    /// of a field stop typing without a second binding for it.
    fn aim_at_focus(&mut self) {
        let Answer::Focus { quad, .. } = self.viewer.query(Query::Focus) else {
            self.typing = None;
            return;
        };
        // The centre of the ring, which is inside the widget's `/Rect` by construction: §12.5.5
        // places the appearance *on* that rectangle and `Query::Focus` answers with it.
        let at = ((quad[0] + quad[4]) * 0.5, (quad[1] + quad[5]) * 0.5);
        self.typing = match self.viewer.query(Query::FieldAt(at)) {
            // The same refusal a click makes, for the same reason — see `aim_at_field`.
            Answer::Field {
                value: Some(shown), ..
            } if shown.obscured => None,
            Answer::Field {
                name,
                value: Some(value),
            } => {
                println!("note: typing into the field {}", name.shown());
                // The end of the value, and here that is not a fallback: a tab press names no
                // point inside the value, so there is nothing for `Query::Offset` to measure and
                // the end is the place ADR 0211 chose for a walk that arrives without one.
                Some(Typing::at_offset(Target::Field, at, value.text.len()))
            }
            _ => None,
        };
        self.redraw();
    }

    /// Ctrl + C, X and V inside a field's value: what the press does to the value, if anything.
    ///
    /// **The finding this round records: copying, cutting and pasting inside a field needed no
    /// message.** The two offsets are into the value `Query::FieldAt` just answered with, so the
    /// characters between them are a slice this host already holds; cutting and pasting are that
    /// slice spliced out or in and sent back as exactly the `Edit::SetField` every keystroke
    /// sends. Nothing crosses the boundary that did not cross it before, which is why ADR 0225
    /// added two questions and no verbs.
    ///
    /// `None` for a character this does not answer, which the caller consumes rather than passing
    /// on. A copy answers `None` for a different reason and says so where it happens: it changes
    /// no value, so there is nothing for the caller to send.
    fn clipped(
        &mut self,
        text: &str,
        current: &str,
        range: (usize, usize),
    ) -> Option<(Option<String>, usize, usize)> {
        let (low, high) = range;
        match text {
            "c" | "x" => {
                current
                    .get(low..high)
                    .unwrap_or_default()
                    .clone_into(&mut self.clipboard);
                println!(
                    "note: {} {} bytes out of the field",
                    if text == "c" { "copied" } else { "cut" },
                    self.clipboard.len()
                );
                if text == "c" {
                    self.redraw();
                    return Some((None, low, high));
                }
                Some((Some(spliced(current, low, high, "")), low, low))
            }
            "v" => {
                let to = low.saturating_add(self.clipboard.len());
                Some((Some(spliced(current, low, high, &self.clipboard)), to, to))
            }
            _ => None,
        }
    }

    /// One key press, while a field has the keyboard. Answers whether it was consumed.
    ///
    /// **Nothing is buffered here.** Every press re-asks the core what the field says and sends
    /// back that value with one character added or one removed, so §12.7.5.3's `DoNotScroll`
    /// truncating a value is a thing the host *reads* rather than a thing it has to predict
    /// (ADR 0197). It costs a query per keystroke, which is a walk of one page's annotations.
    fn typed(&mut self, key: &Key<&str>) -> bool {
        let Some(typing) = self.typing else {
            return false;
        };
        let Some((current, aim)) = self.aimed(typing) else {
            // The field or the annotation went away — a page turned under the pointer — so the
            // keyboard goes back.
            self.typing = None;
            return false;
        };
        // The caret is clamped to the value *this* press starts from, because the last one may
        // have been truncated by §12.7.5.3's `DoNotScroll` — the same reason nothing is buffered.
        let caret = caret_boundary(&current, typing.caret);
        let anchor = caret_boundary(&current, typing.anchor);
        let (low, high) = (caret.min(anchor), caret.max(anchor));
        // **Shift holds the anchor still and moves only the caret**, which is this host's
        // convention and no clause's — the standard states neither a cursor nor a selection inside
        // a value (ADR 0225). Without it a move collapses the selection, which is why every arm
        // below says where *both* ends go.
        let held = |to: usize| if self.shift { (to, anchor) } else { (to, to) };
        let (next, moved, anchored) = match *key {
            Key::Named(NamedKey::Escape) => {
                self.typing = None;
                println!("note: the keyboard is back on the page");
                // The caret goes with the keyboard, and the window is what has to be told: this
                // press changes nothing about the *document*, so no command is sent and nothing
                // else would ask for the frame that takes the caret off the screen.
                self.redraw();
                return true;
            }
            // Ctrl + C, X and V, which needed no message at all — see `clipped`.
            Key::Character(text) if self.control => match self.clipped(text, &current, (low, high))
            {
                Some(outcome) => outcome,
                // Every other key with Control held is consumed rather than sent to the page: a
                // field has the keyboard, and a magnification while somebody is typing is the
                // surprise this whole state exists to prevent.
                None => return true,
            },
            // Moving the caret changes nothing about the document, so these send no edit at all
            // and only ask for the frame that redraws the caret. A move that is not extending the
            // selection lands on the *edge* of it rather than one character further, which is what
            // a person means by pressing Left with something selected.
            Key::Named(NamedKey::ArrowLeft) => {
                let (to, anchor) = stepped(&current, (caret, anchor), self.shift, false);
                (None, to, anchor)
            }
            Key::Named(NamedKey::ArrowRight) => {
                let (to, anchor) = stepped(&current, (caret, anchor), self.shift, true);
                (None, to, anchor)
            }
            Key::Named(NamedKey::Home) => {
                let (to, anchor) = held(0);
                (None, to, anchor)
            }
            Key::Named(NamedKey::End) => {
                let (to, anchor) = held(current.len());
                (None, to, anchor)
            }
            // With something selected, Backspace and Delete take out what is selected and nothing
            // more — the same statement typing a character makes, and the reason both are one
            // splice rather than two behaviours.
            Key::Named(NamedKey::Backspace) => {
                let from = if low < high {
                    low
                } else {
                    before(&current, caret)
                };
                (Some(spliced(&current, from, high, "")), from, from)
            }
            Key::Named(NamedKey::Delete) => {
                let to = if low < high {
                    high
                } else {
                    after(&current, caret)
                };
                (Some(spliced(&current, low, to, "")), low, low)
            }
            Key::Named(NamedKey::Enter) => {
                // §12.7.5.3's Multiline decides whether a return is a character or the end of
                // typing, and the core is what knows: a value with a newline in it lays out on two
                // lines only where Table 231 bit 13 is set, and `variable_text::wrap` is where
                // that is read. So the host offers the newline and the field decides what to keep.
                let to = low.saturating_add(1);
                (Some(spliced(&current, low, high, "\n")), to, to)
            }
            Key::Character(text) if !text.is_empty() => {
                let to = low.saturating_add(text.len());
                (Some(spliced(&current, low, high, text)), to, to)
            }
            Key::Named(NamedKey::Space) => {
                let to = low.saturating_add(1);
                (Some(spliced(&current, low, high, " ")), to, to)
            }
            _ => return false,
        };
        self.typing = Some(Typing {
            caret: moved,
            anchor: anchored,
            ..typing
        });
        // A caret that moved is chrome and not a page: `Query::Caret` answers from state this host
        // holds, so nothing has to be interpreted again and the window only repaints. A keystroke
        // that leaves the value as it was is the same case, and there are two of them — Backspace
        // at the start and Delete at the end — where sending the edit anyway would put an entry in
        // the log, mark the document unsaved and re-interpret the page for a picture that cannot
        // differ.
        let Some(next) = next.filter(|next| *next != current) else {
            self.redraw();
            return true;
        };
        // Through `dispatch`, not through `Viewer::handle` directly: the events an edit raises
        // are what asks for the next frame, and a host that counted them instead of pumping them
        // would type into a page that never redraws. (It did, for one run.)
        self.dispatch(Command::Edit(match aim {
            Aim::Field(field) => Edit::SetField {
                field,
                value: Entered::Text(next),
            },
            Aim::FreeText(annotation) => Edit::SetFreeText {
                annotation,
                text: next,
            },
        }));
        // And the field decides how much of that it took, so where the caret ended up is read
        // back rather than assumed — a value §12.7.5.3 truncated is shorter than what was sent.
        if let Some((taken, _)) = self.aimed(typing) {
            self.typing = Some(Typing {
                caret: caret_boundary(&taken, moved),
                anchor: caret_boundary(&taken, anchored),
                ..typing
            });
        }
        true
    }

    /// What the thing being typed into says now, and how an edit to it is addressed.
    ///
    /// One question per keystroke, asked twice — once before the key is applied and once after,
    /// because what the *document* took is what the caret has to be clamped to (ADR 0197). Which
    /// question it is depends on the target and nothing else does: a field is addressed by
    /// §12.7.4.2's qualified name and §12.5.6.6's annotation by its object, because an annotation
    /// has no name for anything to address it by.
    ///
    /// `None` where the thing is no longer under the point — a page turned under the pointer —
    /// which is what takes the keyboard back to the page.
    fn aimed(&self, typing: Typing) -> Option<(String, Aim)> {
        match typing.target {
            Target::Field => match self.viewer.query(Query::FieldAt(typing.at)) {
                // A value that is Table 231 bit 14's echo is not a value to append to, which is
                // why `aim_at_field` never puts the keyboard here; this is the same refusal one
                // step later, so that the two cannot come apart. ADR 0247.
                Answer::Field {
                    value: Some(shown), ..
                } if shown.obscured => None,
                Answer::Field { name, value } => Some((
                    value.map_or_else(String::new, |shown| shown.text),
                    Aim::Field(name.qualified),
                )),
                _ => None,
            },
            Target::FreeText(annotation) => {
                match self.viewer.query(Query::FreeTextAt { at: typing.at }) {
                    Answer::FreeText { text, .. } => Some((text, Aim::FreeText(annotation))),
                    _ => None,
                }
            }
        }
    }

    /// Whether the pointer is over the panel rather than over the page.
    fn over_panel(&self) -> bool {
        #[expect(
            clippy::cast_precision_loss,
            reason = "a panel width in pixels, which is hundreds"
        )]
        let edge = self.inset() as f32;
        edge > 0.0 && at(self.cursor).0 < edge
    }

    /// A window point in the page's own viewport, which begins where the panel ends.
    fn on_page(&self, cursor: (f64, f64)) -> (f32, f32) {
        #[expect(
            clippy::cast_precision_loss,
            reason = "a panel width in pixels, which is hundreds"
        )]
        let edge = self.inset() as f32;
        let (x, y) = at(cursor);
        (x - edge, y)
    }

    /// What a click inside the panel does.
    fn click_panel(&mut self) {
        let scale = self.window().map_or(1.0, |(_, _, scale)| scale);
        // The outline and the attachments are fields rather than queries for exactly this:
        // `Sidebar::click` produces a command for the viewer, and an `Answer` borrowing it would
        // still be alive. The layers are queried and the answer is *owned*, so the borrow ends
        // before the command goes out.
        let layers = self.layers();
        let hit = self.panel.click(
            at(self.cursor),
            Content {
                outline: &self.outline,
                layers: &layers,
                attachments: &self.attachments,
                articles: &self.articles,
                collection: self.collection.as_ref().map(|(collection, initial)| {
                    viewer_ui::chrome::Presentation {
                        collection,
                        initial,
                    }
                }),
                information: &self.information,
                metadata: self.metadata.as_ref(),
                pages: &self.pages,
            },
            scale,
        );
        drop(layers);
        match hit {
            Some(Hit::Activate(object)) => self.dispatch(Command::Activate(object)),
            Some(Hit::Extract(name)) => self.dispatch(Command::Extract { name }),
            // §8.11.2.2: switching a group re-decides what the page draws, so this goes to the
            // core and comes back as a render rather than as a repaint of the panel.
            Some(Hit::SetGroup { group, on }) => self.dispatch(Command::SetGroup { group, on }),
            // §12.3.4: a click on a page's miniature shows that page. A page index rather than
            // a destination — the thumbnail *is* the page, so there is nothing to resolve.
            Some(Hit::GoTo(page)) => self.dispatch(Command::GoTo(PageTarget::Index(page))),
            Some(Hit::Redraw) => self.redraw(),
            Some(Hit::Nothing) | None => {}
        }
    }

    /// A wheel notch: the About card, the panel's list, or the page — and under Ctrl, a zoom.
    fn wheel(&mut self, delta: winit::event::MouseScrollDelta) {
        // A line is not a pixel and winit reports whichever the device produced. Sixteen logical
        // pixels a line is about one row of this program's own text, which is what a line means
        // on a list; a touchpad reports pixels and needs no conversion.
        let by = match delta {
            winit::event::MouseScrollDelta::LineDelta(_, lines) => -lines * 16.0,
            #[expect(
                clippy::cast_possible_truncation,
                reason = "a scroll delta in pixels, which is tens"
            )]
            winit::event::MouseScrollDelta::PixelDelta(position) => -(position.y as f32),
        };
        if self.about.shown {
            let Some((_, height, scale)) = self.window() else {
                return;
            };
            self.about.scroll(by / scale, NOTICE, height, scale);
            self.redraw();
            return;
        }
        // Ctrl is a magnification of the *page*, and the sidebar has no scale to change — so a
        // notch over the sidebar still zooms the page, with **no anchor**: there is no point of
        // the page under the pointer to hold, and `None` is the core's word for that. A step per
        // notch, and a step per `WHEEL_ZOOM_PIXELS` of a touchpad — the sixteen-pixels-a-line
        // conversion above is a distance on a list and says nothing about a magnification.
        if self.control {
            let whole = match delta {
                winit::event::MouseScrollDelta::LineDelta(_, lines) => {
                    self.pinch = 0.0;
                    lines.trunc()
                }
                winit::event::MouseScrollDelta::PixelDelta(position) => {
                    #[expect(
                        clippy::cast_possible_truncation,
                        reason = "a scroll delta in pixels, which is tens"
                    )]
                    let pixels = position.y as f32;
                    self.pinch += pixels;
                    let whole = (self.pinch / WHEEL_ZOOM_PIXELS).trunc();
                    self.pinch -= whole * WHEEL_ZOOM_PIXELS;
                    whole
                }
            };
            // `ZOOM_RANGE` spans 0.02 to 64, which is thirty-six steps of 1.25 end to end, so a
            // bound of sixty-four cannot hide a magnification anybody could have reached — it is
            // there because a `f32` cast saturates and a device reporting nonsense would
            // otherwise be a loop of two billion commands.
            #[expect(
                clippy::cast_possible_truncation,
                reason = "clamped to ±64 on the line above"
            )]
            let steps = whole.clamp(-64.0, 64.0) as i32;
            let zoom = if steps > 0 { Zoom::In } else { Zoom::Out };
            let at = (!self.over_panel()).then(|| self.on_page(self.cursor));
            for _ in 0..steps.unsigned_abs() {
                self.dispatch(Command::Zoom { zoom, at });
            }
            return;
        }
        if self.over_panel() {
            let Some((_, height, scale)) = self.window() else {
                return;
            };
            let layers = self.layers();
            self.panel.scroll(
                by / scale,
                Content {
                    outline: &self.outline,
                    layers: &layers,
                    attachments: &self.attachments,
                    articles: &self.articles,
                    collection: self.collection.as_ref().map(|(collection, initial)| {
                        viewer_ui::chrome::Presentation {
                            collection,
                            initial,
                        }
                    }),
                    information: &self.information,
                    metadata: self.metadata.as_ref(),
                    pages: &self.pages,
                },
                height,
                scale,
            );
            drop(layers);
            self.redraw();
        } else {
            self.dispatch(Command::Scroll { dx: 0.0, dy: by });
        }
    }

    /// Hands a command to the core and deals with everything that comes back.
    ///
    /// A queue rather than recursion, because reacting to an event may produce a command — a
    /// password supplied, a file read — and a chain of those is a loop rather than a stack.
    fn dispatch(&mut self, command: Command) {
        self.pump(VecDeque::from([command]));
    }

    /// Reacts to events that were produced somewhere other than a [`Self::dispatch`].
    ///
    /// One caller: the thread that opens the document while the window and the graphics device
    /// come up. Its events are a `Vec` rather than an iterator over the viewer, because the
    /// viewer they came from was on another thread — and everything after that is the ordinary
    /// loop, so a `PasswordRequired` from the thread is answered exactly as one from a command.
    fn receive(&mut self, events: Vec<Event>) {
        if self.trace.on(Topic::Events) {
            self.trace.say(
                Topic::Events,
                format_args!("opened on its own thread -> {} event(s)", events.len()),
            );
            for event in &events {
                self.trace
                    .more(Topic::Events, format_args!("{}", describe_event(event)));
            }
        }
        let mut queue = VecDeque::new();
        for event in events {
            self.react(event, &mut queue);
        }
        self.pump(queue);
    }

    /// Runs commands until nothing is left, reacting to what each produces.
    fn pump(&mut self, mut queue: VecDeque<Command>) {
        while let Some(command) = queue.pop_front() {
            let started = std::time::Instant::now();
            // The pointer is its own topic and not `events`: 285 of the 1490 lines of the trace
            // that raised ADR 0227 were pointer moves, and they arrive faster than a person
            // can read whatever else is happening.
            let topic = if matches!(command, Command::Pointer { .. }) {
                Topic::Pointer
            } else {
                Topic::Events
            };
            let described = self.trace.on(topic).then(|| describe_command(&command));
            let events: Vec<Event> = self.viewer.handle(command).collect();
            if let Some(described) = described {
                self.trace.say(
                    topic,
                    format_args!(
                        "{described} -> {} event(s) in {:?}",
                        events.len(),
                        started.elapsed()
                    ),
                );
                for event in &events {
                    self.trace
                        .more(topic, format_args!("{}", describe_event(event)));
                }
            }
            for event in events {
                self.react(event, &mut queue);
            }
        }
    }

    /// Does what one event asks.
    fn react(&mut self, event: Event, queue: &mut VecDeque<Command>) {
        match event {
            Event::Opened { pages, .. } => {
                println!("{}: {pages} page(s)", self.title);
                if pages == 0 {
                    eprintln!("the document has no pages");
                    std::process::exit(1);
                }
                self.attempts = 0;
                self.gather();
            }
            Event::OpenFailed { reason, .. } => {
                eprintln!("cannot open {}: {reason}", self.title);
                std::process::exit(1);
            }
            // §7.6.4.1: a processor tries the default user password and then prompts. This is the
            // prompt, and it is the whole of what this program owed the clause.
            Event::PasswordRequired { document } => {
                self.attempts = self.attempts.saturating_add(1);
                if self.attempts > PASSWORD_ATTEMPTS {
                    eprintln!("{}: too many attempts", self.title);
                    std::process::exit(1);
                }
                let Some(password) = ask_password(&self.title) else {
                    eprintln!("{}: needs a password", self.title);
                    std::process::exit(1);
                };
                let Ok(bytes) = std::fs::read(&self.path) else {
                    eprintln!("cannot re-read {}", self.title);
                    std::process::exit(1);
                };
                queue.push_back(Command::Open {
                    id: document,
                    bytes,
                    password: Some(password),
                    fragment: self.fragment.clone(),
                });
            }
            Event::Closed(_) => {}
            Event::PageChanged {
                index,
                label,
                of,
                section,
                ..
            } => {
                // ISO 32000-2 §12.4.2: "Page labels and page indices need not coincide". Where
                // the document states a label it is what a reader is meant to see — a page of
                // front matter is *iv*, not page four — so the index is shown beside it rather
                // than instead of it, because a title saying only `iv` cannot say `of 320`.
                let page = match label {
                    Some(label) => format!("{label} — page {} of {of}", index.saturating_add(1)),
                    None => format!("page {} of {of}", index.saturating_add(1)),
                };
                // §12.3.3's outline is a table of contents, so the item covering this page is
                // the section a reader is in. After the page number rather than before it,
                // because it is context for a position rather than the position itself.
                self.caption = match section {
                    Some(section) if !section.is_empty() => format!("{page} — {section}"),
                    _ => page,
                };
                self.retitle();
            }
            Event::NeedsRender(request) => {
                self.request = Some(request);
                self.acknowledged = false;
                // §12.4.4: the page a transition moves *to* is the one whose list has just
                // arrived, so this is where one that was armed can be drawn.
                if let Some(transition) = self.arming.take() {
                    self.begin_transition(transition);
                }
                self.redraw();
            }
            Event::Damage(_) => self.redraw(),
            // §12.6.4.8: printed rather than opened. What this program will not do is hand a
            // string a document controls to a browser, because that is a decision about this
            // machine and not about the document.
            Event::OpenUri { uri, .. } => println!("link: {uri}"),
            Event::NeedsFile { purpose, name, .. } => {
                let bytes = self.supply(purpose, &name);
                queue.push_back(Command::Supply { purpose, bytes });
            }
            // §12.4.4: the frames of it are drawn, since the three-hundred-and-ninety-third
            // session — by this host, because a transition is an animation over wall time and
            // `viewer-core` has no clock (rule 3). What arrives here is the *shape* of what to
            // draw; when to draw it is this window's, and a window that is not presenting shows
            // the page, which is the transition's own end state. ADR 0230.
            Event::Transition { transition, .. } => self.arm_transition(transition),
            // Rule 2 in one arm: the core produced the bytes and the host owns the filesystem.
            // Written beside the document with `.edited.pdf` appended rather than over it,
            // because overwriting somebody's file is a decision this program has not been given.
            Event::Extracted { name, bytes, .. } => self.write_extracted(&name, &bytes),
            Event::Saved { bytes, .. } => self.write_saved(&bytes),
            // What a host does with this is mark its window and ask before closing. This one
            // has no dialogue to ask with, so it marks the title and says so on the way past.
            Event::Dirty { dirty, .. } => {
                self.dirty = dirty;
                self.retitle();
                if dirty {
                    println!("note: this document has unsaved changes");
                }
            }
            Event::Reported { page, notes, .. } => {
                for note in &notes {
                    println!("note: {note}");
                }
                if page.is_some() {
                    self.retitle_incomplete(notes.len());
                }
            }
            Event::Refused { notes, .. } => Self::say_refused(&notes),
            Event::Searched {
                found,
                remaining,
                wrapped,
                ..
            } => self.searched(found, remaining, wrapped),
        }
    }

    /// A step of the search reported: what it found, and how much is left to read.
    ///
    /// The sentence goes into the bar rather than only onto stdout, because a person watching a
    /// thousand-page document being read is the reason a step is one page.
    fn searched(&mut self, found: Option<viewer_core::Found>, remaining: usize, wrapped: bool) {
        self.pages_left = remaining;
        self.find.note = match found {
            Some(found) => format!(
                "page {}{}",
                found.page.saturating_add(1),
                if wrapped { ", wrapped" } else { "" }
            ),
            None if remaining == 0 => "not in this document".to_owned(),
            None => format!("{remaining} page(s) left"),
        };
        if found.is_some() || remaining == 0 {
            println!(
                "note: search for {:?} — {}",
                self.find.needle, self.find.note
            );
        }
        // **Not on every step**, and the number is measured rather than chosen. A redraw here is a
        // whole window presented, and under `Xvfb` with lavapipe that is about 13 ms — so
        // repainting once per page made a 1023-page sweep **19.25 s** of presenting a bar whose
        // text changes by one digit. Once every 16 steps, plus the step that answers, is
        // **6.19 s** for the same sweep, with a progress count a person still sees moving. ADR
        // 0250 has the measurement; the cost in readability is this comment.
        if found.is_some() || remaining == 0 || remaining.is_multiple_of(SEARCH_REDRAW) {
            self.redraw();
        }
    }

    /// A key press while the find bar is open. Answers whether the bar took it.
    ///
    /// Every printable key is a character of the string, which is what makes this the first
    /// branch in the key handler rather than the last. Enter is *next* and shift-Enter is
    /// *previous* — one keystroke for each direction, which is what every find bar has —
    /// and Escape closes the bar and forgets the plan.
    fn find_key(&mut self, key: &Key<&str>) -> bool {
        match key {
            Key::Named(NamedKey::Escape) => {
                self.find.toggle();
                self.pages_left = 0;
                self.dispatch(Command::Find(Find::Stop));
            }
            Key::Named(NamedKey::Enter) => {
                if self.find.needle.is_empty() {
                    return true;
                }
                let needle = self.find.needle.clone();
                let direction = if self.shift {
                    FindDirection::Backward
                } else {
                    FindDirection::Forward
                };
                self.dispatch(Command::Find(Find::Start { needle, direction }));
            }
            Key::Named(NamedKey::Backspace) => {
                self.find.backspace();
                self.find.note.clear();
            }
            Key::Named(NamedKey::Space) => {
                self.find.typed(" ");
                self.find.note.clear();
            }
            Key::Character(text) if !text.is_empty() => {
                self.find.typed(text);
                self.find.note.clear();
            }
            // A key with no character and no meaning here — an arrow, a function key. Taken
            // anyway: while the bar has the keyboard, letting one through to turn the page would
            // move the document out from under the search a person is typing.
            _ => {}
        }
        self.redraw();
        true
    }

    /// §7.5.6's update, written beside the document rather than over it.
    ///
    /// Rule 2 in one method: the core produced the bytes and the host owns the filesystem.
    /// `.edited.pdf` appended rather than the file replaced, because overwriting somebody's
    /// document is a decision this program has not been given.
    fn write_saved(&self, bytes: &[u8]) {
        let path = self.path.with_extension("edited.pdf");
        match std::fs::write(&path, bytes) {
            Ok(()) => println!("saved {} bytes to {}", bytes.len(), path.display()),
            Err(error) => println!("note: cannot write {}: {error}", path.display()),
        }
    }

    /// What this window does about an operation the document restricted.
    ///
    /// Said rather than swallowed, and said as the *reader's* doing rather than as the
    /// document's: this host obeys what a file asserts unless it was started with
    /// `--ignore-restrictions`, and a person whose keystroke did nothing is owed both the clause
    /// and the way out. Trap 5 on the one path where this program declines on somebody else's
    /// instructions rather than on its own.
    ///
    /// **This is not a user interface for the levels**, which `CLAUDE.md` says is not to be built
    /// yet. It is the sentence a person needs, in the terminal this program already prints to.
    fn say_refused(notes: &[String]) {
        for note in notes {
            println!("note: {note}");
        }
        println!(
            "note: this reader is obeying that; --ignore-restrictions turns it off \
             (CLAUDE.md: a document's restrictions are the reader's to set)"
        );
    }

    /// §12.7.6.4's file, under the narrowest policy that still performs the action.
    ///
    /// The clause says a processor "shall import data … from a specified file" and specifies
    /// nothing about *which* files a document may name, because that is a property of the
    /// processor. So this states the policy, and it is a host's to state:
    ///
    /// - the name must be a single path component, so `../…` and any absolute path are refused;
    /// - it is resolved against the directory the open document is in, and nowhere else.
    ///
    /// Every refusal is printed, which is trap 5 on the one path where a click can decline.
    fn supply(&self, purpose: Purpose, name: &str) -> Option<Vec<u8>> {
        let Purpose::ImportData = purpose;
        let directory = self.directory.as_ref().or_else(|| {
            println!("import-data: declined — the document is not in a known directory");
            None
        })?;
        // One component, checked as a path rather than as a string, so that a separator this
        // platform recognises and this program does not cannot slip through.
        let named = std::path::Path::new(name);
        let mut components = named.components();
        let (Some(std::path::Component::Normal(single)), None) =
            (components.next(), components.next())
        else {
            println!("import-data: declined — {name} is not a plain file name beside the document");
            return None;
        };
        let path = directory.join(single);
        match std::fs::read(&path) {
            Ok(bytes) => Some(bytes),
            Err(error) => {
                println!("import-data: cannot read {}: {error}", path.display());
                None
            }
        }
    }

    /// Asks the window to draw again, where there is one.
    fn redraw(&self) {
        if let Some(state) = self.state.as_ref() {
            state.window.request_redraw();
        }
    }

    /// Puts the caption in the title bar.
    fn retitle(&self) {
        if let Some(state) = self.state.as_ref() {
            let mark = if self.dirty { "• " } else { "" };
            state
                .window
                .set_title(&format!("{mark}{} — {}", self.named(), self.caption));
        }
    }

    /// What the title bar calls the document — §12.2's `/DisplayDocTitle`.
    ///
    /// Table 147: "[a] flag specifying whether the window's title bar should display the
    /// document title taken from the `dc:title` entry of the XMP metadata stream … If false, the
    /// title bar should instead display the name of the PDF file containing the document."
    ///
    /// **The clause is obeyed as written since the two-hundred-and-ninety-fourth session.** It
    /// names `dc:title` and nothing else, and `pdf_model::xmp` reads it, so that is what a
    /// document asking for its title gets. §14.3.3's `/Info /Title` is the *fallback* now rather
    /// than the substitution: it is used where the document states no metadata stream, where the
    /// stream states no `dc:title`, or where the stream could not be read — and the last of those
    /// three is printed, because it is the only one where this program failed at something.
    ///
    /// Table 349's NOTE 1 is why the fallback is a reading rather than a guess: "[t]he `dc:title`
    /// entry in the document's metadata stream **can be used to represent** the document's
    /// title." Measured over the corpus, 93 documents state a title in both places and one
    /// disagrees, so the ranking is what decides a single file (ADR 0186).
    fn named(&self) -> &str {
        if !self.display_doc_title() {
            return &self.title;
        }
        let stated = self
            .metadata
            .as_ref()
            .and_then(|metadata| metadata.as_ref().ok())
            .and_then(pdf_model::xmp::Xmp::title)
            .or(self.information.title.as_deref());
        stated
            .filter(|title| !title.is_empty())
            .unwrap_or(&self.title)
    }

    /// Table 147's `/DisplayDocTitle`, **default false**.
    fn display_doc_title(&self) -> bool {
        matches!(
            self.viewer.query(Query::Preferences),
            Answer::Preferences(preferences) if preferences.display_doc_title
        )
    }

    /// Brings the accessibility bridge up, once, and keeps it in step with the page.
    ///
    /// **Called from the first present and from every one after it, and never before one.** That
    /// ordering is `CLAUDE.md`'s startup rule made concrete: `Bridge::new` spawns a thread that
    /// connects to the session bus, and page one may not wait behind a D-Bus round trip for a
    /// screen reader that is probably not there. What it costs after the first frame is one
    /// comparison per frame, because the structure of a page does not change when it is scrolled.
    ///
    /// **It cost 2.0 ms on average and 3.9 at worst on every page turn, and the three-hundred-
    /// and-ninety-first session found out where** (ADR 0228). Not §14.7's tree: `Query::
    /// AccessibilityTree` is 0.13 to 0.25 ms on this document and the whole of [`App::speak`]
    /// — both queries, the tree built and the bridge given it — is 0.17 to 0.33. It was
    /// [`App::place_window`], at **1.8 to 3.2 ms**, which is two synchronous X11 round trips for
    /// a window position that **does not change when a page turns**. So it is asked where it can
    /// change — when the bridge comes up, when the window moves, and when it is resized — and
    /// the page turn is left with the query that is actually about the page.
    fn attend(&mut self) {
        if self.accessibility.is_none() {
            self.accessibility = Some(viewer_accessibility::Bridge::new());
            // **One of these two sentences used to be false, and ADR 0227 settled which.**
            // The Windows trace printed `note: this build has no accessibility bridge` at line
            // 2 and `trace: accessibility bridge up` at line 46. The note was right:
            // `Bridge::new` builds an `accesskit_unix::Adapter` on Linux and, on every other
            // platform, a struct with no adapter in it at all — so what came "up" there was a
            // tree with nowhere to publish it. The line now asks `shortfall`, which is the same
            // function the note came from, so the two cannot disagree again.
            let said = match viewer_accessibility::Bridge::shortfall() {
                None => "accessibility bridge up",
                Some(_) => {
                    "accessibility: no bridge on this platform — §14.7's tree is still built \
                     and published to nothing, and the note above says so"
                }
            };
            self.trace.say(Topic::Access, format_args!("{said}"));
            // The one place a page turn used to pay for this: a bridge that has just come up has
            // never been told where the window is, and this is the frame that knows there is one.
            self.place_window();
        }
        let Some((width, height, _)) = self.window() else {
            return;
        };
        let Answer::Page { index: page, .. } = self.viewer.query(Query::CurrentPage) else {
            return;
        };
        if self.spoken == Some((page, width, height)) {
            return;
        }
        self.spoken = Some((page, width, height));
        self.speak();
    }

    /// Tells the bridge where the window is, which is what AT-SPI needs to place a node.
    ///
    /// A node's bounds cross in the window's own pixels and AT-SPI reports them in the screen's,
    /// so the adapter adds the window's position. Under Wayland an application cannot learn its
    /// own position and winit says so by refusing the call; there is nothing to be done about
    /// that here and nothing is claimed instead.
    ///
    /// **Called when the answer can have changed and not otherwise**: when the bridge comes up,
    /// on `WindowEvent::Moved` and on `WindowEvent::Resized`. It used to be called on every page
    /// turn, where the two winit calls below are two synchronous X11 round trips — 1.8 to 3.2 ms
    /// — for a number a page turn cannot move (ADR 0228).
    #[expect(
        clippy::cast_precision_loss,
        reason = "a window's position and size in device pixels; f32 is exact to 2^24"
    )]
    fn place_window(&mut self) {
        // Asked of the crate that has the adapter rather than decided here, because the answer
        // is about AT-SPI's screen coordinates rather than about having a bridge at all — see
        // `Bridge::wants_window_bounds`, which is where the two part company when `doc/todo/31`
        // wires the Windows and macOS adapters in.
        if !viewer_accessibility::Bridge::wants_window_bounds() || self.accessibility.is_none() {
            return;
        }
        let Some(state) = self.state.as_ref() else {
            return;
        };
        let outer = state.window.outer_size();
        let inner = state.window.inner_size();
        let (Ok(outer_at), Ok(inner_at)) =
            (state.window.outer_position(), state.window.inner_position())
        else {
            return;
        };
        let outer = (
            outer_at.x as f32,
            outer_at.y as f32,
            (outer_at.x.saturating_add_unsigned(outer.width)) as f32,
            (outer_at.y.saturating_add_unsigned(outer.height)) as f32,
        );
        let inner = (
            inner_at.x as f32,
            inner_at.y as f32,
            (inner_at.x.saturating_add_unsigned(inner.width)) as f32,
            (inner_at.y.saturating_add_unsigned(inner.height)) as f32,
        );
        if let Some(bridge) = self.accessibility.as_mut() {
            bridge.placed(outer, inner);
        }
    }

    /// Hands §14.7's structure for the page being shown to the platform's accessibility API.
    ///
    /// **The fifth of the five things `doc/ui-boundary.md` lists as blocked on the
    /// `viewer-core` boundary, and the last.** `Query::AccessibilityTree` has answered since the
    /// hundred-and-forty-ninth session and nothing asked; this asks. What crosses is
    /// `viewer-accessibility`'s business — §14.8.4's types onto AccessKit's roles, and AT-SPI
    /// underneath that — and what is this host's is the three things only a host knows: what the
    /// window is called, which page is showing, and what the page could not draw.
    ///
    /// **Everything is copied before the bridge is touched.** `Query::Reports` borrows the
    /// viewer, and the bridge is a field of the same struct; owning the answer first is what lets
    /// both be reached in one function without the borrow checker having to be argued with.
    ///
    /// Does nothing until [`App::accessibility`] exists, which is after the first present.
    fn speak(&mut self) {
        if self.accessibility.is_none() {
            return;
        }
        let document = self.named().to_owned();
        let window = format!("{document} — {}", self.caption);
        #[expect(
            clippy::cast_precision_loss,
            reason = "a viewport in device pixels; f32 is exact to 2^24 and no display is"
        )]
        let viewport = self.window().map_or((0.0, 0.0), |(width, height, _)| {
            (width as f32, height as f32)
        });
        let (page, label, pages) = match self.viewer.query(Query::CurrentPage) {
            Answer::Page {
                index, label, of, ..
            } => (index, label, of),
            _ => (0, None, 0),
        };
        let reports: Vec<String> = match self.viewer.query(Query::Reports) {
            Answer::Reports(notes) => notes.to_vec(),
            _ => Vec::new(),
        };
        let nodes = match self.viewer.query(Query::AccessibilityTree) {
            Answer::Accessibility(nodes) => nodes,
            _ => Vec::new(),
        };
        self.trace.say(
            Topic::Access,
            format_args!(
                "accessibility: {} element(s), {} report(s) on page {}",
                nodes.len(),
                reports.len(),
                page.saturating_add(1)
            ),
        );
        let view = viewer_accessibility::PageView {
            window: &window,
            document: &document,
            page,
            label: label.as_deref(),
            pages,
            viewport,
            nodes: &nodes,
            reports: &reports,
        };
        if let Some(bridge) = self.accessibility.as_mut() {
            bridge.publish(&view);
            // An action nobody carries out is said out loud rather than dropped. The tree this
            // program publishes declares no actions at all, so this list is expected to be empty
            // — and a line here would mean a client asked for something anyway, which is worth
            // knowing (trap 5).
            for asked in bridge.requested() {
                println!(
                    "note: an assistive technology asked for {:?} on node {:?}, which this host \
                     does not carry out",
                    asked.action, asked.node
                );
            }
        }
    }

    /// Adds what the page could not draw to the title bar.
    ///
    /// A count rather than the list: a page may report dozens of items and a title bar that
    /// scrolls off the screen tells a person less than a number does. The items themselves are
    /// printed, in the core's own words.
    fn retitle_incomplete(&self, items: usize) {
        if let Some(state) = self.state.as_ref() {
            state.window.set_title(&format!(
                "{} — {} — incomplete: {items} item(s) not drawn",
                self.title, self.caption
            ));
        }
    }

    /// Draws the outstanding request onto the surface and presents it.
    ///
    /// Returns what to tell the core, or `None` where there is nothing to tell it: a redraw the
    /// swapchain gave back never reached a screen, and saying it did would leave the window
    /// showing the last page until something else happened to change.
    ///
    /// **That last sentence is the whole reason this returns an outcome rather than a `bool`.**
    /// A draw that *fails* used to print a line to stdout and answer `Presented`, so the core
    /// recorded the page as shown, never asked again, and the window kept the previous page under
    /// a title bar naming the new one. A person looking at the window saw a page that would not
    /// change and no reason why. Trap 5, on a path a person reaches with an arrow key.
    /// The selection's shapes, in the window's own pixels, or `None` when nothing is selected.
    ///
    /// Interactive chrome crosses as geometry, not pixels: the core hands over the shapes and this
    /// host draws them in its own colour. A native one would use macOS's selection colour, KDE's
    /// accent or the Windows highlight brush; this one has no theme to ask, so it picks a blue and
    /// says so.
    fn selection_list(
        &self,
        edge: f32,
        width: u32,
        height: u32,
    ) -> Option<pdf_render::DisplayList> {
        let mut quads = match self.viewer.query(Query::Selection) {
            Answer::Selected(selection) => selection.quads,
            _ => Vec::new(),
        };
        // The quads are device pixels of the *page's* viewport, which begins where the panel
        // ends. One addition here rather than a second coordinate space in the core.
        for quad in &mut quads {
            for x in quad.iter_mut().step_by(2) {
                *x += edge;
            }
        }
        // The number every part of `doc/todo/13` turned on: the frame the compositor refused was
        // 63 quads, and a present cost 1.9 ms a quad before it. Kept in the tree so that a
        // selection's cost stays visible rather than being rediscovered. On stdout with every
        // other trace line since the three-hundred-and-ninetieth: it was the one on stderr, and
        // PowerShell wrapped each of its lines in six of its own in the trace that raised
        // ADR 0227.
        self.trace.say(
            Topic::Selection,
            format_args!("SELECTION quads {}", quads.len()),
        );
        highlight_list(&quads, width, height)
    }

    /// Every occurrence of the find bar's string on the page being shown.
    ///
    /// Asked on every frame while the bar is open, because it is a query over a readback that
    /// already exists — `Query::Find` is this page and `Command::Find` is the document, and only
    /// the second interprets anything. Drawn *under* the selection, so that the occurrence a
    /// person is on reads as the one the selection colour is over.
    fn matches_list(&self, edge: f32, width: u32, height: u32) -> Option<pdf_render::DisplayList> {
        if !self.find.shown || self.find.needle.is_empty() {
            return None;
        }
        let Answer::Found(occurrences) = self.viewer.query(Query::Find(&self.find.needle)) else {
            return None;
        };
        let mut quads: Vec<[f32; 8]> = occurrences.into_iter().flatten().collect();
        // Device pixels of the *page's* viewport, which begins where the panel ends — the same
        // one addition `selection_list` makes.
        for quad in &mut quads {
            for x in quad.iter_mut().step_by(2) {
                *x += edge;
            }
        }
        match_list(&quads, width, height)
    }

    /// §12.5.6.14's popup windows, over the page and under the sidebar.
    ///
    /// The core says which windows are open, where they are and what they say; this host decides
    /// what a window looks like, because the clause describes none of that — see
    /// `chrome::popup_windows`. Over the page and *under* the panel, which is the order the
    /// overlays already state: a window belongs to the document and the sidebar belongs to the
    /// program.
    fn popup_list(&self, edge: f32, width: u32, height: u32) -> Option<pdf_render::DisplayList> {
        let chrome = self.chrome.as_ref()?;
        let Answer::Popups(mut windows) = self.viewer.query(Query::Popups) else {
            return None;
        };
        // Device pixels of the *page's* viewport, which begins where the panel ends — the same
        // one addition `selection_list` makes, and for the same reason.
        for window in &mut windows {
            for x in window.quad.iter_mut().step_by(2) {
                *x += edge;
            }
        }
        let scale = self.window().map_or(1.0, |(_, _, scale)| scale);
        viewer_ui::chrome::popup_windows(chrome, &windows, width, height, scale)
    }

    /// §12.5.1's focus ring: a stroked box round whatever the tab key last landed on.
    ///
    /// The clause lets a processor walk the annotations with the tab key and says nothing about
    /// showing which one a person is on — so the ring is entirely this host's, in this host's own
    /// colour, and a native one would use its platform's focus ring instead. What it is *not* is
    /// this host's arithmetic: `Query::Focus` answers with the quadrilateral in the viewport's
    /// own pixels, for the same reason `Query::Selection` does.
    fn focus_list(&self, edge: f32, width: u32, height: u32) -> Option<pdf_render::DisplayList> {
        let Answer::Focus { quad, .. } = self.viewer.query(Query::Focus) else {
            return None;
        };
        let mut quad = quad;
        for x in quad.iter_mut().step_by(2) {
            *x += edge;
        }
        #[expect(
            clippy::cast_precision_loss,
            reason = "window dimensions are far below f32's exact integer range"
        )]
        let mut list = pdf_render::DisplayList::new(Size::new(width as f32, height as f32));
        let mut path = Path::new();
        for (index, corner) in quad.chunks_exact(2).enumerate() {
            let point = Point::new(corner[0], corner[1]);
            path.push(if index == 0 {
                PathCommand::MoveTo(point)
            } else {
                PathCommand::LineTo(point)
            });
        }
        path.push(PathCommand::Close);
        list.push(DrawCommand::Stroke {
            path: Arc::new(path),
            transform: Transform::IDENTITY,
            stroke: pdf_render::Stroke {
                width: FOCUS_RING_WIDTH,
                ..pdf_render::Stroke::default()
            },
            paint: Paint::Solid(FOCUS_RING),
            clip: None,
            mask: None,
            blend: BlendMode::Normal,
        });
        Some(list)
    }

    /// What is selected *inside* a field's value, in the window's own pixels.
    ///
    /// The same blue `Query::Selection`'s shapes are drawn in, deliberately: a person sweeping
    /// text on the page and a person sweeping text in a field are doing one thing, and two colours
    /// would say they were doing two. Where the shapes come from is the core — `Query::FieldSelection`
    /// answers one quadrilateral per line, because §12.7.5.3's Multiline flag lets the layout break
    /// a value where this host cannot see (ADR 0225).
    ///
    /// `None` while nothing is selected, which is a caret's ordinary state: the two offsets are
    /// equal and the core answers with no shapes at all.
    fn field_selection_list(
        &self,
        edge: f32,
        width: u32,
        height: u32,
    ) -> Option<pdf_render::DisplayList> {
        let typing = self.typing?;
        let (from, to) = typing.range();
        if from == to {
            return None;
        }
        let Answer::FieldSelection(mut quads) = self.viewer.query(Query::FieldSelection {
            at: typing.at,
            from,
            to,
        }) else {
            return None;
        };
        // Device pixels of the *page's* viewport, which begins where the panel ends — the same one
        // addition `selection_list`, `focus_list` and `caret_list` make.
        for quad in &mut quads {
            for x in quad.iter_mut().step_by(2) {
                *x += edge;
            }
        }
        highlight_list(&quads, width, height)
    }

    /// The caret: a line where the next character will be drawn, while a field has the keyboard.
    ///
    /// **The standard states no caret**, and §12.5.6.11's caret *annotation* is a different thing
    /// entirely — so its width, its colour and whether it blinks are this host's, exactly as
    /// §12.5.1's focus ring is. This one is a steady line two pixels wide: a blink needs a clock,
    /// and `viewer-core` has none by rule 3, so a host that wanted one would drive it from its own
    /// timer. What is *not* this host's is where it goes — `Query::Caret` answers that from
    /// §12.7.4.3's own layout, because a host laying the value out again to find the place would
    /// be a second opinion about the field's font, its auto-sizing and its wrapping. ADR 0211.
    fn caret_list(&self, edge: f32, width: u32, height: u32) -> Option<pdf_render::DisplayList> {
        let typing = self.typing?;
        let Answer::Caret { from, to } = self.viewer.query(Query::Caret {
            at: typing.at,
            offset: typing.caret,
        }) else {
            return None;
        };
        #[expect(
            clippy::cast_precision_loss,
            reason = "window dimensions are far below f32's exact integer range"
        )]
        let mut list = pdf_render::DisplayList::new(Size::new(width as f32, height as f32));
        let mut path = Path::new();
        // Device pixels of the *page's* viewport, which begins where the panel ends — the same
        // one addition `selection_list` and `focus_list` make.
        path.push(PathCommand::MoveTo(Point::new(from.0 + edge, from.1)));
        path.push(PathCommand::LineTo(Point::new(to.0 + edge, to.1)));
        list.push(DrawCommand::Stroke {
            path: Arc::new(path),
            transform: Transform::IDENTITY,
            stroke: pdf_render::Stroke {
                width: CARET_WIDTH,
                ..pdf_render::Stroke::default()
            },
            paint: Paint::Solid(CARET),
            clip: None,
            mask: None,
            blend: BlendMode::Normal,
        });
        Some(list)
    }

    /// Starts or stops driving §12.4.4's clock, which is the whole of what presentation mode is
    /// in this program.
    ///
    /// ADR 0135 decided that `viewer-core` has no presentation *state*: "is a presentation
    /// running" is answered by whether something is driving the clock, and a host that stops
    /// presenting stops ticking. So this is a host field and a key, and nothing crosses the
    /// boundary but `Command::Tick`.
    ///
    /// Full screen is deliberately not part of it. §12.4.4.1 says a processor "may allow a
    /// document to be displayed in the form of a presentation or slide show" and says nothing
    /// about a window; what the clause states is the advance timing and the transition, and those
    /// are what this draws.
    fn present_or_stop(&mut self) {
        if self.presentation.take().is_some() {
            println!("presentation: stopped — no clock is being driven");
            self.redraw();
            return;
        }
        let now = std::time::Instant::now();
        self.presentation = Some(Presentation {
            ticked: now,
            wake: now,
            playing: None,
        });
        println!(
            "presentation: running — §12.4.4's /Dur advances the page and its /Trans is drawn"
        );
        self.redraw();
    }

    /// Tells the core how long the page has been up, where a presentation is running.
    ///
    /// **The clock is held while a transition is being drawn**, and that is the clause's own
    /// arithmetic rather than a convenience: §12.4.4.1's EXAMPLE describes "a page to be
    /// displayed for 5 seconds" whose 3.5-second transition happens "[b]efore the page is
    /// displayed", so the transition is not part of the display duration it precedes. A tick
    /// during it would spend the new page's `/Dur` on the effect that introduces it.
    fn drive_the_clock(&mut self) {
        let Some(presentation) = self.presentation.as_mut() else {
            return;
        };
        if presentation.playing.is_some() {
            presentation.ticked = std::time::Instant::now();
            return;
        }
        let now = std::time::Instant::now();
        let elapsed = now.saturating_duration_since(presentation.ticked);
        presentation.ticked = now;
        let millis = u32::try_from(elapsed.as_millis()).unwrap_or(u32::MAX);
        if millis == 0 {
            return;
        }
        self.dispatch(Command::Tick { millis });
    }

    /// Takes `transition` to be drawn as soon as there is a page to draw it to, or says why this
    /// program will not draw it at all.
    ///
    /// Two rasters are taken per transition and none per frame: the page being left, as it was
    /// last presented, and the page arriving, at the same size. Both are drawn by
    /// [`CpuRasterizer`] — the one rasteriser this host can ask for *pixels* rather than for a
    /// present — and each crosses to the graphics device once, because [`pdf_render::Image`]
    /// holds its samples behind an `Arc` and quorra's caches are keyed by that pointer.
    ///
    /// The cost is therefore two page renders at the start of a transition and two image draws
    /// per frame after it, which is the trade this host makes deliberately: a transition frame
    /// that re-rasterised both pages would pay a page's interpretation sixty times a second.
    fn arm_transition(&mut self, transition: pdf_model::navigation::Transition) {
        let Some((width, height, _)) = self.window() else {
            return;
        };
        // A window that is not presenting draws the page, which is the transition's end state.
        if self.presentation.is_none() {
            println!(
                "note: transition: {:?} over {} s — nothing is presenting, so the page is shown \
                 at once (press p)",
                transition.style, transition.duration
            );
            return;
        }
        let viewport = Rect::from_corners(Point::new(0.0, 0.0), whole(width, height));
        if viewer_core::transition::frame(&transition, viewport, 0.0).is_none() {
            // The core has already said *why* through `Event::Reported`; a second sentence here
            // would say it twice.
            return;
        }
        // **Armed rather than begun, and the ordering is the reason.** `Viewer::handle` settles
        // *after* the command that turned the page, so the events arrive as page change,
        // transition, render request — and the arriving page's display list is in the last of
        // the three. A transition begun here would have rasterised the page being left twice
        // and animated it against itself, which is what the window showed before this was found.
        // §12.4.4.1's transition is one *to* a page, so waiting for that page's own request is
        // the clause's own order as well as this host's.
        self.arming = Some(transition);
    }

    /// Draws the armed transition between the page last presented and the one just asked for.
    ///
    /// Two rasters are taken here and none per frame: see [`App::arm_transition`] for why this
    /// is not the moment the event arrived.
    fn begin_transition(&mut self, transition: pdf_model::navigation::Transition) {
        let began = std::time::Instant::now();
        let Some((width, height, _)) = self.window() else {
            return;
        };
        let Some((list, target)) = self.presented.clone() else {
            return;
        };
        let Some(request) = self.request.clone() else {
            return;
        };
        let origin = match self.viewer.query(Query::PageGeometry(request.page)) {
            Answer::Geometry(geometry) => geometry.origin,
            _ => (0.0, 0.0),
        };
        #[expect(
            clippy::cast_precision_loss,
            reason = "a panel width in pixels, which is hundreds"
        )]
        let edge = self.inset() as f32;
        // The same composition `present` makes, because the page arriving has to be drawn where
        // it is about to be presented — otherwise the last frame of the transition would move.
        let arriving = TargetSpec {
            width,
            height,
            transform: request
                .target
                .transform
                .then(Transform::translate(origin.0 + edge, origin.1)),
        };
        let (Some(outgoing), Some(incoming)) = (face(&list, target), face(&request.list, arriving))
        else {
            println!(
                "note: transition: {:?} was named but the pages behind it would not rasterise, \
                 so the page is shown at once",
                transition.style
            );
            return;
        };
        // The cost of starting one, which is where all of a transition's page work is: a frame
        // after this draws two images and interprets nothing.
        self.trace.say(
            Topic::Frames,
            format_args!(
                "TRANSITION {:?} over {} s: two {width}x{height} pages rasterised in {:?}",
                transition.style,
                transition.duration,
                began.elapsed()
            ),
        );
        if let Some(presentation) = self.presentation.as_mut() {
            presentation.playing = Some(Playing {
                transition,
                began: std::time::Instant::now(),
                outgoing,
                incoming,
            });
        }
        self.redraw();
    }

    /// The frame of a transition in flight, or `None` when there is nothing being drawn.
    ///
    /// Ends the transition at `/D` seconds, which is the one place this host decides that time
    /// has run out: the fraction handed to `viewer_core::transition::frame` is elapsed over
    /// duration, linear, because Table 164 states a duration and no curve.
    fn transition_frame(&mut self, width: u32, height: u32) -> Option<pdf_render::DisplayList> {
        let presentation = self.presentation.as_mut()?;
        let playing = presentation.playing.as_ref()?;
        let elapsed = playing.began.elapsed().as_secs_f32();
        let progress = if playing.transition.duration > 0.0 {
            elapsed / playing.transition.duration
        } else {
            1.0
        };
        if progress >= 1.0 {
            presentation.playing = None;
            // The clock restarts where the transition ends, so the page gets the whole of its
            // own `/Dur` — see `drive_the_clock`.
            presentation.ticked = std::time::Instant::now();
            return None;
        }
        let viewport = Rect::from_corners(Point::new(0.0, 0.0), whole(width, height));
        let shaped = viewer_core::transition::frame(&playing.transition, viewport, progress)?;
        match shaped.draw(viewport, &playing.outgoing, &playing.incoming) {
            Ok(list) => Some(list),
            Err(problem) => {
                // Not reachable from a frame — the largest one adds four clips — and said rather
                // than swallowed for the reason every refusal in this tree is.
                println!("note: transition: this frame would not draw: {problem}");
                presentation.playing = None;
                None
            }
        }
    }

    /// What this frame draws: a transition's own picture, or the page itself.
    ///
    /// A transition frame is already in the window's pixels — it is two page rasters placed by
    /// [`viewer_core::transition`] — so it draws at the identity transform where a page draws
    /// through its own placement.
    fn frame_to_draw(
        &mut self,
        request: &RenderRequest,
        target: TargetSpec,
        width: u32,
        height: u32,
    ) -> (Option<pdf_render::DisplayList>, TargetSpec) {
        if let Some(frame) = self.transition_frame(width, height) {
            return (
                Some(frame),
                TargetSpec {
                    width,
                    height,
                    transform: Transform::IDENTITY,
                },
            );
        }
        // What the screen is about to show, kept so that the next transition has a page to move
        // *from*. Only the page itself: a transition frame is already a picture of two of them.
        self.presented = Some((Arc::clone(&request.list), target));
        (None, target)
    }

    /// `stages` is filled in as the frame goes: see [`Stages`] for why one number was not enough.
    fn present(&mut self, stages: &mut Stages) -> Option<Rendered> {
        let began = std::time::Instant::now();
        // §12.3.4's list is built here and nowhere else: this is the one place that holds
        // `&mut self` and runs before the panel is drawn.
        self.ensure_pages();
        let request = self.request.clone()?;
        stages.page = request.page.saturating_add(1);
        stages.commands = request.list.commands().len();
        // Where the page sits in the window: the core centres it and scrolls it, and the host
        // draws it there by composing that offset into the target's own transform.
        let origin = match self.viewer.query(Query::PageGeometry(request.page)) {
            Answer::Geometry(geometry) => geometry.origin,
            _ => (0.0, 0.0),
        };
        let (width, height) = {
            let state = self.state.as_ref()?;
            state.size
        };
        #[expect(
            clippy::cast_precision_loss,
            reason = "a panel width in pixels, which is hundreds"
        )]
        let edge = self.inset() as f32;
        let target = TargetSpec {
            width,
            height,
            transform: request
                .target
                .transform
                .then(Transform::translate(origin.0 + edge, origin.1)),
        };

        // §12.4.4: a transition in flight substitutes its own picture of *two* pages for the one,
        // and it is a display list, so everything below this line is unchanged by it — which is
        // the point of shaping a frame in `viewer_core::transition` rather than compositing here.
        let (playing, target) = self.frame_to_draw(&request, target, width, height);
        let list = playing.as_ref().map_or(&*request.list, |frame| frame);

        let chrome = Overlays::of(self, edge, width, height);
        let overlays = chrome.lists();
        stages.host = began.elapsed();

        let state = self.state.as_mut()?;
        let drawn = match &mut state.surface {
            // No device to ask. Not a refusal either: the software surface below is this run's
            // only path, and calling it a failure would print a note on every frame.
            Surface::Processor(_) => Err(String::new()),
            Surface::Device(presenter) => {
                // Which lane draws this frame's coverage, decided from this frame's
                // magnification: see `coverage_for`. Set every frame rather than when it
                // changes, because it is a field write and tracking the change would be
                // more state than the thing it saved.
                presenter.set_coverage(coverage_for(target.transform));
                let outcome = presenter.present(PresentFrame {
                    width,
                    height,
                    page: Some((list, target)),
                    raster: None,
                    overlays: &overlays,
                });
                // Read back whatever the frame cost before anything is decided about it: a
                // refusal has an accounting too, and it is the one a person most wants.
                stages.gpu = presenter.last_frame();
                match outcome {
                    Ok(()) => Ok(()),
                    // Swapchain states are events, not failures: nothing was presented,
                    // nothing is stale, and the processor cannot help a window that is
                    // not presentable — so these return rather than fall back.
                    Err(render_quorra::QuorraRasterError::Render(
                        quorra_gpu::RenderError::SurfaceUnavailable { reason },
                    )) => {
                        return match reason {
                            quorra_gpu::SurfaceProblem::Outdated
                            | quorra_gpu::SurfaceProblem::Lost => {
                                state.window.request_redraw();
                                None
                            }
                            quorra_gpu::SurfaceProblem::Timeout
                            | quorra_gpu::SurfaceProblem::Occluded => None,
                            quorra_gpu::SurfaceProblem::Validation => {
                                Some(Rendered::Failed("swapchain validation failed".to_owned()))
                            }
                        };
                    }
                    Err(error) => Err(error.to_string()),
                }
            }
        };
        if let Err(problem) = drawn {
            let fell_back = std::time::Instant::now();
            let second = on_the_processor(&mut state.surface, list, target, &overlays);
            stages.fallback = fell_back.elapsed();
            if let Err(second) = second {
                return Some(Rendered::Failed(if problem.is_empty() {
                    second
                } else {
                    format!("the graphics device {problem}, and {second}")
                }));
            }
            // Reported when there *was* a device that refused. A page drawn by the slower of two
            // backends is a fact about this build worth saying out loud, and saying it is what
            // would have made the hundred-and-forty-second session's report a sentence rather
            // than a mystery — but under `--cpu` there is nothing to report, which is why the
            // empty `problem` above is a sentinel rather than a message.
            if !problem.is_empty() {
                println!(
                    "note: page {}: the graphics device {problem}, so it was drawn on the \
                     processor instead",
                    request.page.saturating_add(1)
                );
            }
        }
        Some(Rendered::Presented)
    }

    /// Brings up whatever will put pixels on this window, or says why nothing can.
    ///
    /// **Two paths, and `--cpu` chooses between them.** With the flag, no `wgpu::Instance` is
    /// created, no adapter is enumerated and no device is made — the driver is never loaded, which
    /// is what makes the flag an answer to a driver that faults while loading. Without it, the
    /// device is brought up as it always was, and the processor's path is what a machine falls to
    /// when the device will not come at all.
    ///
    /// `None` means neither worked, which is the one launch failure this program cannot show a
    /// page past.
    fn bring_up(&mut self, window: &Arc<Window>) -> Option<Surface> {
        if self.processor {
            let surface = Self::software(window);
            self.launch.mark("software surface");
            return surface;
        }

        // Shaders compile on a background thread and nothing here waits for them —
        // `CLAUDE.md`'s rule, since page one goes to the graphics device: what bringing the
        // device up costs is part of time-to-first-page, so it is measured rather than
        // assumed. The presenter reports uncaptured device errors itself, for the same
        // silent-window reason the Vello host did.
        let mut instance = self
            .instancing
            .take()
            .map(|thread| thread.join().expect("the thread creating the instance"));
        self.launch.mark("graphics instance");
        let began = std::time::Instant::now();
        let mut attempt = match instance.as_ref() {
            Some(instance) => QuorraPresenter::with_instance(instance, window.clone()),
            None => QuorraPresenter::new(window.clone()),
        };

        // **A default gives way; a flag does not.** This arm is only reachable where this build
        // restricted the backends by itself — today that is Windows and DX12 — and the machine
        // turned out to have no adapter behind it. Refusing there would be this project's guess
        // deciding that somebody's machine cannot start, so it is a note and a second attempt
        // with everything. `QuorraPresenter::new` makes its own all-backends instance, which is
        // why the old one is dropped rather than reused.
        if attempt.is_err()
            && !self.backend_asked_for
            && let Some(named) = self.backend.take()
        {
            println!(
                "note: no graphics adapter behind the {} backend, which is the one this build \
                 asks for first on this platform, so every backend was offered instead",
                named.name()
            );
            instance = None;
            attempt = QuorraPresenter::new(window.clone());
        }

        let presenter = match attempt {
            Ok(presenter) => presenter,
            Err(problem) => return self.no_device(window, instance.as_ref(), &problem),
        };
        let brought_up = began.elapsed();
        self.launch.mark("graphics device");
        if self.trace.on(Topic::Launch) {
            let startup = presenter.startup();
            // Two lines about one choice, and they answer different questions. The first is what
            // was *asked for* — which is a fact about this command line — and the second ends in
            // the backend that was actually chosen, because quorra's adapter description carries
            // it: `llvmpipe (LLVM 22.1.8, 256 bits) (Cpu, Vulkan)`. A person diagnosing a driver
            // crash needs both, and before the three-hundred-and-eighty-fourth session there was
            // no way to ask for the first at all.
            self.trace.say(
                Topic::Launch,
                format_args!("backend asked for: {}", self.backend_description()),
            );
            self.trace.say(
                Topic::Launch,
                format_args!("rendering with {}", presenter.adapter_description()),
            );
            self.trace.say(
                Topic::Launch,
                format_args!(
                    "device up in {brought_up:?} — instance {:?}, surface {:?}, adapter {:?}, \
                     device {:?}, pipelines {}",
                    startup.instance_creation,
                    startup.surface_creation,
                    startup.adapter_selection,
                    startup.device_creation,
                    startup
                        .pipeline_compilation
                        .map_or_else(|| "still compiling".to_owned(), |d| format!("{d:?}"))
                ),
            );
        }
        Some(Surface::Device(Box::new(presenter)))
    }

    /// What this run asked the graphics stack for, in words, for `--trace` and for a refusal.
    fn backend_description(&self) -> String {
        match (self.backend, self.backend_asked_for) {
            (Some(named), true) => format!("{} (--backend)", named.name()),
            (Some(named), false) => format!("{} (this platform's default)", named.name()),
            (None, _) => "every backend this build has".to_owned(),
        }
    }

    /// The window written to by the processor, or a sentence saying why there is not one.
    fn software(window: &Arc<Window>) -> Option<Surface> {
        match SoftwareSurface::new(Arc::clone(window)) {
            Ok(surface) => Some(Surface::Processor(surface)),
            Err(problem) => {
                eprintln!(
                    "this window cannot be drawn on without a graphics device: {problem}\n\
                     The software path is compiled for X11 and Wayland here; a session that is \
                     neither has no way to show a page without a device."
                );
                None
            }
        }
    }

    /// What to say — and what to do — when the graphics device will not come up.
    ///
    /// **This was `.expect("presenter creation")` until the three-hundred-and-eighty-fourth
    /// session**, which is `CLAUDE.md` principle 1's rule about panics in the one place a person
    /// most needs a sentence: a device that will not come up is a fact about the machine, not a
    /// defect in this program, and the shape to use is `Confinement::shortfall`'s — name the
    /// stage, name what was seen, name what to try.
    ///
    /// The two outcomes are deliberately different. A backend a **person** named is honoured or
    /// refused, because "this machine has no DX12 adapter" is an answer to the question they
    /// asked and starting on the stack they were avoiding is not. A device that failed with no
    /// backend named is a broken machine rather than a mistaken command line, so the page is
    /// drawn on the processor and the note says so.
    fn no_device(
        &self,
        window: &Arc<Window>,
        instance: Option<&quorra_gpu::wgpu::Instance>,
        problem: &render_quorra::QuorraRasterError,
    ) -> Option<Surface> {
        eprintln!("the graphics device could not be brought up: {problem}");
        eprintln!("  asked for: {}", self.backend_description());
        if let Some(instance) = instance {
            // What *this* instance could see, which is the number that distinguishes a backend
            // this machine does not have (none) from a driver that failed later (some).
            let visible = QuorraPresenter::adapters_on(instance);
            eprintln!(
                "  adapters behind it: {}",
                if visible.is_empty() {
                    "none — this machine has no adapter for that backend".to_owned()
                } else {
                    visible.join(", ")
                }
            );
        }
        // And what the machine has by every route, which is the list a person picks their next
        // `--backend` out of. Its own all-backends instance, so it costs a driver load — which is
        // acceptable here and nowhere else on this path: this run has already failed.
        let every = quorra_gpu::Device::adapter_names();
        eprintln!(
            "  adapters on this machine: {}",
            if every.is_empty() {
                "none".to_owned()
            } else {
                every.join(", ")
            }
        );
        if self.backend_asked_for {
            eprintln!(
                "Refused rather than started on another backend: --backend named one, and a flag \
                 that silently did something else would be worse than no flag. Try --backend with \
                 one of {}, or --cpu, which opens no graphics driver at all.",
                backend_names()
            );
            return None;
        }
        eprintln!(
            "Drawing on the processor instead, which opens no graphics driver. Pages will be slower."
        );
        Self::software(window)
    }

    /// Closes the one line the launch timeline never had: when the pipelines finished compiling.
    ///
    /// **ADR 0227.** Bring-up prints `pipelines still compiling` because
    /// `CLAUDE.md` forbids waiting for warmth on the launch path, and nothing ever said when the
    /// wait nobody did would have ended — so a first frame that absorbed a shader compilation
    /// and one that did not read identically. Polled once a frame, which behind the topic check
    /// is one `Option` read; *noticed* rather than *finished*, because the compilation ends on
    /// quorra's own thread and this is only the first frame to look.
    fn pipelines_compiled(&mut self) {
        if self.frames.pipelines || !self.trace.on(Topic::Launch) {
            return;
        }
        let Some(State {
            surface: Surface::Device(presenter),
            ..
        }) = self.state.as_ref()
        else {
            return;
        };
        let Some(compiling) = presenter.startup().pipeline_compilation else {
            return;
        };
        self.frames.pipelines = true;
        self.trace.say(
            Topic::Launch,
            format_args!(
                "pipelines compiled in {compiling:?}, noticed at this frame — nothing on the \
                 launch path waited for them, so every frame before this one drew with whatever \
                 was ready"
            ),
        );
    }

    /// Draws the frame the window asked for, and tells the core what became of it.
    ///
    /// **The frame's number is the frame's, since the three-hundred-and-ninetieth session.**
    /// This used to start a timer, present, close the launch timeline, publish the accessibility
    /// tree, and *then* read the timer — so `present -> presented in T` was the frame plus the
    /// bridge plus the timeline's own printing. ADR 0227 called that a measurement
    /// defect rather than a design choice, which is exactly what it was: on a page turn the tree
    /// is rebuilt and published, and that work was being attributed to the graphics device.
    fn redraw_requested(&mut self) {
        // Before the frame rather than after it: a tick can advance the page, and a page that
        // advanced after its frame was drawn would be one frame late for the whole slide show.
        self.drive_the_clock();
        let started = std::time::Instant::now();
        let mut stages = Stages::default();
        let outcome = self.present(&mut stages);
        stages.total = started.elapsed();
        if matches!(outcome, Some(Rendered::Presented | Rendered::Raster(_))) {
            self.launch.arrived(self.trace);
            // **After the timeline is closed, never before it.** Everything the accessibility
            // bridge does is off the launch path by construction, and this line is where that is
            // enforced rather than merely intended.
            let attending = std::time::Instant::now();
            self.attend();
            stages.attend = attending.elapsed();
        }
        let outcome_said = match &outcome {
            None => "nothing to show".to_owned(),
            Some(Rendered::Presented) => "presented".to_owned(),
            Some(Rendered::Failed(why)) => format!("failed: {why}"),
            Some(Rendered::Raster(_)) => "a raster".to_owned(),
        };
        let trace = self.trace;
        self.frames.frame(trace, &stages, &outcome_said);
        self.pipelines_compiled();
        let Some(rendered) = outcome else {
            return;
        };
        if !self.acknowledged
            && let Some(token) = self.request.as_ref().map(|request| request.token)
        {
            self.acknowledged = true;
            self.dispatch(Command::RenderReady { token, rendered });
        }
    }
}

impl ApplicationHandler for App {
    /// The percentiles, printed on the way out.
    ///
    /// **The shape of the question "why did it feel slow" is a distribution**, and the trace
    /// that raised ADR 0227 had 63 frame lines and no way to say that their median was
    /// 60 ms and their worst 514 without a spreadsheet. Here because it costs nothing per frame
    /// and everything it needs is already recorded.
    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        self.frames.summary(self.trace);
    }

    /// Keeps §12.4.4's clock, and only while there is one to keep.
    ///
    /// **Three speeds, and the idle one is the default.** A window reading a document waits for
    /// an event, which is what `main` sets and what a viewer is doing almost all of the time. A
    /// presentation between transitions wakes ten times a second, which is enough to notice a
    /// `/Dur` stated in seconds. A transition in flight polls, because it *is* an animation and
    /// every frame it can draw is one a person sees.
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        // One page of the search, once per turn round the loop. This is where the choice in
        // `viewer_core::search` is paid for on this host: the core reads one page per command and
        // has no thread to read a thousand on, so the loop that would otherwise be idle drives it
        // and the window keeps drawing while it does.
        if self.pages_left > 0 {
            self.dispatch(Command::Find(Find::Continue));
            event_loop.set_control_flow(ControlFlow::Poll);
            return;
        }
        let Some(presentation) = self.presentation.as_mut() else {
            event_loop.set_control_flow(ControlFlow::Wait);
            return;
        };
        if presentation.playing.is_some() {
            event_loop.set_control_flow(ControlFlow::Poll);
            self.redraw();
            return;
        }
        let now = std::time::Instant::now();
        let due = now >= presentation.wake;
        if due {
            presentation.wake = now.checked_add(PRESENTATION_TICK).unwrap_or(now);
        }
        let wake = presentation.wake;
        if due {
            self.redraw();
        }
        event_loop.set_control_flow(ControlFlow::WaitUntil(wake));
    }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_some() {
            return;
        }

        let attributes = Window::default_attributes()
            .with_title(self.title.clone())
            .with_inner_size(winit::dpi::LogicalSize::new(800.0, 1000.0));
        let window = Arc::new(
            event_loop
                .create_window(attributes)
                .expect("window creation"),
        );

        self.launch.mark("window");

        let size = window.inner_size();
        let Some(surface) = self.bring_up(&window) else {
            // Nothing can put pixels on this window: said above, in a sentence, and not survived.
            // An event loop that runs with nothing to present to shows a blank window for ever,
            // which is the failure this program spends its rounds removing.
            std::process::exit(1);
        };

        #[expect(
            clippy::cast_possible_truncation,
            reason = "a display's scale factor is a small ratio"
        )]
        let scale = window.scale_factor() as f32;
        self.state = Some(State {
            window,
            surface,
            size: (size.width.max(1), size.height.max(1)),
        });

        // **The document's thread is joined here and not a line earlier.** Everything above this
        // — the event loop, the window, the instance, the device — is what it was running beside,
        // and joining after the presenter exists is what makes the two costs the *longer* of the
        // pair rather than the sum. If the mark below reads a few hundred microseconds after
        // `graphics device`, the document was ready and waiting; if it reads milliseconds later,
        // this thread waited, which is a document large enough for the overlap to have been worth
        // more than it took.
        if let Some(opening) = self.opening.take() {
            let (viewer, events) = opening.join().expect("the thread opening the document");
            self.viewer = viewer;
            self.launch.mark("document joined");
            self.receive(events);
        }
        self.retitle();
        // The window's size is the first thing the core has been told about the viewport, and
        // it is what makes page one render. **Less the sidebar**, which Table 29's `/PageMode`
        // may already have opened: the document was opened before this window existed, so the
        // first `Resize` is the first chance to say how much of it the page has.
        self.dispatch(Command::Resize {
            width: size.width.saturating_sub(self.panel.inset(scale)).max(1),
            height: size.height.max(1),
            scale,
        });
    }

    #[expect(
        clippy::too_many_lines,
        reason = "every window event this host answers, in one match — which is where a reader \
                  looking for \"what does this program do with a click\" should find them all"
    )]
    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        if self.state.is_none() {
            return;
        }
        // Every window event but the pointer's, which arrives faster than a person can read and
        // which `pump` prints under `pointer` a moment later as the command it becomes — one
        // line for the movement rather than two. What this answers is the question a stuck
        // window raises first: *is the program being told anything at all?*
        if self.trace.on(Topic::Window) && !matches!(event, WindowEvent::CursorMoved { .. }) {
            self.trace.say(
                Topic::Window,
                format_args!("window event {}", describe_window_event(&event)),
            );
        }

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),

            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        state: ElementState::Pressed,
                        logical_key,
                        ..
                    },
                ..
            } => {
                if matches!(logical_key.as_ref(), Key::Named(NamedKey::Escape)) {
                    // **A field with the keyboard takes this key first**, which is what ADR 0201
                    // decided and what this branch quietly defeated: the press exited the program
                    // before `typed` was ever asked, so the one binding typing changes the meaning
                    // of was dead code from the round that wrote it. Found by reading the two
                    // against each other in the three-hundred-and-seventy-first session, because
                    // no gate in this tree presses a key twice in one window.
                    if self.typing.is_some() && self.typed(&logical_key.as_ref()) {
                        return;
                    }
                    event_loop.exit();
                    return;
                }
                // **A field being typed into takes every character key**, which is what makes `+`
                // a plus sign there and a magnification everywhere else — and what this branch
                // defeated for two of them in exactly the shape Escape's did: an `o` typed into a
                // field toggled the sidebar and a `?` opened the About card, because both were
                // answered before `typed` was ever asked. Escape's copy of this defect was found
                // in the three-hundred-and-seventy-first session and these two survived it to the
                // three-hundred-and-eighty-eighth, which is the reason `keys_reach_the_field`
                // now presses one of them at a field rather than trusting the order.
                if self.typing.is_some() && self.typed(&logical_key.as_ref()) {
                    return;
                }
                // **The find bar takes every key while it is open**, for the reason the field
                // above does and in the same place: a `/` typed into a search string is a
                // slash, and answering `o`, `p` or `c` first would be the defect ADR 0201 found
                // twice already. Opening it is a key this host answers itself — whether a bar is
                // on the screen is chrome, and `viewer-core` has no opinion about chrome (rule 5).
                if self.find.shown && self.find_key(&logical_key.as_ref()) {
                    return;
                }
                if matches!(logical_key.as_ref(), Key::Character("/")) {
                    let shown = self.find.toggle();
                    if !shown {
                        self.pages_left = 0;
                        self.dispatch(Command::Find(Find::Stop));
                    }
                    self.redraw();
                    return;
                }
                // The two keys this program answers itself rather than by sending a command:
                // whether a panel is shown is chrome, and `viewer-core` has no opinion about
                // chrome by construction (rule 5).
                if matches!(logical_key.as_ref(), Key::Character("o")) {
                    self.panel.toggle();
                    self.resize_page();
                    return;
                }
                if matches!(logical_key.as_ref(), Key::Character("?")) {
                    self.about.toggle();
                    self.redraw();
                    return;
                }
                // §12.4.4's presentation, and the third key this program answers itself: whether
                // a clock is running is a fact about this host and not about the document
                // (ADR 0135), so there is no command for it.
                if matches!(logical_key.as_ref(), Key::Character("p")) {
                    self.present_or_stop();
                    return;
                }
                // §14.8.2.5, and the fifth key this program answers itself: what a copy *is* —
                // a clipboard — belongs to the platform, so `viewer-core` has no command for it
                // and only the two orders to answer with. Ctrl and no Ctrl reach the same arm,
                // because a field being typed into took this key above (`clipped`) and what is
                // left is a page, where the modifier changes nothing.
                if matches!(logical_key.as_ref(), Key::Character("c")) {
                    self.copy_selection();
                    return;
                }
                // §12.5.6.6, and the sixth: whether the next drag draws a text box is a mode this
                // host is in, and `viewer-core` has no opinion about chrome by construction
                // (rule 5). The command goes out on the *release*, with both corners.
                if matches!(logical_key.as_ref(), Key::Character("f")) {
                    self.drawing = if self.drawing.is_some() {
                        println!("note: not drawing a free text annotation after all");
                        None
                    } else {
                        println!(
                            "note: drag out a rectangle for a free text annotation (§12.5.6.6)"
                        );
                        Some(Drawing::Armed)
                    };
                    return;
                }
                // Everything else goes to the page, and the About card is over it: a key press
                // that turned a page nobody can see would be answering the wrong question.
                if self.about.shown {
                    return;
                }
                let Some(command) = key_command(&logical_key.as_ref(), self.shift) else {
                    return;
                };
                // §12.5.6.10's markups are defined over selected text, so a press with nothing
                // selected asks for an annotation over nothing. `viewer-core` answers by doing
                // nothing, which is right and silent — and a person who pressed a key and saw no
                // change has been told nothing at all (trap 5). The host has the selection
                // already, because it draws it.
                if matches!(command, Command::Edit(Edit::Markup { .. })) && !self.has_selection() {
                    println!("note: select some text first — §12.5.6.10's markups mark up text");
                    return;
                }
                let walked = matches!(command, Command::Focused(_));
                self.dispatch(command);
                if walked {
                    self.aim_at_focus();
                }
            }

            WindowEvent::CursorMoved { position, .. } => {
                self.cursor = (position.x, position.y);
                self.pointer_moved();
            }

            WindowEvent::MouseInput {
                state: element,
                button: MouseButton::Left,
                ..
            } => {
                if self.about.shown {
                    return;
                }
                if self.over_panel() {
                    // Answered once, on the press: a panel that acted on both ends of a click
                    // would follow a destination twice.
                    if element == ElementState::Pressed {
                        self.click_panel();
                    }
                    return;
                }
                // §12.5.1's activation, for the one subtype that takes a keyboard: a press
                // inside a text field's rectangle is how a person says "type here". The core
                // already raises §12.6.3's focus events from the same press; what this adds is
                // the host's own state, because *where the keys go* is chrome and `viewer-core`
                // has no opinion about chrome by construction (rule 5).
                // §12.5.6.6's geometry is a *drag*, which is the whole of what this mode adds:
                // the press puts one corner down and the release sends both. It runs before the
                // two below because while it is armed the press means nothing else — a person who
                // has said "draw a box here" has not said "type in the field underneath it".
                if let Some(drawing) = self.drawing {
                    self.draw_free_text(drawing, element);
                    self.dragging = element == ElementState::Pressed;
                    return;
                }
                if element == ElementState::Pressed {
                    self.aim_at_field();
                    // And §12.7.5.2's other kind of press, which takes no keyboard: a click on a
                    // check box or a radio button is what *gives* it a value. Nothing happens
                    // where the press was not on one.
                    self.toggle_button();
                }
                self.dragging = element == ElementState::Pressed;
                self.dispatch(Command::Pointer {
                    at: self.on_page(self.cursor),
                    action: match element {
                        ElementState::Pressed => PointerAction::Pressed,
                        ElementState::Released => PointerAction::Released,
                    },
                });
            }

            WindowEvent::Resized(size) => {
                let scale = self.state.as_ref().map_or(1.0, |state| {
                    #[expect(
                        clippy::cast_possible_truncation,
                        reason = "a display's scale factor is a small ratio"
                    )]
                    let scale = state.window.scale_factor() as f32;
                    scale
                });
                if let Some(state) = self.state.as_mut() {
                    // The presenter reconfigures its surface from the viewport on
                    // the next frame; the host only has to remember the size.
                    state.size = (size.width.max(1), size.height.max(1));
                }
                self.dispatch(Command::Resize {
                    width: size.width.saturating_sub(self.panel.inset(scale)).max(1),
                    height: size.height.max(1),
                    scale,
                });
                // A resize moves the *inner* rectangle inside the outer one, so AT-SPI's screen
                // coordinates change with it. This and `Moved` are the two events that can move
                // them; a page turn is not one, which is what took it off that path (ADR 0228).
                self.place_window();
            }

            // See `Resized`: the window's place on the screen is asked for where it can change.
            WindowEvent::Moved(_) => self.place_window(),

            WindowEvent::MouseWheel { delta, .. } => self.wheel(delta),

            // Remembered rather than read at the wheel, because winit puts no modifier state in
            // the wheel's own event.
            WindowEvent::ModifiersChanged(modifiers) => {
                self.control = modifiers.state().control_key();
                self.shift = modifiers.state().shift_key();
            }

            WindowEvent::RedrawRequested => self.redraw_requested(),

            _ => {}
        }
    }
}

/// What a key press asks for, where it asks for anything.
///
/// One place rather than an arm apiece inside the event handler, because this is the whole of
/// this program's key bindings and a reader looking for them should find them together.
fn key_command(key: &Key<&str>, shift: bool) -> Option<Command> {
    Some(match *key {
        // §12.5.1 names this key: "[i]nteractive PDF processors may permit the user to navigate
        // through the annotations on a page by using the keyboard (in particular, the tab key)".
        // The *order* is the document's, in `pdf_model::tab_order`; shift is the only thing that
        // separates the two directions, because winit reports one key for both.
        Key::Named(NamedKey::Tab) => Command::Focused(if shift {
            FocusMove::Previous
        } else {
            FocusMove::Next
        }),
        Key::Named(NamedKey::ArrowRight | NamedKey::PageDown | NamedKey::Space) => {
            Command::GoTo(PageTarget::Next)
        }
        Key::Named(NamedKey::ArrowLeft | NamedKey::PageUp) => Command::GoTo(PageTarget::Previous),
        Key::Named(NamedKey::Home) => Command::GoTo(PageTarget::First),
        Key::Named(NamedKey::End) => Command::GoTo(PageTarget::Last),
        // No anchor: a keyboard names no point, so the core holds the viewport's centre.
        Key::Character("+" | "=") => Command::Zoom {
            zoom: Zoom::In,
            at: None,
        },
        Key::Character("-") => Command::Zoom {
            zoom: Zoom::Out,
            at: None,
        },
        Key::Character("0") => Command::Zoom {
            zoom: Zoom::FitPage,
            at: None,
        },
        Key::Character("a") => Command::Select(Selection::All),
        Key::Character("s") => Command::Save,
        // §12.5.6.10 over what is selected. Four subtypes and one key apiece would be four
        // bindings a person has to learn; this host offers the one a person means by "mark
        // this" and leaves the other three to a host with a menu. The colour is this host's
        // choice — the standard states none, Table 166's `/C` simply carries what a processor
        // was told — and a soft yellow is what a highlighter is.
        Key::Character("h") => Command::Edit(Edit::Markup {
            kind: pdf_model::view::Markup::Highlight,
            colour: [1.0, 0.9, 0.2],
        }),
        // The same mark struck through rather than washed over, because a person marking up a
        // draft means both and the two are one construction in `pdf-model`.
        Key::Character("k") => Command::Edit(Edit::Markup {
            kind: pdf_model::view::Markup::StrikeOut,
            colour: [0.85, 0.15, 0.15],
        }),
        // A page taller than the window: the scroll is in device pixels, so this is about a
        // fifteenth of a fitted A4 page and the same on any display.
        Key::Named(NamedKey::ArrowDown) => Command::Scroll { dx: 0.0, dy: 60.0 },
        Key::Named(NamedKey::ArrowUp) => Command::Scroll { dx: 0.0, dy: -60.0 },
        _ => return None,
    })
}

/// Receives what `wgpu`, `vello` and `naga` say about themselves.
///
/// Those three write to the `log` facade, and a facade with nothing behind it drops every record
/// — which is why a page that would not draw produced no output at all (ADR 0126). Twenty lines
/// rather than a logging framework: there is one destination, one format, and one filter, and a
/// configuration language for those would be longer than this.
struct Speak {
    /// The most detailed level to print, from `PDFVIEWER_LOG`.
    level: log::LevelFilter,
}

impl log::Log for Speak {
    fn enabled(&self, metadata: &log::Metadata<'_>) -> bool {
        metadata.level() <= self.level
    }

    fn log(&self, record: &log::Record<'_>) {
        if self.enabled(record.metadata()) {
            eprintln!("{}: {}: {}", record.level(), record.target(), record.args());
        }
    }

    fn flush(&self) {}
}

/// Installs [`Speak`], at `PDFVIEWER_LOG`'s level or `warn`.
///
/// `warn` by default because that is the level at which a graphics driver says something is
/// wrong; `PDFVIEWER_LOG=debug` is what to set when *nothing* is wrong and the question is what
/// the device is doing.
fn speak_up() {
    let level = match std::env::var("PDFVIEWER_LOG").unwrap_or_default().as_str() {
        "error" => log::LevelFilter::Error,
        "info" => log::LevelFilter::Info,
        "debug" => log::LevelFilter::Debug,
        "trace" => log::LevelFilter::Trace,
        _ => log::LevelFilter::Warn,
    };
    // A failure here means a logger is already installed, which nothing else in this program
    // does; there is no second thing to try and nothing is lost by carrying on quietly.
    if log::set_boxed_logger(Box::new(Speak { level })).is_ok() {
        log::set_max_level(level);
    }
}

/// One line naming a window event, for `--trace`.
fn describe_window_event(event: &WindowEvent) -> String {
    match event {
        WindowEvent::RedrawRequested => "redraw requested".to_owned(),
        WindowEvent::Resized(size) => format!("resized to {}x{}", size.width, size.height),
        WindowEvent::KeyboardInput { event, .. } => {
            format!("key {:?} {:?}", event.logical_key, event.state)
        }
        WindowEvent::MouseInput { state, button, .. } => format!("mouse {button:?} {state:?}"),
        WindowEvent::CloseRequested => "close requested".to_owned(),
        other => format!("{other:?}"),
    }
}

/// One line naming a command, for `--trace`.
///
/// The command's own `Debug` would print a document's bytes and a raster's pixels, which is not a
/// line. This is what a person following a page turn needs to see.
fn describe_command(command: &Command) -> String {
    match command {
        Command::Open { id, bytes, .. } => format!("open {:?}, {} bytes", id, bytes.len()),
        Command::Close(id) => format!("close {id:?}"),
        Command::Restrict(level) => format!("restrictions {level:?}"),
        Command::Delegate(appearances) => format!("widget appearances {appearances:?}"),
        Command::Tick { millis } => format!("tick {millis} ms"),
        Command::Focus(id) => format!("focus {id:?}"),
        Command::Resize {
            width,
            height,
            scale,
        } => format!("resize {width}x{height} at {scale}"),
        Command::GoTo(target) => format!("go to {target:?}"),
        Command::Zoom { zoom, at } => format!("zoom {zoom:?} at {at:?}"),
        Command::Scroll { dx, dy } => format!("scroll {dx} {dy}"),
        Command::SetGroup { group, on } => format!("layer {group:?} {on}"),
        Command::Activate(object) => format!("activate {object:?}"),
        Command::Extract { name } => format!("extract {name:?}"),
        Command::Pointer { at, action } => format!("pointer {action:?} at {at:?}"),
        Command::Select(what) => format!("select {what:?}"),
        Command::Focused(move_to) => format!("focus {move_to:?} annotation"),
        Command::Edit(edit) => format!("edit {edit:?}"),
        Command::Undo => "undo".to_owned(),
        Command::Redo => "redo".to_owned(),
        Command::Save => "save".to_owned(),
        Command::Supply { purpose, bytes } => format!(
            "supply {purpose:?}, {}",
            bytes
                .as_ref()
                .map_or_else(|| "declined".to_owned(), |b| format!("{} bytes", b.len()))
        ),
        Command::Find(find) => format!("find {find:?}"),
        Command::RenderReady { token, .. } => format!("render ready {token:?}"),
    }
}

/// One line naming an event, for `--trace`.
fn describe_event(event: &Event) -> String {
    match event {
        Event::Opened { pages, .. } => format!("opened, {pages} page(s)"),
        Event::OpenFailed { reason, .. } => format!("open failed: {reason}"),
        Event::PasswordRequired { .. } => "a password is required".to_owned(),
        Event::Closed(_) => "closed".to_owned(),
        Event::PageChanged { index, of, .. } => format!("page {} of {of}", index.saturating_add(1)),
        Event::NeedsRender(request) => format!(
            "needs render: page {}, {}x{}, {} command(s), {:?}",
            request.page.saturating_add(1),
            request.target.width,
            request.target.height,
            request.list.command_count(),
            request.token
        ),
        Event::Damage(_) => "damage".to_owned(),
        Event::OpenUri { uri, .. } => format!("open uri {uri}"),
        Event::NeedsFile { name, .. } => format!("needs file {name}"),
        Event::Transition { .. } => "a transition".to_owned(),
        Event::Dirty { dirty, .. } => format!("dirty {dirty}"),
        Event::Saved { bytes, .. } => format!("saved, {} bytes", bytes.len()),
        Event::Extracted { name, bytes, .. } => {
            format!("extracted {name:?}, {} bytes", bytes.len())
        }
        Event::Reported { page, notes, .. } => {
            format!("reported about page {page:?}: {}", notes.join("; "))
        }
        Event::Refused {
            operation, notes, ..
        } => format!("refused {}: {}", operation.as_str(), notes.join("; ")),
        Event::Searched {
            found, remaining, ..
        } => match found {
            Some(found) => format!("searched: page {}, {:?}", found.page, found.range),
            None => format!("searched: nothing yet, {remaining} page(s) left"),
        },
    }
}

/// A window position as the device pixels `viewer-core` speaks in.
#[expect(
    clippy::cast_possible_truncation,
    reason = "a window coordinate is a small number of pixels"
)]
fn at(cursor: (f64, f64)) -> (f32, f32) {
    (cursor.0 as f32, cursor.1 as f32)
}

/// Lays the selection's shapes over the page.
///
/// The selection quads as a display list in the window's own pixels.
///
/// The quadrilaterals arrive from `viewer-core` in device pixels of this window, so nothing here
/// composes a transform: that is the whole point of chrome crossing as geometry rather than as
/// pixels. Drawn with `Multiply`, which darkens what is under it and leaves the glyphs readable —
/// §11.3.5.2 makes it the one mode whose "result colour is always at least as dark as either of
/// the two constituent colours", so the text under the wash survives it. A native host asks its
/// platform for the colour; this one has nobody to ask, and a hard-coded blue that says so is
/// better than one that pretends.
///
/// **One fill, one subpath per quad**, and the count matters rather than the shape: a compositor
/// gives every non-`Over` blend its own layer and prices its internal textures before allocating
/// them, so a fill per quad made a selection cost `(quads + 1) × 2 × width × height × 4` bytes of
/// frame budget — 6.4 MB a quad at 800 × 1000, spending a 256 MiB budget at 63 quads, which is one
/// short paragraph. Under one layer the cost stops depending on what is selected at all. The
/// per-quad blend it replaces was preserving something nobody wants: `Query::Selection` answers
/// one quad per *run*, runs tile rather than overlap, and the two overlapping pairs out of 171
/// measured on three lines of `tracemonkey.pdf` overlap by 0.28 and 0.17 of a device pixel. Under
/// the non-zero rule one path is one shape, so those slivers stop darkening twice as well.
fn highlight_list(quads: &[[f32; 8]], width: u32, height: u32) -> Option<pdf_render::DisplayList> {
    if quads.is_empty() {
        return None;
    }
    #[expect(
        clippy::cast_precision_loss,
        reason = "window dimensions are far below f32's exact integer range"
    )]
    let mut list = pdf_render::DisplayList::new(Size::new(width as f32, height as f32));
    let colour = Color::rgb(140.0 / 255.0, 180.0 / 255.0, 1.0);
    let mut path = Path::new();
    for quad in quads {
        for (index, corner) in quad.chunks_exact(2).enumerate() {
            let point = Point::new(corner[0], corner[1]);
            path.push(if index == 0 {
                PathCommand::MoveTo(point)
            } else {
                PathCommand::LineTo(point)
            });
        }
        path.push(PathCommand::Close);
    }
    list.push(DrawCommand::Fill {
        path: Arc::new(path),
        transform: Transform::IDENTITY,
        fill_rule: FillRule::NonZero,
        paint: Paint::Solid(colour),
        clip: None,
        mask: None,
        blend: BlendMode::Multiply,
    });
    Some(list)
}

/// The shapes over every occurrence of a search string, in the window's own pixels.
///
/// A paler yellow than [`highlight_list`]'s blue selection and multiplied over the page for the
/// same reason: the glyphs underneath have to stay readable. **The colour is a choice** — the
/// standard describes no find bar and says nothing about what a match looks like — and it is
/// chosen to be a different *hue* from the selection rather than a different weight of it, so
/// that "where else the word is" and "which one you are on" cannot be confused at a glance. The
/// two native hosts made the other choice, one hue at two alphas, because a platform hands them a
/// selection colour and no second one; this host has no theme to ask and so may pick both.
fn match_list(quads: &[[f32; 8]], width: u32, height: u32) -> Option<pdf_render::DisplayList> {
    if quads.is_empty() {
        return None;
    }
    #[expect(
        clippy::cast_precision_loss,
        reason = "window dimensions are far below f32's exact integer range"
    )]
    let mut list = pdf_render::DisplayList::new(Size::new(width as f32, height as f32));
    let mut path = Path::new();
    for quad in quads {
        for (index, corner) in quad.chunks_exact(2).enumerate() {
            let point = Point::new(corner[0], corner[1]);
            path.push(if index == 0 {
                PathCommand::MoveTo(point)
            } else {
                PathCommand::LineTo(point)
            });
        }
        path.push(PathCommand::Close);
    }
    list.push(DrawCommand::Fill {
        path: Arc::new(path),
        transform: Transform::IDENTITY,
        fill_rule: FillRule::NonZero,
        paint: Paint::Solid(Color::rgb(1.0, 0.87, 0.35)),
        clip: None,
        mask: None,
        blend: BlendMode::Multiply,
    });
    Some(list)
}

/// How many steps of a search go by between repaints of the find bar's progress count.
///
/// A choice with a measurement behind it — see [`App::searched`], which is where the two numbers
/// are — and it belongs to this host alone: a native host repaints when its toolkit says to.
const SEARCH_REDRAW: usize = 16;

/// The colour §12.5.1's focus ring is drawn in.
///
/// A choice, and the only one available: the clause says nothing about showing a focus and this
/// host has no theme to ask. A native host uses its platform's ring and never sees this constant.
const FOCUS_RING: Color = Color {
    r: 0.10,
    g: 0.42,
    b: 0.85,
    a: 1.0,
};

/// How wide that ring is, in device pixels.
const FOCUS_RING_WIDTH: f32 = 2.0;

/// The colour the caret is drawn in.
///
/// A choice, and for the same reason the focus ring's is: no clause states a text cursor at all.
/// Black rather than the ring's blue, because a caret stands *in* the text and a person reads it
/// as part of the line — and this host has no theme to ask for the platform's insertion-point
/// colour, which a native one would use instead.
const CARET: Color = Color {
    r: 0.0,
    g: 0.0,
    b: 0.0,
    a: 1.0,
};

/// How wide the caret is, in device pixels.
const CARET_WIDTH: f32 = 2.0;

/// The colour §12.5.6.6's text is written in when this host creates one, as `DeviceRGB`.
///
/// **A choice**, and the same one the highlight's yellow is: Table 177's `/DA` carries whatever a
/// processor was told and no clause states what to tell it. A dark red is what a note written on
/// somebody else's page is, and it is what this host offers a person with one key.
const FREE_TEXT_INK: [f32; 3] = [0.7, 0.1, 0.1];

/// The chrome drawn over a page, as display lists in the window's own pixels.
///
/// Gathered once per frame and handed to the presenter beside the page, which is why they are
/// display lists and not backend calls: they draw through the same translation as the page
/// itself, and that is what a `--cpu` run and a page the graphics device refuses both need.
///
/// Held in one value so that the borrowed lists [`Self::lists`] hands the presenter outlive the
/// call: each of these is built for this frame and dropped after it.
#[derive(Default)]
struct Overlays {
    /// Every occurrence of the find bar's string on this page, under the selection.
    matches: Option<pdf_render::DisplayList>,
    /// What is selected on the page, which belongs to the page and so is under everything else.
    selection: Option<pdf_render::DisplayList>,
    /// What is selected inside a form field (ADR 0225).
    field_selection: Option<pdf_render::DisplayList>,
    /// §12.5.1's focus ring, round whatever the tab key last landed on.
    focus: Option<pdf_render::DisplayList>,
    /// §12.7.4.3's caret, where the next character goes (ADR 0211).
    caret: Option<pdf_render::DisplayList>,
    /// §12.5.6.14's popup windows, which belong to the document and so are under the sidebar.
    popups: Option<pdf_render::DisplayList>,
    /// The sidebar, where it is shown.
    panel: Option<pdf_render::DisplayList>,
    /// The find bar, over the sidebar and under the modal card.
    find: Option<pdf_render::DisplayList>,
    /// `/NOTICE`, where it is shown. **Last, so it is on top**: it is a modal card and the
    /// sidebar is behind it.
    about: Option<pdf_render::DisplayList>,
}

impl Overlays {
    /// Builds every one of them for this frame.
    fn of(app: &App, edge: f32, width: u32, height: u32) -> Self {
        Self {
            matches: app.matches_list(edge, width, height),
            selection: app.selection_list(edge, width, height),
            field_selection: app.field_selection_list(edge, width, height),
            focus: app.focus_list(edge, width, height),
            caret: app.caret_list(edge, width, height),
            popups: app.popup_list(edge, width, height),
            panel: app.panel_list(height),
            find: app.find_list(width),
            about: app.about_list(width, height),
        }
    }

    /// The ones there are, in the order they are drawn: selection first (it belongs to the page),
    /// then the sidebar, then the modal card on top — the order the Vello host drew them in.
    fn lists(&self) -> Vec<&pdf_render::DisplayList> {
        [
            self.matches.as_ref(),
            self.selection.as_ref(),
            self.field_selection.as_ref(),
            self.focus.as_ref(),
            self.caret.as_ref(),
            self.popups.as_ref(),
            self.panel.as_ref(),
            self.find.as_ref(),
            self.about.as_ref(),
        ]
        .into_iter()
        .flatten()
        .collect()
    }
}

/// Reads a password from the terminal, or `None` if the person cancelled with an empty line.
fn ask_password(name: &str) -> Option<String> {
    eprint!("{name} needs a password (empty line to give up): ");
    std::io::stderr().flush().ok()?;
    let mut line = String::new();
    std::io::stdin().read_line(&mut line).ok()?;
    let password = line.trim_end_matches(['\r', '\n']).to_owned();
    (!password.is_empty()).then_some(password)
}

#[cfg(test)]
mod tests {
    use super::{
        Backend, DEFAULT_BACKEND, GPU_COVERAGE_MAGNIFICATION, QuorraPresenter, after,
        backend_names, before, caret_boundary, copied, coverage_for, spawn_instancing, spliced,
    };
    use pdf_render::Transform;

    /// The lane follows the magnification, and the page transform a frame is drawn
    /// with is what states it: scale, y flip, translation.
    #[test]
    fn the_lane_follows_the_magnification() {
        let page = |magnification: f32| {
            Transform::scale(magnification, -magnification)
                .then(Transform::translate(0.0, 842.0 * magnification))
        };
        assert_eq!(
            coverage_for(page(8.0)),
            quorra_gpu::Coverage::Cpu,
            "below the atlas cliff the cached lane is cheaper"
        );
        assert_eq!(
            coverage_for(page(12.0)),
            quorra_gpu::Coverage::Gpu,
            "above it the CPU lane rasterises every glyph on every frame"
        );
        assert_eq!(
            coverage_for(page(GPU_COVERAGE_MAGNIFICATION)),
            quorra_gpu::Coverage::Gpu,
            "the threshold itself belongs to the lane it names"
        );
    }

    /// §7.7.3.3's page rotation puts the magnification in `b` and `c` rather than `a`
    /// and `d`, so a rotated page must land on the same lane as an upright one at the
    /// same zoom. This is the case a `transform.a` test would get wrong — and get wrong
    /// silently, by choosing the slow lane on a quarter of the corpus.
    #[test]
    fn a_rotated_page_reads_the_same_magnification() {
        let upright = Transform::scale(12.0, -12.0);
        // A quarter turn: the scale moves off the diagonal entirely.
        let turned = Transform {
            a: 0.0,
            b: 12.0,
            c: -12.0,
            d: 0.0,
            e: 0.0,
            f: 0.0,
        };
        assert_eq!(coverage_for(upright), coverage_for(turned));
        assert_eq!(coverage_for(turned), quorra_gpu::Coverage::Gpu);
    }

    /// §14.8.2.5's two orders, and which one a copy takes.
    ///
    /// The clause gives a page a page content order and a logical content order and says they
    /// "should" agree; `viewer_core::Query::LogicalSelection` answers with the second where the
    /// structure tree reaches every byte of the selection and with nothing otherwise. What this
    /// host owns is the choice between them and the sentence it prints, because a person who
    /// copied a two-column page and got the columns interleaved has been told nothing at all.
    #[test]
    fn a_copy_takes_the_logical_order_where_the_core_could_give_one() {
        assert_eq!(
            copied(None, ""),
            None,
            "nothing selected leaves the clipboard alone"
        );
        assert_eq!(
            copied(Some("a tree that reaches it".to_owned()), ""),
            None,
            "and does so even if the tree would have answered"
        );
        assert_eq!(
            copied(Some("logical".to_owned()), "page"),
            Some(("logical".to_owned(), "§14.8.2.5's logical content order")),
            "a rearrangement the core could make is the one a copy wants"
        );
        assert_eq!(
            copied(None, "page"),
            Some(("page".to_owned(), "page content order")),
            "and the order it did not get is not claimed"
        );
    }

    /// What a keystroke does to a value, on the one input that breaks a naive index.
    ///
    /// A caret is a byte offset into a value the *core* owns: §12.7.5.3's `DoNotScroll` shortens
    /// it under the host, a page turn can take the field away, and a character is one to four
    /// bytes. So every use of the offset goes through `caret_boundary`, and the splice is written
    /// with `get` rather than with indexing — a panic here would be a program that quits because
    /// somebody typed an accent.
    #[test]
    fn a_caret_never_falls_inside_a_character() {
        let value = "café";
        assert_eq!(value.len(), 5, "é is two bytes");
        // The offset between the two bytes of `é` is not a place a caret can be.
        assert_eq!(caret_boundary(value, 4), 3);
        // And one past the end of a value the field truncated is its end.
        assert_eq!(caret_boundary(value, 99), value.len());
        assert_eq!(
            before(value, 5),
            3,
            "one character back from the end is before é"
        );
        assert_eq!(after(value, 3), 5, "and one forward from there is past it");
        assert_eq!(after(value, 5), 5, "the end of the value stays put");
        assert_eq!(before(value, 0), 0, "so does the start");
        // Backspace at the end, and an insertion in the middle.
        assert_eq!(spliced(value, before(value, 5), 5, ""), "caf");
        assert_eq!(spliced(value, 1, 1, "X"), "cXafé");
    }

    /// **The whole of what `--cpu` promises.** A run on the processor creates no
    /// `wgpu::Instance`, which is what loads the driver: before the
    /// three-hundred-and-eighty-fourth session the flag chose which rasteriser drew the page
    /// and the driver was loaded regardless, so a driver that faulted while loading took the
    /// run down whether or not the flag was given. That is the defect the project owner hit on
    /// Windows, and this test is what stops it coming back.
    ///
    /// A backend named alongside `--cpu` changes nothing: there is nothing to name a backend
    /// *for*.
    #[test]
    fn cpu_creates_no_graphics_instance() {
        assert!(spawn_instancing(true, None).is_none());
        assert!(spawn_instancing(true, Some(Backend::Vulkan)).is_none());
        assert!(spawn_instancing(true, Some(Backend::Dx12)).is_none());
    }

    /// And the control, without which the test above passes on a function that never spawns:
    /// a run that did *not* ask for the processor makes one, and it is a real instance.
    #[test]
    fn without_cpu_an_instance_is_created() {
        let thread = spawn_instancing(false, None).expect("a run without --cpu creates one");
        let instance = thread.join().expect("the thread creating the instance");
        // Enumerating proves the instance is usable rather than merely constructed. The list
        // may be empty on a machine with no adapter at all, which is not what is being asked.
        let _ = QuorraPresenter::adapters_on(&instance);
    }

    /// Every value `--backend` accepts is parsed by the name it prints, and nothing else is.
    #[test]
    fn a_backend_is_named_by_the_word_it_prints() {
        for backend in Backend::ALL {
            assert_eq!(Backend::parse(backend.name()), Some(backend));
        }
        assert_eq!(Backend::parse("Vulkan"), None, "the match is exact");
        assert_eq!(Backend::parse("webgpu"), None, "not a value on this build");
        assert_eq!(Backend::parse(""), None);
        assert_eq!(backend_names(), "vulkan, dx12, metal, gl");
    }

    /// The platform default, stated as a test because it is a decision rather than a detail:
    /// on Windows this project asks for DX12 first, and everywhere else it asks for nothing and
    /// leaves the choice where it was. ADR 0221 argues it; no machine in this project can run
    /// the Windows arm, which is why the arm is written down here as well as there.
    #[test]
    fn the_default_backend_is_dx12_on_windows_and_nothing_elsewhere() {
        #[cfg(windows)]
        assert_eq!(DEFAULT_BACKEND, Some(Backend::Dx12));
        #[cfg(not(windows))]
        assert_eq!(DEFAULT_BACKEND, None);
    }
}
