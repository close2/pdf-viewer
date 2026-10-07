//! What a script asks the person reading and what they answer: Adobe's `app.alert` and
//! `app.response`, the two calls RFC 0008 section 4.2 says need a host (ADRs 1627, 1628).
//!
//! The arguments and the numbers are Adobe's *JavaScript for Acrobat API Reference*, "app
//! methods", cited and never quoted: `app.alert` takes a message, an icon numbered 0 to 3 (error,
//! warning, question, status) and a button set numbered 0 to 3 (OK; OK and Cancel; Yes and No;
//! Yes, No and Cancel), and returns the button pressed as 1 to 4 (OK, Cancel, No, Yes);
//! `app.response` takes a question, a title, a default answer, whether the answer is a password and
//! a label, and returns the text or `null` where the person cancelled. The shapes are the ones
//! `viewer_core::ScriptQuestion` and `ScriptAnswer` carry to a window, variant for variant, so a
//! host converts one to the other without deciding anything.
//!
//! Where a question waits is the runner's: a realm hands it to an [`Asker`] and is handed back an
//! [`Answer`]. In the confined worker the asker is the wire, and the script is held in its worker
//! until the host sends the answer; in process it is whatever the caller supplied, or nobody.

/// What a script asks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Question {
    /// `app.alert`: a message and the buttons it offers.
    Alert {
        /// The script's message.
        message: String,
        /// The icon the script named.
        icon: Icon,
        /// The buttons the script asked for.
        buttons: Buttons,
        /// The script's title for the dialogue, where it gave one.
        title: Option<String>,
    },
    /// `app.response`: a question answered by typing.
    Response {
        /// The script's question.
        question: String,
        /// The script's title for the dialogue, where it gave one.
        title: Option<String>,
        /// The text the entry starts with; empty where the script gave none.
        default: String,
        /// The label beside the entry, where the script gave one.
        label: Option<String>,
        /// Whether the entry is a password's.
        password: bool,
    },
}

impl Question {
    /// The answer a dialogue closed without a press gives: [`Buttons::dismissed`]'s button for an
    /// alert, `null` for a response — ADR 1628's choice, and what a question nobody answered in
    /// time is answered with (ADR 1627).
    #[must_use]
    pub fn dismissed(&self) -> Answer {
        match self {
            Self::Alert { buttons, .. } => Answer::Pressed(buttons.dismissed()),
            Self::Response { .. } => Answer::Typed(None),
        }
    }

    /// The question in one line, for a report.
    #[must_use]
    pub fn summary(&self) -> String {
        let cut = |text: &str| -> String {
            let line: String = text.chars().take(120).collect();
            line.replace(['\n', '\r'], " ")
        };
        match self {
            Self::Alert { message, .. } => format!("app.alert({:?})", cut(message)),
            Self::Response { question, .. } => format!("app.response({:?})", cut(question)),
        }
    }
}

/// `app.alert`'s icon: Adobe's 0 to 3.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Icon {
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

impl Icon {
    /// The four, in Adobe's order.
    pub const ALL: [Self; 4] = [Self::Error, Self::Warning, Self::Question, Self::Status];
}

/// `app.alert`'s button set: Adobe's 0 to 3.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Buttons {
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

impl Buttons {
    /// The four, in Adobe's order.
    pub const ALL: [Self; 4] = [Self::Ok, Self::OkCancel, Self::YesNo, Self::YesNoCancel];

    /// Whether the set offers `button`.
    #[must_use]
    pub fn offers(self, button: Button) -> bool {
        match self {
            Self::Ok => button == Button::Ok,
            Self::OkCancel => matches!(button, Button::Ok | Button::Cancel),
            Self::YesNo => matches!(button, Button::Yes | Button::No),
            Self::YesNoCancel => matches!(button, Button::Yes | Button::No | Button::Cancel),
        }
    }

    /// The button a dialogue closed without a press answers: Cancel where it is offered, No where
    /// the choice is Yes or No, OK where OK is all there is — never Yes (ADR 1628).
    #[must_use]
    pub fn dismissed(self) -> Button {
        match self {
            Self::Ok => Button::Ok,
            Self::OkCancel | Self::YesNoCancel => Button::Cancel,
            Self::YesNo => Button::No,
        }
    }
}

/// One button of an `app.alert` dialogue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Button {
    /// OK: 1.
    Ok,
    /// Cancel: 2.
    Cancel,
    /// No: 3.
    No,
    /// Yes: 4.
    Yes,
}

impl Button {
    /// The four, in the order of the numbers `app.alert` returns.
    pub const ALL: [Self; 4] = [Self::Ok, Self::Cancel, Self::No, Self::Yes];

    /// The number `app.alert` returns for it.
    #[must_use]
    pub fn returned(self) -> u8 {
        match self {
            Self::Ok => 1,
            Self::Cancel => 2,
            Self::No => 3,
            Self::Yes => 4,
        }
    }
}

/// What a question was answered with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Answer {
    /// An alert's button.
    Pressed(Button),
    /// A response's text, or `None` — `null` — where the person cancelled.
    Typed(Option<String>),
    /// Nobody can be asked: the face has no dialogue, or no face was supplied. The script is
    /// answered as a closed dialogue answers ([`Question::dismissed`]), and the run says so.
    Unanswerable,
}

/// Where a realm hands a question and waits for its answer.
///
/// Called on the realm's own thread, inside the script's call, and the call returns what this
/// answers: in the confined worker that is the wire to the host, which holds the script until the
/// person has answered (ADR 1627); in process it is the caller's, and a caller that supplies none
/// has every question answered [`Answer::Unanswerable`].
pub trait Asker: std::fmt::Debug {
    /// The answer to `question`.
    fn ask(&self, question: &Question) -> Answer;
}

/// The asker of a realm nobody supplied one to: every question is unanswerable.
#[derive(Debug, Clone, Copy, Default)]
pub struct Nobody;

impl Asker for Nobody {
    fn ask(&self, _question: &Question) -> Answer {
        Answer::Unanswerable
    }
}

#[cfg(test)]
mod tests {
    use super::{Button, Buttons, Question};

    #[test]
    fn a_closed_dialogue_answers_with_a_button_it_offered_and_never_yes() {
        for set in Buttons::ALL {
            let closed = set.dismissed();
            assert!(set.offers(closed), "{set:?}");
            assert_ne!(closed, Button::Yes, "{set:?}");
        }
        let asked = Question::Response {
            question: "Name?".to_owned(),
            title: None,
            default: String::new(),
            label: None,
            password: false,
        };
        assert_eq!(asked.dismissed(), super::Answer::Typed(None));
    }
}
