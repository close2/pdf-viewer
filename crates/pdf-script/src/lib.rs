//! A document's field scripts evaluated against a bounded host object model: RFC 0008's Tier 1.
//!
//! One responsibility: *evaluate a document's scripts against a host object model, and hand back
//! what they did*. A [`Request`] goes in — which of ISO 32000-2 §12.6.3's sites fired, or the
//! document's open, on which field, with the event's values, Table 221's `/JS` text and the fields
//! whose state changed since the realm last heard — and an [`Outcome`] comes out: the event's `rc`,
//! the value or change the script left, every edit it made to a field, every call it was refused by
//! name, the budget it exceeded if it exceeded one, and the lines it logged. Both are data and both
//! have a byte encoding ([`wire`]), because RFC 0008 section 6.2 puts the engine in a confined
//! process of its own and what crosses to it is bytes; nothing here holds a document or reads a
//! file. A [`Realm`] is one document's engine context, which persists across every request so that
//! what Table 32's name tree defines at the open is what a field's script calls later (ADR 1602).
//!
//! # The engine, and the `unsafe` this crate does not contain
//!
//! The ECMAScript engine is Boa (`boa_engine`), behind the `engine` feature, which is off by
//! default: RFC 0008 section 5.4's recommendation, accepted by the owner in `doc/questions/A193`.
//! This crate's own code forbids `unsafe`. Boa does not — no ECMAScript engine that exists is
//! written without a garbage collector in `unsafe` — and that is the dependency's, counted in ADR
//! 1590, in the position CLAUDE.md principle 3 gives every decoder of hostile input: a library
//! whose memory safety is the library's, contained by the process it runs in. A build without the
//! feature has the request, the outcome and the wire, and no engine to audit.
//!
//! # What a script reaches
//!
//! RFC 0008 section 4.2 admits a host object model that stays inside the process, and section 4.3
//! excludes the rest by name. This crate carries `event` with the properties each site raises, the
//! document's `getField` of any field and its field-walking methods, every field's value and the
//! properties that decide its appearance, the reference's `display`, `border` and `color`
//! constants, `console.println`, and the `AF*` library as the very functions Tier 0 runs
//! (`pdf_model::aform`) — ADRs 1591 and 1603. A script's write to a field is an edit in the outcome,
//! which the view state applies beside a person's typing. Every other member either table names is
//! a property that throws a `NotAllowedError` naming the call and its tier — never `undefined`,
//! never silence — and is recorded in the outcome whether or not the script catches it
//! ([`surface`] is the list).
//!
//! # A question to the person reading
//!
//! `app.alert` and `app.response` are the two calls RFC 0008 section 4.2 says need a host: a
//! realm hands a [`Question`] to an [`Asker`] and the script is answered with what it answers
//! ([`Answer`]). In the confined worker the asker is the wire and the script is held there until
//! the host has the person's answer; one question is put per run, and the rest are answered as a
//! closed dialogue answers and named (RFC 0008 section 6.8, ADRs 1627, 1628).
//!
//! # The reference the object model is read from
//!
//! Adobe's *JavaScript for Acrobat API Reference*, read at `adobe/dc-acrobat-sdk-docs` commit
//! `ab3b42a7` — the owner's working source for the API (`doc/questions/A193`, answer 7) — on its
//! "event properties", "Field properties", "Doc methods" and "app methods" pages, cited by name and
//! never quoted (`doc/third-party-data.md`).
//!
//! # Budgets
//!
//! [`Budget`] is every ceiling a run is held to, each a number with its reason: wall time, the
//! engine's step count, its loop-iteration, recursion and stack limits, how deep a script may nest
//! before it is parsed ([`depth`]), and the sizes an argument can ask a built-in to allocate. A run that exceeds one stops, changes nothing, and says which
//! ([`Exceeded`]). What the in-process budgets cannot bound is the process's, and ADR 1590 names
//! it.

#![forbid(unsafe_code)]

pub mod depth;
mod outcome;
mod question;
mod request;
pub mod surface;
pub mod viewer;
pub mod wire;

#[cfg(feature = "engine")]
mod engine;

pub use outcome::{Ending, Exceeded, Outcome, Refusal, RefusalKind};
pub use question::{Answer, Asker, Button, Buttons, Icon, Nobody, Question};
pub use request::{Budget, Event, Request, utf16_offset};

#[cfg(feature = "engine")]
pub use engine::{Engine, Realm, run};
