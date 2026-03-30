# Databend Query 最小编译 Gate 改动成本盘点

日期：2026-03-30

这份文档不是继续讨论“理论上 gate 是否优雅”，而是从更务实的角度回答一个问题：

- 当前为了最小开发编译，我们到底为每个 gate 改了多少文件
- 这些 gate 目前拿到了什么收益
- 哪些 gate 属于“改动面很大但收益不明显”，值得优先回退或收缩

## 统计口径

- 统计范围只看当前 branch 里与最小编译路线直接相关的 gate 改动
- 统计对象只包含和 gate 直接相关的文件，不包含文档文件
- 同一个文件如果同时服务多个 gate，会在多个 gate 中重复计数
- 文件数是“当前改动面”的人工归类统计，目的是做工程取舍，不是做精确审计

## 当前纳入评估的 Gate

- `storage-stage`
- `script-udf`
- `cloud-control`
- `virtual-column`
- `sql-script`
- `storage-stream`
- `fuse-management`

## 一览表

| Gate | Manifest 文件数 | 代码文件数 | 新增 stub 文件数 | 合计文件数 | 当前收益判断 | 是否建议回退 |
| --- | ---: | ---: | ---: | ---: | --- | --- |
| `storage-stream` | 5 | 32 | 1 | 38 | 依赖图收益明确，冷编译收益未证实 | 优先考虑回退或大幅收缩 |
| `cloud-control` | 4 | 16 | 0 | 20 | 依赖图收益明确，领域边界清晰 | 不建议回退 |
| `storage-stage` | 3 | 16 | 0 | 19 | 已有明确冷编译收益 | 不建议回退 |
| `virtual-column` | 4 | 9 | 0 | 13 | 依赖图收益明确，但时间收益偏小 | 可回退，但不是首选 |
| `fuse-management` | 3 | 7 | 0 | 10 | 功能边界收益有，但冷编译收益不明显 | 第二优先回退候选 |
| `script-udf` | 3 | 2 | 2 | 7 | 改动很小，依赖图收益明确 | 不建议回退 |
| `sql-script` | 3 | 3 | 0 | 6 | 改动小，依赖图收益明确 | 不建议回退 |

## 每个 Gate 的收益和回退建议

### 1. `storage-stage`

改动面：

- manifest：3 个
- 代码：16 个左右
- 主要分布在 `service` interpreter / session / HTTP / table function / stage 入口

已经确认的收益：

- 这是当前已知收益最明确的一刀
- 历史正式基准里，裁掉 `stage/lance/protobuf` 后，最小冷编译从 `423.30s` 降到 `379.06s`
- `protobuf-src` 已不在最小依赖图里

判断：

- 虽然改动文件不少，但它是目前少数“已经拿到明确冷编译收益”的 gate
- 这条不应该回退

结论：

- 不建议回退

### 2. `script-udf`

改动面：

- manifest：3 个
- 代码：2 个模块导出文件
- stub：2 个

已经确认的收益：

- `arrow-udf-runtime` 已经不在最小图里
- 改动面很小，边界也清晰

判断：

- 这是低成本 gate
- 即使收益不是最大，也不值得为减少 7 个文件把它退掉

结论：

- 不建议回退

### 3. `cloud-control`

改动面：

- manifest：4 个
- 代码：16 个左右
- 主要落在：
  - `global_services`
  - `system_database`
  - `interpreter_factory`
  - task interpreters
  - system tables
  - cloud task table functions

已经确认的收益：

- `databend-common-cloud-control` 已经不在最小图里
- 这条改动虽然大，但领域边界天然清楚

判断：

- 它的改动面不小，但这些文件本来就属于 cloud task / worker / notification 这一类“控制面能力”
- 这条 gate 从产品边界上是自洽的
- 即使冷编译收益没有单独拆出来，它也不像 `storage-stream` 那样需要大量常量替换和兼容逻辑

结论：

- 不建议回退

### 4. `virtual-column`

改动面：

- manifest：4 个
- 代码：9 个左右
- 分布在：
  - `service` interpreters
  - `system_database`
  - `storages/system`
  - `table_function_factory`
  - `ee` 初始化和 fuse operation 导出

已经确认的收益：

- `databend-enterprise-virtual-column` 已经不在最小图里
- 但从我们现有基准看，这条单独拿出来的时间收益不大

判断：

- 这条 gate 的边界是干净的
- 改动面中等，维护成本也不算离谱
- 但如果后面真要“减代码优先于减依赖图”，它可以作为备选回退项

结论：

- 可回退，但优先级不高

### 5. `sql-script`

改动面：

- manifest：3 个
- 代码：3 个
- 主要就是：
  - `Cargo.toml`
  - `interpreter_factory`
  - `interpreter mod`
  - `util.rs` 中的 `ScriptClient`

已经确认的收益：

- `databend-common-script` 已经不在最小图里
- 改动面非常小

判断：

- 这是高性价比 gate
- 它几乎没有把代码复杂度扩散到大面积业务文件

结论：

- 不建议回退

### 6. `storage-stream`

改动面：

- manifest：5 个
- 代码：32 个
- stub：1 个
- 是当前所有 gate 里改动面最大的

主要分布：

- `service` interpreters 大量 `STREAM_ENGINE` 常量替换
- `query_ctx` / `query_ctx_shared`
- `system_database`
- `table_function_factory`
- HTTP / Admin route 注册
- `storages/factory`
- `storages/system`
- `ee` 初始化

已经确认的收益：

- `databend-common-storages-stream` 已经不在最小图里

但目前没有确认到的收益：

- 还没有看到冷编译墙钟的直接改善
- 同口径基准下，本轮总墙钟反而比上一轮慢
- 当然，这不一定是 `storage-stream` 自己导致的，但至少说明“这条大改动还没有转化成可见时间收益”

判断：

- 这条 gate 最大的问题不是理念，而是工程成本太高
- 很多改动并不是简单的 `cfg`，而是：
  - 大量 service 文件里的常量依赖切断
  - 路由和 table function 注册兼容
  - stream stub 和上下文兼容
- 换句话说，这条 gate 为了把一个边缘能力彻底摘出，付出了非常大的代码面

结论：

- 如果目标是“先减复杂度，再慢慢找真正的大热点”，`storage-stream` 是最应该优先回退或收缩的一条
- 更保守的替代方案是：
  - 不完全回退
  - 但把大量常量替换式兼容先撤回，只保留 manifest 层和少数核心入口 gate

### 7. `fuse-management`

改动面：

- manifest：3 个
- 代码：7 个
- 主要集中在：
  - `interpreter_factory`
  - `interpreter mod`
  - `hook`
  - `table_functions`

已经确认的收益：

- 语义上把 `Analyze/Recluster/Vacuum/Refresh*` 和一批 `fuse_*` 诊断型函数从最小路径剥离了

但目前没有确认到的收益：

- 它没有把 `databend-common-storages-fuse` 本体从最小图里移出去
- 因此对冷编译墙钟的帮助目前看不明显

判断：

- 这条 gate 的代码面不如 `storage-stream` 夸张
- 但收益也更偏“边界合理”，不是“编译立刻变快”
- 如果我们现在要减分支复杂度，它比 `virtual-column` 更值得先回退

结论：

- 是第二优先的回退候选

## 按“高成本低收益”排序的回退建议

如果当前目标是先收缩代码复杂度，而不是继续扩大 gate 面，优先级建议如下：

| 优先级 | Gate | 原因 |
| --- | --- | --- |
| 1 | `storage-stream` | 改动面最大，涉及 38 个文件，但目前只确认到依赖图收益，未确认冷编译收益 |
| 2 | `fuse-management` | 不切 `fuse` 核心，只切外围维护能力，代码分支增加了，但冷编译收益不明显 |
| 3 | `virtual-column` | 收益偏小，但边界较清晰，所以回退优先级低于前两者 |
| 4 | `cloud-control` | 改动虽多，但领域边界清晰，且已确认从最小图中移出，不建议回退 |
| 5 | `storage-stage` | 收益最明确，不应回退 |
| 6 | `script-udf` | 改动很小，不应回退 |
| 7 | `sql-script` | 改动最小且收益明确，不应回退 |

## 如果只保留最划算的 Gate

如果目标是“尽量少改代码，但保住最确定的收益”，当前更合理的组合是：

- 保留：
  - `storage-stage`
  - `sql-script`
  - `script-udf`
  - `cloud-control`
- 视情况保留：
  - `virtual-column`
- 优先考虑回退或收缩：
  - `storage-stream`
  - `fuse-management`

## 当前更实际的结论

从“是否值得继续扩大 gate 面”这个角度看，当前最值得警惕的不是 `sql-script`，也不是 `script-udf`，而是：

1. `storage-stream`
   - 为了摘掉 stream，代码面扩散得太广
2. `fuse-management`
   - 虽然方向合理，但它并没有切掉 `fuse` 这个真正重的核心 crate

所以如果接下来要做减法，最合理的动作不是把所有 gate 都回掉，而是：

- 优先审视 `storage-stream`
- 其次审视 `fuse-management`
- 把“低成本、边界清晰、收益明确”的 gate 留下来

