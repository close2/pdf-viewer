//! Script dialogue wire encoding and strict decoding.

use super::{ProtocolError, Reader, Writer};

/// A script's question, on the wire: a kind byte, then its fields in [`viewer_core::ScriptQuestion`]'s
/// order, with the icon and the button set as Adobe's own numbers 0 to 3 (ADR 1628).
pub(super) fn write_script_question(writer: &mut Writer, question: &viewer_core::ScriptQuestion) {
    use viewer_core::{AlertButtons, AlertIcon, ScriptQuestion};
    match question {
        ScriptQuestion::Alert {
            message,
            icon,
            buttons,
            title,
        } => {
            writer
                .u8(0)
                .str(message)
                .u8(match icon {
                    AlertIcon::Error => 0,
                    AlertIcon::Warning => 1,
                    AlertIcon::Question => 2,
                    AlertIcon::Status => 3,
                })
                .u8(match buttons {
                    AlertButtons::Ok => 0,
                    AlertButtons::OkCancel => 1,
                    AlertButtons::YesNo => 2,
                    AlertButtons::YesNoCancel => 3,
                })
                .option_str(title.as_deref());
        }
        ScriptQuestion::Response {
            question,
            title,
            default,
            label,
            password,
        } => {
            writer
                .u8(1)
                .str(question)
                .option_str(title.as_deref())
                .str(default)
                .option_str(label.as_deref())
                .bool(*password);
        }
    }
}

/// [`write_script_question`]'s inverse; a number outside Adobe's four is refused, never mapped.
pub(super) fn read_script_question(
    reader: &mut Reader<'_>,
) -> Result<viewer_core::ScriptQuestion, ProtocolError> {
    use viewer_core::{AlertButtons, AlertIcon, ScriptQuestion};
    Ok(match reader.u8("a script question's kind")? {
        0 => ScriptQuestion::Alert {
            message: reader.string("an alert's message")?,
            icon: match reader.u8("an alert's icon")? {
                0 => AlertIcon::Error,
                1 => AlertIcon::Warning,
                2 => AlertIcon::Question,
                3 => AlertIcon::Status,
                other => {
                    return Err(ProtocolError::Unrecognised {
                        what: "an alert's icon",
                        value: u32::from(other),
                    });
                }
            },
            buttons: match reader.u8("an alert's buttons")? {
                0 => AlertButtons::Ok,
                1 => AlertButtons::OkCancel,
                2 => AlertButtons::YesNo,
                3 => AlertButtons::YesNoCancel,
                other => {
                    return Err(ProtocolError::Unrecognised {
                        what: "an alert's buttons",
                        value: u32::from(other),
                    });
                }
            },
            title: reader.option_string("an alert's title")?,
        },
        1 => ScriptQuestion::Response {
            question: reader.string("a response's question")?,
            title: reader.option_string("a response's title")?,
            default: reader.string("a response's default")?,
            label: reader.option_string("a response's label")?,
            password: reader.bool("whether a response is a password")?,
        },
        other => {
            return Err(ProtocolError::Unrecognised {
                what: "a script question's kind",
                value: u32::from(other),
            });
        }
    })
}

/// A person's answer to a script, on the wire: a kind byte, then the button as `app.alert`'s own
/// return value or the text typed.
pub(super) fn write_script_answer(writer: &mut Writer, answer: &viewer_core::ScriptAnswer) {
    use viewer_core::ScriptAnswer;
    match answer {
        ScriptAnswer::Pressed(button) => {
            writer.u8(0).u8(button.returned());
        }
        ScriptAnswer::Typed(text) => {
            writer.u8(1).option_str(text.as_deref());
        }
        ScriptAnswer::Unanswerable => {
            writer.u8(2);
        }
    }
}

/// [`write_script_answer`]'s inverse.
pub(super) fn read_script_answer(
    reader: &mut Reader<'_>,
) -> Result<viewer_core::ScriptAnswer, ProtocolError> {
    use viewer_core::{AlertButton, ScriptAnswer};
    Ok(match reader.u8("a script answer's kind")? {
        0 => ScriptAnswer::Pressed(match reader.u8("an alert's button")? {
            1 => AlertButton::Ok,
            2 => AlertButton::Cancel,
            3 => AlertButton::No,
            4 => AlertButton::Yes,
            other => {
                return Err(ProtocolError::Unrecognised {
                    what: "an alert's button",
                    value: u32::from(other),
                });
            }
        }),
        1 => ScriptAnswer::Typed(reader.option_string("a response's text")?),
        2 => ScriptAnswer::Unanswerable,
        other => {
            return Err(ProtocolError::Unrecognised {
                what: "a script answer's kind",
                value: u32::from(other),
            });
        }
    })
}
