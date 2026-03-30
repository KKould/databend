# Databend Query 真 Crate 拆分 Workspaces 方案

日期：2026-03-30

这份文档记录当前用于“逐个拆、逐个测”的 workspace 组织方式。

目标：

- 每个候选拆分方向放到独立 workspace
- 各 workspace 都从同一份当前实验基线出发
- 后续串行推进、串行跑基准，避免互相污染

## 1. 当前约定的实验对象

这里先按当前讨论里最值得观察的五个方向组织 workspace。

假设“这五个”指的是：

1. `exchange`
2. `pipelines`
3. `physical_plans`
4. `interpreters`
5. `databend-common-sql`

同时额外保留一个当前基线 workspace：

- `baseline-current`

所以当前总共会准备 6 个 workspaces：

| workspace | 用途 |
| --- | --- |
| `baseline-current` | 当前基线，作为后续所有拆分对照组 |
| `split-exchange` | 实验性拆分 `servers/flight/v1/exchange` |
| `split-pipelines` | 实验性拆分 `pipelines` |
| `split-physical-plans` | 实验性拆分 `physical_plans` |
| `split-interpreters` | 实验性拆分 `interpreters` |
| `split-common-sql` | 实验性拆分 `databend-common-sql` |

## 2. 为什么要单独 workspace

这样做的意义主要有三点：

1. 每个方向都能从相同基线出发，结果更容易公平比较。
2. 各拆分尝试互不干扰，不会出现“上一刀的半成品污染下一刀”的情况。
3. 最后即使只保留其中一两个方向，其它实验 workspace 也能独立清理。

## 3. 初始化脚本

初始化脚本：

- [`scripts/query-split/setup-experiment-workspaces.sh`](/home/kould/RustroverProjects/databend/scripts/query-split/setup-experiment-workspaces.sh)

这个脚本会：

1. 从当前仓库 `HEAD` 创建多个 `git worktree`
2. 把当前工作区的已跟踪修改以 patch 形式同步到每个 workspace
3. 把当前未跟踪文件同步到每个 workspace
4. 在每个 workspace 根目录写入 `.query-split-experiment` 元信息

也就是说，它不是简单从干净 `HEAD` 起新目录，而是会把“当前实验基线状态”复制到每个 workspace。

## 4. 使用方式

默认会在当前仓库同级目录下创建：

- `../databend-split-workspaces`

执行命令：

```bash
./scripts/query-split/setup-experiment-workspaces.sh
```

也可以手动指定目标根目录：

```bash
./scripts/query-split/setup-experiment-workspaces.sh /path/to/workspaces
```

## 5. 后续实验节奏

建议后续按下面的节奏推进：

1. 先在 `baseline-current` 记录当前正式基准
2. 进入某一个 `split-*` workspace 做单方向拆分
3. 用统一口径跑正式基准
4. 把结果追加到实验记录文档
5. 再决定这条拆分是否值得继续保留

## 6. 关联文档

- [query-crate-split-experiments-2026-03-30.md](/home/kould/RustroverProjects/databend/benchmark/query-crate-split-experiments-2026-03-30.md)
- [query-core-split-candidates-2026-03-30.md](/home/kould/RustroverProjects/databend/benchmark/query-core-split-candidates-2026-03-30.md)
