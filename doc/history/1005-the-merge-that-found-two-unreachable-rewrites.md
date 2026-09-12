# 1005 — The merge of 999–1004, and two rewrites the exemption left unreachable

Date: 2026-09-12. Rounds merged: 999 (the tool request and the one shared executor, ADR 1019),
1000 (`pdf-signature` extracted, ADR 1020), 1001 (`Examination::reaches()`, ADR 1021), 1002 (the
alpha's kind as a field of the raster, ADR 1022), 1003 (the comment rule and four documents as
*what is*, ADR 1023), 1004 (a threshold for a lint nobody ran, ADR 1024). Second batch in a
worktree, fast-forwarded into `main`.

## What the merge itself did

**Twelve converter tests failed, and ten were the fixtures' fault.** Session 1001 made ISO 19005-2
6.2.2's exemption readable — a named resource no content stream references is not judged — and
twelve fixtures in `crates/pdf-transform/tests/archive.rs` declared an `/XObject`, an `/ExtGState`,
a `/Font` or a `/ColorSpace` that their empty page never drew. The builder draws what a fixture
declares now (`draws()`), and a page that draws *deliberately* nothing states its own empty stream
and says why. That is ten of them.

**The other two are `doc/questions/Q62`.** `Rewrite::PostScriptXObject` and
`Rewrite::SymbolicTrueTypeEncodingRemoved` are reachable by no document: drawn, the converter
refuses (removing a PostScript XObject leaves the `Do` naming nothing, which 6.2.2 forbids; a
rendered symbolic font needs a `cmap` subtable ADR 0816's fence keeps out), and undrawn, the
exemption withdraws the requirement. Both rewrites were built against fixtures whose pages drew
nothing, which is why nothing noticed until reachability existed. The two tests assert the
exemption now, which is what the clause says.

**And the merge got a clause wrong, which the gate caught in a minute.** It wrote "§8.8.2 says a
conforming reader shall ignore a PostScript XObject" into both the test and the question.
**ISO 32000-2 has no PostScript XObjects at all** — `grep -c` returns 0, and its 8.8.1 says there
are two types of external object, image and form. The rule is ISO 32000-1:2008, 8.8.2's, which is
the edition part 2 delegates to, and its sentence is stronger than the paraphrase: such fragments
"shall have no effect either when viewing the document on-screen or when printing it to a
non-PostScript device". The habit is in `doc/habits/reading-the-specification.md`.

Also the merge's: `SampleAlpha`'s enum carries an `#[expect(clippy::doc_markdown)]` because `Both`'s
comment quotes §8.9.6.1 verbatim and a quotation is not marked up; and three habits placed, one of
them 1000's correction of the review's own prediction.

## The figures, off the merged run

`cargo fmt --all --check` and the `fuzz/` pair clean; `clippy --workspace --all-targets` under
`RUSTFLAGS="-D warnings"` exit 0; `nextest --workspace` **4456 passed, 36 skipped**; doctests exit
0; `cargo test -p conformance` exit 0; **`tools/state.sh` whole exit 0**, its first run carrying
995's six new walks and 996's golden — which reports `974 tracked documents, 966 first pages drawn,
974 entries, held 974, moved 0`. Pointers 211 absent / 17 undefined, every new one a record naming
a module 1000 moved, none in a live document; quotations unchanged.
