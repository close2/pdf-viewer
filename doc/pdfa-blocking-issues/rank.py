#!/usr/bin/env python3
"""The PDF/A blocking-issues document's instrument: every refusal, its documents, and the remedy it is given.

    python3 -I -B doc/pdfa-blocking-issues/rank.py <census.log> [--repository <dir>] [--render <quorra-transform>]

`<census.log>` is the `--nocapture` output of
`cargo test --profile gates -p pdf-transform --test archive_corpus
the_shipped_profiles_list_what_real_producers_files_refuse_on -- --ignored --nocapture --exact`,
which converts pdf.js's test documents and `doc/corpora` under the two shipped profiles at PDF/A-2b
and PDF/A-4 and prints, per census line, every refusing requirement with its documents, and every
document the verb declined as a source.

It prints four tables:

1. the census lines as the test printed them;
2. every reason (kind, requirement, first 110 characters of the sentence) with `documents/alone` per
   census line, *alone* being the documents that reason is the only refusal of;
3. every proposed remedy of `doc/pdfa-blocking-issues.md` (`REMEDY` below maps each reason to one)
   with the same two figures per profile and target, then the order in which building them frees
   the most documents (greedy, `D-` and `X` rows never chosen: a defect is not a remedy and an
   exception is not built), and what is left;
4. the documents behind each remedy.

`--render <quorra-transform>` runs the render verb on page 1 of each document the verb declined as
a source, and files one the reader draws no page of under `X2-no-page-to-see` instead of
`R01-recovered-structure`: the owner's test is "if we can see the document", and the reader's own
answer decides it, not a list here. Without it every declined document is `R01`.

A refusal can hide one behind it: a document is counted under every reason its conversion reported,
and a reason a remedy would only then reach is not in the log. So a remedy's "frees" is the
population it would free if nothing new appeared behind it, which the remedy's own round measures.
"""

import collections
import os
import pathlib
import re
import subprocess
import sys
import tempfile

HEAD = re.compile(r"^(pdf\.js|corpora) profile (\S+) (PDF/A-\S+): (\d+) failing documents converted, "
                  r"(\d+) conforming, (\d+) refused, (\d+) unreadable")
ROW = re.compile(r"^ {4} *(\d+) (\S+) +(\S+): (.*)$")
DECLINED = re.compile(r"^ {4}(\S.*\.pdf): (.*)$", re.IGNORECASE)


def remedy(kind, site, why):
    """The remedy `doc/pdfa-blocking-issues.md` proposes for one reason, by its section's key."""
    if kind == "source-refused":
        return "R06b-the-other-part" if "header's version" in why else "R01-recovered-structure"
    if kind in ("output-fails", "no-decision"):
        return "D-defect"
    if site == "graphics/image-interpolation-is-off":
        return "R02-image-smoothing"
    if site == "graphics/device-cmyk-needs-a-default-a-blending-space-or-a-cmyk-output-intent":
        return "R05-press"
    if site == "fonts/font-programs-embedded" and kind == "unauthorised":
        return "R09-as-shown-font-losses"
    if site in ("fonts/embedded-programs-define-every-glyph-shown", "fonts/no-notdef-glyph-shown"):
        return "R03a-notdef-named"
    if site == "fonts/font-programs-embedded" and kind == "the-fence" and "names no glyph for" in why:
        return "R08-unembedded-font-photograph"
    if kind == "the-fence" and site in ("fonts/non-symbolic-truetype-differences-are-listed-names",
                                        "fonts/non-symbolic-truetype-program-maps-every-code",
                                        "fonts/symbolic-truetype-program-has-a-usable-cmap"):
        return "R11-truetype-route"
    if site == "fonts/cid-system-info-agrees-with-the-cmap":
        return "R10-font-refiling"
    if kind == "the-fence" and site in ("fonts/font-programs-embedded", "fonts/widths-agree-with-the-program") \
            and "same glyph" in why:
        return "R10-font-refiling"
    if site in ("graphics/only-operators-the-base-standard-defines", "graphics/inline-image-interpolation-is-off",
                "file-structure/hexadecimal-string-holds-only-digits", "graphics/no-transfer-function-in-a-graphics-state",
                "fonts/type3-glyph-procedures-state-their-width", "fonts/actual-text-states-no-private-use",
                "file-structure/bound-names-are-valid-utf8"):
        return "R03-photograph"
    if site in ("graphics/named-resources-are-defined", "graphics/content-streams-have-an-explicit-resources-dictionary"):
        return "R04-absent-resource"
    if site == "file-structure/file-header" and kind == "not-built-yet":
        return "R06a-header-written"
    if site == "file-structure/file-header" or (site.startswith("implementation-limits/") and kind == "not-this-target"):
        return "R06b-the-other-part"
    if site.startswith("implementation-limits/"):
        return "R03-photograph"
    if site == "file-structure/stream-filters-are-standard":
        return "R07-undecodable-stream"
    if site == "fonts/font-programs-embedded" and kind == "not-built-yet":
        if "directly inside the font dictionary" in why:
            return "R10-font-refiling"
        if "descendant names one of the Adobe" in why:
            return "R08b-cjk-pack"
        return "R08-unembedded-font-photograph"
    if site in ("fonts/font-programs-filed-by-their-format", "fonts/cid-to-gid-map-present",
                "fonts/descendant-fonts-holds-one-cidfont", "fonts/widths-agree-with-the-program"):
        return "R10-font-refiling"
    if site in ("fonts/truetype-codes-reach-glyphs-by-the-standard-route", "fonts/symbolic-truetype-states-no-encoding",
                "fonts/non-symbolic-truetype-uses-a-standard-encoding"):
        return "R11-truetype-route"
    if site == "graphics/spot-colourants-appear-in-the-colorants-dictionary":
        return "R12-spot-colourant"
    if site == "graphics/jpeg2000-uses-the-baseline-feature-set":
        return "R13-jpx-transcode"
    if site.startswith("metadata/"):
        return "R14-appended-page-composition"
    if site.startswith("embedded-files/"):
        return "R15-embedded-files"
    if site.startswith(("annotations/", "signatures/")) or site == "forms/need-appearances-absent-or-false":
        return "R16-annotations-and-forms"
    if site == "forms/no-xfa-key":
        return "X1-dynamic-xfa"
    if site in ("graphics/no-halftone-name", "graphics/no-overprint-mode-one-under-icc-cmyk"):
        return "R17-press-statements"
    if site == "file-structure/document-information-dictionary-holds-only-a-modification-date":
        return "R18-piece-info"
    return "UNMAPPED"


def parse(path):
    """Census line -> {'counts', 'reasons': [(kind, site, why, docs)], 'declined': [(doc, said)]}."""
    out, current, pending = {}, None, None
    for line in pathlib.Path(path).read_text(encoding="utf-8", errors="replace").splitlines():
        head = HEAD.match(line)
        if head:
            current = {"counts": head.group(4, 5, 6, 7), "reasons": [], "declined": []}
            out[(head.group(1), head.group(2), head.group(3))] = current
            pending = None
            continue
        if current is None:
            continue
        if pending is not None:
            docs = [d for d in line.strip().split(", ") if d]
            if len(docs) != pending[0]:
                sys.exit(f"{path}: {len(docs)} documents under a reason that counts {pending[0]}: {line[:80]}")
            current["reasons"].append((*pending[1:], docs))
            pending = None
            continue
        row = ROW.match(line)
        if row and not line.startswith(" " * 9):
            pending = (int(row.group(1)), row.group(2), row.group(3), row.group(4))
            continue
        declined = DECLINED.match(line)
        if declined:
            current["declined"].append((declined.group(1), declined.group(2)))
        elif line.startswith("test "):
            current = None
    return out


def draws_no_page(binary, repository, name):
    """Whether the render verb answers that `name` (found under pdf.js or corpora) has no page."""
    # The corpora are symbolic links into the main checkout, which `Path.rglob` does not follow.
    found = [pathlib.Path(directory, name) for root in ("doc/pdf.js", "doc/corpora")
             for directory, _, files in os.walk(pathlib.Path(repository, root), followlinks=True) if name in files]
    if not found:
        sys.exit(f"{name} is in the log and under neither doc/pdf.js nor doc/corpora")
    with tempfile.TemporaryDirectory() as scratch:
        run = subprocess.run([binary, "render", str(found[0]), "--pages", "1", "--dpi", "10",
                              "-o", str(pathlib.Path(scratch, "page.png"))],
                             capture_output=True, text=True, timeout=60, check=False)
    return run.returncode != 0 and "has no pages" in run.stderr


def main():
    arguments = sys.argv[1:]
    if not arguments:
        sys.exit(__doc__)
    options = {}
    for flag in ("--repository", "--render"):
        if flag in arguments:
            at = arguments.index(flag)
            options[flag] = arguments[at + 1]
            del arguments[at:at + 2]
    census = parse(arguments[0])
    if not census:
        sys.exit(f"{arguments[0]}: no census line; is it the real producers' census?")
    no_pages = {}
    per = collections.defaultdict(lambda: collections.defaultdict(set))  # (profile, target) -> doc -> remedies
    reasons = collections.defaultdict(lambda: collections.defaultdict(set))  # reason -> line -> docs
    for (corpus, profile, target), line in census.items():
        for kind, site, why, docs in line["reasons"]:
            key = (kind, site, why[:110])
            for doc in docs:
                reasons[key][(corpus, profile, target)].add(f"{corpus}:{doc}")
                per[(profile, target)][f"{corpus}:{doc}"].add(remedy(kind, site, why))
        for doc, said in line["declined"]:
            key = ("source-refused", "-", said[:110])
            reasons[key][(corpus, profile, target)].add(f"{corpus}:{doc}")
            rid = remedy("source-refused", "-", said)
            if "--render" in options and rid == "R01-recovered-structure":
                if doc not in no_pages:
                    no_pages[doc] = draws_no_page(options["--render"], options.get("--repository", "."), doc)
                rid = "X2-no-page-to-see" if no_pages[doc] else rid
            per[(profile, target)][f"{corpus}:{doc}"].add(rid)
    lines = list(census)

    print("== 1. the census lines (converted, conforming, refused, unreadable; declined)")
    for key in lines:
        print(f"   {' '.join(key)}: {', '.join(census[key]['counts'])}; declined {len(census[key]['declined'])}")

    print("\n== 2. every reason, documents/alone per census line")
    print("   lines: " + " | ".join(" ".join(k) for k in lines))
    by_doc = collections.defaultdict(lambda: collections.defaultdict(set))
    for key, by in reasons.items():
        for line, docs in by.items():
            for doc in docs:
                by_doc[line][doc].add(key)
    ranked = sorted(reasons.items(), key=lambda item: (-len(set().union(*item[1].values())), item[0]))
    for key, by in ranked:
        cells = [f"{len(by[l])}/{sum(1 for d in by[l] if by_doc[l][d] == {key})}" if l in by else "-" for l in lines]
        print(f"   {len(set().union(*by.values())):>4} {' '.join(cells)}  {remedy(*key)}  {key[0]} {key[1]}: {key[2]}")
    unmapped = [key for key in reasons if remedy(*key) == "UNMAPPED"]
    if unmapped:
        print(f"   UNMAPPED: {len(unmapped)} reasons no section of the document answers: {unmapped}")

    print("\n== 3. every remedy, documents/alone per profile and target")
    targets = sorted(per)
    remedies = sorted({r for t in targets for rs in per[t].values() for r in rs})
    for rid in remedies:
        cells = [f"{sum(1 for rs in per[t].values() if rid in rs)}/{sum(1 for rs in per[t].values() if rs == {rid})}"
                 for t in targets]
        print(f"   {rid:<32} " + "  ".join(f"{' '.join(t)} {c}" for t, c in zip(targets, cells)))
    for profile in sorted({t[0] for t in targets}):
        sets = [set(per[t]) for t in targets if t[0] == profile]
        print(f"   {profile}: refused or declined at every target {len(set.intersection(*sets))}, "
              f"at one at least {len(set.union(*sets))}")
    for target in targets:
        left, chosen = dict(per[target]), []
        print(f"   built in this order under {' '.join(target)}, of {len(left)} refused or declined:")
        while True:
            best, freed = None, 0
            for rid in remedies:
                if rid in chosen or rid.startswith(("D-", "X")):
                    continue
                n = sum(1 for rs in left.values() if rs <= set(chosen) | {rid})
                if n > freed:
                    best, freed = rid, n
            if best is None:
                break
            chosen.append(best)
            for doc in [d for d, rs in left.items() if rs <= set(chosen)]:
                del left[doc]
            print(f"      +{best}: frees {freed}, {len(left)} left")
        rest = collections.Counter(" + ".join(sorted(rs)) for rs in left.values())
        print(f"      left: {dict(rest)}")

    print("\n== 4. the documents behind each remedy")
    for rid in remedies:
        docs = sorted({d for t in targets for d, rs in per[t].items() if rid in rs})
        print(f"   {rid} ({len(docs)}): {', '.join(docs)}")


if __name__ == "__main__":
    main()
