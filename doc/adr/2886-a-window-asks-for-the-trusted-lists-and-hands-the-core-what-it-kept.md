# 2886 — A window asks for the trusted lists, and hands the core what it kept

Session 2035, a signatures round (`doc/questions/A800` item 3, `doc/todo/77` item 4). Status:
implemented. Rows: §12.8.3.4.4, §12.8.3.4.5. Context: ADRs 1180, 1291, 1728, 1738, 2105, 2309, 2834,
2882; `doc/questions/A98`; ETSI TS 119 615 V1.3.1 clauses 4.1 and 4.2 and GPR-4.0-03, cited by clause
and paraphrased (ADR 1085).

## Context

ADR 2882 built the refresh and the host's port, `viewer_host::trusted_list_fetch`, and no window
called it. ADR 1738 is how the other fetch a signature makes reached the windows: the level asked
once in `native_window::requests` for the three windows on the shared trait, `quorra`'s own copy in
its dispatch, a `Submitter` thread looked for on the timer, the answer handed back to the core. A
document asks for a policy's copy; nothing a document says asks for the lists, so this one is a key.

## Decision

1. **Shift + T is `WindowAct::TrustedLists`**, on the letter the lists are named after and on
   Shift for ADR 1180's reason: `t` alone is §12.5.6.6's free text and keeps it, and no clause
   names a key. It is kept while a presentation runs, because a question it raises is held until
   the presentation ends (ADR 2309) and at `send` nothing is shown at all.
2. **A window act that ends in a command**, `OpenDocument`'s shape: the level, the question and
   the thread are the host's, and what crosses is `Command::Trust`. The C ABI numbers it
   `QUORRA_ACT_TRUSTED_LISTS` 23 (the count 24) and carries it out inside the library, as it
   carries out `Archive`: a C caller is owed the question or the sentences, never the act, so the
   WinUI host needs only the constant.
3. **The level is ADR 2882's**: `may_fetch_trusted_lists` at `Submissions`, refused at every level
   where no `--trusted-lists` directory was named. At `ask` the question is
   `asked_to_fetch_trusted_lists`, which names the directory, under `Subject::TrustedLists`; asked
   while another question waits it is declined with that reason, as ADR 1738's is. A `no` and a
   refusal are said with "the lists held are as they were".
4. **The refresh and the reading again are on one thread of their own** (`Refresher`,
   `fetch-trusted-lists`), never the event thread and never the confined worker: the GETs are
   `submit::fetch`'s, the persisting is ADR 2882's, and both directories are then read again
   through `reader::Trusting` — the three reader words a window now keeps past its first document
   — so the policy that crosses is the whole of `Command::Trust`, anchors and acceptance as the
   command line gave them and the lists as the directory now holds them. The core's existing arm
   forgets every report worded under the old policy (ADR 1044). The instant is this machine's
   clock, as a policy without anchors already takes it.
5. **One refresh at a time**: two would write one directory and rename over each other's files.

## The tests

`viewer-host/tests/trusted_list_fetch.rs` serves lists `pdf-signature`'s own generator signed with
its own test keys (`tests/trusted_lists/`, no third-party data): a refresh hands the core the LOTL
and both Member States' lists the reader did not hold, a `BE` list its pointer's certificate did not
sign (PRO-4.2.4-07) is not handed over while `AT`'s is, and a second refresh and one without a
directory are declined. `viewer-ffi`'s `window::trusted_list_tests` press Shift + T on a real window
at each level against a loopback listener: `refuse` sends nothing and says why, no directory is
refused at `send`, `ask` owes one question naming the directory and a `no` starts no thread, a yes
and `send` each make PRO-4.1.4-01's GET to `OJEU-LOTL-Loc` and say its 404. Planted — the policy
read before the refresh rather than after, and `ask` fetching without the question — four fail.

## GPR-4.0-03, read again

Its *should* asks a procedure to go on past a failed `https` validation with a warning. The
recorded refusal holds: the client (`ureq` over `rustls`, `doc/questions/A130`) has no unvalidated
mode, and adding one is a change to the one network client principle 3 names, not effort; a failed
connection is said as the location not fetched, and the held copy stays.

## Consequences

- The verdicts a window shows are worded from the lists it fetched, without a restart.
- A `quorra_window` fetches into the `--trusted-lists` directory its caller's words named, and
  refuses where they named none. `quorra_trusted_lists` is a session's, which hands lists as bytes
  and which no window act reaches, so a C host holding its lists that way fetches them itself, as
  ADR 2879 has it.
