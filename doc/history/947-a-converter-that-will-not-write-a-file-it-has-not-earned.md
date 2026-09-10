# 947 — A converter that will not write a file it has not earned

Date: 2026-09-10. ADR: 0947.
Files: `crates/pdf-transform/src/archive.rs` (new), `tests/archive.rs` (new),
`tests/archive_corpus.rs` (new), `src/bin/quorra-transform.rs`, `Cargo.toml`.

The first slice of `quorra-transform archive`, the verb `A22` named.

**Three stages, and the middle one is the product.** Validate with `pdf_archive::check`, decide,
apply. `Decision` has four variants at this stage because `doc/pdf-a-conversion-limits.md`'s §2,
§3 and §4 are three *different answers* to a failed requirement rather than three degrees of one,
and `Because` separates an edit the fence forbids from a gap this converter has from a target for
which no conforming file exists. **A requirement in neither table is refused by name**, never
passed over and never answered by a rewrite the converter invented.

Two rules under it. *Nothing is changed that no failed requirement asked for*, which makes "an
already conforming file is not rewritten" true by construction rather than by test. And **a file
leaves the verb only if it conforms**: stage three assembles in memory, re-opens the result, holds
it to the same target, and refuses with the requirements named — naming separately any requirement
the *source* had met, because a conversion that breaks what worked is the worse failure. The cost
is stated rather than discovered: peak memory is the whole output.

The report is a first-class output rather than a log, because all four of the owner's permissions
are conditional on it (ADR 0927), and it carries the validator's `Unchecked` reasons **word for
word** — `A20` applied one layer out.

Two things this slice deliberately did not do, and named instead of stubbing. Removing a `Crypt`
filter, because the requirement *permits* an `Identity` one and removing it would be a change with
no requirement behind it. And rewriting a hexadecimal string inside a content stream, which the
whole-file rewrite does not reach — the first version "fixed" one and wrote a file that still
failed, which is what the stage-three net exists to catch.
