# 2936 — A policy's referenced lists are asked of the core, named in the question, and given to the refresh

Session 2061, a signatures round (`doc/questions/A800` item 3, `doc/todo/77` section 2 item 4).
Status: implemented. Rows: §12.8.3.4.4. Context: ADRs 0247, 1728, 1738, 2580, 2624, 2874, 2882,
2886, 2932; `doc/questions/A98`; `doc/ui-boundary.md`; ETSI TS 119 172-2 V1.1.1 clause 4.22.2.1 b)
and TS 119 172-3 clause 4.22.2.2, cited by clause and paraphrased (ADR 1085).

## Context

ADR 2932 gave `trusted_list::refresh::refresh_with` the references a policy's `TAsInTrustedList`s
make, and no host had any to give: the policy document is read and bound inside `viewer-core`, and
the window that refreshes the lists (ADR 2886) knew only the directory.

## Decision

1. **`PolicyDocument::trusted_list_references`**, on TS 119 172-2's and TS 119 172-3's documents
   (one walk over their shared `ScopedRule`s): the `TLReference` (`uri`) of every
   `TAsInTrustedList` in clause 4.24's, 4.25's and 4.26's trust trees, in every scope and commitment
   rule, in order and each once. TR 102 272's trust trees name certificates only.
2. **Only a bound copy's references count.** `Enforcement::Enforced` carries them, so they exist
   exactly where the constraints were enforced: a store or a server is not the signer, and a copy
   neither digest binds is nobody's agreement (ADRs 2580, 2624).
3. **A question, not an event: `Query::TrustedListReferences`**, answered from the focused
   document's report with `Answer::TrustedListReferences(&[String])`. It passes the boundary's test
   (a host holds neither the policy nor its binding). An event was weighed and refused: a copy a
   host fetches later adds references with no second event to carry them, and a window needs the
   locations only at the moment the person asks for the lists. It crosses `viewer-confined`'s
   protocol as query and answer 41.
4. **The window asks once, naming each location.** `requests::trusted_lists`, which GTK, Qt and the
   C window share, asks the core, and `may_fetch_trusted_lists` and `asked_to_fetch_trusted_lists`
   take the references: the question adds the count, the clause, and every location, ADR 1728's
   shape for a location a document names. `Asked::TrustedLists` carries what was named, so a yes
   fetches exactly that. The level is the reader's `Submissions`; at `send` and `warn` there is no
   question, as for every fetch under it. The document adds locations to a refresh the person asked
   for and never starts one (principle 3).
5. **`Refresher::start` and `start_with` take the references**, and `refresh_trusted_lists` calls
   `refresh_with`, so what was kept is read again and crosses as `Command::Trust`, as before.
   `quorra`'s own copy in its dispatch (ADR 1738's) does the same.

## Tests

`pdf-signature`: the references of two rules, in order and once, and none without a reference.
`viewer-core`: an open document answers an empty list, then the reference once a bound copy is
handed in; the same copy with one octet added answers nothing, and no document answers `None`.
`viewer-confined`: the query round-trips and the answer keeps every location in order.
`viewer-host`: the question names the location and its clause only when given one, and a refresh
given it asks for it last, keeps `AT`'s list served there under its name and hands it to the core,
while one given none never asks.

## Consequences

The reference reaches a window only through a report the core has worded; a document that is not
focused is not asked about. No window-level test drives a signed document with such a policy through
Shift + T: the fixture is `viewer-core`'s, and the three pieces are tested where they live.
