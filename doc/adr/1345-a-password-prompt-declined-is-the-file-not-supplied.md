# 1345 — A password prompt declined is the file not supplied

Session 1254. Status: **accepted** and built.
Context: `crates/viewer-core/src/viewer.rs` (`supply`, `withdraw`), `crates/viewer-core/src/interact.rs`
(`Locked::purpose`, `withdraw`), `crates/viewer-host/src/documents.rs` (`Arrivals::offer`,
`cancel`, `declined`), the three windows' `cancelled` and `open_the_next`,
`crates/viewer-ffi/include/quorra.h` (`quorra_supply`, `quorra_event_needs_file_beside`).
Amends: ADR 1335 section 3 (the cost it left). Clauses: ISO 32000-2 §7.6.4.1, §12.6.4.3,
§12.6.4.7, §12.7.8.3.3 (Table 253).

## 1. What was left

ADR 1335 held a file a document named — Table 203's, Table 209's or Table 253's `/F` — while §7.6.4.1's
prompt was up, and retried it with `Command::Open`. A prompt a person cancelled told the core
nothing: the hold stayed until the next open of any name, and a §12.7.8 import's references into the
file stayed waiting, unsaid, until a later import asked for the same file.

## 2. What a reader owes on a cancel

§7.6.4.1 says the processor "should prompt for a password" and says nothing of a prompt declined;
§12.7.8.3.3's Table 253 names "[t]he file containing the named page" and nothing about a file that
cannot be read. So the reading is the one the tree already has for a file no host would give: the
file is not supplied. The references are declined with a sentence each, each widget keeps the
appearance its own dictionary states, a template page is not added, and a jump declines. The reason
is said first — the password was not given — because "was not supplied" alone would be untrue of a
file the person chose and could not open.

## 3. Decisions

- **No new message.** A decline is `Command::Supply` with no bytes, under the purpose the file was
  supplied for. Where the core holds a file for that purpose from the document in front, the decline
  is about the held file: the hold ends, the sentence is said, and a name `Command::Beside` held out
  is given back with `Event::Closed`, as after a password that was right. A C host does the same with
  `quorra_supply(purpose, NULL)`, and the confined wire already carries it.
- **The windows send it outside the pump.** A prompt runs out of attempts inside the events of the
  command that raised it, so `Arrivals::cancel` keeps the purpose and `Arrivals::declined` hands the
  command over where each window already starts its next arrival, outside that loop. Only an
  arrival made by `Arrivals::locked` carries a purpose, so a cancelled prompt about a document a
  reader named declines nothing.
- **`beside` crosses the C ABI** as `quorra_event_needs_file_beside`, in `quorra_event_dirty`'s shape.

## 4. Costs

A decline arriving while the document in front is not the one that asked leaves the hold as before;
the three windows' prompts are modal, so no window can produce that order.
