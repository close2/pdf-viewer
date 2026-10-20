//! §12.7.6.2's verb in this window: the request the worker composed, sent from this side under the
//! level `--submissions=` set, and §12.7.8's answer handed back to the worker (ADR 2076).
//!
//! **The policy and the transmission are `viewer_host`'s, and nothing here decides either.**
//! [`viewer_host::may_submit`] and [`viewer_host::may_fetch_import`] take the level and answer as
//! they answer the three windows that hold the viewer in process, and
//! [`viewer_host::submit::Submitter`] sends on a thread of its own through
//! `viewer_host::submit::transmit` — the one function every window sends through. What is this
//! window's is where each half runs: **the request is composed inside the confinement**, which
//! holds the form, and **sent from out here**, which holds the network the worker never acquires
//! (`CLAUDE.md` principle 3); and the answer goes back as `Command::Respond`, so the form data a
//! server wrote is parsed by the worker under its limits, exactly as a document named on a disk
//! is, and never by this process.
//!
//! **This window looks for an answer rather than being woken by one**: it has no event-loop proxy,
//! so the loop waits at most [`viewer_host::submit::LOOK`] while a request is out — the toolkits'
//! arrangement, which `Submitter::interval` is written for — and not at all when none is.

use pdf_model::submission::Submission;
use viewer_core::{Command, DocumentId, Purpose};

use crate::Host;
use crate::asking::Asked;

impl Host {
    /// §12.7.6.2's composed request, under the level this window was started at (ADRs 1291, 2076).
    pub(crate) fn submit(&mut self, document: DocumentId, submission: Submission) {
        match viewer_host::may_submit(&submission, self.submissions) {
            viewer_host::Sending::Send => self.send_form(document, submission, None),
            viewer_host::Sending::Warn(note) => self.send_form(document, submission, Some(note)),
            viewer_host::Sending::Ask(words) => self.put_the_question(
                Asked::Submit {
                    document,
                    submission: Box::new(submission),
                },
                &words,
            ),
            viewer_host::Sending::Refuse(why) => eprintln!(
                "{}",
                viewer_host::policy::submission_note(&submission, Some(&why))
            ),
        }
    }

    /// Table Annex O.4's `fdf`, or §12.7.6.4's file, naming an absolute URI: fetched under the
    /// level a submission is sent at — the one answer `viewer_host::may_fetch_import` gives (ADRs
    /// 1527, 2076). A refusal is said and supplied as nothing, so the worker says the import
    /// declined; a fetch arrives as `Command::Respond`.
    pub(crate) fn fetch_import(&mut self, document: DocumentId, url: String) {
        match viewer_host::may_fetch_import(&url, self.submissions) {
            viewer_host::Sending::Send => self.start_fetch(document, url, None),
            viewer_host::Sending::Warn(note) => self.start_fetch(document, url, Some(note)),
            viewer_host::Sending::Ask(words) => {
                self.put_the_question(Asked::Fetch { document, url }, &words);
            }
            viewer_host::Sending::Refuse(why) => {
                eprintln!("{}", viewer_host::fetch_note(&url, Some(&why)));
                self.dispatch(Command::Supply {
                    purpose: Purpose::ImportData,
                    bytes: None,
                });
            }
        }
    }

    /// Puts the request on a `submit-form` thread of its own, and says where it went.
    pub(crate) fn send_form(
        &mut self,
        document: DocumentId,
        submission: Submission,
        warned: Option<String>,
    ) {
        eprintln!(
            "{}",
            viewer_host::policy::submission_note(&submission, None)
        );
        if let Err(sentence) = self.submitter.send(document, submission, warned, None) {
            eprintln!("note: {sentence}");
        }
    }

    /// Puts an import's GET on a `fetch-import` thread of its own, and says where it went.
    pub(crate) fn start_fetch(
        &mut self,
        document: DocumentId,
        url: String,
        warned: Option<String>,
    ) {
        eprintln!("{}", viewer_host::fetch_note(&url, None));
        if let Err(sentence) = self.submitter.fetch(document, url, warned, None) {
            eprintln!("note: {sentence}");
        }
    }

    /// Every server's answer that has arrived: handed to the worker, kept, or said (ADR 2076).
    ///
    /// **A PDF answer is kept and not opened**, where the other three windows open it beside the
    /// form: this window holds one document, and opening the answer in its place would put away
    /// the form the person just filled in, unsaved, for a document they did not ask for. The file
    /// is kept where `viewer_host::submit::keep_answer` keeps it for every window, and the sentence
    /// says where, so the person can open it themselves.
    pub(crate) fn take_the_answers(&mut self) {
        for returned in self.submitter.collect() {
            let (document, url) = (returned.document, returned.url.clone());
            match returned.reply() {
                viewer_host::submit::Reply::Import {
                    answers,
                    format,
                    bytes,
                    note,
                } => {
                    eprintln!("{note}");
                    self.dispatch(Command::Respond {
                        document,
                        answers,
                        source: url,
                        format,
                        bytes,
                    });
                }
                viewer_host::submit::Reply::Document { bytes, note } => {
                    match viewer_host::submit::keep_answer(&bytes) {
                        Ok(path) => eprintln!(
                            "{note}; kept at {}, and not opened: this window holds one document \
                             and opens none beside it",
                            path.display()
                        ),
                        Err(sentence) => eprintln!("{note}; {sentence}"),
                    }
                }
                viewer_host::submit::Reply::Say(note) => eprintln!("{note}"),
            }
        }
    }
}
