# 2277 — A redaction restates a direct element, names a copy by its number, and a failed pass leaves no file

Lane 3. Subject: redaction's leftovers from 2254 and 2256 (§12.5.6.23, A834) and the serializer's
partial file. ADRs 3809–3812.

- **A structure element written directly in its parent's `/K`** (ADR 3809): `traces` addresses an
  element by its place — holder object and `/K` path — walks direct kids, and writes each holder
  once with its elements' `/ActualText`, `/Alt`, `/E` (`/T`) removed at their places.
  `redaction_leaves_no_trace::a_structure_element_written_directly_in_its_parent_is_restated_there`
  found the four strings without it.
- **A replacement part of whose codes went** (ADR 3810): read from §14.9.4 — the value substitutes
  for the whole content and nothing maps a part of it to a part of the codes — so it is taken
  whole: written nowhere, the whole value looked for elsewhere, the kept codes reading back as
  themselves. Behaviour already so; the new test holds the reading (passes before and after).
- **A page's own `/Metadata`**: the premise did not hold — ADR 3725 already drops a redacted page's
  packet and the fixture's `page-xmp` is held at zero. Recorded in ADR 3810; no change.
- **A form's placement copies** (ADR 3811): the stale-entry report names a form once, with the
  number of streams it became (`redact::a_form_written_as_two_streams_is_reported_once`, failed
  before with two); a sequence reached in a copy records the copy, the form's own item counts
  only outside copies, and an element whose sequence a copy still partly shows gains an MCR naming
  the copy's output number (`redact::a_placement_copy_is_named_to_the_structure_tree_by_its_own_number`).
- **The partial file** (ADR 3812): `Sinks::commit` (provided, default nothing); the CLI's
  `FileSinks` (`bin/quorra-transform/files.rs`) stages each output as a hidden sibling, renames at
  commit, removes on drop; `convert` commits after `apply` answers; the confined route carries
  `SINKS_COMMIT` to the broker. `files::tests` inject a failure after a partial write.
- No redaction corpus census exists (none in `tools/state.sh` or the transform tests).
- Ledger: 12.5.6.23 stays `departed`; note and tests restated.
- Left, in order: a structure tree root written directly in the catalog does not restate its
  direct kids (ADR 3809); a placement copy's `/StructParents` is the form's, so a parent tree that
  maps the form's MCIDs per placement cannot tell copies apart.

**Gates.** Rebased on `492622d82`, over `-p pdf-syntax -p pdf-model -p pdf-transform -p
pdf-archive -p viewer-qt -p conformance`: fmt 0, clippy `-D warnings --all-targets` 0,
`pdf-sandbox --bins` 0, nextest `--lib --bins --tests` (`xvfb-run -a`) 5909 of 5910, the one
failure `records` on this record's unfilled paragraph (`audit-diff.py`: no manifest entry, none
expected); rustdoc of `pdf-transform` 0; fuzz `cargo check --bins` 0 (`Sinks` gained a method);
`cargo test -p conformance` 0 (records included); launch queued behind round 2276's clock hold.
