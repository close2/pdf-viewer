# 1332 — A password is asked about the document it would open, and retried from that document's bytes

Session 1247. Status: **accepted** and built.
Context: `crates/viewer-core/src/interact.rs` (`resume_remote`, `Locked`, `place_remote`,
`Outcome::locked`), `crates/viewer-core/src/viewer.rs` (`Viewer::locked`, `open`, `apply`),
`crates/viewer-core/src/open.rs` (`Open::new` takes the bounds), `crates/viewer-host/src/documents.rs`
(`Arrivals::offer`, `Arrivals::locked`, `Arrivals::settle`, `may_ask_about`, `not_asked`), the three
windows' `offer_a_name`, `opened_beside`, password prompts and retries.
Builds: ADR 1263 (`Command::Beside`), ADR 1275 (`Arrivals`), ADR 1316 (`Arriving::bytes`).
Clauses: ISO 32000-2 §7.6.4.1, §12.6.4.3 (Table 203).

## 1. What was wrong

A file that a §12.6.4.3 action names and that wants a password was never prompted for. The core
opened it with the empty user password and declined the link ("cannot read"), even though
§7.6.4.1 says "the interactive PDF processor should prompt for a password". Behind that sat the
defect round 1239 found by reading. Each window's second attempt for any document that was not
arriving went to `open_document` (GTK, Qt), or to the front's path (`quorra`). So a prompt about
any other name would have reopened the document in front under it.

## 2. One route for every document that asks

The core holds the jump (`Locked`: the action and the source's bounds) and raises
`Event::PasswordRequired` under the name the document would open under. With `/NewWindow true`
that is the name the host reserved; otherwise it is the document the link was in, which Table 203's
`/NewWindow false` "replaces". The retry is `Command::Open` under that name. When it succeeds,
`place_remote` puts the document at the action's page, which is the same function the jump without
a prompt uses. Any other open ends the hold.

On the host side, the reservation moved into `Arrivals`. `offer` keeps the file beside the name.
`locked` turns it into the arriving document under the name the core asked about, and `settle`
gives it its tab when it opens without a prompt. So a GoToR target and §O.2.1's held bytes take one
path: `Arriving::bytes` for the second attempt. `may_ask_about` is the one rule for which documents
a prompt may be put about, and the first document is the only one retried from the window's own
fields.

## 3. Costs

A replacement after a prompt relabels the tab and replaces the window's per-document fields. A
replacement without a prompt still leaves the tab's label and path as they were. That is unchanged,
and it is the next round's to route the same way.
