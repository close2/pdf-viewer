# 1371 — A field's one-call `AF*` script runs, and `/CO` is walked

Date: 2026-10-06. ADRs: 1578, 1579. The SCRIPTS slot of batch fifty-eight: RFC 0008's Tier 0, on
the owner's A193. No status moved; two notes rewritten; no question written.

**The library (ADR 1578).** `pdf_model::aform`, all twenty-one functions named, every rule a
documented choice. The brief's premise did not hold: Adobe's *JavaScript for Acrobat API Reference*
lists no `AF*` function (re-read at `ab3b42a7`); the menus are the *Interapplication Communication
API Reference*'s `SetJavaScriptAction`. Adobe's own expected values are fixtures: `printd`'s table,
`scand`'s horizon, `printx`'s telephone, `printf`'s `314.16`, the four `AFTime` examples. The two
texts disagree on the 12-hour marker; each function follows its own page.

**The dispatch (ADR 1579).** A `/JS` textually one call with literal arguments runs: `/K` in
`set_field`, `/K`'s commit form and `/V` in the new `ViewState::commit_field`, `/F` where
`appearance::field_text` lays a value out (typed values as typed until committed; a save always
formatted), `/CO` after every value change. All else is reported once (`script_reports`,
`Owed::Script`); the JavaScript action's refusal says the same, one tier narrower. **Handed to a
`viewer-core` round: no host calls `commit_field` yet** — `/Bl` and Enter are where it belongs.

**The gate.** `tests/script_corpus.rs` over RFC 0008 section 3's population, derived from the disk:
90 763 PDFs, 347 in the gate, 46 s, so `t2-script_corpus` joins `tools/batch.sh gates`. Its first
run found comma-style fields storing `5.25` that their own keystroke refused on read-back; the comma
styles now take a period too (four lines moved, digests unchanged).

**Moved pages: none.** The only page-one widgets a format could reach are `160F-2019.pdf`'s five
constructed ones, all `/F 6` (Hidden). `saving.rs` typed letters into that file's number field and
now types `1234.5`, asserting `(1,234.50) Tj`; `save_round_trip.rs` fell to 75 of its floor 80 at
the merge; it now types `Call::accepted_example` (ten documents), commits, asserts the formatted
appearance: 80. **The drive.** `quorra` under Xvfb, a three-field form: `12.5` and `7x` typed, `x` refused, the
total drawn `$19.50`; the saved file: `/V` `12.5`, `7`, `19.5`, appearances `$12.50`, `$7.00`,
`$19.50`.

**Rows.** §12.7.3 stays `implemented`, its `/CO` sentence now executed — Table 224's "when the
value of any field changes". §12.6.4.17 stays `out-of-scope`: Tier 0 runs the library, no ECMAScript.

**Gates.** rustfmt --check on my sixteen Rust files: exit 0. `RUSTFLAGS="-D warnings" cargo clippy
-p pdf-model --all-targets`: exit 0. `cargo nextest run -p pdf-model`: 1862 passed; `-p viewer-core
-p viewer-ffi`: 339 passed. `cargo test -p conformance`: exit 0. Behind the lock: `script_corpus`
347 held, 0 moved (46 s); `raster_golden` held 974, moved 0; `render-raster --test corpus` at 1×:
968 / 0 / 0 / 6, one-versus-many 0.
