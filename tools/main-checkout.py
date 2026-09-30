#!/usr/bin/env python3
"""What the main checkout holds that a merge does not carry, read without touching it.

    python3 tools/main-checkout.py            # from any checkout of this repository

A round works in a worktree and may not edit the main checkout, so what a batch leaves for the
owner is a list of things on the owner's disk: a gitignored or untracked file a merge cannot
update, a fuzz artefact whose defect is fixed, a corpus a campaign found stale, a local edit that
will stop the fast-forward. `doc/environment.md`'s *After a merge* section is the commands; this
prints which of them has anything to do today. Every figure is read from the disk and from git,
never written down (ADR 1440). It exits non-zero only when it cannot read the main checkout.

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
    result = subprocess.run(["git", "-C", directory, *arguments], capture_output=True, text=True)
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
    return f"fuzz/corpus: {len(empty)} of {len(targets)} target(s) unseeded" + (
        f": {' '.join(empty)}" if empty else "")


def uncommitted_answers(main):
    status = git(main, "status", "--porcelain", "--untracked-files=all", "--", "doc/questions") or ""
    answers = [line[3:] for line in status.splitlines() if re.search(r"/A\d+[^/]*\.md$", line)]
    return f"doc/questions: {len(answers)} owner's answer file(s) uncommitted in the main checkout"


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
    return 0


if __name__ == "__main__":
    sys.exit(main())
