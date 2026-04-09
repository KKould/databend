# Query Hot Compile Benchmark: `server-api` / `metrics-server` Extraction

## Summary

This benchmark compares the hot compile time of `cargo check -p databend-query`
before and after extracting server-layer pieces into:

- `databend-query-server-api`
- `databend-query-metrics-server`

Two hot-edit scenarios were measured:

1. Editing the metrics implementation moved from
   `src/query/service/src/servers/metrics/metric_service.rs` to
   `src/query/service/metrics-server/src/lib.rs`
2. Editing the shared server interface moved from
   `src/query/service/src/servers/server.rs` to
   `src/query/service/server-api/src/lib.rs`

Result:

- The `metrics-server` extraction gives a small hot compile win for metrics-only
  implementation edits.
- The `server-api` extraction does not materially improve hot compile time for
  interface edits, because changing that crate still rebuilds downstream service
  crates and `databend-query`.

## Environment

- Date: 2026-04-09
- Baseline commit: `87c0131e1b`
- Workspace under test: `/home/kould/RustroverProjects/databend`
- Baseline tree: temporary detached `git worktree` at `HEAD`

## Benchmark Target

- Command: `cargo check -p databend-query`

### Scenario A: Metrics Implementation Edit

- Baseline touched file:
  `src/query/service/src/servers/metrics/metric_service.rs`
- Extracted touched file:
  `src/query/service/metrics-server/src/lib.rs`

### Scenario B: Server Interface Edit

- Baseline touched file:
  `src/query/service/src/servers/server.rs`
- Extracted touched file:
  `src/query/service/server-api/src/lib.rs`

## Results

### Scenario A: Metrics Implementation Edit

| Variant | warm | hot1 | hot2 | hot3 |
| --- | ---: | ---: | ---: | ---: |
| Baseline | 371.56s | 7.55s | 6.29s | 5.99s |
| Extracted `metrics-server` | 356.48s | 6.11s | 5.16s | 5.65s |

### Scenario B: Server Interface Edit

The interface scenario reused the warmed target directories above and measured
hot rebuild time only.

| Variant | hot1 | hot2 | hot3 |
| --- | ---: | ---: | ---: |
| Baseline `server.rs` | 5.60s | 5.54s | 5.60s |
| Extracted `server-api` | 5.66s | 5.18s | 5.84s |

## Notes

- For Scenario A, the baseline hot edit rebuilt `databend-query`.
- For Scenario A, the extracted hot edit rebuilt both
  `databend-query-metrics-server` and `databend-query`.
- Even with the extra crate rebuild, Scenario A was still modestly faster in
  the measured runs, likely because the changed implementation now sits behind a
  smaller crate boundary.
- For Scenario B, the baseline hot edit rebuilt `databend-query`.
- For Scenario B, the extracted hot edit rebuilt:
  `databend-query-server-api`, `databend-query-metrics-server`, and
  `databend-query`.
- That means `server-api` is not an effective hot-compile boundary by itself for
  interface-level edits.
