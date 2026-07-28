The **SafeDocs Stressful PDF Corpus is not available as a Git repository**, nor can it be natively hosted on GitHub or GitLab.

The primary limitation is sheer file volume and size. The SafeDocs Issue Tracker corpus alone is over **31 GB** (packaged into six compressed `.tgz` archives), while the untruncated Common Crawl corpus spans **24 TB** (nearly 8 million files). Git's architecture degrades and crashes when tracking thousands of heavy binary files or multi-gigabyte payloads. Instead, DARPA and the PDF Association distribute these corpora via **AWS Open Data** and the **Digital Corpora project** as standard downloadable archives.

However, several other official, highly regarded PDF test suites **are** available as standard Git repositories and are ideal for including as submodules in your unit testing pipeline.

## Git-Ready PDF Test Corpora for Submodules

If you want to use `git submodule add` to pull in robust test cases without committing binaries to your own repository, these open-source repositories are the best targets:

### 1. Mozilla `pdf.js` (`mozilla/pdf.js`)

Mozilla's repository contains one of the most comprehensive, battle-tested regression corpora in the open-source ecosystem.

* **Target Directory:** `/test/pdfs/`
* **What it tests:** Malformed TrueType/CFF fonts, broken XREFs, complex ICC color profiles, soft-mask gradients, and AcroForm interactivity. Because these files were added to test specific browser rendering bugs over a decade of development, they serve as an excellent edge-case suite.

### 2. The PDF Association GitHub Organization (`pdf-association`)

The official standards body for PDF maintains several lightweight repositories specifically designed for developers and parser testing:

* **`pdf20examples`:** A collection of clean, compliant, human-readable PDF 2.0 files that illustrate proper ISO 32000-2 feature implementation.
* **`pdf-differences`:** Targeted test files that highlight specific non-conforming functionality and ambiguous edge cases that cause rendering divergences across different PDF readers.
* **`arlington-pdf-model`:** While not a raw PDF corpus, this repo provides a complete, machine-readable definition of the PDF DOM, which can be ingested by unit tests to programmatically validate parser syntax trees against the ISO standard.

### 3. OpenPreserve Foundation (`openpreserve/format-corpus`)

A digital preservation-focused corpus that is openly licensed and covers a wide range of formats, creation tools, and structural variations found in legacy documents over several decades.

* **Target Directory:** `/pdf/`
* **What it tests:** Long-term archival conformance (PDF/A), metadata parsing, and legacy encoding anomalies.

### 4. Apache PDFBox (`apache/pdfbox`)

While the repository itself is large due to its Java source code, pulling it in gives you access to their extensive input test directory.

* **Target Directory:** `/pdfbox/src/test/resources/input/`
* **What it tests:** Real-world parsing failures submitted by users to the Apache issue tracker over the years, focusing heavily on text extraction and font-mapping bugs.

---

## Best Practices for PDF Submodules

When submoduling repositories that contain binary files (like PDFs), default Git behavior will download the entire commit history of every binary blob, which can cause your CI/CD clone times to skyrocket.

To prevent bloat, always add and clone test submodules using a **shallow clone** (`--depth 1`):

```bash
# Add the submodule fetching ONLY the latest commit
git submodule add --depth 1 https://github.com/mozilla/pdf.js.git test/vendor/pdfjs

# When cloning a repo that already has submodules, initialize them shallowly:
git submodule update --init --depth 1

```

## How to Handle the Massive SafeDocs Corpus

Because you cannot submodule SafeDocs directly, standard industry practice for CI/CD pipelines is to use a **lazy-loading fetch script**.

Instead of tracking the files in Git, write a simple script (e.g., a `Makefile` target or a `pretest` npm/cargo script) that:

1. Checks if a local, git-ignored `/test/fixtures/safedocs/` directory exists.
2. If missing, uses `curl` or `wget` to pull down a specific, targeted `.zip` cluster from the Digital Corpora HTTP endpoint or AWS S3 bucket.
3. Extracts the PDFs and caches them locally (or in your CI's build cache) so the download only happens once per environment.
