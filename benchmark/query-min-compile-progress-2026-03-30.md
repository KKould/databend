# Databend Query 最小开发编译进展与新 Gate 收益说明

日期：2026-03-30

这份文档延续前两份基准记录：

- [query-min-compile-hotspots-2026-03-29.md](/home/kould/RustroverProjects/databend/benchmark/query-min-compile-hotspots-2026-03-29.md)
- [query-min-compile-progress-2026-03-29.md](/home/kould/RustroverProjects/databend/benchmark/query-min-compile-progress-2026-03-29.md)

本轮重点回答 4 个问题：

- 这次新增了哪些 feature gate
- 这些 gate 具体把什么从最小开发图里摘掉了
- 当前最新正式基准是多少
- 相比上一轮，这些 gate 到底带来了什么提升，哪些地方又还没有体现在冷编译时间里

## 统计范围

- 目标 crate：`databend-query`
- 编译类型：`cargo check`
- 最小开发 feature 组合：`--no-default-features --features simd`
- 本轮新增 gate 聚焦：
  - `sql-script`
  - `storage-stream`
  - `fuse-management`

## 本轮正式基准命令

同口径正式基准：

```bash
ionice -c3 nice -n 15 /usr/bin/time -f 'elapsed=%e\nmax_rss_kb=%M' \
  -o target/bench-logs/query-lib-min-stream-script-fuse-gates-j4.time \
  cargo check -j 4 -p databend-query --lib --no-default-features --features simd \
  --target-dir target/bench-q-lib-min-stream-script-fuse-gates-j4 --quiet
```

安全口径冷编译基准：

```bash
ionice -c3 nice -n 15 /usr/bin/time -f 'elapsed=%e\nmax_rss_kb=%M' \
  -o target/bench-logs/query-lib-min-stream-script-fuse-gates.time \
  cargo check -j 2 -p databend-query --lib --no-default-features --features simd \
  --target-dir target/bench-q-lib-min-stream-script-fuse-gates --quiet
```

同 target-dir 热复跑：

```bash
ionice -c3 nice -n 15 /usr/bin/time -f 'elapsed=%e\nmax_rss_kb=%M' \
  -o target/bench-logs/query-lib-min-stream-script-fuse-gates-rerun.time \
  cargo check -j 2 -p databend-query --lib --no-default-features --features simd \
  --target-dir target/bench-q-lib-min-stream-script-fuse-gates --quiet
```

说明：

- 这份文档现在把 `-j 4` 的同口径基准作为正式对比口径
- `-j 2` 的结果只保留为“当前更保守、更安全的机器使用口径”
- 这样可以同时回答两个问题：
  - 和上一轮相比，编译到底有没有提升
  - 在最近机器不稳定的情况下，安全模式下实际要花多久

## 当前正式基准

本轮新增 gate 之后的同口径正式结果：

- 墙钟时间：`423.33s`
- 峰值 RSS：`2499436 KB`

原始产物：

- `target/bench-logs/query-lib-min-stream-script-fuse-gates-j4.time`

本轮新增 gate 之后的安全口径冷编译结果：

- 墙钟时间：`479.85s`
- 峰值 RSS：`2550620 KB`

原始产物：

- `target/bench-logs/query-lib-min-stream-script-fuse-gates.time`

同 target-dir 热复跑结果：

- 墙钟时间：`1.20s`
- 峰值 RSS：`231408 KB`

原始产物：

- `target/bench-logs/query-lib-min-stream-script-fuse-gates-rerun.time`

另外，本轮代码修改后也已经成功跑通一次最小开发编译：

```bash
ionice -c3 nice -n 15 cargo check -p databend-query --lib --no-default-features --features simd \
  -j 2 --target-dir target/dev-query-min-gates-20260330
```

这条命令已经成功结束，说明当前最小开发路径在功能上是可编译闭环的。

## 和上一轮怎么对比

| 阶段 | 命令口径 | 墙钟时间 | 峰值 RSS | 备注 |
| --- | --- | ---: | ---: | --- |
| 热点快照基线 | `-j 8` | `423.30s` | `2670904 KB` | 来自 2026-03-29 热点文档 |
| 裁掉 `stage/lance/protobuf` 后 | 历史 bench | `379.06s` | `2606712 KB` | `target/bench-logs/query-lib-min-stagecut.time` |
| 上一轮正式基准 | `-j 4` | `396.09s` | `2626716 KB` | `target/bench-logs/query-lib-min-virtual-column.time` |
| 本轮新增 gate 后 | `-j 4` | `423.33s` | `2499436 KB` | 当前同口径正式对比 |
| 本轮安全口径 | `-j 2` | `479.85s` | `2550620 KB` | 当前保守机器配置 |
| 本轮热复跑 | `-j 2` | `1.20s` | `231408 KB` | 同 target-dir 增量复跑 |

如何理解这张表：

- 真正可比较的是两条 `-j 4` 数据：
  - 上一轮：`396.09s`
  - 本轮：`423.33s`
- 也就是说，按同口径看，本轮冷编译墙钟暂时没有变快，反而慢了 `27.24s`，约 `6.88%`
- 但峰值 RSS 从 `2626716 KB` 降到 `2499436 KB`，减少了 `127280 KB`，约 `4.85%`
- 所以这轮 gate 的收益更多体现为依赖图纯度、内存占用和热复跑，而不是当前这次冷编译墙钟

## 这轮新增 Gate 是什么

### 1. `sql-script`

入口：

- [src/query/service/Cargo.toml](/home/kould/RustroverProjects/databend/src/query/service/Cargo.toml)
- [src/query/ee/Cargo.toml](/home/kould/RustroverProjects/databend/src/query/ee/Cargo.toml)
- [src/binaries/Cargo.toml](/home/kould/RustroverProjects/databend/src/binaries/Cargo.toml)

主要代码点：

- [src/query/service/src/interpreters/mod.rs](/home/kould/RustroverProjects/databend/src/query/service/src/interpreters/mod.rs)
- [src/query/service/src/interpreters/interpreter_factory.rs](/home/kould/RustroverProjects/databend/src/query/service/src/interpreters/interpreter_factory.rs)
- [src/query/service/src/interpreters/util.rs](/home/kould/RustroverProjects/databend/src/query/service/src/interpreters/util.rs#L17)

意义：

- 把 `EXECUTE IMMEDIATE`、`CALL PROCEDURE` 以及 `ScriptClient` 运行路径从最小开发图里按需摘开
- feature 关闭时，相关 plan 在 interpreter factory 里降级为明确的 `Unimplemented`
- 最关键的是，把 `databend-common-script` 从最小图里真正拿掉，而不是只在语义层面“看起来没走到”

### 2. `storage-stream`

入口：

- [src/query/service/Cargo.toml](/home/kould/RustroverProjects/databend/src/query/service/Cargo.toml)
- [src/query/ee/Cargo.toml](/home/kould/RustroverProjects/databend/src/query/ee/Cargo.toml)
- [src/binaries/Cargo.toml](/home/kould/RustroverProjects/databend/src/binaries/Cargo.toml)
- [src/query/storages/factory/Cargo.toml](/home/kould/RustroverProjects/databend/src/query/storages/factory/Cargo.toml)
- [src/query/storages/system/Cargo.toml](/home/kould/RustroverProjects/databend/src/query/storages/system/Cargo.toml)

主要代码点：

- [src/query/service/src/interpreters/common/mod.rs](/home/kould/RustroverProjects/databend/src/query/service/src/interpreters/common/mod.rs)
- [src/query/service/src/interpreters/common/stream_stub.rs](/home/kould/RustroverProjects/databend/src/query/service/src/interpreters/common/stream_stub.rs#L24)
- [src/query/service/src/servers/http/v1/catalog/mod.rs](/home/kould/RustroverProjects/databend/src/query/service/src/servers/http/v1/catalog/mod.rs#L15)
- [src/query/service/src/servers/http/v1/http_query_handlers.rs](/home/kould/RustroverProjects/databend/src/query/service/src/servers/http/v1/http_query_handlers.rs)
- [src/query/service/src/servers/admin/v1/mod.rs](/home/kould/RustroverProjects/databend/src/query/service/src/servers/admin/v1/mod.rs)
- [src/query/service/src/servers/admin/admin_service.rs](/home/kould/RustroverProjects/databend/src/query/service/src/servers/admin/admin_service.rs#L86)
- [src/query/service/src/table_functions/table_function_factory.rs](/home/kould/RustroverProjects/databend/src/query/service/src/table_functions/table_function_factory.rs#L287)

意义：

- 把 stream 的 interpreter、system table、HTTP catalog 路由、admin route、`stream_status` table function、EE stream handler 初始化统一收成显式 gate
- 对最小 SQL 开发路径来说，stream 并不是必需能力
- 这轮除了关模块，还把一批只是为了 `STREAM_ENGINE` 常量而保留的 `storages-stream` 依赖替换成本地常量，避免出现“类型不引用了但 crate 还在图里”的情况

### 3. `fuse-management`

入口：

- [src/query/service/Cargo.toml](/home/kould/RustroverProjects/databend/src/query/service/Cargo.toml)
- [src/query/ee/Cargo.toml](/home/kould/RustroverProjects/databend/src/query/ee/Cargo.toml)
- [src/binaries/Cargo.toml](/home/kould/RustroverProjects/databend/src/binaries/Cargo.toml)

主要代码点：

- [src/query/service/src/interpreters/mod.rs](/home/kould/RustroverProjects/databend/src/query/service/src/interpreters/mod.rs)
- [src/query/service/src/interpreters/interpreter_factory.rs](/home/kould/RustroverProjects/databend/src/query/service/src/interpreters/interpreter_factory.rs)
- [src/query/service/src/interpreters/hook/hook.rs](/home/kould/RustroverProjects/databend/src/query/service/src/interpreters/hook/hook.rs#L46)
- [src/query/service/src/table_functions/mod.rs](/home/kould/RustroverProjects/databend/src/query/service/src/table_functions/mod.rs#L15)
- [src/query/service/src/table_functions/table_function_factory.rs](/home/kould/RustroverProjects/databend/src/query/service/src/table_functions/table_function_factory.rs#L162)

意义：

- 这不是去掉 `fuse` 核心，而是只切 `fuse` 上层的维护/诊断能力
- 具体包括 `Analyze/Recluster/Vacuum/Refresh*` interpreter 路径，以及 `fuse_snapshot`、`fuse_segment`、`fuse_vacuum2`、`clustering_information`、`fuse_encoding`、`fuse_time_travel_size` 等 table function 注册
- 这条路线符合“最小编译仍然能跑基本 SQL”的约束，不会把 `fuse` 这个核心表引擎整体拆掉

## 这轮最重要的依赖图变化

下面两条结论已经重新通过 `cargo tree -i` 确认：

```bash
ionice -c3 nice -n 15 cargo tree -p databend-query --no-default-features --features simd \
  -e normal,build -i databend-common-storages-stream
```

结果：

- `error: package ID specification 'databend-common-storages-stream' did not match any packages`

```bash
ionice -c3 nice -n 15 cargo tree -p databend-query --no-default-features --features simd \
  -e normal,build -i databend-common-script
```

结果：

- `error: package ID specification 'databend-common-script' did not match any packages`

这说明：

1. `storage-stream` 这条依赖链已经不在最小开发图里
2. `sql-script` 这条依赖链也已经不在最小开发图里

从“改动有没有生效”这个角度看，这轮 gate 是明确有效的。

## 这轮到底带来了什么提升

### 1. 依赖图缩减比上一轮更扎实

上一轮虽然已经裁掉了 `virtual-column` 和 `script-udf`，但最小图里其实仍然残留：

- `databend-common-script`
- `databend-common-storages-stream`

这也是为什么当时体感收益没有预期那么大。

本轮把这两条链真正摘掉之后，至少可以确认两件事：

- “只切 `virtual-column` / `script-udf`”还不够
- `stream` 和 SQL script 执行路径确实是最小开发图里的实打实残留项

### 2. 峰值内存有明确下降

和上一轮正式基准相比：

- 上一轮：`2626716 KB`
- 本轮同口径：`2499436 KB`
- 变化：`-127280 KB`，约 `-4.85%`

这说明即使冷编译墙钟没有体现出正向收益，依赖图缩小已经反映到峰值内存上了。

### 3. 热复跑明显更轻

上一轮文档里记录的热路径数据大约是：

- 增量复跑约 `7.08s`

本轮在同 target-dir 上的热复跑结果是：

- `1.20s`

这条数据不能被理解成严格的 A/B，因为 target-dir、机器状态、并发设置都不是完全同口径，但它至少说明：

- 这轮新的 gate 没有伤害最小开发内循环
- 相反，在 target-dir 已经热起来以后，当前最小图的反馈是非常快的

### 4. `fuse` 切法更符合“基本 SQL 可跑通”的目标

这轮最大的方向性进步不是数字，而是边界更合理：

- 没有把 `fuse` 核心能力粗暴从最小开发路径里摘掉
- 只把维护/诊断/刷新类能力收到了 `fuse-management`

这证明了一点：

- `fuse` 本身未必需要 gate
- 但 `fuse` 上层的外围能力非常适合 gate

这个方向比“把核心存储能力整体拆空”更可持续。

## 为什么冷编译时间没有直接变好

如果只看同口径 `-j 4` 的正式冷编译，这轮确实还没有变快。

更深一层的原因也很清楚：

1. 当前冷编译成本仍然很大程度受 workspace 级大依赖影响
   - TLS / OpenSSL
   - meta / telemetry / http 链路
   - `databend-query` 根 crate 本体
2. 本轮新 gate 主要缩的是“最小图外围功能”
   - 这会明显改善依赖图纯度和热复跑反馈
   - 但不一定立刻压过冷编译里的所有原生依赖和 query 根 crate 成本
3. 机器状态波动仍然很大
   - 最近几轮已经反复出现 `cargo check` 把编辑器或整机拖死的情况
   - 这会影响我们对冷编译时间的稳定观测

## 当前结论

这一轮新增的 `sql-script`、`storage-stream`、`fuse-management` 三个 gate，带来的提升主要体现在：

1. 最小开发依赖图更干净了
   - `databend-common-storages-stream` 已经不在图里
   - `databend-common-script` 也已经不在图里
2. 最小开发路径仍然保持“基本 SQL 可跑通”
   - 没有把 `fuse` 核心直接拆掉
   - 只切了不必要的外围管理能力
3. 热复跑反馈已经非常轻
   - 当前同 target-dir 热复跑只要 `1.20s`
4. 同口径冷编译正式基准暂时没有变快
   - 当前 `-j 4` 口径下是 `423.33s`
   - 相比上一轮 `396.09s` 慢了 `27.24s`
   - 因此目前不能把这轮 gate 说成“冷编译时间优化成功”

更直接地说：

- 从“依赖图是否真的缩了”看，这轮是有效的
- 从“热开发内循环是否更轻了”看，这轮也是有效的
- 从“冷编译墙钟是否更好看”看，目前同口径结论是没有变好
- 所以这轮 gate 的价值应当表述为“裁干净了最小图、压低了内存、改善了热路径”，而不是“已经缩短了冷编译时间”

## 下一步更值得做什么

在当前基础上，后续如果继续优化，优先级建议是：

1. 补一条和上一轮完全同口径的冷编译基准
   - 例如在机器状态允许时再跑一次 `-j 4`
   - 这样才能更公平地比较这轮 gate 的真实墙钟收益
2. 继续看 `databend-query` root crate 本体
   - 当前最小图里，真正剩下的主要热点还是 query 自身
3. 再看 workspace 级重依赖
   - 尤其是 TLS / OpenSSL / http / telemetry / meta 侧冷编译成本
