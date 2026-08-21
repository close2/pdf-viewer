# 634 — Two rounds that answered one question from both ends

Fourth merge round, four branches, and the first where two rounds solved **the same problem
independently and the merge had to keep both halves**. 618 found nothing; 623 found a claim that did
not survive and mis-attributed it; 629 had to decide two things; this one had to *compose* two.

## What was merged

`round-630`, `round-631`, `round-632`, `round-633`, branched from `489ccf93`. Four conflicts in
`round-632` and `round-631`, none in the other two.

## The one that mattered: answer 3, twice

Sessions 631 and 633 both took §8.9.7 — an inline image whose data is filtered and states no `/L`,
whose end was **searched for** as a token rather than derived. Neither knew the other was on it.

- **633** ran the **first filter of the chain** and asked where it stopped
  (`Document::filtered_extent` over `Engine::consumed`). That answers `FlateDecode` and
  `LZWDecode` — the two filters with a resumable decoder in this tree.
- **631** read §7.4.2's GREATER-THAN SIGN and §7.4.3's `(7Eh)(3Eh)` **straight out of the data**,
  which needs no decoder at all and answers `ASCII85Decode` and `ASCIIHexDecode`. Neither encoding
  can contain its own marker, and the clause's own EXAMPLE ends `…2HCqC~> EI`.

**They are complementary and each covers what the other cannot**, which Table 5 decides: the *first*
filter of the chain is the one asked, so `/F /FlateDecode` is answered only by the decoder route and
`/F [/A85 /Fl]` — 631's own witness, `5097148.pdf` — only by the marker. 633's own owed-list asked
for precisely what 631 had built: *"three of the five need no decoder at all."*

So the merge kept both. Textually only the module's doc block conflicted; the code hunks merged
silently and then **did not compile** — 631's branch called a `check` closure that 633's rewrite had
replaced with a three-way `terminator_at`. The bridge is exact rather than a guess: 631's two-way
`check` is 633's three-way answer with two cases collapsed, and the third case is one 631 already
handles by another name — a marker whose `EI` a window cannot reach is a window too short, which is
the same answer its derived lengths give.

**The merge is what proved it.** `fixed_documents` — session 624's file, built for exactly this —
now holds **29 rows** from sessions 603, 613, 615, 619, 621, 631 and 633, and passes with 0 absent.
Both rounds' witnesses are in it, so a composition that had lost either half would have said so by
name. That is the mechanism working the first time it could.

## The other three conflicts

- **`doc/checks/fixed-documents.toml`** — both appended; union, 29 rows.
- **`doc/todo/03`** — both wrote a section 22; 633's keeps the number, 631's becomes §23.
- **`doc/conformance/ledger.toml`, §8.7.3** — 630 fixed a defect there (a glyph stroked in a tiling
  pattern was reported on the *path* route and not the *text* one, so §9.3.6's rendering modes 1, 2,
  5 and 6 outlined a glyph in the last solid colour **silently**), while 632 read the same row as a
  confirmation and re-derived why it stays `partial` (no crate that builds a display list names a
  stroke expander; tiling the outline would mean computing it a fourth time in the one crate whose
  point is that it does not). **Both notes survive**, spliced at their common prefix — one is a
  defect and the other is the argument for the remaining debt, and dropping either would have been
  the unrecoverable mistake `CLAUDE.md` names.
- **§8.9.7's row** — the same shape, and merged the same way.

## The sequence, whole, on a quiet machine

| | |
|---|---|
| `fmt`, `clippy --workspace --all-targets` under `-D warnings` | clean, silent |
| `nextest --workspace` | **2333 passed, 17 skipped** |
| doctests, `-p conformance` | clean (157 + 5 + 1) |
| corpus | 974 documents, 68 incomplete |
| oracle | 1794 pages — 907 agrees, 66 contradicted, 786 ambiguous |
| `render-quorra` | 957 pages — 932 agree, 23 differ, 2 refused |
| **`fixed_documents`** | **29 checked, 0 absent** |
| text extraction, both censuses, dates, XMP, JPEG 2000 | clean |
| `cargo deny check` | advisories, bans, licenses, sources ok |

**The ledger now carries a `silent` row** — 875 rows at 437 implemented, 221 `partial`, 17
`reported`, **1 `silent`**, 78 inapplicable, 8 writer-side, 113 out-of-scope. §11.7.5.2, put there
by 632, is the first: a requirement this program fails without saying so, which is the status
`doc/HANDOVER.md` calls the one worth hunting. Its population is measured — one corpus document
states a real `/TR`, and it draws fully opaque, so no page is wrong today — and the work is priced
in `doc/todo/13` rather than guessed at.

## Housekeeping the round did

`/home/AI/cargo-target/pdf-viewer/release/examples/pdf-sandbox-worker` — the stale binary session
624 found `worker_program()` preferring over the one above it — is deleted again, because §5's
release build recreates the directory it lives in. §5's seven artefacts are rebuilt and installed.

## Owed

- **CI**: 630's two fixes have no run behind them, and cannot until `main` is pushed — the token in
  the tree is read-only for contents. The owner's last push reproduced both failures, which is
  expected: the fixes were on a branch.
- **The owner's own session**: whether `tmp/pi.pdf` launches repeatedly now (628).
- **Lead 2 of `doc/todo/37`**, which 633 declined to claim without a trace it could not take on a
  loaded machine, with the two refusals a quiet round should look for named.
