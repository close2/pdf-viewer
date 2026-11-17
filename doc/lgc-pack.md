# The Latin-Greek-Cyrillic pack — Noto Serif and Noto Sans Mono, the third download

ADR 2630 decided it and ADR 2934 built it: Noto's `latin-greek-cyrillic` family, as a second font
pack beside the CJK pack (A569's shape, A737 item 1), for a simple font the file does not embed
whose shown codes the Foxit serif or fixed face this program ships cannot all draw. Those faces
hold the Standard Latin set and nothing of Greek, Cyrillic or Latin Extended-A, so such a font has
been refused, or its pages photographed; the pack's face draws it as text.

## What it is for

A `Type1` font dictionary that states a descriptor and no program, whose name asks a serif or an
upright fixed-pitch face, and which shows a code the shipped face has no glyph for. The face of the
pack for the same style is embedded in place of the shipped one, for the whole font: one design
draws every code, and the file's widths are restated into it so no glyph moves. An `Adobe-Identity`
composite font re-keyed from its `/ToUnicode` (ADR 2378) asks it last, after the shipped face and
the CJK pack.

The face is the pack's and neither the producer's nor the one the window drew, which is the loss
`machine-face` (ADR 2162): `as-if-printed` discards it, `as-shown` stops. Ahead of it, where the
profile withholds that loss, comes the machine's own face the window drew the font from, where its
licence permits (ADR 2773). The viewer does not read the pack.

Not answered, each refused by its sentence as before: a `TrueType` dictionary (Table 124 admits a
bare CFF under no key of it, and the pack face is not restated as another kind), an italic
fixed-pitch request (Noto Sans Mono is published in no italic, and an upright face is not the
italic asked for), a sans-serif request (Liberation Sans already covers Latin, Greek and Cyrillic),
and the descriptor-less route of the standard fourteen, which embeds the shipped face and holds it
to the window.

## Layout

One directory, named `lgc-pack`, unpacked **beside the `quorra-transform` executable**, or anywhere
else and named with `--lgc-pack <dir>`:

| file | what it is |
|---|---|
| `NotoSerif-Regular.cff` | Noto Serif at weight 400, width 100 |
| `NotoSerif-Bold.cff` | Noto Serif at weight 700 |
| `NotoSerif-Italic.cff` | Noto Serif Italic at weight 400 |
| `NotoSerif-BoldItalic.cff` | Noto Serif Italic at weight 700 |
| `NotoSansMono-Regular.cff` | Noto Sans Mono at weight 400 |
| `NotoSansMono-Bold.cff` | Noto Sans Mono at weight 700 |
| `LICENSE.txt` | the family's `OFL.txt`: the SIL Open Font License 1.1 and its copyright line, verbatim |
| `SHA256SUMS` | each file's SHA-256, in `sha256sum`'s format |

Each program is a bare CFF, which Table 124 admits under a `Type1` dictionary as `/FontFile3
/Type1C` — the form the Foxit faces take — with every charstring desubroutinised so that its width
is its own first operand (ADR 2642). Every glyph is named by the Adobe Glyph List: one glyph under
each name the list gives a character the source maps, and `uniXXXX` where it gives none, so a
simple font's `/Differences` name reaches its glyph by §9.6.5's route.

`tools/state.sh` does not count the pack; `python3 -I -B tools/lgc-pack.py --source <dir> --out
<dir>` prints each face's glyph count and size.

**The program never fetches the pack.** A person downloads it — `quorra-lgc-pack-<n>.zip` on the
`snapshot` release, beside the binaries and the CJK pack — and unpacks it, which gives the directory
`lgc-pack`; without it, a font only the pack could draw keeps the shipped face's refusal, whose
sentence names the pack.

## How it is built

```
python3 -I -B tools/lgc-pack.py --fetch <empty dir>
python3 -I -B tools/lgc-pack.py --source <dir> --out <empty dir> [--zip <dir>]
```

The first downloads the inputs from the google/fonts commit the script names and refuses a file
whose SHA-256 is not the one it names: `NotoSerif[wdth,wght].ttf`, `NotoSerif-Italic[wdth,wght].ttf`,
`NotoSansMono[wdth,wght].ttf` and `OFL.txt`. google/fonts publishes the family only as variable
fonts, and the notofonts repositories hold no binaries on their default branch, so each face is one
static instance made by fontTools' `instancer` at its weight and the default width, and its
quadratic outlines are written as the cubic ones that draw the same curves. CI's `lgc-pack` job
runs both with fontTools 4.66.1's wheel fetched by its digest, then the pack's tests against the
built files, and on `main` uploads the zip for `snapshot.yml` to publish. `<n>` is the script's
`PACK_VERSION`, the next one only when a change moves a byte of the output, which is when the
digests `lgc_pack.rs` carries change.

**The licence.** ADR 2630 read `OFL.txt`: SIL Open Font License 1.1, one copyright line ("The Noto
Project Authors"), no Reserved Font Name, so the faces keep the family's name. The obligations are
the CJK pack's (A550's pattern): `LICENSE.txt` in the pack, the notice in `/NOTICE` section 8, and
every program the converter writes from the pack carries the licence verbatim in its stream's
`/Metadata` packet under `xmpRights:UsageTerms`. Every source states `fsType` 0, installable
embedding.

**The build is reproducible**: two builds from the same inputs under fontTools 4.66.1 wrote the
same bytes, and `crates/pdf-transform/src/archive/fonts/lgc_pack.rs` carries those bytes' digests.
A pack built from anything else is refused by name rather than embedded.
