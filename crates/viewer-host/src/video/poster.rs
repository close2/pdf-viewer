//! §13.4's poster retrieved from the movie file: Table 306's `/Poster` `true` — "the poster image
//! shall be retrieved from the movie file" — drawn by the window over the movie annotation's
//! rectangle until the movie plays, and again after it stops (ADR 3609).
//!
//! **What is decoded where.** The page renderer decodes no video, so the page is drawn without the
//! poster and the viewer raises `viewer_core::Event::Poster` with the movie's file for a window
//! that shows video. The file goes to a media worker of the posters' own, on a thread of their own
//! started at the first poster and never at open: the movie's first H.264 picture is taken — the
//! first the worker hands on, which is the first presented on the movie's timeline once the edit
//! list is applied (ADR 3554), "displayable" in the sense that one composed outside it is never
//! handed on — and the decode stopped there. The picture is converted on that thread by
//! [`super::on_the_processor`], the oracle's conversion, so the window takes a finished raster and
//! converts nothing.
//!
//! **Its own worker, not the player's.** The player's thread decodes a playing clip a pass at a
//! time and holds back on a queue of pictures; a poster asked for while a movie plays would wait
//! for the pass to end, and one decoded between its pictures would have to be told apart from
//! them. A second worker started only where a page holds a poster keeps both simple.
//!
//! **What is held.** One raster for each poster of a document that is open, up to
//! [`HELD_BYTES`]; a poster past that is not drawn and is said, and a document's posters go when it
//! closes.

use std::ops::ControlFlow;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender};
use std::thread::JoinHandle;
use std::time::Duration;

use pdf_media_worker::{MediaError, MediaWorker};
use pdf_render::video::Conversion;
use pdf_syntax::ObjectId;
use viewer_core::DocumentId;

use super::{Frame, Seen};
use crate::sound::Wake;

/// The most the posters held at once may weigh: eight pictures of 1920 by 1080, which is more
/// posters than a screen shows, and a bound a document of a thousand movies cannot pass.
pub const HELD_BYTES: usize = 8 * 1920 * 1080 * 4;

/// A poster asked for: `viewer_core::Event::Poster`'s fields.
#[derive(Debug, Clone)]
pub struct Asked {
    /// The document the movie annotation is in.
    pub document: DocumentId,
    /// The movie annotation, whose rectangle the poster is drawn in.
    pub annotation: ObjectId,
    /// The movie file, read only by the media worker.
    pub movie: Vec<u8>,
    /// How the picture is fitted to the rectangle: the fit the movie plays with.
    pub fit: pdf_model::media::Fit,
}

/// One poster shown: the picture, and where and how it goes.
#[derive(Debug)]
pub struct Poster<'a> {
    /// The document the movie annotation is in.
    pub document: DocumentId,
    /// The movie annotation.
    pub annotation: ObjectId,
    /// The movie's first picture, converted.
    pub raster: &'a pdf_render::Raster,
    /// How it is fitted to the annotation's rectangle.
    pub fit: pdf_model::media::Fit,
}

impl Poster<'_> {
    /// Where the poster goes inside `quad`, the annotation's rectangle in window pixels
    /// ([`super::placed`]).
    #[must_use]
    pub fn placed(&self, quad: [f32; 8]) -> Option<super::Placed> {
        super::placed(self.fit, (self.raster.width, self.raster.height), quad)
    }
}

/// One poster, asked for or decoded.
#[derive(Debug)]
struct Held {
    document: DocumentId,
    annotation: ObjectId,
    fit: pdf_model::media::Fit,
    raster: Option<pdf_render::Raster>,
}

/// What the thread is sent.
struct Order {
    document: DocumentId,
    annotation: ObjectId,
    movie: Vec<u8>,
}

/// What the thread answers.
struct Answer {
    document: DocumentId,
    annotation: ObjectId,
    raster: Result<pdf_render::Raster, String>,
}

/// The decoding thread and its two channels.
struct Running {
    orders: Option<Sender<Order>>,
    answers: Receiver<Answer>,
    thread: Option<JoinHandle<()>>,
}

/// A window's posters: none, and no thread or worker, until the first.
#[derive(Default)]
pub struct Posters {
    worker: Option<PathBuf>,
    running: Option<Running>,
    held: Vec<Held>,
    seen: Vec<Seen>,
}

impl std::fmt::Debug for Posters {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Posters")
            .field("started", &self.running.is_some())
            .field("held", &self.held)
            .finish_non_exhaustive()
    }
}

impl Posters {
    /// The same, decoding with the worker `program` rather than the one beside this executable.
    #[must_use]
    pub fn with_worker(mut self, program: PathBuf) -> Self {
        self.worker = Some(program);
        self
    }

    /// Asks for `asked`'s poster, once for each annotation of a document. The first poster starts
    /// the thread, with `wake` as what wakes the window's loop when a poster is ready; later ones
    /// do not call it.
    pub fn ask(&mut self, asked: Asked, wake: impl FnOnce() -> Wake) {
        if self
            .held
            .iter()
            .any(|held| held.document == asked.document && held.annotation == asked.annotation)
        {
            return;
        }
        let running = self
            .running
            .get_or_insert_with(|| start(self.worker.clone(), wake()));
        let sent = running.orders.as_ref().is_some_and(|orders| {
            orders
                .send(Order {
                    document: asked.document,
                    annotation: asked.annotation,
                    movie: asked.movie,
                })
                .is_ok()
        });
        if !sent {
            self.seen.push(Seen::Refused(
                "a movie's poster is not shown, because the poster decoding thread has ended"
                    .to_owned(),
            ));
            return;
        }
        self.held.push(Held {
            document: asked.document,
            annotation: asked.annotation,
            fit: asked.fit,
            raster: None,
        });
    }

    /// Takes every poster the thread has finished; answers whether one came up. Never waits.
    pub fn look(&mut self) -> bool {
        let Some(running) = self.running.as_ref() else {
            return false;
        };
        let mut changed = false;
        while let Ok(answer) = running.answers.try_recv() {
            let at = self.held.iter().position(|held| {
                held.document == answer.document && held.annotation == answer.annotation
            });
            // A poster of a document closed since it was asked for.
            let Some(at) = at else { continue };
            let label = format!(
                "the poster of the movie annotation {} {} R",
                answer.annotation.number, answer.annotation.generation
            );
            match answer.raster {
                Ok(raster) if self.weight().saturating_add(raster.data.len()) <= HELD_BYTES => {
                    self.held[at].raster = Some(raster);
                    changed = true;
                }
                Ok(_) => {
                    self.held.remove(at);
                    self.seen.push(Seen::Refused(format!(
                        "{label} is not shown, because the posters already shown hold the {} \
                         MiB this window keeps for them",
                        HELD_BYTES >> 20
                    )));
                }
                // Table 306's poster is not drawn: the rectangle shows what the page draws there.
                Err(why) => {
                    self.held.remove(at);
                    self.seen.push(Seen::Refused(format!(
                        "{label} is not shown, because {why}"
                    )));
                }
            }
        }
        changed
    }

    /// Whether a poster has been asked for and not yet answered: what a window that is not woken
    /// looks again for.
    #[must_use]
    pub fn awaited(&self) -> bool {
        self.held.iter().any(|held| held.raster.is_none())
    }

    /// `document` closed: its posters go.
    pub fn closed(&mut self, document: DocumentId) {
        self.held.retain(|held| held.document != document);
    }

    /// The posters shown, in the order they were asked for.
    pub fn shown(&self) -> impl Iterator<Item = Poster<'_>> {
        self.held.iter().filter_map(|held| {
            Some(Poster {
                document: held.document,
                annotation: held.annotation,
                raster: held.raster.as_ref()?,
                fit: held.fit,
            })
        })
    }

    /// What was said since the last call.
    pub fn seen(&mut self) -> Vec<Seen> {
        std::mem::take(&mut self.seen)
    }

    /// The bytes the posters shown hold.
    fn weight(&self) -> usize {
        self.held
            .iter()
            .filter_map(|held| held.raster.as_ref())
            .map(|raster| raster.data.len())
            .sum()
    }
}

impl Drop for Posters {
    fn drop(&mut self) {
        if let Some(mut running) = self.running.take() {
            // Closing the orders ends the thread once the poster in hand is decoded; a join that
            // failed is a panic already reported on the thread's own stderr.
            running.orders = None;
            if let Some(thread) = running.thread.take() {
                let _ = thread.join();
            }
        }
    }
}

/// Starts the thread with a media worker of its own, which starts no process until its first
/// movie.
fn start(worker: Option<PathBuf>, wake: Wake) -> Running {
    let (orders, inbox) = std::sync::mpsc::channel::<Order>();
    let (outbox, answers) = std::sync::mpsc::channel::<Answer>();
    let thread = std::thread::Builder::new()
        .name("poster".to_owned())
        .spawn(move || {
            let mut media = worker.map_or_else(MediaWorker::new, MediaWorker::with_program);
            for order in inbox {
                let raster = first_picture(&mut media, &order.movie);
                let answered = outbox.send(Answer {
                    document: order.document,
                    annotation: order.annotation,
                    raster,
                });
                if answered.is_err() {
                    break;
                }
                wake();
            }
        })
        .ok();
    Running {
        orders: Some(orders),
        answers,
        thread,
    }
}

/// The movie's first picture, converted on the processor: the decode is stopped at the first the
/// worker hands on (ADR 3609).
fn first_picture(media: &mut MediaWorker, movie: &[u8]) -> Result<pdf_render::Raster, String> {
    let mut first = None;
    let streamed = media.decode_mp4_video_from(movie, Duration::ZERO, &mut |picture, at| {
        first = Some((picture, at));
        ControlFlow::Break(())
    });
    match streamed {
        Ok(_) => {}
        Err(MediaError::Interlaced(why)) => {
            return Err(format!(
                "its movie is field-coded ({why}), which no decoder this program admits decodes"
            ));
        }
        Err(error) => return Err(format!("its movie's pictures are not decoded: {error}")),
    }
    let (picture, at) =
        first.ok_or_else(|| "its movie's video track presents no picture".to_owned())?;
    let height = u32::try_from(picture.height).unwrap_or(u32::MAX);
    let conversion = Conversion::for_signal(super::signal_of(picture.signal), height)
        .map_err(|refused| refused.to_string())?;
    super::on_the_processor(&Frame {
        picture,
        conversion,
        at,
    })
    .map_err(|refused| refused.to_string())
}
