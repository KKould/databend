# Databend Query 最小开发编译进展与 Feature Gate 说明

日期：2026-03-29

这份文档记录最近一轮 `databend-query` 最小开发编译路径的整理结果，重点回答 3 个问题：

- 当前最小开发编译的正式基准是多少
- 这轮到底增加了哪些 feature gate
- 这些 gate 分别切掉了什么能力，以及它们对最小 SQL 开发路径的意义

## 统计范围

- 目标 crate：`databend-query`
- 编译类型：`cargo check`
- 最小开发 feature 组合：`--no-default-features --features simd`
- 本轮正式基准命令：

```bash
ionice -c3 nice -n 15 /usr/bin/time -f 'elapsed=%e\nmax_rss_kb=%M' \
  -o target/bench-logs/query-lib-min-virtual-column.time \
  cargo check -j 4 -p databend-query --lib --no-default-features --features simd \
  --target-dir target/bench-q-lib-min-virtual-column --quiet
```

说明：

- 本轮正式基准使用 `-j 4`，不是之前文档里的 `-j 8`
- 原因不是策略变化，而是最近机器在较高并发 `cargo check` 下容易卡死，所以这里优先保守稳定
- 因此下面的墙钟时间更适合用来表示“当前安全配置下的实际开发成本”，不适合和旧的 `-j 8` 数据做严格一对一横比

## 当前正式基准

本轮正式基准结果：

- 墙钟时间：`396.09s`
- 峰值 RSS：`2626716 KB`

原始产物：

- `target/bench-logs/query-lib-min-virtual-column.time`

同一轮里还观测到一条热路径数据：

- 在已经热起来的 `target/dev-query-min` 上复跑
- `cargo check -j 4 -p databend-query --lib --no-default-features --features simd --target-dir target/dev-query-min`
- 增量复跑约 `7.08s`

这条 `7.08s` 不是正式冷编译基准，只能说明“最小路径热起来以后，内循环反馈会比较快”。

## 历史里程碑

| 阶段 | 命令口径 | 墙钟时间 | 峰值 RSS | 备注 |
| --- | --- | ---: | ---: | --- |
| 早期最小路径 | 旧记录 | `451.84s` | - | 来自上一轮总结，不是本文件重新测量 |
| 热点快照基线 | `-j 8` | `423.30s` | `2670904 KB` | 对应 [`query-min-compile-hotspots-2026-03-29.md`](/home/kould/RustroverProjects/databend/benchmark/query-min-compile-hotspots-2026-03-29.md) |
| 裁掉 `stage/lance/protobuf` 后 | 历史 bench | `379.06s` | `2606712 KB` | 原始文件：`target/bench-logs/query-lib-min-stagecut.time` |
| 本轮整理后 | `-j 4` | `396.09s` | `2626716 KB` | 当前安全配置下的正式基准 |

如何理解这张表：

- 从依赖图角度看，本轮又成功裁掉了 `virtual-column`
- 但墙钟时间受 `-j 4` 与 `-j 8` 差异影响，不能直接拿 `396.09s` 和 `379.06s` 做结论性比较
- 如果后面机器状态允许，再补一条同口径 `-j 8` 基准，才能得到更严格的 before/after 数字

## 当前最小图已确认被裁掉的重依赖

下面几条结论已经通过 `cargo tree -i` 确认：

- `protobuf-src` 已不在最小图里
- `databend-common-cloud-control` 已不在最小图里
- `databend-enterprise-virtual-column` 已不在最小图里

对应表现：

- `cargo tree -i protobuf-src` 在最小 feature 图下直接提示找不到包
- `cargo tree -i databend-common-cloud-control` 在最小 feature 图下显示 `nothing to print`
- `cargo tree -i databend-enterprise-virtual-column` 在最小 feature 图下直接提示找不到包

这说明当前的最小开发路径已经不再携带以下三类重成本：

1. `stage -> lance-* -> protobuf-src`
2. `cloud-control` 控制面依赖
3. `fuse` 上的 `virtual-column` 扩展能力

## 这轮增加了哪些 Feature Gate

这轮不是只加了一个 `virtual-column`，而是把最小开发路径需要的“外围能力”系统地改成了显式 gate。

总原则是：

- 默认构建行为不变
- 正常 `default` feature 仍然保留这些能力
- 只有最小开发路径 `--no-default-features --features simd` 才把它们关掉

### 1. `full-build-info`

入口：

- [src/query/service/Cargo.toml](/home/kould/RustroverProjects/databend/src/query/service/Cargo.toml#L10)
- [src/query/ee/Cargo.toml](/home/kould/RustroverProjects/databend/src/query/ee/Cargo.toml#L10)
- [src/binaries/Cargo.toml](/home/kould/RustroverProjects/databend/src/binaries/Cargo.toml#L10)

意义：

- 把完整构建元信息收集从最小开发路径中剥离
- 避免为了日常 planner / executor / SQL 修改而每次都走完整 build-info 链路

### 2. `script-udf`

入口：

- [src/query/service/Cargo.toml](/home/kould/RustroverProjects/databend/src/query/service/Cargo.toml#L32)
- [src/query/ee/Cargo.toml](/home/kould/RustroverProjects/databend/src/query/ee/Cargo.toml#L32)
- [src/binaries/Cargo.toml](/home/kould/RustroverProjects/databend/src/binaries/Cargo.toml#L36)

意义：

- 把脚本 UDF 运行时从最小开发路径中拆开
- 最小 SQL 开发默认不再为了脚本 UDF 引入额外编译负担

### 3. `cloud-control`

入口：

- [src/query/service/Cargo.toml](/home/kould/RustroverProjects/databend/src/query/service/Cargo.toml#L34)
- [src/query/storages/system/Cargo.toml](/home/kould/RustroverProjects/databend/src/query/storages/system/Cargo.toml#L9)
- [src/query/service/src/interpreters/interpreter_factory.rs](/home/kould/RustroverProjects/databend/src/query/service/src/interpreters/interpreter_factory.rs#L142)
- [src/query/service/src/databases/system/system_database.rs](/home/kould/RustroverProjects/databend/src/query/service/src/databases/system/system_database.rs#L192)
- [src/query/service/src/table_functions/table_function_factory.rs](/home/kould/RustroverProjects/databend/src/query/service/src/table_functions/table_function_factory.rs#L316)

意义：

- 把 worker / notification / cloud task dependents / system 中 cloud-only 表从最小路径中拿掉
- 这些能力不属于“本地基本 SQL 开发”必需内容
- feature 关闭时，不是静默缺功能，而是对相关操作返回明确的 `Unimplemented`

### 4. `storage-stage`

入口：

- [src/query/service/Cargo.toml](/home/kould/RustroverProjects/databend/src/query/service/Cargo.toml#L43)
- [src/query/service/src/interpreters/interpreter_factory.rs](/home/kould/RustroverProjects/databend/src/query/service/src/interpreters/interpreter_factory.rs#L148)
- [src/query/service/src/table_functions/table_function_factory.rs](/home/kould/RustroverProjects/databend/src/query/service/src/table_functions/table_function_factory.rs#L273)

意义：

- 把 `COPY INTO`、streaming load、`infer_schema` 等 stage 专属能力收成显式 gate
- 这是目前最有实际收益的一刀，因为它直接切掉了 `stage -> lance-* -> protobuf-src`
- 对最小 SQL 路线来说，最重要的意义是去掉本地 `protoc` / C++ 编译这条重依赖

### 5. `storage-delta`

入口：

- [src/query/service/Cargo.toml](/home/kould/RustroverProjects/databend/src/query/service/Cargo.toml#L50)
- [src/query/storages/factory/Cargo.toml](/home/kould/RustroverProjects/databend/src/query/storages/factory/Cargo.toml#L9)
- [src/query/service/src/sessions/query_ctx.rs](/home/kould/RustroverProjects/databend/src/query/service/src/sessions/query_ctx.rs#L2127)

意义：

- 让 `DELTA` 引擎从“默认一定参与编译”变成“按需显式开启”
- 关闭时 `DELTA` 相关入口返回 `Unimplemented`
- 保留基础 SQL 路线的同时，减少 data lake 扩展引擎对内循环的拖累

### 6. `storage-iceberg`

入口：

- [src/query/service/Cargo.toml](/home/kould/RustroverProjects/databend/src/query/service/Cargo.toml#L57)
- [src/query/storages/factory/Cargo.toml](/home/kould/RustroverProjects/databend/src/query/storages/factory/Cargo.toml#L10)
- [src/query/service/src/table_functions/table_function_factory.rs](/home/kould/RustroverProjects/databend/src/query/service/src/table_functions/table_function_factory.rs#L388)

意义：

- 让 `ICEBERG` 引擎和对应 table function 从最小路径里按需进入
- 这一类功能更适合做外围 gate，而不是让每次改 planner / executor 都必须为它付编译成本

### 7. `virtual-column`

入口：

- [src/query/service/Cargo.toml](/home/kould/RustroverProjects/databend/src/query/service/Cargo.toml#L39)
- [src/query/ee/Cargo.toml](/home/kould/RustroverProjects/databend/src/query/ee/Cargo.toml#L34)
- [src/binaries/Cargo.toml](/home/kould/RustroverProjects/databend/src/binaries/Cargo.toml#L38)
- [src/query/storages/system/Cargo.toml](/home/kould/RustroverProjects/databend/src/query/storages/system/Cargo.toml#L10)
- [src/query/service/src/interpreters/mod.rs](/home/kould/RustroverProjects/databend/src/query/service/src/interpreters/mod.rs#L206)
- [src/query/storages/system/src/lib.rs](/home/kould/RustroverProjects/databend/src/query/storages/system/src/lib.rs#L76)
- [src/query/ee/src/lib.rs](/home/kould/RustroverProjects/databend/src/query/ee/src/lib.rs#L34)

意义：

- 这是本轮新增的重点 gate
- 它切掉的是 `fuse` 上层的 `virtual column refresh/vacuum`、`system.virtual_columns`、`fuse_virtual_column` table function，以及 EE 侧的 handler / module 初始化
- 这条能力不属于“基本 SQL 可跑通”的必要条件，但之前会把一整条 EE handler 依赖边带进最小图
- 它证明了“保留 `fuse` 核心，只切 `fuse` 上层非核心功能”这条路线是可行的

### 8. 对象存储 backend gate

入口：

- [src/query/service/Cargo.toml](/home/kould/RustroverProjects/databend/src/query/service/Cargo.toml#L47)
- [src/query/ee/Cargo.toml](/home/kould/RustroverProjects/databend/src/query/ee/Cargo.toml#L39)
- [src/binaries/Cargo.toml](/home/kould/RustroverProjects/databend/src/binaries/Cargo.toml#L46)

包括：

- `storage-azblob`
- `storage-cos`
- `storage-gcs`
- `storage-hive`
- `storage-huggingface`
- `storage-ipfs`
- `storage-obs`
- `storage-oss`
- `storage-webhdfs`

意义：

- 这一组更多是在 manifest 层把外部 backend 能力显式化
- 主要价值不是单点大幅提速，而是让“最小开发路径”与“完整对象存储支持”之间的边界更清楚

## 这一轮代码层面到底改了什么

可以把这轮改动概括成 4 类：

### 1. 统一 feature 面

把 `databend-query`、`databend-enterprise-query`、`databend-binaries` 三层的 feature 定义对齐，使最小路径能从顶层一路传递到下游。

相关文件：

- [src/query/service/Cargo.toml](/home/kould/RustroverProjects/databend/src/query/service/Cargo.toml#L10)
- [src/query/ee/Cargo.toml](/home/kould/RustroverProjects/databend/src/query/ee/Cargo.toml#L10)
- [src/binaries/Cargo.toml](/home/kould/RustroverProjects/databend/src/binaries/Cargo.toml#L10)

### 2. 把“注册入口”改成条件注册

包括：

- interpreter module 导出
- system table 注册
- table function 注册
- EE module 导出和服务初始化

相关文件：

- [src/query/service/src/databases/system/system_database.rs](/home/kould/RustroverProjects/databend/src/query/service/src/databases/system/system_database.rs#L150)
- [src/query/service/src/table_functions/table_function_factory.rs](/home/kould/RustroverProjects/databend/src/query/service/src/table_functions/table_function_factory.rs#L217)
- [src/query/ee/src/enterprise_services.rs](/home/kould/RustroverProjects/databend/src/query/ee/src/enterprise_services.rs#L43)

### 3. feature 关闭时显式降级

不是简单把模块删掉，而是在入口处返回清晰错误，例如：

- `COPY INTO TABLE` 需要 `storage-stage`
- `DELTA` 需要 `storage-delta`
- `REFRESH VIRTUAL COLUMN` 需要 `virtual-column`

相关文件：

- [src/query/service/src/interpreters/interpreter_factory.rs](/home/kould/RustroverProjects/databend/src/query/service/src/interpreters/interpreter_factory.rs#L142)
- [src/query/service/src/sessions/query_ctx.rs](/home/kould/RustroverProjects/databend/src/query/service/src/sessions/query_ctx.rs#L2127)

意义：

- 最小路径不会因为引用不到某个模块而在编译期炸掉
- 用户侧也能清楚知道某条语义为什么不可用

### 4. 把 `fuse` 的外围扩展能力和核心读写链路分开看

本轮最重要的判断不是“把 `fuse` 摘掉”，而是：

- `fuse` 核心表引擎仍然是最小 SQL 路线的一部分
- 但 `virtual-column` 这种建立在 `fuse` 上的增强能力，完全可以独立 gate

这对后续继续裁：

- `stream`
- `vacuum / recluster / analyze / index refresh`
- 各类 `fuse_*` 中偏诊断或运维的函数

都有直接参考价值。

## 当前结论

到本轮为止，最小开发路径已经验证有效的方向是：

1. 切 `stage/lance/protobuf` 这类明确的重依赖链
2. 切 `cloud-control` 这类明显不属于本地 SQL 开发必需的控制面功能
3. 切 `fuse` 上层的非核心增强能力，例如 `virtual-column`

同时也已经确认：

- `fuse` 核心本体不适合在“基本 SQL 可跑通”的目标下整块摘掉
- 继续提速的正确方向是“保留核心读写，继续裁外围功能”

下一步如果继续沿这条线推进，优先级更高的候选仍然是：

1. `stream`
2. `vacuum / recluster / analyze / index refresh`
3. 偏运维、偏诊断的 `fuse_*` table functions
