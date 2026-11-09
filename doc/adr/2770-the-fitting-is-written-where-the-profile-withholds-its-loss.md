# 2770 — The fitting is written where the profile withholds its loss

Session 1979, a single archive round.
Status: accepted and built: `prepare/font_rewrites.rs` sets `Routes::write_fitting` from the plan's
authorisations.
Context: ADR 2769 (the construction), `doc/questions/A566` and ADR 2262 (the unmodified face under
the printing profile), ADR 2490 (each profile's remedy), ADRs 2701 and 2705 (a construction chosen
by whether the profile authorises a loss).

## Decision

**The construction follows the loss, as the press does.** ADR 2705 states the shipped press where
`press-assumed` is authorised and this reader's own press where it is withheld. The fitting is
chosen the same way:

- **Where `substitute-stretch` is withheld** — `as-shown`, `refuse-any-loss`, and every profile that
  does not name it — the face goes in with the fitting written (ADR 2769). The output draws what the
  window drew, and nothing is lost.
- **Where it is authorised** — `as-if-printed` and `state-every-loss` — the face goes in unmodified
  and the loss is named with the scale it removed. That is A566's answer. The owner chose it for
  the printing profile because a modified program becoming routine output costs more there than
  the fit it would keep; the fit is at most 0.026 % of a glyph's width on the documents A566 was
  asked over and at most 0.72 % on the census's.

A profile that authorises the loss has not asked for it. The alternative, writing the fitting under
every profile, would change every `as-if-printed` output that substitutes Liberation. The census
counts 38 such documents among the ones R09 names, and more among the rest. A566 states the
contrary preference, so that change is the owner's to make, not a consequence of this build.

## Consequences

`as-if-printed` and `state-every-loss` outputs do not move. If the owner prefers the lossless face
everywhere, the change is the one line that computes `write_fitting`.
