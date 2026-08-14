# 0324 — The second sweep becomes a program, and a split tests the instruments

Status: accepted (session 489)

## Context

`doc/todo/01` binds a sweep round to commit one prose sweep as a program before running any of
them. Thirteen of the fifteen sweeps were still descriptions, and a description is what let the
fifteenth go unrun for twenty-four rounds and then be rebuilt from its own paragraph (ADR 0319) —
a reconstruction is not the same instrument twice. Separately, three rounds this block were pure
motion: `content.rs` became `content/` with the root kept, `pdf-font`'s `lib.rs` became modules,
and `viewer-ui`'s binary became modules — the first and third deliberately keeping the old path
valid so that citations survive.

## Decision 1: sweep 2 is `conformance --bin unread`

The second sweep — every `/Key` in a claim of unreadness, grepped against the tree — is now
`tools/conformance/src/unread.rs` and `cargo run --release -p conformance --bin unread`. It was
chosen over the other twelve because it is among the most-run, its false-positive shapes are the
best documented (a note quoting its own retired wording; one short key, three clauses), and the
crate already held everything it needs: the ledger parser and the source walk.

Three choices shape it, each from a paid-for lesson:

- **The claim is matched by shape, not wording** — `not read`, `unread`, `read by no`,
  `read nowhere`, `nobody reads`, `reads nothing`, `never read`, `none of which is read` —
  because §8.11.2.1's "read by nothing" hid from a grep that knew only "Not read:".
- **Sentence-scoped key extraction**, because a note is corrected by appending and holds its own
  history; a key three sentences from the claim is usually the subject of a different statement.
  The cost runs the safe way: a key dropped by an early sentence split is a claim left for the
  by-hand read, not a false hit.
- **Reading means the quoted-string form** — `"FS"`, the shape a lookup takes — because a `/FS`
  in a comment is *naming* the entry, which is what the note already does. A key quoted by a file
  in the row's own `code = [...]` is the sharpest hit there is, and the program says which files
  quote each key so the one-short-key noise is settled by the path.

It is a reading list and not a gate, for ADR 0249's ratio reason: the noise population does not
shrink under a tighter grep, because the same short key legitimately belongs to several clauses.

**First run: 62 rows claim an entry unread, 171 keys — 55 confirmed, 116 quoted somewhere over 49
rows, 53 by the row's own code.** One defect: §7.5.5's "`/Info` is unread because §14.3.3
deprecates it", standing while `metadata::Information::read` takes the entry off the trailer for
the properties panel. The disposal-by-deprecation is worth naming as a shape: deprecated does not
mean unread, and the row treated the first as implying the second.

## Decision 2: a `code` path that is a module root covers its module

The entries sweep's first run at this head printed 140 entries over 43 rows — up from 102 over 41
at its first committed run — and 34 of the difference was the splits: rows naming
`crates/pdf-model/src/content.rs` or `crates/viewer-ui/src/bin/pdf-viewer.rs` read as "named only
elsewhere" for keys whose readers had moved into `content/` and `pdf-viewer/`, with nothing in
the ledger or the readers changed. The split commits kept those roots *so that citations of the
path stay valid*; an instrument reading the path as one file was measuring the split, not the
tree.

`entries::covered_by` now applies the rule Rust's module system already states — a module root
`…/foo.rs` owns everything under `…/foo/` — and `unread` shares it. `content_stream.rs` is not
under `content.rs`, which the test pins. After the correction: 106 entries over 42 rows. The
alternative — repointing every `code` array at the new submodules, as the `pdf-font` split did
for its 114 ledger lines — remains available row by row, but `lib.rs` had to be repointed because
a crate root's siblings are not its directory; `content.rs`'s are, and an instrument that
understands that keeps the split's own promise.

## What the round's sweeps found (headline)

- **Sweep 9's block**: ten wrong table numbers, all one shape — an entry attributed to the table
  its *value* points at, or the neighbouring dictionary's: `/Configs` to 99 (it is 98's, twice),
  `/Contents` to 172 (166's, five places), `/SV` to 237 (235's), `/StructTreeRoot` to 354 and
  `/DPartRoot` to 408 (both Table 29's), `/N` to 168 (170's, twice), "`/DocMDP` level" for
  Table 257's `/P`, `/DecodeParms` to 11 (Table 5's). Two ADRs carried retired numbers and were
  amended in the same commit (0284, 0295), per ADR 0265's rule.
- **Sweep 4 on the noun `silent`**: §14.12.4.1 and §14.13.8 each called a neighbour "`silent`"
  in a ledger that has had none since Annex O was built; §14.8.6 used the status word for a
  requirement addressed to a document. All three reworded to say what they mean.
- **The blame list's next band** (commits 138–165, seventeen rows) — six wrong, the sharpest
  §14.8.2's "needs a consumer" surviving §14.8.2.5 going `implemented` with three consumers,
  and §7.7.4 owing `/Pages` and `/Templates` to §12.7.7 after `named_page.rs` landed.

## Decision 3: the catalog's associated files reach the list

§14.13.3 read `implemented` on `attachment::associated`, and that function had no caller outside
its own tests — ADR 0295's shape one family over: a document that associated a payload with the
catalog and filed it under no name carried a file no panel could list and no host could extract.
`attachments` now appends the catalog's `/AF` files to §7.7.4's tree, deduplicated by embedded
stream (`Arc` identity, which holds because the document caches resolved objects), because a
PDF/A-3 writer states one payload both ways and that is one file, not two. The structure-element
and `DPart` `/AF` sites are deliberately not folded in: each needs a tree walk, each has a row
naming that cost, and a list is not a walk.

## Consequences

- Three of the fifteen sweeps are commands; twelve remain, one per sweep round.
- The unread sweep's numbers are comparable from now on, which no by-hand run's ever were.
- A future split that keeps a module root needs no ledger repointing for the sweeps to stay
  honest; one that does not (a `lib.rs`) still does.
