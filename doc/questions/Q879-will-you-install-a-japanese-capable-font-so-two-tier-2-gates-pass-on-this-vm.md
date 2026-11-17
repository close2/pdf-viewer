# Q879 — Will you install a Japanese-capable font, so that two tier-2 gates can pass on this VM?

Source: the post-merge train's first run on main (fe09fccb, 2026-11-16) and a triage session (2050) that re-ran
both walks at 7f6511ab. Nothing waits on the answer: the gates are known to fail here, and rounds compare against
HEAD.

## What was found

Two tier-2 gates have failed on this VM since development moved here on 2026-11-03, and they fail identically on
every run:

- `pdf-model`'s `corpus`: the incomplete-drawing ratchet is joined by `issue15053.pdf` and `mixedfonts.pdf`. Their
  reports name the machine's face: "font /KozMinPr6N-Regular is substituted and the face this machine offers draws
  none of the 29 character(s) it is asked for (§9.10.2)".
- `pdf-model`'s `raster_golden`: 23 first pages move, all of them pages with Japanese or other unembedded fonts
  (the `noembed-*` family, `90ms_rksj_h_sample`, `mixedfonts`, `issue15053` and the like).

No commit since the move changed these pages. This VM's only face that covers Japanese is Droid Sans Fallback
(`fc-list :lang=ja`); the installed font packages are `fonts-dejavu-core`, `fonts-dejavu-mono`,
`fonts-droid-fallback`, `fonts-noto-mono` and `fonts-urw-base35`. The golden entries were recorded on the notebook,
which had other faces. Whether a CJK face makes both gates pass is untested.

## The question

Fonts are system packages, so installing one is yours. The options:

- **(a) Recommended: install `fonts-noto-cjk`** (Debian trixie). A round then re-runs both walks. A page that
  still moves is then a real finding and is treated as one: either a golden entry is restated, citing its clause,
  or the code is fixed.
- **(b) Make the gates machine-aware**: entries that depend on a substituted face are marked, and a machine
  without the face skips them by name. This is more code, and it keeps the VM's runs from checking those pages.
- **(c) Restate the 23 entries and the two names to this VM's output.** Not recommended: a golden recorded with
  Droid Sans Fallback would fail on any machine with a better face, and would hold a substitute's look as
  "correct".
