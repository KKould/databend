# Query Hot Compile Benchmark: `name-resolution` Extraction

## Summary

This benchmark compares the hot compile time of `cargo check -p databend-common-sql`
before and after extracting SQL name resolution utilities into the new
`databend-common-sql-name-resolution` crate.

The measured scenario is:

1. Warm up a dedicated `CARGO_TARGET_DIR`
2. Touch a name resolution source file
3. Re-run `cargo check -p databend-common-sql`

Result: this extraction also did not improve this hot compile scenario.
The extracted version was slightly slower in the measured runs because a change in
the moved file still rebuilds both `databend-common-sql-name-resolution` and
`databend-common-sql`.

## Environment

- Date: 2026-04-09
- Baseline commit: `1fe5b6d0f7`
- Workspace under test: `/home/kould/RustroverProjects/databend`
- Baseline tree: temporary detached `git worktree` at `HEAD`

## Benchmark Target

- Command: `cargo check -p databend-common-sql`
- Baseline touched file:
  `src/query/sql/src/planner/semantic/name_resolution.rs`
- Extracted touched file:
  `src/query/sql/name-resolution/src/lib.rs`

## Results

| Variant | warm | hot1 | hot2 | hot3 |
| --- | ---: | ---: | ---: | ---: |
| Baseline | 81.70s | 2.49s | 1.91s | 1.94s |
| Extracted `name-resolution` | 91.87s | 2.71s | 2.17s | 2.44s |

## Notes

- In the baseline run, touching the file rebuilt `databend-common-sql`.
- In the extracted run, touching the file rebuilt both
  `databend-common-sql-name-resolution` and `databend-common-sql`.
- This extraction cleaned up layering and makes later `metadata` and `optimizer`
  work easier, but by itself it does not reduce hot compile time for
  `databend-common-sql`.
