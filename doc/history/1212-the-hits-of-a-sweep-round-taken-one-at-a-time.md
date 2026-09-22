# 1212 — Round 1206's sweep hits taken one at a time, and an erratum repair that has no first branch

Consolidation round of batch thirty-two. ADRs 1261, 1262.

## The §8.11 family closes, and the ring it was held open by

§8.11, §8.11.1, §8.11.4 and §8.11.4.1 were each `partial` on the `Print` and `Export` events of
§8.11.4.5, built by ADRs 1173 and 1174; three then rested on the fourth, which rested on the retired
sentence. §8.11.4.1 is one paragraph of pointers whose `may` is a document's permission and whose
`shall` is carried out by §8.11.4.4 and §8.11.4.5, both `implemented` — so all four move to
`implemented` and `doc/todo/65`'s aggregate list loses them (ADR 1261). §8.11.4.5 kept the same
sentence twice beside its own correction; both deleted.

## Three notes whose present tense had outlived the code

§12.2 called Table 147's print half and §8.11.4.5's `Print` event "one missing program capability";
§12.5.3 opened "Print is a printing decision this viewer does not yet make"; §7.5.7 said `/Extends`
is unread "for a *writer*" while `serialize.rs` writes it and chains its carriers. Each says what is.

## Two misquotations

§11.3.6 had `αs` "control[s] the influence of the backdrop and source colours"; the standard makes
the two alphas control the two colours *respectively* and their product control the blend function,
so the bracket had moved the subject. §12.8.3.4.5 quoted RFC 5035 where the nearest sentence on this
disk is an ETSI one no quotation may reproduce; paraphrase now.

## The erratum repair has one branch, not two

`doc/md/` is the sponsored PDF's conversion and that PDF carries Errata Collection 3 as annotations
the conversion drops (ADR 0252), so no amended sentence exists to quote and the gate would refuse
one. The six source sites keep the 2020 words and say the collection retired them, in verbs
`spec-errata applied` recognises; its read-first list falls 14 → 8 (ADR 1262).

## The rest

`print_preferences.rs` no longer calls printing an operation this program does not perform;
`write.rs` refuses five documents, not four; `doc/raster-gpu-coverage.md` was the rename and is
`doc/quorra-gpu-coverage.md`; four `pdf-render` sentences counting two backends count every backend
now, two naming their own pair left alone; `oracle.rs`'s superseded ssim figures are the gate's to
print; `pdf-signature`'s header missed `ess`, `revision`, `timestamp` and `policy`.
