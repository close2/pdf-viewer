# ADR 1244 — Two transparency rows whose residue was not a requirement: §11.3.6 and §11.4.8

Status: accepted. Session 1203.
Applies: ADR 1220's reading — a residue recorded in a row is a claim about the clause, and one way
it decays is that the sentence has no modal verb at all — to clause 11's compositing family.
Context: `doc/conformance/ledger.toml` (§11.3.6, §11.4.8), `crates/render-cpu/src/blend.rs`,
`crates/render-cpu/tests/soft_mask.rs`.
Clauses: ISO 32000-2 §11.3.6, §11.4.8, §11.3.8, §11.4.4, §11.4.6, §11.3.7.2.

`§N` is ISO 32000-2 and nothing else.

## 1. §11.3.6 — one `shall`, and it is executed

The row was `partial` on this sentence of its own note: "[f]or the twelve separable modes the
formula is tiny-skia's and Vello's rather than this tree's, and what is implemented here is the
choice of what they composite onto … `partial` for the first half."

That is a true sentence about *whose code* evaluates the formula and not about a requirement. The
clause states exactly one:

> The result colour shall be normalised by the result alpha, ensuring that when this colour and
> alpha are subsequently used together in another compositing operation, the colour's contribution
> is correctly represented.

Everything else in §11.3.6 is the formula, NOTE 1's unsimplified form, NOTE 2's undefined colour at
zero result alpha, and prose saying what the weights mean. The `shall` is executed by all three
evaluators a premultiplied raster gives this tree — both rasterisers, `render-cpu/src/blend.rs` for
§11.3.5.3's four modes, and `viewer_ui::software` for the chrome — and
`soft_mask.rs::the_weighted_average_is_exact_at_every_mask_value` sweeps all 256 mask values with no
slack at all, which is the closed form held rather than approached (ADR 0418 is why the
floating-point pipeline is asked for).

A requirement executed by a dependency is executed. The ledger's `implemented` is "every normative
requirement in the clause is executed", not "every line of arithmetic was written here" — and no
other row in this ledger is held to the second reading.

**§11.3.6 is `implemented`.**

## 2. §11.4.8 — no modal verb anywhere in the clause

The row was `partial` because "[t]he status follows §11.4.4's and §11.4.6's rather than adding
anything: this clause introduces no requirement they do not."

Counted rather than read for tone: §11.4.8 contains **no `shall` and no `should`**. It opens by
saying what it is — "[t]his subclause is a restatement of the group compositing formulas that also
takes isolated groups and knockout groups into account" — and then gives the initialisation, the
per-element recurrence, the result, and a NOTE saying the formulas "can be significantly simplified
when some sources of shape and opacity are not present".

So the clause asks a reader for nothing of its own. Every requirement its formulas restate belongs
to §11.4.4 or §11.4.6, both of which are `partial` and both of which name their residue: the
knockout element whose one alpha is the product of shape and opacity. Recording that debt a third
time under a clause that states no obligation is double counting, and it hides the rows that do.

This is exactly §11.3.8's shape one clause down. That row — "Summary of basic compositing
computations", a restatement of §11.3.3 to §11.3.7 — is `implemented` beside a `partial` §11.3.6 and
a `partial` §11.3.7, with the note "[n]othing new is required of a reader; the rows above are where
each of its lines is answered". §11.4.8 takes that status for that reason.

The one quantity a reader might expect this clause to add on its own is the shape recurrence,
`f_gi` as the union of the elements' shapes. That sentence is §11.3.7.2's — "[t]he shape of a group
object shall be the union" — and §11.3.7.2's row carries it, `partial`, priced at a second raster
per element (ADR 1022 section 5, ADR 1205).

**§11.4.8 is `implemented`.**

## 3. Consequences

- Two rows move; `doc/todo/65`'s third bucket loses §11.3.6 and its aggregate list loses §11.4.3,
  which states its own residue now rather than deferring to §11.4.4 (§4).
- No code changed for either. The instrument was `grep -c '\bshall\b'` over the clause's own span in
  `doc/md/`, which is one minute and is the reading ADR 1220 asks for.

## 4. §11.4.3, which was read the same way and did not move

§11.4.3 states seven `shall`s and its note deferred to "the groups §11.4.4 still reports". That
deferral had expired for the group this round was asked about — a non-isolated group whose `Do`
composites under a mode of its own is built (ADR 1107) and priced (ADR 1206) — but the clause has a
residue of its own, in its own sentence: "[t]hese objects shall be composited against a selected
initial backdrop and the resulting colour, shape, and opacity shall then be treated as if they
belonged to a single object." A raster of premultiplied samples carries the alpha and not the shape.
`Command::Group`'s `alpha_is_shape` states the groups where the two coincide, which §11.6.4.2 makes
the groups nothing inside which states a mask or a constant as opacity; everywhere else the single
object carries the product. The row stays `partial` with that sentence as its note, which is a
narrower and checkable claim than the one it replaced.
