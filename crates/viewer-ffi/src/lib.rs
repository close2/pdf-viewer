//! `viewer-core`'s vocabulary, spelled in C.
//!
//! The third host on `doc/ui-boundary.md`'s boundary and the last one `doc/todo/30` names, after
//! GTK4 (ADR 0244) and Qt 6 (ADR 0246). Its whole subject is the one thing those two did not
//! have to face: **a C caller cannot fail to compile.** Every guarantee this project gets from
//! leaving `viewer-core`'s enums exhaustive — *"a new `Event` should fail to compile in every
//! consumer"* — is unavailable here, and what replaces it has to be said rather than assumed.
//!
//! ADR 0247 is the argument. What follows is the shape, and the four questions it answers.
//!
//! # 1. Commands are functions, not a tagged union
//!
//! `pdfv_open`, `pdfv_go_to_page`, `pdfv_zoom`: one entry point per [`viewer_core::Command`],
//! taking that command's own arguments. The alternative — one `pdfv_command` struct with a tag
//! and a union — was refused for a reason that is entirely about C: **a union's size is part of
//! the ABI**, so a command added later changes the size of a type every caller has already
//! compiled, and an old caller passing an old-sized struct to a new library is undefined
//! behaviour that no diagnostic catches. A function added later is a symbol an old caller never
//! looks up. A caller compiled against version 1 keeps working against version 2 with no source
//! change and no recompilation; what it loses is the new command, which is what "an old program
//! does not use a new feature" should cost.
//!
//! # 2. Events and answers arrive owned, in a batch the caller frees
//!
//! [`Viewer::handle`](viewer_core::Viewer::handle) returns an iterator borrowing the viewer, and
//! [`Viewer::query`](viewer_core::Viewer::query) an [`Answer`](viewer_core::Answer) that may too.
//! Neither survives a C boundary: a caller holding a borrow and calling back into the viewer is
//! the aliasing bug `viewer-qt` needed a `Busy` guard for (ADR 0246, section 3), and here nothing at all
//! would say so.
//!
//! So a command returns a `pdfv_events *` — an owned batch, read with indexed accessors and
//! released with `pdfv_events_free`. **The viewer's borrow ends before the caller sees anything**,
//! which is what makes re-entrancy a non-question rather than a rule. The same shape carries a
//! structured answer: `pdfv_outline` is owned, and it is owned *cheaply* because ADR 0247's second
//! amendment made [`Answer::Outline`](viewer_core::Answer::Outline) owned on the Rust side too.
//!
//! **A callback was the other candidate and is refused.** A `void (*)(const pdfv_event *, void *)`
//! saves an allocation and buys the caller a chance to call `pdfv_go_to_page` from inside the
//! dispatch of the events `pdfv_open` produced — which on the Rust side is `&mut Viewer` held
//! twice.
//!
//! # 3. A raster goes back through the request that asked for it
//!
//! [`Event::NeedsRender`](viewer_core::Event::NeedsRender) carries an `Arc<DisplayList>`, and a
//! display list is not a thing to put in a header: it is clause 8 and clause 9 in a data
//! structure, and a C caller has no use for it that is not "draw this", which is what the request
//! already means. So the request crosses as an **opaque handle** the caller may move to another
//! thread, and `pdfv_render_request_rasterise` draws it with `render-cpu` — this crate holds a
//! rasteriser for exactly that reason and for no other.
//!
//! The round trip is kept as a round trip rather than hidden inside `pdfv_open`, because the
//! whole point of `NeedsRender`/`RenderReady` is that the *host* decides where the work runs. A C
//! caller that wants the page drawn on a worker thread has the same lever `viewer-ui` has.
//!
//! `pdfv_render_ready_raster` takes the request back beside the raster, which is how the token
//! returns without being in the header at all: `RenderToken` is opaque in Rust and stays opaque
//! here.
//!
//! Pixels reach the caller by **copy into a buffer the caller owns** (`pdfv_frame_copy`). No
//! pointer into the viewer's memory is ever handed out, so there is no lifetime for a C program
//! to get wrong, and the cost is the one copy `doc/ui-boundary.md` prices tier 1 at — the same
//! copy `gdk::MemoryTexture` and `QImage` make, measured on both at 11.5 and 12.0 GB/s.
//!
//! # 4. A variant added later, and what it costs a compiled C caller
//!
//! This is the question the other two hosts never had to answer, and the honest answer has three
//! parts.
//!
//! **An event kind is a `uint32_t` a caller switches on**, and a kind it does not know cannot be
//! made to fail its build. So every event also answers `pdfv_events_describe` — one sentence,
//! for *every* kind including ones the caller has never heard of — so that an unknown event can
//! be logged rather than dropped in silence. That is trap 5 translated: the loud failure this
//! project prefers, in the only form C leaves available.
//!
//! **And the count is checkable at startup.** The header states `PDFV_EVENT_KIND_COUNT` as it was
//! when the caller was compiled; the library answers `pdfv_event_kind_count()` with what it has.
//! `pdfv_abi_check` compares them and the version, and a caller that runs it in `main` has
//! converted "fails to compile in every consumer" into "**fails to start, once, saying which
//! number moved**". It is weaker — it is a runtime check rather than a build failure, and a
//! caller may decline to make it — and it is the strongest thing available.
//!
//! **What it costs, stated plainly.** A new `Event` costs a compiled C caller *nothing* until it
//! meets one, and then costs it a `default:` arm that says what it does not understand. A new
//! `Command` costs it nothing. A changed *shape* — the kind of change ADR 0167 and ADR 0247 both
//! made, where a variant gains a field — costs it a new accessor function and leaves the old one
//! answering what it always answered. And a change to a **struct passed by value**, of which this
//! header has exactly two (`pdfv_geometry`, `pdfv_frame`), costs it a recompilation it has no
//! way of knowing it needs — which is why those two are small, are output-only, and are the
//! boundary's most expensive kind of change. `pdfv_abi_version` is what a caller checks before
//! believing either of them.
//!
//! # Errors, because a C caller cannot see a `Result`
//!
//! Every fallible entry point returns [`Status`] as an `int32_t`, and every out-parameter is
//! written only on `PDFV_OK`. `pdfv_status_message` turns one into a sentence. Nothing here
//! returns a sentinel value that has to be told apart from data, and nothing swallows a refusal:
//! a document that will not open is `Event::OpenFailed` carrying the reason `pdf-syntax` gave,
//! which is a *fact about the file* and reaches the caller through the same channel every other
//! fact does.
//!
//! # Where the `unsafe` is
//!
//! One module, [`abi`], and `tests/unsafe_position.rs` counts its tokens and asserts that
//! everything else in the crate is safe Rust. `doc/todo/30` reserved the permission for this
//! crate; `viewer-qt` took one token of it a round early with a test on its position (ADR 0246),
//! and this crate follows that precedent rather than inventing a second.
//!
//! Every entry point does the same three things and nothing else: turn raw arguments into safe
//! Rust values, call a safe function in one of the modules below, and write the result through
//! an out-parameter. **There is no PDF parsing behind an `unsafe`**: every crate that touches a
//! document's bytes keeps `#![forbid(unsafe_code)]`, unchanged, and this crate touches messages.

#![deny(unsafe_code)]

// The only place in this crate the permission is used, and `tests/unsafe_position.rs` is what
// keeps that true. Everything it calls is safe Rust in the modules beside it.
#[allow(unsafe_code)]
pub mod abi;

mod events;
mod kinds;
mod panels;
mod session;
mod status;

pub use events::Events;
pub use kinds::{EventKind, PageTargetKind, PixelFormat, ZoomKind};
pub use panels::Outline;
pub use session::{FrameInfo, Session, rasterise};
pub use status::Status;
