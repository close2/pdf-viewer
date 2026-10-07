//! What a document's script asks the person reading it, and what they answered: Adobe's
//! `app.alert` and `app.response`, carried as an event out and a command back (ADR 1628).
//!
//! RFC 0008 section 4.2 puts the two among Tier 1's calls and says how they reach a person: they
//! are the two that need a host, and they get one as every other question here does — an event a
//! window with a dialogue answers and a window without one refuses by name. So this module is data, like every
//! other question this crate puts: [`ScriptQuestion`] goes out in
//! [`crate::Event::ScriptAsking`], and [`ScriptAnswer`] comes back in
//! [`crate::Command::AnswerScript`]. What a dialogue *looks* like is each window's, and which
//! buttons it carries is not: [`AlertButtons::buttons`] is the one list all three draw from.
//!
//! **The arguments are Adobe's, read from the *JavaScript for Acrobat API Reference*** — the
//! working source the owner named for the API (RFC 0008, `doc/questions/A193`), cited and never
//! quoted. ISO 32000-2 names JavaScript and states none of its objects. `app.alert` takes a message,
//! an icon numbered 0 to 3 (error, warning, question, status) and a button set numbered 0 to 3 (OK;
//! OK and Cancel; Yes and No; Yes, No and Cancel), and returns the button pressed as 1 to 4 (OK,
//! Cancel, No, Yes); `app.response` takes a question, a title, a default answer, whether the answer
//! is a password and a label for the entry, and returns the text, or `null` where the person
//! cancelled. The numbers are the script's, and stay on the script's side of the wire: what
//! crosses here is what they mean.

/// What a script asks, as a window puts it to a person.
///
/// Closed, and **not** `#[non_exhaustive]`, for `doc/ui-boundary.md`'s reason: a third kind of
/// question added here must fail to compile in every window that answers the first two.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScriptQuestion {
    /// `app.alert`: a message and the buttons it offers.
    Alert {
        /// The script's message, as text.
        message: String,
        /// Which of Adobe's four icons the script named.
        icon: AlertIcon,
        /// Which of Adobe's four button sets the script asked for.
        buttons: AlertButtons,
        /// The script's own title for the dialogue, where it gave one.
        title: Option<String>,
    },
    /// `app.response`: a question answered by typing.
    Response {
        /// The script's question, as text.
        question: String,
        /// The script's own title for the dialogue, where it gave one.
        title: Option<String>,
        /// The text the entry starts with; empty where the script gave none.
        default: String,
        /// The label the script put beside the entry, where it gave one.
        label: Option<String>,
        /// Whether the entry is a password's: shown as bullets, and never said back.
        password: bool,
    },
}

/// `app.alert`'s icon: Adobe's numbers 0 to 3, error first and the default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AlertIcon {
    /// 0, and the default.
    #[default]
    Error,
    /// 1.
    Warning,
    /// 2.
    Question,
    /// 3.
    Status,
}

impl AlertIcon {
    /// The word a window shows beside the message where it draws no picture.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::Error => "Error",
            Self::Warning => "Warning",
            Self::Question => "Question",
            Self::Status => "Status",
        }
    }
}

/// `app.alert`'s button set: Adobe's numbers 0 to 3, a lone OK first and the default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AlertButtons {
    /// 0: OK, and the default.
    #[default]
    Ok,
    /// 1: OK and Cancel.
    OkCancel,
    /// 2: Yes and No.
    YesNo,
    /// 3: Yes, No and Cancel.
    YesNoCancel,
}

impl AlertButtons {
    /// The buttons a dialogue carries, in the order a window lays them out — the affirming one
    /// last, where every toolkit this tree binds puts its default.
    #[must_use]
    pub const fn buttons(self) -> &'static [AlertButton] {
        match self {
            Self::Ok => &[AlertButton::Ok],
            Self::OkCancel => &[AlertButton::Cancel, AlertButton::Ok],
            Self::YesNo => &[AlertButton::No, AlertButton::Yes],
            Self::YesNoCancel => &[AlertButton::Cancel, AlertButton::No, AlertButton::Yes],
        }
    }

    /// The button a dialogue closed without a press answers with: Cancel where it is offered, No
    /// where the choice is Yes or No, and OK where OK is all there is.
    ///
    /// **A choice, written down** (ADR 1628): Adobe's reference states what each button returns
    /// and nothing about a dialogue closed some other way. Closing is the person declining to
    /// choose, so it is the least committal button the set has — never Yes, which would have the
    /// document act on a word nobody said.
    #[must_use]
    pub const fn dismissed(self) -> AlertButton {
        match self {
            Self::Ok => AlertButton::Ok,
            Self::OkCancel | Self::YesNoCancel => AlertButton::Cancel,
            Self::YesNo => AlertButton::No,
        }
    }
}

/// One button of an `app.alert` dialogue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertButton {
    /// OK: Adobe's 1.
    Ok,
    /// Cancel: Adobe's 2.
    Cancel,
    /// No: Adobe's 3.
    No,
    /// Yes: Adobe's 4.
    Yes,
}

impl AlertButton {
    /// The button's label, which every window shows and the drive presses by name.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Ok => "OK",
            Self::Cancel => "Cancel",
            Self::No => "No",
            Self::Yes => "Yes",
        }
    }

    /// The number `app.alert` returns for it.
    #[must_use]
    pub const fn returned(self) -> u8 {
        match self {
            Self::Ok => 1,
            Self::Cancel => 2,
            Self::No => 3,
            Self::Yes => 4,
        }
    }
}

/// What the person answered a [`ScriptQuestion`] with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScriptAnswer {
    /// An alert's button, or [`AlertButtons::dismissed`]'s where the dialogue was closed.
    Pressed(AlertButton),
    /// A response's text, or `None` where the person cancelled — `app.response`'s `null`.
    Typed(Option<String>),
    /// The window has no dialogue to put the question on, and said so. The script is answered
    /// as though the dialogue had been closed: [`AlertButtons::dismissed`]'s button, or `null`.
    Unanswerable,
}

impl ScriptAnswer {
    /// The answer a dialogue closed without a press gives `question`.
    #[must_use]
    pub fn dismissed(question: &ScriptQuestion) -> Self {
        match question {
            ScriptQuestion::Alert { buttons, .. } => Self::Pressed(buttons.dismissed()),
            ScriptQuestion::Response { .. } => Self::Typed(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{AlertButton, AlertButtons, ScriptAnswer, ScriptQuestion};

    /// Every set's close is one of its own buttons, and never Yes.
    #[test]
    fn a_closed_dialogue_answers_with_a_button_it_offered_and_never_yes() {
        for set in [
            AlertButtons::Ok,
            AlertButtons::OkCancel,
            AlertButtons::YesNo,
            AlertButtons::YesNoCancel,
        ] {
            let closed = set.dismissed();
            assert!(set.buttons().contains(&closed), "{set:?}");
            assert_ne!(closed, AlertButton::Yes, "{set:?}");
        }
    }

    /// Adobe's return values, one to four, each once.
    #[test]
    fn the_four_buttons_return_one_to_four() {
        let mut returned: Vec<u8> = [
            AlertButton::Ok,
            AlertButton::Cancel,
            AlertButton::No,
            AlertButton::Yes,
        ]
        .iter()
        .map(|button| button.returned())
        .collect();
        returned.sort_unstable();
        assert_eq!(returned, [1, 2, 3, 4]);
    }

    /// A response closed is `null`.
    #[test]
    fn a_closed_response_is_null() {
        let asked = ScriptQuestion::Response {
            question: "Your name?".to_owned(),
            title: None,
            default: String::new(),
            label: None,
            password: false,
        };
        assert_eq!(ScriptAnswer::dismissed(&asked), ScriptAnswer::Typed(None));
    }
}
