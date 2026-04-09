# Query Hot Compile Split Summary

## Goal

This note summarizes the crate-splitting experiments that were run to improve
hot compile time for `cargo check -p databend-query` and
`cargo check -p databend-common-sql`.

The key question was not whether the code could be moved into more crates, but
whether edits in the moved code would stop forcing an expensive downstream
rebuild of the main crate under development.

## Results

| Experiment | Scenario | Baseline avg | Extracted avg | Delta | Verdict |
| --- | --- | ---: | ---: | ---: | --- |
| `sql-plans` | edit moved SQL plan module | 2.19s | 2.61s | +0.42s | no gain |
| `name-resolution` | edit moved SQL name resolution module | 2.11s | 2.44s | +0.33s | no gain |
| `service server split` | edit moved metrics implementation | 6.61s | 5.64s | -0.97s | keep |
| `server-api` | edit moved server interface | 5.58s | 5.56s | -0.02s | neutral |
| `federated` | edit moved MySQL federated compatibility module | 5.39s | 5.45s | +0.06s | no gain |
| `admin-server` | edit moved admin hosting module | 6.14s | 5.60s | -0.54s | keep |
| `http-handler-server` | edit moved HTTP hosting module | 5.67s | 6.20s | +0.53s | no gain |
| `mysql-server` | edit moved MySQL hosting module | 5.92s | 5.61s | -0.31s | keep |

## What Helped

- Moving leaf-ish service hosting code out of `databend-query` gave small but
  repeatable gains in a few cases.
- The clearest wins came from:
  - `metrics-server`
  - `admin-server`
  - `mysql-server`

These wins are still modest. In practice they are mostly in the
`0.3s` to `1.0s` range per hot compile for the specific edited file under test.

## What Did Not Help

- Pure API extraction by itself did not help much.
- Moving utility or compatibility logic without cutting downstream rebuilds did
  not help.
- SQL-side extractions did not improve the tested hot compile path because
  changes still rebuilt the downstream SQL crate.

The repeated pattern was:

1. the moved crate rebuilt
2. `databend-query` or `databend-common-sql` still rebuilt
3. total hot compile time stayed flat or got slightly worse

## Recommendation

- Keep the splits that showed measurable wins:
  - `metrics-server`
  - `admin-server`
  - `mysql-server`
- Treat the others as structural cleanups, not compile-speed wins.
- Avoid continuing to add many more small crates unless a candidate is both:
  - frequently edited
  - able to reduce rebuild work in the downstream hot crate

At this point, crate count is no longer the main problem. The main bottleneck is
still whether edits in the extracted code can avoid recompiling
`databend-query` itself.
