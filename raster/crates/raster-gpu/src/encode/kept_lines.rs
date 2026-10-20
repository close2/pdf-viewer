//! The line lane's batches kept from the encode before, so that a step to a new view walks only
//! what the view changed (`doc/todo/75` item 3, viewer ADR 2056).
//!
//! A line's instance is the page's and the view is the shader's (ADR 2022): an instance is a
//! function of its command and the scene alone, and the vertex stage collapses a line the view
//! admits nothing of. A walk culls such a line, as it always has, so that a page's first frame in a
//! window sends what the window shows; **a walk that culled none wrote the page's whole stream**, and
//! what a walk at another view would do with each of those lines is known before it starts: the
//! same bytes at the same place in the stream, the same batch, the same lane count — and one thing
//! of the view's own, the box it grows its plan by. So an encode records its lines as runs —
//! consecutive commands of the page, consecutive instances of the stream, one clip chain, one soft
//! mask ([`KeptRun`]) — and the encode of the next view of the same scene takes each run in one step
//! where the walk would take its lines one by one, re-walking every other command as the walk always
//! has (`encode.rs`'s `encode_kept`). A line the new view admits nothing of stays in the kept stream
//! and draws nothing, which a fresh walk would have culled: the pixels are the fresh walk's, and the
//! line lane's count and `commands_culled` are the first walk's.
//!
//! **A walk that culled a line keeps nothing**, and the next view is walked whole, culling again: a
//! window opened on a page of lines pays what it shows and no more. The page whose first frame
//! showed all of it — the fit a page opens at — is the one whose later views keep its lines; a
//! window opened zoomed gets them from the idle build ([`Before::Whole`], viewer ADR 2134), a walk
//! at the same view that culls no line, made while nobody waits, whose encode replaces the first.
//!
//! **Only the page's own plan keeps runs.** A line inside a group or a soft mask is walked into a
//! plan the walk makes again at every view, and its instance is written where that walk reaches it;
//! an encode with one such line keeps nothing, and the next view is walked whole.
//!
//! **What is asked again, and what is checked.** A run's chain is resolved where its first line
//! would have resolved it and its mask planned there, so the clip resolver and the mask plans meet
//! them in the walk's own order. The chain must still be rectangles alone and state the page's
//! rectangle the run's instances carry; a line outside every run must still take the stroke road.
//! Where either fails, the kept walk is abandoned and the view walked whole, which is the encode the
//! tree made before this module — so a kept walk draws the walk's frame or does not stand in for it.
//!
//! **The box is the view's, and it is a union.** Each line grows the page's plan by its ink at this
//! view inside the rectangle its chain and the target admit, exactly as `push_line_instance` grows
//! it. The page's plan's box is read only as the region it renders into (`Region::of`, ADR 0036),
//! whole pixels clamped to the target, and a union of boxes each inside the target cannot move that
//! region once it is the whole target — so the lines after that are not asked, and the region is the
//! walk's. Where the page's medium covers the target, which is every view past the fit, no line is
//! asked at all.

use raster_scene::{ClipId, Command, MaskId, Rect};

use super::Encoder;
use super::device_space::unviewed;
use super::instance::BatchKind;
use super::line::{LineMark, device_ink};
use crate::compose::Region;
use crate::error::RenderError;

/// What a walk takes from the encode of the view before.
#[derive(Debug, Clone, Copy)]
pub(super) enum Before<'a> {
    /// Nothing: the walk walks every line, culling a line its view admits nothing of.
    Nothing,
    /// Nothing, and the walk is for the views after it: it culls no line, so the stream it writes
    /// is the page's and its runs are every later view's (the idle build, viewer ADR 2134). A line
    /// its view admits nothing of writes its instance and grows no box, which is what the vertex
    /// stage draws of it at this view.
    Whole,
    /// That encode's runs, which the walk takes in one step each.
    Kept(&'a super::Encoded),
    /// That encode's runs turned out not to be this view's: the walk is abandoned and the view
    /// walked whole.
    Diverged,
}

impl Before<'_> {
    /// Whether the walk keeps the encode before's runs, and so writes no line instance of its own.
    pub(super) fn keeps(self) -> bool {
        matches!(self, Self::Kept(_))
    }

    /// Whether the kept runs turned out not to be this view's.
    pub(super) fn diverged(self) -> bool {
        matches!(self, Self::Diverged)
    }

    /// Whether the walk culls a line its view admits nothing of: every walk but the idle build's.
    pub(super) fn culls(self) -> bool {
        !matches!(self, Self::Whole)
    }

    /// The kept runs are not this view's; a walk that keeps none has nothing to abandon.
    pub(super) fn diverge(&mut self) {
        if self.keeps() {
            *self = Self::Diverged;
        }
    }
}

/// One run of the line lane's stream: consecutive top-level commands, each of which wrote one
/// closed-form instance, the instances consecutive in the stream, under one clip chain and one soft
/// mask.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct KeptRun {
    /// The scene index of the run's first command.
    command: usize,
    /// The scene index one past the run's last command: one store a line where the walk extends
    /// the run.
    end: usize,
    /// The stream index of the run's first instance.
    instance: u32,
    clip: Option<ClipId>,
    mask: Option<MaskId>,
    /// The chain's rectangle in the page's space, which every instance of the run carries: a
    /// resolution at another view that states another has met a different chain, and the kept
    /// bytes are not that view's.
    page_rect: Rect,
}

impl KeptRun {
    /// The scene index one past the run's last command.
    fn end(&self) -> usize {
        self.end
    }

    /// How many commands, and instances, the run holds — below `u32::MAX`, since the stream
    /// indexes its instances in a `u32` and the frame budget refuses a stream long before.
    fn count(&self) -> u32 {
        u32::try_from(self.end.saturating_sub(self.command)).unwrap_or(u32::MAX)
    }
}

/// An encode's line lane as runs, for the encode of the next view of the same scene.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct KeptLines {
    runs: Vec<KeptRun>,
}

impl KeptLines {
    /// Heap bytes the runs hold, for the retained encode's count.
    pub(crate) fn bytes(&self) -> u64 {
        (self.runs.capacity() as u64).saturating_mul(size_of::<KeptRun>() as u64)
    }
}

/// The runs a walk is writing.
///
/// `whole` is also the walk's culling's: a line the walk culled is not in the stream, so the stream
/// is this view's and not the page's, and nothing is kept (`line.rs`'s `place_line`).
///
/// **The run being extended is held here and not at the end of the vector**: the walk extends it
/// once a line, and a store a line into a small heap block, which the allocator may place beside
/// the blocks the line makers write on other threads, is the hazard ADR 2022 measured — 60–80 ms
/// of the witness's walk in some processes and none in others.
#[derive(Debug)]
pub(super) struct KeptBuilder {
    ended: Vec<KeptRun>,
    open: Option<KeptRun>,
    /// False once a line instance went to a plan other than the page's, or a line was culled:
    /// such an encode keeps nothing.
    pub(super) whole: bool,
}

impl Default for KeptBuilder {
    fn default() -> Self {
        Self {
            ended: Vec::new(),
            open: None,
            whole: true,
        }
    }
}

impl KeptBuilder {
    /// The runs written, or `None` where some line instance is not the page's plan's.
    pub(super) fn finish(mut self) -> Option<KeptLines> {
        if !self.whole {
            return None;
        }
        if let Some(open) = self.open.take() {
            self.ended.push(open);
        }
        Some(KeptLines { runs: self.ended })
    }
}

impl<'a> Encoder<'a> {
    /// Note that the line at scene index `index`, under `clip` and `mask`, has just written the
    /// stream's last instance under the chain whose page rectangle is `page_rect` — the walk's
    /// note where no thread made its lines ahead (`lines_ahead` notes a block's runs at once).
    pub(super) fn keep_line(
        &mut self,
        index: usize,
        key: (Option<ClipId>, Option<MaskId>),
        page_rect: Rect,
    ) {
        let instance = self.last_instance(BatchKind::Line) as usize;
        self.keep_run((index, 1), instance, key, page_rect);
    }

    /// Note that the `count` lines from scene index `index` on, under `clip` and `mask`, wrote the
    /// stream's instances from `instance` on, under the chain whose page rectangle is
    /// `page_rect`: the run before extended where this one continues it, a new run otherwise.
    pub(super) fn keep_run(
        &mut self,
        (index, count): (usize, u32),
        instance: usize,
        (clip, mask): (Option<ClipId>, Option<MaskId>),
        page_rect: Rect,
    ) {
        if !self.kept_build.whole {
            return;
        }
        let Ok(instance) = u32::try_from(instance) else {
            // A stream the frame budget admits indexes its instances in a `u32`.
            self.kept_build.whole = false;
            return;
        };
        if self.current_plan != usize::MAX {
            self.kept_build.whole = false;
            return;
        }
        let end = index.saturating_add(count as usize);
        if let Some(open) = self.kept_build.open.as_mut()
            && open.end == index
            && open.instance.saturating_add(open.count()) == instance
            && open.clip == clip
            && open.mask == mask
        {
            // One chain is one resolution in a walk, so its page rectangle is the run's: asked
            // once, where the run starts.
            open.end = end;
            return;
        }
        let started = KeptRun {
            command: index,
            end,
            instance,
            clip,
            mask,
            page_rect,
        };
        if let Some(ended) = self.kept_build.open.replace(started) {
            self.kept_build.ended.push(ended);
        }
    }

    /// Walk `commands` in order, each kept run in one step and every other command as the walk
    /// takes it, leaving [`Before::Diverged`] where the kept runs are not this view's.
    pub(super) fn walk_kept(
        &mut self,
        commands: &'a [Command],
        kept: &KeptLines,
    ) -> Result<(), RenderError> {
        let mut runs = kept.runs.iter().peekable();
        let mut at = 0;
        while at < commands.len() && !self.before.diverged() {
            if let Some(run) = runs.next_if(|run| run.command == at) {
                self.place_kept_run(run, commands)?;
                at = run.end();
                continue;
            }
            if let Some(command) = commands.get(at) {
                self.command(at, command)?;
            }
            at = at.saturating_add(1);
        }
        // A run the walk never reached starts inside another, or past the scene's end: the runs
        // are not this scene's.
        if runs.next().is_some() {
            self.before.diverge();
        }
        Ok(())
    }

    /// One kept run in its place in the walk: every step [`Encoder::encode_line`] takes for each of
    /// its lines in the frame's order, taken once where its answer is the run's.
    fn place_kept_run(&mut self, run: &KeptRun, commands: &[Command]) -> Result<(), RenderError> {
        self.begin_command_records(run.command);
        self.record_slow();
        let mask_index = match run.mask {
            None => None,
            mask => self.use_mask(mask)?,
        };
        // A soft mask takes the frame off the replay road at its first line, so only an unmasked
        // run writes a record for every line after it, as the walk does.
        if self.replay.is_some() {
            for index in run.command.saturating_add(1)..run.end() {
                self.begin_command_records(index);
                self.record_slow();
            }
        }
        let resolved = self.resolve_clip(run.clip)?;
        if resolved.residues.is_some() || resolved.page_rect != run.page_rect {
            self.before.diverge();
            return Ok(());
        }
        let admit = resolved.rect.intersection(self.visible);
        // A queued mark draws before the run, as it would draw before the run's first line; drawn
        // now rather than queued behind, which commits the same ops in the same order (`parallel`).
        self.drain_queue()?;
        let count = run.count();
        self.lanes.line = self.lanes.line.saturating_add(count);
        self.lines_kept = self.lines_kept.saturating_add(count);
        let style = self.style;
        self.note_run(BatchKind::Line, (run.instance, count), style, mask_index);
        self.mark_kept_ink(run, commands, admit);
        Ok(())
    }

    /// Grow the page's plan by each line of `run`'s ink at this view inside `admit`, as
    /// `push_line_instance` grows it, until the plan's region is the whole target.
    fn mark_kept_ink(&mut self, run: &KeptRun, commands: &[Command], admit: Rect) {
        let view = unviewed(self.viewport.transform);
        let (width, height) = (self.viewport.width, self.viewport.height);
        let whole = Region::whole(width, height);
        for index in run.command..run.end() {
            if Region::of(self.plan_mut().bounds, width, height) == whole {
                return;
            }
            let Some(ink) = commands
                .get(index)
                .and_then(LineMark::of)
                .and_then(|line| device_ink(&line, &view))
            else {
                // The run was written for a closed-form line here, and at every view that keeps
                // the axes the road is the page's.
                self.before.diverge();
                return;
            };
            let ink = ink.intersection(admit);
            if !ink.is_empty() {
                self.plan_mut()
                    .mark([ink.min.x, ink.min.y, ink.max.x, ink.max.y]);
            }
        }
    }
}
