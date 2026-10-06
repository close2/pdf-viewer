# 1363 — Clause 7's notes state what is, and a movie shows its poster

Ledger slot of batch fifty-six. ADR 1561; one row moved; no question written.

**The notes.** ADR 1547's rule, applied to the family carrying the most: the 82 clause-7 rows with
any session ordinal (344 of 1278 by `tests/ledger_notes.rs`'s count) were rewritten as what is by
eight readers working from JSON copies, each output checked by a script that every kept
double-quoted quotation, backticked name, ADR and `§` number is in the old note and that no
ordinal remains, then applied chunk by chunk under a lock with `assert s.count(old) == 1` on a
fresh read and `cargo test -p conformance` after each chunk. Clause 7's rewritten notes went from
399 198 to 354 225 characters and carry no ordinal; the ledger's count is 934 and the constant
holds it. Five rows spot-read against their old notes (§7.2.2, §7.10.2, §7.6.4.1, §7.5.7,
§7.9.2.2.2). §7.5.7's stale sentence about Annex F being unratified now states what
`serialize::packable`'s comment states. `doc/todo/14`'s `Cited by` named §7.4 and §7.8.2, whose
only mention of it was a retired sentence; corrected. No status moved in this pass.

**The poster.** `appearance::construct`'s `Movie` arm is `movie_poster`: Table 189's `/Movie`,
Table 306's `/Poster` — a stream is an image painted through `Stream::form`, fitted into `/Rect`
by Table 250's defaults and reported as a placement this program chose (ADR 1561, under A72's
rule); `false` or absent draws nothing and owes nothing; `true` is refused, since a frame from the
movie file is playing the movie. The comment naming Q33 as open and the refusal string calling the
poster excluded are gone. Five fixtures in `crates/pdf-model/tests/movie_poster.rs`.

**One row moved: §13.4 `reported` → `departed`** — the playing (Table 307, and `/Poster true`) is
the one withholding, on the clause 13 exclusion A33 bounded; `doc/todo/65`'s bucket 6 bullet left
with it. §12.5.6.17 stays `out-of-scope`, its note made current.

**Gates.** `rustfmt --check --edition 2024` on the four Rust files: exit 0. `RUSTFLAGS="-D warnings" cargo
clippy -p pdf-model -p conformance --all-targets --keep-going`: exit 0. `cargo nextest run -p
pdf-model`: 1785 passed, exit 0. `cargo test -p conformance`: exit 0 (`ledger_notes` at 934,
`the_frontier_map`). `--bin quotations`: 4 ledger divergences, as before. Behind the lock:
`pdf-model --test corpus` exit 0 (59 incomplete, every ratchet at its ceiling), `raster_golden`
exit 0 (held 974, moved 0), `render-raster --test corpus` at 1× exit 0 (968 / 0 / 0 / 6). No page
changed, and no corpus report names a movie annotation before or after.
