# 1278 — A tagged page's silence is its own, and a `Form` is named by its title or its field

The HOST-UI round, batch forty-three. ADRs 1393, 1394. No ledger row moved: the contract named
none, and §14.7.2, §14.8.1, §14.8.4.7.2 and §14.9.3 were already `implemented`.

## Three silences (ADR 1393)

- The brief's §14.7.4 is Namespaces; a page's content is reached through §14.7.5.4's parent tree.
- `PageStructure::tagging` (`viewer_core::Tagging`): `Untagged`, `Unread`, `Unreached { marked }`,
  `Reached`. The page node says ADR 0214's sentence only for `Untagged`; one byte on the wire.
- Census: page answered as the wrong one held at zero; new floors 44 tracked unreached pages (3 in
  `/Marked true` documents), 63 and 15 whole population. No existing floor moved.

## A `Form` with no text (ADR 1394)

- Order: `/Alt`/`/E` (substitution), own text, element `/T`, field `/TU`, §12.7.4.2 name.
- Measured before and after on the corpus: 272 of 272 `Form` elements crossed unnamed, now 0;
  new floor 272, the unnamed class held at zero.

## Driven (`scratchpad/r1278/fx/`, Xvfb, gi `Atspi` walker)

- `tagged-two.pdf` in `quorra-gtk`, `quorra-qt`, `quorra`: page 1 `paragraph`, `button 'Place the
  order'` (`/TU`), `entry 'Your postcode'` (`/T`), `entry 'city'` (qualified); page 2 the tagged
  sentence. `unmarked-two.pdf` and `untagged-two.pdf` say theirs; `issue9972-1.pdf` names its fields.
- CJK tab: GTK draws `多边形批注.pdf`; Qt draws tofu. Eleven installed faces share the family
  `Droid Sans` and Qt's fallback is by family; with the non-CJK ones rejected by a `FONTCONFIG_FILE`
  Qt draws it. The toolkit's own fontconfig is the route; this machine's packaging defeats it.
- Tab + Enter and Tab + Space on a push button in `quorra`: `/A` `LastPage` fired (page 3 of 3).
  `quorra-confined` has no form controls by ADR 0713's scope: Enter does nothing, Space turns a page.

## Left

- Qt's tab strip on a machine whose CJK face shares a family with Latin faces.
- `quorra-confined` presses no push button: its scope (ADR 0713), not a missed wire.
- Table 355's `/T` names only a `Form`; whether a titled `Sect` is named by it is not decided.
