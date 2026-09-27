# 1335 — Every file a document names is asked about, retried and replaced by one route

Session 1249. Status: **accepted** and built.
Context: `crates/viewer-core/src/interact.rs` (`Locked`, `Held`, `place_threaded`,
`named_pages_from`), `crates/viewer-core/src/viewer.rs` (`Hold`, `hold`, `supply_held`,
`ask_for_files`, `apply_behind`), `crates/viewer-core/src/open.rs` (`Open::read`),
`crates/viewer-core/src/event.rs` (`Event::NeedsFile::beside`), `crates/viewer-host/src/documents.rs`
(`Arrivals::settle`, `Arrivals::supplied`), `crates/viewer-host/src/policy.rs` (`remote`,
`asked_to_open_remote`), the three windows' `run`/`pump` and `Event::Closed` arms,
`crates/viewer-confined/src/protocol.rs`, `crates/viewer-ffi/include/quorra.h`.
Amends: ADR 1332 section 3 (the cost it left). Builds: ADRs 1227, 1239, 1263, 1275.
Clauses: ISO 32000-2 §7.6.4.1, §12.6.4.3 (Table 203), §12.6.4.7 (Table 209), §12.7.8.3.3 (Table 253).

## 1. What was left

ADR 1332 gave §12.6.4.3's file a password prompt and left three things. A replacement made without
a prompt kept the tab's label and the window's path, because `Arrivals::settle` knew only the name
the host offered and Table 203's `/NewWindow false` opens under the *source's* name. §12.6.4.7's and
§12.7.8's files still declined on a password. And the remote-documents question said "in place of
the one you are reading" for a link that opens beside it.

## 2. Decisions

- **One hold for three clauses.** `interact::Locked` holds a `Held::Remote`, `Held::Thread` or
  `Held::NamedPage`, and the retry is `Command::Open` under the name the prompt was raised under.
  A jump's file becomes the document; Table 209 states no `/NewWindow`, so a thread's replaces the
  source, which is this reader's preference and the same as Table 203's `false`.
- **A named page opens under no name.** It is asked about under the name the host offered with
  `Command::Beside` (under the source where none was offered). The retry reads the file with
  `Open::read`, draws it into the document that asked, and answers `Event::Closed` for the offered
  name, so a host holding it for the prompt lets it go. Nothing new was added to the vocabulary for
  this; `Closed` says "nothing is open under this name", and that is true.
- **The offer settles under either name, and is spent by its own supply.** `settle` takes the
  offered name or the name of the document the offer came from. Each window calls
  `Arrivals::supplied` once a `Command::Supply`'s events have been seen. Without that, an offer
  whose jump was declined would name the tab's next replacement, which could be a §12.6.4.4
  embedded document.
- **The question carries `/NewWindow`.** `Event::NeedsFile` gained `beside` (Table 203's and Table
  204's `true`, `false` for every other purpose). Every window offers a name for each supplied file,
  so `true` opens beside. The confined wire carries it as one bit. The C ABI does not expose it
  yet: a C host has no question to word, and `quorra_supply`'s comment now states the retry rule.

## 3. Costs

A named page whose prompt a person cancels leaves its references waiting and unsaid until the next
import asks for the file again. No recent-file list exists in any window, so there was nothing to
make follow the replacement.
