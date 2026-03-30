# Benchmark Directory

This directory contains subdirectories dedicated to various performance tests,

specifically for TPCH tests, Hits tests, and internal query performance tests. Below is a brief overview of each subdirectory:

编译耗时分析文档：

- [`query-min-compile-hotspots-2026-03-29.md`](/home/kould/RustroverProjects/databend/benchmark/query-min-compile-hotspots-2026-03-29.md) 记录了最小 `databend-query` 开发编译配置下的 crate 级 timing 热点快照。
- [`query-min-compile-progress-2026-03-29.md`](/home/kould/RustroverProjects/databend/benchmark/query-min-compile-progress-2026-03-29.md) 记录了当前最小开发编译的正式基准，以及这一轮 feature gate 改动和各自的意义。
- [`query-distributed-split-benchmark-2026-03-30.md`](/home/kould/RustroverProjects/databend/benchmark/query-distributed-split-benchmark-2026-03-30.md) 记录了 `distributed` 做成真实 crate 之后，在同口径 `-j 4` 冷编译下的对齐基准结果。
- [`query-crate-split-experiments-2026-03-30.md`](/home/kould/RustroverProjects/databend/benchmark/query-crate-split-experiments-2026-03-30.md) 汇总记录了当前“逐个拆、逐个测、按收益决定是否保留”的真 crate 拆分实验方法和阶段性结果。
- [`query-crate-split-workspaces-2026-03-30.md`](/home/kould/RustroverProjects/databend/benchmark/query-crate-split-workspaces-2026-03-30.md) 说明了如何把多个拆分方向放进独立 workspaces，并从同一份当前实验基线出发进行串行观测。

## 1. tpch

This subdirectory includes performance evaluation tools and scripts related to TPCH tests.

TPCH tests are designed to simulate complex query scenarios to assess the system's performance when handling large datasets. In this directory, you can find testing scripts, configuration files, and documentation for test results.

## 2. hits

Hits tests focus on specific queries or operations for performance testing.

In this subdirectory, you'll find scripts for Hits tests, sample queries, and performance analysis tools.

## 3. internal

The internal subdirectory contains testing tools and scripts dedicated to ensuring the performance of internal queries.

These tests may be conducted to ensure the system performs well when handling internal queries specific.
