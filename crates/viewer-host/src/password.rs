//! §7.6.4.1's prompt, as far as it is not a toolkit's.
//!
//! # What the clause says, which is less than three hosts were quoting
//!
//! ISO 32000-2 §7.6.4.1, on a document that did not authenticate with the default user password:
//!
//! > If this authentication attempt fails, the interactive PDF processor should prompt for a
//! > password. Correctly supplying either password ( owner or user password) should enable the
//! > user to gain access to the document.
//!
//! **`should`, and `interactive`.** Both native hosts carried the comment *"§7.6.4.1: 'the
//! interactive PDF processor shall … prompt the user for a password'"* — a sentence with quotation
//! marks round it that the standard does not contain, upgrading a recommendation to a requirement
//! and adding four words. Corrected in the six-hundred-and-ninety-fifth session. The clause states
//! no number of attempts, states nothing about what a prompt looks like, and — see [`ATTEMPTS`] —
//! leaves both here as documented choices.
//!
//! The standard also says what a processor with nobody to ask is, and it is not a processor that
//! gives up. §7.6.4.1's NOTE 2:
//!
//! > This enables limited access to a document when a user is not be able to respond to a prompt
//! > for a password. For example, there can be non-interactive PDF readers that do not have a
//! > person running them such as printing off-line or on a server.
//!
//! A window on a screen is an *interactive* processor whatever it was launched from, so "there is
//! no terminal" is not the non-interactive case the NOTE describes — it is a processor that has a
//! person and looked for them in the wrong place. `viewer-ui` read `stdin` and called
//! `std::process::exit(1)` when there was none, so a desktop launcher could not open an encrypted
//! document at all; that is what [`crate::password`] and the card in `viewer_ui::chrome` replace.
//!
//! # Why the policy is here and the prompt is not
//!
//! [`crate::keys`]'s argument verbatim: **what a window is obeying is shared, and what a widget
//! looks like is a toolkit's.** Three hosts each held their own `PASSWORD_ATTEMPTS`, their own
//! `attempts` counter, their own `saturating_add` and their own comparison, and the third copy is
//! where two hosts stop agreeing — `viewer-ui`'s counted to the same three and then *exited*,
//! while the two native hosts said a sentence and left the window up. [`Asking`] is the one
//! counter now, [`Ask`] is the closed set of things it can say, and what a host supplies is the
//! `gtk4::PasswordEntry`, the `QLineEdit` with `QLineEdit::Password`, or the card this program
//! draws for itself.
//!
//! # What a host still owns
//!
//! Everything with a pixel in it, and one thing without: **when to stop**. [`Ask::Exhausted`] says
//! the attempts are used up and says [`EXHAUSTED`] to be shown; it does not say to close the
//! window, and no host may make it mean that. A reader who mistyped three times still has an open
//! program, a title bar and a way to try another file — which is the whole difference between this
//! and what `viewer-ui` did.

use viewer_core::Secret;

/// How many times §7.6.4.1's password is asked for before a host stops asking.
///
/// **A documented choice, and the clause states no number at all** — it says an interactive
/// processor *should* prompt and stops there. Three is what a login prompt asks, and the argument
/// for a limit rather than an unbounded loop is that a prompt a person cannot leave is a prompt
/// that has taken the window: [`Ask::Exhausted`] gives it back.
///
/// It was three in all three hosts before this module existed, so nothing about what a person sees
/// changed when the constant moved — which is the point of moving it rather than choosing again.
pub const ATTEMPTS: u32 = 3;

/// What a host says when [`ATTEMPTS`] are used up.
///
/// One sentence for three hosts, so that the thing a person reads does not depend on which build
/// they picked up. It says what happened and not what to do next, because what to do next is the
/// window's — and every one of these windows is still open when it is shown.
pub const EXHAUSTED: &str = "too many password attempts";

/// What a host says when a person dismisses the prompt without typing anything.
///
/// A separate sentence from [`EXHAUSTED`] because they are different facts about the reader:
/// one tried and failed, the other declined. Trap 5 in a window — a document that is not on the
/// screen has to say why it is not.
pub const CANCELLED: &str = "the document is encrypted and no password was given";

/// What a host puts above its entry box, in the two parts every one of them draws separately.
///
/// **One format string for three hosts.** Each of them built this sentence for itself before the
/// six-hundred-and-ninety-fifth session — `viewer-gtk` from the file name, `viewer-qt` from a
/// `QStringLiteral` that named no file at all, `viewer-ui` from a `eprint!` — so a person could
/// tell which build they had picked up by reading the question. The clause number is in it
/// deliberately: this program says which sentence it is obeying wherever it has room to.
///
/// Two strings rather than one because the hosts draw them differently and none of them may make
/// that up: a `gtk4::Label` apiece, one `QLabel` with both, and two lines of the card in two
/// colours.
#[must_use]
pub fn prompt(name: &str, attempt: u32, of: u32) -> Wording {
    Wording {
        question: format!("{name} is encrypted (ISO 32000-2 §7.6.4.1) and needs a password."),
        counted: format!("Attempt {attempt} of {of}."),
    }
}

/// The words of one prompt, from [`prompt`].
///
/// Named for the words rather than for the event, because [`Ask::Prompt`] is the event and the
/// two are not the same thing: one says *ask now, for attempt two of three*, and this is what to
/// put on the screen when it does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Wording {
    /// What is being asked, and why.
    pub question: String,
    /// Which attempt this is, of how many.
    pub counted: String,
}

/// What [`Asking::required`] tells a host to do about [`viewer_core::Event::PasswordRequired`].
///
/// A closed set, and **not** `#[non_exhaustive]`, for `doc/ui-boundary.md`'s reason: a host that
/// grew a catch-all arm here is a host where the next case goes to be ignored. Each of the three
/// windowed hosts matches this exhaustively, and [`Ask::ALL`] is what their tests walk — the shape
/// ADR 0526 established for [`crate::keys::Key`], applied to the other thing all three hosts do
/// with a clause that says only *should*.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ask {
    /// Put the prompt in front of the person, for the attempt this is.
    ///
    /// The numbers are carried so that a host can say *2 of 3* if its toolkit has room for it, and
    /// so that the count is the shared one rather than a host's own arithmetic.
    Prompt {
        /// Which attempt this is, counting from one.
        attempt: u32,
        /// How many there are, which is [`ATTEMPTS`].
        of: u32,
    },
    /// Stop asking, say [`EXHAUSTED`], and **leave the window open**.
    Exhausted,
}

impl Ask {
    /// Every case a host is held to answering.
    ///
    /// Checked against the enumeration by `every_case_is_in_the_list_a_host_is_held_to`, which is
    /// the one thing a hand-written array of variants can get wrong.
    pub const ALL: &'static [Self] = &[
        Self::Prompt {
            attempt: 1,
            of: ATTEMPTS,
        },
        Self::Exhausted,
    ];
}

/// §7.6.4.1's attempts, counted once for every host that has a window.
///
/// Held by a host beside the document it is trying to open. [`Self::required`] is driven from
/// [`viewer_core::Event::PasswordRequired`] and [`Self::opened`] from
/// [`viewer_core::Event::Opened`], and those two calls are the whole of the protocol.
#[derive(Debug, Default)]
pub struct Asking {
    /// How many prompts have been put in front of the person for the document being opened.
    attempts: u32,
}

impl Asking {
    /// A document nobody has been asked about yet.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// [`viewer_core::Event::PasswordRequired`] arrived: ask, or stop.
    ///
    /// Counts the attempt *before* answering, so the first prompt is attempt one and the
    /// [`ATTEMPTS`]-th failure is the last thing a person sees rather than the last thing they are
    /// asked. Saturating, because a host that somehow drove this past four billion documents
    /// should stop asking rather than start again.
    pub fn required(&mut self) -> Ask {
        self.attempts = self.attempts.saturating_add(1);
        if self.attempts > ATTEMPTS {
            Ask::Exhausted
        } else {
            Ask::Prompt {
                attempt: self.attempts,
                of: ATTEMPTS,
            }
        }
    }

    /// The document opened, so the next one starts from nothing.
    ///
    /// **`viewer-gtk` and `viewer-qt` never did this** and it did not show, because each opens one
    /// document and never another. `viewer-ui` did, and it is the correct behaviour for all three:
    /// §7.6.4.1's count is about *a* document, and Annex O's `ef` already gives this program a
    /// second one to open without restarting.
    pub fn opened(&mut self) {
        self.attempts = 0;
    }

    /// How many prompts this document has had, which is what a status line can say.
    #[must_use]
    pub fn attempts(&self) -> u32 {
        self.attempts
    }
}

/// What a person did with the prompt, turned into what a host sends next.
///
/// A host calls this with what its own widget produced — `gtk4::PasswordEntry::text`, a
/// `QLineEdit`'s, or the card's buffer — and gets back either a password to open with or the
/// sentence to say. **An empty password is a cancellation** and is deliberately not sent: the
/// empty string is §7.6.4.1's *default user password*, which the reader has already tried by the
/// time this event exists, so sending it would spend an attempt on the answer that already failed.
#[must_use]
pub fn supplied(typed: Secret) -> Supplied {
    if typed.is_empty() {
        Supplied::Cancelled
    } else {
        Supplied::Open(typed)
    }
}

/// The two things a host does with what was typed.
///
/// Closed, exhaustively matched in three hosts, and in [`Ask::ALL`]'s spirit: the enumeration is
/// what makes "all three hosts stay level" a compile error rather than a habit.
#[derive(Debug)]
pub enum Supplied {
    /// Open the document again with this, which is [`viewer_core::Command::Open`].
    Open(Secret),
    /// The person declined. Say [`CANCELLED`] and leave the window open.
    Cancelled,
}

#[cfg(test)]
mod tests {
    use super::{ATTEMPTS, Ask, Asking, Supplied, supplied};
    use viewer_core::Secret;

    /// [`Ask::ALL`] is the list three hosts are held to, so it has to be the whole enumeration.
    ///
    /// The match is what makes this a check rather than a second hand-written list: a variant
    /// added above and forgotten here fails to compile.
    #[test]
    fn every_case_is_in_the_list_a_host_is_held_to() {
        let mut prompts = 0;
        let mut exhausted = 0;
        for case in Ask::ALL {
            // Exhaustive on purpose.
            match case {
                Ask::Prompt { .. } => prompts += 1,
                Ask::Exhausted => exhausted += 1,
            }
        }
        assert_eq!((prompts, exhausted), (1, 1), "the list is the enumeration");
    }

    /// The sequence every host now shares: three prompts, then a sentence and an open window.
    #[test]
    fn three_prompts_and_then_it_stops_asking() {
        let mut asking = Asking::new();
        for attempt in 1..=ATTEMPTS {
            assert_eq!(
                asking.required(),
                Ask::Prompt {
                    attempt,
                    of: ATTEMPTS
                }
            );
        }
        assert_eq!(asking.required(), Ask::Exhausted);
        // And it stays stopped, rather than coming round again on the next event.
        assert_eq!(asking.required(), Ask::Exhausted);
        assert_eq!(asking.attempts(), ATTEMPTS + 2);
    }

    /// A second document starts from nothing, which is what Annex O's `ef` makes reachable.
    #[test]
    fn opening_a_document_forgets_what_the_last_one_cost() {
        let mut asking = Asking::new();
        assert!(matches!(asking.required(), Ask::Prompt { attempt: 1, .. }));
        assert!(matches!(asking.required(), Ask::Prompt { attempt: 2, .. }));
        asking.opened();
        assert!(matches!(asking.required(), Ask::Prompt { attempt: 1, .. }));
    }

    /// One question for three hosts, and it says which clause it is obeying.
    #[test]
    fn the_prompt_is_one_sentence_and_names_its_clause() {
        let words = super::prompt("locked.pdf", 2, ATTEMPTS);
        assert!(words.question.contains("locked.pdf"));
        assert!(words.question.contains("7.6.4.1"));
        assert_eq!(words.counted, format!("Attempt 2 of {ATTEMPTS}."));
    }

    /// §7.6.4.1's default user password has already been tried, so an empty entry is a decline.
    #[test]
    fn an_empty_entry_is_a_cancellation_and_not_the_default_user_password() {
        assert!(matches!(supplied(Secret::new()), Supplied::Cancelled));
        let mut typed = Secret::new();
        typed.push_str("open sesame");
        match supplied(typed) {
            Supplied::Open(secret) => assert_eq!(secret.reveal(), "open sesame"),
            Supplied::Cancelled => panic!("a password that was typed was read as a decline"),
        }
    }
}
