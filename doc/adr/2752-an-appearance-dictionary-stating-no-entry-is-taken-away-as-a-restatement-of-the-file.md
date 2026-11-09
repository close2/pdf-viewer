# 2752 — An appearance dictionary stating no entry is taken away, as a restatement of the file

Session 1970, a single archive round.
Status: **accepted and built** in `pdf-archive` (`table/interaction.rs`: `states_an_appearance`)
and `pdf-transform` (`archive/sites.rs`: `appearance_states`, `AppearanceStates::emptied`;
`archive/rewrite/appearance_states.rs`: `collapse_appearance_states`). It narrows the
`normal-appearance-shape` refusal of R16 in `doc/pdfa-blocking-issues.md`.
Context: ADRs 0955, 1234, 2319, 2751; `doc/questions/A48`.
Clauses: ISO 32000-2 §7.3.7, §7.3.10, §12.5.2 (Table 166), §12.5.5 (Table 170); ISO 19005-2 and
ISO 19005-4 section 6.3.3.

## 1. The construction

The `normal-appearance-shape` site refused every appearance dictionary stating no `/N`: the file
states no appearance to move there. One shape of that is a dictionary stating **no entry at all**:
`/AP << >>`, or `/AP << /N 0 0 R >>`, which §7.3.10 reads as a reference to null and §7.3.7 as
"the same as if the entry does not exist". Table 170 names the three entries an appearance
dictionary holds and requires `/N`; a dictionary holding none describes no appearance, which is
what an annotation with no `/AP` describes.

- **The reading is one reading.** `pdf_archive::states_an_appearance` is it, and the
  appearance-present rows use it: an `/AP` of null entries now counts as none there, as an empty
  one already did, so such an annotation is in `annotations_without_an_appearance`'s population
  whenever its subtype is not exempt. Before, the two rows read the same dictionary two ways.
- **The construction**: `appearance_states` records the annotation in `emptied` and the rewrite
  removes the key. What the annotation owes after that is the other row's: a `Link`, a `Popup`
  and, at part 4, a `Projection` need no appearance (Table 166; section 6.3.3), and any other
  subtype is constructed by the appearance rows over the same population, or refused there by
  their own sentence.
- **Every profile, both parts**, as the site's `Stated` decision; no loss, since nothing the file
  states goes.
- **Kept refused**: a dictionary with a non-null entry and no `/N` (an `/R` or `/D` alone), and an
  `/AP` that is not a dictionary. The first states something the removal would lose; the
  existing fixture of a null `/N` now carries a `/D` beside it to keep that sentence's case.

## 2. What it freed

`sakula` (format-corpus): one `Link` with `/AP << /N 0 0 R >>`, `/Border [0 0 0]`. Under
`as-if-printed` at 2b it was the only refusal; at part 4 R03 still stops it, and under `as-shown`
R09.

## What it costs

Nothing: `sakula` with the entry blanked out of the source draws every one of its 49 pages as the
source does at 72 dpi (`doc/history/1970`); the output's other differences are the profile's named
font substitutions.
