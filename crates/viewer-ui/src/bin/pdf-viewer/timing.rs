//! What the launch cost and what each frame cost, which `CLAUDE.md` makes two first-class numbers.
//!
//! Together in one module because they are the same discipline over two different spans: a
//! milestone is only legible beside the ones on either side of it, and a frame's stages are only
//! legible beside the distribution of every other frame. Both print through [`Trace`] and neither
//! decides anything — they are instruments, and the code that does the work is elsewhere.

use crate::trace::{Topic, Trace};

/// When each milestone on the launch path was reached, from this process's own first instruction.
///
/// **`CLAUDE.md` makes this a first-class number.** Page one goes to the graphics device by the
/// project owner's decision, so creating that device and compiling its pipelines is *part of*
/// time-to-first-page — "a number to measure and to keep small". The per-step durations `--trace`
/// already printed cannot say that: each says what one step cost, and a launch is the sum of the
/// steps plus everything nobody thought to time. One `Instant`, taken before the arguments are
/// parsed, and one mark per step, so the difference between two marks is a step nobody named.
pub(crate) struct Launch {
    /// Taken as the first statement of `main`, so nothing before it is invisible.
    pub(crate) began: std::time::Instant,
    /// Each milestone and how long after `began` it was reached.
    ///
    /// Owned strings rather than `&'static str` because two milestones carry a number of
    /// their own — the interpreted command count is what says whether a slow launch was a
    /// large page, and a step name that cannot say so is the hole `doc/todo/44` closed.
    marks: Vec<(String, std::time::Duration)>,
    /// Whether the timeline has been printed, which happens once, at the first present.
    reported: bool,
}

impl Launch {
    /// Starts the clock. The first statement of `main`.
    pub(crate) fn new() -> Self {
        Self {
            began: std::time::Instant::now(),
            marks: Vec::new(),
            reported: false,
        }
    }

    /// Records that `step` has just finished.
    pub(crate) fn mark(&mut self, step: &'static str) {
        self.marks.push((step.to_owned(), self.began.elapsed()));
    }

    /// Records that page one's display list exists — interpretation is over, `commands`
    /// commands.
    ///
    /// **This is the milestone the table jumped over** (ADR 0332): between `document joined`
    /// and `first present` sits the whole interpretation of page one, which on the document
    /// that raised `doc/todo/44` was seven unnamed seconds for 58 009 commands. The caller is
    /// the render-request handler, which cannot know it is on the launch path — every later
    /// request is the steady state and the frame log's business — so only the first request
    /// is kept, and nothing is kept once the timeline has closed.
    pub(crate) fn interpreted(&mut self, commands: usize) {
        if self.reported
            || self
                .marks
                .iter()
                .any(|(step, _)| step.starts_with("interpreted"))
        {
            return;
        }
        self.marks
            .push((format!("interpreted, {commands} cmd"), self.began.elapsed()));
    }

    /// Records that the first frame's display lists have been translated into a GPU scene.
    ///
    /// The boundary is relayed rather than fabricated: scene building and device submission
    /// happen inside one `QuorraPresenter::present` call, so this host cannot take a clock
    /// reading between them — but `FrameCost::scene` is quorra's own measurement of the
    /// translation from the moment that call began, so the mark is `handed` (when this host
    /// handed the frame over) plus that duration. First frame only, as above.
    pub(crate) fn scene_built(&mut self, handed: std::time::Instant, scene: std::time::Duration) {
        if self.reported
            || self
                .marks
                .iter()
                .any(|(step, _)| step == "first scene built")
        {
            return;
        }
        let at = handed
            .saturating_duration_since(self.began)
            .saturating_add(scene);
        self.marks.push(("first scene built".to_owned(), at));
    }

    /// The first frame has reached the window: closes the timeline and, under `--trace`, prints it.
    ///
    /// Two columns because two questions are asked of a launch: *when did this step finish*,
    /// which is what a person waiting sees, and *what did it cost*, which is what a regression
    /// shows up in. Neither is derivable from the other once steps are added or reordered.
    pub(crate) fn arrived(&mut self, trace: Trace) {
        if self.reported {
            return;
        }
        self.reported = true;
        self.marks
            .push(("first present".to_owned(), self.began.elapsed()));
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
pub(crate) struct Stages {
    /// The page this frame drew, counting from one.
    pub(crate) page: usize,
    /// How many display-list commands the page carried.
    pub(crate) commands: usize,
    /// This host's own work before anything is handed over: the page's geometry, the selection,
    /// the focus ring, the caret, the popup windows and the panel, each a query into the core.
    pub(crate) host: std::time::Duration,
    /// What `render-quorra` reported for the same frame, which is where the device's own
    /// accounting arrives from.
    pub(crate) gpu: render_quorra::FrameCost,
    /// `render-cpu` rasterising the page and presenting it, on a frame the device refused.
    pub(crate) fallback: std::time::Duration,
    /// The accessibility publication — measured *beside* the frame rather than inside it.
    pub(crate) attend: std::time::Duration,
    /// The whole of `redraw_requested`, which is what the old single number was.
    pub(crate) total: std::time::Duration,
}

/// Every frame this run has drawn, for the summary at exit.
#[derive(Debug, Default)]
pub(crate) struct FrameLog {
    /// The samples, capped at [`FRAME_SAMPLES`].
    samples: Vec<Stages>,
    /// How many frames there were, which is not `samples.len()` past the cap.
    count: usize,
    /// Whether the line explaining the frame line's columns has been printed.
    legend: bool,
    /// Whether the closing line for pipeline compilation has been printed.
    pub(crate) pipelines: bool,
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
    pub(crate) fn frame(&mut self, trace: Trace, stages: &Stages, outcome: &str) {
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
        // The third strange frame, and the same rule: a repack throws every atlas placement away
        // and so kills the retained encode the next frame would have replayed (quorra's ADR
        // 0050). A page settles after at most one, so the word appearing on frame after frame is
        // the pathology rather than the event.
        if stages.gpu.atlas_repacked {
            unusual = format!("{unusual} repacked");
        }
        // Whether this frame's device commands were encoded or replayed, one character wide,
        // beside the number it explains. A frame loop that means to reuse its scene and does
        // not is the one defect ADR 0351 can have, and reading it off a small `encode` is
        // exactly the inference quorra's `EncodeSource` exists so that nobody has to make.
        let source = match stages.gpu.encode_source {
            Some(quorra_gpu::EncodeSource::Replayed) => " replayed",
            Some(quorra_gpu::EncodeSource::Encoded) => " encoded",
            None => "",
        };
        trace.say(
            Topic::Frames,
            format_args!(
                "frame p{} {}cmd {outcome} {:.1} | host {:.1} scene {:.1} device {:.1}{source} \
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
                "quorra's render: encoding, transfers, and the passes it already waits on — \
                 followed by whether it encoded this frame's scene or replayed one (ADR 0351)",
            ),
            (
                "settle",
                "the replaced scene's transients released and settled cache entries evicted — \
                 a rebuild's work, so zero on a frame that reused its scene",
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
                "repacked",
                "the device threw its glyph atlas away after this frame, so the next one cannot \
                 replay — absent on every frame that did not",
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
    pub(crate) fn summary(&self, trace: Trace) {
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
        self.retention(trace);
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

    /// What the retained frame did over the run, and the one thing outside this program that
    /// stops it working.
    ///
    /// **The claim ADR 0351 has to keep, as a count rather than as a shape in the medians.** A run
    /// whose frames are mostly unchanged should replay mostly, and the bytes are what that reuse
    /// is being held at — the number `doc/QUORRA_RETAINED_FRAME.md` section 6 says to budget a
    /// resident page against, which nothing but the device can compute.
    ///
    /// **The repack is on the same line because it is the one explanation of a low replay count
    /// this program cannot otherwise give.** Every other reason a frame re-encodes is
    /// `render_quorra`'s own scene key, which ADR 0351 enumerated and gave a test each; a repack
    /// is the device's own invalidation (quorra's ADR 0050), and the working set beside it is what
    /// says whether raising the atlas budget would stop it or whether this page has never fitted.
    fn retention(&self, trace: Trace) {
        let replayed = self
            .samples
            .iter()
            .filter(|frame| {
                matches!(
                    frame.gpu.encode_source,
                    Some(quorra_gpu::EncodeSource::Replayed)
                )
            })
            .count();
        let retained = self
            .samples
            .iter()
            .map(|frame| frame.gpu.retained_bytes)
            .max()
            .unwrap_or(0);
        trace.more(
            Topic::Frames,
            format_args!(
                "{replayed} of {} frame(s) replayed a retained encode; the handle held at most \
                 {retained} byte(s)",
                self.samples.len()
            ),
        );
        let repacked = self
            .samples
            .iter()
            .filter(|frame| frame.gpu.atlas_repacked)
            .count();
        let working_set = self
            .samples
            .iter()
            .map(|frame| frame.gpu.atlas_working_set_bytes)
            .max()
            .unwrap_or(0);
        trace.more(
            Topic::Frames,
            format_args!(
                "the atlas was repacked after {repacked} of them; the busiest frame's glyph tiles \
                 wanted {working_set} byte(s) of it"
            ),
        );
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
