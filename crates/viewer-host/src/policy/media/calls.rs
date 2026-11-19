//! The portal calls a window has out, and the failure of each brought back to the window to be
//! said there (ADR 3219).
//!
//! [`super::Portal`] waits for `gdbus` on a thread of its own, so a portal that answers with an
//! error does so after [`super::hand_media`] has returned and its sentence has been said. The
//! thread hands what became of the call to a [`Told`]; the window holds the [`PortalCalls`] it came
//! from and takes the failures in with its other threads' answers — [`crate::submit::Submitter`]'s
//! shape, a channel and a count its timer reads through [`PortalCalls::interval`].

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::Duration;

/// Where a portal call's thread says what became of it: `None` where the portal took the call,
/// the sentence where it did not.
#[derive(Debug, Clone)]
pub struct Told {
    sender: Sender<Option<String>>,
    out: Arc<AtomicUsize>,
}

impl Told {
    /// A call is about to be waited for: counted before its thread starts, so that the count is
    /// never behind the answers.
    pub(super) fn waiting(&self) {
        self.out.fetch_add(1, Ordering::SeqCst);
    }

    /// The thread [`Self::waiting`] counted could not be started, so no answer will come.
    pub(super) fn not_waiting(&self) {
        self.out.fetch_sub(1, Ordering::SeqCst);
    }

    /// What became of the call. A window that has closed has no receiver, and an answer nobody can
    /// read is one nobody is owed, as [`crate::submit::Submitter`]'s.
    pub(super) fn finished(&self, failure: Option<String>) {
        let _ = self.sender.send(failure);
    }
}

/// The portal calls one window has out (ADR 3219).
#[derive(Debug)]
pub struct PortalCalls {
    told: Told,
    receiver: Receiver<Option<String>>,
}

impl Default for PortalCalls {
    fn default() -> Self {
        Self::new()
    }
}

impl PortalCalls {
    /// A window with no call out.
    #[must_use]
    pub fn new() -> Self {
        let (sender, receiver) = channel();
        Self {
            told: Told {
                sender,
                out: Arc::new(AtomicUsize::new(0)),
            },
            receiver,
        }
    }

    /// The desktop portal, its calls' failures brought back here.
    #[must_use]
    pub fn portal(&self) -> super::Portal {
        super::Portal {
            told: Some(self.told.clone()),
            ..super::Portal::default()
        }
    }

    /// [`Self::portal`], called through `program` in place of `gdbus` — a test's double.
    #[must_use]
    pub fn portal_through(&self, program: std::path::PathBuf) -> super::Portal {
        super::Portal {
            program,
            told: Some(self.told.clone()),
        }
    }

    /// The sentence of every call that has failed since the last look, each call that has answered
    /// no longer counted out.
    pub fn collect(&mut self) -> Vec<String> {
        let mut failed = Vec::new();
        while let Ok(answer) = self.receiver.try_recv() {
            self.told.out.fetch_sub(1, Ordering::SeqCst);
            failed.extend(answer);
        }
        failed
    }

    /// How long to wait before looking again, or `None` where no call is out — which stops a
    /// window's timer, as [`crate::submit::Submitter::interval`]'s does.
    #[must_use]
    pub fn interval(&self) -> Option<Duration> {
        (self.told.out.load(Ordering::SeqCst) > 0).then_some(crate::submit::LOOK)
    }
}
