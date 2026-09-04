# 0881 — Four hosts that can offer a face, and what the port is worth over the corpus

Session 920. Status: **accepted**. The second of this round's two records: which hosts can turn
`doc/todo/59`'s resource port on and how, and the measurement session 914 traded away, taken again
with the port on. ADR 0880 is the construction underneath.

## Context

ADR 0880 built a port that provides nothing until a host says otherwise. A port that can be opened
and is not is the same shortfall it was before — which is the sentence ADR 0875 opened with about
the *ask* level, one item over, and it is as true here.

The owner's own description of what the hosts should do: "the cli would wrap the access with a
flag. GUIs could either have a setting, or ask the user." The last of those three is answered
rather than implemented, and ADR 0880 §7 has the argument: fonts specifically need no prompt,
because a prompt protects against a channel that does not exist here.

## Decision

### 1. The command line — `pdffs --machine-fonts`

The mount is the face with a command line, and RFC 0003 §7's usage line is the whole of its
interface. `--machine-fonts` sets `Config`-adjacent `ConfinedWorkers { faces }` and nothing else;
absent, the mount is what session 914 left. The refusal for an unknown flag is unchanged and is
trap 5's rule: an ignored `--machine-fonts` would be a mount somebody believes has its fonts.

**There is still no flag for *whether* the worker is confined**, and the module comment now says
why the two are different questions: `--machine-fonts` widens nothing the worker can reach on its
own, and a switch that turned the confinement off would.

### 2. The windows — `--machine-fonts`, or `PDF_VIEWER_MACHINE_FONTS=on`

`viewer_host::MACHINE_FONTS` is the word and `viewer_host::MACHINE_FONTS_VARIABLE` is the
environment name, beside `IGNORE_RESTRICTIONS` and for the same reason ADR 0604 gives about that
one: a host decision stated once, in `viewer-host`, rather than as a string literal in each window.
`viewer_host::offers_machine_fonts` reads the flag first and the environment only where the flag is
absent, and a word the variable does not define is *off* rather than a guess.

`pdf-viewer-confined` is the window this reaches, because it is the only one with a confined worker.
It takes the flag, and `Host::start_confined` is one function rather than two call sites — deliberate,
because the first worker and the one that replaces a dead one must be given the same thing: a window
whose *second* worker silently lost the port would draw a page differently after a crash, which is
the shape nobody would look for.

**The environment variable is there because a window is not usually typed at.** It is started from a
desktop entry or a file manager, so the flag reaches only the person who runs it from a terminal.
It is a poor interface and it is what exists until `doc/todo/38`'s user interface is asked for — the
same sentence, and the same limit, ADR 0875 recorded for `PDF_KIO_RESTRICTIONS`.

### 3. The KIO face — `PDF_VFS_MACHINE_FONTS=on`

`pdf_vfs_ffi::Tree::open` reads it. **Not a parameter**, and that is the C boundary rather than a
preference: ADR 0868 fixed that boundary at thirty-five functions and `pdfvfs_open` already takes
the restriction level; adding a second integer would be an ABI change for a setting the KIO worker
has no way to obtain except from its environment anyway (ADR 0875 §1). Anything but `on` — including
absence, including a word the build does not know — is off.

### 4. The FUSE face gets it through `pdffs`, and `pdf-transform` gets nothing

`pdf-fuse` is the library under `pdffs` and takes the setting from the program. `pdf-transform` is
not confined at all — it parses in its own process — so it has the machine's fonts already and there
is nothing for a port to give it.

| face | can it offer a face | how | default |
|---|---|---|---|
| `pdffs` | **yes** | `--machine-fonts` | withheld |
| `pdf-viewer-confined` | **yes** | `--machine-fonts`, or `PDF_VIEWER_MACHINE_FONTS=on` | withheld |
| the KIO worker | **yes** | `PDF_VFS_MACHINE_FONTS=on` | withheld |
| a C host of `pdf-vfs-ffi` | **yes**, through the same variable | — | withheld |
| `pdf-viewer`, `pdf-viewer-gtk`, `pdf-viewer-qt` | **not applicable** | they run the interpreter in-process and have the fonts | — |
| `pdf-transform` | **not applicable** | unconfined | — |

## The measurement

`crates/pdf-vfs/examples/faces_on_the_port.rs`. Page one of each document at 150 dpi through
`pdf_vfs::Vfs`, three columns: `here` is this process, unconfined, with the machine's own fonts and
one rasterising strip; `withheld` is a confined worker offered nothing, which is the posture ADR 0870
left; `offered` is a confined worker with the port on. **The comparison is byte identity on the
mount's own PNG against `here`** — which is exactly what `tests/read_corpus.rs` holds the two
transports to on every document whose fonts are embedded, and had to exclude on the population this
port is about. Ink, the oracle's mean of `255 - luminance`, is printed beside it so that a
*difference* can be told from a *blank page*.

### The four documents ADR 0870 named

```text
document                         here   withheld    offered   verdict
XiaoBiaoSong.pdf                10.02       9.67      10.02   the port pays ADR 0870's cost back in full
SimFang-variant.pdf              2.55       2.54       2.55   the port pays ADR 0870's cost back in full
90ms_rksj_h_sample.pdf           0.25       0.11       0.25   the port pays ADR 0870's cost back in full
ThuluthFeatures.pdf             16.48      16.48      16.48   nothing was owed: withheld already matched

3 of 4 paid back in full, 0 still short
```

**The fourth is worth reading rather than skipping.** `ThuluthFeatures.pdf` is the Arabic one, and
ADR 0870 listed it because it was *killed*; the kill was fixed by `no_machine_fonts()` and it turns
out to have cost no fidelity at all — this machine's catalogue offers nothing better for that face
than the compiled-in one, so the withheld column already was the unconfined answer. A record that
had said "four documents lost their page" would have been wrong about one of them, and only the
measurement says which.

### Over the whole `doc/pdf.js` corpus

974 documents, page one at 150 dpi, all three columns, in 332 s under `tools/bounded.sh --data 12
--tree 12` at a 2.59 GiB peak:

| | |
|---|---|
| **40** | the confined page differed from this machine's own, and **with the port on it is byte-identical** |
| 918 | nothing was owed — the fonts are embedded, or standard-14, or this machine offers nothing better |
| 16 | every column refused the document, so there is nothing to attribute |
| **0** | offered and still different |

**Twelve of the forty went from a blank page to a drawn one**, which is the part of ADR 0870's cost
that was a page rather than a glyph: `issue2840.pdf` 0.00 → 21.20, `issue5244.pdf` 0.00 → 15.33,
`issue9084.pdf` 0.00 → 14.17, `issue13343.pdf` 0.00 → 8.97, `issue8372.pdf` 0.00 → 6.05,
`issue2128r.pdf`, `issue3521.pdf`, `issue11555.pdf`, `issue20065.pdf`, `noembed-eucjp.pdf`,
`noembed-identity-2.pdf` and `noembed-sjis.pdf`. Those are §9.7.4.2's case — a substituted composite
font is reachable only by character, so `installed_covering` answering `None` is a page with no
glyphs at all rather than a page in the wrong face.

### The first run of this measurement was wrong, and the reason is trap 10

It reported **124 documents "still short"**. 119 of them had a reference page with **no ink at all**,
and the five that did not were named `ccitt_…`, `jpx_…`, `images_1bit_…` and `S2.pdf`. The cause is
not fonts: `Column::Here` decodes §7.4.6's, §7.4.7's and §7.4.9's images by *spawning*
`pdf-sandbox-worker`, the confined columns decode them in-process because a confined process cannot
spawn (ADR 0218), and the worker was not beside the example's binary under
`target/release/examples/`. With it there the same command reports **0 still short**.

So the example now refuses to run without it and says why. **A difference attributed to fonts that
is really a missing decoder is exactly trap 16's shape**, and it is worth noticing that the failure
was *legible as a finding*: 124 documents where the port did not help would have been a plausible,
publishable, wrong result.

## Consequences

- `viewer-host` gains `MACHINE_FONTS`, `MACHINE_FONTS_VARIABLE` and `offers_machine_fonts`, which is
  where the next window that grows a confined worker will find them.
- `pdf-vfs` gains a `png` dev-dependency, for the example's ink and nothing else: no crate in the
  tree decodes an image except through the codecs, and ink needs the mount's own PNG read back.
- **`doc/todo/58` §4's shortfall is closed and `doc/todo/59` is not.** The port exists for fonts; ICC
  profiles (§14.11.5, RFC 0006 §5.3) are the named second resource and nothing speaks it yet, and a
  way for a person to *choose* rather than to set an environment variable is `doc/todo/38`'s.
- The two probes that pin it are `pdf-vfs`'s
  `a_face_this_broker_can_look_up_reaches_a_generator_that_cannot` and `viewer-confined`'s
  `a_face_this_host_can_look_up_reaches_a_worker_that_cannot`, each with the *withheld* column as its
  own calibration (trap 13): without that second assertion, a machine with no CJK face installed
  would pass the first for the wrong reason.
