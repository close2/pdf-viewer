//! Commands, host to viewer: their discriminants, the document's descriptor, encode and decode.
//!
//! The bytes are unchanged by the split that made this module (ADR 2256).

use pdf_model::action::DataFormat;
use pdf_model::view::WidgetAppearances;
use viewer_core::{
    Answered, Command, Find, FindDirection, FocusMove, PointerAction, PresentationMode, Purpose,
    RestrictionLevel, RestrictionOverride, RestrictionPolicy, RestrictionScope, Selection,
    TrustPolicy,
};

use pdf_signature::verdict::Acceptance;

use super::NO_DEPARTURE;
use super::edit::{decode_edit, encode_edit};
use super::error::{ProtocolError, Uncarried};
use super::frame::as_u64;
use super::panels;
use super::script_dialogues::{read_script_answer, write_script_answer};
use super::view_fields::{
    decode_page_target, decode_viewing, decode_zoom, encode_page_target, encode_viewing,
    encode_zoom,
};
use super::wire::{Reader, Writer};

/// Command discriminants. One per variant of [`viewer_core::Command`] that crosses.
pub(super) mod command_kind {
    pub(in crate::protocol) const OPEN: u8 = 1;
    pub(in crate::protocol) const TICK: u8 = 2;
    pub(in crate::protocol) const CLOSE: u8 = 3;
    pub(in crate::protocol) const FOCUS: u8 = 4;
    pub(in crate::protocol) const RESIZE: u8 = 5;
    pub(in crate::protocol) const GO_TO: u8 = 6;
    pub(in crate::protocol) const ZOOM: u8 = 7;
    pub(in crate::protocol) const SCROLL: u8 = 8;
    pub(in crate::protocol) const RESTRICT: u8 = 9;
    pub(in crate::protocol) const EDIT: u8 = 10;
    pub(in crate::protocol) const UNDO: u8 = 11;
    pub(in crate::protocol) const REDO: u8 = 12;
    pub(in crate::protocol) const EXTRACT: u8 = 13;
    pub(in crate::protocol) const SAVE: u8 = 14;
    pub(in crate::protocol) const SELECT: u8 = 15;
    pub(in crate::protocol) const FOCUSED: u8 = 16;
    pub(in crate::protocol) const ACTIVATE: u8 = 17;
    pub(in crate::protocol) const SET_GROUP: u8 = 18;
    pub(in crate::protocol) const POINTER: u8 = 19;
    pub(in crate::protocol) const SUPPLY: u8 = 20;
    pub(in crate::protocol) const DELEGATE: u8 = 21;
    pub(in crate::protocol) const FIND: u8 = 22;
    pub(in crate::protocol) const PRESENT: u8 = 23;
    pub(in crate::protocol) const LAYOUT: u8 = 24;
    // Where the reader is looking, said absolutely: what a host hands back after starting another
    // worker. ADR 0737.
    pub(in crate::protocol) const VIEW: u8 = 25;
    // The person's answer to `Event::Asking`: the *ask* level's second half, which a confined host
    // has to be able to supply. ADR 0814.
    pub(in crate::protocol) const ANSWER: u8 = 26;
    // What the document says about itself, asked for once the reader has their page:
    // `notes::about` is off the open path so that a signed document's digest is not part of a
    // launch. ADR 1044.
    pub(in crate::protocol) const REPORT: u8 = 27;
    // §12.8.1's third question: whom the reader believes. A confined worker holds the document and
    // therefore reads its signatures, so RFC 5280 section 6.1.1's inputs (d) and (b) have to cross
    // to reach the party that validates a path. The certificates cross as DER, which is what they
    // are on a host's disk (ADR 1076).
    pub(in crate::protocol) const TRUST: u8 = 28;
    // §8.10.4's target documents: which files a reference XObject may import a page from. The bytes
    // cross for the reason the certificates above do, turned round — the party that reads a file
    // off a disk is outside the confinement and the party that *parses* a PDF is inside it, which
    // is where every other document is parsed (ADR 1101).
    pub(in crate::protocol) const REFERENCE_FILES: u8 = 29;
    // §8.11.4.4's two usage categories about the reader: who is reading, and in what language.
    // Three lists of names and a language tag cross, for the reason every host-supplied policy
    // value does — the confined worker holds the document and therefore decides which of its layers
    // are drawn, and only the host was told who is reading (ADR 1106).
    pub(in crate::protocol) const AUDIENCE: u8 = 30;
    // §7.6.4.2's bit 5 asked as an operation: a person pressed copy. It crosses because the
    // confined worker holds the document and therefore the policy, exactly as `RESTRICT` does — and
    // because a query could not be asked, held or refused (ADR 1144).
    pub(in crate::protocol) const COPY: u8 = 31;
    // Table 166's `/M`: what the host's clock says, for the annotations a save writes. It crosses
    // for the reason every other host-supplied policy value does — the confined worker holds the
    // document and therefore writes §7.5.6's update, and only the host has a clock at all
    // (ADR 1160).
    pub(in crate::protocol) const CLOCK: u8 = 32;
    // §7.6.4.2's bit 3 asked as an operation, and the sheet the pages go onto. It crosses for
    // `COPY`'s reason — the confined worker holds the document and therefore the policy, and a
    // query could not be asked, held or refused — and because the interpretation the whole print
    // path rests on is the worker's (ADR 1180).
    pub(in crate::protocol) const PRINT: u8 = 33;
    // §10.8.3's simulation, asked for by a person. It crosses for the reason every other
    // host-supplied policy value does: the confined worker holds the document and therefore
    // decides what colour every mark is, and only the host was told which of the two pictures
    // this reader asked for (ADR 1228).
    pub(in crate::protocol) const SEPARATIONS: u8 = 34;
    // The name a document opened beside the one showing would be called by. It crosses for the
    // reason every other host-supplied policy value does: the confined worker reads Table 203's
    // and Table 204's `/NewWindow` and the host is the only party that knows whether this window
    // has a second place to put a document (ADR 1263).
    pub(in crate::protocol) const BESIDE: u8 = 35;
    // §12.7.4.3's commit, sent by a host for Enter in a single-line field or a control losing the
    // keyboard. It crosses because the confined worker holds the view, and therefore the typed
    // value and the scripts that judge it (ADR 1592).
    pub(in crate::protocol) const COMMIT_FIELD: u8 = 36;
    // The window's first frame is on its screen, so the document's open sequence runs. It crosses
    // because the confined worker holds the view and the scripts, and only the window knows when it
    // presented (ADR 1602).
    pub(in crate::protocol) const PRESENTED: u8 = 37;
    // RFC 0008 section 6.3's level, at the one value a confined window may send: `off`. A runner
    // is a process the host starts, so it cannot cross, and the confined window is pinned to `off`
    // for it (ADR 1616).
    pub(in crate::protocol) const SCRIPTS_OFF: u8 = 38;
    // The person's answer to `Event::AskingToRunScripts`, which crosses for `ANSWER`'s reason.
    pub(in crate::protocol) const ANSWER_SCRIPTS: u8 = 39;
    // The person's answer to `Event::ScriptAsking`, which crosses for `ANSWER`'s reason (ADR 1628).
    pub(in crate::protocol) const ANSWER_SCRIPT: u8 = 40;
    // The window's keys, which cross because a script reads them in the worker and the window
    // holds the keyboard (ADR 1771).
    pub(in crate::protocol) const KEYS: u8 = 41;
    // What the window does with a page whose whole raster would be past `MAX_PIXELS`. It crosses
    // because the confined worker holds the viewer that asks for such a page, and only the window
    // knows that it draws one from its marks at the size of its window (ADR 1944).
    pub(in crate::protocol) const OVERSIZE: u8 = 42;
    // §12.7.8's form data a server answered a submission or a fetched import with. It crosses
    // because the host holds the network and the confined worker holds the form the answer is
    // imported into, and only the worker parses it (ADR 2076).
    pub(in crate::protocol) const RESPOND: u8 = 43;
}

/// How [`Command::Open`]'s document is held, on the wire.
mod open_kind {
    /// The bytes follow, whole.
    pub(in crate::protocol) const BYTES: u8 = 0;
    /// The file's length follows, and its descriptor rides beside the frame (ADR 0812).
    pub(in crate::protocol) const ON_DISK: u8 = 1;
}

/// A descriptor as the sender holds it while the frame it rides beside is written.
pub(crate) type SentDescriptor<'a> = confined_transport::SentDescriptor<'a>;

/// A descriptor as the receiver holds it: the kernel's duplicate, owned.
pub(crate) type ReceivedDescriptor = confined_transport::link::ReceivedDescriptor;

/// Whether a document crosses as its descriptor rather than as its bytes.
///
/// One predicate for the two halves of the encoder — [`encode_command`]'s tag and
/// [`command_descriptor`]'s answer — so that they cannot disagree. A file on disk crosses as a
/// descriptor wherever the platform can pass one; elsewhere it does not cross at all, and
/// [`encode_command`] says so by name.
fn crosses_as_descriptor(bytes: &pdf_syntax::FileBytes) -> bool {
    cfg!(unix) && bytes.is_on_disk()
}

/// What rides beside an encoded command: the document's open file, where it opens one on disk.
///
/// `None` for every other command and for a document held in memory. The transport sends it
/// with the frame's header (`SCM_RIGHTS`, ADR 0812), and the receiving side takes it back out
/// in [`decode_command_holding`].
pub(crate) fn command_descriptor(command: &Command) -> Option<SentDescriptor<'_>> {
    match command {
        #[cfg(unix)]
        Command::Open { bytes, .. } if crosses_as_descriptor(bytes) => bytes.descriptor(),
        _ => None,
    }
}

/// The document a descriptor beside the frame opens, at the length the host stated.
#[cfg(unix)]
fn document_from_descriptor(
    descriptors: &mut Vec<ReceivedDescriptor>,
    length: u64,
) -> Result<pdf_syntax::FileBytes, ProtocolError> {
    if descriptors.is_empty() {
        return Err(ProtocolError::NoDescriptor {
            what: "a document on disk",
        });
    }
    // The first, because a frame carries one document and the sender attaches it to the
    // header; anything after it is nobody's and the caller closes it.
    let descriptor = descriptors.remove(0);
    pdf_syntax::FileBytes::from_handle(std::fs::File::from(descriptor), length).map_err(|_| {
        ProtocolError::Unbuildable {
            what: "a document on disk",
            why: "its stated length is more than this process can address",
        }
    })
}

/// On a platform that passes no descriptor, none can have arrived.
#[cfg(not(unix))]
fn document_from_descriptor(
    _descriptors: &mut Vec<ReceivedDescriptor>,
    _length: u64,
) -> Result<pdf_syntax::FileBytes, ProtocolError> {
    Err(ProtocolError::NoDescriptor {
        what: "a document on disk",
    })
}

/// Encodes one command.
///
/// # Errors
///
/// [`Uncarried`] for [`Command::RenderReady`], which the confined process answers itself.
#[expect(
    clippy::too_many_lines,
    reason = "one arm per variant of a `viewer-core` enum, and the count is that enum's. Splitting it would put half the vocabulary in another function and lose the property the whole module rests on: the compiler naming the variant nobody handled"
)]
pub(crate) fn encode_command(command: &Command) -> Result<Vec<u8>, Uncarried> {
    use command_kind as k;

    let mut writer = Writer::new();
    match command {
        Command::Open {
            id,
            bytes,
            password,
            fragment,
        } => {
            writer.u8(k::OPEN).document(*id);
            // **A file on disk crosses as its descriptor and its length, and bytes in memory
            // cross as bytes** (ADR 0812). The confined process has no file system, and it
            // needs none for a file somebody else opened: the host sends the open file's
            // descriptor beside this frame with `SCM_RIGHTS` — [`command_descriptor`] is what
            // rides beside these bytes — and the worker holds the same open file, reading it
            // where the document's offsets point through `pread64`, the one system call the
            // filter admits on it. The length crosses here because the worker cannot ask for
            // it: `fstat` is not on the allow-list, deliberately. Bytes in memory are what a
            // test hands over and what §7.11.4's extracted file is handed straight back as,
            // and those still cross whole.
            if crosses_as_descriptor(bytes) {
                writer.u8(open_kind::ON_DISK).u64(as_u64(bytes.len()));
            } else {
                let whole = bytes.whole().map_err(|_| Uncarried {
                    message: "Command::Open",
                    reason: "the document is on disk, which crosses as a descriptor on a \
                             platform that passes one, and this is not that platform",
                })?;
                writer.u8(open_kind::BYTES).bytes(whole);
            }
            writer
                .option_str(password.as_ref().map(viewer_core::Secret::reveal))
                .option_str(fragment.as_deref());
        }
        Command::Tick { millis } => {
            writer.u8(k::TICK).u32(*millis);
        }
        Command::Keys(keys) => {
            writer.u8(k::KEYS).u8(u8::from(keys.shift)
                | u8::from(keys.modifier) << 1
                | u8::from(keys.arrows) << 2);
        }
        Command::Close(document) => {
            writer.u8(k::CLOSE).document(*document);
        }
        Command::Focus(document) => {
            writer.u8(k::FOCUS).document(*document);
        }
        Command::Resize {
            width,
            height,
            scale,
        } => {
            writer.u8(k::RESIZE).u32(*width).u32(*height).f32(*scale);
        }
        Command::GoTo(target) => {
            writer.u8(k::GO_TO);
            encode_page_target(&mut writer, *target);
        }
        Command::Zoom { zoom, at } => {
            writer.u8(k::ZOOM);
            encode_zoom(&mut writer, *zoom);
            match at {
                Some(point) => {
                    writer.u8(1).point(*point);
                }
                None => {
                    writer.u8(0);
                }
            }
        }
        Command::Scroll { dx, dy } => {
            writer.u8(k::SCROLL).f32(*dx).f32(*dy);
        }
        Command::View(view) => {
            writer.u8(k::VIEW);
            encode_viewing(&mut writer, *view);
        }
        // A scope, then one byte per operation in `RestrictionPolicy::OPERATIONS`'s order, which
        // is the one order the wire, the C ABI and a command line all enumerate a policy in (ADR
        // 1144). The document's scope spells *no departure* as a fifth value rather than as a
        // shorter frame, so that the bytes are read the same way whichever scope sent them
        // (ADR 1145).
        Command::Restrict(scope) => {
            writer.u8(k::RESTRICT);
            let level = |level| match level {
                RestrictionLevel::On => 0,
                RestrictionLevel::Off => 1,
                RestrictionLevel::Ask => 2,
                RestrictionLevel::Warn => 3,
            };
            match scope {
                RestrictionScope::Window(policy) => {
                    writer.u8(0);
                    for operation in RestrictionPolicy::OPERATIONS {
                        writer.u8(level(policy.level(operation)));
                    }
                }
                RestrictionScope::Document(departures) => {
                    writer.u8(1);
                    for operation in RestrictionPolicy::OPERATIONS {
                        writer.u8(departures.level(operation).map_or(NO_DEPARTURE, level));
                    }
                }
            }
        }
        Command::Copy => {
            writer.u8(k::COPY);
        }
        Command::Answer { document, proceed } => {
            writer.u8(k::ANSWER).document(*document).bool(*proceed);
        }
        // §12.8.1's third question crosses for the reason every other policy does, and for a
        // sharper one of its own: the confined process is where a certification path is built, and
        // a *document* that could name its own anchors would be vouching for itself — which is the
        // one thing a trust store exists to prevent. So the anchors come from outside the
        // confinement, from the party that read them off a disk (ADR 1039, ADR 1076).
        Command::Trust(policy) => {
            writer
                .u8(k::TRUST)
                .u8(match policy.acceptance {
                    Acceptance::UnknownRevocationAccepted => 1,
                    // `RevocationMustBeGood`, and every variant added later: an acceptance this
                    // encoder does not know crosses as the conservative one, which is the
                    // direction a policy value is safe to be wrong in.
                    _ => 0,
                })
                .i64(policy.anchors.at().unix_seconds())
                .str(policy.anchors.source())
                .u32(u32::try_from(policy.anchors.len()).unwrap_or(u32::MAX));
            for (name, der) in policy.anchors.certificates() {
                writer.str(name).bytes(der);
            }
        }
        // §8.10.4's target documents. Named as well as carried, because a refusal the worker
        // raises has to say *which* file it was about in the words the host used for it.
        Command::References(files) => {
            writer
                .u8(k::REFERENCE_FILES)
                .str(&files.source)
                .u32(u32::try_from(files.files.len()).unwrap_or(u32::MAX));
            for (name, bytes) in &files.files {
                writer.str(name).bytes(bytes);
            }
        }
        // §8.11.4.4's answers about the reader. Three lists because Table 100's `/Type` decides
        // what the names beside it mean, so they may not be flattened into one.
        Command::Audience(audience) => {
            writer.u8(k::AUDIENCE);
            for names in [
                &audience.reader.individual,
                &audience.reader.title,
                &audience.reader.organisation,
            ] {
                writer.u32(u32::try_from(names.len()).unwrap_or(u32::MAX));
                for name in names {
                    writer.str(name);
                }
            }
            // An absent language is not an empty one: §14.9.2.2 gives the empty tag a meaning.
            writer
                .u8(u8::from(audience.language.is_some()))
                .str(audience.language.as_deref().unwrap_or_default());
        }
        // Table 166's `/M`, field by field rather than as §7.9.4's string: the string's grammar
        // is four digits of year and this type's is an `i32`, so spelling it here would be a
        // second encoding with a narrower range than the value it carries. An unstated zone is
        // not `Z`, which `pdf_syntax::Date` keeps apart for §7.9.4's own reason.
        Command::Clock(at) => {
            writer.u8(k::CLOCK).bool(at.is_some());
            let at = at.unwrap_or(pdf_syntax::Date {
                year: 0,
                month: 1,
                day: 1,
                hour: 0,
                minute: 0,
                second: 0,
                offset: None,
            });
            writer
                .i64(i64::from(at.year))
                .u8(at.month)
                .u8(at.day)
                .u8(at.hour)
                .u8(at.minute)
                .u8(at.second)
                .bool(at.offset.is_some())
                .i64(i64::from(at.offset.unwrap_or_default()));
        }
        // §10.8.3's simulation: one bit, because the preference has two states and the algorithm
        // is the worker's — it holds the document and therefore every colour on the page.
        Command::Beside(name) => match name {
            Some(name) => {
                writer.u8(k::BESIDE).u8(1).document(*name);
            }
            None => {
                writer.u8(k::BESIDE).u8(0);
            }
        },
        Command::Separations(simulate) => {
            writer.u8(k::SEPARATIONS).bool(*simulate);
        }
        // Table 29's arrangement crosses for the reason every other policy value does: the
        // confined process is the one that decides which pages to interpret and where each of
        // them lands, and only the host knows what the person reading has chosen.
        Command::Layout(layout) => {
            writer.u8(k::LAYOUT).u8(panels::layout_code(*layout));
        }
        // §12.4.4's mode crosses for the reason every other policy does: the confined process
        // holds the document and therefore §12.4.4.2's current navigation node, and whether a
        // presentation is running is a fact about a window on the other side of the pipe.
        Command::Present(mode) => {
            writer.u8(k::PRESENT).u8(match mode {
                PresentationMode::Off => 0,
                PresentationMode::On => 1,
            });
        }
        // §6.3.2.2's instruction crosses the confinement like every other policy: the confined
        // process interprets, so the party that draws the widgets has to tell it.
        Command::Delegate(appearances) => {
            writer.u8(k::DELEGATE).u8(match appearances {
                WidgetAppearances::Drawn => 0,
                WidgetAppearances::Delegated => 1,
            });
        }
        Command::Edit(edit) => {
            writer.u8(k::EDIT);
            encode_edit(&mut writer, edit);
        }
        Command::Undo => {
            writer.u8(k::UNDO);
        }
        Command::Redo => {
            writer.u8(k::REDO);
        }
        Command::Extract { name } => {
            writer.u8(k::EXTRACT).str(name);
        }
        Command::Save => {
            writer.u8(k::SAVE);
        }
        Command::Report => {
            writer.u8(k::REPORT);
        }
        Command::Select(selection) => {
            writer.u8(k::SELECT).u8(match selection {
                Selection::All => 0,
                Selection::None => 1,
            });
        }
        Command::Find(find) => {
            writer.u8(k::FIND);
            match find {
                Find::Start { needle, direction } => {
                    writer.u8(0).str(needle).u8(match direction {
                        FindDirection::Forward => 0,
                        FindDirection::Backward => 1,
                    });
                }
                Find::Continue => {
                    writer.u8(1);
                }
                Find::Stop => {
                    writer.u8(2);
                }
            }
        }
        Command::Focused(move_) => {
            writer.u8(k::FOCUSED).u8(match move_ {
                FocusMove::Next => 0,
                FocusMove::Previous => 1,
                FocusMove::None => 2,
            });
        }
        Command::CommitField { field } => {
            writer.u8(k::COMMIT_FIELD).str(field);
        }
        Command::Presented => {
            writer.u8(k::PRESENTED);
        }
        Command::Scripts(viewer_core::Scripting::Off) => {
            writer.u8(k::SCRIPTS_OFF);
        }
        Command::Oversize(oversize) => {
            writer.u8(k::OVERSIZE).u8(match oversize {
                viewer_core::Oversize::Refused => 0,
                viewer_core::Oversize::Listed => 1,
            });
        }
        Command::Scripts(viewer_core::Scripting::Ask(_) | viewer_core::Scripting::Run(_)) => {
            return Err(Uncarried {
                message: "Command::Scripts",
                reason: "a runner is a process the window starts and holds, and the confined \
                         window runs no document script: its level is pinned to off (ADR 1616)",
            });
        }
        Command::AnswerScripts { document, proceed } => {
            writer
                .u8(k::ANSWER_SCRIPTS)
                .document(*document)
                .bool(*proceed);
        }
        Command::AnswerScript { document, answer } => {
            writer.u8(k::ANSWER_SCRIPT).document(*document);
            write_script_answer(&mut writer, answer);
        }
        Command::Activate(object) => {
            writer.u8(k::ACTIVATE).object(*object);
        }
        Command::SetGroup { group, on } => {
            writer.u8(k::SET_GROUP).object(*group).bool(*on);
        }
        Command::Pointer { at, action } => {
            writer.u8(k::POINTER).point(*at).u8(match action {
                PointerAction::Moved => 0,
                PointerAction::Pressed => 1,
                PointerAction::Dragged => 2,
                PointerAction::Released => 3,
            });
        }
        Command::Supply { purpose, bytes } => {
            writer.u8(k::SUPPLY).u8(match purpose {
                Purpose::ImportData => 0,
                Purpose::TargetRoot => 1,
                Purpose::RemoteDocument => 2,
                Purpose::NamedPage => 3,
                Purpose::ThreadDocument => 4,
                Purpose::LaunchDocument => 5,
            });
            match bytes {
                Some(bytes) => {
                    writer.u8(1).bytes(bytes);
                }
                None => {
                    writer.u8(0);
                }
            }
        }
        // Table 194's percentages are of the media's dimensions and the scale is pixels per unit,
        // so all five numbers are the sender's and none is derived on this side.
        Command::Print(printing) => {
            let (tag, sheet) = match printing {
                viewer_core::Printing::Start(sheet) => (1, Some(sheet)),
                viewer_core::Printing::Paper(sheet) => (2, Some(sheet)),
                viewer_core::Printing::Finish => (0, None),
            };
            writer.u8(k::PRINT).u8(tag);
            if let Some(sheet) = sheet {
                // Table 193's "not known" crosses as its own byte rather than as a rectangle of
                // no extent, because the two are different answers: one says the media box
                // stands in, and the other would place a watermark at a percentage of nothing.
                match sheet.media {
                    Some(media) => {
                        writer.u8(1);
                        for corner in media {
                            writer.f32(corner);
                        }
                    }
                    None => {
                        writer.u8(0);
                    }
                }
                writer.f32(sheet.scale);
                // §12.5.6.22's matrix B, as much of it as this program produces: what the page is
                // scaled by onto that media. A separate number from the resolution above and
                // never derived from it — one is pixels per unit and the other is the page's own
                // placement (ADR 1204).
                writer.f32(sheet.page_scale);
            }
        }
        Command::Archive(_) => {
            return Err(Uncarried {
                message: "Command::Archive",
                reason: "a conversion parses the whole document again and writes a new one, and \
                         the confined window's process parses no document, so it offers no \
                         conversion (ADR 1954)",
            });
        }
        // §12.7.8's form data a server answered with, crossing to the worker that holds the form
        // it is imported into (ADR 2076). The URL crosses as the sentence's source, and the format
        // as the host read it off the answer — the worker parses the bytes, this side never does.
        Command::Respond {
            document,
            answers,
            source,
            format,
            bytes,
        } => {
            writer
                .u8(k::RESPOND)
                .document(*document)
                .u8(match answers {
                    Answered::Submission => 0,
                    Answered::Import => 1,
                })
                .str(source)
                .u8(match format {
                    DataFormat::Fdf => 0,
                    DataFormat::Xfdf => 1,
                    DataFormat::Other => 2,
                })
                .bytes(bytes);
        }
        Command::RenderReady { .. } => {
            return Err(Uncarried {
                message: "Command::RenderReady",
                reason: "the confined process holds the rasteriser, so a render request never \
                         leaves it and the answer to one never comes back in",
            });
        }
    }
    Ok(writer.finish())
}

/// Reads one command.
///
/// # Errors
///
/// [`ProtocolError`] where a field is truncated, a discriminant is not one this build defines,
/// or bytes are left over.
pub(crate) fn decode_command(bytes: &[u8]) -> Result<Command, ProtocolError> {
    decode_command_holding(bytes, &mut Vec::new())
}

/// Reads one command, taking the document's descriptor from what arrived beside the frame.
///
/// **The descriptor is consumed here and nowhere else**, so a caller that reads a frame with
/// descriptors beside it and decodes it through this function is left holding exactly the ones
/// no command claimed — which it closes, because a descriptor nobody asked for is not a thing a
/// confined process should keep (ADR 0812). [`decode_command`] is this with nothing beside the
/// bytes, which is what a fuzz target and a test hand over.
///
/// # Errors
///
/// [`ProtocolError`] as [`decode_command`], and [`ProtocolError::NoDescriptor`] where the
/// command opens a file on disk and no descriptor arrived with it.
#[expect(
    clippy::too_many_lines,
    reason = "one arm per variant of a `viewer-core` enum, and the count is that enum's. Splitting it would put half the vocabulary in another function and lose the property the whole module rests on: the compiler naming the variant nobody handled"
)]
pub(crate) fn decode_command_holding(
    bytes: &[u8],
    descriptors: &mut Vec<ReceivedDescriptor>,
) -> Result<Command, ProtocolError> {
    use command_kind as k;

    let mut reader = Reader::new(bytes);
    let what = "a command";
    let command = match reader.u8(what)? {
        k::OPEN => Command::Open {
            id: reader.document(what)?,
            bytes: match reader.u8("how a document is held")? {
                open_kind::BYTES => reader.owned_bytes("a document's bytes")?.into(),
                open_kind::ON_DISK => {
                    let length = reader.u64("a document's length on disk")?;
                    document_from_descriptor(descriptors, length)?
                }
                value => {
                    return Err(ProtocolError::Unrecognised {
                        what: "how a document is held",
                        value: u32::from(value),
                    });
                }
            },
            password: reader.option_string("a password")?.map(Into::into),
            fragment: reader.option_string("a fragment identifier")?,
        },
        k::TICK => Command::Tick {
            millis: reader.u32("a tick")?,
        },
        k::KEYS => {
            let held = reader.u8("the keys")?;
            Command::Keys(pdf_model::view::Keys {
                shift: held & 1 != 0,
                modifier: held & 2 != 0,
                arrows: held & 4 != 0,
            })
        }
        k::CLOSE => Command::Close(reader.document(what)?),
        k::FOCUS => Command::Focus(reader.document(what)?),
        k::RESIZE => Command::Resize {
            width: reader.u32("a viewport width")?,
            height: reader.u32("a viewport height")?,
            scale: reader.f32("a display scale")?,
        },
        k::GO_TO => Command::GoTo(decode_page_target(&mut reader)?),
        k::ZOOM => {
            let zoom = decode_zoom(&mut reader)?;
            let at = if reader.bool("a zoom's anchor")? {
                Some(reader.point("a zoom's anchor")?)
            } else {
                None
            };
            Command::Zoom { zoom, at }
        }
        k::SCROLL => Command::Scroll {
            dx: reader.f32("a scroll")?,
            dy: reader.f32("a scroll")?,
        },
        k::VIEW => Command::View(decode_viewing(&mut reader)?),
        k::LAYOUT => Command::Layout(panels::layout_of(reader.u8("a page layout")?)?),
        k::ANSWER => Command::Answer {
            document: reader.document(what)?,
            proceed: reader.bool("an answer")?,
        },
        k::TRUST => {
            let acceptance = match reader.u8("a revocation acceptance")? {
                0 => Acceptance::RevocationMustBeGood,
                1 => Acceptance::UnknownRevocationAccepted,
                other => {
                    return Err(ProtocolError::Unrecognised {
                        what: "a revocation acceptance",
                        value: u32::from(other),
                    });
                }
            };
            let at = pdf_signature::x509::Instant::from_unix_seconds(reader.i64("an instant")?);
            let source = reader.string("the anchors' source")?;
            let count = reader.u32("a trust anchor count")?;
            let mut certificates = Vec::new();
            for _ in 0..count {
                let name = reader.string("a trust anchor's name")?;
                certificates.push((name, reader.bytes("a trust anchor")?.to_vec()));
            }
            Command::Trust(TrustPolicy {
                anchors: pdf_signature::trust::Supply::of(certificates, source, at),
                acceptance,
            })
        }
        k::REFERENCE_FILES => {
            let source = reader.string("the reference files' source")?;
            let count = reader.u32("a reference file count")?;
            let mut files = Vec::new();
            for _ in 0..count {
                let name = reader.string("a reference file's name")?;
                files.push((name, reader.bytes("a reference file")?.to_vec()));
            }
            Command::References(viewer_core::ReferenceFiles { files, source })
        }
        k::AUDIENCE => {
            let mut lists: [Vec<String>; 3] = [Vec::new(), Vec::new(), Vec::new()];
            for names in &mut lists {
                let count = reader.u32("a reader name count")?;
                for _ in 0..count {
                    names.push(reader.string("a reader's name")?);
                }
            }
            let [individual, title, organisation] = lists;
            let stated = reader.u8("whether a language was stated")? != 0;
            let language = reader.string("the interface language")?;
            Command::Audience(pdf_model::optional_content::Audience {
                reader: pdf_model::optional_content::Reader {
                    individual,
                    title,
                    organisation,
                },
                language: stated.then_some(language),
            })
        }
        k::CLOCK => {
            let stated = reader.bool("whether a modification date was stated")?;
            let year = reader.i64("a modification date's year")?;
            let year = i32::try_from(year).map_err(|_| ProtocolError::Unrecognised {
                what: "a modification date's year",
                value: u32::try_from(year.unsigned_abs()).unwrap_or(u32::MAX),
            })?;
            let month = reader.u8("a modification date's month")?;
            let day = reader.u8("a modification date's day")?;
            let hour = reader.u8("a modification date's hour")?;
            let minute = reader.u8("a modification date's minute")?;
            let second = reader.u8("a modification date's second")?;
            let zoned = reader.bool("whether a modification date states a zone")?;
            let offset = reader.i64("a modification date's zone offset")?;
            let offset = i16::try_from(offset).map_err(|_| ProtocolError::Unrecognised {
                what: "a modification date's zone offset",
                value: u32::try_from(offset.unsigned_abs()).unwrap_or(u32::MAX),
            })?;
            Command::Clock(stated.then_some(pdf_syntax::Date {
                year,
                month,
                day,
                hour,
                minute,
                second,
                offset: zoned.then_some(offset),
            }))
        }
        k::SEPARATIONS => Command::Separations(reader.bool("whether separations are simulated")?),
        k::BESIDE => Command::Beside(match reader.u8("a reserved name")? {
            0 => None,
            1 => Some(reader.document(what)?),
            value => {
                return Err(ProtocolError::Unrecognised {
                    what: "a reserved name",
                    value: u32::from(value),
                });
            }
        }),
        k::PRINT => {
            let tag = reader.u8("which end of a print operation")?;
            if tag == 0 {
                Command::Print(viewer_core::Printing::Finish)
            } else {
                let media = match reader.u8("whether the target media is known")? {
                    0 => None,
                    1 => {
                        let mut media = [0.0_f32; 4];
                        for corner in &mut media {
                            *corner = reader.f32("a sheet's corner")?;
                        }
                        Some(media)
                    }
                    value => {
                        return Err(ProtocolError::Unrecognised {
                            what: "whether the target media is known",
                            value: u32::from(value),
                        });
                    }
                };
                let scale = reader.f32("a print resolution")?;
                let page_scale = reader.f32("a page's scale on the media")?;
                // A scale that is not a finite positive number is a target nobody could
                // rasterise, and the worker is the untrusted side of a wire whose sender it does
                // not choose. `TargetSpec::for_page` would refuse it too, one page at a time and
                // as though the document were at fault.
                if !scale.is_finite()
                    || scale <= 0.0
                    || !page_scale.is_finite()
                    || page_scale <= 0.0
                    || !media.iter().flatten().all(|corner| corner.is_finite())
                {
                    return Err(ProtocolError::Unbuildable {
                        what: "a print sheet",
                        why: "a corner, a resolution or a placement that is not a finite number",
                    });
                }
                let sheet = viewer_core::Sheet {
                    media,
                    scale,
                    page_scale,
                };
                match tag {
                    1 => Command::Print(viewer_core::Printing::Start(sheet)),
                    2 => Command::Print(viewer_core::Printing::Paper(sheet)),
                    value => {
                        return Err(ProtocolError::Unrecognised {
                            what: "which end of a print operation",
                            value: u32::from(value),
                        });
                    }
                }
            }
        }
        k::RESTRICT => {
            let scope = reader.u8("a restriction scope")?;
            let mut levels = [None; RestrictionPolicy::OPERATIONS.len()];
            for level in &mut levels {
                *level = match reader.u8("a restriction level")? {
                    0 => Some(RestrictionLevel::On),
                    1 => Some(RestrictionLevel::Off),
                    2 => Some(RestrictionLevel::Ask),
                    3 => Some(RestrictionLevel::Warn),
                    NO_DEPARTURE if scope == 1 => None,
                    value => {
                        return Err(ProtocolError::Unrecognised {
                            what: "a restriction level",
                            value: u32::from(value),
                        });
                    }
                };
            }
            match scope {
                0 => {
                    let mut policy = RestrictionPolicy::default();
                    for (operation, level) in RestrictionPolicy::OPERATIONS.into_iter().zip(levels)
                    {
                        // Unreachable by construction: the arm above answers `None` only where
                        // the scope byte said 1, and this is the branch where it said 0.
                        if let Some(level) = level {
                            policy = policy.with(operation, level);
                        }
                    }
                    Command::Restrict(RestrictionScope::Window(policy))
                }
                1 => {
                    let mut departures = RestrictionOverride::NONE;
                    for (operation, level) in RestrictionPolicy::OPERATIONS.into_iter().zip(levels)
                    {
                        departures = departures.with(operation, level);
                    }
                    Command::Restrict(RestrictionScope::Document(departures))
                }
                value => {
                    return Err(ProtocolError::Unrecognised {
                        what: "a restriction scope",
                        value: u32::from(value),
                    });
                }
            }
        }
        k::COPY => Command::Copy,
        k::PRESENT => Command::Present(match reader.u8("a presentation mode")? {
            0 => PresentationMode::Off,
            1 => PresentationMode::On,
            value => {
                return Err(ProtocolError::Unrecognised {
                    what: "a presentation mode",
                    value: u32::from(value),
                });
            }
        }),
        k::DELEGATE => Command::Delegate(match reader.u8("a widget-appearance policy")? {
            0 => WidgetAppearances::Drawn,
            1 => WidgetAppearances::Delegated,
            value => {
                return Err(ProtocolError::Unrecognised {
                    what: "a widget-appearance policy",
                    value: u32::from(value),
                });
            }
        }),
        k::EDIT => Command::Edit(decode_edit(&mut reader)?),
        k::UNDO => Command::Undo,
        k::REDO => Command::Redo,
        k::EXTRACT => Command::Extract {
            name: reader.string("an attachment's name")?,
        },
        k::SAVE => Command::Save,
        k::REPORT => Command::Report,
        k::FIND => Command::Find(match reader.u8("a find step")? {
            0 => Find::Start {
                needle: reader.string("a search string")?,
                direction: match reader.u8("a search direction")? {
                    0 => FindDirection::Forward,
                    1 => FindDirection::Backward,
                    other => {
                        return Err(ProtocolError::Unrecognised {
                            what: "a search direction",
                            value: other.into(),
                        });
                    }
                },
            },
            1 => Find::Continue,
            2 => Find::Stop,
            other => {
                return Err(ProtocolError::Unrecognised {
                    what: "a find step",
                    value: other.into(),
                });
            }
        }),
        k::SELECT => Command::Select(match reader.u8("a selection")? {
            0 => Selection::All,
            1 => Selection::None,
            value => {
                return Err(ProtocolError::Unrecognised {
                    what: "a selection",
                    value: u32::from(value),
                });
            }
        }),
        k::FOCUSED => Command::Focused(match reader.u8("a focus move")? {
            0 => FocusMove::Next,
            1 => FocusMove::Previous,
            2 => FocusMove::None,
            value => {
                return Err(ProtocolError::Unrecognised {
                    what: "a focus move",
                    value: u32::from(value),
                });
            }
        }),
        k::RESPOND => Command::Respond {
            document: reader.document(what)?,
            answers: match reader.u8("which request an answer answers")? {
                0 => Answered::Submission,
                1 => Answered::Import,
                value => {
                    return Err(ProtocolError::Unrecognised {
                        what: "which request an answer answers",
                        value: u32::from(value),
                    });
                }
            },
            source: reader.string("where an answer came from")?,
            format: match reader.u8("a form data format")? {
                0 => DataFormat::Fdf,
                1 => DataFormat::Xfdf,
                2 => DataFormat::Other,
                value => {
                    return Err(ProtocolError::Unrecognised {
                        what: "a form data format",
                        value: u32::from(value),
                    });
                }
            },
            bytes: reader.owned_bytes("an answer's body")?,
        },
        k::COMMIT_FIELD => Command::CommitField {
            field: reader.string("a field's qualified name")?,
        },
        k::PRESENTED => Command::Presented,
        k::SCRIPTS_OFF => Command::Scripts(viewer_core::Scripting::Off),
        k::OVERSIZE => Command::Oversize(match reader.u8("what a page past the bound is")? {
            0 => viewer_core::Oversize::Refused,
            1 => viewer_core::Oversize::Listed,
            value => {
                return Err(ProtocolError::Unrecognised {
                    what: "what a page past the bound is",
                    value: u32::from(value),
                });
            }
        }),
        k::ANSWER_SCRIPTS => Command::AnswerScripts {
            document: reader.document(what)?,
            proceed: reader.bool("an answer")?,
        },
        k::ANSWER_SCRIPT => Command::AnswerScript {
            document: reader.document(what)?,
            answer: read_script_answer(&mut reader)?,
        },
        k::ACTIVATE => Command::Activate(reader.object("an object")?),
        k::SET_GROUP => Command::SetGroup {
            group: reader.object("an optional content group")?,
            on: reader.bool("a group's state")?,
        },
        k::POINTER => Command::Pointer {
            at: reader.point("a pointer position")?,
            action: match reader.u8("a pointer action")? {
                0 => PointerAction::Moved,
                1 => PointerAction::Pressed,
                2 => PointerAction::Dragged,
                3 => PointerAction::Released,
                value => {
                    return Err(ProtocolError::Unrecognised {
                        what: "a pointer action",
                        value: u32::from(value),
                    });
                }
            },
        },
        k::SUPPLY => Command::Supply {
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
            bytes: if reader.bool("a supplied file")? {
                Some(reader.owned_bytes("a supplied file")?)
            } else {
                None
            },
        },
        value => {
            return Err(ProtocolError::Unrecognised {
                what,
                value: u32::from(value),
            });
        }
    };
    reader.end(what)?;
    Ok(command)
}
