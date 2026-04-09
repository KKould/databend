# Query Hot Compile Benchmark: `http-handler-server` Extraction

## Summary

This benchmark compares the hot compile time of `cargo check -p databend-query`
before and after extracting the HTTP handler hosting and TLS lifecycle logic
into the new `databend-query-http-handler-server` crate.

The measured scenario is:

1. Warm up a dedicated `CARGO_TARGET_DIR`
2. Touch the HTTP handler hosting source file
3. Re-run `cargo check -p databend-query`

Result: this extraction does not show a consistent hot compile improvement in
the measured runs. The extracted version was slightly slower on the 3-run hot
compile sample, even though one visible rebuild run was faster.

## Environment

- Date: 2026-04-10
- Baseline commit: `3a276879be`
- Workspace under test: `/home/kould/RustroverProjects/databend`
- Baseline tree: temporary detached `git worktree` at `HEAD`

## Benchmark Target

- Command: `cargo check -p databend-query`
- Baseline touched file:
  `src/query/service/src/servers/http/http_services.rs`
- Extracted touched file:
  `src/query/service/http-handler-server/src/lib.rs`

## Results

| Variant | warm | hot1 | hot2 | hot3 |
| --- | ---: | ---: | ---: | ---: |
| Baseline | 329.06s | 6.69s | 5.15s | 5.17s |
| Extracted `http-handler-server` | 346.81s | 6.65s | 6.00s | 5.96s |

## Notes

- In the baseline run, touching the file rebuilt `databend-query`.
- In the extracted run, touching the file rebuilt both
  `databend-query-http-handler-server` and `databend-query`.
- The extracted version was slower on the 3-run hot sample overall, so this is
  not a clear win.
- One visible rebuild run was faster after extraction (`7.25s` to `5.82s`),
  which suggests there is some variance in this path, but the repeated sample
  still does not justify treating this split as a reliable hot compile
  improvement yet.
