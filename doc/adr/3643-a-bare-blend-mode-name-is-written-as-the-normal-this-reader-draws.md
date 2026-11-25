# 3643 — A bare blend mode name the standard does not define is written as the `Normal` this reader draws, at the loss `content-as-drawn`

Session 2235. Status: **accepted and built**; extends ADR 0955 (the split by shape), ADR 3558
(`content-as-drawn` for what this reader decided) and ADR 3621 (`model/blend-mode-unknown`).
Context: the owner's PDF/A stance — archive every PDF it can see, the elegant remedy first, else a
stated loss; where the input is bad, bake in the state the viewer shows (A694). Clauses: ISO
32000-2 §8.4.5 Table 57, §11.3.5 Tables 134 and 135, §11.6.3; ISO 19005-2 sections 5.1 and 6.2.10,
ISO 19005-4 sections 5.1 and 6.2.9.

## Context

§11.6.3 states that the `/BM` value "shall be either a name object, designating one of the
standard blend modes listed in" Tables 134 and 135, "or its value shall be an array of such names",
and for the array only: "a PDF processor shall use the first blend mode in the array that it
recognizes (or Normal if it recognizes none of them)". For a single name outside the tables it
says nothing. So `Normal` is the standard's meaning for the array form and **not** for a bare
name — round 2230 read it so, and this ADR does not claim otherwise. This reader draws such a
name as `Normal` (`pdf_model::content::ext_gstate::blend_mode`) and logs `model/blend-mode-unknown`
at the content stream that set it, which section 5.1 counts as a repair; section 6.2.10's row
fails at the dictionary. Until now the bare-name half was refused outright (`doc/questions/A48`:
"never fill in an absence").

## Decision

1. **The bare-name half is answered at the loss `content-as-drawn`.** `/BM /Name` outside Tables
   134 and 135 is restated as `/Normal` by the same rewrite that answers the array half
   (`Rewrite::BlendModeNormal`); the row is `Conditional::PerDocument` and its decision becomes
   `Loses(ContentAsDrawn, BlendModeNormal)` wherever a bare name was found. A profile that does
   not grant the loss stops there by the loss's name, as before by the row's.
2. **Why a loss and not a statement.** No sentence of the standard makes the name `Normal`, so this
   is not ADR 0948's `Stated` class: another reader may composite differently. What is written is
   exactly what this window drew, which is `content-as-drawn`'s definition (ADR 3558), and the
   producer's name is gone and named in the report. A48's line is held by saying so, not by
   refusing the file.
3. **Section 5.1's `model/blend-mode-unknown` is answered where the rewrite restates the name**
   (`base_standard::settled`), so the page is not photographed for it; the output's own check holds
   the rewrite to having reached every one.
4. **A value that is neither a name nor an array** stays refused: there is no name to restate.
5. Shape `name` of both blend-mode rows is now answered; the configuration test of a shape
   qualifier takes `annotations/normal-appearance-shape`, whose `no-appearance-state` half has none.

## Why this is on the near side of the authoring line

No mark is invented: the composite the page shows is the one the window showed.
