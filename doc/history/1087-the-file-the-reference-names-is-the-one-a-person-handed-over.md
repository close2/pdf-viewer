# 1087 — the file a reference names is the one a person handed over

Date: 2026-09-15. ADR 1101. §8.10.4's reference `XObject`s, built as far as principle 3 allows.

**Census first (trap 8).** `examples/absence_audit` gained a `--cached` scope, because §8.10.4's row
said "67 195 of the 67 460 PDFs on this disk" — a population with a stated remainder nothing had ever
walked. It walks it now: **0 witnesses in `corpus-cache/openpreserve` and `tika-issue-tracker`'s
23 342**, 0 in the curated 1479 (this session), 0 in the crawl's 65 944 (session 853's, not re-run).
**The second half is empty by construction**: no document states a `/Ref`, so none states a `/Ref`
`/ID`, so no target file named by one can be here to be matched. Calibrated against `/Group`.

**The design, and it is ADR 1076's.** §8.10.4.1 addresses a `shall` to each of two processor
classes, and which one this is depends on what a *host* supplied. So: `pdf_model::reference::Supply`
(opened once per file, `Supply::NONE` the default, `interpret_with_fonts` **is**
`interpret_importing` with it); `viewer_core::Command::References(ReferenceFiles)` carrying bytes,
outside reads the disk and inside parses the PDF; `viewer_host::reference_files`;
`--reference-files <dir>`; confined command kind 29; `quorra_reference_files`. **The match is
§14.4's identifier and that clause states it** — "[i]f only the first identifier matches, a
different version of the correct PDF file has been found" — so the permanent string decides which
file and a moved changing string is Table 95's own warning, drawn *and* said. Nothing reads a path,
because a path a document writes would let the file choose what this machine opens.

**What is drawn** is §8.10.4.1's own placement rule, which is §8.10.1's steps b) and c) over another
document's page: the proxy's `/Matrix`, its `/BBox` as the clip, its `/Group` where it states one.
§8.10.4.3's first consideration goes inside that box — `Interpreter::annotations_clipped_to` is the
field that puts an imported page's appearances under the page's clip where the containing document's
own are under nothing — and its second is taken as the `may` it is. **Every memo the interpreter
holds is swapped with the document** (`ImportedFrame`, exhaustively destructured), because two files
hand out the same object numbers; `across` became an `Option` because a `FontCache` empties itself
when other bytes arrive. Nine tests over a hand-built pair, four colours so each assertion excludes
a named wrong answer, and three planted defects each caught by exactly the tests it should be: the
proxy's clip not reaching the annotations, the annotation clip unset, and `resource_tables` not
swapped — the last drawing the target page in a decoy shading its own file states and never paints.

**§8.10.4, §8.10.4.1 and §8.10.4.3 `partial` → `implemented`**; §8.10's aggregate note narrowed to
§8.10.2's unread entries. The one residue moved to **§11.4.7's** row, where its clause is: a page
used this way "shall be treated as a transparency group using the page Group attributes dictionary"
— the *target page's* own, where §8.10.4.1's last sentence is the proxy's and is honoured.

fmt 0 (mine; a sibling's `nchannel_census.rs` differs); clippy `-D warnings` 0 on this diff — one
`too_many_lines` left in `viewer-core/src/notes.rs`, a sibling's 488-line in-flight edit;
`nextest --workspace` 4915/4915; doc 0; fuzz fmt and clippy 0; conformance 251 + 18, 0 failures.
`raster_golden` **held 974, moved 0**, as predicted — no gate supplies a reference file, and no
corpus document states a `/Ref`. `pdf-model --test corpus` 0, every ratchet at its ceiling;
`viewer-confined --test awkward_classes` killed 0; `viewer-ffi` 18 + 1 + 3 + 2 + 3, entry points
182 → 183. **Trap 1 paid by eye**: the pair rendered at 8×, imported and proxy, nothing spilling
past the box.
