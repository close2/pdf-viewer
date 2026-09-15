# 1087 — the file a reference names is the one a person handed over

ADR 1101. §8.10.4's reference `XObject`s, built as far as principle 3 allows.

**Census first (trap 8).** `examples/absence_audit` gained a `--cached` scope: §8.10.4's row said
"67 195 of the 67 460 PDFs on this disk" and nothing had walked the remainder. Now: **0 witnesses in
`corpus-cache/openpreserve` and `tika-issue-tracker`'s 23 342**, 0 in the curated 1479, 0 in the
crawl's 65 944 (session 853's). **The second half is empty by construction**: no document states a
`/Ref`, so none names a target file to match. Calibrated against `/Group`.

**The design is ADR 1076's.** §8.10.4.1 addresses a `shall` to each of two processor classes, and
which this is depends on what a *host* supplied. So: `reference::Supply` (`NONE` the default),
`Command::References(ReferenceFiles)` carrying bytes (outside reads the disk, inside parses the
PDF), `viewer_host::reference_files`, `--reference-files <dir>`, confined kind 29,
`quorra_reference_files`. **The match is §14.4's identifier and that clause states it**: the
permanent string decides which file, and a moved changing string is Table 95's own warning, drawn
*and* said. No path is read (ADR 1101).

**What is drawn** is §8.10.4.1's placement rule — §8.10.1's steps b) and c) over another document's
page: the proxy's `/Matrix`, its `/BBox` as the clip, its `/Group` where stated. §8.10.4.3's first
consideration goes inside that box — `Interpreter::annotations_clipped_to` puts an imported page's
appearances under the page's clip — and its second is the `may` it is. **Every memo the interpreter
holds is swapped with the document** (`ImportedFrame`, exhaustively destructured): two files hand
out the same object numbers. Nine tests over a hand-built pair, four colours so each assertion
excludes a named wrong answer; three planted defects, each caught by the tests it should be: the
proxy's clip not reaching the annotations, the annotation clip unset, `resource_tables` not
swapped — the last visible only in a decoy shading the target's own file states and never paints.

**§8.10.4, §8.10.4.1 and §8.10.4.3 `partial` → `implemented`**; §8.10's aggregate note narrowed to
§8.10.2's unread entries. The one residue moved to **§11.4.7's** row, where its clause is: such a
page is treated as a transparency group using the *target page's* own Group attributes dictionary,
and §8.10.4.1's last sentence is the proxy's and is honoured.

fmt and clippy `-D warnings` 0 on this diff; what differs is a sibling's in-flight `nchannel_census`
and `notes.rs`. `nextest --workspace` 4915/4915; doc 0; fuzz fmt and clippy 0; conformance 251 + 18,
0 failures. `raster_golden` **held 974, moved 0**, as predicted: no gate supplies a reference file.
`pdf-model --test corpus` 0, every ratchet at its ceiling; `viewer-confined --test awkward_classes`
killed 0; `viewer-ffi` 18 + 1 + 3 + 2 + 3, entry points 182 → 183. **Trap 1 paid by eye** at 8×,
imported and proxy: nothing spills past the box.
