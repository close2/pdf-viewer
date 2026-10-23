//! Events, viewer to host: their discriminants, encode and decode, and Table 164's transition.
//!
//! The bytes are unchanged by the split that made this module (ADR 2256).

use pdf_model::navigation::{Dimension, Direction, Motion, Style, Transition};
use pdf_model::submission::{Format, Method, Submission};
use pdf_render::{Point, Rect};
use pdf_syntax::Name;
use viewer_core::{Event, Extraction, Found, Purpose};

use super::edit::{operation_code, operation_of};
use super::error::{ProtocolError, Uncarried};
use super::published_policy::{read_published, write_published};
use super::script_dialogues::{read_script_question, write_script_question};
use super::wire::{Reader, Writer};

/// Event discriminants. One per variant of [`viewer_core::Event`] that crosses.
mod event_kind {
    pub(in crate::protocol) const OPENED: u8 = 1;
    pub(in crate::protocol) const OPEN_FAILED: u8 = 2;
    pub(in crate::protocol) const PASSWORD_REQUIRED: u8 = 3;
    pub(in crate::protocol) const CLOSED: u8 = 4;
    pub(in crate::protocol) const PAGE_CHANGED: u8 = 5;
    pub(in crate::protocol) const DAMAGE: u8 = 6;
    pub(in crate::protocol) const OPEN_URI: u8 = 7;
    pub(in crate::protocol) const NEEDS_FILE: u8 = 8;
    pub(in crate::protocol) const TRANSITION: u8 = 9;
    pub(in crate::protocol) const DIRTY: u8 = 10;
    pub(in crate::protocol) const SAVED: u8 = 11;
    pub(in crate::protocol) const EXTRACTED: u8 = 12;
    pub(in crate::protocol) const REFUSED: u8 = 13;
    pub(in crate::protocol) const REPORTED: u8 = 14;
    pub(in crate::protocol) const SEARCHED: u8 = 15;
    // The *ask* and *warn* levels' events, and the attachment list moving. ADR 0814.
    pub(in crate::protocol) const ASKING: u8 = 16;
    pub(in crate::protocol) const WARNED: u8 = 17;
    pub(in crate::protocol) const ATTACHMENTS_CHANGED: u8 = 18;
    /// §12.7.6.2's composed submission, for a host with a network. ADR 1062.
    pub(in crate::protocol) const SUBMIT: u8 = 19;
    /// `Command::Copy` granted, with §14.8.2.5's two orders of the text. ADR 1144.
    pub(in crate::protocol) const COPIED: u8 = 20;
    // The grant `PRINT` asked for: this document is interpreted for paper until the operation
    // ends. It crosses because the window process is the one with a printer and the worker is
    // the one with the document (ADR 1180).
    pub(in crate::protocol) const PRINTING: u8 = 21;
    // RFC 0008 section 6.3's one question per document, which crosses for `ASKING`'s reason: the
    // worker holds the view and the window holds the person (ADR 1616).
    pub(in crate::protocol) const ASKING_TO_RUN_SCRIPTS: u8 = 22;
    // RFC 0008 section 4.2's `app.alert` and `app.response`, which cross for `ASKING`'s reason: a
    // script that asks runs beside the view, and the window holds the person (ADR 1628).
    pub(in crate::protocol) const SCRIPT_ASKING: u8 = 23;
    // The question `SCRIPT_ASKING` put, withdrawn when its wait ran out (ADR 1643). This window
    // answers every question at once, so it is the arm a newer worker's event lands in.
    pub(in crate::protocol) const SCRIPT_QUESTION_WITHDRAWN: u8 = 24;
    // `app.beep`'s sound, which crosses for `SCRIPT_ASKING`'s reason: the script runs beside the
    // view, and the window holds the speaker (ADR 1702).
    pub(in crate::protocol) const BEEP: u8 = 25;
    // §12.8.3.4.4's published policy copies, which cross because the worker reads the signature
    // and the window holds the network level and the person (ADR 1738).
    pub(in crate::protocol) const SIGNATURE_POLICIES_PUBLISHED: u8 = 26;
    // A script's console request, which crosses for `BEEP`'s reason: the window holds the console
    // (ADR 1771).
    pub(in crate::protocol) const CONSOLE: u8 = 27;
    // §13.2.4.2's clip handed over, which crosses for `EXTRACTED`'s reason: the window is the host
    // that would start a player (ADR 1961).
    pub(in crate::protocol) const MEDIA: u8 = 28;
}

/// Encodes one event.
///
/// # Errors
///
/// [`Uncarried`] for [`Event::NeedsRender`], which the confined process answers itself.
#[expect(
    clippy::too_many_lines,
    reason = "one arm per variant of a `viewer-core` enum, and the count is that enum's. Splitting it would put half the vocabulary in another function and lose the property the whole module rests on: the compiler naming the variant nobody handled"
)]
pub(super) fn encode_event(event: &Event) -> Result<Vec<u8>, Uncarried> {
    use event_kind as k;

    let mut writer = Writer::new();
    match event {
        Event::Opened { document, pages } => {
            writer.u8(k::OPENED).document(*document).usize(*pages);
        }
        Event::OpenFailed { document, reason } => {
            writer.u8(k::OPEN_FAILED).document(*document).str(reason);
        }
        Event::PasswordRequired { document } => {
            writer.u8(k::PASSWORD_REQUIRED).document(*document);
        }
        Event::Closed(document) => {
            writer.u8(k::CLOSED).document(*document);
        }
        Event::PageChanged {
            document,
            index,
            label,
            of,
            section,
        } => {
            writer
                .u8(k::PAGE_CHANGED)
                .document(*document)
                .usize(*index)
                .option_str(label.as_deref())
                .usize(*of)
                .option_str(section.as_deref());
        }
        Event::Damage(rect) => {
            writer
                .u8(k::DAMAGE)
                .f32(rect.min.x)
                .f32(rect.min.y)
                .f32(rect.max.x)
                .f32(rect.max.y);
        }
        Event::OpenUri { document, uri } => {
            writer.u8(k::OPEN_URI).document(*document).str(uri);
        }
        // §12.7.6.2. `owed` does not cross: `interact::perform` has already put every sentence
        // of it into an `Event::Reported`, and carrying the same words twice would have a host
        // print them twice.
        Event::Submit {
            document,
            submission,
        } => {
            writer
                .u8(k::SUBMIT)
                .document(*document)
                .str(&submission.url)
                .u8(match submission.method {
                    Method::Get => 0,
                    Method::Post => 1,
                })
                .u8(match submission.format {
                    Format::Fdf => 0,
                    Format::HtmlForm => 1,
                    Format::Pdf => 2,
                    Format::Xfdf => 3,
                })
                // §12.7.5.3 makes the media type a function of the submission rather than of the
                // format, so it crosses as itself: a `multipart/form-data` carries RFC 2046
                // section 5.1.1's boundary parameter, which no enumerator can stand for.
                .str(&submission.media_type)
                .usize(submission.fields)
                .bytes(&submission.body);
        }
        Event::NeedsFile {
            document,
            purpose,
            name,
            beside,
        } => {
            writer
                .u8(k::NEEDS_FILE)
                .document(*document)
                .u8(match purpose {
                    Purpose::ImportData => 0,
                    Purpose::TargetRoot => 1,
                    Purpose::RemoteDocument => 2,
                    Purpose::NamedPage => 3,
                    Purpose::ThreadDocument => 4,
                    Purpose::LaunchDocument => 5,
                })
                .str(name)
                .bool(*beside);
        }
        Event::Transition {
            document,
            transition,
        } => {
            writer.u8(k::TRANSITION).document(*document);
            encode_transition(&mut writer, transition);
        }
        Event::Dirty { document, dirty } => {
            writer.u8(k::DIRTY).document(*document).bool(*dirty);
        }
        Event::Searched {
            document,
            found,
            remaining,
            wrapped,
        } => {
            writer.u8(k::SEARCHED).document(*document);
            match found {
                Some(found) => {
                    writer
                        .u8(1)
                        .usize(found.page)
                        .usize(found.range.0)
                        .usize(found.range.1);
                }
                None => {
                    writer.u8(0);
                }
            }
            writer.usize(*remaining).bool(*wrapped);
        }
        Event::Saved { document, bytes } => {
            writer.u8(k::SAVED).document(*document).bytes(bytes);
        }
        Event::Extracted {
            document,
            asked,
            name,
            bytes,
            fragment,
        } => {
            writer
                .u8(k::EXTRACTED)
                .document(*document)
                // What asked for the file, which decides whether the host on the other side of
                // this pipe may write it without a person: Annex O's `ef` is a URI's sentence and
                // a click is a person's. One byte, and the decode below refuses any other value
                // rather than guessing at the safer one.
                .u8(match asked {
                    Extraction::Asked => 0,
                    Extraction::Fragment => 1,
                })
                .str(name)
                .bytes(bytes)
                // §O.2.1's remaining parameters, which travel with the file: a host that opens
                // these bytes as a document hands them straight back as `Command::Open`'s
                // fragment, and one on the far side of a pipe needs them as much as one in the
                // same process (ADR 0431).
                .option_str(fragment.as_deref());
        }
        Event::Copied {
            document,
            logical,
            page_order,
        } => {
            writer
                .u8(k::COPIED)
                .document(*document)
                .option_str(logical.as_deref())
                .str(page_order);
        }
        Event::Printing {
            document,
            pages,
            fidelity,
        } => {
            // §7.6.4.2's bit 12 as a byte: 0 faithful, 1 degraded. A grant the window process
            // needs before it opens a dialogue, because one of the two things it decides is
            // which destinations that dialogue may offer (ADR 1203).
            writer
                .u8(k::PRINTING)
                .document(*document)
                .usize(*pages)
                .u8(match fidelity {
                    viewer_core::Fidelity::Faithful => 0,
                    viewer_core::Fidelity::Degraded => 1,
                });
        }
        Event::Refused {
            document,
            operation,
            notes,
        } => {
            writer
                .u8(k::REFUSED)
                .document(*document)
                .u8(operation_code(*operation))
                .strings(notes);
        }
        Event::Asking {
            document,
            operation,
            notes,
        } => {
            writer
                .u8(k::ASKING)
                .document(*document)
                .u8(operation_code(*operation))
                .strings(notes);
        }
        Event::Warned {
            document,
            operation,
            notes,
        } => {
            writer
                .u8(k::WARNED)
                .document(*document)
                .u8(operation_code(*operation))
                .strings(notes);
        }
        Event::AttachmentsChanged { document } => {
            writer.u8(k::ATTACHMENTS_CHANGED).document(*document);
        }
        Event::AskingToRunScripts {
            document,
            script,
            first_line,
        } => {
            writer
                .u8(k::ASKING_TO_RUN_SCRIPTS)
                .document(*document)
                .str(script)
                .str(first_line);
        }
        Event::ScriptAsking { document, question } => {
            writer.u8(k::SCRIPT_ASKING).document(*document);
            write_script_question(&mut writer, question);
        }
        Event::ScriptQuestionWithdrawn { document } => {
            writer.u8(k::SCRIPT_QUESTION_WITHDRAWN).document(*document);
        }
        Event::Beep { document, sound } => {
            let tag = pdf_model::view::Sound::ALL
                .iter()
                .position(|held| held == sound)
                .and_then(|index| u8::try_from(index).ok())
                .unwrap_or(u8::MAX);
            writer.u8(k::BEEP).document(*document).u8(tag);
        }
        Event::Media {
            document,
            content_type,
            media,
        } => {
            writer
                .u8(k::MEDIA)
                .document(*document)
                .option_str(content_type.as_deref());
            match media {
                viewer_core::Media::Embedded { name, bytes } => {
                    writer.u8(0).str(name).bytes(bytes);
                }
                viewer_core::Media::Url(url) => {
                    writer.u8(1).str(url);
                }
                viewer_core::Media::File(name) => {
                    writer.u8(2).str(name);
                }
            }
        }
        Event::Console { document, request } => {
            let command = match request.command {
                pdf_model::view::ConsoleCommand::Show => 0,
                pdf_model::view::ConsoleCommand::Hide => 1,
                pdf_model::view::ConsoleCommand::Clear => 2,
            };
            writer
                .u8(k::CONSOLE)
                .document(*document)
                .u8(command)
                .u64(u64::try_from(request.at).unwrap_or(u64::MAX));
        }
        Event::SignaturePoliciesPublished { document, policies } => {
            writer
                .u8(k::SIGNATURE_POLICIES_PUBLISHED)
                .document(*document)
                .usize(policies.len());
            for policy in policies {
                write_published(&mut writer, policy);
            }
        }
        Event::Reported {
            document,
            page,
            notes,
        } => {
            writer.u8(k::REPORTED).document(*document);
            match page {
                Some(page) => {
                    writer.u8(1).usize(*page);
                }
                None => {
                    writer.u8(0);
                }
            }
            writer.strings(notes);
        }
        Event::Archiving { .. } => {
            return Err(Uncarried {
                message: "Event::Archiving",
                reason: "it answers `Command::Archive`, which the confined window does not send \
                         (ADR 1954)",
            });
        }
        Event::NeedsRender(_) => {
            return Err(Uncarried {
                message: "Event::NeedsRender",
                reason: "the confined process rasterises what it interprets, so the host is sent \
                         the pixels rather than the display list",
            });
        }
    }
    Ok(writer.finish())
}

/// Reads one event.
///
/// # Errors
///
/// [`ProtocolError`] where a field is truncated, a discriminant is not one this build defines,
/// or bytes are left over.
#[expect(
    clippy::too_many_lines,
    reason = "one arm per variant of a `viewer-core` enum, and the count is that enum's. Splitting it would put half the vocabulary in another function and lose the property the whole module rests on: the compiler naming the variant nobody handled"
)]
pub(super) fn decode_event(bytes: &[u8]) -> Result<Event, ProtocolError> {
    use event_kind as k;

    let mut reader = Reader::new(bytes);
    let what = "an event";
    let event = match reader.u8(what)? {
        k::OPENED => Event::Opened {
            document: reader.document(what)?,
            pages: reader.usize("a page count")?,
        },
        k::OPEN_FAILED => Event::OpenFailed {
            document: reader.document(what)?,
            reason: reader.string("a reason")?,
        },
        k::PASSWORD_REQUIRED => Event::PasswordRequired {
            document: reader.document(what)?,
        },
        k::CLOSED => Event::Closed(reader.document(what)?),
        k::PAGE_CHANGED => Event::PageChanged {
            document: reader.document(what)?,
            index: reader.usize("a page index")?,
            label: reader.option_string("a page label")?,
            of: reader.usize("a page count")?,
            section: reader.option_string("an outline section")?,
        },
        k::DAMAGE => Event::Damage(Rect {
            min: Point::new(reader.f32("a rectangle")?, reader.f32("a rectangle")?),
            max: Point::new(reader.f32("a rectangle")?, reader.f32("a rectangle")?),
        }),
        k::SUBMIT => Event::Submit {
            document: reader.document(what)?,
            submission: Box::new(Submission {
                url: reader.string("a submission's URL")?,
                method: match reader.u8("a submission's method")? {
                    0 => Method::Get,
                    1 => Method::Post,
                    value => {
                        return Err(ProtocolError::Unrecognised {
                            what: "a submission's method",
                            value: u32::from(value),
                        });
                    }
                },
                format: match reader.u8("a submission's format")? {
                    0 => Format::Fdf,
                    1 => Format::HtmlForm,
                    2 => Format::Pdf,
                    3 => Format::Xfdf,
                    value => {
                        return Err(ProtocolError::Unrecognised {
                            what: "a submission's format",
                            value: u32::from(value),
                        });
                    }
                },
                media_type: reader.string("a submission's media type")?,
                fields: reader.usize("a submission's field count")?,
                body: reader.owned_bytes("a submission's body")?,
                owed: Vec::new(),
            }),
        },
        k::OPEN_URI => Event::OpenUri {
            document: reader.document(what)?,
            uri: reader.string("a URI")?,
        },
        k::NEEDS_FILE => Event::NeedsFile {
            document: reader.document(what)?,
            purpose: match reader.u8("a purpose")? {
                0 => Purpose::ImportData,
                1 => Purpose::TargetRoot,
                2 => Purpose::RemoteDocument,
                3 => Purpose::NamedPage,
                4 => Purpose::ThreadDocument,
                5 => Purpose::LaunchDocument,
                value => {
                    return Err(ProtocolError::Unrecognised {
                        what: "a purpose",
                        value: u32::from(value),
                    });
                }
            },
            name: reader.string("a file specification's name")?,
            beside: reader.bool("whether the file opens beside")?,
        },
        k::TRANSITION => Event::Transition {
            document: reader.document(what)?,
            transition: decode_transition(&mut reader)?,
        },
        k::SEARCHED => Event::Searched {
            document: reader.document("a document")?,
            found: match reader.u8("whether a search found anything")? {
                0 => None,
                1 => Some(Found {
                    page: reader.usize("a page index")?,
                    range: (
                        reader.usize("a match's first offset")?,
                        reader.usize("a match's last offset")?,
                    ),
                }),
                other => {
                    return Err(ProtocolError::Unrecognised {
                        what: "a search result",
                        value: other.into(),
                    });
                }
            },
            remaining: reader.usize("how many pages are left to search")?,
            wrapped: reader.bool("whether the search wrapped")?,
        },
        k::DIRTY => Event::Dirty {
            document: reader.document(what)?,
            dirty: reader.bool("a dirty flag")?,
        },
        k::SAVED => Event::Saved {
            document: reader.document(what)?,
            bytes: reader.owned_bytes("a saved file")?,
        },
        k::EXTRACTED => Event::Extracted {
            document: reader.document(what)?,
            asked: match reader.u8("what asked for an attachment")? {
                0 => Extraction::Asked,
                1 => Extraction::Fragment,
                value => {
                    return Err(ProtocolError::Unrecognised {
                        what: "what asked for an attachment",
                        value: u32::from(value),
                    });
                }
            },
            name: reader.string("an attachment's name")?,
            bytes: reader.owned_bytes("an attachment")?,
            fragment: reader.option_string("a fragment identifier")?,
        },
        k::COPIED => Event::Copied {
            document: reader.document(what)?,
            logical: reader.option_string("§14.8.2.5's logical content order")?,
            page_order: reader.string("the selected text")?,
        },
        k::PRINTING => Event::Printing {
            document: reader.document(what)?,
            pages: reader.usize("how many pages a print operation has")?,
            fidelity: match reader.u8("what Table 22 bit 12 left of a print job")? {
                0 => viewer_core::Fidelity::Faithful,
                1 => viewer_core::Fidelity::Degraded,
                value => {
                    return Err(ProtocolError::Unrecognised {
                        what: "what Table 22 bit 12 left of a print job",
                        value: u32::from(value),
                    });
                }
            },
        },
        k::REFUSED => Event::Refused {
            document: reader.document(what)?,
            operation: operation_of(&mut reader)?,
            notes: reader.strings("a refusal's notes")?,
        },
        k::ASKING => Event::Asking {
            document: reader.document(what)?,
            operation: operation_of(&mut reader)?,
            notes: reader.strings("a question's notes")?,
        },
        k::WARNED => Event::Warned {
            document: reader.document(what)?,
            operation: operation_of(&mut reader)?,
            notes: reader.strings("a warning's notes")?,
        },
        k::ATTACHMENTS_CHANGED => Event::AttachmentsChanged {
            document: reader.document(what)?,
        },
        k::ASKING_TO_RUN_SCRIPTS => Event::AskingToRunScripts {
            document: reader.document(what)?,
            script: reader.string("the script a question names")?,
            first_line: reader.string("a script's first line")?,
        },
        k::SCRIPT_ASKING => Event::ScriptAsking {
            document: reader.document(what)?,
            question: read_script_question(&mut reader)?,
        },
        k::SCRIPT_QUESTION_WITHDRAWN => Event::ScriptQuestionWithdrawn {
            document: reader.document(what)?,
        },
        k::BEEP => {
            let document = reader.document(what)?;
            let tag = reader.u8("a sound")?;
            Event::Beep {
                document,
                sound: pdf_model::view::Sound::ALL
                    .get(usize::from(tag))
                    .copied()
                    .ok_or(ProtocolError::Unrecognised {
                        what: "a sound",
                        value: u32::from(tag),
                    })?,
            }
        }
        k::CONSOLE => {
            let document = reader.document(what)?;
            let tag = reader.u8("a console request")?;
            let command = match tag {
                0 => pdf_model::view::ConsoleCommand::Show,
                1 => pdf_model::view::ConsoleCommand::Hide,
                2 => pdf_model::view::ConsoleCommand::Clear,
                _ => {
                    return Err(ProtocolError::Unrecognised {
                        what: "a console request",
                        value: u32::from(tag),
                    });
                }
            };
            let at =
                usize::try_from(reader.u64("a console request's place")?).unwrap_or(usize::MAX);
            Event::Console {
                document,
                request: pdf_model::view::ConsoleRequest { command, at },
            }
        }
        k::MEDIA => {
            let document = reader.document(what)?;
            let content_type = reader.option_string("a media clip's content type")?;
            let media = match reader.u8("where a media clip's data is")? {
                0 => viewer_core::Media::Embedded {
                    name: reader.string("a media clip's name")?,
                    bytes: reader.owned_bytes("a media clip")?,
                },
                1 => viewer_core::Media::Url(reader.string("a media clip's URL")?),
                2 => viewer_core::Media::File(reader.string("a media clip's file")?),
                value => {
                    return Err(ProtocolError::Unrecognised {
                        what: "where a media clip's data is",
                        value: u32::from(value),
                    });
                }
            };
            Event::Media {
                document,
                content_type,
                media,
            }
        }
        k::SIGNATURE_POLICIES_PUBLISHED => Event::SignaturePoliciesPublished {
            document: reader.document(what)?,
            policies: reader.list("a signature's published policies", read_published)?,
        },
        k::REPORTED => Event::Reported {
            document: reader.document(what)?,
            page: if reader.bool("a page number")? {
                Some(reader.usize("a page number")?)
            } else {
                None
            },
            notes: reader.strings("a report's notes")?,
        },
        value => {
            return Err(ProtocolError::Unrecognised {
                what,
                value: u32::from(value),
            });
        }
    };
    reader.end(what)?;
    Ok(event)
}

/// Encodes everything one command caused, as one payload.
///
/// # Errors
///
/// [`Uncarried`] where one of the events does not cross. The whole answer fails rather than the
/// one event being dropped: a host given three of four events would have no way to know.
pub(crate) fn encode_events(events: &[Event]) -> Result<Vec<u8>, Uncarried> {
    let mut writer = Writer::new();
    writer.usize(events.len());
    for event in events {
        writer.bytes(&encode_event(event)?);
    }
    Ok(writer.finish())
}

/// Reads everything one command caused.
///
/// # Errors
///
/// [`ProtocolError`] where a field is truncated or a discriminant is not one this build defines.
pub(crate) fn decode_events(bytes: &[u8]) -> Result<Vec<Event>, ProtocolError> {
    let what = "a run of events";
    let mut reader = Reader::new(bytes);
    let events = reader.list(what, |reader| decode_event(reader.bytes(what)?))?;
    reader.end(what)?;
    Ok(events)
}

/// Encodes Table 164's transition, whole.
fn encode_transition(writer: &mut Writer, transition: &Transition) {
    match &transition.style {
        Style::Split => writer.u8(0),
        Style::Blinds => writer.u8(1),
        Style::Box => writer.u8(2),
        Style::Wipe => writer.u8(3),
        Style::Dissolve => writer.u8(4),
        Style::Glitter => writer.u8(5),
        Style::Replace => writer.u8(6),
        Style::Fly => writer.u8(7),
        Style::Push => writer.u8(8),
        Style::Cover => writer.u8(9),
        Style::Uncover => writer.u8(10),
        Style::Fade => writer.u8(11),
        Style::Unrecognised(name) => writer.u8(12).bytes(&name.0),
    };
    writer
        .f32(transition.duration)
        .u8(match transition.dimension {
            Dimension::Horizontal => 0,
            Dimension::Vertical => 1,
        })
        .u8(match transition.motion {
            Motion::Inward => 0,
            Motion::Outward => 1,
        });
    match transition.direction {
        Direction::Degrees(degrees) => writer.u8(0).f32(degrees),
        Direction::None => writer.u8(1),
    };
    writer.f32(transition.scale).bool(transition.opaque);
}

/// Reads Table 164's transition.
fn decode_transition(reader: &mut Reader<'_>) -> Result<Transition, ProtocolError> {
    let what = "a transition style";
    let style = match reader.u8(what)? {
        0 => Style::Split,
        1 => Style::Blinds,
        2 => Style::Box,
        3 => Style::Wipe,
        4 => Style::Dissolve,
        5 => Style::Glitter,
        6 => Style::Replace,
        7 => Style::Fly,
        8 => Style::Push,
        9 => Style::Cover,
        10 => Style::Uncover,
        11 => Style::Fade,
        12 => Style::Unrecognised(Name::new(
            reader.bytes("a transition style's name")?.to_vec(),
        )),
        value => {
            return Err(ProtocolError::Unrecognised {
                what,
                value: u32::from(value),
            });
        }
    };
    let duration = reader.f32("a transition's duration")?;
    let dimension = match reader.u8("a transition's dimension")? {
        0 => Dimension::Horizontal,
        1 => Dimension::Vertical,
        value => {
            return Err(ProtocolError::Unrecognised {
                what: "a transition's dimension",
                value: u32::from(value),
            });
        }
    };
    let motion = match reader.u8("a transition's motion")? {
        0 => Motion::Inward,
        1 => Motion::Outward,
        value => {
            return Err(ProtocolError::Unrecognised {
                what: "a transition's motion",
                value: u32::from(value),
            });
        }
    };
    let direction = match reader.u8("a transition's direction")? {
        0 => Direction::Degrees(reader.f32("a transition's direction")?),
        1 => Direction::None,
        value => {
            return Err(ProtocolError::Unrecognised {
                what: "a transition's direction",
                value: u32::from(value),
            });
        }
    };
    Ok(Transition {
        style,
        duration,
        dimension,
        motion,
        direction,
        scale: reader.f32("a transition's scale")?,
        opaque: reader.bool("a transition's opacity")?,
    })
}
