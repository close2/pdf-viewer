# 2568 — An action a removed annotation performs goes with the annotation

Status: implemented — session 1867, the robustness archive slot of batch 127. Context: ADRs 0947
section 2 (nothing is changed that no failed requirement asked for), 1099 (a forbidden subtype is
removed under `forbidden-annotation`), 1105 (a row whose every place is a removed annotation is
the removal's) and 2442 (a row met by another rewrite takes that rewrite's decision).
Clauses: ISO 19005-2 sections 6.3.1, 6.5.1 and 6.5.2; ISO 19005-4 sections 6.3.1, 6.6.1 and
6.6.3; ISO 32000-2 §12.6.2 (Table 196's `/Next`), §12.6.3 (Tables 197 and 198), §12.6.4.14.

## The defect

`archive/actions.rs` walked every holder `pdf_archive::action_sites` hands out, the annotations the
section 6.3.1 subtype rows remove among them. A Screen annotation's §12.6.4.14 Rendition action
was therefore listed as removed, its object edited, and the six action-type rows charged the
`interactive-behaviour` loss for an action tree the output does not hold: a document whose only
multimedia action sat in a removed Screen was refused at PDF/A-4 under `forbidden-annotation`
alone. ADR 2536's media-clip test authorised `interactive-behaviour` for exactly that reason.

## The decision

1. **The walk reads the removal first.** `actions::prepare` takes the objects
   `prepare_forbidden_annotations` removes and does not walk those holders: nothing is taken out
   of their trees and nothing is listed in `removed_actions`.
2. **An action row met by the removal takes the removal's decision**
   (`actions::met_by_the_removal`, after `met_by_the_replacement`). The validator names a failed
   action by its own object, an action written into its holder by the holder's, and visits a
   shared action once under whichever holder came first; so a row is met only where every object
   its `named` set holds is reached from a removed holder and from no kept one, the set is not cut
   short and no place named no object. The two reaches are walked from the validator's own roots
   (`/A`, each `/AA` entry, the catalog's `/OpenAction` and `/JavaScript` tree), each object once,
   at most 32 `/Next` links below a root; a chain cut at the bound meets nothing.
3. **Only a removal that happens meets anything.** Where a subtype row's decision does not proceed,
   or none was decided, the action row keeps its own decision, so a refusal is reported once at
   the row it belongs to. ADR 2442 names the stopping decision at every row it meets; that is not
   taken here, because the census showed it adding the subtype row's refusal under the action row
   to three refused documents, which `rank.py` would count as a second reason.

## What it moved

Against HEAD's census, without and with the CJK pack, no document moves: the two census profiles
answer the action rows with `discard`, so the defect reached a caller or a profile that accepted
the annotation's removal and not the actions' — whose decision the removal's now replaces, a
configured one included. `tests/archive/removed_annotation_actions.rs` is the case and
its plants: the same action on a kept Link still costs the loss, one action object both holders
perform is the kept holder's, a chain past the bound is charged, and a refused removal meets
nothing.
