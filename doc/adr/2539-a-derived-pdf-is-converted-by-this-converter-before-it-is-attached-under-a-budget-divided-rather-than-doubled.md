# 2539 — A derived PDF is converted by this converter before it is attached, under a budget divided rather than doubled

Status: implemented, `attach` and `append`. Session 1852.
Context: RFC 0014 section 5; RFC 0007 sections 4.1, 4.2 and 4.6; `doc/questions/A54`, `A55`, `A56`, `A706` item 5;
ISO 19005-2 section 6.8; ISO 19005-4 section 6.9; ISO 32000-2 §7.11.3 Table 43; ADRs 0947, 2534.
Code: `crates/pdf-transform/src/archive/derived.rs` (`convert`, `target_for`), `src/archive/remedies.rs` (`place`,
`Remedies::alternatives`, `Remedies::held`), `src/archive/rewrite.rs` (the `/AFRelationship` edit),
`src/archive/report.rs` (`ConvertedTo`, `DerivedOutcome::NotConverted`), `src/archive/config.rs`
(`Derivation::placement`), `src/archive/mod.rs` (`appended_pages`) and `src/lib.rs` (`run`/`run_pass` take the caller's
`Budget`), `src/archive/derived.rs` (`import_pages`, `licensed_families`), `src/archive/prepare.rs` (`as_pages`,
`AttachmentsToLayOut::derived`), `tests/archive/derived_converted.rs`.

## Decision

1. **Stage two is the converter on the tool's bytes.** Where a `derive` tool promised `application/pdf` and returned a
   file that opens, `derived::convert` opens it under the caller's limits and runs `run_pass` over it with the outer plan,
   the target swapped, and `derivations`, `resolutions`, `tool_outputs` and `external_data` emptied. What is attached is
   that conversion's output; the tool's bytes stay identified by the row's digest (RFC 0007 section 4.4).
2. **The inner target** is the least demanding part the outer clause admits for an attachment: PDF/A-2b under any part-2
   target (section 6.8 admits part 1 or part 2 at any level, and 2b's requirements are contained in 2a's and 2u's), plain
   PDF/A-4 under a part-4 target (section 6.9 admits parts 1, 2 and 4). Under `append` it is the outer target itself,
   because the pages become the archive's own.
3. **One level, by the plan's shape.** The inner plan names no program, so an attachment inside the derived PDF is answered
   by what the configuration answers without one, and no second program runs; a request reaching the inner report is
   refused by name. The inner run takes no `settle` pass (ADR 2534's one extra pass is the outer run's).
4. **The budget is divided, not doubled.** Every derived file one conversion holds is charged against one stream's
   ceiling, `Limits::max_stream_len`, of the caller's `Budget` — the ceiling the archive's reader holds each of those
   streams to. Before a file is opened its length is checked against what is left; the inner document is opened under
   what is left; its output is checked again and charged. Past the bound the row is `not-converted` with the sentence
   naming both figures, and the site keeps its refusal.
5. **At part 4 the specification states `/AFRelationship /Alternative`** — Table 43's value for an alternative
   representation of content — replacing what the producer stated about the original file, which is no longer there.
   Part 2's base, ISO 32000-1, has no such key, so nothing is written there.
6. **The inner report is folded into the row, not into the outer `decided`.** `Derived::converted` carries the inner
   target, the placement word, the converted size and the requirements the inner run decided; the rendered report and
   `xmpMM:History` say the attachment was converted. The inner requirements are about the derived file, so they are not
   the document's and are not added to its decisions.

7. **`append` lays the converted file's pages after the document's** (ADR 1014: every mark on them is a producer's, the
   program's, and none is this converter's). Each derived page becomes two form XObjects — the page's content with its
   resources renumbered into the source's numbering and its `/Group`, inside a wrapper that also invokes, in order, the
   normal appearance of each annotation §12.5.3's flags show (not `Hidden`, `NoView` or a `Popup`) under §12.5.5's
   matrix — and the embedded file goes, its file specification answered "removed, and its pages appended from …" in
   `xmpMM:History`. The annotations do not come across as annotations: the archive's pages state none of their own.
8. **Bounds, each checked before the work it bounds**: the page count against `preserve::MOST_PAGES` (64) before any page
   is read, every object number against `MOST_IMPORTED_OBJECTS` (65 536) before it is taken, and a resource that reaches
   a `/Page` or `/Pages` refused rather than followed into the derived file's tree.
9. **The device colour the pages carry is licensed by the archive's own intent.** The inner file's output intent states
   which families it admitted (section 6.2.4.3, ISO 32000-2 Table 65's `/N`); the outer conversion counts those
   families among the device-colour rows its own output intent answers, so the archive's intent is written to cover
   them, and where no intent can be, stage three refuses rather than write appended pages in an unlicensed device space.
10. **A tagged document refuses `append`** by name: §14.8.2.2.1 would make the program's unbracketed marks artifacts, and
   naming a structure type for them is a decision this converter does not make; `placement = "attach"` is offered.

## Measured

Max RSS of `quorra-transform archive --to 2b` (the gates build) and its confined workers, by Python's
`RUSAGE_CHILDREN`, over a part-2 document holding one CSV attachment, the stand-in tool returning each PDF below:
| the tool returns | this tree, `attach` | this tree, `append` | HEAD (no stage two: refused, exit 4) | the same PDF as the source, HEAD / this tree |
|---|---|---|---|---|
| a 989-byte PDF without metadata | 11 776 KiB | 12 580 KiB | 11 972 KiB | — |
| one page whose content decodes to 1 GiB (2 086 083 bytes) | 1 076 436 KiB, 3.92 s | 2 125 972 KiB, 15.66 s | 23 392 KiB | 1 069 048 / 1 069 988 KiB |
| 64 pages of 1 024 forms each (10 598 855 bytes) | 260 216 KiB | 260 556 KiB, refused at `MOST_IMPORTED_OBJECTS` | 67 364 KiB | 240 044 / 239 140 KiB |

Stage two costs what the derived PDF costs this converter as a source document, within 1 % to 9 %, so it adds no
amplification of its own: the converter holds a page's decoded content while it checks it, under the reader's limits.
The appended bomb's 2 125 972 KiB is the same file's cost to HEAD's converter as a source (2 108 020 KiB, the appended
output re-archived), so the doubling is the checker's on a form invoking a form, not this construction's.

## Consequences

- A tool returning a PDF that already conforms at the inner target is copied by the identity conversion, so its bytes
  reach the archive unchanged, byte for byte.
- `run` and `run_pass` take a `&Budget`; `settle::second_conversion` opens under its limits instead of the defaults.
- `append` is RFC 0014 section 5's third stage at both parts; veraPDF found the appended and the drawn outputs compliant
  at 2b and 4, and a drawn output with its intent renamed fails section 6.2.4.3, which is the licence item 9 relies on.
