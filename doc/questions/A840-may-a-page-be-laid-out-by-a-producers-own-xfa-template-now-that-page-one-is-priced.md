Status: complete
Given: 2026-11-13, in conversation — transcribed by the round
Owes: (1) amend CLAUDE.md and doc/scope-ratifications.md to accept RFC 0011 §7's fifth near-side construction (a page laid out by a producer's own XFA template), lifting XFA's dynamic half from the exclusions and scheduling it in doc/todo/78 §4's order after the ledger rows reach their final states; (2) lay out page one from the template and data alone when the script level is off, with the window stating that the form's scripts did not run; (3) add XFA 3.3 to doc/specifications.zip with the first test that cites it, and evaluate public/internet documentation for the XFA barcode symbologies (PDF417 / ISO/IEC 15438, Code 39 / ISO/IEC 16388, Data Matrix / ISO/IEC 16022) — implementing them from public specifications where reasonable (accepting documented edge-case limits without the ISO PDF) or, if any cannot reasonably be built that way, recording an owner todo to obtain that specification rather than leaving barcodes unbuilt for lack of a corpus hit (XFA / ledger rounds)

> Q840: agree on 1 and 2.  Again: not having a use case is no reason to not follow the spec.  Please evaluate what information you can get from the internet without buying the spec.  If it seams reasonable to implement the features without the spec (accepting, that we might be in some edge cases incorrect) follow this path.  Otherwise mark ist as my todo to get the spec

Reading: adopts recommendations 1 and 2, and replaces recommendation 3's "refuse a barcode by name until a form in the corpora needs it" with a build-or-ask directive:
1. **The amendment:** accepts RFC 0011 section 7's fifth near-side construction (*a page laid out by a producer's own XFA template*), lifting XFA's dynamic half from `CLAUDE.md`'s exclusions and scheduling the build after the ISO 32000-2 ledger rows reach their final states, in `doc/todo/78` section 4's order.
2. **Scripts at `off`:** when the reader's script level is `off`, lay out page one from the template and data alone and state in the window that the form's scripts did not run.
3. **Barcodes and `doc/specifications.zip`:** add XFA 3.3 to `doc/specifications.zip` with the first test that cites it; for the barcode symbologies XFA names (such as PDF417, Code 39, and Data Matrix), do not skip them because no local corpus file hits them — evaluate what can be gathered from public documentation on the internet, implement them from those public sources if reasonable (accepting that edge cases not covered without the paid ISO text may differ, noted honestly under principle 5), and otherwise open a question / owner todo to purchase the specific ISO standard needed.

---

## Supplementary Research Brief on XFA Barcodes (for `Q840` item 3)

> **Note to the acting round:** The research summary and resource list below were gathered at the owner's request to assist with `Q840` item 3. **It is entirely up to the acting model/round to decide whether to rely on these pointers, conduct its own independent research, implement clean-room encoders in-tree, audit/adopt a Rust crate, or flag a specific standard as an owner purchase todo.**

### 1. Feasibility Verdict

**All three primary XFA barcode symbologies found in the corpus census (`pdf417`, `code3Of9`, `dataMatrix`), as well as the broader XFA `<barcode>` tier implemented by other open-source XFA engines, can be implemented cleanly and accurately without purchasing paid ISO PDFs**, because:
1. **XFA-layer framing, pre-processing, and geometry are already held in `doc/XFA-3_3.pdf` (`doc/md/XFA-3_3.md`):** Chapter 12 (*Using Barcodes*, pp. 418–444) and the `<barcode>` Template Reference normatively define XFA's character encoding (`UTF-8` without BOM by default, `ISO-8859-1`, `Shift-JIS`, etc.), `dataPrep="flateCompress"` (`0x81` prefix byte + deflate payload, p. 425), `moduleWidth` / `moduleHeight`, `wideNarrowRatio`, 1D checksum modes (`1mod10`, `2mod10`, `1mod10_1mod11`), human-readable `textLocation` (`none`, `above`, `below`, `aboveEmbedded`, `belowEmbedded`), and `type="rfid"` (non-visual).
2. **The underlying symbologies were placed into the public domain by their inventors (Symbol Technologies, International Data Matrix, Welch Allyn, UPS, US DoD, GS1, USPS)**, with complete mathematical tables and bit-level encoding rules published in expired US patents, US Federal Standards (`MIL-STD-1189B`, `USPS-B-3200`), free GS1 General Specifications, and permissively licensed (`MIT` / `BSD-3-Clause` / `Apache-2.0`) open-source implementations.
3. **Every generated symbol can be round-trip verified in tests** against an independent pure-Rust decoder (e.g., `rxing` or `datamatrix` as a `[dev-dependencies]` oracle): even if an encoder does not implement every optional compaction heuristic (e.g., choosing Base256/ASCII over optimal C40/X12 switching on an unusual string), any valid encodation produces a 100% standards-compliant, scanner-decodable symbol.

---

### 2. Primary Symbologies (`pdf417`, `code3Of9`, `dataMatrix`) — Public Specs & Exact Parameters

#### A. Code 39 (`code3Of9`, `code3Of9extended`, `logmars` — ISO/IEC 16388)
- **Public-domain specification:** US Department of Defense **`MIL-STD-1189B`** (*Standard Department of Defense Bar Code Symbology*, 1984 — US Government public-domain work, archived at EverySpec `http://everyspec.com/MIL-STD/MIL-STD-1100-1299/MIL-STD-1189B_14515/` and DLA ASSIST `https://quicksearch.dla.mil/`).
- **Complete algorithm (~100 LoC):**
  - Alphabet of 43 characters (`0–9`, `A–Z`, `-`, `.`, space, `$`, `/`, `+`, `%`, mapped to `0..42` for the optional modulo-43 check character) framed by `*` start/stop delimiters (`010010100`).
  - Each character has 9 alternating elements (5 bars, 4 spaces), with **3 wide (`1`) and 6 narrow (`0`)** elements (`wideNarrowRatio` in `[2.0, 3.0]`, XFA 3.3 p. 440), separated by an inter-character gap (1 narrow space).
  - `code3Of9extended` encodes all 128 ASCII characters (`0x00..0x7F`) via standard 2-character `$`, `%`, `/`, `+` shift pairs (also defined in `MIL-STD-1189B`).
- **Edge-case risk without ISO/IEC 16388 PDF:** Essentially zero; the 44 nine-element patterns and modulo-43 table are fixed and universally identical.

#### B. PDF417 (`pdf417` — ISO/IEC 15438; 24 hits in the 53-form XFA sample)
- **Public-domain specification:** **US Patent `5,243,655`** (*System for encoding and decoding data in machine readable graphic form*, Ynjiun P. Wang / Symbol Technologies, 1993 — `https://patents.google.com/patent/US5243655A/en`). Symbol Technologies dedicated PDF417 to the public domain.
- **Core mathematical specification:**
  - **Row geometry:** $R \in [3, 90]$ rows, $C \in [1, 30]$ data columns. Each row consists of: Start pattern (`81111113` = `0x1FEA8`, 17 modules), Left Row Indicator codeword, $C$ symbol codewords, Right Row Indicator codeword, and Stop pattern (`711311121` = `0x3FA29`, 18 modules).
  - **Base-929 codewords & clusters:** Each codeword (`0..928`) is 17 modules wide with 4 bars and 4 spaces. Rows cycle through **Cluster 0, Cluster 3, Cluster 6** (`cluster = (row % 3) * 3`), where a 4-bar/4-space pattern belongs to cluster $(b_1 - b_2 + b_3 - b_4 + 9) mod 9$.
  - **Symbol length descriptor:** `CW[0]` equals the total number of data codewords (including `CW[0]` itself and any pad `900` codewords, *excluding* Reed-Solomon ECC codewords).
  - **Compaction modes:**
    - *Text Compaction* (default initial mode, latch `900`): Alpha, Lower, Mixed, and Punctuation submodes packing 2 values per codeword ($30 	imes h + l$).
    - *Byte Compaction*: Latch **`924`** when byte count is an exact multiple of 6 (packing 6 bytes from base 256 into 5 base-900 codewords); latch **`901`** when byte count is *not* a multiple of 6 (6-byte groups into 5 codewords, trailing $1..5$ bytes emitted 1 byte per codeword).
    - *Numeric Compaction* (latch `902`): Groups of up to 44 digits prefixed with `'1'` converted from base 10 to base 900 (up to 15 codewords).
  - **Reed-Solomon ECC over prime field $\mathbb{F}_{929}$:** Primitive root $\alpha = 3$; for error correction level $s \in [0, 8]$, $k = 2^{s+1} \in \{2, 4, 8, 16, 32, 64, 128, 256, 512\}$ ECC codewords with generator polynomial $g(x) = \prod_{i=1}^{k} (x - 3^i) \pmod{929}$. Note also XFA 3.3 p. 442's explicit rule on `dataCount`, `moduleWidth`, and fitting within field width/height.
- **Edge-case risk without ISO/IEC 15438 PDF:** Low; ECI (`925`–`927`) and Macro PDF417 (`928`) are rarely used in XFA forms, and compaction mode transitions can be cross-checked against `rxing` / `pdfium` / `zint`.

#### C. Data Matrix (`dataMatrix` — ISO/IEC 16022; XFA 3.3 p. 442 defaults to ECC 200)
- **Public-domain / free official specifications:**
  - **GS1 General Specifications** (`https://www.gs1.org/genspecs`) and **GS1 DataMatrix Guideline** (`https://www.gs1.org/docs/barcodes/GS1_DataMatrix_Guideline.pdf`) — free official downloads detailing ECC 200 symbol sizes, data/ECC block interleaving, finder/clock/alignment patterns.
  - **US Patent `5,612,524`** (International Data Matrix, Inc., 1997 — `https://patents.google.com/patent/US5612524A/en`).
- **Core mathematical specification:**
  - **Symbol sizes:** 24 even square sizes ($10\times 10$ to $144\times 144$) and 6 rectangular sizes ($8\times 18$ to $16\times 48$); matrices larger than $24\times 24$ split into $2\times 2$, $4\times 4$, or $6\times 6$ data regions separated by solid/alternating alignment borders.
  - **Encodations:** ASCII (default: digit pairs `dd + 130`, ASCII `b + 1`), C40 (`230`), Base 256 (`231`, with $255$-state pseudo-randomizing function $R(p) = ((149 \times p) \bmod 255) + 1$ at 1-based codeword position $p$), X12 (`238`), Text (`239`), EDIFACT (`240`), Unlatch (`254`). Pad starts with `129`, followed by $253$-state randomized `129`s.
  - **Reed-Solomon ECC over $\mathrm{GF}(2^8)$:** Primitive polynomial $x^8 + x^5 + x^3 + x^2 + 1$ (`0x12D` = `301`, distinct from QR's `0x11D`), generator polynomial $g(x) = \prod_{i=1}^{k} (x - 2^i)$ (first root $2^1$). Large symbols (`144x144`) interleave 8 or 10 RS blocks.
  - **Module placement:** Standard Annex F "Utah" 8-module L-shaped diagonal placement routine with the 4 standard corner cases.

---

### 3. Other XFA 3.3 Barcode Types — Public Sources & Industry Practice

- **What other XFA engines implement:** Chromium **PDFium's `fxbarcode`** (`BSD-3-Clause`, `https://pdfium.googlesource.com/pdfium/+/refs/heads/main/fxbarcode/` and `xfa/fxfa/cxfa_ffbarcode.cpp`, originally Foxit's XFA barcode module) implements **10 symbologies**: `pdf417`, `code3Of9`, `dataMatrix`, `QRCode`, `code128`, `codabar`, `ean8`, `ean13`, `upcA`, and `postUSImb` (`OneCode`), treating the rarer legacy hardware/postal identifiers per XFA 3.3 p. 419.
- **Free normative sources for the remaining XFA identifiers:**
  - **GS1 General Specifications (`https://www.gs1.org/genspecs`, free PDF):** Normatively defines `ean8`, `ean13`, `upcA`, `upcE`, `upcean2`, `upcean5`, `code128` / `ucc128`, `code2of5interleaved`, `rss14*` (GS1 DataBar), and `QRCode` (Model 2).
  - **US Postal Service (`https://postalpro.usps.gov/`, US Government public domain):** `USPS-B-3200` defines `postUSImb` (Intelligent Mail 4-State Barcode); Domestic Mail Manual (DMM 708) defines `postUSStandard` / `postUS5Zip` / `postUSDPBC` (POSTNET) and `fim` (Facing Identification Mark).
  - **UK & Australia Postal (free carrier specs):** Royal Mail Mailsort / CBC guide defines `postUKRM4SCC`; Australia Post *Customer Barcoding Technical Specifications* defines `postAUS*`.
  - **Aztec (`aztec`) & MaxiCode (`upsMaxicode`):** Public-domain US Patents `US5591956` (Aztec, Welch Allyn) and `US4874936` / `US5053609` (MaxiCode, UPS).

---

### 4. Permissive Open-Source Crates & Reference Implementations

1. **Pure-Rust crates (`crates.io`):**
   - **`datamatrix`** (`https://github.com/jannschu/datamatrix-rs`, `Apache-2.0 OR MIT`, `#![no_std]`, zero `unsafe`): Complete ECC 200 optimal encoder + decoder (including C40/Text/X12/EDIFACT/Base256 lookahead, `0x12D` Reed-Solomon, and Utah placement).
   - **`rxing`** (`https://github.com/rxing-core/rxing`, `Apache-2.0`, pure-Rust port of ZXing): Encodes and decodes `PDF417`, `DataMatrix`, `Code39`, `Code93`, `Code128`, `Codabar`, `ITF`, `EAN-8/13`, `UPC-A/E`, `QRCode`, `Aztec`, and `DataBar` (`rss14`). Particularly useful either audited/trimmed or in `[dev-dependencies]` as an automated round-trip scanner oracle for clean-room encoders.
   - **`qrcode`** (`https://github.com/kennytm/qrcode-rust`, `MIT OR Apache-2.0`) and **`barcoders`** (`MIT`, lightweight 1D barcode encoders).
2. **Permissive non-Rust reference implementations (for algorithm/table verification):**
   - **PDFium `fxbarcode`** (`BSD-3-Clause`, `https://pdfium.googlesource.com/pdfium/+/refs/heads/main/fxbarcode/`): Shows exact XFA `<barcode>` attribute-to-renderer rules.
   - **BWIPP — Barcode Writer in Pure PostScript** (`MIT`, `https://github.com/bwipp/postscriptbarcode`): Self-contained, readable single-file encoders covering *every* 1D, 2D, stacked, and postal barcode named in XFA 3.3 Chapter 12 (including `code49`, `telepen`, `plessey`, `msi`, `maxicode`, `rm4scc`, `auspost`, `onecode`).
   - **Zint (`libzint` backend)** (`BSD-3-Clause` for `backend/`, `https://github.com/zint/zint`, technical manual at `https://zint.org.uk/manual/`): Exhaustive reference tables and test vectors across 50+ symbologies.
