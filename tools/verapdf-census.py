#!/usr/bin/env python3
"""Where veraPDF fails an archive this tree wrote as conforming, by rule and by document.

Five of the last six gaps in this tree's PDF/A checker were found by putting one output it had
written through veraPDF (ADRs 2557, 2582, 2608, 2619, 2632). This makes that an instrument: the
census of real producers' files keeps every output it writes when `QUORRA_CENSUS_OUTPUTS` names an
empty directory (`crates/pdf-transform/tests/archive_corpus.rs`), and this script puts every one of
them through veraPDF at the flavour it was written for and prints each rule veraPDF fails, the
clause of ISO 19005 the rule cites, and the outputs it fails.

**Every output kept is one this tree passes.** A conversion writes its file only when the output,
held to the target again, conforms (`stands_as_departed` in `crates/pdf-transform/src/archive/
mod.rs`, ADR 0947; neither shipped profile names a departure), or when the source already conformed
and is copied. So a rule veraPDF fails on a kept output is a disagreement between the two
validators, and which of them the standard supports is a question for `doc/pdfa/`, never settled
by either program's verdict (`CLAUDE.md` principle 5): veraPDF is evidence about this tree's
reading, as this tree is about veraPDF's.

**A rule on a clause this tree checks only in part is counted apart.** Beside each output the
census writes `<output>.unchecked.tsv`, the requirements this tree's validator names as not checked,
one `requirement<TAB>citation` line each. A rule whose clause is one of those citations, or under
one, may be a requirement this tree never held the file to — a gap in what was checked rather than
a contrary reading — but a clause holds several requirements and the citation names only the
clause: `metadata/identification-amendment-and-corrigendum` is unchecked at ISO 19005-2 6.6.4 while
the identification's conformance value at the same clause is checked. So the split is where to read
first, never a classification.

The layout read is the census's own, `<outputs>/<profile>/<flavour>/<corpus>/<path in the corpus>`,
the flavour being veraPDF's `--flavour` word (`2b`, `4`). Every veraPDF run is bounded: at most
`--chunk` files a process, `--jobs` processes at once, each under `--heap`, and a chunk that does
not end inside its time or does not answer for every file it was given is re-run one file at a
time, where a file that still does not answer is printed as one veraPDF did not finish — never as
compliant and never as a disagreement.

    python3 -I -B tools/verapdf-census.py <outputs> [--results <file.tsv>] [--jobs 3]

The lines a reader takes are `veraPDF census:` and `disagreements:`; `tools/state.sh archive`
prints those two (ADR 2652). veraPDF's own text is not printed: a rule is named by its
specification, clause and test number, which is how ADR 2652 cites it.
"""

from __future__ import annotations

import argparse
import concurrent.futures
import json
import os
import re
import shutil
import subprocess
import sys
from collections import defaultdict
from dataclasses import dataclass, field
from pathlib import Path

CITATION = re.compile(r"ISO 19005-(\d+) section ([0-9]+(?:\.[0-9]+)*)")
SPECIFICATION = re.compile(r"ISO 19005-(\d+)")


@dataclass(frozen=True)
class Output:
    """One kept output, named by where the census put it."""

    path: Path
    profile: str
    flavour: str
    corpus: str
    document: str

    def unchecked(self) -> set[tuple[str, str]]:
        """The (part, clause) pairs this tree's validator did not check for this output."""
        sidecar = Path(str(self.path) + ".unchecked.tsv")
        pairs = set()
        for line in sidecar.read_text(encoding="utf-8").splitlines():
            for part, clause in CITATION.findall(line):
                pairs.add((part, clause))
        return pairs


@dataclass(frozen=True)
class Failed:
    """One rule veraPDF failed on one output."""

    rule: str
    part: str
    clause: str
    checks: int
    object: str


@dataclass
class Verdict:
    """What veraPDF said of one output: compliant, the rules it failed, or why it did not finish."""

    compliant: bool | None = None
    failed: list[Failed] = field(default_factory=list)
    unfinished: str | None = None


def outputs_under(root: Path) -> list[Output]:
    """Every kept output under `root`, in a stable order."""
    found = []
    for path in sorted(root.rglob("*")):
        if not path.is_file() or path.suffix.lower() != ".pdf":
            continue
        parts = path.relative_to(root).parts
        if len(parts) < 4:
            raise SystemExit(f"{path}: not under <profile>/<flavour>/<corpus>/ as the census writes")
        found.append(Output(path, parts[0], parts[1], parts[2], "/".join(parts[3:])))
    return found


def rule_of(summary: dict) -> Failed:
    """One veraPDF rule summary as a rule identifier and the clause it cites."""
    specification = str(summary.get("specification", "?"))
    part = SPECIFICATION.search(specification)
    clause = str(summary.get("clause", "?"))
    test = str(summary.get("testNumber", "?"))
    return Failed(
        rule=f"{specification.split(':')[0]} {clause}-{test}",
        part=part.group(1) if part else "?",
        clause=clause,
        checks=int(summary.get("failedChecks", 0) or 0),
        object=str(summary.get("object", "?")),
    )


def verdict_of(job: dict) -> Verdict:
    """One veraPDF job as a verdict; a job with no normal validation result did not finish."""
    results = job.get("validationResult") or []
    if not results:
        said = job.get("taskException") or job.get("errorMessage") or "no validation result"
        if isinstance(said, dict):
            said = f"{said.get('type', '?')}: {said.get('exception', '?')}"
        return Verdict(unfinished=str(said)[:200])
    verdict = Verdict(compliant=True)
    for result in results:
        if result.get("jobEndStatus") != "normal":
            return Verdict(unfinished=f"job ended {result.get('jobEndStatus')}")
        if not result.get("compliant", False):
            verdict.compliant = False
        for summary in result.get("details", {}).get("ruleSummaries", []):
            if summary.get("status") == "failed" or summary.get("ruleStatus") == "FAILED":
                verdict.failed.append(rule_of(summary))
    if verdict.compliant is False and not verdict.failed:
        return Verdict(unfinished="non-compliant with no failed rule named")
    return verdict


def run_verapdf(
    verapdf: str, flavour: str, files: list[Output], heap: str, timeout: float
) -> dict[Path, Verdict] | str:
    """One veraPDF process over `files`; a verdict per file, or why the run did not answer."""
    environment = dict(os.environ)
    environment["JAVA_OPTS"] = f"-Xmx{heap}"
    command = [verapdf, "--flavour", flavour, "--format", "json", "--maxfailuresdisplayed", "1"]
    command += [str(output.path.resolve()) for output in files]
    try:
        done = subprocess.run(
            command,
            capture_output=True,
            timeout=timeout,
            env=environment,
            check=False,
        )
    except subprocess.TimeoutExpired:
        return f"no answer within {timeout:.0f} s"
    try:
        report = json.loads(done.stdout)
    except json.JSONDecodeError:
        tail = done.stderr.decode("utf-8", "replace").strip().splitlines()[-1:] or ["no output"]
        return f"exit {done.returncode}, no report: {tail[0][:160]}"
    by_name = {output.path.resolve(): output.path for output in files}
    answered: dict[Path, Verdict] = {}
    for job in report.get("report", {}).get("jobs", []):
        name = Path(job.get("itemDetails", {}).get("name", ""))
        if name in by_name:
            answered[by_name[name]] = verdict_of(job)
    if len(answered) != len(files):
        return f"answered {len(answered)} of {len(files)} files"
    return answered


def validate(
    verapdf: str, flavour: str, files: list[Output], heap: str, timeout: float
) -> dict[Path, Verdict]:
    """A chunk, re-run one file at a time where the chunk as a whole did not answer."""
    answered = run_verapdf(verapdf, flavour, files, heap, timeout * len(files))
    if isinstance(answered, dict):
        return answered
    if len(files) == 1:
        return {files[0].path: Verdict(unfinished=answered)}
    verdicts: dict[Path, Verdict] = {}
    for output in files:
        verdicts.update(validate(verapdf, flavour, [output], heap, timeout))
    return verdicts


def covered(failed: Failed, unchecked: set[tuple[str, str]]) -> bool:
    """Whether a rule's clause is one at which this tree named a requirement as not checked, or
    under one — a clause this tree checks only in part, which the rule may or may not be."""
    return any(
        part == failed.part and (failed.clause == clause or failed.clause.startswith(clause + "."))
        for part, clause in unchecked
    )


def main() -> int:
    """Validates every kept output and prints the disagreements by rule and by document."""
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("outputs", type=Path, help="the directory QUORRA_CENSUS_OUTPUTS named")
    parser.add_argument("--verapdf", default=shutil.which("verapdf") or "verapdf")
    parser.add_argument("--jobs", type=int, default=3, help="veraPDF processes at once")
    parser.add_argument("--chunk", type=int, default=25, help="files per veraPDF process")
    parser.add_argument("--heap", default="1500m", help="each veraPDF process's -Xmx")
    parser.add_argument("--timeout", type=float, default=120.0, help="seconds per file")
    parser.add_argument("--results", type=Path, help="write every failed rule, one per line")
    arguments = parser.parse_args()
    if shutil.which(arguments.verapdf) is None:
        print(f"veraPDF census: veraPDF is not here ({arguments.verapdf}); nothing validated")
        return 1
    if not 1 <= arguments.jobs <= 16 or not 1 <= arguments.chunk <= 200:
        raise SystemExit("--jobs is 1 to 16 and --chunk 1 to 200, so that the run stays bounded")
    outputs = outputs_under(arguments.outputs)
    if not outputs:
        print(f"veraPDF census: no output under {arguments.outputs}; nothing validated")
        return 1
    version = subprocess.run(
        [arguments.verapdf, "--version"], capture_output=True, text=True, check=False
    ).stdout.splitlines()[:1] or ["version unknown"]

    chunks = []
    by_flavour: dict[str, list[Output]] = defaultdict(list)
    for output in outputs:
        by_flavour[output.flavour].append(output)
    for flavour, files in sorted(by_flavour.items()):
        for start in range(0, len(files), arguments.chunk):
            chunks.append((flavour, files[start : start + arguments.chunk]))
    verdicts: dict[Path, Verdict] = {}
    with concurrent.futures.ThreadPoolExecutor(max_workers=arguments.jobs) as pool:
        running = [
            pool.submit(
                validate, arguments.verapdf, flavour, files, arguments.heap, arguments.timeout
            )
            for flavour, files in chunks
        ]
        for future in concurrent.futures.as_completed(running):
            verdicts.update(future.result())

    by_rule: dict[str, list[tuple[Output, Failed, bool]]] = defaultdict(list)
    by_document: dict[str, set[str]] = defaultdict(set)
    unfinished = []
    compliant = 0
    lines = []
    for output in outputs:
        verdict = verdicts[output.path]
        if verdict.unfinished is not None:
            unfinished.append((output, verdict.unfinished))
            continue
        if verdict.compliant:
            compliant += 1
            continue
        unchecked = output.unchecked()
        for failed in verdict.failed:
            here = covered(failed, unchecked)
            by_rule[failed.rule].append((output, failed, here))
            by_document[f"{output.corpus}/{output.document}"].add(failed.rule)
            lines.append(
                "\t".join(
                    [
                        output.profile,
                        output.flavour,
                        output.corpus,
                        output.document,
                        failed.rule,
                        failed.object,
                        str(failed.checks),
                        "checked-in-part" if here else "checked-whole",
                    ]
                )
            )
    if arguments.results:
        arguments.results.write_text("".join(line + "\n" for line in lines), encoding="utf-8")

    failing = len(outputs) - compliant - len(unfinished)
    print(
        f"veraPDF census: {len(outputs)} outputs this tree wrote as conforming, {compliant} "
        f"compliant to veraPDF, {failing} not, {len(unfinished)} veraPDF did not finish "
        f"({version[0]})"
    )
    contrary = sorted(rule for rule, rows in by_rule.items() if not all(row[2] for row in rows))
    print(
        f"disagreements: {len(by_rule)} rules over {failing} outputs of {len(by_document)} "
        f"documents; {len(contrary)} rules on a clause this tree checks whole, "
        f"{len(by_rule) - len(contrary)} only on a clause it checks in part"
    )
    print("\nby rule (outputs, documents, outputs on a clause this tree checks in part):")
    ranked = sorted(by_rule.items(), key=lambda item: (-len(item[1]), item[0]))
    for rule, rows in ranked:
        documents = sorted({f"{row[0].corpus}/{row[0].document}" for row in rows})
        here = sum(1 for row in rows if row[2])
        objects = ", ".join(sorted({row[1].object for row in rows}))
        print(f"  {len(rows):>5} {len(documents):>5} {here:>5}  {rule}  [{objects}]")
        by_target: dict[str, list[str]] = defaultdict(list)
        for output, _, _ in rows:
            by_target[f"{output.profile} {output.flavour}"].append(
                f"{output.corpus}/{output.document}"
            )
        for target, names in sorted(by_target.items()):
            print(f"        {target}: {', '.join(sorted(names))}")
    print("\nby document (the rules veraPDF fails on any of its outputs):")
    for document, rules in sorted(by_document.items()):
        print(f"  {document}: {', '.join(sorted(rules))}")
    if unfinished:
        print("\nveraPDF did not finish:")
        for output, said in unfinished:
            print(f"  {output.profile} {output.flavour} {output.corpus}/{output.document}: {said}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
