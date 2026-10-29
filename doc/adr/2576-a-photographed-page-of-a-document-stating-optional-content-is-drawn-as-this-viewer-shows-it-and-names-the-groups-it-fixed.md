# 2576 — A photographed page of a document stating optional content is drawn as this viewer shows it, and names the groups it fixed

Status: implemented. Session 1871, the first archive slot of batch 128. Builds the owner's answer
A717 (`doc/questions/A717`); the gate it lifts is ADR 2394's, as ADR 2516 widened it; the loss is
carried the way ADR 2516 item 7 carries A707's `tagging-of-photographed-pages`.

## What the answer allowed

ADR 2394 refused to photograph any page of a document stating `/OCProperties` (ISO 32000-2
§8.11.4.2), because a photograph of the configuration this viewer shows discards the others. A717
allows it under `as-if-printed`, at 2b and every part 4 flavour, baking in the configuration this
viewer shows and never switching to §8.11.4.4's `Print` usage (A694 item 2: a document is assumed
displayed only), under a loss of its own, the groups discarded named per page in the report and
in `xmpMM:History`, every other page keeping its optional content, Level A and Level U keeping the
refusal.

## Decisions

1. **The loss is `optional-content-of-photographed-pages`**, carried by the `discard` that answers
   `fonts/font-programs-embedded` (the bundle in `decision.rs`'s `CONDITIONAL`), exactly as
   `tagging-of-photographed-pages` is. A plan without it keeps the refusal, whose sentence now names
   the word that lifts it.
2. **The configuration is the one the window opens the document in**:
   `pdf_model::view::ViewState::of`, which `interpret_ledgered` already draws under — §8.11.4.3's
   `/D` with §8.11.4.5's `/BaseState` and `/ON`/`/OFF`, then its `View` usage at an unstated
   magnification, under `Purpose::View`. What was recorded is read from the same
   `OptionalContent::read`, so what is named is what was drawn.
3. **Every photographed page of such a document incurs the loss, even one whose content names no
   group**: the page was drawn in one configuration all the same, and the walk over its resources
   does not interpret it, so the over-statement is the direction that hides nothing. Such a page
   says its content's resources name no group.
4. **The groups recorded are the ones the page's content names**, through
   `pdf_model::optional_content::groups_referenced_by_content` — the page's `/Properties`
   (§8.11.3.2) and an `XObject`'s `/OC` (§8.11.3.3), into nested forms under that walk's depth bound
   and visited set. Annotations are left out: they stay annotations with their own `/OC`. Each is
   named by Table 96's `/Name` (or its object number) with the state it was drawn in — on, off, or
   not considered where the configuration's `/Intent` leaves it out (§8.11.2.3). Bounded before the
   report grows: 64 groups a page, 200 characters a name, the rest counted.
5. **`/OCProperties` and the other pages are untouched**; only the photographed page's content and
   resources go, as for every photograph.
6. **`state-every-loss` answers the font site with `try = ["append-statement", "discard"]`**, the
   profile's shape at every presentation site (`doc/rfc/0014` section 4, `doc/adr/2562`): its
   `discard` carries the whole bundle of decision 1, `optional-content-of-photographed-pages` among
   it, so A717's "(and `state-every-loss`)" photographs the page there as in `as-if-printed`. The
   first step is passed over by name (`config/steps.rs`'s `STATED_ONLY_PER_OBJECT`): the statement
   page is built where a loss is reported one object at a time, and listing photographed pages on it
   is a construction of its own. The loss is named per page in the report and in `xmpMM:History`.

## What it does not reach

- §8.11.3.2 also names a pattern's content stream and a Type 3 glyph's; the walk follows neither a
  `/Pattern` nor a Type 3 font's `/Resources`, so a group named only there is drawn as shown but not
  listed. The loss is still recorded for the page (decision 3).
- **The statement page at the font site**: `append-statement` there is passed over (decision 6)
  until `statement.rs` has a section for photographed pages and fonts.

## Figures

The census, both corpora, in an export of this change against HEAD's (`--profile gates`):

| census line | HEAD | this change |
|---|---|---|
| pdf.js `as-if-printed` 2b, no pack (converted / refused) | 872 / 86 | 874 / 84 |
| pdf.js `as-if-printed` 4, no pack | 883 / 77 | 885 / 75 |
| pdf.js `as-if-printed` either part, no pack (at 2b / refused at both) | 874 / 71 | 876 / 69 |
| pdf.js `as-if-printed` 2b, with the pack | 873 / 85 | 875 / 83 |
| pdf.js `as-if-printed` 4, with the pack | 884 / 76 | 886 / 74 |
| pdf.js `as-if-printed` either part, with the pack | 875 / 70 | 877 / 68 |
| corpora, "states optional content" (2b and 4, both runs) | 3 | 0 |
| corpora, "does not draw whole" (2b and 4, both runs) | 16 | 19 |

The two pdf.js documents freed, `issue19484_1.pdf` and `issue19484_2.pdf`, were refused at HEAD by
the fence's own sentence and are photographed now (one page each, no group named: decision 3);
veraPDF passes all four outputs at 2b and 4, and a copy of one whose `pdfaid:conformance` is
planted as `Z` fails clause 6.6.4. The three corpora documents (`050734`, `064034`, `504076`) pass
the optional-content gate and are held by the next one, a page whose content this reader does not
draw whole (`photographed.rs`'s `NOT_DRAWN_WHOLE`); `as-shown` and every other census line are
unchanged.

`getrusage` high-water, one conversion at 2b under `as-if-printed`, of a hostile document of one
photographed page whose content names 20 000 groups: HEAD refuses it at 67 MiB; this change converts
it at 102 MiB, 64 groups named and 19 936 counted (`MAX_GROUPS_NAMED`).

