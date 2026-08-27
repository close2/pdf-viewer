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
    /// happen inside one `QuorraWindowRenderer::present` call, so this host cannot take a clock
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
    ///
    /// `first` is the frame that closed the timeline, and it is an argument rather than something
    /// this type could compute: see [`Self::frame`] for why the table is not finished without it.
    pub(crate) fn arrived(&mut self, trace: Trace, first: &Stages) {
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
        Self::frame(trace, first);
    }

    /// What the *first frame* was made of, under the table it is the last two rows of.
    ///
    /// **The launch table's last two steps are one frame, and a step is only as useful as its
    /// name.** `first scene built` and `first present` are the frame's translation and the frame's
    /// device work, and on a large drawing they are together half the launch — a lump exactly like
    /// the one ADR 0332 split out of `document joined → first present`, one round of optimisation
    /// later. The per-frame line the trace already prints carries `scene` and `device` and stops
    /// there, and the only place the phases *inside* `device` appear is the summary at exit, as
    /// medians over every frame of the run: a distribution cannot answer a question about the one
    /// frame a person waited for, and a launch measured by killing the window never prints one at
    /// all.
    ///
    /// So the phases are printed here, off the same [`render_quorra::FrameCost`] the frame line
    /// reads — including [`render_quorra::FrameCost::handover`], which divides `scene` into this
    /// host's own walk of the display list and the boundary it hands each resource across (ADR
    /// 0423). `readback` is not in the arithmetic because a window frame never reads back; the
    /// remainder is named `elsewhere` for the same reason and with the same caveat the summary
    /// gives it.
    fn frame(trace: Trace, first: &Stages) {
        let gpu = first.gpu;
        if gpu.total == std::time::Duration::ZERO {
            // A window with no graphics device drew this page on the processor, so there are no
            // phases to name and one number that says everything: `CLAUDE.md` puts page one on the
            // device, and a launch that did not is the fact worth printing.
            trace.more(
                Topic::Launch,
                format_args!(
                    "the first frame was drawn on the processor in {:.1} ms — no device phases",
                    ms(first.composed)
                ),
            );
            return;
        }
        let elsewhere = gpu.device.saturating_sub(
            gpu.encode
                .saturating_add(gpu.upload)
                .saturating_add(gpu.execute),
        );
        trace.more(
            Topic::Launch,
            format_args!(
                "the first frame's own phases, ms: scene {:.1} (of it {:.1} handed to the device, \
                 {} outline segment(s) in {} upload(s)), device {:.1} = encode {:.1} + transfer \
                 {:.1} + execute {:.1} + elsewhere {:.1}, present {:.1}",
                ms(gpu.scene),
                ms(gpu.handover),
                gpu.outline_segments,
                gpu.uploads,
                ms(gpu.device),
                ms(gpu.encode),
                ms(gpu.upload),
                ms(gpu.execute),
                ms(elsewhere),
                ms(first.present),
            ),
        );
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
    /// The first page this frame drew, counting from one.
    pub(crate) page: usize,
    /// How many pages Table 29's arrangement put in this frame.
    ///
    /// One under `SinglePage`, which is the table's own default; more under `OneColumn` and the
    /// four two-page values. Beside [`Self::page`] rather than replacing it, because a person
    /// reading a trace of a scroll wants to know *where* they are as well as how much is up.
    pub(crate) pages: usize,
    /// How many display-list commands the pages carried between them.
    pub(crate) commands: usize,
    /// This host's own work before anything is handed over: the page's geometry, the selection,
    /// the focus ring, the caret, the popup windows and the panel, each a query into the core.
    pub(crate) host: std::time::Duration,
    /// What `render-quorra` reported for the same frame, which is where the device's own
    /// accounting arrives from.
    pub(crate) gpu: render_quorra::FrameCost,
    /// What drawing this frame cost the composing thread, on a window with no device.
    ///
    /// **Somebody else's thread since ADR 0461**, which is why it is beside [`Self::total`] rather
    /// than inside it: `render-cpu` walks the arrangement on `crate::composer`'s thread and this
    /// is what it reported, so a tick whose `total` is a tenth of this is the window keeping the
    /// cadence while a frame of hundreds of milliseconds is drawn beside it. It was the event
    /// thread's own elapsed time until then, and the two numbers were the same number.
    pub(crate) composed: std::time::Duration,
    /// Acquiring the swapchain, recording three textured quads and presenting them.
    ///
    /// **The whole of what the event thread spends on a picture, since ADR 0391**, and the number
    /// `doc/todo/36`'s rate stands or falls on: everything in [`Self::gpu`] happens on another
    /// thread now, so a frame line whose `present` is inside one refresh is a window keeping the
    /// cadence however long its render takes. quorra's own `PresentCost`, summed over the three
    /// wall clocks it reports — and they are wall clocks, which that type says in their names.
    pub(crate) present: std::time::Duration,
    /// The accessibility publication — measured *beside* the frame rather than inside it.
    pub(crate) attend: std::time::Duration,
    /// Which approximate layers this frame was made of, or `None` for a rendering of the page.
    ///
    /// `doc/todo/37`'s third rule: a reader of the trace must never have to infer that a frame
    /// was approximated. It is in the frame line's outcome word, it is counted in the summary,
    /// and it is here rather than derived from a small duration for the same reason
    /// [`render_quorra::FrameCost::encode_source`] is an observable — an inference is not
    /// something a person or a test can assert on.
    ///
    /// **A [`crate::stale::Source`] rather than a `bool` since ADR 0443**, because there are two
    /// approximate layers now and they are different amounts of wrong: a frame line that said
    /// only `approximated` for both had stopped saying what it was showing.
    pub(crate) approximated: Option<crate::stale::Source>,
    /// Which way rule 5 knew the frame this stand-in covers for had missed — set exactly
    /// where [`Self::approximated`] is, so the trace's sentence states the reason that
    /// actually fired rather than always printing the prediction (which, on a tick where
    /// the observation fired, was a sentence about the wrong number).
    pub(crate) missed: Option<crate::stale::Missed>,
    /// Whether this frame put anything on the window at all.
    ///
    /// `doc/todo/36`'s sixth rule counts *presents*, and the two things that are not one are a
    /// frame with nothing to show and a swapchain state that asked for a redraw. A rendering and
    /// a reprojection are both presents, which is why this is not `!approximated`.
    pub(crate) presented: bool,
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
    /// When the last frame that put something on the window did so.
    ///
    /// `doc/todo/36`'s rule 6 asks for the distribution of intervals *between presents*, which is
    /// a different sequence from the frames: a frame that had nothing to show is not a present,
    /// and the gap either side of it is one interval rather than two.
    presented_at: Option<std::time::Instant>,
    /// The interval before each present after the first, capped at [`FRAME_SAMPLES`].
    intervals: Vec<std::time::Duration>,
    /// How many presents there were, and how many of those were reprojections.
    ///
    /// **The two claims `doc/todo/36` rule 6 says must both be in the record.** "We present every
    /// 8.3 ms" is [`Self::intervals`]; "we show the right pixels" is this pair, and neither
    /// implies the other.
    presents: usize,
    /// See [`Self::presents`].
    approximated: usize,
    /// Whether the line explaining the frame line's columns has been printed.
    legend: bool,
    /// Whether the closing line for pipeline compilation has been printed.
    pub(crate) pipelines: bool,
    /// Whether the sentence about a swapchain that cannot be rebuilt has been printed.
    ///
    /// Once per run rather than once per frame: the state does not clear itself, so a window in
    /// it produces the refusal at every refresh and a person would get a screenful of one fact.
    pub(crate) refused_swapchain: bool,
}

/// How to read one stage out of one frame.
type StageOf = fn(&Stages) -> std::time::Duration;

/// The summary's rows: what to call each stage and how to read it.
///
/// One table rather than nine `format!`s, so that a stage added later cannot appear in the
/// per-frame line and go missing from the percentiles.
const SUMMARY_ROWS: [(&str, StageOf); 11] = [
    ("frame", |frame| frame.total),
    ("host", |frame| frame.host),
    ("scene", |frame| frame.gpu.scene),
    // Inside `scene`, and the only part of it this host does not do itself: what the `upload_*`
    // calls across quorra's boundary took. Nested under it exactly as the three device phases are
    // nested under `device`, because it is a subdivision and not a phase beside one — a reader who
    // added it to the rows above would be counting the same milliseconds twice. ADR 0423.
    ("  handover", |frame| frame.gpu.handover),
    ("device", |frame| frame.gpu.device),
    ("  encode", |frame| frame.gpu.encode),
    ("  transfer", |frame| frame.gpu.upload),
    ("  execute", |frame| frame.gpu.execute),
    // What is inside `Device::render` and outside the three phases it names. Printed as its own
    // row rather than left for a reader to subtract, because an unnamed remainder is where a cost
    // hides — and on this machine it is a sixth of the frame.
    //
    // **This comment used to name what was in it — "acquiring the swapchain texture, presenting
    // it, and reading the timestamp queries back" — and session 552 measured those three and they
    // are not it.** `render_quorra::QuorraWindowRenderer::last_phases` now carries quorra's own
    // `target acquire` and `present` spans across the boundary, and on the project owner's adapter
    // the pair is under a twentieth of a millisecond on a frame whose remainder is over a hundred.
    // What is left is host time inside `Device::render` that quorra measures and discards — it
    // times its own submit-and-wait and then reports the adapter's timestamp instead — plus the
    // wgpu command recording, which nothing times at all. `doc/QUORRA_FEEDBACK.md` section 29.
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
    ("composed", |frame| frame.composed),
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
        if stages.presented {
            let now = std::time::Instant::now();
            self.presents = self.presents.saturating_add(1);
            if stages.approximated.is_some() {
                self.approximated = self.approximated.saturating_add(1);
            }
            // The interval is taken here rather than at the present itself so that the sequence
            // is the one a person watching the window sees: one sample per frame that changed
            // what is on it, measured between the ends of two such frames.
            if let Some(previous) = self.presented_at.replace(now)
                && self.intervals.len() < FRAME_SAMPLES
            {
                self.intervals.push(now.saturating_duration_since(previous));
            }
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
        for (name, spent) in [("composed", stages.composed), ("attend", stages.attend)] {
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
            Some(quorra_gpu::EncodeSource::RecordReplayed) => " re-placed",
            Some(quorra_gpu::EncodeSource::Encoded) => " encoded",
            None => "",
        };
        // Table 29's arrangement, in the page column: `p3` is one page and `p3+1` is page three
        // with one more of the column beside or below it. Said rather than left to be inferred
        // from a command count that doubled.
        let column = if stages.pages > 1 {
            format!("+{}", stages.pages.saturating_sub(1))
        } else {
            String::new()
        };
        trace.say(
            Topic::Frames,
            format_args!(
                "frame p{}{column} {}cmd {outcome} {:.1} | present {:.2} | host {:.1} scene {:.1} \
                 device {:.1}{source} settle {:.1}{unusual} | {} up, {} culled",
                stages.page,
                stages.commands,
                ms(stages.total),
                ms(stages.present),
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
                "present",
                "the swapchain acquired and three textured quads put on the window, on this \
                 thread — the whole of what a refresh waits for since ADR 0391, because every \
                 column after it is another thread's",
            ),
            (
                "host",
                "this host's queries — page geometry, selection, focus, caret, popups, panel",
            ),
            (
                "scene",
                "display lists translated into a GPU scene, this frame's uploads included — on \
                 the render thread, and reported by the frame that landed at this tick",
            ),
            (
                "device",
                "quorra's render: encoding, transfers, and the passes it already waits on — \
                 followed by whether it encoded this frame's scene or replayed one (ADR 0351). \
                 Also the render thread's, and zero on a tick that adopted no frame",
            ),
            (
                "settle",
                "the replaced scene's transients released and settled cache entries evicted — \
                 a rebuild's work, so zero on a frame that reused its scene",
            ),
            (
                "composed",
                "render-cpu drawing this frame on the composing thread, for a window with no \
                 graphics device — absent on every frame that had one",
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
            (
                "approximated",
                "the outcome of a frame that is the previous view's own pixels moved rather \
                 than a rendering of the page — always followed by the real frame (ADR 0378). \
                 It says which layers filled the picture: the last frame alone, the last frame \
                 over a retained low-resolution page, or such a page alone where nothing about \
                 the last frame is true of this view (ADR 0443)",
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
    pub(crate) fn summary(
        &self,
        trace: Trace,
        approximated: crate::stale::Drawn,
        refused: crate::stale::Refusals,
        cadence: &crate::cadence::Cadence,
    ) {
        if !trace.on(Topic::Frames) || self.count == 0 {
            return;
        }
        self.cadence(trace, cadence);
        let truncated = if self.count > self.samples.len() {
            format!(", percentiles over the first {}", self.samples.len())
        } else {
            String::new()
        };
        trace.say(
            Topic::Frames,
            format_args!("{} frame(s){truncated}, milliseconds:", self.count),
        );
        // Rule 3's count, and it is deliberately printed even when it is zero: "none of these
        // frames was approximated" is the answer a person reading a trace of a slow session
        // most needs, and a line that appears only sometimes cannot give it.
        trace.more(
            Topic::Frames,
            format_args!(
                "{} of them stood in while the real frame was built — {} the previous view's \
                 pixels moved, {} those over a retained low-resolution page, {} such a page \
                 alone; every one was replaced by the frame it stood in for (ADR 0378, ADR 0443)",
                approximated.total(),
                approximated.last_frame,
                approximated.over_pages,
                approximated.pages_only
            ),
        );
        // Rule 3's other count, and ADR 0385's half of it. A reprojection that does not happen is
        // what the project owner reported twice, so the *number* of them belongs beside the number
        // that did: a refusal named in one frame line of six hundred is a line nobody finds, and
        // the two kinds answer different questions — an impossibility is a fact about the
        // document or the window, a judgement is a decision this program made and can be argued
        // with. Printed at zero for the same reason the line above is.
        trace.more(
            Topic::Frames,
            format_args!(
                "{} view change(s) showed the real frame instead: {} had nothing true to move and \
                 {} were a judgement between two measurements — each says which, above (ADR \
                 0385). {} more had no sharp layer and were drawn from a retained page (ADR 0443)",
                refused.total(),
                refused.impossible,
                refused.unwise,
                refused.proxied
            ),
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
        self.transferred(trace);
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

    /// The cadence asked for, the cadence achieved, and how many of the presents were correct.
    ///
    /// **`doc/todo/36`'s rule 6, which is this project's first acceptance criterion that is a
    /// rate.** Three things are printed and they are three different claims:
    ///
    /// - what the presenter *asks for*, which is the surface's own refresh rate where it states
    ///   one and `doc/todo/36`'s floor where it does not;
    /// - what it *achieved*, as a distribution and never as a mean — the failure mode of a
    ///   cadence is the tail, so the median is printed beside p90, p99 and the worst interval,
    ///   and a mean would hide exactly the frame a person notices;
    /// - what share of the presents were the page rather than a picture of it moved, because a
    ///   window that hits its cadence by reprojecting everything has answered the wrong question.
    fn cadence(&self, trace: Trace, cadence: &crate::cadence::Cadence) {
        let asked = cadence.period();
        trace.say(
            Topic::Frames,
            format_args!("presenting on a cadence of {}", cadence.described()),
        );
        let correct = self.presents.saturating_sub(self.approximated);
        trace.more(
            Topic::Frames,
            format_args!(
                "{} present(s): {correct} a rendering of the page and {} a reprojection of one \
                 ({:.1}% correct)",
                self.presents,
                self.approximated,
                percent(correct, self.presents),
            ),
        );
        if self.intervals.is_empty() {
            trace.more(
                Topic::Frames,
                format_args!(
                    "fewer than two presents, so this run states no interval distribution"
                ),
            );
            return;
        }
        let mut sorted = self.intervals.clone();
        sorted.sort_unstable();
        // **How many refreshes the gap spans, to the nearest one** — `gap ≤ 1.5 × period` — and
        // not `gap ≤ period`. A present aimed at the next refresh lands a few microseconds
        // either side of it, so the strict test is a coin toss on exactly the intervals that hit
        // their mark: measured on the witness it read 53% where the median interval was 16.6 ms
        // against a 16.667 ms refresh. The question this answers is the one worth asking — did
        // the window present on the next refresh, or on a later one.
        let within = sorted
            .iter()
            .filter(|gap| gap.saturating_mul(2) <= asked.saturating_mul(3))
            .count();
        trace.more(
            Topic::Frames,
            format_args!(
                "intervals between presents, ms: median {:.1}  p90 {:.1}  p99 {:.1}  max {:.1} \
                 — {} of {} on the next refresh ({:.1}%)",
                ms(at_rank(&sorted, 1, 2)),
                ms(at_rank(&sorted, 9, 10)),
                ms(at_rank(&sorted, 99, 100)),
                ms(sorted.last().copied().unwrap_or_default()),
                within,
                sorted.len(),
                percent(within, sorted.len()),
            ),
        );
    }

    /// What the `transfer` row was moving, which is the denominator that row never had.
    ///
    /// **The line above this one counts *resources* and it is a different quantity**, which is
    /// the whole reason this exists. `up` is what `render-quorra`'s caches handed the device —
    /// outlines, images, ramps, programs — and on a magnification of the project owner's own
    /// drawing it is 40 while `transfer` is tens of milliseconds. Reading the second as the cost
    /// of the first is the inference a trace with only one of the two numbers invites, and it is
    /// wrong by three orders of magnitude: the bytes are quorra's own encoded scene — coverage
    /// tiles, instance streams, the atlas — staged for the device on every frame that encodes,
    /// and a frame that uploads no resource at all still moves megabytes of them. ADR 0387.
    ///
    /// quorra has counted them since ADR 0227 and `FrameCost` has carried the number across the
    /// boundary ever since; nothing read it until this line.
    fn transferred(&self, trace: Trace) {
        let mut bytes: Vec<u64> = self
            .samples
            .iter()
            .map(|frame| frame.gpu.bytes_uploaded)
            .collect();
        bytes.sort_unstable();
        let total: u64 = bytes.iter().copied().fold(0, u64::saturating_add);
        trace.more(
            Topic::Frames,
            format_args!(
                "the transfer row moved {total} byte(s) over those frames: median {}, most {} \
                 in one — quorra's own encoded scene, which is not what up counts",
                at_rank(&bytes, 1, 2),
                bytes.last().copied().unwrap_or(0),
            ),
        );
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

/// `part` as a percentage of `whole`, and zero rather than a division by nothing.
#[expect(
    clippy::cast_precision_loss,
    reason = "a count of frames, far below f64's exact integer range"
)]
fn percent(part: usize, whole: usize) -> f64 {
    if whole == 0 {
        return 0.0;
    }
    part as f64 * 100.0 / whole as f64
}

/// The value `numerator/denominator` of the way through a sorted sample, by nearest rank.
///
/// Integer arithmetic throughout: the workspace forbids unchecked arithmetic, and a percentile
/// computed in floating point would have to justify its rounding as well as its rank.
///
/// Generic over the sample because the summary now has one distribution that is not a duration —
/// the bytes quorra staged for the device — and two ranks that disagreed about their rounding
/// would be worse than one that is used twice.
fn at_rank<T: Copy + Default>(sorted: &[T], numerator: usize, denominator: usize) -> T {
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
