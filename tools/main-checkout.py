#!/usr/bin/env python3
"""What the main checkout holds that a merge does not carry, read without touching it.

    python3 tools/main-checkout.py            # from any checkout of this repository

A round works in a worktree and may not edit the main checkout, so what a batch leaves for the
owner is a list of things on the owner's disk: a gitignored or untracked file a merge cannot
update, a fuzz artefact whose defect is fixed, a corpus a campaign found stale, a local edit that
will stop the fast-forward, an uncommitted question whose `§` the main checkout's own conformance
run fails on, a patch a dependency's fork has not taken. This prints one line per kind, with its
count; `doc/environment.md`'s *After a merge* section says what each line means and the command that
clears it, in the order printed here, so a line added here owes its entry there, and
`tools/conformance/tests/owner_section.rs` holds the two to one list in one order. Every figure is
read from the disk and from git, never written down (ADR 1440). It exits non-zero only when it cannot read the main checkout.

The main checkout is the directory holding the repository's common git directory — the same
derivation `tools/batch.sh` makes — so run from the main checkout itself it reads itself.
"""

import os
import re
import subprocess
import sys
import tomllib

HERE = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))


def git(directory, *arguments):
    # `--no-optional-locks`: `git status` and `git diff` otherwise refresh the index's stat cache
    # and take `index.lock` to write it back — in the owner's checkout and in a worktree a round is
    # merging. This script reads and never writes, and a state section is read-only (ADR 1487).
    result = subprocess.run(
        ["git", "--no-optional-locks", "-C", directory, *arguments], capture_output=True, text=True
    )
    return result.stdout if result.returncode == 0 else None


def main_checkout():
    common = git(HERE, "rev-parse", "--path-format=absolute", "--git-common-dir")
    return os.path.dirname(common.strip()) if common else None


def locked(path):
    """Each package a `Cargo.lock` holds, by name, with the versions it pins."""
    try:
        with open(path, "rb") as handle:
            packages = tomllib.load(handle).get("package", [])
    except (OSError, tomllib.TOMLDecodeError):
        return None
    held = {}
    for package in packages:
        held.setdefault(package["name"], set()).add(package["version"])
    return held


def fuzz_lock(main):
    """Whether the main checkout's fuzz lock pins only what its root lock pins, ADR 1439's rule."""
    tracked = git(main, "ls-files", "--error-unmatch", "fuzz/Cargo.lock") is not None
    root, fuzz = locked(os.path.join(main, "Cargo.lock")), locked(os.path.join(main, "fuzz/Cargo.lock"))
    if root is None or fuzz is None:
        return f"fuzz/Cargo.lock: {'tracked' if tracked else 'untracked'}, not readable here"
    apart = sorted(f"{name} {version}" for name, versions in fuzz.items() if name in root
                   for version in versions - root[name])
    state = "agrees with Cargo.lock" if not apart else f"pins {len(apart)} version(s) Cargo.lock does not: {', '.join(apart)}"
    return f"fuzz/Cargo.lock: {'tracked' if tracked else 'untracked, so no merge updates it'}, {state}"


def named_in_tree(prefix):
    """Whether the committed tree names an artefact by its hash's first eight digits."""
    found = git(HERE, "grep", "-l", "-F", prefix, "--", "crates", "doc", "tools", "fuzz", "raster")
    return bool(found and found.strip())


def artefacts(main):
    """The artefacts libFuzzer left in the main checkout, sorted into read and unread."""
    base = os.path.join(main, "fuzz/artifacts")
    read, unread, slow = [], [], 0
    for target in sorted(os.listdir(base)) if os.path.isdir(base) else []:
        for name in sorted(os.listdir(os.path.join(base, target))):
            found = re.match(r"(crash|timeout|oom|slow-unit|leak)-([0-9a-f]{8})", name)
            if not found:
                continue
            if found.group(1) == "slow-unit":
                slow += 1
                continue
            (read if named_in_tree(found.group(2)) else unread).append(f"{target}/{name}")
    lines = [f"fuzz/artifacts: {len(read)} read (the tree names them, so a fix and its test hold "
             f"them), {len(unread)} unread, {slow} slow-unit warning(s)"]
    lines += [f"  read, removable:  fuzz/artifacts/{path}" for path in read]
    lines += [f"  unread:           fuzz/artifacts/{path}" for path in unread]
    return lines


def stale_corpora(main, targets):
    """The targets whose disk corpus the last census found stale, and the record that says so.

    `tools/state.sh fuzz-stale` is a census behind the lock, and a state section writes nothing
    (ADR 1487), so its finding lives where a round puts it: the newest record under `doc/history/`
    whose `**Stale today:**` sentence names targets. The line names that record, because a census
    is of its day and the corpus may have been re-seeded since (ADR 1575)."""
    history = os.path.join(main, "doc/history")
    names = sorted((n for n in os.listdir(history) if re.match(r"\d+-.*\.md$", n)),
                   key=lambda n: int(n.split("-")[0]), reverse=True) if os.path.isdir(history) else []
    for name in names:
        with open(os.path.join(history, name), encoding="utf-8", errors="replace") as handle:
            text = handle.read()
        sentence = re.search(r"\*\*Stale today:\*\*(.*?\.)(\s|$)", text, re.S)
        if sentence:
            found = [t for t in re.findall(r"`([a-z0-9_]+)`", sentence.group(1)) if t in targets]
            return found, f"doc/history/{name}'s census"
    return [], "no census any record states"


def unseeded(main):
    manifest = os.path.join(main, "fuzz/Cargo.toml")
    try:
        with open(manifest, "rb") as handle:
            targets = [b["name"] for b in tomllib.load(handle).get("bin", [])]
    except (OSError, tomllib.TOMLDecodeError):
        return "fuzz/corpus: fuzz/Cargo.toml not readable here"
    empty = [t for t in targets
             if not os.path.isdir(os.path.join(main, "fuzz/corpus", t))
             or not os.listdir(os.path.join(main, "fuzz/corpus", t))]
    stale, source = stale_corpora(main, targets)
    stale = [t for t in stale if t not in empty]
    owed = empty + stale
    return f"fuzz/corpus: {len(empty)} of {len(targets)} target(s) unseeded" + (
        f": {' '.join(empty)}" if empty else "") + (
        f"; {len(stale)} stale by {source}" + (f": {' '.join(stale)}" if stale else "")) + (
        f"; the owner's re-seed, behind the lock: flock /home/AI/heavy-walk.lock fuzz/seeds.sh "
        f"fuzz/corpus {' '.join(owed)}" if owed else "")


def uncommitted_answers(main):
    status = git(main, "status", "--porcelain", "--untracked-files=all", "--", "doc/questions") or ""
    answers = [line[3:] for line in status.splitlines() if re.search(r"/A\d+[^/]*\.md$", line)]
    return f"doc/questions: {len(answers)} owner's answer file(s) uncommitted in the main checkout"


def section_signs(main):
    """The `§` rule over the main checkout's uncommitted instruction documents (ADR 1452).

    A file no merge carries is checked by the main checkout's own `cargo test -p conformance` and
    by no worktree's, so its finding reaches the owner only here. The scan is the conformance
    crate's own (`--bin section_signs`), run from this checkout and reading the main one."""
    status = git(main, "status", "--porcelain", "--untracked-files=all", "--", "doc") or ""
    files = [line[3:] for line in status.splitlines() if line.endswith(".md")]
    if not files:
        return ["section signs: no uncommitted document in the main checkout's doc/"]
    result = subprocess.run(["cargo", "run", "-q", "-p", "conformance", "--bin",
                             "section_signs", "--", main, *files],
                            cwd=HERE, capture_output=True, text=True)
    if result.returncode != 0:
        return [f"section signs: the scan failed: {result.stderr.strip()[-300:]}"]
    lines = result.stdout.splitlines()
    return [f"section signs, uncommitted: {lines[-1]}" if lines else "section signs: no output",
            *lines[:-1]]


def pinned(main):
    """Each git dependency the main checkout's `Cargo.toml` pins, by package name: (repository, rev)."""
    try:
        with open(os.path.join(main, "Cargo.toml"), "rb") as handle:
            manifest = tomllib.load(handle)
    except (OSError, tomllib.TOMLDecodeError):
        return None
    found = {}

    def walk(table):
        for name, value in table.items():
            if isinstance(value, dict):
                if "git" in value and "rev" in value:
                    found[name] = (value["git"], value["rev"])
                else:
                    walk(value)
    walk(manifest)
    return found


def patch_header(path):
    """A patch's `Repository:` and `Base:` lines, from the preamble `git apply` skips, and the
    packages its paths touch — the first directory of each `+++ b/<package>/...` line."""
    header, packages = {}, set()
    with open(path, encoding="utf-8", errors="replace") as handle:
        for line in handle:
            found = re.match(r"(Repository|Base):\s*(\S+)", line)
            if found and not packages:
                header[found.group(1)] = found.group(2)
            target = re.match(r"\+\+\+ b/([^/\s]+)/", line)
            if target:
                packages.add(target.group(1))
    return header, packages


def patches(main):
    """The patches under `doc/patches/` the owner still owes a dependency's fork.

    A round may not push to a fork, so a fix to a dependency is a patch beside the tree whose
    preamble names the repository and the revision it was written against; the owner applies it
    to the fork and bumps the `rev` the manifest pins. A patch is owed while the manifest still
    pins its base: a bumped `rev` is the patch applied, and it drops off the list. A patch whose
    repository the manifest pins no fork of at all — `zune-jpeg`, taken from crates.io — is neither:
    it waits on the decision to carry a fork, which its preamble names, and is listed as waiting
    rather than counted as applied."""
    directory = os.path.join(main, "doc/patches")
    names = sorted(n for n in os.listdir(directory) if n.endswith(".patch")) if os.path.isdir(directory) else []
    pins = pinned(main)
    if pins is None:
        return ["doc/patches: Cargo.toml not readable here"]
    owed, applied, unstated, waiting = [], 0, [], []
    forks = {repository for repository, _ in pins.values()}
    for name in names:
        header, packages = patch_header(os.path.join(directory, name))
        base, repository = header.get("Base"), header.get("Repository")
        if not base or not repository:
            unstated.append(name)
            continue
        if repository not in forks:
            with open(os.path.join(directory, name), encoding="utf-8", errors="replace") as handle:
                question = re.search(r"doc/questions/Q\d+", handle.read())
            waiting.append(f"  waiting:          doc/patches/{name} — {repository}, which the manifest "
                           f"pins no fork of; it waits on "
                           f"{question.group(0) if question else 'a decision its preamble does not name'}")
            continue
        held = [package for package in sorted(packages)
                if pins.get(package, (None, None)) == (repository, base)]
        if held:
            owed.append(f"  owed:             doc/patches/{name} — {repository} at {base[:12]}, "
                        f"pinned by {' '.join(held)}; apply it to the fork and bump `rev`")
        else:
            applied += 1
    lines = [f"doc/patches: {len(owed)} owed to a fork the manifest still pins at the patch's base, "
             f"{applied} whose base it no longer pins, {len(waiting)} waiting for a fork the manifest "
             f"does not have, {len(unstated)} stating no base"]
    lines += owed
    lines += waiting
    lines += [f"  no Repository:/Base: preamble: doc/patches/{name}" for name in unstated]
    return lines


def in_the_way(main):
    """Local edits in the main checkout to paths this branch changes: each stops `--ff-only`."""
    head = (git(main, "rev-parse", "HEAD") or "").strip()
    here = set((git(HERE, "diff", "--name-only", head) or "").split()) if head else set()
    here |= {line[3:] for line in (git(HERE, "status", "--porcelain") or "").splitlines()}
    local = set((git(main, "diff", "--name-only", "HEAD") or "").split())
    clash = sorted(here & local) if os.path.realpath(main) != os.path.realpath(HERE) else []
    return f"local edits the fast-forward would refuse over: {len(clash)}" + (
        f" ({' '.join(clash)})" if clash else "")


def main():
    main_dir = main_checkout()
    if not main_dir or not os.path.isdir(main_dir):
        print("main-checkout.py: the main checkout cannot be found from here")
        return 1
    print(f"main checkout: {main_dir}")
    print(in_the_way(main_dir))
    print(fuzz_lock(main_dir))
    for line in artefacts(main_dir):
        print(line)
    print(unseeded(main_dir))
    print(uncommitted_answers(main_dir))
    for line in patches(main_dir):
        print(line)
    for line in section_signs(main_dir):
        print(line)
    return 0


if __name__ == "__main__":
    sys.exit(main())
