# 2087 — A font stating no descriptor, and an italic fixed-pitch request

Session 2087, a fonts round in the lane3 worktree, on three font items records 2062 and 2064 left. ADRs 3046-3048.

**The premises.** Item 1 had moved for a name-alone standard font (ADR 2948) and held for a font stating no
descriptor: HEAD's own test pinned `/Arial` at 0 for a Cyrillic name, and HEAD's code advanced code 10 and a code
outside a `/Widths` range by 0. Item 2 held: `WIDTHS_WITHOUT_A_DESCRIPTOR` refused a font stating `/Widths` and no
descriptor; ADR 2522's fonts need nothing. Item 3 held: `for_request` gave an italic fixed request `None`.

**Item 1 (ADR 3046).** Table 109 sends a code outside `/Widths` to the descriptor's `/MissingWidth`, ADR 2898's third
step; with no descriptor neither is stated. `metrics::painted_glyph_widths` gives such a code §9.2.4's width of the
glyph painted: the drawn face's own, or the `.notdef` §9.6.5.2 substitutes. The owner's order (A859, A868) stands
wherever a descriptor is stated; `SuppliedWidths::from_drawn_face` reports the rest. The archive's new descriptor
would read 0 there, so `restated_entries` states the window's advance for every code and `keep_stated_widths` puts
the file's own entries back; two fitted-Helvetica tests now read `H`'s width instead of the whole array.

**Item 2 (ADR 3047).** §9.2.4 positions by `/Widths`; §9.5 and §9.8.1 leave the substitute to the processor, and the
tree's answer (a request from `/BaseFont`, the face drawn to the widths) is stated as a choice. `Layout::TheFiles`
converts the font: a descriptor measured off the shipped face, the file's widths kept, no `layout-from-this-viewer`
loss. This supersedes ADR 2504's second refusal.

**Item 3 (ADR 3048).** `tools/lgc-pack.py` writes `NotoSansMono-Oblique` and `-BoldOblique`, the upright outlines
sheared 12 degrees (`Courier-Oblique.afm`'s `ItalicAngle`), `PACK_VERSION` 3; two builds identical, the six old
digests unchanged. No owner question: no font file enters the repository, and CI builds the pack.

**Evidence.** veraPDF 1.30.3 passes six outputs (`/Arial` with widths, `Type1` and `TrueType`, at 2b and 4;
`Courier-Oblique` and `-BoldOblique` at 2b). Rendered and looked at: `Hello` set evenly; a-ogonek, n-acute and
z-dot-above lean right, no overlap. **Plants**: `painted_glyph_widths` off fails three pdf-font tests; the old
refusal fails both archive width tests; `for_request`'s italic arms removed fail the oblique and the unit test.

**Ledger.** 9.6.2.1, 9.2.4, 9.8.2 and 9.9 stay `implemented`, with new tests; notes on 9.6.2.1 and 9.8.2.

**Gates.** Over `pdf-font`, `pdf-transform`, `pdf-model`: fmt exit 0; clippy `-D warnings --all-targets` exit 101 on
two pdf-font lints, then 0; nextest `--lib --bins --tests` under `xvfb-run -a` 4535 of 4535 passed, exit 0, after a
first run's 5 archive failures (a descriptor-less font with `/Widths` now states every code) were fixed; rustdoc
exit 0; conformance after `git add -N` exit 101 on this record alone, then 0; `cd fuzz && cargo check --bins` exit 0.

**Left.** A shipped face restated to arbitrary `/Widths` on the descriptor-less route (still `WIDTHS_NOT_THE_FACES`);
the pack for a `TheFiles` font; CI to publish `quorra-lgc-pack-3.zip`.
