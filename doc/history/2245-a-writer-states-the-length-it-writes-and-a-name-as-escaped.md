# Round 2245 — a writer states the `/Length` it writes, and a default appearance in §7.3's syntax

Fix round for audit 2243's fix groups 1 and 3 (B01, B05, B06), in the `audit` worktree. 2026-11-25.
ADRs 3681 to 3683. B02 to B04 wait for the census round's `pdf-syntax` pass.

## What changed

- **B01** (§7.3.8.2): `write::incremental_update_extending` writes every stream's `/Length` as the
  bytes it writes, after §7.6.2's encryption. The rewrite is not counted (ADR 3681).
- **The encrypted route, tested** (`pdf-transform/tests/update.rs`, six tests). A page carried out
  of an AES-256 source into a clear document was written with `/Length 64` over 39 bytes; the plant
  confirmed it. Into an encrypted destination it was already right. The same route had two more
  defects, now fixed (ADR 3682). A stream's own `/Crypt` stage, which names the source's `/CF`, was
  carried, so the page read back empty in a clear destination; the carry now drops a non-`Identity`
  stage. A stream whose decryption failed was carried empty; it is now refused by name.
- **B05** (§7.3.5): `scripted_appearance` keeps the restated font a `Name` and writes it through
  `Name::escaped`. **B06** (§7.3.3): the `/DA` replay and the restated `Tf` size are written through
  `pdf_syntax::write`'s formatter, from the `f64` the lexer read (`variable_text::real_operand`,
  `DefaultAppearance::size_as_stated`). The lexer holds a literal past `f64`'s range at `±f64::MAX`,
  so no infinity reaches the writer (ADR 3683). `format_number` is removed.

## Census of hand-formatted writers (`pdf-model`, `pdf-transform`; Sonnet)

No writer writes a name taken from a document unescaped: every raw `/`-and-bytes site writes a
name the program generated (`redact/painting.rs`, `redact/forms.rs`, `view/pages.rs`,
`archive/derived.rs`). About twenty sites write reals with `{}` or `{:.N}`. Neither prints an
exponent, but both print `inf` or `NaN` for a non-finite value. None is fixed here, and each is a
risk only for non-finite input: `appearance.rs`'s `colour_operator` (NaN passes `clamp`), `cm`,
`re`, `m`/`l`/`c` and `rotate`; `variable_text.rs`'s `re W n` and `Tm`; `rich_text/layout.rs`'s
`re`, `Tm`, `Tz`, `Tf` and `rg`; `view/markup.rs`'s `rg`; `redact/overlay.rs`'s `cm`;
`archive/derived.rs`'s matrix; and `archive/preserve.rs` and `archive/fonts/photographed.rs`'s
`number`.

## Left

`merge` and `split` carry `decryption_failed` streams and `/Crypt` stages through `serialize` as the
insert-pages carry did. The real writers above should share one finite-checked formatter.

**Gates.** fmt 0; clippy `-D warnings --all-targets` 0; nextest of pdf-syntax, pdf-model, pdf-transform,
pdf-archive and viewer-qt 100 with 3 failures, exactly the manifest's B02 to B04; rustdoc 0; conformance 0.
