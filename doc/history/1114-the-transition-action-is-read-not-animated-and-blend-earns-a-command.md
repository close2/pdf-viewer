# 1114 — The transition action is read, not animated, and /Blend earns a command

Contract: §12.6.4's presentation-action residues — the `partial` rows that are not the excluded
ECMAScript/embedded-go-to ones. Only **§12.6.4.15 (transition actions)** has that shape; §12.6.4.8
(URI) is `partial` for a host-policy reason (reaching a resource), not a timing behaviour, and the
other siblings are `implemented`/`reported`/`out-of-scope`.

## The decision: §12.6.4.15 stays `partial`

Read against Table 164, Table 219 and §12.4.4.1. The styles are **not misread**: `navigation.rs`
reads all twelve of Table 164's `/S`, defaults an absent one to `R`, keeps a thirteenth name
(`/Blend`) as `Unrecognised` reported by name. The row is `partial` for two owed things, so not a
pure `departed` (which needs *one* decided departure, nothing else owed):

1. **Four styles no frame is shaped for** — `Blinds`, `Glitter`, `Dissolve`, `Fly` — read and
   reported by name but not animated: presentation-fidelity debt shared with §12.4.4, owed toward
   Acrobat-class presentation, not a decided-against departure. So no ADR.
2. **Outside presentation mode nothing animates** — a documented choice for a page viewer (the
   target page *is* the transition's end state), already named in the row and code. §12.4.4.1
   conditions a *page's* `/Trans`/`/Dur` on presentation mode; §12.6.4.15 does not, so the action
   reached outside it is a loud departure. No row moved; no ADR 1125.

## Measured first (trap 8), under the lock

Curated corpus: `refused_action_census` finds **0** `/S /Trans` actions; `presentation_census` finds
**0** page `/Trans`/`/Dur`/`/PresSteps`. The crawl's `/Blend` (14 actions in `7680405.pdf`) is prior.

## What was added

- **`navigation.rs` calibration test** (trap 13): a page's `/Trans`+`/Dur` are read, and interpret
  gives the display list the same page without them gives, against a control whose marks differ.
- **`refused_action_census` style breakdown**: for a `/S /Trans` action it now reads `/Trans` via
  `navigation::transition` and prints Table 164's `/S` in its own section, so `/Blend` is
  reproducible by command, not by hand. Ledger note updated to say so.

## Gates

fmt (my files clean; `fmt --all` differs only on a sibling's `pdf-transform`); clippy `-p pdf-model`
0; nextest `-p pdf-model` 1507 passed; `--test actions` 0; `--test corpus` ratchets at ceiling, slack 0;
`raster_golden` **held 974, moved 0**; `-p conformance` ok; pdf-model doctests ok.
