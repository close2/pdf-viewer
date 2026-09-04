# 928 — Five rows off the blame list's head, and three of them owed a permission

Date: 2026-09-04.
ADR: 0896 (a `partial` whose debt is a permission — §12.11.5, §14.9.2, §14.9.2.2);
0897 (§14.7.4.2's unnamed `shall`, and the cousin already carrying it).
Files: `doc/conformance/ledger.toml`, `crates/pdf-model/src/attachment.rs`,
`doc/todo/01-ledger-partial-rows.md`, `doc/adr/0896-*.md`, `doc/adr/0897-*.md`.

A coverage round, taking `doc/todo/01`'s standing task: the ledger's `partial` rows read against
the code, ordered by `git blame` over `ledger.toml` on the commit that last wrote each `note = `
line. The base was 1389 commits and 217 `partial` rows; the head of the list is a band of five
sharing rank 646 — commit `0a431506`, session 489 — with the next band eleven commits above.
All five were read whole.

## The five rows

| row | what it claimed | what the code does | what changed |
|---|---|---|---|
| **§12.11.5** | `/RH` is read by nobody, and there is nothing to disable | true — nothing under `crates/` quotes `"RH"` | **`partial` → `out-of-scope`**, script exclusion. Every requirement in the clause binds a processor that invokes a handler, and a handler is a script |
| **§14.9.2** | `partial` for Table 122's `/Lang` on a CIDFont descriptor | true — nothing names it | **`partial` → `implemented`**. Table 122 says the entry "may be used"; a permission, declined with its reason |
| **§14.9.2.2** | the same entry, "the one of the four still read by nothing" | true | **`partial` → `implemented`**, with the clause's three `shall`s checked one at a time |
| **§14.7.4.2** | `partial` because `/Schema` is stored and never fetched | true, and not what the clause asks | **kept `partial`, for a different sentence** — the clause's closing `shall` about attribute owners, which the row had never named |
| **§14.12.4.1** | Table 408's `/DPartRoot` is not read | still true of *following* it; two new sites *name* it | kept, with the evidence that kept it |
| **§14.13.8** | `/AF` on a `DPart` "reads like any other" | true, and held by nothing | kept, and the cited test now states a `DPart` |

Six rows in a table headed "five" because §14.9.2 and §14.9.2.2 are the same debt at two levels,
and §14.13.8 was read beside §14.12.4.1 as its sibling; both are in the band.

## What the round is about

Three of the six were `partial` for an entry the standard offers with *may* or *can*. That is a
status promising an unexecuted requirement over a note naming none, and no sweep in `doc/todo/01`
can print it, because every one of them reads a row that owes something and asks whether the owed
thing exists. ADR 0896 has the argument and the grep that would find the rest.

The fourth is the opposite: a row whose `partial` was resting on a permission while the clause's
real `shall` — that a namespace name identifies an attribute object's owner, and is equivalent to a
Table 376 owner value where it corresponds to one — went unmentioned, with a live consumer in
`Tree::attribute`'s filter and an open question already written into §14.8.5.3's row since session
811. Neither row cited the other. ADR 0897.

## Two citations that did not assert what their row said

- **§12.11.5** cited `requirements.rs::a_requirement_states_a_type_a_version_and_a_penalty`, which
  builds a `/Requirements` array and asserts Table 273's `/S`, `/V` and `/Penalty` defaults —
  §12.11.1's subject, and nothing of §12.11.5. It went with the status.
- **§14.13.8** cited `attachment.rs::an_associated_file_carries_its_relationship`, which asserted
  `/AF` on a catalog and on a structure element. The row's claim is about a `DPart`, and
  `associated` takes any dictionary, so the claim was an inference from a signature. The test
  states a `DPart` now, calibrated by planting the defect the claim denies — `associated`
  answering only for a `/Type` of `Catalog` or `StructElem` — under which the new assertion is the
  one that fails and the other two still pass.

## What the next round takes

The band above this one is rank 657 (`fc41aff8`, session 501): §12.7.7, §12.7.8.3.2 and
§12.7.8.3.3, all three carrying a read-and-kept sentence from that session; then rank 668
(`bad96d5f`, session 510) with seven, of which §11.7.4.4 is the one that can change a pixel.
`doc/todo/01`'s own section says the same.
