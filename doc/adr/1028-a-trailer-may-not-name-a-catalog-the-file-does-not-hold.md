# 1028 — A trailer may not name a catalog the file does not hold

Session 1009. Status: **accepted**. Takes ADR 1024 §5's crasher — the first finding of a fuzz target
that had never run — diagnoses it against §7.5.5's Table 15, fixes it in the serializer, and runs the
target properly. Prices, and does not take, the third extraction ADR 1020 §2 hands on.

Context: `crates/pdf-syntax/src/serialize.rs`, `crates/pdf-syntax/src/document.rs`,
`crates/pdf-syntax/tests/serialize.rs`, `doc/conformance/ledger.toml` (§7.3.10, §7.5.5),
`fuzz/corpus/serialize`. ADRs 0817, 0842, 0852, 1020, 1024. RFC 0002 §10, §11.3.

`§N` is ISO 32000-2 and nothing else. Every figure below was produced in this session by the
command beside it.

## 1. The defect is the writer's, and the clause says so in its type column

The 184 bytes are ADR 1024 §5's and they reproduce exactly. What they are: a file with no `endobj`,
no `xref` and an unterminated stream, whose trailer states `/Root 1 0 R`. Nothing closes object 1's
dictionary — the bytes run from `1 0 obj <<` straight into `2 0 obj` — so the scan that recovers
this document finds no object 1 at all. `Document::get(1 0)` answers `Null`, and
`Document::catalog` answers `TrailerMissing { key: "/Root (not a dictionary)" }` **about the input,
before anything is written**.

So the two candidates ADR 1024 §5 named resolve one way:

- **Not the recovery's.** The trailer's `/Root 1 0 R` is what the file's own bytes state, and
  restating it is the only honest thing a recovery can do with it. That the object behind it is
  unreadable is a fact the *reader* reports, typed, at the moment a caller asks for a catalog —
  which is where `CLAUDE.md` principle 1 wants it. `Document::open` deliberately does not demand a
  catalog; §7.5.5's row for this clause records the one case where it does rebuild on `/Root`, and
  it is the other one (a table that leads nowhere, disproved by the file itself).
- **The writer's.** Table 15 states `/Root` three ways and the serializer read only two of them:
  the entry is required, it "shall be an indirect reference", and its **Type** column says
  `dictionary`. `SerializeError::NoRoot` covered the first. Nothing covered the third, so an
  assembly that *named* a root was written whatever the root turned out to be — a trailer naming
  object 1, above an object 1 written as `null`, in a file this project had put its name on. RFC
  0002 §11.3 is what that costs: "A malformed output is this project's defect in a way a misrendered
  page never was."

## 2. Where the fix went, and the second road to the same null

`serialize` refuses by name — `SerializeError::RootNotADictionary` — **before a byte reaches the
caller's sink**, which the regression test asserts because every other refusal in this function
arrives after part of a file has been written.

The refusal has two roads into it and they are one clause apart, which is why they are one variant:

- the root's slot is written as something that is not a dictionary (the 184-byte case), and
- the root names a number the assembly has no slot for, which §7.3.10 makes "a reference to the
  null object" — the same null, named from the other end.

**What the check follows is a chain, not one object**, and that is the part that is a reading rather
than a test. §7.3.10 resolves a reference to a reference, `Document::resolve` does so up to
`MAX_REFERENCE_DEPTH`, and `Document::catalog` therefore does too — so a file whose `/Root` names an
object holding `2 0 R`, where object 2 is the catalog, is a file this reader opens. A writer that
refused it would be enforcing a stricter rule than the one it is protecting. The constant is now
`pub(crate)` for exactly this: the two sides follow chains the same distance or the writer is
checking a different question from the one it guards. A stream is not refused either, on the same
footing — §7.3.8.1 makes a stream "a dictionary followed by zero or more bytes" and `Object::as_dict`,
which is what `Document::catalog` applies, answers with it.

Nothing in the tree had to change with it, and the corpus says so rather than the argument: all seven
of `pdf-transform`'s writer walks and its perf gate pass unchanged, `foreign_corpus` — qpdf, poppler
and mupdf over each of the five writers' output — included. `pdf-transform`'s writers reach the
serializer with a root they took from `catalog_of`, and `optimize`'s own `Refusal::Reconstructed`
(ADR 0852) already declines the documents where this could bite — **by a heuristic about recovery
rather than by Table 15**, which is why the backstop belongs one crate down where the clause is.

## 3. The crasher is a test, twice

`CLAUDE.md` principle 3 asks that every crasher become a permanent regression test.
`fuzz/corpus/serialize` has the bytes, and because that directory is gitignored it would otherwise be
this disk's alone — so `crates/pdf-syntax/tests/serialize.rs` holds them too, as the 184 bytes with
their length asserted against ADR 1024 §5's figure. Three tests, and each was calibrated against the
defect before the fix was believed (trap 13):

| test | with the refusal removed | with the chain arm removed |
|---|---|---|
| `a_root_that_reaches_no_dictionary_is_refused_rather_than_written` | fails | passes |
| `a_root_naming_no_slot_at_all_is_refused` | fails | passes |
| `a_root_that_reaches_its_dictionary_through_a_reference_is_written` | passes | **fails** |

The third is there because the first two, alone, are satisfied by a rule that is too strict — and a
refusal nobody has calibrated in *that* direction is how a writer starts declining valid files.

## 4. The target, run properly

`tools/fuzz.sh serialize`, which is ADR 1024's own fix to `--list` finally paying: the invocation is
`doc/verify.md`'s, `-runs=50000`, over the 1,133 seeds that file's recipe produced plus the crasher.
A fuzz target cannot run under `tools/bounded.sh` — ADR 1024 §5's warning, and it was heeded.

The seed pass, which is where the target aborted the first time, now reads every unit. A second,
longer bounded run followed it — `-runs=1000000 -max_total_time=900`, 92 298 runs in 901 s — so
142 298 runs in all, over a corpus that went from 1 134 inputs to 4 778. Neither run produced an
artifact, and the one file in `fuzz/artifacts/serialize/` is still ADR 1024 §5's; the figures are in
`doc/history/1009-…`.

## 5. What blocks the third extraction is one type **and one predicate**

ADR 1020 §2 handed on the third extraction with a diagnosis: "What blocks the second step is one
type. `signature::Permissions` carries `doc_mdp_signature: Option<Signature>` and
`usage_rights_signature: Option<Signature>`". Measured here, that is the smaller half of it, and the
correction is the same shape as the one 1020 made to the review it took.

The chain is unchanged and still says what it said — `cargo tree -p spec-errata -i p521` prints
`p521 -> pdf-signature -> pdf-model -> spec-errata` — and `pdf-model`'s whole code dependency on
`pdf-signature` is four call sites: `restriction.rs` asks `permissions(document).doc_mdp`,
`field_locks` and `field_mdp`; `view.rs` asks `permissions(document).usage_rights`. Every one of
those readings is a walk over dictionaries with no cryptography anywhere in it, so on the face of it
the move is mechanical.

**It is not, and the reason is one line.** `field_locks` and `field_mdp` are defined over *signed*
fields — §12.7.5.5's lock binds "after this signature has been signed" — and the predicate this tree
uses for "signed" is `signature::read(document, value).is_some()`, which is the full signature
parser: `/ByteRange`, `/Contents`, the CMS. Lifting the two readings away from the verification stack
therefore means deciding what *signed* means without it, and that is a §12.8 reading with a
population — it changes which fields the corpus counts as signed — rather than a `git mv`.

So the third extraction is: split `Permissions` in two, decide the signed predicate against
§12.7.5.5 and §12.8.2.4, move roughly 550 lines of §12.8 restriction reading to `pdf-model`'s
`restriction.rs` where `CLAUDE.md`'s "a document's restrictions are the reader's to set" already
puts them, and rewrite the 71 ledger rows naming `crates/pdf-signature/src/signature.rs` that follow
them. The prize is unchanged and still worth having: 33 packages leave the page tree's graph, and
`doc/todo/02` §2 rule 2's list of eight crates goes back to seven.

**Not taken here, and not for want of room** — the predicate is the decision, and a round that took
it between two other items would be taking it quietly.

## 6. Consequences

- A trailer this project writes names a catalog the file holds, or the write is refused by name.
- `fuzz/corpus/serialize` carries the crasher, and `crates/pdf-syntax/tests/serialize.rs` carries it
  where the history can reach it.
- The `serialize` target's first real run is on the record, so the next round's is a comparison.
- The third extraction has a scope and a blocker, both measured. ADR 1020 §5's sentence about the
  list of eight going back to seven still stands and still costs one decision.
