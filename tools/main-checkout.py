#!/usr/bin/env python3
"""What the main checkout holds that a merge does not carry, read without touching it.

    python3 tools/main-checkout.py            # from any checkout of this repository

A round works in a worktree and may not edit the main checkout, so what a batch leaves for the
owner is a list of things on the owner's disk: a gitignored or untracked file a merge cannot
update, an owner's answer no commit holds yet (printed first, and alone under `--answers`, which
`tools/batch.sh check` repeats), a fuzz artefact whose defect is fixed, a corpus a campaign found stale, a local edit that
will stop the fast-forward, an uncommitted question whose `§` the main checkout's own conformance
run fails on, a patch a dependency's fork has not taken, an upstream report not yet filed, a build
directory over the hundred-gigabyte rule, an `sccache` cache at its ceiling, the agent's cgroup
with no task or memory limit that would have held trap 116's incident. This prints one line per
kind, with its count, and then **the owner's list**: every one of those that has something to do, once,
numbered in the order a person would do them, each with its command or its file — the counts say what
is on the disk, the list says what to do about it, and nothing is said in both (ADR 1601).
`doc/environment.md`'s *After a merge* section says what each line means and the command that clears
it, in the order printed here, so a line added here owes its entry there, and
`tools/conformance/tests/owner_section.rs` holds the two to one list in one order and the owner's list
to its shape. Every figure is read from the disk and from git, never written down (ADR 1440). It exits
non-zero only when it cannot read the main checkout.

The main checkout is the directory holding the repository's common git directory — the same
derivation `tools/batch.sh` makes — so run from the main checkout itself it reads itself.
`MAIN_CHECKOUT` names another, and `SECTION_SIGNS_BIN` the built scanner to run in place of `cargo
run`, which is how the shape test reads a planted checkout without a nested build.
`MAIN_CHECKOUT_AGENT_USER`, `MAIN_CHECKOUT_PROC` and `MAIN_CHECKOUT_CGROUP_ROOT` name the agent's
account (a name or a uid), the process table and the cgroup hierarchy the agent's scope is read from.
"""

import os
import pwd
import re
import subprocess
import sys
import time
import tomllib

HERE = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

# The owner's list's order, the order a person does the work in (ADR 1601): what stops the
# fast-forward; the `§` a question file carries, before that file is committed; the commit of the
# answers on the disk; the questions only the owner can answer; the forks and the reports, which
# are the owner's accounts' work; the agent's scope, bounded before anything walks again; the
# re-seed, a walk behind the lock; and the disk's hygiene last.
(IN_THE_WAY, SECTION_SIGNS, COMMIT, ANSWER, FORK, REPORT, APPLY, BOUND, RESEED, REMOVE, PRUNE,
 CEILING) = range(12)

# The agent's scope is bounded when the tightest `pids.max` and `memory.max` on its cgroup's path
# are at or under these: twice the task budget `tools/bounded.sh` holds every heavy command to,
# so the rlimit acts first and the cgroup is the backstop for what no rlimit reaches (the agent's
# own processes, anything started without the wrapper), and ADR 0798's `MemoryMax`. The incident
# they answer reached 52 259 tasks and 50 GB resident (trap 116, ADR 1612).
AGENT_TASKS = 16384
AGENT_MEMORY_GIB = 40


def git(directory, *arguments):
    # `--no-optional-locks`: `git status` and `git diff` otherwise refresh the index's stat cache
    # and take `index.lock` to write it back — in the owner's checkout and in a worktree a round is
    # merging. This script reads and never writes, and a state section is read-only (ADR 1487).
    result = subprocess.run(
        ["git", "--no-optional-locks", "-C", directory, *arguments], capture_output=True, text=True
    )
    return result.stdout if result.returncode == 0 else None


def main_checkout():
    if os.environ.get("MAIN_CHECKOUT"):
        return os.environ["MAIN_CHECKOUT"]
    common = git(HERE, "rev-parse", "--path-format=absolute", "--git-common-dir")
    return os.path.dirname(common.strip()) if common else None


def owe(owed, rank, text):
    """One thing for the owner to do, at its place in the list's order."""
    owed.append((rank, len(owed), text))


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


def artefacts(main, owed):
    """The artefacts libFuzzer left in the main checkout, sorted into read and unread.

    A read one is the owner's to remove, and the list carries the one `rm` that removes them all;
    an unread one is a round's to read, so it stays a line here."""
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
    lines += [f"  unread:           fuzz/artifacts/{path}" for path in unread]
    if read:
        owe(owed, REMOVE, f"remove the {len(read)} artefact(s) the tree has read: `rm -- "
            + " ".join(f"fuzz/artifacts/{path}" for path in read) + "`")
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


def unseeded(main, owed):
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
    if empty or stale:
        owe(owed, RESEED, f"re-seed the {len(empty) + len(stale)} corpora above, behind the lock: "
            f"`tools/bounded.sh --lock --round <session> --tree 12 -- fuzz/seeds.sh fuzz/corpus {' '.join(empty + stale)}`")
    return f"fuzz/corpus: {len(empty)} of {len(targets)} target(s) unseeded" + (
        f": {' '.join(empty)}" if empty else "") + (
        f"; {len(stale)} stale by {source}" + (f": {' '.join(stale)}" if stale else ""))


def answers(main, owed):
    """The owner's answers the main checkout holds and no commit does, newest first, and the
    questions still open once they are counted.

    An `A` file lands in the owner's checkout untracked (or as an edit to a tracked one) and stays
    so until the owner commits it, so a list read from tracked files — `tests/questions.rs`'s, any
    round's — calls its question open the whole time. This reads the disk: every `A` file `git
    status` reports, dated by its modification time, which is when it landed; and every `Q` whose
    number has no `A` beside it on the disk, which is what is open (ADR 1588)."""
    directory = os.path.join(main, "doc/questions")
    status = git(main, "status", "--porcelain", "--untracked-files=all", "--", "doc/questions") or ""
    landed = []
    for line in status.splitlines():
        found = re.search(r"doc/questions/((A0*(\d+))[^/]*\.md)$", line)
        if not found or not os.path.isfile(os.path.join(directory, found.group(1))):
            continue
        stamp = os.path.getmtime(os.path.join(directory, found.group(1)))
        landed.append((stamp, int(found.group(3)), found.group(2), line.startswith("??")))
    landed.sort(key=lambda entry: (-entry[0], entry[1]))
    dated = " ".join(f"{name} ({time.strftime('%Y-%m-%d', time.localtime(stamp))})"
                     for stamp, _, name, _ in landed)
    names = os.listdir(directory) if os.path.isdir(directory) else []
    files = {int(m.group(1)): m.string for m in
             (re.match(r"Q0*(\d+)-", n) for n in names if n.endswith(".md")) if m}
    asked = {number: name.split("-", 1)[0] for number, name in files.items()}
    answered = {int(m.group(1)) for m in (re.match(r"A0*(\d+)-", n) for n in names) if m}
    still_open = [asked[n] for n in sorted(set(asked) - answered)]
    # An edit to a tracked answer leaves its question answered in the tracked files already; an
    # untracked answer is one the tracked files cannot see.
    unseen = [f"Q{name[1:]}" for _, number, name, new in sorted(landed, key=lambda e: e[1])
              if new and number in asked]
    if landed:
        owe(owed, COMMIT, f"commit the {len(landed)} answer(s) on the disk, and the questions beside "
            f"them: `git status --short doc/questions`, then `git add doc/questions && git commit`")
    if still_open:
        owe(owed, ANSWER, f"answer the {len(still_open)} open question(s), each with an `A` file of "
            f"its own name: " + " ".join(f"doc/questions/{files[number]}"
                                         for number in sorted(set(asked) - answered)))
    lines = [f"answered, uncommitted: {len(landed)} in the main checkout's doc/questions, newest "
             f"first" + (f": {dated}" if dated else "")]
    lines += [f"open questions, less those answered on the disk: {len(still_open)}"
              + (f": {' '.join(still_open)}" if still_open else "")
              + (f"; the tracked files alone call {len(unseen)} more open: {' '.join(unseen)}"
                 if unseen else "")]
    return lines


def section_signs(main, owed):
    """The `§` rule over the main checkout's uncommitted instruction documents (ADR 1452).

    A file no merge carries is checked by the main checkout's own `cargo test -p conformance` and
    by no worktree's, so its finding reaches the owner only here. The scan is the conformance
    crate's own (`--bin section_signs`), run from this checkout and reading the main one."""
    status = git(main, "status", "--porcelain", "--untracked-files=all", "--", "doc") or ""
    files = [line[3:] for line in status.splitlines() if line.endswith(".md")]
    if not files:
        return ["section signs: no uncommitted document in the main checkout's doc/"]
    scanner = os.environ.get("SECTION_SIGNS_BIN")
    command = [scanner] if scanner else ["cargo", "run", "-q", "-p", "conformance", "--bin",
                                         "section_signs", "--"]
    result = subprocess.run([*command, main, *files], cwd=HERE, capture_output=True, text=True)
    if result.returncode != 0:
        return [f"section signs: the scan failed: {result.stderr.strip()[-300:]}"]
    lines = result.stdout.splitlines()
    places = {}
    for line in lines[:-1]:
        found = re.match(r"\s*(\S+?:\d+): a `§` after (.+?) —", line)
        if found:
            places.setdefault(found.group(1), []).append(found.group(2))
    if places:
        owe(owed, SECTION_SIGNS, f"write the other standard's section in words (\"ISO 19005-2 "
            f"section 6.7\") where a question file puts `§` after its name, then `cargo test -p "
            f"conformance --test documents`: "
            + "; ".join(f"doc/{place} ({', '.join(names)})" if not place.startswith("doc/")
                        else f"{place} ({', '.join(names)})" for place, names in places.items()))
    return [f"section signs, uncommitted: {lines[-1]}" if lines else "section signs: no output"]


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
    """A patch's preamble lines, from the text `git apply` skips, and the packages it touches.

    `Repository:` and `Base:` are the repository and revision it was written against. A patch
    whose paths are relative to its package (`+++ b/src/...`, as a crates.io source is laid out)
    names the package's place in the repository with `Directory:`, and its package is that
    directory's last name; otherwise the packages are the first directory of each
    `+++ b/<package>/...` line. `Fork:` names the repository under the owner's control that carries
    the patch, or is to."""
    header, packages = {}, set()
    with open(path, encoding="utf-8", errors="replace") as handle:
        for line in handle:
            found = re.match(r"(Repository|Base|Fork|Directory):\s*(\S+)", line)
            if found and not packages:
                header[found.group(1)] = found.group(2)
            target = re.match(r"\+\+\+ b/([^/\s]+)/", line)
            if target:
                packages.add(target.group(1))
    if "Directory" in header:
        packages = {header["Directory"].rstrip("/").rsplit("/", 1)[-1]}
    return header, packages


def patches(main, owed):
    """The patches under `doc/patches/` the owner still owes a dependency's fork.

    A round may not push to a fork, so a fix to a dependency is a patch beside the tree whose
    preamble names the repository and the revision it was written against; the owner applies it
    to the fork and bumps the `rev` the manifest pins. A patch is owed while the manifest still
    pins its base: a bumped `rev` is the patch applied, and it drops off the list. Where the manifest
    pins the upstream repository itself and the preamble names the `Fork:` to carry the patch — the
    `hayro` codecs, taken at upstream's release commit so that the reference renderer's features
    cannot reach the worker's copy (ADR 1714) — the owner applies it to the fork and moves those
    pins to it.

    A patch whose `Fork:` the manifest pins is applied: the fork carries it, as `close2/zune-image`
    carries both `zune-jpeg` patches (ADR 1730). A patch to a dependency the manifest takes from
    crates.io has no fork to apply it to until one exists. While the question its preamble names is
    unanswered it is listed as waiting; once the answer is on the disk and the preamble names the
    `Fork:` to carry it, the fork is the owner's to create, and the line after the patches says how
    in one sentence (ADR 1589).

    What the owner does is the list's: one item per fork to create and one per fork to apply to,
    each naming its patches, so a patch is named once. An upstream report beside a patch (its `.md`,
    whose first lines say where it is to be filed) is the list's until the report carries a
    `Filed:` line naming the issue, which the owner writes when it is filed (ADR 1601)."""
    directory = os.path.join(main, "doc/patches")
    names = sorted(n for n in os.listdir(directory) if n.endswith(".patch")) if os.path.isdir(directory) else []
    pins = pinned(main)
    if pins is None:
        return ["doc/patches: Cargo.toml not readable here"]
    questions = os.path.join(main, "doc/questions")
    on_disk = os.listdir(questions) if os.path.isdir(questions) else []
    to_apply, applied, unstated, waiting, to_fork, forks_owed = {}, 0, [], [], [], {}
    forks = {repository for repository, _ in pins.values()}
    for name in names:
        header, packages = patch_header(os.path.join(directory, name))
        base, repository, fork = header.get("Base"), header.get("Repository"), header.get("Fork")
        if not base or not repository:
            unstated.append(name)
            continue
        if fork in forks:
            applied += 1
            continue
        if repository not in forks:
            with open(os.path.join(directory, name), encoding="utf-8", errors="replace") as handle:
                question = re.search(r"doc/questions/Q(\d+)", handle.read())
            answer = question and next((n for n in on_disk if re.match(
                rf"A0*{int(question.group(1))}-", n)), None)
            if answer and fork:
                to_fork.append(name)
                forks_owed.setdefault(fork, (repository, base, header.get("Directory"), packages,
                                             answer, []))[5].append(name)
                continue
            waiting.append(f"  waiting:          doc/patches/{name} — {repository}, which the manifest "
                           f"pins no fork of; it waits on "
                           f"{question.group(0) if question else 'a decision its preamble does not name'}")
            continue
        held = [package for package in sorted(packages)
                if pins.get(package, (None, None)) == (repository, base)]
        if held:
            to_apply.setdefault((repository, base, fork), []).append(name)
        else:
            applied += 1
    for fork, (repository, base, place, packages, answer, names) in forks_owed.items():
        crate = " ".join(sorted(packages))
        owe(owed, FORK, f"create the fork doc/questions/{answer} asks for: fork {repository} as "
            f"{fork}, apply " + " ".join(f"doc/patches/{n}" for n in names) + f" on {base} with "
            f"`git apply --directory={place}`, push, and put the pushed commit in the `rev` of the "
            f"stanza the root Cargo.toml's comment above `{crate} =` writes out, in place of that line")
    for (repository, base, fork), names in to_apply.items():
        moved = (f"move every stanza the root Cargo.toml pins to {repository} at {base[:12]} to "
                 f"{fork} at the pushed commit" if fork and fork != repository else
                 f"move every `rev` the root Cargo.toml pins to it to the pushed commit")
        owe(owed, APPLY, f"apply " + " ".join(f"doc/patches/{n}" for n in names) + f" to "
            f"{fork or repository} on {base[:12]}, push, and {moved}; then `cargo update` the "
            f"packages and `cargo test -p conformance --test fuzz_workspace`")
    for report in reports(directory):
        owe(owed, REPORT, report)
    owed_count = sum(len(names) for names in to_apply.values())
    lines = [f"doc/patches: {owed_count} owed while the manifest still pins the patch's base, "
             f"{applied} whose base it no longer pins, {len(to_fork)} for a fork the owner is to "
             f"create, {len(waiting)} waiting for a fork the manifest does not have, "
             f"{len(unstated)} stating no base"]
    lines += waiting
    lines += [f"  no Repository:/Base: preamble: doc/patches/{name}" for name in unstated]
    return lines


def reports(directory):
    """Each upstream report under `doc/patches/` not yet filed, as the list's item."""
    by_place = {}
    for name in sorted(os.listdir(directory)) if os.path.isdir(directory) else []:
        if not name.endswith(".md"):
            continue
        with open(os.path.join(directory, name), encoding="utf-8", errors="replace") as handle:
            text = handle.read()
        where = re.search(r"to file at <([^>]+)>", text)
        if where and not re.search(r"^Filed:", text, re.M):
            by_place.setdefault(where.group(1), []).append(f"doc/patches/{name}")
    return [f"file the {len(names)} upstream report(s) at {place}, and write `Filed: <the issue>` at "
            f"the head of each: " + " ".join(names) for place, names in by_place.items()]


def kib(text):
    """A size written the way sccache's configuration writes one (`50G`, `512M`), in KiB."""
    found = re.fullmatch(r"\s*(\d+)\s*([KMGT]?)i?B?\s*", text or "", re.I)
    if not found:
        return None
    return int(found.group(1)) * 1024 ** "KMGT".index((found.group(2) or "K").upper())


def sized(kibibytes):
    """A size in KiB as a person reads it."""
    for unit, scale in (("TiB", 1024 ** 3), ("GiB", 1024 ** 2), ("MiB", 1024)):
        if kibibytes >= scale:
            return f"{kibibytes / scale:.0f} {unit}"
    return f"{kibibytes} KiB"


def du_kib(path):
    result = subprocess.run(["du", "-sk", path], capture_output=True, text=True)
    first = result.stdout.split()
    return int(first[0]) if first and first[0].isdigit() else None


def disk(main, owed):
    """The main checkout's build directory against the hundred-gigabyte rule, and `sccache`'s
    cache against its ceiling (`doc/environment.md`'s build-directory entry, ADR 1500).

    The build directory is the one the main checkout's configuration names, else this user's,
    else `target/`; the rule is `MAIN_CHECKOUT_BUILD_RULE_KIB` or a hundred gigabytes. The cache is
    `SCCACHE_DIR` or the default, its ceiling `SCCACHE_CACHE_SIZE`, the configuration's `size`, or
    sccache's default of ten gigabytes; a cache within a twentieth of its ceiling is evicting by
    age, which costs the warm builds their oldest entries and nothing else."""
    built = None
    for config in (os.path.join(main, ".cargo/config.toml"),
                   os.path.expanduser("~/.cargo/config.toml")):
        try:
            with open(config, "rb") as handle:
                built = tomllib.load(handle).get("build", {}).get("target-dir")
        except (OSError, tomllib.TOMLDecodeError):
            continue
        if built:
            break
    built = built or os.path.join(main, "target")
    rule = int(os.environ.get("MAIN_CHECKOUT_BUILD_RULE_KIB", 100 * 1024 * 1024))
    size = du_kib(built) if os.path.isdir(built) else None
    lines = [f"build directory: {built}, "
             + (f"{sized(size)} against the rule of {sized(rule)}"
                if size is not None else "not on this disk")]
    if size is not None and size > rule:
        owe(owed, PRUNE, f"prune the main checkout's build directory, with no round running: "
            f"`rm -rf {built}/debug {built}/gates`")
    cache = os.environ.get("SCCACHE_DIR") or os.path.expanduser("~/.cache/sccache")
    ceiling = kib(os.environ.get("SCCACHE_CACHE_SIZE"))
    conf = os.environ.get("SCCACHE_CONF") or os.path.expanduser("~/.config/sccache/config")
    if ceiling is None:
        try:
            with open(conf, "rb") as handle:
                ceiling = kib(tomllib.load(handle).get("cache", {}).get("disk", {}).get("size"))
        except (OSError, tomllib.TOMLDecodeError):
            ceiling = None
    ceiling = ceiling or 10 * 1024 * 1024
    held = du_kib(cache) if os.path.isdir(cache) else None
    lines.append(f"sccache: {cache}, " + (f"{sized(held)} of a {sized(ceiling)} ceiling"
                                          if held is not None else "no cache on this disk"))
    if held is not None and held * 20 >= ceiling * 19:
        owe(owed, CEILING, f"decide sccache's ceiling: the cache is at it and evicting by age; "
            f"raise `size` under `[cache.disk]` in {conf}, or leave it to evict")
    return lines


def limit(text):
    """A cgroup limit file's value, `max` as no limit."""
    text = (text or "").strip()
    return int(text) if text.isdigit() else float("inf")


def tightest(hierarchy, path, name):
    """The smallest `name` limit on the cgroup at `path` and every ancestor: a limit anywhere above
    a scope bounds it as well."""
    smallest, parts = float("inf"), [part for part in path.split("/") if part]
    for depth in range(len(parts), -1, -1):
        try:
            with open(os.path.join(hierarchy, *parts[:depth], name), encoding="utf-8") as handle:
                smallest = min(smallest, limit(handle.read()))
        except OSError:
            continue
    return smallest


def agent_tasks(proc, uid):
    """Each cgroup holding a process whose real uid is the agent's, with its task count."""
    scopes = {}
    for entry in os.listdir(proc) if os.path.isdir(proc) else []:
        if not entry.isdigit():
            continue
        try:
            with open(os.path.join(proc, entry, "status"), encoding="utf-8") as handle:
                status = handle.read()
            with open(os.path.join(proc, entry, "cgroup"), encoding="utf-8") as handle:
                membership = handle.read()
        except OSError:
            continue
        real = re.search(r"^Uid:\s+(\d+)", status, re.M)
        threads = re.search(r"^Threads:\s+(\d+)", status, re.M)
        unified = re.search(r"^0::(\S+)", membership, re.M)
        if real and unified and int(real.group(1)) == uid:
            scopes[unified.group(1)] = scopes.get(unified.group(1), 0) + (
                int(threads.group(1)) if threads else 1)
    return scopes


def agent_scope(owed):
    """The cgroups the agent's processes run in, against the task and memory limits that would have
    held trap 116's incident (ADR 1612).

    The agent runs in the owner's own session, a terminal tab's scope beneath the owner's slice, so
    no rlimit this tree sets reaches the agent itself and the cgroup is the owner's to set (ADR
    0798). A tab's limit files belong to the owner, who writes them without root; a session started
    in a scope of its own carries the limits from its launch. The scope is read from the processes,
    because the tab changes with every session."""
    user = os.environ.get("MAIN_CHECKOUT_AGENT_USER", "AI")
    proc = os.environ.get("MAIN_CHECKOUT_PROC", "/proc")
    hierarchy = os.environ.get("MAIN_CHECKOUT_CGROUP_ROOT", "/sys/fs/cgroup")
    try:
        uid = int(user) if user.isdigit() else pwd.getpwnam(user).pw_uid
    except KeyError:
        return [f"agent's cgroup: no user {user} on this machine, so no scope to read"]
    scopes = agent_tasks(proc, uid)
    memory = AGENT_MEMORY_GIB * 1024 ** 3
    unbounded, lines = [], []
    for path, tasks in sorted(scopes.items()):
        pids, held = tightest(hierarchy, path, "pids.max"), tightest(hierarchy, path, "memory.max")
        if pids > AGENT_TASKS or held > memory:
            unbounded.append(path)
        shown = ["max" if figure == float("inf") else str(figure) for figure in (pids, held)]
        lines.append(f"  scope:            {path} — {tasks} task(s); the tightest pids.max on its "
                     f"path {shown[0]}, memory.max {shown[1]}")
    if unbounded:
        writes = " && ".join(f"echo {AGENT_TASKS} > '{hierarchy}{path}/pids.max' && echo "
                             f"{AGENT_MEMORY_GIB}G > '{hierarchy}{path}/memory.max'"
                             for path in unbounded)
        owe(owed, BOUND, f"bound the agent's scope at {AGENT_TASKS} tasks and {AGENT_MEMORY_GIB} "
            f"GiB, as yourself, who owns the files: `{writes}`; or start the next session in a scope "
            f"of its own: `systemd-run --user --scope -p TasksMax={AGENT_TASKS} -p MemoryHigh=36G "
            f"-p MemoryMax={AGENT_MEMORY_GIB}G -p MemorySwapMax=4G sudo -u {user} bash -lc 'cd "
            f"/home/cl/projects/pdf-viewer && claude'`")
    return [f"agent's cgroup: {len(scopes)} scope(s) hold user {user}'s processes, "
            f"{len(unbounded)} with a limit above {AGENT_TASKS} tasks or {AGENT_MEMORY_GIB} GiB"] + lines


def in_the_way(main, owed):
    """Local edits in the main checkout to paths this branch changes: each stops `--ff-only`."""
    head = (git(main, "rev-parse", "HEAD") or "").strip()
    here = set((git(HERE, "diff", "--name-only", head) or "").split()) if head else set()
    here |= {line[3:] for line in (git(HERE, "status", "--porcelain") or "").splitlines()}
    local = set((git(main, "diff", "--name-only", "HEAD") or "").split())
    clash = sorted(here & local) if os.path.realpath(main) != os.path.realpath(HERE) else []
    if clash:
        owe(owed, IN_THE_WAY, f"set aside the local edit(s) to {' '.join(clash)} around the "
            f"fast-forward: `git diff -- <path> > /tmp/owner.patch && git checkout -- <path>`, the "
            f"merge, then `git apply --3way /tmp/owner.patch`")
    return f"local edits the fast-forward would refuse over: {len(clash)}" + (
        f" ({' '.join(clash)})" if clash else "")


def owners_list(owed):
    """Everything the lines above found for the owner to do, once each, numbered in the order a
    person does them (ADR 1601): each item carries its command in backticks or the files it acts
    on, and an item that acts on several files names them together rather than once a line."""
    lines = [f"the owner's list, in the order to do them: {len(owed)}"
             + ("" if owed else ", so nothing is owed")]
    lines += [f"  {number}. {text}" for number, (_, _, text) in enumerate(sorted(owed), start=1)]
    return lines


def main():
    main_dir = main_checkout()
    if not main_dir or not os.path.isdir(main_dir):
        print("main-checkout.py: the main checkout cannot be found from here")
        return 1
    owed = []
    print(f"main checkout: {main_dir}")
    for line in answers(main_dir, owed):
        print(line)
    if sys.argv[1:] == ["--answers"]:
        return 0
    print(in_the_way(main_dir, owed))
    print(fuzz_lock(main_dir))
    for line in artefacts(main_dir, owed):
        print(line)
    print(unseeded(main_dir, owed))
    for line in patches(main_dir, owed):
        print(line)
    for line in section_signs(main_dir, owed):
        print(line)
    for line in disk(main_dir, owed):
        print(line)
    for line in agent_scope(owed):
        print(line)
    for line in owners_list(owed):
        print(line)
    return 0


if __name__ == "__main__":
    sys.exit(main())
