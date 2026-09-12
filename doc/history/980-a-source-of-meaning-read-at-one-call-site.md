# 980 — The output intent reached `k` and not `cs`, so one page drew one colour two ways

2026-09-11. ADR 1001. Colour: ISO 32000-2 §8.6, §10.3 and §10.4, the routes a colour takes to a
pixel.

Files: `crates/pdf-model/src/colour.rs`, `crates/pdf-model/src/content/colour.rs`,
`crates/pdf-model/src/icc.rs`, `crates/pdf-model/tests/colour_paths.rs`,
`doc/conformance/ledger.toml` (§8.6.4.4, §8.6.5.5, §8.6.5.6, §8.6.5.7, §8.6.6.5, §10.4.2.5,
§14.11.5), `doc/adr/1001-a-source-of-meaning-read-at-one-call-site.md`, this file.

The round asked, of every colour that reaches a pixel, which of §10.4.2.1's two routes it takes
and whether the code says why. Reading the route rather than the table found that §14.11.5's
output intent — the second of the three sources this tree ranks above `CMYK_CORNERS` — was
consulted by `Interpreter::device_space`, which `g`, `rg` and `k` call, and by nothing else that
selects a device colour space. `cs /DeviceCMYK … scn` reached `ColourSpace::parse`, which asks
§8.6.5.6's default and stops. On a fixture with a readable `/DestOutputProfile`, `k` gave
(1, 255, 0) and `scn` (0, 173, 239). The ranking now lives in `ColourSpace::device_family`, reached
by name, by array form, and by every special space's reference to a family, and
`ColourSpace::parse_with_output_intent` carries the intent to it.

The corpus could not see it: `examples/raster_digest` over the 974 first pages is byte-identical
across the change. Both new tests fail on the previous code and pass on this one.

Measured on the way: `data/icc/sRGB2014.icc` through the profile route is the identity to 0.88
of 255 at worst, which prices §8.6.5.7's declined `should`.

Left for a round that owns the files: `image.rs`, `shading.rs`, `inline_image.rs` and
`soft_mask.rs` still parse a colour space with no intent, and §14.11.5's row is `partial` naming
them. A `'Lab '` data-space profile is refused by `Profile::parse` and §8.6.5.5's row is `partial`
for it. `ColourSpace::Icc`'s `Box<Profile>` is cloned per `cs` where the intent applies, as it
was per `k`; an `Arc` needs `image.rs:2115`.
