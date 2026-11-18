# 2084 — An image's and a form's own measure reaches the measurement query

Session 2084, an audit-fix round in `pdf-viewer-lane3`. What the audit rounds left of review 2038: B73, a failing
`pdf-script::hook` test, `page_label::letters`'s comment, two `pdf-script` rustdoc links, three partial plant checks.

- **B73, ADR 3030.** Table 87 gives an image its own `/Measure` and Table 95 gives a form one; Table 269's `/LPTS`
  maps the unit square to the object's "rectangular bounds". The interpreter now reports each image or form drawn
  with one, with where its bounds landed (`Interpretation::measured_xobjects`). `Viewports::on_page` makes each a
  `Viewport` with a `Placement`, which handles rotation and shear, and searches them before the page's `/VP`.
  That order is a stated choice, because ISO 32000-2 orders only `/VP`. `/PCSM`'s "XObject position coordinates"
  are the object's own space. The audit's `image_xobject_measurement_spec_audit` passes unchanged. A turned-form test
  and a unit test were added.
- **`hook` test.** It fails on main since 65fe934f (round 2076's B78, ADR 2986). That commit moved `/PI` out of a page's
  close. §12.6.3 ("[t]he PV and PI entries allow a distinction between pages that are open and pages that are
  visible") makes the code right, so the test now expects two on the close and raises `/PI` as an annotation trigger.
  Its name is kept because review 1983's features.json lists it.
- **`letters`** now says Issue #593's repeat is the amended reading and #432 was closed as its duplicate (ADR 3018,
  round 2082's).
- **Rustdoc**: `[w]` and `[m]` in two verbatim quotes are escaped.
- **Plants**: T66's partial-tie half, T44's Normal and Multiply, and T82's BE half each fail on a plant and pass
  restored. No new test was needed. A first T66 plant ran beside the T44 Normal plant in `render-cpu`, which
  draws nothing over a clear backdrop. That failed T66 at its exact half, so it is not counted; the counted
  plant ran beside the Multiply plant, which a Normal fill does not reach.
- Ledger: 8.9.5.1 (`implemented`), 12.10.2 (`departed`) notes and tests updated; no status changed.

**Gates.** fmt exit 0; clippy `-D warnings --all-targets` (pdf-model, viewer-core, pdf-script with `engine`) exit 0;
rustdoc `-D warnings --document-private-items` of the three exit 0; nextest `--lib --bins --tests` of the three exit 0,
3711 passed, the manifest's one entry for them (B73) now passing; `fuzz` `cargo check --bins` exit 0;
`cargo test -p conformance` exit 0.

Trap: plants in one batch are disjoint only if no planted function is on another test's route. A blend-mode plant
in `render-cpu` reaches every rasterising test, the optional-content ones included. Also, `cargo test` without
`--no-fail-fast` stops at the first failing binary.
