# Round 2119 — the `inapplicable` rows re-read, and §14.8.5.4.1 is `implemented`

A clause round in the `gwg` worktree: the ledger's 32 `inapplicable` rows read against their clauses
once no row was `partial` (ADR 3171), each confirmed or moved.

## Read

- 28 rows hold on their clauses' own words and are unchanged: §10.6.1–§10.6.4 (continuous-tone
  device; no route of this program writes bilevel pixels — `render` exports eight-bit grey), §14.2
  (a PostScript device), §14.10 and its 21 subrows, §14.11.4, §14.11.6.1, §14.11.6.3, §14.11.7
  (§3.15's *deprecated*), §14.12.4.2 (a production workflow's metadata).
- §14.8.3.2, §14.8.5.4.6, §14.8.5.4.7 hold, and their reasons were restated: each said "exports to
  no format", which A634's accessibility tree made false. They now say what that export carries
  and what it cannot (no indent, no text column in `accesskit`), and that A634 keeps out the
  reflowing view that would flip them.
- The brief expected a possible Q941; none was needed: grepping the answers found A634 had already
  ruled on the one scope change (round 1991's habit).

## Built

§14.8.5.4.1's owner sentence is read on the reader's side, as §14.8.5.8's already was (ADR 3177):
`Tree::layout_attribute` and `Tree::inherited_layout_attribute` rank only `Layout` and owners other
than `List`, `Table`, `PrintField`, `Artifact`; `writing_mode`, `block_spacing`, `inline_statements`
read through them; `bounds` admits `Artifact` too (Table 385). The test writes each refused object
after the admitted one in one band, and its control assertion shows `Tree::attribute` taking it.

## Ledger

§14.8.5.4.1 `inapplicable` to `implemented`; three notes restated. `doc/todo/65`'s not-owed bucket
says so. Left (ADR 3177): `Tree::print_field` claims a §14.8.5.6 narrowing `attribute` does not
apply; §14.8.5.4's aggregate note still says "nothing in it exports".

**Gates.** pdf-model: fmt 0; clippy `-D warnings --all-targets` 0 (after an unfulfilled `expect` was dropped from the library); `pdf-sandbox --bins` 0;
nextest `--lib --bins --tests` 2876 run, 2876 passed, no difference from
`expected-failures-current.json` (empty); rustdoc `-D warnings --document-private-items` 0 after two
links to private items became plain code; `cargo test -p conformance` 0. Fuzz not owed (no public
type changed; two methods added).
