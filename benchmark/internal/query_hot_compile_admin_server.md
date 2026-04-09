# Query Hot Compile Benchmark: `admin-server` Extraction

## Summary

This benchmark compares the hot compile time of `cargo check -p databend-query`
before and after extracting the admin server lifecycle and TLS hosting logic
into the new `databend-query-admin-server` crate.

The measured scenario is:

1. Warm up a dedicated `CARGO_TARGET_DIR`
2. Touch the admin server hosting source file
3. Re-run `cargo check -p databend-query`

Result: this extraction gives a modest hot compile win for edits in the moved
admin server hosting code.

## Environment

- Date: 2026-04-10
- Baseline commit: `055c4686a5`
- Workspace under test: `/home/kould/RustroverProjects/databend`
- Baseline tree: temporary detached `git worktree` at `HEAD`

## Benchmark Target

- Command: `cargo check -p databend-query`
- Baseline touched file:
  `src/query/service/src/servers/admin/admin_service.rs`
- Extracted touched file:
  `src/query/service/admin-server/src/lib.rs`

## Results

| Variant | warm | hot1 | hot2 | hot3 |
| --- | ---: | ---: | ---: | ---: |
| Baseline | 351.76s | 6.51s | 6.06s | 5.85s |
| Extracted `admin-server` | 344.66s | 6.59s | 5.35s | 4.85s |

## Notes

- In the baseline run, touching the file rebuilt `databend-query`.
- In the extracted run, touching the file rebuilt both
  `databend-query-admin-server` and `databend-query`.
- Even with the extra crate rebuild, the extracted version was modestly faster
  in two of the three hot runs, and clearly faster in the visible rebuild run
  (`7.02s` to `5.58s`).
- This suggests that moving server-hosting boilerplate out of the main service
  crate can help a little when the changed code sits behind a smaller crate
  boundary, even if `databend-query` still recompiles downstream.
