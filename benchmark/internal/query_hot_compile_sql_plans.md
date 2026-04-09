# Query Hot Compile Benchmark: `sql-plans` Extraction

## Summary

This benchmark compares the hot compile time of `cargo check -p databend-common-sql`
before and after extracting a subset of SQL plan types into the new
`databend-common-sql-plans` crate.

The measured scenario is:

1. Warm up a dedicated `CARGO_TARGET_DIR`
2. Touch a plan definition file
3. Re-run `cargo check -p databend-common-sql`

Result: the first extraction did not improve this hot compile scenario.
The extracted version was slightly slower in the measured runs because a change in
the moved plan file still rebuilds both `databend-common-sql-plans` and
`databend-common-sql`.

## Environment

- Date: 2026-04-09
- Baseline commit: `dad47cfb9c`
- Workspace under test: `/home/kould/RustroverProjects/databend`
- Baseline tree: temporary detached `git worktree` at `HEAD`

## Benchmark Target

- Command: `cargo check -p databend-common-sql`
- Baseline touched file:
  `src/query/sql/src/planner/plans/ddl/catalog.rs`
- Extracted touched file:
  `src/query/sql/plans/src/ddl/catalog.rs`

## Commands

Baseline:

```bash
TARGET=/tmp/databend-hot-base-target
FILE=src/query/sql/src/planner/plans/ddl/catalog.rs
rm -rf "$TARGET"

/usr/bin/time -f "warm %e" \
  env CARGO_TARGET_DIR="$TARGET" cargo check -p databend-common-sql -q

touch "$FILE"
/usr/bin/time -f "hot1 %e" \
  env CARGO_TARGET_DIR="$TARGET" cargo check -p databend-common-sql -q

touch "$FILE"
/usr/bin/time -f "hot2 %e" \
  env CARGO_TARGET_DIR="$TARGET" cargo check -p databend-common-sql -q

touch "$FILE"
/usr/bin/time -f "hot3 %e" \
  env CARGO_TARGET_DIR="$TARGET" cargo check -p databend-common-sql -q
```

Extracted:

```bash
TARGET=/tmp/databend-hot-ref-target
FILE=src/query/sql/plans/src/ddl/catalog.rs
rm -rf "$TARGET"

/usr/bin/time -f "warm %e" \
  env CARGO_TARGET_DIR="$TARGET" cargo check -p databend-common-sql -q

touch "$FILE"
/usr/bin/time -f "hot1 %e" \
  env CARGO_TARGET_DIR="$TARGET" cargo check -p databend-common-sql -q

touch "$FILE"
/usr/bin/time -f "hot2 %e" \
  env CARGO_TARGET_DIR="$TARGET" cargo check -p databend-common-sql -q

touch "$FILE"
/usr/bin/time -f "hot3 %e" \
  env CARGO_TARGET_DIR="$TARGET" cargo check -p databend-common-sql -q
```

## Results

| Variant | warm | hot1 | hot2 | hot3 |
| --- | ---: | ---: | ---: | ---: |
| Baseline | 86.33s | 2.36s | 2.10s | 2.10s |
| Extracted `sql-plans` | 85.51s | 3.23s | 2.31s | 2.29s |

## Notes

- In the baseline run, touching the file rebuilt `databend-common-sql`.
- In the extracted run, touching the file rebuilt both
  `databend-common-sql-plans` and `databend-common-sql`.
- This means the new crate boundary is real, but it does not yet isolate this edit
  well enough to reduce hot compile time for `databend-common-sql`.
- The next useful experiments are:
  - move more consumers off `databend-common-sql` re-exports and depend on
    `databend-common-sql-plans` directly;
  - split `optimizer` or `interpreter` paths where a changed file can avoid
    rebuilding the top-level SQL crate.
