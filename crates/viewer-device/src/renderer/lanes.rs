//! Which lane draws a frame: what each lane charged per command for the pages it drew last, and
//! the frames the device refused, which it is not asked for again (ADRs 1821, 1831, 1886).

use std::time::Duration;

use render_raster::QuorraRasterError;

/// What the window predicted a frame would cost on each lane, where it predicted the processor
/// the cheaper (ADR 1831).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Prediction {
    /// How many commands the arrangement's pages hold between them — the figure both lanes'
    /// costs are read per.
    pub commands: usize,
    /// What the device was predicted to take, and whether from this window's own measurement of
    /// these pages or from a seed (`Costs::DEVICE_SEED`, `Costs::DEVICE_KEPT_SEED`).
    pub device: Duration,
    /// Whether [`Self::device`] is this window's own measurement rather than a seed.
    pub device_measured: bool,
    /// Whether the device's figure is for a frame that draws the scene the device already holds of
    /// these pages (ADR 1877) rather than one that builds it (`Costs::holds`, ADR 1886).
    pub scene_kept: bool,
    /// What the processor was predicted to take, likewise.
    pub processor: Duration,
    /// Whether [`Self::processor`] is this window's own measurement rather than a seed.
    pub processor_measured: bool,
}

/// What each lane cost per command the last time it drew a frame of these pages (ADR 1831).
///
/// **Why a cost per command, and why per page.** ADR 1823 measured where the device's frame goes on
/// the project owner's drawing: host encode, which walks every command — 7 884 ms for 4 408 405 of
/// them, nearly two microseconds each, whatever their size. The processor's cost on a page of
/// hairlines is the same walk at a different price per command, divided among the strips the page
/// grants; on a page of large fills it is the pixels instead. So the count of commands is the one
/// figure both lanes scale with that a frame states before it is drawn, and what each lane charges
/// for one of *this page's* commands is read off the last frame of the page that lane drew. Until a
/// lane has drawn the page, its price is a seed measured on this machine (the constants say where).
///
/// **When it is asked at all.** A frame the device is predicted to draw inside
/// `viewer_host::drawing::WARN` stays on the device whatever the processor's prediction: that is a
/// frame nobody waits for, and the device is where `CLAUDE.md` puts a page. Past it, the lane with
/// the smaller prediction draws — a person waiting seconds is owed the faster of the two, and the
/// processor's draw is the one that can be shown as it is built, taken back and stopped.
///
/// **The device has two prices, and the frame it is asked decides which** (ADR 1886). A frame that
/// builds its scene walks every command into it; a frame of pages whose scene the device already
/// holds re-encodes it at the new view and builds nothing (ADR 1877), and on the owner's drawing that
/// is about a quarter of the build. One price per command for both made the device's first, built,
/// frame price every later one: a page whose build passes `WARN` would go to the processor at every
/// notch of a zoom after it, though each of them is a re-encode.
#[derive(Debug, Default)]
pub(super) struct Costs {
    /// Newest first, at most [`Self::KEPT`].
    held: Vec<Measured>,
    /// The pages whose scene the device holds: those of the last frame it drew, built or kept or
    /// replayed, until it draws others or refuses (`render-raster`'s one `FrameSlot`).
    ///
    /// The window's record rather than the device's word, because the next frame is priced before
    /// it is asked: where the render thread's idle work drew another page on the device in between,
    /// the scene was replaced and the frame priced as kept builds — one frame mispriced, and its
    /// cost read into the built price where it belongs.
    holds: Option<Vec<crate::stale::Picture>>,
}

/// One arrangement's last measured price per command on each lane, in nanoseconds.
#[derive(Debug)]
struct Measured {
    pictures: Vec<crate::stale::Picture>,
    /// A device frame that built its scene.
    device_built: Option<f64>,
    /// A device frame that drew the scene an earlier frame built.
    device_kept: Option<f64>,
    processor: Option<f64>,
    /// Whether the device has been asked these pages at all — it drew them, or refused them at
    /// some magnification — which is what [`Costs::forecast`]'s one ask of an unmeasured device
    /// waits for.
    device_asked: bool,
    /// Whether the processor has drawn these pages, a frame finished from the page drawn while it
    /// was read included — the other half of that one ask's condition.
    ///
    /// Not [`Self::processor`]: a frame finished from its drawing ahead measures nothing (trap
    /// 143), and every notch of a zoom over a page the processor is drawing is taken back before
    /// it lands, so a price is never measured during a gesture and an ask that waited for one
    /// waited until the person stopped (ADR 1886).
    processor_drew: bool,
}

/// Which lane drew a frame, for [`Costs::measure`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Lane {
    /// The device, and whether the frame drew a scene an earlier frame built
    /// (`FrameCost::scene_kept`).
    Device {
        scene_kept: bool,
    },
    Processor,
}

impl Costs {
    /// How many arrangements' prices are kept — [`Refusals::KEPT`]'s count, for its reason.
    const KEPT: usize = 32;

    /// What one command costs the device in a frame that builds its scene, before it has drawn a
    /// page here: the owner's drawing's first zoom notch in this window on the owner's display, on
    /// the Radeon 890M's compute lane and the global pool, the quicker of the first hold's two at
    /// 825.4 ms over 4 408 405 commands (ADR 1886 section 3; the second read 797.3 and 750.5). Headless at four threads the same build is 801.9
    /// and 1 373.6 ms on the `Cpu` lane; the window's own figure is the one it predicts.
    const DEVICE_SEED: f64 = 187.0;

    /// What one command costs the device in a frame that draws a scene it holds, before it has
    /// drawn one here: the notch after that one, the least culled frame of the scene kept, the
    /// quicker of two at 320.8 ms over 4 408 405 commands (ADR 1886 section 3).
    const DEVICE_KEPT_SEED: f64 = 73.0;

    /// What one command costs the processor before it has drawn a page here: the same drawing's
    /// first frame in this window on the owner's display, on the global pool, the quickest of four
    /// at 530.0 ms over 4 408 405 commands (ADR 1860 section 3). Kept rather than re-measured:
    /// that frame is now finished from the page drawn while it was read and prices nothing, and the
    /// one zoomed frame the processor drew in ADR 1886's runs, 785.8 ms at the first notch, prices
    /// above it.
    const PROCESSOR_SEED: f64 = 120.0;

    /// The pictures an arrangement is priced by.
    fn key(pages: &[crate::stale::Placed]) -> Vec<crate::stale::Picture> {
        pages.iter().map(|placed| placed.of).collect()
    }

    /// What `pages` are predicted to cost on each lane, and whether that gives them to the
    /// processor.
    pub(super) fn forecast_for(&self, pages: &[crate::stale::Placed]) -> (Prediction, bool) {
        let commands: usize = pages
            .iter()
            .map(|placed| placed.list.commands().len())
            .sum();
        self.forecast(&Self::key(pages), commands)
    }

    /// [`Self::forecast_for`]'s prediction where it gives the processor the pages; `None` where the
    /// device keeps them.
    #[cfg(test)]
    fn predict(&self, pictures: &[crate::stale::Picture], commands: usize) -> Option<Prediction> {
        let (prediction, processor) = self.forecast(pictures, commands);
        processor.then_some(prediction)
    }

    /// [`Self::forecast_for`] over the pictures and the count it reads.
    fn forecast(&self, pictures: &[crate::stale::Picture], commands: usize) -> (Prediction, bool) {
        let held = self.held.iter().find(|held| held.pictures == *pictures);
        let scene_kept = self.holds.as_deref() == Some(pictures);
        let (device, device_seed) = if scene_kept {
            (
                held.and_then(|held| held.device_kept),
                Self::DEVICE_KEPT_SEED,
            )
        } else {
            (held.and_then(|held| held.device_built), Self::DEVICE_SEED)
        };
        let processor = held.and_then(|held| held.processor);
        let prediction = Prediction {
            commands,
            device: Self::cost(device.unwrap_or(device_seed), commands),
            device_measured: device.is_some(),
            scene_kept,
            processor: Self::cost(processor.unwrap_or(Self::PROCESSOR_SEED), commands),
            processor_measured: processor.is_some(),
        };
        // **The device is asked these pages once before the processor keeps them.** The
        // processor's price is replaced by what it measured the moment it draws a frame, and the
        // device's would be only if it drew one: so a page the seeds gave the processor would keep
        // the device's seed for as long as the window is open, and a device faster than its seed
        // would never be found (ADR 1860). The processor having drawn them is what says the
        // device is the contender here, and the frame it is asked is the one it is measured on.
        //
        // A refusal is an answer too: a device that refused these pages at one magnification is
        // not asked again at the next on the strength of its seed, or every notch of a zoom on a
        // device that refuses would pay the attempt (ADR 1831 section 1 measured 2.2 s of it).
        //
        // "The processor having drawn them" is the processor's frame, not its price: a page drawn
        // while it was read and every notch taken back before it landed are frames the processor
        // drew that measured nothing (`Measured::processor_drew`, ADR 1886).
        let device_unasked = held.is_some_and(|held| !held.device_asked && held.processor_drew);
        let processor = !device_unasked
            && prediction.device >= viewer_host::drawing::WARN
            && prediction.processor < prediction.device;
        (prediction, processor)
    }

    /// What building the device's scene of `pages` is predicted to cost, where it is worth building
    /// while nobody waits — `None` where the device holds it already, or where a frame of these
    /// pages drawn from it would still go to the processor by [`Self::forecast`]'s rule (ADR 1898).
    ///
    /// **The build is what a built frame costs over a kept one**: the scene is walked once and the
    /// encode and the draw are every view's, so the difference between the device's two prices is
    /// the part a later frame no longer pays. It is also how long the view has to have been still
    /// before the build begins, the sharp pass's rent-or-buy bound (ADR 1289): the device's build
    /// cannot be taken back, so a notch arriving during it waits at most as long as the person had
    /// already been still.
    pub(super) fn build_ahead_for(&self, pages: &[crate::stale::Placed]) -> Option<Duration> {
        let commands: usize = pages
            .iter()
            .map(|placed| placed.list.commands().len())
            .sum();
        self.build_ahead(&Self::key(pages), commands)
    }

    /// [`Self::build_ahead_for`] over the pictures and the count it reads.
    fn build_ahead(&self, pictures: &[crate::stale::Picture], commands: usize) -> Option<Duration> {
        if self.holds.as_deref() == Some(pictures) {
            return None;
        }
        let held = self.held.iter().find(|held| held.pictures == *pictures);
        let built = held
            .and_then(|held| held.device_built)
            .unwrap_or(Self::DEVICE_SEED);
        let kept = held
            .and_then(|held| held.device_kept)
            .unwrap_or(Self::DEVICE_KEPT_SEED);
        let processor = held
            .and_then(|held| held.processor)
            .unwrap_or(Self::PROCESSOR_SEED);
        let kept_frame = Self::cost(kept, commands);
        let processor_keeps = kept_frame >= viewer_host::drawing::WARN
            && Self::cost(processor, commands) < kept_frame;
        (!processor_keeps).then(|| Self::cost((built - kept).max(0.0), commands))
    }

    /// `commands` at `nanoseconds` each.
    fn cost(nanoseconds: f64, commands: usize) -> Duration {
        #[expect(
            clippy::cast_precision_loss,
            reason = "a count of commands priced for a prediction, far inside f64's exact range"
        )]
        let total = nanoseconds * commands as f64 / 1e9;
        Duration::try_from_secs_f64(total).unwrap_or(Duration::MAX)
    }

    /// Keeps what `lane` took to draw `pages`, newest first.
    pub(super) fn measure(&mut self, pages: &[crate::stale::Placed], lane: Lane, took: Duration) {
        let commands: usize = pages
            .iter()
            .map(|placed| placed.list.commands().len())
            .sum();
        self.measured(Self::key(pages), commands, lane, took);
    }

    /// [`Self::measure`] over the pictures and the count it reads.
    fn measured(
        &mut self,
        pictures: Vec<crate::stale::Picture>,
        commands: usize,
        lane: Lane,
        took: Duration,
    ) {
        if commands == 0 {
            return;
        }
        #[expect(
            clippy::cast_precision_loss,
            reason = "a count of commands priced for a prediction, far inside f64's exact range"
        )]
        let each = took.as_secs_f64() * 1e9 / commands as f64;
        let mut measured = self.taken(pictures);
        match lane {
            Lane::Device { scene_kept: true } => {
                measured.device_kept = Some(each);
                measured.device_asked = true;
            }
            Lane::Device { scene_kept: false } => {
                measured.device_built = Some(each);
                measured.device_asked = true;
            }
            Lane::Processor => {
                measured.processor = Some(each);
                measured.processor_drew = true;
            }
        }
        self.held.insert(0, measured);
        self.held.truncate(Self::KEPT);
    }

    /// Keeps that the processor drew `pages` in a frame that priced nothing — one finished from
    /// the page drawn while it was read, or one taken back ([`Measured::processor_drew`]).
    pub(super) fn drawn_by_the_processor(&mut self, pages: &[crate::stale::Placed]) {
        let mut measured = self.taken(Self::key(pages));
        measured.processor_drew = true;
        self.held.insert(0, measured);
        self.held.truncate(Self::KEPT);
    }

    /// Keeps that the device's scene is of `pages` now, or of nothing after a refusal.
    pub(super) fn scene_of(&mut self, pages: Option<&[crate::stale::Placed]>) {
        self.holds = pages.map(Self::key);
    }

    /// Keeps that the device refused `pages`, which prices nothing but is the device's answer for
    /// [`Self::forecast`]'s one ask.
    pub(super) fn refused(&mut self, pages: &[crate::stale::Placed]) {
        self.refused_pictures(Self::key(pages));
    }

    /// [`Self::refused`] over the pictures it reads.
    fn refused_pictures(&mut self, pictures: Vec<crate::stale::Picture>) {
        let mut measured = self.taken(pictures);
        measured.device_asked = true;
        self.held.insert(0, measured);
        self.held.truncate(Self::KEPT);
    }

    /// The entry for `pictures`, taken out to be put back newest, or a new one where none is held.
    fn taken(&mut self, pictures: Vec<crate::stale::Picture>) -> Measured {
        match self.held.iter().position(|held| held.pictures == pictures) {
            Some(at) => self.held.remove(at),
            None => Measured {
                pictures,
                device_built: None,
                device_kept: None,
                processor: None,
                device_asked: false,
                processor_drew: false,
            },
        }
    }
}

/// The frames the graphics device refused, so that it is not asked for them again (ADR 1821).
///
/// **A refused frame costs the device's whole attempt before the processor starts**: raster prices
/// a frame's edges after it has flattened them (`raster-gpu`'s `ComputePersist::price`), so on the
/// project owner's drawing every refusal is the scene built, the edges counted, the frame priced
/// and turned down — and then the processor's draw on top. The answer does not change while the
/// question does not, so it is asked once.
///
/// **The question is the pages and the magnification, not the whole view.** Raster culls what
/// reaches no pixel of the target, so a pan at a magnification well past the page's fit could in
/// principle bring a refused frame under the budget; remembering by magnification means such a
/// pan is drawn by the processor, which is the correctness oracle and draws the same page, more
/// slowly. The other choice — the whole transform — would ask the device again at every pan of a
/// refused page and pay the attempt each time, which is the cost this exists to remove.
///
/// Only a refusal that is a fact about the frame or about a lane is kept ([`remembered`]); a
/// device lost says nothing about either and is asked again, as it always was.
///
/// **A lane the adapter refused is kept apart from the frames, and for the device's life** (ADR
/// 1968). The compute lane's four pipelines are built or refused together on the first frame that
/// takes the lane, and raster keeps the answer (ADR 1962), so every later frame of that coverage
/// would be the scene encoded only to hear the same words. Those frames go to the processor without
/// the device being asked, whatever their pages and magnification, and age out of nothing: the
/// window's device is the render thread's, which lives as long as the window.
#[derive(Debug, Default)]
pub(super) struct Refusals {
    /// Newest first, at most [`Self::KEPT`].
    held: Vec<Refused>,
    /// The coverage lanes the adapter refused, with what it said — at most one entry per
    /// [`raster_gpu::Coverage`], never aged.
    lanes: Vec<(raster_gpu::Coverage, String)>,
}

/// What a refusal the window keeps is a fact about (ADRs 1821, 1968).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kept {
    /// These pages at this magnification.
    Frame,
    /// Every frame drawn under this coverage lane on this device.
    Lane(raster_gpu::Coverage),
}

/// One frame the device refused: which pictures, at which magnification, and what it said.
#[derive(Debug)]
struct Refused {
    pictures: Vec<crate::stale::Picture>,
    linear: [u32; 4],
    said: String,
}

impl Refusals {
    /// How many refusals are kept: a zoom gesture's worth of magnifications of one page, and the
    /// pages a column shows beside it. An entry that ages out costs one device attempt, once.
    const KEPT: usize = 32;

    /// The key a frame is remembered by: its pictures, and the linear part of the first page's
    /// placement — the arrangement's magnification, since `viewer_core::layout` places every page
    /// of a column at the same one.
    fn key(pages: &[crate::stale::Placed]) -> Option<(Vec<crate::stale::Picture>, [u32; 4])> {
        let first = pages.first()?;
        Some((
            pages.iter().map(|placed| placed.of).collect(),
            crate::lane::linear_bits_of(first.target.transform),
        ))
    }

    /// What the device said when it refused this frame's coverage lane, or exactly these pages at
    /// exactly this magnification, and which of the two it was. A frame with no page asks nothing
    /// of a lane, so it is never refused.
    pub(super) fn said(
        &self,
        pages: &[crate::stale::Placed],
        coverage: raster_gpu::Coverage,
    ) -> Option<(&str, Kept)> {
        let (pictures, linear) = Self::key(pages)?;
        if let Some((lane, said)) = self.lanes.iter().find(|(lane, _)| *lane == coverage) {
            return Some((said.as_str(), Kept::Lane(*lane)));
        }
        self.held
            .iter()
            .find(|held| held.linear == linear && held.pictures == pictures)
            .map(|held| (held.said.as_str(), Kept::Frame))
    }

    /// Keeps a refusal: a lane's for the device's life, a frame's newest first.
    pub(super) fn remember(&mut self, pages: &[crate::stale::Placed], said: &str, kept: Kept) {
        if let Kept::Lane(coverage) = kept {
            self.lanes.retain(|(lane, _)| *lane != coverage);
            self.lanes.push((coverage, said.to_owned()));
            return;
        }
        let Some((pictures, linear)) = Self::key(pages) else {
            return;
        };
        self.held
            .retain(|held| !(held.linear == linear && held.pictures == pictures));
        self.held.insert(
            0,
            Refused {
                pictures,
                linear,
                said: said.to_owned(),
            },
        );
        self.held.truncate(Self::KEPT);
    }
}

/// What a refusal heard on a frame drawn under `coverage` is a fact about, where it is a fact
/// about anything this window can keep (ADRs 1821, 1968).
///
/// **The frame** — its pages at its magnification — for the budgets and the limits: raster prices
/// what the frame asks for against what it states it will hold, and the same frame is priced the
/// same way next time. So is what the display list itself carries — a value the scene boundary
/// refuses, a clip or a soft mask it does not define, a paint this backend cannot draw — because the
/// list is the same list. And so is a render pipeline, a shader module or a generated §7.10.5
/// program the adapter would not build: raster keeps each refusal for the device's life and refuses
/// in the same words every frame that needs it (ADRs 1929, 1962), and the same frame needs the same
/// ones.
///
/// **The lane** for the compute lane's pipelines: they are built or refused together by the first
/// frame that takes the lane, so the refusal is the answer to every frame of that coverage. A
/// refused compute *module* arrives as `PipelineProblem::Shader`, under a label that names the
/// module and not its lane, so it is kept by the frame: one encode per frame rather than one per
/// device, and never a frame sent to the processor that did not need to be.
///
/// A device lost, a readback failed, or an allocation the machine could not make at that moment
/// is a fact about neither, and a page refused for one of those is asked again.
pub(super) fn remembered(
    problem: &QuorraRasterError,
    coverage: raster_gpu::Coverage,
) -> Option<Kept> {
    use raster_gpu::{PipelineProblem, RenderError};
    match problem {
        QuorraRasterError::Render(RenderError::PipelineUnavailable {
            reason: PipelineProblem::ComputePipeline { .. },
        }) => Some(Kept::Lane(coverage)),
        QuorraRasterError::Render(
            RenderError::FrameBudgetExceeded { .. }
            | RenderError::ScratchExhausted { .. }
            | RenderError::OutlineConversionBudgetExceeded { .. }
            | RenderError::TargetTooLarge { .. }
            | RenderError::ViewportTransformTooLarge { .. }
            | RenderError::PipelineUnavailable { .. },
        )
        | QuorraRasterError::Scene(_)
        | QuorraRasterError::UnknownClip(_)
        | QuorraRasterError::CyclicClip(_)
        | QuorraRasterError::UnknownSoftMask(_)
        | QuorraRasterError::Unsupported(_)
        | QuorraRasterError::Target(_) => Some(Kept::Frame),
        // The device, an allocation, and whatever `render-raster` adds later: asked again until
        // somebody argues that the new one is a fact about the frame.
        _ => None,
    }
}

/// The lane the processor's picture of a frame goes back to the device through, where the adapter
/// refused the frame's own (ADR 1968).
///
/// The compute lane's pipelines are the only lane refusal kept, and two lanes reach them: `Compute`
/// always, and `Cpu` where raster's compute assist sends the compute lane the tiles the atlas will
/// not hold (ADR 0090). `Cpu` is the exact lane and draws the chrome's text from the atlas, so it
/// takes a refused `Compute` lane's picture; `Gpu` sends nothing to the compute lane, so it takes a
/// refused `Cpu` lane's.
pub(super) fn upload_lane(refused: raster_gpu::Coverage) -> raster_gpu::Coverage {
    match refused {
        raster_gpu::Coverage::Cpu => raster_gpu::Coverage::Gpu,
        raster_gpu::Coverage::Gpu | raster_gpu::Coverage::Compute => raster_gpu::Coverage::Cpu,
    }
}

/// The coverage lanes the adapter refused, as the render thread knows them (ADRs 1968, 1984, 2000).
///
/// [`Refusals`] is the event thread's, and it reaches a frame as the job's [`Known`]; a retained
/// picture is drawn while no job is in hand, so the render thread keeps the lanes it has heard of
/// for itself — from every frame that carried a lane's refusal or heard one, and from the retained
/// layer's own ask, sorted by the frames' rule ([`remembered`]). A refusal about a frame is not a
/// fact about another page, so it is not kept here. Never aged, for [`Refusals`]'s reason: the
/// device is this thread's and lives as long as it does.
///
/// **The window is told a lane the retained layer heard** (`Finished::Lane`), once, and this record
/// answers a job asked before the window read it ([`Self::said`]): either way the device is asked
/// once per window for a lane, whichever path heard the refusal first (ADR 2000).
///
/// [`Known`]: super::Known
#[derive(Debug, Default)]
pub(super) struct RefusedLanes(Vec<(raster_gpu::Coverage, String)>);

impl RefusedLanes {
    /// Keeps a refusal where it is a lane's, with what the adapter said; answers whether the lane
    /// was new to this thread.
    pub(super) fn learn(&mut self, kept: Option<Kept>, said: &str) -> bool {
        let Some(Kept::Lane(lane)) = kept else {
            return false;
        };
        if self.refused(lane) {
            return false;
        }
        self.0.push((lane, said.to_owned()));
        true
    }

    /// Whether the adapter refused this lane's pipelines on this device.
    pub(super) fn refused(&self, lane: raster_gpu::Coverage) -> bool {
        self.0.iter().any(|(refused, _)| *refused == lane)
    }

    /// What the adapter said when it refused this frame's coverage lane, where it did — the
    /// answer [`Refusals::said`] gives for a lane, for a job asked before the window heard of it.
    /// A frame with no page asks nothing of a lane, so it is never refused.
    pub(super) fn said(
        &self,
        pages: &[crate::stale::Placed],
        coverage: raster_gpu::Coverage,
    ) -> Option<(&str, Kept)> {
        if pages.is_empty() {
            return None;
        }
        self.0
            .iter()
            .find(|(refused, _)| *refused == coverage)
            .map(|(lane, said)| (said.as_str(), Kept::Lane(*lane)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    use pdf_render::{DisplayList, TargetSpec, Transform};

    const CPU: raster_gpu::Coverage = raster_gpu::Coverage::Cpu;
    const COMPUTE: raster_gpu::Coverage = raster_gpu::Coverage::Compute;

    /// A page of `page`'s number in document 1, placed at `magnification` and shifted right by
    /// `pan` device pixels — the two things a view change does to one page.
    fn placed(page: usize, magnification: f32, pan: f32) -> Vec<crate::stale::Placed> {
        vec![crate::stale::Placed {
            of: crate::stale::Picture::new(viewer_core::DocumentId(1), page, 0),
            list: Arc::new(DisplayList::new(pdf_render::Size::new(595.0, 842.0))),
            target: TargetSpec {
                width: 800,
                height: 1000,
                transform: Transform::scale(magnification, -magnification)
                    .then(Transform::translate(pan, 842.0 * magnification)),
            },
            depicts: crate::stale::Depicts::Page,
        }]
    }

    /// **A refusal is remembered by its pages and its magnification, and not by where the page
    /// sits** (ADR 1821): a pan of a refused page is the same question, a notch of zoom or another
    /// page is a different one.
    #[test]
    fn a_refusal_is_remembered_by_its_pages_and_magnification() {
        let mut refusals = Refusals::default();
        refusals.remember(&placed(0, 1.0, 0.0), "frame refused: too much", Kept::Frame);
        assert_eq!(
            refusals.said(&placed(0, 1.0, 120.0), CPU),
            Some(("frame refused: too much", Kept::Frame)),
            "a pan of the refused page is not asked again"
        );
        assert_eq!(
            refusals.said(&placed(0, 1.25, 0.0), CPU),
            None,
            "a notch of zoom is asked"
        );
        assert_eq!(
            refusals.said(&placed(1, 1.0, 0.0), CPU),
            None,
            "another page is asked"
        );
        assert_eq!(refusals.said(&[], CPU), None, "no page is never refused");
    }

    /// The memory is bounded, newest first, and the same refusal learned twice is one entry.
    #[test]
    fn the_memory_of_refusals_keeps_the_newest() {
        let mut refusals = Refusals::default();
        for step in 0..=Refusals::KEPT {
            #[expect(
                clippy::cast_precision_loss,
                reason = "a step count far below f32's exact integer range"
            )]
            let magnification = 1.0 + step as f32 / 8.0;
            refusals.remember(&placed(0, magnification, 0.0), "refused", Kept::Frame);
            refusals.remember(&placed(0, magnification, 0.0), "refused", Kept::Frame);
        }
        assert_eq!(refusals.held.len(), Refusals::KEPT);
        assert_eq!(
            refusals.said(&placed(0, 1.0, 0.0), CPU),
            None,
            "the oldest aged out"
        );
        assert!(refusals.said(&placed(0, 1.125, 0.0), CPU).is_some());
    }

    /// The owner's drawing, before anything has been measured: the device's seed for a frame that
    /// builds its scene keeps it inside `WARN`, so the device draws it, at the figures the seeds were
    /// measured as (ADRs 1860, 1886).
    #[test]
    fn the_owner_s_drawing_stays_on_the_device_on_the_seeds() {
        let pictures = [crate::stale::Picture::new(viewer_core::DocumentId(1), 0, 0)];
        let (prediction, processor) = Costs::default().forecast(&pictures, 4_408_405);
        assert!(!processor, "0.82 s on the device is inside WARN");
        assert!(!prediction.device_measured && !prediction.processor_measured);
        assert!(!prediction.scene_kept);
        assert_eq!(prediction.device.as_millis(), 824);
        assert_eq!(prediction.processor.as_millis(), 529);
    }

    /// A frame the device is predicted to draw inside `WARN` stays there, whatever the processor
    /// is predicted to take: nobody waits for it, and the device is where page one goes. At the
    /// seeds the line is `WARN` over 187 ns, 5 347 593 commands.
    #[test]
    fn a_frame_nobody_waits_for_stays_on_the_device() {
        let pictures = [crate::stale::Picture::new(viewer_core::DocumentId(1), 0, 0)];
        assert_eq!(Costs::default().predict(&pictures, 5_300_000), None);
        assert!(Costs::default().predict(&pictures, 5_400_000).is_some());
    }

    /// Commands enough that the seeds give the processor the page: twice the owner's drawing,
    /// whose own 4 408 405 the device's seed now keeps inside `WARN`.
    const HEAVY: usize = 8_816_810;

    /// What a lane measured on these pages replaces its seed, and only on these pages.
    #[test]
    fn a_measured_price_replaces_the_seed_for_its_own_pages() {
        let page = |page| {
            vec![crate::stale::Picture::new(
                viewer_core::DocumentId(1),
                page,
                0,
            )]
        };
        let mut costs = Costs::default();
        // Large fills: the processor took twenty seconds over the page, so the device draws it.
        costs.measured(page(0), HEAVY, Lane::Processor, Duration::from_secs(20));
        assert_eq!(costs.predict(&page(0), HEAVY), None);
        assert!(
            costs.predict(&page(1), HEAVY).is_some(),
            "another page is still priced by the seeds"
        );
        // A device that drew it in half a second keeps it, measured.
        costs.measured(
            page(2),
            HEAVY,
            Lane::Device { scene_kept: false },
            Duration::from_millis(500),
        );
        assert_eq!(costs.predict(&page(2), HEAVY), None);
        // The processor measured faster than the device measured: the processor, and it says so.
        costs.measured(
            page(2),
            HEAVY,
            Lane::Device { scene_kept: false },
            Duration::from_secs(8),
        );
        costs.measured(page(2), HEAVY, Lane::Processor, Duration::from_secs(2));
        let chose = costs.predict(&page(2), HEAVY).expect("2 s against 8 s");
        assert!(chose.device_measured && chose.processor_measured);
        assert_eq!(costs.held.len(), 2, "one entry an arrangement");
    }

    /// A page the seeds gave the processor is asked of the device once, after the processor has
    /// drawn it, so a device faster than its seed is found; and a device that refused the page has
    /// answered, so the seeds decide again rather than asking at every magnification (ADR 1860).
    #[test]
    fn the_device_is_asked_once_a_page_the_seeds_gave_the_processor() {
        let pictures = vec![crate::stale::Picture::new(viewer_core::DocumentId(1), 0, 0)];
        let mut costs = Costs::default();
        assert!(
            costs.predict(&pictures, HEAVY).is_some(),
            "the seeds give the processor the first frame"
        );
        costs.measured(
            pictures.clone(),
            HEAVY,
            Lane::Processor,
            Duration::from_millis(900),
        );
        assert_eq!(
            costs.predict(&pictures, HEAVY),
            None,
            "the next frame is the device's, its one ask"
        );
        costs.measured(
            pictures.clone(),
            HEAVY,
            Lane::Device { scene_kept: false },
            Duration::from_millis(1_500),
        );
        let chose = costs
            .predict(&pictures, HEAVY)
            .expect("measured, 0.9 s against 1.5 s");
        assert!(chose.device_measured && chose.processor_measured);

        let refusing = vec![crate::stale::Picture::new(viewer_core::DocumentId(1), 1, 0)];
        costs.measured(
            refusing.clone(),
            HEAVY,
            Lane::Processor,
            Duration::from_millis(900),
        );
        costs.refused_pictures(refusing.clone());
        let chose = costs
            .predict(&refusing, HEAVY)
            .expect("a refusal is the device's answer; the seed decides again");
        assert!(!chose.device_measured && chose.processor_measured);
    }

    /// A frame the processor drew that priced nothing — one finished from the page drawn while it
    /// was read, or one taken back by the next notch — is the processor's frame of the page, so the
    /// device is asked the next one; and once the device holds the page's scene, the next frame is
    /// priced by what drawing a held scene costs, not by what building it did (ADR 1886).
    #[test]
    fn the_device_is_asked_after_a_frame_drawn_ahead_and_a_kept_scene_is_priced_as_kept() {
        let at_fit = |page: usize| placed(page, 1.0, 0.0);
        let pictures = |page: usize| Costs::key(&at_fit(page));
        let commands = HEAVY;
        let mut costs = Costs::default();
        assert!(
            costs.predict(&pictures(0), commands).is_some(),
            "the seeds give the processor a page this heavy"
        );
        costs.drawn_by_the_processor(&at_fit(0));
        assert_eq!(
            costs.predict(&pictures(0), commands),
            None,
            "a page drawn ahead is the processor's frame: the device is asked the next"
        );
        // The device's first frame built the scene in 1.5 s; drawing it again is a re-encode.
        costs.measured(
            pictures(0),
            commands,
            Lane::Device { scene_kept: false },
            Duration::from_millis(1_500),
        );
        costs.measured(
            pictures(0),
            commands,
            Lane::Processor,
            Duration::from_millis(900),
        );
        let chose = costs
            .predict(&pictures(0), commands)
            .expect("no scene held: 0.9 s against the 1.5 s build");
        assert!(!chose.scene_kept);
        costs.scene_of(Some(&at_fit(0)));
        let (kept, processor) = costs.forecast(&pictures(0), commands);
        assert!(kept.scene_kept && !kept.device_measured);
        assert!(
            !processor,
            "a held scene at the kept seed stays on the device"
        );
        costs.measured(
            pictures(0),
            commands,
            Lane::Device { scene_kept: true },
            Duration::from_millis(180),
        );
        let (kept, processor) = costs.forecast(&pictures(0), commands);
        assert!(kept.scene_kept && kept.device_measured && !processor);
        assert!((kept.device.as_secs_f64() - 0.18).abs() < 1e-6);
        // Another page's frame on the device replaces the scene, and the build prices again.
        costs.scene_of(Some(&at_fit(1)));
        assert!(costs.predict(&pictures(0), commands).is_some());
        costs.scene_of(None);
        assert!(!costs.forecast(&pictures(1), commands).0.scene_kept);
    }

    /// Only a refusal that is a fact about the frame or the lane is kept: the budget and a refused
    /// render pipeline are the frame's, the compute lane's pipelines are the lane's, a lost device
    /// is neither (ADR 1968).
    #[test]
    fn only_a_refusal_about_the_frame_or_the_lane_is_remembered() {
        use raster_gpu::{PipelineProblem, RenderError};
        let refused =
            |reason| QuorraRasterError::Render(RenderError::PipelineUnavailable { reason });
        assert_eq!(
            remembered(
                &QuorraRasterError::Render(RenderError::FrameBudgetExceeded {
                    needed: 423_206_976,
                    budget: 268_435_456,
                }),
                CPU
            ),
            Some(Kept::Frame)
        );
        assert_eq!(
            remembered(
                &refused(PipelineProblem::ComputePipeline {
                    pipeline: "raster compute count",
                    detail: "no entry point".to_owned(),
                }),
                COMPUTE
            ),
            Some(Kept::Lane(COMPUTE)),
            "the compute lane's pipelines answer every frame of the lane"
        );
        assert_eq!(
            remembered(
                &refused(PipelineProblem::Pipeline {
                    pipeline: "raster image quad",
                    format: raster_gpu::wgpu::TextureFormat::Rgba8Unorm,
                    detail: "no entry point".to_owned(),
                }),
                CPU
            ),
            Some(Kept::Frame),
            "a render pipeline answers the frames that need it"
        );
        assert_eq!(
            remembered(
                &refused(PipelineProblem::Shader {
                    shader: "raster compute scan",
                    detail: "does not parse".to_owned(),
                }),
                COMPUTE
            ),
            Some(Kept::Frame),
            "a module is named by its label, not by its lane"
        );
        assert_eq!(
            remembered(
                &QuorraRasterError::Render(RenderError::DeviceLost {
                    detail: "gone".to_owned(),
                }),
                CPU
            ),
            None
        );
        assert_eq!(
            remembered(
                &QuorraRasterError::Allocation {
                    width: 1,
                    height: 1,
                },
                CPU
            ),
            None
        );
    }

    /// **A refused lane answers every frame of its coverage, at any page and magnification, and
    /// for the device's life** (ADR 1968): no number of frame refusals ages it out, and a frame of
    /// another coverage is still asked.
    #[test]
    fn a_refused_lane_answers_every_frame_of_its_coverage_for_the_device_s_life() {
        let mut refusals = Refusals::default();
        let said = "frame refused: a pipeline this frame needs could not be built";
        refusals.remember(&placed(0, 1.0, 0.0), said, Kept::Lane(COMPUTE));
        for step in 0..=Refusals::KEPT {
            #[expect(
                clippy::cast_precision_loss,
                reason = "a step count far below f32's exact integer range"
            )]
            let magnification = 1.0 + step as f32 / 8.0;
            refusals.remember(&placed(1, magnification, 0.0), "refused", Kept::Frame);
        }
        assert_eq!(
            refusals.said(&placed(7, 3.5, 40.0), COMPUTE),
            Some((said, Kept::Lane(COMPUTE))),
            "another page at another magnification is not asked"
        );
        assert_eq!(
            refusals.said(&placed(7, 3.5, 40.0), CPU),
            None,
            "a frame of another coverage is"
        );
        assert_eq!(refusals.said(&[], COMPUTE), None, "no page asks nothing");
        refusals.remember(&placed(2, 1.0, 0.0), said, Kept::Lane(COMPUTE));
        assert_eq!(refusals.lanes.len(), 1, "a lane is one entry");
    }

    /// **The render thread keeps a lane's refusal, from a frame or from its own ask, and nothing
    /// else** (ADR 1984): a refusal about a frame says nothing about another page, and a lane
    /// heard twice is one lane, new once — the once the window is told (ADR 2000).
    #[test]
    fn the_render_thread_keeps_a_lane_s_refusal_and_not_a_frame_s() {
        use raster_gpu::{PipelineProblem, RenderError};
        let mut lanes = RefusedLanes::default();
        assert!(!lanes.learn(Some(Kept::Frame), "refused"));
        assert!(!lanes.learn(None, "lost"));
        assert!(
            !lanes.refused(CPU) && !lanes.refused(COMPUTE),
            "nothing is a lane's"
        );
        assert!(lanes.learn(Some(Kept::Lane(COMPUTE)), "carried"));
        assert!(
            lanes.refused(COMPUTE),
            "a frame's carried refusal of its lane"
        );
        assert!(!lanes.refused(CPU), "is its lane's alone");
        let own = QuorraRasterError::Render(RenderError::PipelineUnavailable {
            reason: PipelineProblem::ComputePipeline {
                pipeline: "raster compute count",
                detail: "no entry point".to_owned(),
            },
        });
        let said = own.to_string();
        assert!(
            lanes.learn(remembered(&own, CPU), &said),
            "the retained layer's own ask, sorted by the frames' rule, is new and is told"
        );
        assert!(
            !lanes.learn(remembered(&own, CPU), &said),
            "and heard again is not told again"
        );
        assert!(lanes.refused(CPU));
        assert_eq!(lanes.0.len(), 2, "a lane heard twice is one lane");
    }

    /// **A lane the retained layer heard answers the next frame of it, on either side of the
    /// thread** (ADR 2000): the render thread's record answers a job asked before the window was
    /// told, and the window, told, answers it at the ask as it does a lane a frame heard.
    #[test]
    fn a_lane_the_retained_layer_heard_answers_the_next_frame_on_either_side() {
        let said = "frame refused: a pipeline this frame needs could not be built";
        let mut lanes = RefusedLanes::default();
        assert!(lanes.learn(Some(Kept::Lane(CPU)), said));
        let page = placed(3, 0.5, 0.0);
        assert_eq!(lanes.said(&page, CPU), Some((said, Kept::Lane(CPU))));
        assert_eq!(lanes.said(&page, raster_gpu::Coverage::Gpu), None);
        assert_eq!(lanes.said(&[], CPU), None, "no page asks nothing");
        let mut refusals = Refusals::default();
        assert_eq!(
            refusals.said(&page, CPU),
            None,
            "the window before it is told"
        );
        refusals.remember(&[], said, Kept::Lane(CPU));
        assert_eq!(
            refusals.said(&page, CPU),
            lanes.said(&page, CPU),
            "and after: one answer on both sides"
        );
    }

    /// The processor's picture of a frame whose lane was refused goes back through a lane that
    /// does not reach the compute lane's pipelines (ADR 1968).
    #[test]
    fn a_refused_lane_s_picture_goes_back_through_another() {
        assert_eq!(upload_lane(COMPUTE), CPU);
        assert_eq!(upload_lane(CPU), raster_gpu::Coverage::Gpu);
    }

    /// **The build ahead is asked where the device would draw the next frame from it, and only
    /// then** (ADR 1898): at the seeds it is what a built frame costs over a kept one; not where
    /// the device holds the scene already; not where a kept frame would still be the processor's.
    #[test]
    fn a_scene_is_built_ahead_only_where_the_device_would_draw_from_it() {
        let at_fit = placed(0, 1.0, 0.0);
        let pictures = Costs::key(&at_fit);
        let mut costs = Costs::default();
        let witness = 4_408_405;
        let ahead = costs
            .build_ahead(&pictures, witness)
            .expect("the seeds keep a kept frame of the owner's drawing on the device");
        let seeds = Costs::cost(Costs::DEVICE_SEED - Costs::DEVICE_KEPT_SEED, witness);
        assert_eq!(ahead, seeds);
        costs.scene_of(Some(&at_fit));
        assert_eq!(costs.build_ahead(&pictures, witness), None, "held already");
        costs.scene_of(None);
        costs.measured(
            pictures.clone(),
            HEAVY,
            Lane::Device { scene_kept: true },
            Duration::from_secs(2),
        );
        costs.measured(
            pictures.clone(),
            HEAVY,
            Lane::Processor,
            Duration::from_millis(900),
        );
        assert_eq!(
            costs.build_ahead(&pictures, HEAVY),
            None,
            "a kept frame of 2 s past WARN against the processor's 0.9 s is the processor's"
        );
    }
}
