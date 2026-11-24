Status: complete
Given: 2026-11-24, in conversation — transcribed by the round
Owes: keep the 2020 edition as target (dated-revision agreements apply once published and held) and keep erratum #77 applied while published; add a note to the validator's output whenever a document is accepted or rejected on a font where erratum #77's CIDFontSupplement/CMapSupplement direction decides or is exercised, stating that #77 is published as <= on the errata page but parked by the PDF/A TWG pending a white paper

> Q1043: agreed.  However I think that the validator should add this information in its output, when it rejects or accepts such documents

Reading:
Option 1, with an output note on erratum #77:
1. Keep ISO 19005-4:2020 as the target edition. The seven agreements into the dated revision (ISO/DIS 19005-4:202x: #192, #197, #232, #244, #626, #633, #842) stay `not applicable` until that dated revision is published and held.
2. Keep erratum #77 applied as published (`CIDFontSupplement <= CMapSupplement`), matching the errata page and veraPDF 1.30.3. However, whenever the validator accepts or rejects a document that exercises #77 (a non-Identity Type 0 font whose CIDFont and CMap supplements differ so the comparison direction matters), the validator's output must include this context — noting that the published errata page specifies `<=` while the PDF/A TWG later decided not to adopt that change in the dated revision and parked it pending a white paper.
