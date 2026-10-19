//! `CLAUDE.md`'s *ask* level in this window: §12.6.4.8's link and §12.6.4.3's file a document
//! names, put on a card the window draws and answered by two keys (ADR 2040).
//!
//! **The policy is `viewer_host`'s, and nothing here decides it.** [`viewer_host::link`] and
//! [`viewer_host::remote`] take the level and answer with a sentence to say, a file to supply or a
//! question to put, exactly as they answer the three windows that hold the viewer in process; what
//! is this window's is where the question goes — the flagship's [`viewer_ui::chrome::QuestionCard`],
//! over whatever the screen holds — and which key is the answer. Both acts are this side's: the
//! handler a link is given to runs out here, and the file a document named is read out here and
//! crosses to the worker as bytes inside `Command::Supply`, which the confinement then parses as it
//! parses the document it was started with. Nothing new crosses the wire.

use std::path::{Path, PathBuf};

use viewer_core::{Command, Purpose};
use viewer_ui::chrome::Chrome;
use winit::keyboard::{Key, NamedKey};

use crate::Host;

/// What the card is asking about, held until a key answers it.
#[derive(Debug)]
pub(crate) enum Asked {
    /// §12.6.4.8's URI, resolved against the document's location.
    Link {
        /// The URI as `viewer_host::resolve_uri` left it.
        uri: String,
    },
    /// §12.6.4.3's file, or one of the three other purposes `viewer_host::under_remote_documents`
    /// puts under the same level.
    Remote {
        /// Which purpose asked, so that the answer goes back under it.
        purpose: Purpose,
        /// The file as the document named it, for the sentence a decline prints.
        name: String,
        /// Where `viewer_host::resolve_import` put it, which is what would be read.
        path: PathBuf,
    },
}

/// A file a document named, supplied to the worker, until the worker says what became of it.
///
/// `viewer_host::Arrivals`'s offer, for the one document this window holds: an `Event::Opened`
/// makes it the window's document, an `Event::PasswordRequired` makes it the file the prompt is
/// about, and once the supply's own events have been seen an offer neither took is spent — a
/// declined jump, or a named page drawn into the document that asked (ADRs 1332, 2040).
#[derive(Debug)]
pub(crate) struct Offered {
    /// What the file was supplied for, which is what a cancelled prompt about it declines.
    purpose: Purpose,
    /// The file.
    path: PathBuf,
}

impl Host {
    /// §12.6.4.8's URI, under the level this window was started at (ADRs 1155, 2040).
    pub(crate) fn open_uri(&mut self, uri: &str) {
        let uri = viewer_host::resolve_uri(Some(&self.path), uri);
        match viewer_host::link(&uri, self.links) {
            viewer_host::Link::Say(note) => eprintln!("{note}"),
            viewer_host::Link::Ask(words) => self.put_the_question(Asked::Link { uri }, &words),
        }
    }

    /// A file a document named under [`viewer_host::RemoteDocuments`]' level (ADRs 1227, 2040).
    ///
    /// **Asked as a file that opens in place**, whatever Table 203's `/NewWindow` said: this window
    /// holds one document and reserves no name for a second, so the worker's viewer replaces the one
    /// showing and says so itself (ADR 1263), and the question says the same before it happens.
    pub(crate) fn needs_remote(&mut self, purpose: Purpose, name: &str) {
        match viewer_host::remote(
            self.path.parent(),
            name,
            self.remote_documents,
            purpose,
            false,
        ) {
            viewer_host::Remote::Supply { path, note } => {
                self.supply_remote(purpose, name, path, note.as_deref());
            }
            viewer_host::Remote::Ask { path, question } => self.put_the_question(
                Asked::Remote {
                    purpose,
                    name: name.to_owned(),
                    path,
                },
                &question,
            ),
            viewer_host::Remote::Refuse(why) => {
                eprintln!("note: {why}");
                self.dispatch(Command::Supply {
                    purpose,
                    bytes: None,
                });
            }
        }
    }

    /// Puts the question on the card, which then has the keyboard.
    ///
    /// The faces are loaded here the first time a card needs them, as the password card's are, so a
    /// document that asks nothing costs the launch no chrome (principle 2). **One question stands at
    /// a time**: a second that arrives while one is up is declined out loud rather than put over it,
    /// because a person answering a card is answering the words they read.
    fn put_the_question(&mut self, asked: Asked, words: &viewer_host::Question) {
        if self.chrome.is_none() {
            match Chrome::new() {
                Ok(chrome) => self.chrome = Some(chrome),
                Err(said) => {
                    // A build whose compiled-in faces will not parse cannot draw the card;
                    // declining by name is what is left (trap 5).
                    self.decline(
                        asked,
                        &format!("this build cannot draw the question ({said})"),
                    );
                    return;
                }
            }
        }
        if self.asked.is_some() {
            self.decline(asked, "another question is already on the window");
            return;
        }
        self.asked = Some(asked);
        self.question.ask(words);
        self.card_changed();
    }

    /// A question that was never put, said with the reason and answered as a `no`.
    fn decline(&mut self, asked: Asked, why: &str) {
        match asked {
            Asked::Link { uri } => eprintln!("{}", viewer_host::uri_note(&uri, Some(why))),
            Asked::Remote { purpose, name, .. } => {
                eprintln!(
                    "note: {}",
                    viewer_host::remote_note(purpose, &name, Some(why))
                );
                self.dispatch(Command::Supply {
                    purpose,
                    bytes: None,
                });
            }
        }
    }

    /// A key press while the question has the keyboard: Enter goes ahead, Escape does not.
    ///
    /// Every key is taken, for the password card's reason: the card is modal, and Escape is the
    /// question's `no` rather than [`Host::abort`], because what the person is declining is the act
    /// and not the worker. The two keys are named on the card itself (ADR 1145).
    pub(crate) fn question_key(&mut self, key: &Key<&str>) {
        match key {
            Key::Named(NamedKey::Enter) => self.question_answered(true),
            Key::Named(NamedKey::Escape) => self.question_answered(false),
            _ => {}
        }
    }

    /// The card was answered: the act carried out, or the sentence saying it was not.
    fn question_answered(&mut self, proceed: bool) {
        self.question.answered();
        self.card_changed();
        let Some(asked) = self.asked.take() else {
            return;
        };
        match asked {
            Asked::Link { uri } => eprintln!("{}", viewer_host::answered(&uri, proceed)),
            Asked::Remote {
                purpose,
                name,
                path,
            } if proceed => self.supply_remote(purpose, &name, path, None),
            Asked::Remote { purpose, name, .. } => {
                eprintln!("note: {}", viewer_host::remote_declined(purpose, &name));
                self.dispatch(Command::Supply {
                    purpose,
                    bytes: None,
                });
            }
        }
    }

    /// Reads a file a document named and hands it to the worker, or says why it could not.
    ///
    /// Read rather than opened as a descriptor because `Command::Supply` carries bytes; this side
    /// reads them and never parses one, and the worker opens them under its own limits (ADR 1358).
    /// `note` is what [`viewer_host::RemoteDocuments::Warn`] adds, said once the file was read.
    fn supply_remote(&mut self, purpose: Purpose, name: &str, path: PathBuf, note: Option<&str>) {
        let bytes = match std::fs::read(&path) {
            Ok(bytes) => {
                if let Some(note) = note {
                    eprintln!("note: {note}");
                }
                self.offered = Some(Offered { purpose, path });
                Some(bytes)
            }
            Err(error) => {
                eprintln!(
                    "note: {}",
                    viewer_host::remote_note(
                        purpose,
                        name,
                        Some(&format!("cannot read {}: {error}", path.display()))
                    )
                );
                None
            }
        };
        self.dispatch(Command::Supply { purpose, bytes });
    }

    /// `Event::Opened`: where it answers a file this window supplied, that file is the window's
    /// document now — the one its title names, its save is written beside, its next relative link
    /// resolves against and a resume opens again.
    pub(crate) fn opened_an_offer(&mut self) {
        if let Some(offered) = self.offered.take() {
            self.path = offered.path;
        }
    }

    /// `Event::PasswordRequired`: where it is about the file this window just supplied, the prompt
    /// is about that file, and its second attempt opens it.
    pub(crate) fn locked_an_offer(&mut self) {
        if let Some(offered) = self.offered.take() {
            self.prompting = Some(offered);
        }
    }

    /// §7.6.4.1's second attempt at a file a document named goes out: it is offered again, for
    /// the one command that carries it, so its answer is read as the supply's was.
    pub(crate) fn offer_the_prompted(&mut self) {
        if let Some(offered) = self.prompting.take() {
            self.offered = Some(offered);
        }
    }

    /// The command carrying an offer has been answered and its events all seen: an offer nothing
    /// took is spent, so that no later open is mistaken for it.
    pub(crate) fn supplied(&mut self) {
        self.offered = None;
    }

    /// The file §7.6.4.1's card is asking about: one a document named, or the window's own.
    pub(crate) fn prompted_for(&self) -> &Path {
        self.prompting
            .as_ref()
            .map_or(self.path.as_path(), |offered| offered.path.as_path())
    }

    /// The prompt about a file a document named was cancelled or ran out of attempts: the worker
    /// is still holding that file for the password, and `Command::Supply` with no bytes under the
    /// purpose it was supplied for is what ends the hold (ADR 1345).
    pub(crate) fn decline_the_prompted(&mut self) -> bool {
        let Some(offered) = self.prompting.take() else {
            return false;
        };
        self.dispatch(Command::Supply {
            purpose: offered.purpose,
            bytes: None,
        });
        true
    }

    /// A new worker holds the window's document and nothing it was offered.
    pub(crate) fn forget_the_offers(&mut self) {
        self.offered = None;
        self.prompting = None;
    }
}
