# 2747 — `DeviceCMYK` beside an RGB intent takes the assumed inks as its `/DefaultCMYK` at part 4

Session 1967, a single archive round.
Status: accepted and built in `pdf-transform` (`archive/supplied_cmyk.rs`: `Press::Inks`,
`INKS_REINTERPRETS`, `INKS_PARAMETERS`; `archive/prepare.rs`; `archive/decision/colour_and_graphics.rs`:
`cmyk_answer_under_part_four`). It **revisits** ADR 2114 section 4's first bullet and ADR 2130 section 3,
and builds R05 (b) of `doc/pdfa-blocking-issues.md`.
Context: `doc/questions/A694` item 2, A780, A784; ADRs 2518, 2541, 2691, 2700, 2701, 2705.
Clauses: ISO 32000-2 §8.6.5.6, §11.6.6 (Table 145), §14.11.5; ISO 19005-4 sections 6.2.3, 6.2.4.2,
6.2.4.3; ISO 19005-2 section 6.2.4.3.

## 1. What was refused, and why the reason is gone

At part 4 a file that draws in `DeviceCMYK` beside a destination profile of another family (its own
RGB intent, or the shipped sRGB intent a mixed page takes, ADR 1153) has one licence left under ISO
19005-4 section 6.2.4.3: a device independent `DefaultCMYK` set where the space is used. That is an
ICC CMYK profile. ADR 2114 section 4 and ADR 2130 section 3 declined to write one the program ships,
because this reader drew such colours through its assumed inks and **no ICC profile stated those
inks**, so the default would be a statement this reader's own page contradicts. ADR 2700 wrote that
profile (`data/icc/quorra-assumed-inks.icc`), its `A2B` the ink cube sampled at side 21, and ADR 2701
made it the CMYK intent of every profile but `as-if-printed`. The premise of the refusal no longer
holds, so the refusal goes, and only for the profiles ADR 2701 already gives the inks.

## 2. The decision

At part 4, where nothing was supplied with `--output-intent-profile`:

- **`press-assumed` authorised** (`as-if-printed`): unchanged — the shipped GRACoL press, or the
  operator's configured one, as a named loss (ADRs 2518, 2705).
- **otherwise** (`as-shown`, `no-data-loss`, `state-every-loss`, a run with no profile): the assumed
  inks' profile as an `ICCBased` `/DefaultCMYK`, written where ADR 2130 writes the operator's,
  never over the producer's entry. `Decision::Stated` with `INKS_REINTERPRETS`; no loss. §8.6.5.6
  passes the four values unchanged to the default, so the page is drawn through the profile's
  `A2B` where it was drawn through the cube that table samples: a restatement of the drawing
  within ADR 2700's bounds (one level of 255 on fills), as the same profile is when it is the file's
  intent (ADR 2701). The file's `xmpMM:History` records the default, as it does every
  `/DefaultCMYK` this converter adds, and a packet that cannot take the entry does not get it.
- A configured press stands in for the shipped one only (A784): it does not replace the inks.
- **Part 2** is not offered it: its `DeviceN` default states no press and stays its answer.
- **ADR 2518's overprint guard binds the inks as well**: §8.6.5.6 makes the remapped mark an
  `ICCBased` CMYK one, which ISO 19005-4 section 6.2.4.2 forbids under overprint mode 1.

## 3. What it does not settle

- **Blending.** A transparency group whose `CS` is `DeviceCMYK` is remapped by the default too
  (Table 145), and this renderer composites a CMYK group into the page through the profile's `B2A`
  (ADR 0796), within ADR 2700's 3.62 levels of the search on the press grid; a page whose
  appearance depends on that composite can move by that much. `doc/history/1967` measures the
  corpus outputs.
- **Overprint mode 1** (9 documents): setting the mode of a state that governs only `DeviceCMYK`
  marks to 0 changes what an overprint-simulating reader shows, and is ADR 1978's `overprint-mode`
  loss; offering it here is a separate decision, not this one.
