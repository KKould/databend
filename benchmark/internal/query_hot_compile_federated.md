# Query Hot Compile Benchmark: `federated` Extraction

## Summary

This benchmark compares the hot compile time of `cargo check -p databend-query`
before and after extracting MySQL federated compatibility logic into the new
`databend-query-federated` crate.

The measured scenario is:

1. Warm up a dedicated `CARGO_TARGET_DIR`
2. Touch the federated compatibility source file
3. Re-run `cargo check -p databend-query`

Result: this extraction does not materially improve this hot compile scenario.
The extracted version is effectively flat versus baseline, with only negligible
run-to-run variance.

## Environment

- Date: 2026-04-10
- Baseline commit: `8aa2b1c4f0`
- Workspace under test: `/home/kould/RustroverProjects/databend`
- Baseline tree: temporary detached `git worktree` at `HEAD`

## Benchmark Target

- Command: `cargo check -p databend-query`
- Baseline touched file:
  `src/query/service/src/servers/mysql/mysql_federated.rs`
- Extracted touched file:
  `src/query/service/federated/src/lib.rs`

## Results

| Variant | warm | hot1 | hot2 | hot3 |
| --- | ---: | ---: | ---: | ---: |
| Baseline | 357.17s | 6.20s | 5.03s | 4.94s |
| Extracted `federated` | 346.46s | 6.23s | 5.12s | 5.01s |

## Notes

- In the baseline run, touching the file rebuilt `databend-query`.
- In the extracted run, touching the file rebuilt both
  `databend-query-federated` and `databend-query`.
- The crate boundary is cleaner after extraction, but it is not yet an
  effective hot-compile boundary because `databend-query` still recompiles for
  edits in the moved code.
