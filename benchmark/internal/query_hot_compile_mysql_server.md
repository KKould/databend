# Query Hot Compile Benchmark: `mysql-server` Extraction

## Summary

This benchmark compares the hot compile time of `cargo check -p databend-query`
before and after extracting the MySQL listener shell and TLS setup logic into
the new `databend-query-mysql-server` crate.

The measured scenario is:

1. Warm up a dedicated `CARGO_TARGET_DIR`
2. Touch the MySQL hosting source file
3. Re-run `cargo check -p databend-query`

Result: this extraction gives a modest hot compile improvement for edits in the
moved MySQL hosting code.

## Environment

- Date: 2026-04-10
- Baseline commit: `62e71c75c2`
- Workspace under test: `/home/kould/RustroverProjects/databend`
- Baseline tree: temporary detached `git worktree` at `HEAD`

## Benchmark Target

- Command: `cargo check -p databend-query`
- Baseline touched file:
  `src/query/service/src/servers/mysql/mysql_handler.rs`
- Extracted touched file:
  `src/query/service/mysql-server/src/lib.rs`

## Results

| Variant | warm | hot1 | hot2 | hot3 |
| --- | ---: | ---: | ---: | ---: |
| Baseline | 351.76s | 6.84s | 5.02s | 5.89s |
| Extracted `mysql-server` | 358.09s | 6.39s | 5.51s | 4.92s |

## Notes

- In the baseline run, touching the file rebuilt `databend-query`.
- In the extracted run, touching the file rebuilt both
  `databend-query-mysql-server` and `databend-query`.
- Even with the extra crate rebuild, the extracted version was modestly faster
  overall in the 3-run hot sample.
- The visible rebuild run was also faster after extraction (`6.59s` to
  `5.21s`), which makes this split look more similar to `admin-server` than to
  the earlier no-gain extractions.
