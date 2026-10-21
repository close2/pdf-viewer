# 2199 — Pattern tint still loses its conversion inputs

Status: accepted finding, implementation owed. Session 1682.
Context: ADRs 2187, 2198; ISO 32000-2 §11.7.5.3 and §8.7.3.3.

The solid-source repair exposed another conversion boundary. An uncoloured tiling
pattern supplies a tint to its cell, but `content/pattern/tiling.rs` converts the
components with `Rendering::default()` and no black-generation pair, then stores
only `Color` in `Tiling::tint`. Its comment places the parameters at painting,
but neither the source nor those parameters reaches that later operation.

`cargo run -p pdf-model --example black_generation_order` preserves the probe.
A DeviceCMYK page sets BG(k)=1−k and UCR(k)=k/2, invokes a Form inheriting the
pair, selects an uncoloured pattern with RGB tint (0.2, 0.4, 0.6), then paints.
The pattern gives [51, 102, 154]; the same clause arithmetic used for the solid
and image witnesses owes [52, 62, 79]. The instrument prints both, never asserts
the wrong result as desired. The integration control instead proves that changing
the Form's inherited RGB source cannot replace the already-converted pattern tint.

Keep §11.7.5.3 partial. This is owed conversion-state propagation, distinct from
ADR 1242's deliberate output-intent profile bypass. Do not expand the solid/stencil
repair into the pattern cache without tracing its dependencies. The next contract
is source space/components retained through tint selection, conversion at the
painting state, and cache identity including every conversion dependency. Test
pair-before/after selection, repeated uses under different pairs, q/Q, inherited
Forms, fill/stroke, stencil-through-pattern and current rendering intent/BPC.
Use the same arithmetic pixel and preserve the inherited-source isolation control.
