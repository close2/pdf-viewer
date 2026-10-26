//! The confined worker's life: starting it, opening the document in it, stopping it, and starting another when it dies.
//!
//! Methods of [`crate::Host`], which this file continues (ADR 2352).

use std::time::Instant;

use viewer_confined::{Canceller, Confined, ConfinedError, Reopen, Reply, Resume};
use viewer_core::{Command, Query, Viewing};
use viewer_host::trace::Topic;

use crate::exchange::Held;
use crate::{DOCUMENT, Host, OVERSIZE, Opened, Presentation, start_worker};

impl Host {
    /// Starts a confined worker, and offers it the machine's faces where this host was asked to.
    ///
    /// One function rather than two call sites, because the first worker and the one that replaces
    /// a dead one must be given the same thing: a window whose *second* worker lost the port would
    /// draw a page differently after a crash, which is the shape nobody would look for.
    fn start_confined(&self, canceller: &Canceller) -> Result<Confined, ConfinedError> {
        start_worker(canceller, self.faces)
    }

    /// Starts the worker and opens the document in it on a thread of its own, while the window
    /// and the graphics device come up on this one (ADR 1539).
    ///
    /// **The launch path's overlap, in this window's shape.** The worker is a process, so what
    /// the thread holds is a pipe: it starts the worker, tells it there is no viewport yet, and
    /// sends `Command::Open`, and the worker interprets page one with nothing to render it into
    /// (`viewer_confined::worker`'s `perform` calls `Viewer::anticipate` after an open). Decoding
    /// stays where it was, inside the confined process; this thread is in the unconfined host and
    /// reads only the file's descriptor and the worker's frames (trap 83 is about the other side).
    /// The first `Resize` after the join then goes straight to the page's marks.
    pub(crate) fn anticipate(&mut self) {
        let (path, faces, canceller, trace, reader) = (
            self.path.clone(),
            self.faces,
            self.canceller.clone(),
            self.trace,
            self.reader.clone(),
        );
        self.opening = Some(viewer_host::Loading::start_reporting(move |progress| {
            let starting = Instant::now();
            let mut confined = start_worker(&canceller, faces).map_err(|e| e.to_string())?;
            // The worker relays how far its page has got inside the exchange this thread waits on,
            // and the window reads the figure this stores it in (ADR 1811).
            confined.report_progress_to(progress);
            trace.say(
                Topic::Launch,
                format_args!(
                    "worker started and confined in {:.1} ms",
                    starting.elapsed().as_secs_f64() * 1e3
                ),
            );
            // Opened rather than read: the file crosses to the worker as its descriptor beside
            // `Command::Open`, and this side holds no byte of it (ADR 0812).
            let bytes = pdf_syntax::FileBytes::on_disk(&path)
                .map_err(|problem| format!("cannot open {}: {problem}", path.display()))?;
            // No viewport until the window has one: the worker's own starting size would have
            // page one drawn at a size nobody asked for.
            let mut events = confined
                .handle(&Command::Resize {
                    width: 0,
                    height: 0,
                    scale: 1.0,
                })
                .map_err(|e| e.to_string())?;
            // The reader's policy before the document, for `Command::Restrict`'s reason: a policy
            // applied halfway through is not a policy. Read here, on this side's filesystem, and
            // carried across the wire as bytes (ADR 1580).
            let supply = reader.commands();
            for refusal in &supply.refused {
                eprintln!("note: {refusal}");
            }
            for command in supply.commands.iter().chain([&OVERSIZE]) {
                events.extend(confined.handle(command).map_err(|e| e.to_string())?);
            }
            events.extend(
                confined
                    .handle(&Command::Open {
                        id: DOCUMENT,
                        bytes,
                        password: None,
                        fragment: None,
                    })
                    .map_err(|e| e.to_string())?,
            );
            trace.say(Topic::Launch, format_args!("document opened in the worker"));
            Ok(Opened { confined, events })
        }));
    }

    /// The worker is gone or unusable; every later command is declined with this sentence.
    ///
    /// The handle is dropped here rather than kept: dropping a `Confined` kills and **reaps** its
    /// worker, and a host that only cancelled would sit beside a zombie for the rest of the
    /// window's life — observed in this program's own first abort, `pdf-view-worker` `<defunct>`
    /// until exit.
    pub(crate) fn stop(&mut self, said: String) {
        eprintln!("{said}");
        self.stopped = Some(said);
        self.confined = None;
        // Nothing is left to do what the reader did meanwhile, and nothing is being waited for.
        self.held.clear();
        self.waiting = None;
        self.retitle();
    }

    /// The confined viewer answered with a refusal instead of a message: another worker, or not.
    ///
    /// **A dead worker is one page's breach, not the document's end**, and this is where that is
    /// decided. `viewer_confined::Resuming` owns the decision — which refusals are worth another
    /// worker, and how many in a row — because it is the part two confined hosts must not disagree
    /// about; what is here is what only a host has: the file, the window, and the page.
    ///
    /// The dead handle is dropped now rather than at the resume, for the reason [`Host::stop`]
    /// drops it: a `Confined` reaps its worker where a `Canceller` only kills, and a window with
    /// a `<defunct>` process beside it is what the first abort in this program's life left behind.
    pub(crate) fn died(&mut self, problem: &ConfinedError) {
        match self.resuming.after(problem) {
            Resume::Stop => self.stop(problem.to_string()),
            Resume::Reopen(reopen) => {
                self.confined = None;
                eprintln!(
                    "{problem}; starting another and returning to page {} at the magnification \
                     and position the reader was at — attempt {} of {}.",
                    reopen.view.page.saturating_add(1),
                    reopen.attempt,
                    reopen.of
                );
                self.heading = format!(
                    "{} — restarting the confined viewer ({} of {})",
                    self.name(),
                    reopen.attempt,
                    reopen.of
                );
                self.retitle();
                self.resume = Some(reopen);
            }
        }
    }

    /// Starts another worker for the same document and puts the reader back on their page.
    ///
    /// The pages already on the screen are left exactly where they are — `doc/todo/37`'s
    /// show-what-it-had, and the reason nothing here forgets them: they are pixels and marks this
    /// side already holds, and a window that blanked while it recovered would tell the reader
    /// something worse than the truth. The next frame replaces them, page by page, when it lands.
    ///
    /// The command that killed the last worker is *not* among these: the resume sends the extent,
    /// the document and the view, and stops. If the document's own open is what kills a worker,
    /// this fails again and `Resuming`'s budget ends it.
    ///
    /// **The view is one command and not three** (ADR 0737): [`Command::View`] states the page,
    /// the magnification and the offset together, in the values [`Query::View`] answered with,
    /// which is what makes the restore exact. Replaying a `GoTo`, a `Zoom` and a `Scroll` of the
    /// difference would land near it and would need this window to know which of the three resets
    /// the others.
    pub(crate) fn reopen(&mut self, reopen: Reopen) {
        if self.stopped.is_some() {
            return;
        }
        self.forget_the_offers();
        let bytes = match pdf_syntax::FileBytes::on_disk(&self.path) {
            Ok(bytes) => bytes,
            Err(problem) => {
                self.stop(format!("cannot open {}: {problem}", self.path.display()));
                return;
            }
        };
        // A canceller of its own, because a cancellation holds the worker it was made for: the
        // old one names a process that has already been reaped, and Escape must reach the new one.
        let canceller = Canceller::new();
        let starting = Instant::now();
        let confined = match self.start_confined(&canceller) {
            Ok(confined) => confined,
            Err(problem) => {
                self.stop(problem.to_string());
                return;
            }
        };
        self.trace.say(
            Topic::Launch,
            format_args!(
                "worker started again and confined in {:.1} ms",
                starting.elapsed().as_secs_f64() * 1e3
            ),
        );
        // Asked of the new worker rather than assumed from the old one: a kernel's answer is
        // about the process that asked, and a person relying on the sandbox is owed each one.
        if let Some(short) = confined.confinement().shortfall() {
            eprintln!("confinement shortfall: {short}");
        }
        self.canceller = canceller;
        self.confined = Some(confined);

        // The restart goes in front of whatever the reader did while the last worker was dying:
        // those acts are for the view this puts back.
        self.ahead_of_the_held(|host| {
            if let Some(window) = host.window.as_ref() {
                let extent = window.inner_size();
                #[expect(
                    clippy::cast_possible_truncation,
                    reason = "a display's scale factor is a small ratio"
                )]
                let scale = window.scale_factor() as f32;
                host.dispatch(Command::Resize {
                    width: extent.width,
                    height: extent.height,
                    scale,
                });
            }
            // The new worker is a new viewer, so it is told again what the first one was told.
            for command in host
                .reader
                .commands()
                .commands
                .into_iter()
                .chain([OVERSIZE])
            {
                host.dispatch(command);
            }
            host.dispatch(Command::Open {
                id: DOCUMENT,
                bytes,
                // The password is deliberately not kept on this side (`password_answered` says
                // why), so an encrypted document asks for it again — §7.6.4.1's card, exactly as
                // at launch.
                password: None,
                fragment: None,
            });
            host.dispatch(Command::View(reopen.view));
        });
    }

    /// The reader's abort: ends the worker and takes back the drawing thread, blocking on
    /// neither.
    ///
    /// `doc/todo/15` asked for exactly this pair — the `Canceller` is the kill the document
    /// cannot decline (ADR 0241), and the interrupt is what reaches the draw a kill does not,
    /// because on the marks arm the drawing is this side's (ADR 0650).
    pub(crate) fn abort(&mut self) {
        self.drawing.superseded(false);
        self.canceller.cancel();
        self.stop(format!(
            "aborted: the confined viewer for {} was ended by the reader",
            self.path.display()
        ));
    }

    /// Ends a worker that is still performing something as this window leaves.
    ///
    /// The command's thread holds the worker's handle, and a process that leaves does not run that
    /// thread's drop, which is what would otherwise have ended it: the worker would draw on into a
    /// pipe nobody reads. The kill is the abort's own (ADR 0241).
    pub(crate) fn end_a_busy_worker(&self) {
        if self.busy() {
            self.canceller.cancel();
        }
    }

    /// Takes in the worker and the document the opening thread started, and tells it the
    /// window's size as it is now.
    pub(crate) fn take_the_opening(&mut self) {
        let Some(window) = self.window.as_ref() else {
            return;
        };
        let extent = window.inner_size();
        #[expect(
            clippy::cast_possible_truncation,
            reason = "a display's scale factor is a small ratio"
        )]
        let scale = window.scale_factor() as f32;
        if self.loading.take().is_some() {
            println!("note: {}", viewer_host::loading::READ);
        }
        // A reader who pressed Escape while the bar was up has ended this worker already, and the
        // window says so; what the thread then answers is the cancel it was told.
        if self.stopped.is_some() {
            drop(self.opening.take().map(viewer_host::Loading::join));
            self.retitle();
            return;
        }
        let opened = match self.opening.take().map(viewer_host::Loading::join) {
            Some(Ok(Ok(opened))) => opened,
            Some(Ok(Err(problem))) => {
                self.stop(problem);
                return;
            }
            Some(Err(_)) => {
                self.stop("the thread opening the document panicked".to_owned());
                return;
            }
            None => {
                self.stop("no document was opened for this window".to_owned());
                return;
            }
        };
        self.trace
            .say(Topic::Launch, format_args!("document joined"));
        // A kernel can refuse what a build offers, and a person relying on the sandbox is owed
        // the difference out loud (`Confined::confinement` is a report, not a promise).
        if let Some(short) = opened.confined.confinement().shortfall() {
            eprintln!("confinement shortfall: {short}");
        }
        self.confined = Some(opened.confined);
        // The size sent below is the window's now, so a size taken while the open was out is owed
        // to nobody; the keys taken then go after what the open itself caused (ADR 1827).
        self.held.retain(|held| {
            !matches!(
                held,
                Held::Command {
                    command: Command::Resize { .. },
                    ..
                }
            )
        });
        self.ahead_of_the_held(|host| {
            for event in opened.events {
                host.event(event);
            }
            host.dispatch(Command::Resize {
                width: extent.width,
                height: extent.height,
                scale,
            });
        });
        // A window with nothing on the screen yet waits for page one instead of polling for it,
        // out of the launch's one-refresh budget (ADR 0678) — same rule, same numbers, third
        // window. Behind a device that wait is for the first frame the render thread finishes,
        // asked for by the frame pull inside the open's dispatch; under `--cpu`, and for a
        // device-refused fallback, it is the drawing thread's.
        if let Some(Presentation::Device(device)) = self.presentation.as_mut()
            && let Some(landed) = device.settle(viewer_host::drawing::SETTLE)
        {
            self.frame_landed(landed);
        }
        for mut finished in self.drawing.settle(viewer_host::drawing::SETTLE) {
            self.declined_in(
                finished.request.page,
                std::mem::take(&mut finished.declined),
            );
            self.screen.landed(finished);
        }
        self.redraw();
    }

    /// Puts the worker's figure for the page in the title, and says so once on the way in.
    pub(crate) fn mind_the_opening(&mut self) {
        let Some(reading) = self
            .opening
            .as_ref()
            .and_then(viewer_host::Loading::showing)
        else {
            return;
        };
        let sentence = viewer_host::loading::reading(reading);
        let said = match viewer_host::loading::bar(reading, 20) {
            Some(bar) => format!("{bar} {sentence}"),
            None => sentence.clone(),
        };
        if self.loading.as_deref() == Some(said.as_str()) {
            return;
        }
        if self.loading.is_none() {
            println!("note: {sentence}");
        }
        self.loading = Some(said);
        self.retitle();
    }

    /// Where the confined viewer says the reader is looking, or nothing if it cannot say.
    ///
    /// A refusal here is deliberately *not* a death: this question is asked beside a frame that
    /// has already crossed, so a worker that will not answer it will be found by the next command
    /// — and treating it as a death here would start a worker from inside the frame pull, which is
    /// exactly the nesting [`Host::resume`] exists to prevent.
    pub(crate) fn view(&mut self) -> Option<Viewing> {
        match self.confined.as_mut()?.query(Query::View) {
            Ok(Reply::View(view)) => Some(view),
            _ => None,
        }
    }
}
