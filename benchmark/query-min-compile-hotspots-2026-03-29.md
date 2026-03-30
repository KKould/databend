# Databend Query 最小开发编译热点快照

日期：2026-03-29

这份文档记录了当前 Databend `query` 最小开发编译配置下的编译热点分布，用来说明在最近一轮 storage / script-udf / build-info / cloud-control 裁剪之后，剩余时间主要耗在什么地方。

## 统计范围

- 目标 crate：`databend-query`
- 编译类型：`cargo check`
- Feature 组合：`--no-default-features --features simd`
- 目的：找出当前最小 `query` 开发编译里，仍然占主要时间的 crate 和依赖链

## 测量命令

冷编译基线：

```bash
ionice -c3 nice -n 15 cargo check -j 8 -p databend-query --lib --no-default-features --features simd --target-dir target/bench-q-lib-min-current --quiet
```

Timing 采集：

```bash
ionice -c3 nice -n 15 cargo check -j 6 -p databend-query --lib --no-default-features --features simd --target-dir target/timings-query-min-capture -Zunstable-options --timings=json --quiet
```

本次观测到的基线结果：

- 墙钟时间：`423.30s`
- 峰值 RSS：`2670904 KB`

原始产物：

- `target/bench-logs/query-lib-min-current.time`
- `target/bench-logs/query-timings-min.jsonl`

## 如何理解占比

下面的占比统一按 `crate_duration / 423.30s 总墙钟时间` 计算。

需要注意，Cargo timing 记录的是每个编译单元的耗时，很多单元在并行执行，所以这些百分比 **不会** 相加等于 100%。它们更适合用来表示“热点强度”，而不是严格意义上的独占墙钟占比。

## 最重的编译单元

| 排名 | Crate / 单元 | 耗时 | 对总墙钟占比 | 说明 |
| --- | --- | ---: | ---: | --- |
| 1 | `databend-query@0.1.0` | 264.14s | 62.40% | `query service` 根 crate 本体就是最大的热点 |
| 2 | `protobuf-src@2.1.1+27.1` | 149.31s | 35.27% | `protoc` 本地编译，来源是 `stage -> lance-*` |
| 3 | `openssl-sys@0.9.108` | 121.73s | 28.76% | vendored OpenSSL / native TLS 链路 |
| 4 | `databend-common-sql@0.1.0` | 34.25s | 8.09% | planner / binder / optimizer 核心 |
| 5 | `zstd-sys@2.0.15+zstd.1.5.7` | 12.33s | 2.91% | 原生压缩依赖 |
| 6 | `databend-common-storages-fuse@0.1.0` | 10.29s | 2.43% | 最小图里仍然存在 |
| 7 | `databend-common-ast@0.2.5` | 9.99s | 2.36% | SQL AST 及相邻解析层 |
| 8 | `zstd-sys@2.0.15+zstd.1.5.7` | 8.28s | 1.96% | 同一个原生 crate 的另一条 timing 记录 |
| 9 | `lance-bitpacking@1.0.4` | 8.22s | 1.94% | 来自 `stage / lance` 链路 |
| 10 | `ring@0.17.14` | 7.95s | 1.88% | TLS / 加密依赖 |
| 11 | `lz4-sys@1.11.1+lz4-1.10.0` | 7.90s | 1.87% | 原生压缩依赖 |
| 12 | `lzma-sys@0.1.20` | 7.43s | 1.75% | 原生压缩依赖 |
| 13 | `openraft@0.10.0-alpha.17` | 6.71s | 1.59% | 仍然可达的 meta / raft 侧依赖 |
| 14 | `databend-common-functions@0.1.0` | 6.56s | 1.55% | 标量 / 聚合函数层 |
| 15 | `databend-common-storages-system@0.1.0` | 6.02s | 1.42% | system table 和 task 相关逻辑 |
| 16 | `databend-common-expression@0.1.0` | 4.68s | 1.11% | expression 层 |
| 17 | `sqlparser@0.43.1` | 4.34s | 1.03% | 外部 parser crate |
| 18 | `sqlparser@0.58.0` | 3.89s | 0.92% | 依赖图里同时存在的第二个 parser 版本 |
| 19 | `databend-common-management@0.1.0` | 3.75s | 0.89% | management 层 |
| 20 | `databend-common-meta-api@0.1.0` | 3.16s | 0.75% | meta API 层 |

## 最重的本地 Databend Crate

这里只统计本仓库内的本地 crate。

| 排名 | 本地 crate | 耗时 | 对总墙钟占比 |
| --- | --- | ---: | ---: |
| 1 | `databend-query@0.1.0` | 264.38s | 62.46% |
| 2 | `databend-common-sql@0.1.0` | 34.25s | 8.09% |
| 3 | `databend-common-storages-fuse@0.1.0` | 10.29s | 2.43% |
| 4 | `databend-common-ast@0.2.5` | 9.99s | 2.36% |
| 5 | `databend-common-functions@0.1.0` | 6.56s | 1.55% |
| 6 | `databend-common-storages-system@0.1.0` | 6.02s | 1.42% |
| 7 | `databend-common-expression@0.1.0` | 4.68s | 1.11% |
| 8 | `databend-common-management@0.1.0` | 3.75s | 0.89% |
| 9 | `databend-common-meta-api@0.1.0` | 3.16s | 0.75% |
| 10 | `databend-storages-common-index@0.1.0` | 2.24s | 0.53% |
| 11 | `databend-common-meta-app@0.1.0` | 2.24s | 0.53% |
| 12 | `databend-common-protos@0.1.0` | 2.14s | 0.51% |
| 13 | `databend-storages-common-table-meta@0.1.0` | 1.88s | 0.44% |
| 14 | `databend-common-catalog@0.1.0` | 1.80s | 0.43% |
| 15 | `databend-common-config@0.1.0` | 1.78s | 0.42% |
| 16 | `databend-common-storages-parquet@0.1.0` | 1.65s | 0.39% |
| 17 | `databend-common-cloud-control@0.1.0` | 1.51s | 0.36% |
| 18 | `databend-common-pipeline-transforms@0.1.0` | 1.40s | 0.33% |
| 19 | `databend-functions-scalar-geo@0.1.0` | 1.38s | 0.33% |
| 20 | `databend-common-base@0.1.0` | 1.29s | 0.31% |

## 已确认的重依赖链

### 1. `stage -> lance-* -> protobuf-src`

这是当前最大的非 `query` 本体热点。

- `protobuf-src` 是通过 `databend-common-storages-stage` 被带进来的
- `databend-common-storages-stage` 依赖了 `lance-encoding`、`lance-file` 和 `lance-table`
- `lance-file` 和 `lance-table` 开启了 `features = ["protoc"]`

相关 manifest：

- [`src/query/storages/stage/Cargo.toml`](/home/kould/RustroverProjects/databend/src/query/storages/stage/Cargo.toml#L39)
- [`src/query/storages/stage/Cargo.toml`](/home/kould/RustroverProjects/databend/src/query/storages/stage/Cargo.toml#L43)

含义：

- 如果最小开发编译能避开 `stage`，或者避开需要 `protoc` 的 lance feature，收益会非常大。

### 2. `reqwest` / `poem` / vendored OpenSSL

这是当前另一个非常重的非 `query` 本体热点。

- workspace 里的 `reqwest` 使用了 `native-tls-vendored`
- workspace 里的 `poem` 使用了 `openssl-tls`
- `databend-common-base` 依赖 `reqwest`
- `databend-common-http` 依赖 `poem`

相关 manifest：

- [`Cargo.toml`](/home/kould/RustroverProjects/databend/Cargo.toml#L384)
- [`Cargo.toml`](/home/kould/RustroverProjects/databend/Cargo.toml#L404)
- [`src/common/base/Cargo.toml`](/home/kould/RustroverProjects/databend/src/common/base/Cargo.toml#L48)
- [`src/common/http/Cargo.toml`](/home/kould/RustroverProjects/databend/src/common/http/Cargo.toml#L19)

含义：

- 如果能提供 dev-only 的更轻 TLS 栈，冷编译成本大概率会明显下降，但这已经不是单纯的 query feature 开关问题，而是更广的 workspace 级权衡。

### 3. `cloud-control` is still partially reachable in minimal builds

即使已经补了 `cloud-control` feature gate，这个 crate 仍然会出现在最小依赖图里。

已确认原因：

- `databend-common-sql` 的默认 feature 仍然包含 `cloud-control`
- 有些下游 crate 还在使用 `databend-common-sql = { workspace = true }`，会把默认 feature 又重新带回来

相关 manifest：

- [`src/query/sql/Cargo.toml`](/home/kould/RustroverProjects/databend/src/query/sql/Cargo.toml#L9)
- [`src/query/pipeline/transforms/Cargo.toml`](/home/kould/RustroverProjects/databend/src/query/pipeline/transforms/Cargo.toml#L24)
- [`src/query/storages/system/Cargo.toml`](/home/kould/RustroverProjects/databend/src/query/storages/system/Cargo.toml#L31)
- [`src/query/service/Cargo.toml`](/home/kould/RustroverProjects/databend/src/query/service/Cargo.toml#L32)

含义：

- 从依赖图角度看，当前这轮 cloud-control 裁剪还没有真正收干净。

### 4. `fuse` is still in the minimal development graph

`databend-common-storages-fuse` 仍然是最小开发图里较重的本地 crate 之一。

已确认原因：

- `databend-query` 仍然直接依赖 `databend-common-storages-fuse`
- `databend-common-storages-factory` 仍然依赖它
- `databend-common-storages-system` 仍然依赖它
- query service 里多处 physical plan 文件直接引用了 `FuseTable` 以及 fuse 相关操作

相关 manifest 和源码：

- [`src/query/service/Cargo.toml`](/home/kould/RustroverProjects/databend/src/query/service/Cargo.toml#L125)
- [`src/query/storages/factory/Cargo.toml`](/home/kould/RustroverProjects/databend/src/query/storages/factory/Cargo.toml#L21)
- [`src/query/storages/system/Cargo.toml`](/home/kould/RustroverProjects/databend/src/query/storages/system/Cargo.toml#L34)
- [`src/query/service/src/physical_plans/physical_mutation.rs`](/home/kould/RustroverProjects/databend/src/query/service/src/physical_plans/physical_mutation.rs)

含义：

- 如果“基础 query 开发”并不需要 fuse 的 mutation / commit / compact 路径，那么 `fuse` 很可能需要单独划出一层面向开发编译的 feature 边界。

## 当前结论

到目前为止，剩余最大的编译成本主要集中在：

1. `databend-query` root crate size
2. `protobuf-src` via `stage/lance/protoc`
3. vendored OpenSSL / native TLS stack
4. incomplete trimming of `cloud-control`
5. `fuse` still being part of the minimal graph

这也解释了为什么前一轮 feature gate 虽然有效，但最小编译时间只是从 `451.84s` 降到 `423.30s`：被移除的 feature 确实不再参与编译了，但它们已经不是剩余依赖图里的主要矛盾。
