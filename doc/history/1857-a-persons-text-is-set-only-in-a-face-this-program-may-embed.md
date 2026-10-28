# 1857 — A person's text is set only in a face this program may embed

Slot 7 of batch 126, 2026-10-28, a "clause" round (Opus 5.5), in three sessions; ADRs 2548 and 2549 used, Q723 left unused.
I worked in `scratchpad/r1857/` and in two exports built there, their targets deleted; sessions one and two handed over.

**Premise.** "(a) is a filter on one chooser": it held. The chooser is `pdf_font::substitute::installed_covering` as
`variable_text::machine_set` and `machine_faces_for` ask it; `viewer-core/src/viewer/editing.rs` chooses no face. "ADR
0304's case can be met by a shipped face": in part — Latin text under a `/DA` that sets nothing could be, text no face
covers stays owed, so §12.5.2 cannot reach `implemented` on (a). "(b) is the stated pair applied before the profile's
table": it held, as a replacement of the table rather than a step before it.

**Built (ADR 2548).** `embed::may_be_embedded` (the licence and outline halves of `for_embedding`'s refusals, from the
table directory alone); `installed_covering_embeddable`, walking `installed_covering`'s own ranking (`covering_paths`,
shared with the broker's `machine_face`) past each refused face, at most `EMBEDDABLE_CANDIDATES` = 8 reads; the layout's
machine faces come only from it. Found and fixed on the row: `write_added_appearance` wrote an added annotation whose
layout declined with no `/AP` and answered `true`; it is now named in `Written::unappeared`.

**Built (ADR 2549).** A stated pair is §10.4.2.4's arithmetic for `DeviceRGB` whatever the press; with none, the
profile's `B2A` is the device default; the pair is withheld where the target is `ICCBased` (ADR 2170), in `Conversion`,
`Interpreter::colour` and at a `Do`. `Unsupported::BlackGeneration` is gone. A `/BG` or `/UCR` that is not a function
sets nothing and is now said (`Unsupported::UnreadableBlackGeneration`, the file's); `endchar.pdf` joins the list.

**Ledger.** §12.5.2 stays `departed`, on ADR 0304's case alone, the owner's decision of 2026-10-28 on the row;
§11.7.5.3 `departed` → `implemented`, its test list naming tests that exist.

**Plants.** (a): an `fsType` 2 face offered first and refused; the bound read-counted at 8; the silent case back fails
`an_added_note_no_face_can_set_is_named_rather_than_written_bare_in_silence` (no face here is restricted). (b): the old
guard back fails 3 tests; each withholding filter removed fails its test (the `Do`'s,
`a_device_rgb_group_into_an_icc_based_parent_takes_no_pair_at_the_do`); the silent parse arm back fails both new tests.

**Gates.** Part (a), in its export: `rustfmt --check` 0; clippy `-D warnings` 0; nextest `-p pdf-font -p pdf-model -p
viewer-core` 0, 3 078 passed, 24 skipped. Part (b), in its export: nextest `-p pdf-colour -p pdf-model -p viewer-core` 0,
2 933 passed, 25 skipped; clippy `-D warnings` 0; the pdf-model corpus gate 101 (`endchar.pdf` joined the incomplete
population; listed with its reason) then 0; `raster_golden` 101 (`endchar.pdf` p1, reports only) then 0 after the
update, landed by `git merge-file`; the six arms 0 (968, 965, 968, 965, 968, 965 pages), every page's digest equal
against HEAD's six arms; the walk's lock held 3 181 s, 3.64 GiB peak. Shared tree: `cargo test -p conformance
--no-fail-fast` 101 — my blockquote filed under §10.4.2.4, fixed and rerun 0; the rest siblings' (1854's record path,
1858's and 1859's Gates); `--bin cited` 0, `--bin quotations` 0. Probe (debug `callgrind_interpret`, 2 000 isolated
`DeviceRGB` groups each under its own `/BG`): 723 884 KiB and 7.28 s against HEAD's 724 256 KiB and 57.28 s, load 47–50.
Durations 2 800 s, 3 900 s and 5 300 s.
