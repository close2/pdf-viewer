# Every viewable document converts under a profile

Status: **open.** The project owner, 2026-10-15: "The pdf-a converter should be able to convert
*all* pdfs which are viewable", under a profile that is a lower bound on what may be lost, never an
instruction to lose it (ADR 1942). This file is the work that sentence still needs, in order.
Priority: the archive slot's standing list — an archive slot takes the top open item.
Instrument: `cargo run --release -p pdf-transform --example archive_unbuilt -- doc/pdf.js/test/pdfs
doc/corpora` ranks every site `--remedy-sites` counts as not built by the documents failing it,
per target, with its kind (`the-fence`, `not-built-yet`, `not-this-target`). The order below is
that ranking's; re-run it rather than trusting it, and the veraPDF corpus (`doc/veraPDF-corpus`)
separately, because its files are built to fail one rule each.
Catalogue: `doc/pdf-a-mitigations.md`, keyed by the same site identifiers.

## 1. Where a person chooses a profile

- **The window has no archive path.** `viewer-core`, the hosts and the dialogue offer no PDF/A
  conversion at all; only `quorra-transform archive` does. Owed: a command beside Print that names
  the target and one of the shipped profiles by name, runs the verb, and shows the report's
  per-site lines — `--profile <name>` is the CLI's half (ADR 1942).
- **The report names the configuration once** (stderr) and each site "answered by this
  configuration"; a JSON `profile` field in the conversion report is owed so a pipeline can read it.

## 2. The sites with no remedy, by how many documents reach them

Each line: the site, and what its remedy would be. `the-fence` rows are refused by argument
(ADR 0816) and leave this list only by an argued amendment or a departure the owner allows.

1. `optional-content/configuration-names` (not built) — ISO 19005-2 section 6.9: every optional
   content configuration states a `/Name`; supplying one is a fresh text string naming nothing on
   any page, the sibling of the `/Order` row the converter already answers.
2. `graphics/only-operators-the-base-standard-defines` (fence) — an operator the standard does
   not define. §7.8.2 makes one an error outside a `BX`/`EX` section and has it ignored inside
   one (Table 33), so removing one *inside* such a section changes nothing a conforming reader
   draws; outside, what the page means is this reader's tolerance. Argue the first half.
3. `fonts/embedded-programs-define-every-glyph-shown` (fence) — a shown code the program has no
   glyph for. ADR 1943's route may reach a glyph for some; re-count after it before anything else.
4. `graphics/no-transfer-function-in-a-graphics-state` (not built) — §10.5's transfer function is
   in scope and drawn (doc/todo/13); applying it into the colours is a content rewrite, and the
   honest answer is per profile: as-if-printed keeps it for the press, as-shown would bake it in.
5. `graphics/named-resources-are-defined` (fence) — a resource name the page uses and the file
   does not define. Read what ISO 32000-2 has a processor do with one before arguing a remedy;
   the catalogue's "none" rests on supplying an object being invention.
6. `file-structure/stream-filters-are-standard` (not built) — a non-standard filter; decoding and
   re-encoding under `FlateDecode` loses nothing where this tree decodes it.
7. `implementation-limits/values-written-in-content-streams` (not this target) — part 4 lifts it.
8. `fonts/non-symbolic-truetype-differences-are-listed-names` (fence) — a `/Differences` name
   outside the Adobe Glyph List; ADR 1942's construction names the glyph by its `/ToUnicode`
   character instead, proved the same way — the same build widened.
9. `file-structure/bound-names-are-valid-utf8`, `fonts/symbolic-truetype-program-has-a-usable-cmap`,
   `fonts/cid-system-info-agrees-with-the-cmap`, `graphics/no-halftone-name`,
   `graphics/no-overprint-mode-one-under-icc-cmyk` (catalogued as `discard`, OPM 0, Ask),
   `graphics/icc-profiles-carry-the-tags-a-permitted-edition-requires`,
   `fonts/non-symbolic-truetype-program-maps-every-code`, `graphics/inline-image-interpolation-is-off`
   — each reached by a handful; the catalogue entry says what each would be.
10. Part 4 only: `annotations/file-attachment-only-in-embedded-file-files` and
    `annotations/three-dimensional-only-in-engineering-files` (a retarget to 4f or 4e is the
    answer), `fonts/type3-glyph-procedures-state-their-width`, `fonts/actual-text-states-no-private-use`.
11. Level A only: `logical-structure/structure-tree-root` (fence — auto-tagging; 2u is the answer)
    and `logical-structure/role-map-terminates-at-a-standard-type` (catalogued as `supply`).

**One validation that does not end.** `isartor-6-1-12-t01-fail-a.pdf` under
`doc/veraPDF-corpus/Isartor test files/PDFA-1b/` (4 010 934 bytes) holds `pdf_archive::check` at
one target for more than 120 s at full processor; the census over that corpus never finished.
No gate walks that directory. The validator owes a budget, and the walk that stalls a profile,
before a viewer's conversion command can be offered (item 1).

## 3. What the two shipped lower-bound profiles still owe

- **`as-shown`'s `printable-and-visible` split.** One answer covers a hidden annotation (removing
  it is inside the bound) and a visible, non-printing one (it is not); a `shape` per Table 167 bit
  lets the profile keep the second and remove the first. Until then the profile stops there.
- **"As close as possible" at the forbidden subtypes.** `as-if-printed` discards them; the owner's
  rule prefers `preserve` (ADR 1123) with `discard` as the one `on-failure` alternative.
- **The `DefaultCMYK` row's sentence is false for this renderer.** Pol.pdf's DeviceCMYK text moves
  by up to 34 of 255 under §10.4.2.5's conversion the converter writes; the sentence says this
  renderer already drew those colours so. Either draw DeviceCMYK the §10.4.2.5 way without an
  output intent (§10.4.2.1's order is the question) or say what changes.
- **Pol.pdf at 2u**: its Type 3 font names glyphs `/0`…`/7`, outside the Adobe Glyph List, and
  carries no `/ToUnicode` (section 6.2.11.7.2). **At 4**: DeviceRGB and DeviceCMYK on one page,
  and part 4 states no `DeviceN` half for CMYK, so a CMYK output intent is needed beside RGB.
