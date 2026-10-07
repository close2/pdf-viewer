# 1673 — The remedy listing names a split requirement's two halves

Session 1418. Status: **accepted** and **built**. Discharges the expired premise `doc/todo/65`
recorded against ADR 1012: the converter's verbs are carried out and `Qualifier::Shape` selects
against `decision::SHAPES` (ADR 1211), and what was left was the listing. Supersedes nothing.
Context: `doc/rfc/0007` section 3.1 (every site enumerable, from the table the converter decides
from); ADRs 1012, 1209, 1211.
Code: `crates/pdf-transform/src/archive/config.rs` (`Site::shapes`, `SiteShape`, `sites`),
`crates/pdf-transform/src/bin/quorra-transform.rs` (`print_remedy_sites`). Test:
`archive::config::tests::every_requirement_that_splits_into_shapes_is_listed_with_both_halves`.

## 1. What an operator met, and where

Eleven requirements in `decision::SHAPES` fail for two reasons under one identifier, and the two
take different answers — a blend mode written as an array reduces to a name every reader already
used, a bare name the standard does not define does not. A `[site."…".shape."<name>"]` row answers
one half and is inert for the other. `--remedy-sites` printed one remedy sentence per site and no
shape at all, so an operator learnt there were two halves only from the error a misspelt shape
produces — after writing a row for the half they did not mean.

## 2. The decision

**Every listed site that splits names its halves, and which half a row is carried out for**:
`shapes: stated-colour-space (answered), data-colour-space (keeps its refusal) — a
[site."…".shape."<name>"] row answers one half (doc/adr/1211)`. The column is `Site::shapes`,
filled in `sites` from `decision::shapes` — the same table `config::applies_to` reads — so the
listing cannot disagree with what a row does.

## 3. Four of the eleven are not sites, and that is the listing's own definition

Seven split requirements are sites under some target and are listed with both halves. Four —
the two blend-mode requirements, `graphics/content-streams-have-an-explicit-resources-dictionary`
and `annotations/normal-appearance-shape` — are sites under no target: their answered half is a
`Stated` or `Mechanical` remedy applied without any configuration, and a row for the other half is
inert, so no row a configuration could write changes what happens. `sites` enumerates "refusal
site(s) a configuration may answer", and ADR 1209's widening — a site is a requirement a row has
something to say about — excludes them for the same reason it included the conditional
`Mechanical` rows. The test asserts exactly that for each of the four, under every target that
binds it, rather than listing them; a split requirement that became a site would be held to the
column like the other seven. The documents their refused halves stop are named by the conversion
report, which is where a refusal nobody can configure is said.
