# 1165 — The document a level decides to open, and the order its files stand in

The batch's host-UI round. Two ledger rows moved and one small debt from the last batch paid.

## §12.11.6 — `partial` → `implemented`

§12.11.6 says that where a document's unmet requirements pass §12.11.3's penalty threshold "the
processing of the document shall not continue". The number was computed in `pdf-model` and attached
to no level — the one restriction a reader could not turn on, be asked about or be warned of. It is
now `restriction::Operation::Process`, with `Restriction::RequirementsUnmet` as its reason, asked
in `Viewer::open` before anything else opening a document does — so `on` raises no `Event::Opened`
at all, `ask` holds the whole open until `Command::Answer`, `warn` opens it and says so
afterwards, and `off` stays the default. ADR 1167 has the argument and the three readings the arm
needed: Table 22 states no position for it, Table 257 says nothing about it, and NOTE 1 keeps an
unmet requirement from refusing a verb. A declined *open* has something to undo, so `answer`
forgets the document rather than leaving `viewer-core` holding one nobody can see or close.

## §12.3.5, §12.3.5.1 — `/Sort` obeyed, and the C ABI's rule restated

The owner's revisit note on ADR 0576 argues that "a look belongs to the platform" cannot separate
the entries that cross the C ABI from the four dropped, since the same ADR carries Table 29's look
entries. Checked: that holds. Its second claim is right in substance and wrong in detail — the
panel `/Sort` and `/Navigator` describe is the files panel, which reached the native hosts in ADR
0711, *after* 0576 rather than five sessions before, and not ADR 0564's thumbnails.

Stated instead: an entry crosses when it decides something about the document's own content no
caller can derive, and stays out when it dictates the appearance of a surface the platform owns.
`/Sort` and §12.3.6's `/Layout` cross now; `/Colors` and `/Split` do not, and
`unsupported_presentation` says so out loud. `collection::sorted_keys` is the ordering, computed
once below the hosts because two of Table 155's three item subtypes read `/CI`, which only a
`Document` reaches. Three things Table 156 leaves open are decided as decisions. ADR 1168.

## Owed by the last batch, and the gates

`viewer_host::modification::before` is the one statement of "read the clock on the way to a save",
and all three windows use it now — the two native hosts had no such command at all, so their saves
carried no Table 166 `/M`. rustfmt and clippy clean on this round's files, `cargo nextest run`
green on every crate touched, `cargo test -p conformance` green but for a sibling's in-flight
`content/overprint.rs`, and `launch_path` inside its bands (26 figures, 0 outside).
