//! An edit and the operation a restriction names, as the command that carries them writes them.
//!
//! The bytes are unchanged by the split that made this module (ADR 2256).

use pdf_model::restriction::Operation;
use pdf_model::view::{Entered, Markup};
use viewer_core::{AttachHome, Edit};

use super::error::ProtocolError;
use super::wire::{Reader, Writer};

pub(super) fn encode_edit(writer: &mut Writer, edit: &Edit) {
    match edit {
        // §12.7.5.4's three shapes: a value is cleared, a string, or several chosen indices,
        // because Table 233 bit 22's list box chooses several. ADR 0248.
        Edit::SetField { field, value } => {
            writer.u8(0).str(field);
            match value {
                Entered::Cleared => {
                    writer.u8(0);
                }
                Entered::Text(text) => {
                    writer.u8(1).str(text);
                }
                Entered::Chosen(indices) => {
                    writer.u8(2).usize(indices.len());
                    for index in indices {
                        writer.usize(*index);
                    }
                }
            }
        }
        Edit::Markup { kind, colour } => {
            writer
                .u8(1)
                .u8(match kind {
                    Markup::Highlight => 0,
                    Markup::Underline => 1,
                    Markup::StrikeOut => 2,
                    Markup::Squiggly => 3,
                })
                .f32(colour[0])
                .f32(colour[1])
                .f32(colour[2]);
        }
        // §12.5.6.6's free text annotation. ADR 0238.
        Edit::FreeText { from, to, colour } => {
            writer
                .u8(2)
                .point(*from)
                .point(*to)
                .f32(colour[0])
                .f32(colour[1])
                .f32(colour[2]);
        }
        Edit::SetFreeText { annotation, text } => {
            writer.u8(3).object(*annotation).str(text);
        }
        // §7.11.4's file crosses whole. The bytes ship rather than a descriptor: a confined worker
        // has no filesystem to open one through, and `doc/todo/38` records what a descriptor route
        // would need. ADR 0814.
        Edit::Attach {
            bytes,
            name,
            description,
            mime,
            home,
        } => {
            writer
                .u8(4)
                .bytes(bytes.bytes())
                .str(name)
                .option_str(description.as_deref())
                .option_str(mime.as_deref());
            match home {
                AttachHome::Document => {
                    writer.u8(0);
                }
                AttachHome::Page { at } => {
                    writer.u8(1).point(*at);
                }
            }
        }
        Edit::Detach { name } => {
            writer.u8(5).str(name);
        }
        // §12.7.5.3's file-select control: the pathname the field draws and the contents
        // §12.7.6.2 submits. The bytes ship whole for `Edit::Attach`'s reason one arm up — a
        // confined worker has no filesystem to open a path through, and the person who chose the
        // file is outside the confinement. ADR 1216.
        Edit::ChooseFile {
            field,
            pathname,
            bytes,
            mime,
        } => {
            writer
                .u8(6)
                .str(field)
                .str(pathname)
                .bytes(bytes.bytes())
                .option_str(mime.as_deref());
        }
        // §12.5.6.4's note, retyped in its window (ADR 1726).
        Edit::SetNoteText { annotation, text } => {
            writer.u8(7).object(*annotation).str(text);
        }
    }
}

pub(super) fn decode_edit(reader: &mut Reader<'_>) -> Result<Edit, ProtocolError> {
    let what = "an edit";
    Ok(match reader.u8(what)? {
        0 => Edit::SetField {
            field: reader.string("a field name")?,
            value: {
                let what = "a field value";
                match reader.u8(what)? {
                    0 => Entered::Cleared,
                    1 => Entered::Text(reader.string(what)?),
                    2 => Entered::Chosen(
                        reader.list("a chosen option", |reader| reader.usize("a chosen option"))?,
                    ),
                    value => {
                        return Err(ProtocolError::Unrecognised {
                            what,
                            value: u32::from(value),
                        });
                    }
                }
            },
        },
        1 => Edit::Markup {
            kind: match reader.u8("a markup kind")? {
                0 => Markup::Highlight,
                1 => Markup::Underline,
                2 => Markup::StrikeOut,
                3 => Markup::Squiggly,
                value => {
                    return Err(ProtocolError::Unrecognised {
                        what: "a markup kind",
                        value: u32::from(value),
                    });
                }
            },
            colour: [
                reader.f32("a markup colour")?,
                reader.f32("a markup colour")?,
                reader.f32("a markup colour")?,
            ],
        },
        2 => Edit::FreeText {
            from: reader.point("a free text corner")?,
            to: reader.point("a free text corner")?,
            colour: [
                reader.f32("a free text colour")?,
                reader.f32("a free text colour")?,
                reader.f32("a free text colour")?,
            ],
        },
        3 => Edit::SetFreeText {
            annotation: reader.object("a free text annotation")?,
            text: reader.string("a free text annotation's contents")?,
        },
        4 => Edit::Attach {
            bytes: reader.owned_bytes("a file to attach")?.into(),
            name: reader.string("an attachment's name")?,
            description: reader.option_string("an attachment's description")?,
            mime: reader.option_string("an attachment's media type")?,
            home: match reader.u8("an attachment's home")? {
                0 => AttachHome::Document,
                1 => AttachHome::Page {
                    at: reader.point("an attachment's point")?,
                },
                value => {
                    return Err(ProtocolError::Unrecognised {
                        what: "an attachment's home",
                        value: u32::from(value),
                    });
                }
            },
        },
        5 => Edit::Detach {
            name: reader.string("an attachment's name")?,
        },
        6 => Edit::ChooseFile {
            field: reader.string("a field name")?,
            pathname: reader.string("a chosen file's pathname")?,
            bytes: reader.owned_bytes("a chosen file")?.into(),
            mime: reader.option_string("a chosen file's media type")?,
        },
        7 => Edit::SetNoteText {
            annotation: reader.object("a text note")?,
            text: reader.string("a text note's contents")?,
        },
        value => {
            return Err(ProtocolError::Unrecognised {
                what,
                value: u32::from(value),
            });
        }
    })
}

/// One byte for one of `Operation`'s arms, the same for every event that names one.
pub(super) fn operation_code(operation: Operation) -> u8 {
    match operation {
        Operation::FillInForm => 0,
        Operation::Annotate => 1,
        Operation::Print => 2,
        Operation::Extract => 3,
        Operation::Modify => 4,
        Operation::Assemble => 5,
        Operation::Process => 6,
        Operation::PrintFaithfully => 7,
        Operation::HandOverMedia => 8,
    }
}

/// [`operation_code`] read back, refusing a byte no arm has.
pub(super) fn operation_of(reader: &mut Reader<'_>) -> Result<Operation, ProtocolError> {
    Ok(match reader.u8("an operation")? {
        0 => Operation::FillInForm,
        1 => Operation::Annotate,
        2 => Operation::Print,
        3 => Operation::Extract,
        4 => Operation::Modify,
        5 => Operation::Assemble,
        6 => Operation::Process,
        7 => Operation::PrintFaithfully,
        8 => Operation::HandOverMedia,
        value => {
            return Err(ProtocolError::Unrecognised {
                what: "an operation",
                value: u32::from(value),
            });
        }
    })
}
