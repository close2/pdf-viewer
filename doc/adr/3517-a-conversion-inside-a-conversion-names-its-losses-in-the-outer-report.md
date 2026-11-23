# 3517 — A conversion inside a conversion names its losses in the outer report

Session 2204. Status: **accepted**; built. Context: ADR 3509's open item, ADRs 2536, 2539, 2652;
`doc/rfc/0014` section 5; ISO 19005-2 section 6.8 and ISO 19005-4 section 6.9 (paraphrased).

## What was wrong

A PDF an embedded file holds, drawn as pages (`remedies.rs`, `preserve` with `placement = "append"`),
and a PDF a tool returned (`derive:<tool>`), are each converted by `derived::convert` before anything
of them reaches the archive. That inner run decides under the outer plan's authorisations and keeps
its own `Conversion`, and the outer run threw it away: the outer report said "laid out on page(s) N",
the derived row "converted here to PDF/A-2b", and neither the report nor `xmpMM:History` said what the
inner run lost. `digitally_signed_3D_Portfolio` under `as-shown` is the witness: its inner tagged
PDFs' marks stand outside their trees (ADR 3509), and nothing outside said so.

## What is built

1. **`Conversion::losses`** — every loss one conversion took, once each: the union of its
   authorised decisions' losses and of the losses rows record beside their decision (a substituted
   face's, a photographed page's, marks outside the tree, a recovered structure and the pages it
   does not hold, an assumed press).
2. **Carried per attachment.** `AttachmentOutcome::lost` for a file drawn or appended as pages, and
   `ConvertedTo::lost` for a derived file. The sentence the report and `xmpMM:History` share gains
   "; this converter converted it first, losing …", the derived row's "… decided, losing …", and the
   JSON `losses_of_its_conversion` and `converted.losses`.
3. **Asked once, never widened.** The inner plan is the outer plan with the target swapped and no
   program or drawing (ADR 2536): its authorisations are the outer plan's, field for field, so the
   operator's one answer decides both runs, and a loss the operator did not authorise is a refusal
   inside. That refusal now carries each inner refusal's own sentence rather than its requirement
   alone, so the outer row names the loss that was missing.

## Proof

`tests/archive/tagging_of_preserved_marks.rs`: an untagged document holding the tagged fixture, drawn
at 2b and 4 with the loss authorised, names `tagging-of-preserved-marks` on the attachment's outcome,
in the report and in the packet, while the outer run's own `losses()` holds no such loss; without the
authorisation the file takes the row's `on-failure` and the reason names `tagging-of-preserved-marks`.
