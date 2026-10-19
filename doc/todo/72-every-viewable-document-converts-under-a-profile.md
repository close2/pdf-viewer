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

- **The window converts, chosen at launch** (ADR 1954): `--archive <target>:<profile>` and Shift+A
  in `quorra`, `quorra-gtk` and `quorra-qt`; `Command::Archive` asks Table 22's bit 11, the host runs
  the verb on a thread through `pdf_transform::profiles` and the one executor, says the report's
  lines and writes `<name>.pdfa-<target>.pdf` beside the document; `quorra_archive` in the C ABI.
  The JSON report carries `profile`. **Chosen after launch** (ADR 1966): Shift+A with nothing
  chosen offers the six targets and the shipped profiles with each one's sentence — `quorra`'s
  card, a GTK and a Qt dialogue — and the window keeps the choice. **Under a password** (ADR 1967):
  a document opened with one converts under it, from the window's `password::Kept`. **Owed**:
  choosing again in a window that has chosen; the confined window converts nothing, by argument.

## 2. The sites with no remedy, by how many documents reach them

Each line: the site, and what its remedy would be. `the-fence` rows are refused by argument
(ADR 0816) and leave this list only by an argued amendment or a departure the owner allows.

1. `optional-content/configuration-names` — **built as a `supply`** (ADR 1952): the three shipped
   lower-bound profiles name a configuration that states none `Configuration {n}`. Left: a name
   two configurations share (no corpus document does).
2. `graphics/only-operators-the-base-standard-defines` (fence) — an operator the standard does
   not define. §7.8.2 makes one an error outside a `BX`/`EX` section and has it ignored inside
   one (Table 33), so removing one *inside* such a section changes nothing a conforming reader
   draws; outside, what the page means is this reader's tolerance. Argue the first half.
3. `fonts/embedded-programs-define-every-glyph-shown` (fence) — a shown code the program has no
   glyph for. ADR 1943's route may reach a glyph for some; re-count after it before anything else.
4. `graphics/no-transfer-function-in-a-graphics-state` — **built**: every `/TR` `Identity` is a
   statement (26 of 27 documents; ADR 1952 section 3), any other function the loss
   `transfer-function`, kept in `xmpMM:History` before it goes. Both shipped profiles say `stop`:
   a printer applies the function as a screen does, and the witness (`issue6931_reduced.pdf`)
   turns to a negative without it (ADR 1964). Left: `/TR2` other than `Default`
   (`second-transfer-function-is-default`, no corpus document) and a function past the record's
   bound, refused.
5. `graphics/named-resources-are-defined` (fence) — a resource name the page uses and the file
   does not define. Read what ISO 32000-2 has a processor do with one before arguing a remedy;
   the catalogue's "none" rests on supplying an object being invention.
6. `file-structure/stream-filters-are-standard` — **built for Table 92's abbreviations** (ADR
   1952: the name written in full, the bytes kept). Left: `XXXDecode`, `BrotliDecode`, `DCTecode`,
   names this tree does not decode.
7. `implementation-limits/values-written-in-content-streams` (not this target) — part 4 lifts it.
8. `fonts/non-symbolic-truetype-differences-are-listed-names` (fence) — a `/Differences` name
   outside the Adobe Glyph List; ADR 1942's construction names the glyph by its `/ToUnicode`
   character instead, proved the same way — the same build widened.
9. `graphics/no-halftone-name` — **built** as the loss `halftone-name`, `discard` under `as-shown`
   (ADR 1965): `endchar.pdf` converts to 2b. The next ranked, each the only refusal of one witness
   (`examples/archive_unbuilt -- … --who <site>` names them):
   `graphics/no-overprint-mode-one-under-icc-cmyk` (`102952.pdf`) and
   `graphics/icc-profiles-carry-the-tags-a-permitted-edition-requires` (`issue14814.pdf`); the
   first is catalogued as `discard`, OPM 0, Ask. Then `file-structure/bound-names-are-valid-utf8`,
   `fonts/symbolic-truetype-program-has-a-usable-cmap`, `fonts/cid-system-info-agrees-with-the-cmap`,
   `fonts/non-symbolic-truetype-program-maps-every-code`, `graphics/inline-image-interpolation-is-off`
   — each reached by a handful; the catalogue entry says what each would be.
10. Part 4 only: `annotations/file-attachment-only-in-embedded-file-files` and
    `annotations/three-dimensional-only-in-engineering-files` (a retarget to 4f or 4e is the
    answer), `fonts/type3-glyph-procedures-state-their-width`, `fonts/actual-text-states-no-private-use`.
11. Level A only: `logical-structure/structure-tree-root` (fence — auto-tagging; 2u is the answer)
    and `logical-structure/role-map-terminates-at-a-standard-type` (catalogued as `supply`).

**The validation that did not end, ends** (ADR 1953): Isartor's `6-1-12-t01-fail-a.pdf` (ten
thousand pages in one `/Kids` array) cost 269 s per target in quadratic page lookups and costs
1.5 s with `Pages::every`'s one walk. No gate walks `doc/veraPDF-corpus`; the census over it is
the instrument (`examples/archive_unbuilt -- doc/veraPDF-corpus`).

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
