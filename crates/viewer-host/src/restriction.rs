//! `CLAUDE.md`'s four levels as a person sets them, and the question the *ask* level asks.
//!
//! # Why this is here and not in a window
//!
//! [`crate::keys`]'s argument for the third time, and this round is the one where it was cheapest
//! to ignore: **what a window is obeying is shared, and what a widget looks like is a toolkit's.**
//! A menu of four levels for each operation a policy holds one for is a few dozen decisions about
//! wording, order, which entry is ticked and what each one sends, and three windows writing them
//! separately is three
//! answers to every one of those — with the third copy, as ever, being where two hosts stop
//! agreeing. Here it is one list of [`Row`]s; a `gtk4::MenuButton`, a `QMenuBar` and a card this
//! program draws for itself are what a host supplies.
//!
//! It is also where the *state* is, and that is not tidiness either. A menu has to show what the
//! levels are now, and [`viewer_core::Command::Restrict`] carries a whole policy while a menu
//! entry sets one operation of it — so something has to hold the other five. [`Restrictions`] is
//! that something, and it holds both scopes for the same reason `viewer-ffi`'s session does
//! (ADR 1144, ADR 1145).
//!
//! # The two scopes, and why a window's policy needed a second one
//!
//! ADR 0604's rule is that a host-supplied value is a statement about the *reader*: a window keeps
//! its policy for its whole life and every document it opens answers to it. That is right, and it
//! is exactly what cannot say *for this document, ask before copying* — a level set to catch one
//! suspicious file catches every file the window opens afterwards, and a reader who then set it
//! back has changed the policy for documents they were not thinking about. So a document may
//! depart from the window's levels, in as many operations as it likes and no more, for as long as
//! it is open (ADR 1145).
//!
//! # What a host still owns
//!
//! Every pixel, and one thing without any: **what a dismissed question means**. [`DO_NOT`] is the
//! answer a window sends when a person closes the prompt without pressing anything, because going
//! ahead on a question nobody answered would be the *off* level under another name — the choice
//! `pdf-transform` makes for a pipe with `Refusal::Unanswered` and the one [`crate::unanswerable`]
//! makes for a face with no dialogue at all.

use pdf_model::restriction::{Level, Operation};
use viewer_core::{
    Command, RestrictionLevel, RestrictionOverride, RestrictionPolicy, RestrictionScope,
};

/// What a person presses to let an operation the document restricts go ahead, this once.
///
/// One word for three windows, so that what a person reads does not depend on which build they
/// picked up — [`crate::password::EXHAUSTED`]'s rule, applied to the other question this program
/// puts.
pub const GO_AHEAD: &str = "Go ahead";

/// What a person presses to leave it undone, and what a window sends for a prompt that was
/// dismissed rather than answered.
pub const DO_NOT: &str = "Do not";

/// The heading a window puts over the levels that apply to every document it opens.
pub const WINDOW: &str = "Restrictions in this window";

/// The heading a window puts over the levels the open document departs to.
pub const DOCUMENT: &str = "Restrictions for this document";

/// What a window says under [`DOCUMENT`]: the departure ends when the document does.
pub const DOCUMENT_NOTE: &str =
    "these last as long as this document is open; the next one opened is back at the window's";

/// The entry that gives one operation back to the window's policy.
///
/// Not a fifth level — `CLAUDE.md` names four — but the absence of one, which is why it appears
/// under [`DOCUMENT`] and nowhere else.
pub const INHERIT: &str = "use the window's level";

/// What a window says beside an operation it has no verb for.
///
/// §7.6.4.2's Table 22 states eight positions and this program's windows perform six of them;
/// assembling is `pdf-transform`'s verb. The level is settable anyway, because a policy with a
/// hole in it would have to grow a message to fill it and because the day a window gains the verb
/// the level is already the reader's (ADR 1144) — and it says so out loud rather than looking like
/// a switch that does nothing.
pub const INERT: &str = "no verb in this window yet";

/// What a window says when a document asks for §12.2's `/HideMenubar` and this menu stays.
///
/// **A documented choice, and `CLAUDE.md` principle 3 is what decides it.** Table 147's entry is
/// "[a] flag specifying whether to hide the interactive PDF processor's menu bar when the document
/// is active", and this program's only menu is the one holding the reader's levels — so obeying
/// the flag over *this* menu would let a file take away the control over what that file is allowed
/// to do. "[A] restriction a reader cannot switch off is a restriction imposed on the reader by
/// somebody else's file, and this program is the reader's." The flag is not ignored in silence,
/// which is trap 5: it is answered with this sentence (ADR 1145).
pub const NOT_THE_DOCUMENTS_TO_HIDE: &str = "this document asks to hide the menu bar (§12.2's /HideMenubar); the only menu here is this \
     reader's own restriction levels, which a document does not get to take away (CLAUDE.md: it \
     shall always be possible to turn them off). /HideToolbar and /HideWindowUI are obeyed";

/// Which of a reader's two policies an entry sets.
///
/// Closed, and **not** `#[non_exhaustive]`, for `doc/ui-boundary.md`'s reason: a host that grew a
/// catch-all arm here is a host where the next scope goes to be ignored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    /// Every document this window has open, and every one it opens afterwards.
    Window,
    /// The document in front of the person, until it closes.
    Document,
}

impl Scope {
    /// Both of them, in the order a menu offers them: the window a person set at launch, then the
    /// document in front of them.
    pub const ALL: [Self; 2] = [Self::Window, Self::Document];

    /// The heading a window puts over this scope's levels.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Window => WINDOW,
            Self::Document => DOCUMENT,
        }
    }

    /// What that scope means, where it is not the obvious one — empty for [`Scope::Window`].
    #[must_use]
    pub const fn note(self) -> &'static str {
        match self {
            Self::Window => "",
            Self::Document => DOCUMENT_NOTE,
        }
    }
}

/// One level a person picked, for one operation, in one scope.
///
/// Copy, and carrying no string, so that a toolkit can hand it to a callback that outlives the
/// menu that produced it — which every one of the three does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Chose {
    /// Which policy it sets.
    pub scope: Scope,
    /// Which operation's level.
    pub operation: Operation,
    /// The level, or `None` for [`INHERIT`] — which is only ever [`Scope::Document`]'s.
    pub level: Option<RestrictionLevel>,
}

/// One level a person can pick, with the words and the tick a toolkit draws it from.
///
/// **What a menu is made of, asked one group at a time** ([`Restrictions::entries`]). GTK and Qt
/// build a submenu per scope and a submenu per operation, so they walk the two lists this crate
/// already enumerates and ask for the entries of each; `viewer-ui` draws one card and takes
/// [`Restrictions::rows`] instead. Neither re-derives the nesting from the other's shape, which is
/// what a single flat list would have made both of them do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Entry {
    /// `pdf_model::restriction::Level::as_str`'s word, or [`INHERIT`].
    pub label: &'static str,
    /// Whether this is the level that stands now — what a toolkit draws a tick or a radio for.
    pub chosen: bool,
    /// What choosing it means.
    pub chose: Chose,
}

/// One line of the menu, at one of its three depths.
///
/// **One flat list rather than a tree**, because the three windows that draw it need different
/// things from a tree and the same thing from a list: GTK and Qt push a submenu when the depth
/// goes up and pop it when it comes down, and the card `viewer-ui` draws is a list already. A
/// tree would have been a third shape nobody asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Row {
    /// A heading naming one of the two scopes.
    Scope {
        /// [`WINDOW`] or [`DOCUMENT`].
        label: &'static str,
        /// What that scope means, where it is not the obvious one. Empty for [`Scope::Window`].
        note: &'static str,
        /// Which scope it heads.
        scope: Scope,
    },
    /// A heading naming one operation, under the scope above it.
    Operation {
        /// The operation's word — `viewer_core::RestrictionPolicy::word`'s, so that a person
        /// reads the same six everywhere this program takes them.
        label: &'static str,
        /// [`INERT`] where this window performs nothing of the kind, empty otherwise.
        note: &'static str,
        /// Which operation it heads.
        operation: Operation,
    },
    /// A level a person can choose, under the operation above it.
    Level(Entry),
}

/// The words of one [`viewer_core::Event::Asking`], from [`asked`].
///
/// Two strings rather than one because the hosts draw them differently and none of them may make
/// that up — [`crate::password::Wording`]'s rule, and this is the same question shape one clause
/// over.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Question {
    /// What the document asserts, as `viewer-core` worded it, ending in *is waiting on your
    /// answer*.
    pub reasons: String,
    /// What this reader set, and what the two answers do about it.
    pub choice: String,
}

/// What a window puts in front of a person when [`viewer_core::Event::Asking`] arrives.
///
/// The notes are the document's own reason and the operation's word is in them already; what this
/// adds is the two things only a host can say — that *ask* is a level this reader chose, and that
/// answering does not change it. A person who wanted the question to stop being asked has
/// [`Restrictions`]'s menu for that, and saying so here is what keeps a prompt from being read as
/// a program that has made up its mind.
#[must_use]
pub fn asked(operation: Operation, notes: &[String]) -> Question {
    Question {
        reasons: notes.join("; "),
        choice: format!(
            "You have set this reader to ask before {operation}. \"{GO_AHEAD}\" does it this once \
             and leaves the level where it is; \"{DO_NOT}\" leaves it undone (CLAUDE.md: a \
             document's restrictions are the reader's to set).",
            operation = operation.as_str()
        ),
    }
}

/// What a window says after a person declined.
///
/// **`viewer-core` says nothing at all on a `no`, deliberately** — a question declined is neither
/// the document doing something nor this program refusing (ADR 0814) — and that leaves the one
/// fact a person is owed unsaid: the thing they asked for did not happen. It is a host's sentence
/// because it is about what the *person* did, and it is said rather than swallowed because a
/// window that goes quiet is trap 5 wearing a dialogue.
#[must_use]
pub fn declined(operation: Operation) -> String {
    format!(
        "{} was not done, because you answered \"{DO_NOT}\"",
        operation.as_str()
    )
}

/// What a window says when a person picks a level out of the menu.
///
/// **Said rather than left to the tick**, for two reasons that are the same reason: a menu closes
/// over the sentence it changed, and the level a person just set is the one thing the window
/// cannot show them afterwards — a policy has no appearance. It also puts the change where the
/// `Xvfb` runs this project checks a window with can read it, which is what every other decision
/// in these hosts is checked by (`crate::policy::refused`'s terminal, one clause over).
#[must_use]
pub fn chosen(chose: Chose) -> String {
    format!(
        "{} is now {} {}",
        RestrictionPolicy::word(chose.operation),
        chose.level.map_or(INHERIT, |level| level.level().as_str()),
        match chose.scope {
            Scope::Window => "in this window",
            Scope::Document => "for this document",
        }
    )
}

/// The two policies a window holds, and the menu that edits them.
///
/// A host builds one from whatever its command line said and keeps it for the window's life;
/// [`Self::rows`] is what its chrome draws and [`Self::chose`] is what a click comes to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Restrictions {
    /// The window's own levels — what `--restrictions=` set and what a document inherits.
    window: RestrictionPolicy,
    /// What the open document departs from them in.
    document: RestrictionOverride,
}

impl Restrictions {
    /// A window at the levels its command line asked for, with no document departing from them.
    #[must_use]
    pub const fn new(window: RestrictionPolicy) -> Self {
        Self {
            window,
            document: RestrictionOverride::NONE,
        }
    }

    /// The window's levels, which is what a host sends before it opens a document.
    #[must_use]
    pub const fn window(self) -> RestrictionPolicy {
        self.window
    }

    /// What the open document departs from them in.
    #[must_use]
    pub const fn document(self) -> RestrictionOverride {
        self.document
    }

    /// Whether the open document is being treated differently from the rest — what a window says
    /// out loud, for [`declined`]'s reason.
    #[must_use]
    pub fn departs(self) -> bool {
        self.document.departs()
    }

    /// A document opened: the departures are gone.
    ///
    /// **The same thing `viewer_core::Open` does, mirrored here**, and a host that did not do it
    /// would show a menu ticking the last document's departures while the viewer answered to
    /// none. [`crate::password::Asking::opened`] is driven from the same event and for a version
    /// of the same reason.
    pub fn opened(&mut self) {
        self.document = RestrictionOverride::NONE;
    }

    /// The menu, whole: two scopes, every operation in each, four levels each and one way back.
    ///
    /// Built on demand rather than held, because it is a function of two values a host already
    /// has and because a window that built it at startup would have built it before anything
    /// could be restricted — `CLAUDE.md` section 2's rule that nothing page one does not need
    /// happens before page one.
    #[must_use]
    pub fn rows(self) -> Vec<Row> {
        let mut rows = Vec::new();
        for scope in Scope::ALL {
            rows.push(Row::Scope {
                label: scope.label(),
                note: scope.note(),
                scope,
            });
            for operation in RestrictionPolicy::OPERATIONS {
                rows.push(Row::Operation {
                    label: RestrictionPolicy::word(operation),
                    note: inert(operation),
                    operation,
                });
                rows.extend(self.entries(scope, operation).into_iter().map(Row::Level));
            }
        }
        rows
    }

    /// The levels one operation offers in one scope, and which of them stands.
    ///
    /// `CLAUDE.md`'s four everywhere, and under [`Scope::Document`] a fifth entry that is not a
    /// level — [`INHERIT`], which takes the departure away again. A window that offered no way
    /// back would have made an override a thing a reader can enter and not leave, which is the
    /// shape `CLAUDE.md`'s own sentence about turning restrictions off forbids.
    #[must_use]
    pub fn entries(self, scope: Scope, operation: Operation) -> Vec<Entry> {
        let stands = self.stands(scope, operation);
        let mut entries: Vec<Entry> = [Level::Off, Level::On, Level::Ask, Level::Warn]
            .into_iter()
            .map(named)
            .map(|level| Entry {
                label: level.level().as_str(),
                chosen: stands == Some(level),
                chose: Chose {
                    scope,
                    operation,
                    level: Some(level),
                },
            })
            .collect();
        if scope == Scope::Document {
            entries.push(Entry {
                label: INHERIT,
                chosen: stands.is_none(),
                chose: Chose {
                    scope,
                    operation,
                    level: None,
                },
            });
        }
        entries
    }

    /// What a person's choice changes, and the command that tells the viewer.
    ///
    /// The whole of a menu's arithmetic, in one place: a level chosen for one operation leaves the
    /// other five where they were, which is why this crate holds them at all
    /// ([`viewer_core::Command::Restrict`] carries a policy and a menu entry sets an operation).
    pub fn chose(&mut self, chose: Chose) -> Command {
        match chose.scope {
            Scope::Window => {
                // A window entry has a level: `INHERIT` is `Scope::Document`'s alone, and a
                // window with nothing to inherit from would be a policy with a hole in it. A
                // `None` here leaves the levels exactly as they were rather than inventing one.
                if let Some(level) = chose.level {
                    self.window = self.window.with(chose.operation, level);
                }
                Command::Restrict(RestrictionScope::Window(self.window))
            }
            Scope::Document => {
                self.document = self.document.with(chose.operation, chose.level);
                Command::Restrict(RestrictionScope::Document(self.document))
            }
        }
    }

    /// The level that stands for one operation in one scope, which is what a tick is drawn from.
    fn stands(self, scope: Scope, operation: Operation) -> Option<RestrictionLevel> {
        match scope {
            Scope::Window => Some(self.window.level(operation)),
            Scope::Document => self.document.level(operation),
        }
    }
}

/// [`INERT`] where no window performs this operation, and nothing where one does.
///
/// Six of §7.6.4.2's eight positions reach a gesture in this program's windows: copying is
/// `Command::Copy`, annotating and filling in are edits, modifying is what §7.11.4's attach and
/// detach are, printing is `Command::Print` behind [`crate::WindowAct::Print`] (ADR 1180), and
/// bit 12's quality is asked of the same press (ADR 1203). Assembling is `pdf-transform`'s verb
/// and no window has one.
///
/// §12.11.6's processing is not one of the table's positions at all and every window performs it,
/// because opening a document is the first thing any of them does (ADR 1167).
#[must_use]
pub const fn inert(operation: Operation) -> &'static str {
    match operation {
        Operation::Extract
        | Operation::Annotate
        | Operation::FillInForm
        | Operation::Modify
        | Operation::Print
        | Operation::PrintFaithfully
        | Operation::Process => "",
        Operation::Assemble => INERT,
    }
}

/// The same level in this crate's vocabulary as `pdf-model`'s.
///
/// `viewer_core::RestrictionLevel::level` is this read the other way round; the four levels
/// themselves live in `pdf_model::restriction::Level`, which is what [`crate::restrictions`]
/// parses a command line into.
const fn named(level: Level) -> RestrictionLevel {
    match level {
        Level::Off => RestrictionLevel::Off,
        Level::On => RestrictionLevel::On,
        Level::Ask => RestrictionLevel::Ask,
        Level::Warn => RestrictionLevel::Warn,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Chose, DO_NOT, GO_AHEAD, INERT, INHERIT, Restrictions, Row, Scope, asked, declined,
    };
    use pdf_model::restriction::Operation;
    use viewer_core::{Command, RestrictionLevel, RestrictionPolicy, RestrictionScope};

    /// The levels a menu offers for one operation in one scope, in the order it offers them.
    fn levels(rows: &[Row], scope: Scope, operation: Operation) -> Vec<(&'static str, bool)> {
        rows.iter()
            .filter_map(|row| match row {
                Row::Level(entry)
                    if entry.chose.scope == scope && entry.chose.operation == operation =>
                {
                    Some((entry.label, entry.chosen))
                }
                _ => None,
            })
            .collect()
    }

    /// `CLAUDE.md`'s four, everywhere, and one way back under the document alone.
    ///
    /// The count is the claim: a scope that offered three would be a level a reader cannot reach,
    /// and an `INHERIT` under the window would be a policy with a hole in it.
    #[test]
    fn four_levels_for_every_operation_and_one_way_back_for_a_document() {
        let rows = Restrictions::new(RestrictionPolicy::default()).rows();
        for operation in RestrictionPolicy::OPERATIONS {
            let window: Vec<&str> = levels(&rows, Scope::Window, operation)
                .into_iter()
                .map(|(label, _)| label)
                .collect();
            assert_eq!(window, ["off", "on", "ask", "warn"], "{operation:?}");
            let document: Vec<&str> = levels(&rows, Scope::Document, operation)
                .into_iter()
                .map(|(label, _)| label)
                .collect();
            assert_eq!(
                document,
                ["off", "on", "ask", "warn", INHERIT],
                "{operation:?}"
            );
        }
    }

    /// The tick follows the policy, in both scopes, and a document with no departure ticks
    /// [`INHERIT`] rather than the window's level.
    ///
    /// The second half is the one that matters: a menu that ticked the *effective* level under
    /// the document would tell a reader they had departed when they had not, and the entry that
    /// undoes a departure would look like the one that is already set.
    #[test]
    fn the_tick_is_on_the_level_that_stands() {
        let mut restrictions = Restrictions::new(RestrictionPolicy::uniform(RestrictionLevel::On));
        let rows = restrictions.rows();
        assert_eq!(
            levels(&rows, Scope::Window, Operation::Extract),
            [
                ("off", false),
                ("on", true),
                ("ask", false),
                ("warn", false)
            ]
        );
        assert_eq!(
            levels(&rows, Scope::Document, Operation::Extract),
            [
                ("off", false),
                ("on", false),
                ("ask", false),
                ("warn", false),
                (INHERIT, true)
            ]
        );
        drop(restrictions.chose(Chose {
            scope: Scope::Document,
            operation: Operation::Extract,
            level: Some(RestrictionLevel::Ask),
        }));
        let rows = restrictions.rows();
        assert_eq!(
            levels(&rows, Scope::Document, Operation::Extract),
            [
                ("off", false),
                ("on", false),
                ("ask", true),
                ("warn", false),
                (INHERIT, false)
            ]
        );
        // And the window's own tick has not moved, which is the whole point of two scopes.
        assert_eq!(
            levels(&rows, Scope::Window, Operation::Extract),
            [
                ("off", false),
                ("on", true),
                ("ask", false),
                ("warn", false)
            ]
        );
    }

    /// One entry sets one operation and says nothing about the other five.
    #[test]
    fn a_level_chosen_for_one_operation_leaves_the_other_five() {
        let mut restrictions = Restrictions::new(RestrictionPolicy::default());
        let command = restrictions.chose(Chose {
            scope: Scope::Window,
            operation: Operation::Annotate,
            level: Some(RestrictionLevel::Warn),
        });
        match command {
            Command::Restrict(RestrictionScope::Window(policy)) => {
                assert_eq!(policy.level(Operation::Annotate), RestrictionLevel::Warn);
                assert_eq!(policy.level(Operation::Extract), RestrictionLevel::Off);
            }
            other => panic!("a window entry sent {other:?}"),
        }
    }

    /// A departure ends with the document it was about.
    #[test]
    fn a_document_opened_departs_from_the_window_in_nothing() {
        let mut restrictions = Restrictions::new(RestrictionPolicy::default());
        drop(restrictions.chose(Chose {
            scope: Scope::Document,
            operation: Operation::Extract,
            level: Some(RestrictionLevel::Ask),
        }));
        assert!(restrictions.departs());
        restrictions.opened();
        assert!(!restrictions.departs());
        assert!(restrictions.document().level(Operation::Extract).is_none());
    }

    /// [`INHERIT`] is the document's alone: a `None` under the window changes no level.
    ///
    /// The menu never builds one — `rows` puts that entry under one scope — and this is the arm
    /// that says what happens if a host composes a [`Chose`] for itself, which the C ABI's caller
    /// effectively does.
    #[test]
    fn a_window_has_nothing_to_inherit_from() {
        let mut restrictions = Restrictions::new(RestrictionPolicy::uniform(RestrictionLevel::On));
        let command = restrictions.chose(Chose {
            scope: Scope::Window,
            operation: Operation::Extract,
            level: None,
        });
        match command {
            Command::Restrict(RestrictionScope::Window(policy)) => {
                assert_eq!(policy.level(Operation::Extract), RestrictionLevel::On);
            }
            other => panic!("a window entry sent {other:?}"),
        }
    }

    /// The two operations no window performs say so, and the other four do not.
    #[test]
    fn an_operation_with_no_verb_in_this_window_says_so() {
        let rows = Restrictions::new(RestrictionPolicy::default()).rows();
        let inert: Vec<Operation> = rows
            .iter()
            .filter_map(|row| match row {
                Row::Operation {
                    note, operation, ..
                } if *note == INERT => Some(*operation),
                _ => None,
            })
            .collect();
        // Twice over, because the list has both scopes in it.
        assert_eq!(inert, [Operation::Assemble, Operation::Assemble]);
    }

    /// §12.2's `/HideMenubar` is answered in words, and the words name the clause and the rule.
    ///
    /// The sentence is the whole of a documented departure from a Table 147 entry, so what this
    /// holds is that it stays legible as one: a reader who finds a menu bar a document asked to
    /// hide is owed the clause number and the reason, and a note that said only "ignored" would be
    /// this program overriding a file without saying why (ADR 1145).
    #[test]
    fn the_menu_a_document_may_not_hide_says_which_clause_and_why() {
        let said = super::NOT_THE_DOCUMENTS_TO_HIDE;
        assert!(said.contains("12.2"), "the clause: {said}");
        assert!(said.contains("/HideMenubar"), "the entry: {said}");
        assert!(
            said.contains("turn them off"),
            "`CLAUDE.md`'s rule, which is what makes it a choice rather than a refusal: {said}"
        );
        assert!(
            said.contains("/HideToolbar") && said.contains("/HideWindowUI"),
            "and the two entries that are obeyed: {said}"
        );
    }

    /// The question names the operation and both answers, and says the level is unchanged.
    #[test]
    fn the_question_says_what_each_answer_does() {
        let question = asked(
            Operation::Extract,
            &["this document withholds copying".to_owned()],
        );
        assert_eq!(question.reasons, "this document withholds copying");
        assert!(question.choice.contains(GO_AHEAD));
        assert!(question.choice.contains(DO_NOT));
        assert!(question.choice.contains("extracting from the document"));
        assert!(question.choice.contains("leaves the level where it is"));
        assert!(declined(Operation::Extract).contains("was not done"));
    }
}
