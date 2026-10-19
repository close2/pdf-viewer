# 2005 — A god class is ranked by its `impl` block, and the census is a reading list

Session 1584, slot 8 of batch ninety-two. Status: **accepted** and **built** in
`tools/god-census.py`, `tools/state.sh gods` and `doc/todo/74-the-god-classes.md`. Context: ADR 2004;
the brief's premise that the largest file by lines is the top entry.

## 1. The decision

`tools/god-census.py` prints four tables, and none of them is a score. **Files** by lines (with the
share inside `#[cfg(test)]`), **`impl` blocks** by lines (with their methods), **types** by methods
summed over every `impl` of one name within a crate, and **functions** by lines. `doc/todo/74` orders
what the tables show by one rule: **the largest `impl` block first**, because a class is the behaviour
one type carries in one place, and a file of many small types or of tests is not one. A type spread
over eighteen files (`Interpreter`) is ranked by its blocks, and the types table says it exists; the
split a type needs is read per block.

Files that are lists of scenes (`pdf-model/tests/oracle.rs`, `viewer-core/tests/headless.rs`) top the
files table and are not god classes. The census prints them and the todo file says so, rather than
the census hiding them: a reader who sees the largest file should find the sentence that explains why
it is not the first entry.

## 2. What the census does not do

It is not a gate and carries no threshold; a threshold would be a number chosen to pass. It does not
parse Rust: comments, strings and character literals are masked so braces count, and an `impl` a
macro produces is not seen. `tools/state.sh gods` prints the top five of each table, so a round that
splits an entry finds the table moved.
