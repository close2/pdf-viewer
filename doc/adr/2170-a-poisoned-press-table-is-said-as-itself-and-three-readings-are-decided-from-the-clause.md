# 2170 — A poisoned press table is said as itself, and three readings are decided from the clause

Session 1667, the clause slot of batch one hundred and three, under the project owner's switch to code
improvements and bug fixes of 2026-10-14. Status: **accepted**. Clauses: ISO 32000-2 §11.7.2,
§11.7.5.3, §8.7.4.5.7, §8.7.4.5.8, Table 52, §8.4.1, §10.7.2, §12.5.2, §9.7.4.2, §9.9.1. Context:
ADRs 0417, 1217, 1242, 1425, 1622, 2154; trap 11; principle 1.

## 1. A poisoned lock is not a spent budget

`pdf_colour::colour::Presses::press_for_profile` and the process-wide `press_for_profile` answered
`Option`, and every `Mutex::lock().ok()?` among them turned a lock a panicking thread poisoned into
`None` — which `pdf_model`'s `press_or_beyond` reported as "the ninth distinct press this page names",
§11.7.2's budget and a fact about the file. Both now answer `Result<Arc<Press>, NoPress>`:
`Spent` (the budget), `Unsampled` (not four components a grid can be sampled from) and `Poisoned`,
reached through one `guard` that maps the poison and nothing else. `press_or_beyond`,
`process_planes` (§10.8.3's simulated press) and the soft-mask luminosity each say the poison in its
own sentence — "a fault of this program, not of the file" — through `BeyondPress::fault`, a second
constructor beside `stated`, whose doc keeps "every other reason is the document's".
`a_poisoned_table_of_presses_is_said_as_poisoned_and_not_as_a_spent_budget` poisons the table from a
thread and fails with `guard` mapping to `Spent`. **Left with the same shape**: `Presses::rgb_route`
(a cache whose poison reads as "no route") and the two counters `named` and `presses_cached`, which
read a poisoned table as empty; none of the three can reach §11.7.2's sentence.

## 2. §11.7.5.3's report fired where the clause is carried out

The functions "shall be applied only during conversion from DeviceRGB to DeviceCMYK colour spaces".
`note_black_generation_departure` fired on any stated pair over a press that converts in by its
profile — including the test's own fixture, a page group stating `/CS [/ICCBased …]`, which is not
`DeviceCMYK` and whose `B2A` route therefore *is* the clause. Table 145 remaps a device `/CS` through
`/DefaultCMYK`, so a remapped one is the default's `ICCBased` space as well. Both are what
`Interpreter::cmyk_group` holds, and the report is now silent there; it fires on a `DeviceCMYK` group
that §14.11.5's output intent characterises (`two_way_intent_fixture`), and the test holds both. Still
wider than the clause, and named in the row: the report does not ask whether a `DeviceRGB` colour was
painted under the pair.

## 3. §8.7.4.5.7's departure has a `shall` under it

ADR 1622 §1 keeps `departed` for a `shall` this program does not carry out, and the note named none.
It is Table 52's flatness entry — "[t]he precision with which curves shall be rendered on the output
device", "measured in output device pixels", initial value 1.0 — which §8.4.1's initialisation brings
to every page, and which §10.7.2's permission to ignore "any flatness tolerance specified within a PDF
file" does not reach. A patch the backend cannot hold keeps `mesh::PATCH_STEPS`' fixed silhouette, a
curve not rendered to that precision once a magnification makes the step exceed a device pixel. Both
rows stay `departed`; the notes name the sentence.

## 4. §12.5.2's third option is declined

For a face whose `fsType` forbids embedding, ADR 1425 had two answers — embed it (the vendor's
condition broken) or write no `/AP` (`Written::unappeared`, this row's departure). The third, an
appearance naming the face unembedded, satisfies Table 166's column and nothing else: §9.7.4.2 makes
such a font's CIDs take no part in glyph selection and admits only predefined CMaps, which name the CJK
collections and `Identity` and carry no script the compiled-in faces lack, so the shaped presentation
forms ADR 1414 writes cannot be stated through it; on a reader without the face, §9.9.1's NOTE 5 says
the result depends on its environment. That is the wrong-words cost the departure exists to avoid.
