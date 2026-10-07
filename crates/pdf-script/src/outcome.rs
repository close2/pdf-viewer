//! What a run hands back: the event as the script left it, how the run ended, what was refused,
//! and what the script logged.

use std::time::Duration;

/// What one run of a field script did.
///
/// RFC 0008 section 6.1's `Outcome`, narrowed to what one field's `/K` and `/F` can do: a value, a
/// change, and `rc`. A run that did not finish changes nothing — `rc` true, no value, no change —
/// and [`Self::ending`] says why (ADR 1591).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    /// `event.rc` as the script left it: false refuses the keystroke or the commit.
    pub rc: bool,
    /// `event.value` where the script changed it.
    pub value: Option<String>,
    /// `event.change` where the script changed it.
    pub change: Option<String>,
    /// How the run ended.
    pub ending: Ending,
    /// Every call the script was refused, by name, in the order first met — recorded whether or
    /// not the script caught the throw, because a refusal a script swallowed is still one the
    /// reader is owed (RFC 0008 section 4.3).
    pub refusals: Vec<Refusal>,
    /// What the script wrote with `console.println`, a line each.
    pub log: Vec<String>,
}

impl Outcome {
    /// An outcome that changed nothing and ended as `ending`.
    #[must_use]
    pub fn unchanged(ending: Ending) -> Self {
        Self {
            rc: true,
            value: None,
            change: None,
            ending,
            refusals: Vec::new(),
            log: Vec::new(),
        }
    }

    /// Every sentence the outcome owes a report, in order: each refusal, then how the run ended
    /// where it did not finish, then each logged line.
    #[must_use]
    pub fn sentences(&self) -> Vec<String> {
        let mut sentences: Vec<String> = self.refusals.iter().map(Refusal::sentence).collect();
        if let Some(sentence) = self.ending.sentence() {
            sentences.push(sentence);
        }
        sentences.extend(
            self.log
                .iter()
                .map(|line| format!("the script logged: {line}")),
        );
        sentences
    }
}

/// How a run ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ending {
    /// The script ran to its end, and the event is as it left it.
    Finished,
    /// A budget stopped the script.
    Exceeded(Exceeded),
    /// The script threw, and nothing caught it: the thrown value's text.
    Threw(String),
    /// The text is not a script the engine parses: the parser's sentence.
    Unparsed(String),
    /// The request asked for something this bridge does not run: the sentence saying what.
    Declined(String),
}

impl Ending {
    /// The sentence a report carries for an ending other than [`Ending::Finished`].
    #[must_use]
    pub fn sentence(&self) -> Option<String> {
        match self {
            Self::Finished => None,
            Self::Exceeded(exceeded) => {
                Some(format!("the script was stopped: {}", exceeded.sentence()))
            }
            Self::Threw(thrown) => Some(format!("the script threw and was stopped: {thrown}")),
            Self::Unparsed(why) => Some(format!("the script does not parse: {why}")),
            Self::Declined(why) => Some(why.clone()),
        }
    }
}

/// Which budget a run exceeded (ADR 1590).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Exceeded {
    /// The wall-time budget, checked between execution slices.
    Wall(Duration),
    /// The engine's step budget, in its own cost units.
    Steps(u64),
    /// The engine's loop-iteration limit, per call frame.
    LoopIterations(u64),
    /// The engine's recursion limit.
    Recursion(u32),
    /// The engine's value-stack limit.
    Stack(u32),
    /// A built-in was asked to iterate or allocate an array of `asked` elements.
    Elements {
        /// How many the script asked for.
        asked: u64,
        /// The ceiling.
        ceiling: u64,
    },
    /// A built-in was asked to build a string of `asked` UTF-16 code units.
    StringUnits {
        /// How many the script asked for.
        asked: u64,
        /// The ceiling.
        ceiling: u64,
    },
}

impl Exceeded {
    /// The sentence naming the budget and its number.
    #[must_use]
    pub fn sentence(&self) -> String {
        match self {
            Self::Wall(limit) => format!(
                "it ran longer than its wall-time budget of {} ms",
                limit.as_millis()
            ),
            Self::Steps(limit) => {
                format!("it spent more than its step budget of {limit} engine steps")
            }
            Self::LoopIterations(limit) => {
                format!("a loop ran more than its budget of {limit} iterations in one call frame")
            }
            Self::Recursion(limit) => {
                format!("it recursed deeper than its budget of {limit} calls")
            }
            Self::Stack(limit) => {
                format!("it used more than its stack budget of {limit} values")
            }
            Self::Elements { asked, ceiling } => format!(
                "it asked a built-in for an array of {asked} elements, over its memory budget of \
                 {ceiling} elements"
            ),
            Self::StringUnits { asked, ceiling } => format!(
                "it asked a built-in for a string of {asked} code units, over its memory budget \
                 of {ceiling}"
            ),
        }
    }
}

/// One call a script was refused, by name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refusal {
    /// The call or property, as the script spells it: `app.launchURL`, `event.commitKey`,
    /// `this.getField("Total")`.
    pub member: String,
    /// Why it was refused.
    pub kind: RefusalKind,
}

impl Refusal {
    /// The sentence the script's `NotAllowedError` carries, and the report.
    #[must_use]
    pub fn sentence(&self) -> String {
        let member = &self.member;
        match &self.kind {
            RefusalKind::Excluded(reason) => format!(
                "{member} is not allowed: Tier 2, excluded by RFC 0008 section 4.3 because it \
                 {reason}"
            ),
            RefusalKind::NotBridged => format!(
                "{member} is not allowed: Tier 1 admits it and this bridge does not carry it \
                 (ADR 1591)"
            ),
            RefusalKind::Unreachable(why) => format!("{member} is not allowed: {why}"),
            RefusalKind::Library(why) => format!("{member} did not run: {why}"),
        }
    }
}

/// Why a call was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefusalKind {
    /// RFC 0008 section 4.3's Tier 2, with that table's reason.
    Excluded(String),
    /// RFC 0008 section 4.2 admits it to Tier 1, and this bridge does not carry it yet.
    NotBridged,
    /// It reaches something this bridge does not hold — another field, a write to a value the
    /// event owns — with the sentence saying what.
    Unreachable(String),
    /// An `AF*` function refused its arguments, with `pdf_model::aform`'s own sentence.
    Library(String),
}
