# 2705 — A configured press stands in for the shipped press, and for nothing else

Session 1945, a single archive round.
Status: accepted and built in `pdf-transform` (`archive/prepare/colour_and_graphics.rs`:
`shipped_for`, `ProfileSource::Configured`; `archive/supplied_cmyk.rs`: `Press::Configured`,
`CONFIGURED_PARAMETERS`; `archive/decision/colour_and_graphics.rs`:
`OUTPUT_INTENT_STATES_THE_CONFIGURED_PRESS`).
Context: A784, ADRs 2130, 2518, 2700, 2701, 2704.

## Decision

The press an operator's configuration names (ADR 2704) is written exactly where this program would
otherwise write `GRACoL2006_Coated1v2.icc`: where the plan authorises `press-assumed`, as the CMYK
output intent ADR 2701 adds for `DeviceCMYK` the file never characterised, and as part 4's
`/DefaultCMYK` beside the file's own intent of another family (ADR 2518). **It does not replace the
assumed inks** that `as-shown`, `no-data-loss` and `state-every-loss` state: that profile exists so
that the archive draws as the source did (ADR 2700), and any real press moves the drawing (ADR 2691)
— a loss those profiles do not take.

**It keeps the loss.** Where the shipped press costs `press-assumed`, so does the configured one:
nothing in the file names its press, and this renderer drew its `DeviceCMYK` through its own inks,
so the colours move whoever chose the press. What changes is whose press is named. The report
states it as `configured` — "the press your configuration names" — with its own `desc` and `cprt`
(`doc/pdf-a-conversion-limits.md` section 10.1); the intent's sentence says the press is the
configuration's; the `/DefaultCMYK`'s `xmpMM:History` entry says the operator named it in their
configuration and claims no provenance for this document.

**Why it is not ADR 2130's `Stated`.** `--output-intent-profile` is given for a run, by a person
who knows which press this document was made for, so it is their statement about this file and
costs nothing. A configured press is named once for every document whose press nobody stated; it
is the operator's choice of which press to assume, not a statement about any one file.
