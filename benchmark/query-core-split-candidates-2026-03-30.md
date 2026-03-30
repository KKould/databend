# Databend Query 主体拆分候选清单

日期：2026-03-30

这份文档的目的不是继续讨论 `storage-stage`、`storage-stream` 之类的 feature gate，而是把视角切回 `databend-query` 主体本身：

- `src/query/service`
- `src/query/sql`

前提是：

- `stage -> lance -> protobuf-src` 这条重链已经被单独识别出来
- `storage-stage-lance` 已经从普通 `storage-stage` 路线中拆开
- 接下来要找的是：**即使把外围 storage 问题先放下，query 主体为什么还是这么重**

## 1. 范围与口径

本次只做静态体量和结构扫描，不重新跑更重的 profiling。

使用的数据包括：

- 现有最小 query 编译热点文档
- `query/service` 与 `query/sql` 的目录结构
- 各顶层模块的 Rust 文件数量
- 各顶层模块的代码行数
- 仓库内对若干核心模块的直接引用次数

说明：

- 这里的“行数”不是编译时间本身，但可以帮助识别“大块、边界差、容易传播重编译”的区域
- “引用次数”只是粗粒度信号，用来估计耦合度，不是严格依赖图

## 2. 现状总览

### 2.1 `query/service` 体量分布

`src/query/service/src` 总行数约 `156263`。

| 模块 | 行数 | 文件数 | 占 service 比例 |
| --- | ---: | ---: | ---: |
| `pipelines` | `41353` | `167` | `26.46%` |
| `interpreters` | `33052` | `211` | `21.15%` |
| `servers` | `25190` | `143` | `16.12%` |
| `physical_plans` | `20709` | `102` | `13.25%` |
| `table_functions` | `9736` | `49` | `6.23%` |
| `sessions` | `6658` | `13` | `4.26%` |

只看前四块：

- `pipelines`
- `interpreters`
- `servers`
- `physical_plans`

已经占掉 `query/service` 约 `76.98%` 的代码量。

### 2.2 `query/sql` 体量分布

`src/query/sql/src` 总行数约 `85615`。

其中几乎全部重量都在 `planner`。

`planner` 内部再拆分后：

| 模块 | 行数 | 文件数 | 占 sql 比例 |
| --- | ---: | ---: | ---: |
| `binder` | `29358` | `92` | `34.29%` |
| `optimizer` | `28706` | `149` | `33.53%` |
| `plans` | `11923` | `70` | `13.93%` |
| `semantic` | `10340` | `15` | `12.08%` |

也就是说：

- `binder + optimizer` 两块加起来已经占掉 `query/sql` 约 `67.82%`

这和之前的编译热点观察是一致的：

- `databend-query` 根 crate 本体很重
- `databend-common-sql` 也是主路径上的核心热点

## 3. 大文件热点

仅从大文件看，已经能看出几个很典型的“改一点可能牵一大片”的区域：

| 文件 | 行数 | 含义 |
| --- | ---: | --- |
| `src/query/sql/src/planner/semantic/type_check.rs` | `7745` | 语义检查与类型推导核心大文件 |
| `src/query/service/src/sessions/query_ctx.rs` | `2552` | query context 核心状态与能力汇总 |
| `src/query/sql/src/planner/binder/ddl/table.rs` | `2380` | DDL binder 大块逻辑 |
| `src/query/service/src/interpreters/access/privilege_access.rs` | `2347` | 访问控制与语义检查 |
| `src/query/sql/src/planner/plans/scalar_expr.rs` | `1635` | plan / expression 基础表示 |
| `src/query/service/src/physical_plans/physical_hash_join.rs` | `1599` | 执行层 hash join 大块逻辑 |
| `src/query/service/src/servers/flight/v1/exchange/exchange_manager.rs` | `1377` | flight/exchange 服务侧核心大文件 |
| `src/query/sql/src/planner/optimizer/optimizers/rule/agg_rules/rule_eager_aggregation.rs` | `1374` | optimizer 大规则文件 |
| `src/query/service/src/pipelines/processors/transforms/transform_async_function.rs` | `1232` | pipeline transform 大文件 |
| `src/query/service/src/servers/http/v1/http_query_handlers.rs` | `1116` | HTTP query handler 大文件 |

这些文件不一定都适合直接拆 crate，但它们很适合作为“哪里最容易传播编译成本”的观测点。

## 4. 耦合信号

粗略统计 `query/service` 内部对几个核心模块的直接引用次数：

| 模块 | 仓库内直接引用次数 |
| --- | ---: |
| `pipelines` | `866` |
| `physical_plans` | `731` |
| `sessions` | `491` |
| `interpreters` | `477` |
| `servers` | `340` |
| `table_functions` | `36` |

这个数字非常有用，因为它说明了两件事：

### 4.1 `pipelines` / `physical_plans` / `sessions` 是中心耦合点

这些模块不是简单的“大”，而是：

- 被大量代码直接引用
- 更容易形成“改一处，重编很多文件”的传播中心

### 4.2 `table_functions` 很大，但耦合 surprisingly 低

`table_functions` 有接近 `1` 万行，但直接引用次数只有 `36`。

这意味着它很可能是：

- 一块体量不小
- 功能上又比较外围
- 并且边界相对容易抽离

从拆分 ROI 的角度看，这反而是个很好的候选。

## 5. 拆分候选优先级

下面这个优先级，不是按“谁最大”排，而是按：

- 体量
- 与基础 SQL 路线的关系
- 耦合度
- 拆分难度
- 对内循环编译的潜在收益

### P1. `servers`

范围：

- `servers/http`
- `servers/flight`
- `servers/flight_sql`
- `servers/admin`
- `servers/mysql`

理由：

- 总计 `25190` 行，占 `query/service` 的 `16.12%`
- `http` 和 `flight` 两块本身就很重
- 对“开发 SQL planner / interpreter / execution”的内循环来说，它们并不是核心必需路径
- 这类模块天然更接近“运行时入口层”，比执行内核更适合做 crate 边界

我的判断：

- **这是最值得先拆的一层**
- 它不一定能减少所有编译时间，但最有希望减少“改 planner/executor 时把各种 server 一起重编”的情况

建议方向：

- 把 `servers` 从 `databend-query` 主体中剥成独立 crate
- `databend-query` 保留核心服务接口
- 二进制入口 crate 再去依赖 `query-servers-*`

### P2. `table_functions`

范围：

- `table_functions/*`
- `table_function_factory.rs`

理由：

- 约 `9736` 行，体量不小
- 直接引用次数只有 `36`
- 很多 table function 都不属于基础 SQL DML / DDL 主路径
- 模块边界比 `interpreters`、`pipelines` 清晰得多

我的判断：

- **这是最适合做“低风险先手拆分”的第二候选**

建议方向：

- 先按子域拆，不一定一步拆成一个 crate
- 例如：
  - `table_functions/system`
  - `table_functions/infer_schema`
  - `table_functions/cloud`
  - `table_functions/list_stage`
- 先从外围 table function 下手，而不是动 `fuse` 核心函数

### P3. `interpreters` 中的外围簇

重点不是一口气拆完整个 `interpreters`，而是先拆外围簇：

- `interpreters/task`
- `interpreters/access_log`
- `interpreters/hook`
- 一些云管控 / warehouse / policy / notification 类 interpreter

理由：

- `interpreters` 总体太大，约 `33052` 行
- 但它内部有很多“不是 SQL 基本路径必须”的外围解释器
- 如果直接拆整个 `interpreters`，风险太高
- 先拆外围簇，比继续在一个大目录里堆文件更有意义

我的判断：

- **不建议先拆整个 interpreters**
- **建议先拆 interpreter 的外围簇**

### P4. `sessions`

重点文件：

- `query_ctx.rs` (`2552` 行)
- `query_ctx_shared.rs` (`1014` 行)

理由：

- `sessions` 体量不算最夸张，但它是一个强中心节点
- `QueryContext` 往往会把很多系统能力绑在一起
- 这类模块很容易导致“几乎谁都要 import 它”

我的判断：

- 这是高价值但高风险的拆分点
- 更适合在 `servers` / `table_functions` 拆完之后再做

建议方向：

- 先减少 `QueryContext` 直接暴露的能力面
- 再考虑把其中一部分接口抽成更窄的 trait / facade

### P5. `physical_plans + pipelines`

理由：

- 两者合起来是 `query/service` 最大的一组核心执行层
- 同时也是耦合最强的一组
- 真正的编译收益潜力很大
- 但拆分成本和设计风险也最大

我的判断：

- 这不是第一刀
- 但这很可能是**最终不得不碰的一刀**

建议方向：

- 不要现在就硬拆 crate
- 先做边界扫描：
  - 哪些 `physical_plans` 只服务于某些 pipeline
  - 哪些 transform 可以先独立
  - 哪些 runtime / executor 逻辑能从大目录里收口

## 6. `query/sql` 的单独判断

虽然这份文档重点是 `databend-query` 主体，但不能忽略 `databend-common-sql`。

因为它已经是一个独立 crate，所以它的问题不是“先拆 crate”，而是：

- crate 内部仍然过大
- `binder` 与 `optimizer` 两块合计占 `67.82%`
- `type_check.rs` 单文件达到 `7745` 行

所以 `query/sql` 更像是下一阶段的专项方向：

### 不建议现在做的事

- 不要急着再给 `query/sql` 加很多 feature gate
- 不要先拆碎很多很小的 planner 子 crate

### 更建议做的事

1. 先拆超大文件  
   例如：
   - `semantic/type_check.rs`
   - `binder/ddl/table.rs`
   - `binder.rs`

2. 再找 planner 内部的真正边界  
   例如：
   - DDL binder
   - 表引用绑定
   - mutation binder
   - optimizer rule sets

所以对 `query/sql` 的建议是：

- **先做文件级和目录级收口**
- **暂时不把它当作下一刀 crate 拆分的首选**

## 7. 我建议的实际执行顺序

如果目标是“从现在开始，继续减少 query 主体编译负担”，我建议这样排：

1. 先拆 `servers`
2. 再拆 `table_functions`
3. 然后拆 `interpreters` 的外围簇
4. 再审视 `sessions`
5. 最后进入 `physical_plans + pipelines` 的深水区
6. `query/sql` 另开专项，先做超大文件收口，不急着 crate 化

## 8. 现阶段结论

如果只问一句：

> 在已经把 `stage` 这条线单独处理掉以后，下一步最值得做什么？

我的答案是：

1. **优先拆 `servers`**
2. **其次拆 `table_functions`**
3. **不要马上碰 `pipelines + physical_plans`，但要把它视为最终主战场**

原因很简单：

- `servers` 和 `table_functions` 足够大
- 又不像执行内核那样高度纠缠
- 最有机会在不引入大规模行为风险的情况下，直接降低 `databend-query` 主体体量和重编译传播面
