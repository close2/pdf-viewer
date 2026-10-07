# 1601 — The owner's list is one numbered list, in the order a person does it

Session 1382. Status: **accepted**. Extends ADRs 1440 and 1588's printer of what a merge cannot
carry; it still reads the main checkout and never writes it (ADR 1487). Amends ADR 1589's last
step, which printed the fork's sentence as a sub-line of `doc/patches`.
Context: `doc/environment.md` *After a merge*; `tools/conformance/tests/owner_section.rs`.
Code: `tools/main-checkout.py` (`owe`, `owners_list`, `reports`, `disk`, and each kind's function
taking the list), `tools/state.sh` (`section_main_checkout`'s comment).

## 1. What was wrong

By batch fifty-eight the owner had about a dozen things to do: create a fork, apply five patches,
re-seed nine corpora, file two upstream reports, answer three questions, rewrite three `§`, decide
on `sccache`'s ceiling, and prune a build directory that had grown to six hundred gigabytes. They
were spread over seven kinds of line, in the order the script read the disk. Some had their
command at the end of a count line, some in a sub-line, and some not at all: the reports, the
ceiling and the build directory were in no line. A person had to put the steps in order by hand,
and a patch could appear twice, once as an `owed:` line and again in the fork's sentence.

## 2. The decision

The kind lines stay, one per kind, and say **what is on the disk**: counts, and the sub-lines that
are a round's to act on (`unread:`, `waiting:`, a missing preamble). Every action is the owner's,
and each is said **once**, in **one numbered list** printed last. Each item carries its command in
backticks or the files it acts on. An item that acts on several files names them all, so a patch,
a report or an artefact is named in one place. The order is the one a person does the work in, and
it is fixed as ranks in the script:

1. set aside a local edit that would stop the fast-forward;
2. rewrite a question file's `§` after another standard's name, before that file is committed;
3. commit the answers on the disk, and the questions beside them;
4. answer the questions still open;
5. create a fork an answer asks for; file the upstream reports; apply the patches owed to a pinned
   fork — all three are work in the owner's own accounts;
6. re-seed the corpora, a walk behind the lock;
7. remove the artefacts the tree has read, prune a build directory over the hundred-gigabyte
   rule, and decide `sccache`'s ceiling once its cache is at it.

Three of these are new readings. **An upstream report** is a `.md` under `doc/patches/` whose head
says where it is to be filed. It is owed until the owner writes a `Filed:` line naming the issue,
because the issue tracker cannot be read without credentials and the file is the one place that
can say it was done. **The build directory** is the one the main checkout's configuration names,
read against the rule `doc/environment.md` states. **`sccache`'s cache** is read against its
ceiling, and it is at the ceiling when it is within a twentieth of it. That is the owner's choice,
not a prune: at its ceiling the cache evicts by age.

## 3. How it is held

`owner_section.rs` runs the script against a planted main checkout that owes one of each item, with
`MAIN_CHECKOUT` naming it and `SECTION_SIGNS_BIN` the scanner the test target already built. It
checks that the list is the last thing printed, numbered from one with no gap, in the ranks'
order. It checks that every item carries a command or a file, and that no file an item names is
named by another item or by a line above the list. The kinds and `doc/environment.md`'s entries
are still one list in one order, and the list's head is a kind with an entry of its own.
