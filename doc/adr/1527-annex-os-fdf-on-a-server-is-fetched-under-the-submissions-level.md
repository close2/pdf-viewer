# 1527 — Annex O's `fdf` on a server is fetched under the `Submissions` level, and arrives by document

Status: accepted. Session 1346. Builds on ADRs 0357, 1291, 1327 and 1523; carries out the design
`doc/todo/39` wrote, with one departure from it (section 3).
Context: ISO 32000-2 Table Annex O.4 (§O.2.2), §12.7.6.2, §12.7.6.4, §12.7.8; `CLAUDE.md`
principle 3 (the four levels, and the one place a host supplies the policy).
Code: `crates/viewer-host/src/policy.rs` (`import_is_fetched`, `may_fetch_import`,
`asked_to_fetch`, `fetch_note`, `SUBMISSIONS`), `crates/viewer-host/src/submit.rs` (`fetch`,
`fetched`, `Asked`, `Submitter::fetch`), `viewer_core::Answered` on `Command::Respond`; the
`Event::NeedsFile` arm of all three windows and of `quorra-confined`.
Tests: `crates/viewer-host/tests/fetch_import.rs`; the drive's `31-fragment-fdf`.

## 1. What the clause asks, and which act it is

Table Annex O.4's `fdf`: "Open the document and then import the data from the specified FDF or XFDF
file. The URI shall be either a relative or absolute URI to an FDF or XFDF file." The relative case
was built (ADR 0357); an absolute URI was refused out loud. Fetching one is a GET that tells a
server this document was opened here, which is §12.7.6.2's act class, a machine contacting a server
on a document's word. So **the level is `Submissions`'**, read once in `may_fetch_import`, and no
fifth level is invented. The act's menu name becomes "sending a form or fetching its data" so the
one row says both. `--submissions=` starts each window at a level, as `--embedded-documents=` does.

## 2. The order and the bound

`may_submit`'s order, through one shared helper: no scheme, a scheme outside `SUBMIT_SCHEMES` and a
URL `submit::check_url` refuses are refused at every level, before anything reaches the TLS stack
(ADR 1327); only then `refuse` (said), `ask` (the URL put to the person), `warn` and `send`. The GET
is the submit client's: `submit::TIMEOUT`, no redirect followed, and the body read to at most
`submit::RESPONSE_LIMIT` (64 MiB). A larger one is refused by name, not truncated. An FDF that large
is no form's data, but the bound is the client's single bound and not a second number. The format is
`data_format`'s reading of the name, never the media type. Only a 2xx is imported, and a body that
is not the format its name says is reported by the import that reads it. Where the name came from is
not consulted: the command line, an `ef` remainder and an import-data action naming a URL all reach
a host as the same `Event::NeedsFile`.

## 3. The departure: `Respond`, not `Supply`

The design said the bytes cross as `Command::Supply(ImportData)`. `Supply` answers the document in
front, and a fetch answers when the server does. A person who changed tabs meanwhile would have the
data imported into the wrong document, or dropped in silence, because `interact::import` returns
nothing when no import is pending. `Command::Respond` is already named by document for exactly this
reason (ADR 1291). It gains `answers: Answered`. With `Submission` its sentences say
"submit-form answer"; with `Import` they say "import-data", the same words the relative route says,
so the parameter's report reads one way whichever route its file took. A refusal, made while the
asking document is still in front, is still `Supply { bytes: None }`, so the core says that the
import was declined. `quorra-confined` has no network and refuses at every level with its own
sentence.
