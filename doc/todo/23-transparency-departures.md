# Transparency departures

Status: each reported where it can change a pixel.
Priority: 23
Corpus: 19 documents
Clauses: §11.4, §11.5.3, §11.6.6
Code: `crates/pdf-model/src/content.rs`, `crates/pdf-model/src/soft_mask.rs`

Four populations, and the count beside each is what it costs on the corpus's first pages:

| | corpus | what it is |
|---|---|---|
| a knockout element whose shape is not its coverage | 5 | §11.4.6 composites an element with the group's *initial* backdrop; a shape that is not the element's coverage cannot be expressed as one alpha channel |
| a non-isolated group NOTE 5 cannot flatten | 6 | §11.4.4's NOTE 5 makes grouping equivalent to not grouping only where no element blends with the backdrop it excludes; these blend |
| a blending space that is not the device's three components | 4 | all `/DeviceCMYK` |
| a soft-mask group with such a space | 7 | §11.6.6, the same question inside a mask |

Each is refused *by name* rather than approximated, which is the rule that keeps the corpus count
honest. Taking any of them means compositing in a space the backends do not have, so the first
question is not the clause but the display list: what would a backend have to be handed.
