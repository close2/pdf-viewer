# 2774 — The machine's face is embedded where the profile withholds its loss

Session 1981, a single archive round.
Status: accepted and built: `prepare/font_rewrites.rs` sets `Routes::embed_machine_face` from the
plan's authorisations.
Context: ADR 2773 (the construction), ADR 2770 (the same choice for the fitting), ADR 2705 (a
construction chosen by whether the profile authorises a loss), ADR 2162 (the loss `machine-face`).

## Decision

**The construction follows the loss, as the fitting does.**

- **Where `machine-face` is withheld** — `as-shown`, `refuse-any-loss`, and every profile that does
  not name it — the machine's face the window drew goes in where its licence permits (ADR 2773).
  The output draws what the window drew, and nothing is lost.
- **Where it is authorised** — `as-if-printed` and `state-every-loss` — the shipped face goes in and
  the loss is named, as before. A profile that authorises the loss has not asked for it, and the
  shipped face makes the output the same on every converting machine: `as-if-printed` is the page as
  any reader of this program prints it, whatever fonts the converting machine holds.

Writing the machine's face under every profile would change every `as-if-printed` output whose
window drew a face of this machine's, and would make those outputs depend on the machine that
converted them. That is a change of what the printing profile means, and is the owner's to make.

## Consequences

`as-if-printed` and `state-every-loss` outputs do not move. If the owner prefers the machine's face
everywhere, the change is the one line that computes `embed_machine_face`.
