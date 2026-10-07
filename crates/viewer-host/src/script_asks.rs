//! What the three windows say around a script's question — `app.alert` and `app.response`,
//! [`viewer_core::Event::ScriptAsking`] — so that they say the same (ADR 1628).
//!
//! The dialogue itself is each window's: a card `quorra` draws, a `GtkWindow`, a `QDialog`. What
//! is here is the part that is not a widget: the title that names the document asking, the keys
//! `quorra` binds to each button, and the lines each window prints when it puts a question and
//! when it is answered — the lines the drive waits for, and the only record a person has after
//! the dialogue has gone.

use viewer_core::{AlertButton, ScriptAnswer, ScriptQuestion};

/// The dialogue's title: the document that asks, and the script's own title beside it where the
/// script gave one.
///
/// **The document is named whatever the script titled it**, because a script's title is the
/// document's text, and a dialogue titled only by the document could pass for this program's own
/// — or for another document's, with two open.
#[must_use]
pub fn title(document: &str, question: &ScriptQuestion) -> String {
    let asked = match question {
        ScriptQuestion::Alert { title, .. } | ScriptQuestion::Response { title, .. } => title
            .as_deref()
            .map(str::trim)
            .filter(|title| !title.is_empty()),
    };
    match asked {
        Some(asked) => format!("{asked} — a script in {document}"),
        None => format!("A script in {document} asks"),
    }
}

/// The question's text: an alert's message, or a response's question.
#[must_use]
pub fn text(question: &ScriptQuestion) -> &str {
    match question {
        ScriptQuestion::Alert { message, .. } => message,
        ScriptQuestion::Response { question, .. } => question,
    }
}

/// The line a window prints as it puts the question up.
#[must_use]
pub fn put(document: &str, question: &ScriptQuestion) -> String {
    match question {
        ScriptQuestion::Alert { icon, buttons, .. } => format!(
            "a script in {document} alerts ({}, {}): {}",
            icon.word(),
            buttons
                .buttons()
                .iter()
                .map(|button| button.label())
                .collect::<Vec<_>>()
                .join("/"),
            text(question)
        ),
        ScriptQuestion::Response { .. } => {
            format!(
                "a script in {document} asks for an answer: {}",
                text(question)
            )
        }
    }
}

/// The line a window prints once the question is answered.
///
/// A password's text is never said back; a response's is, in quotation marks, because what the
/// script was given is what a person reading the log needs to see.
#[must_use]
pub fn answered(answer: &ScriptAnswer, password: bool) -> String {
    match answer {
        ScriptAnswer::Pressed(button) => {
            format!("you answered the script's alert: {}", button.label())
        }
        ScriptAnswer::Typed(Some(_)) if password => {
            "you answered the script's question with a password, which is not shown".to_owned()
        }
        ScriptAnswer::Typed(Some(text)) => {
            format!("you answered the script's question: \"{text}\"")
        }
        ScriptAnswer::Typed(None) => "you cancelled the script's question".to_owned(),
        ScriptAnswer::Unanswerable => UNANSWERABLE.to_owned(),
    }
}

/// The line a window prints when [`viewer_core::Event::ScriptQuestionWithdrawn`] takes a question
/// down: the script stopped waiting, so what it was given is the closed dialogue's answer and a
/// press at the dialogue afterwards reaches nothing (ADR 1643).
#[must_use]
pub fn withdrawn(document: &str) -> String {
    format!(
        "the question a script in {document} asked was withdrawn: nobody answered it within {} s, \
         so the script was answered as a closed dialogue answers (ADR 1643)",
        crate::policy::script_answer_wait().as_secs_f32()
    )
}

/// How long after a script's question goes up a window wakes to have it withdrawn, if nobody has
/// answered it: the runner's wait ([`crate::policy::script_answer_wait`]) and a tenth of a second,
/// because the runner withdraws a question whose wait has *passed* and the window's clock started
/// after the runner's (ADR 1643).
#[must_use]
pub fn wake_after() -> std::time::Duration {
    crate::policy::script_answer_wait().saturating_add(std::time::Duration::from_millis(100))
}

/// What a window with no dialogue for a script says, beside the
/// [`ScriptAnswer::Unanswerable`] it sends.
///
/// `quorra-confined` is the one: it is pinned to the level `off` and hands its worker no runner,
/// so no question can come; the sentence is what keeps one from being a silence if it ever does.
pub const UNANSWERABLE: &str = "a script's question was not shown: this window has no dialogue for \
                                 a document's script, so it was answered as a closed dialogue \
                                 (ADR 1628)";

/// The key `quorra`'s card binds to a button: the label's first letter, lower case.
#[must_use]
pub const fn key(button: AlertButton) -> char {
    match button {
        AlertButton::Ok => 'o',
        AlertButton::Cancel => 'c',
        AlertButton::No => 'n',
        AlertButton::Yes => 'y',
    }
}

/// The line `quorra`'s card prints under a question: each key and the answer it gives.
///
/// An alert's buttons are each their label's first letter, with Enter for the affirming one — the
/// last [`viewer_core::AlertButtons::buttons`] lays out — and Escape for the one a closed
/// dialogue answers; a response is Enter to answer with what is typed and Escape to cancel.
#[must_use]
pub fn keys_line(question: &ScriptQuestion) -> String {
    match question {
        ScriptQuestion::Alert { buttons, .. } => {
            let mut keys: Vec<String> = buttons
                .buttons()
                .iter()
                .map(|button| format!("{} — {}", key(*button).to_ascii_uppercase(), button.label()))
                .collect();
            if let Some(affirming) = buttons.buttons().last() {
                keys.push(format!("Enter — {}", affirming.label()));
            }
            keys.push(format!("Escape — {}", buttons.dismissed().label()));
            keys.join("   ·   ")
        }
        ScriptQuestion::Response { .. } => {
            "Enter — answer with what is typed   ·   Escape — cancel".to_owned()
        }
    }
}

/// The button an alert's key answers with in `quorra`'s card: a button's letter, Enter for the
/// affirming one, Escape for [`viewer_core::AlertButtons::dismissed`]'s.
#[must_use]
pub fn button_for(buttons: viewer_core::AlertButtons, pressed: Pressed<'_>) -> Option<AlertButton> {
    match pressed {
        Pressed::Enter => buttons.buttons().last().copied(),
        Pressed::Escape => Some(buttons.dismissed()),
        Pressed::Character(text) => {
            let mut characters = text.chars();
            let first = characters.next()?.to_ascii_lowercase();
            if characters.next().is_some() {
                return None;
            }
            buttons
                .buttons()
                .iter()
                .copied()
                .find(|button| key(*button) == first)
        }
    }
}

/// A key as [`button_for`] reads it: the two named keys the card binds, or a character.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pressed<'a> {
    /// Enter.
    Enter,
    /// Escape.
    Escape,
    /// The text a character key produced.
    Character(&'a str),
}

#[cfg(test)]
mod tests {
    use super::{Pressed, answered, button_for, keys_line, put, title};
    use viewer_core::{AlertButton, AlertButtons, AlertIcon, ScriptAnswer, ScriptQuestion};

    fn alert(title: Option<&str>) -> ScriptQuestion {
        ScriptQuestion::Alert {
            message: "Submit now?".to_owned(),
            icon: AlertIcon::Question,
            buttons: AlertButtons::YesNo,
            title: title.map(str::to_owned),
        }
    }

    /// The document is in the title with or without the script's own.
    #[test]
    fn the_title_names_the_document_whatever_the_script_called_it() {
        assert_eq!(title("form.pdf", &alert(None)), "A script in form.pdf asks");
        assert_eq!(
            title("form.pdf", &alert(Some("Acrobat"))),
            "Acrobat — a script in form.pdf"
        );
        assert_eq!(
            title("form.pdf", &alert(Some("  "))),
            "A script in form.pdf asks"
        );
    }

    /// The line put names the icon and the buttons in the order they are laid out.
    #[test]
    fn the_line_put_names_the_buttons() {
        assert_eq!(
            put("form.pdf", &alert(None)),
            "a script in form.pdf alerts (Question, No/Yes): Submit now?"
        );
    }

    /// Each button has its letter, Enter is the affirming one and Escape the closed dialogue's.
    #[test]
    fn the_card_keys_answer_with_the_buttons_offered() {
        let set = AlertButtons::YesNoCancel;
        assert_eq!(
            button_for(set, Pressed::Character("y")),
            Some(AlertButton::Yes)
        );
        assert_eq!(
            button_for(set, Pressed::Character("N")),
            Some(AlertButton::No)
        );
        assert_eq!(
            button_for(set, Pressed::Character("o")),
            None,
            "OK is not offered"
        );
        assert_eq!(button_for(set, Pressed::Enter), Some(AlertButton::Yes));
        assert_eq!(button_for(set, Pressed::Escape), Some(AlertButton::Cancel));
        assert_eq!(
            keys_line(&alert(None)),
            "N — No   ·   Y — Yes   ·   Enter — Yes   ·   Escape — No"
        );
    }

    /// A password is never said back.
    #[test]
    fn a_password_is_not_said_back() {
        let typed = ScriptAnswer::Typed(Some("hunter2".to_owned()));
        assert!(!answered(&typed, true).contains("hunter2"));
        assert!(answered(&typed, false).contains("\"hunter2\""));
        assert_eq!(
            answered(&ScriptAnswer::Pressed(AlertButton::Yes), false),
            "you answered the script's alert: Yes"
        );
    }
}
