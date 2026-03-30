# Databend Query `distributed` 真 Crate 对齐基准

日期：2026-03-30

这份文档专门记录把 `distributed` 的第一刀做成真实 crate 之后，`databend-query` 最小开发编译路径在**和之前完全同口径**条件下的结果。

目的只有一个：

- 判断这次 `distributed` 真 crate 拆分，是否已经带来可见的冷编译收益

相关背景文档：

- [query-min-compile-progress-2026-03-30.md](/home/kould/RustroverProjects/databend/benchmark/query-min-compile-progress-2026-03-30.md)
- [query-core-split-candidates-2026-03-30.md](/home/kould/RustroverProjects/databend/benchmark/query-core-split-candidates-2026-03-30.md)

## 1. 对齐口径

本次严格沿用之前的正式基准口径：

- `cargo check`
- `databend-query --lib`
- `--no-default-features --features simd`
- `-j 4`
- 独立冷 `target-dir`
- `ionice -c3 nice -n 15`
- `/usr/bin/time` 记录墙钟和峰值 RSS

使用命令：

```bash
ionice -c3 nice -n 15 /usr/bin/time -f 'elapsed=%e\nmax_rss_kb=%M' \
  -o target/bench-logs/query-lib-min-distributed-split-j4.time \
  cargo check -j 4 -p databend-query --lib --no-default-features --features simd \
  --target-dir target/bench-q-lib-min-distributed-split-j4 --quiet
```

原始产物：

- `target/bench-logs/query-lib-min-distributed-split-j4.time`

## 2. 第一刀结果

第一刀 `distributed` 真 crate 拆分之后的正式结果：

- 墙钟时间：`376.08s`
- 峰值 RSS：`2518936 KB`

## 3. 第二刀结果

继续把 `network + scatter` 并进 `databend-query-distributed` 之后，再次按同口径测得：

- 墙钟时间：`379.93s`
- 峰值 RSS：`2500872 KB`

使用命令：

```bash
ionice -c3 nice -n 15 /usr/bin/time -f 'elapsed=%e\nmax_rss_kb=%M' \
  -o target/bench-logs/query-lib-min-distributed-split-network-scatter-j4.time \
  cargo check -j 4 -p databend-query --lib --no-default-features --features simd \
  --target-dir target/bench-q-lib-min-distributed-split-network-scatter-j4 --quiet
```

原始产物：

- `target/bench-logs/query-lib-min-distributed-split-network-scatter-j4.time`

和第一刀相比：

- 墙钟时间慢了 `3.85s`，约 `1.02%`
- 峰值 RSS 降了 `18064 KB`，约 `0.72%`

所以第二刀目前的信号是：

- 编译边界继续扩大了
- 但冷编译墙钟没有继续明显下降
- 额外收益已经开始明显小于第一刀

## 4. 和之前正式口径怎么比

| 阶段 | 命令口径 | 墙钟时间 | 峰值 RSS | 备注 |
| --- | --- | ---: | ---: | --- |
| 上一轮正式基准 | `-j 4` | `423.33s` | `2499436 KB` | `stream/sql-script/fuse-management` gate 文档中的正式结果 |
| 更早一轮正式基准 | `-j 4` | `396.09s` | `2626716 KB` | `virtual-column` 那轮正式结果 |
| `stage/lance/protobuf` 裁切后 | 历史 bench | `379.06s` | `2606712 KB` | `target/bench-logs/query-lib-min-stagecut.time` |
| `distributed` 第一刀后 | `-j 4` | `376.08s` | `2518936 KB` | `flight_client/request_builder/keep_alive/packets` |
| `distributed` 第二刀后 | `-j 4` | `379.93s` | `2500872 KB` | 继续并进 `network/scatter` |

按同口径直接比较：

- 第一刀相比上一轮 `423.33s`，快了 `47.25s`，约 `11.16%`
- 第二刀相比上一轮 `423.33s`，快了 `43.40s`，约 `10.25%`
- 第二刀相比第一刀，慢了 `3.85s`，约 `1.02%`
- 第二刀相比 `stage/lance/protobuf` 裁切后的 `379.06s`，慢了 `0.87s`，约 `0.23%`

## 5. 如何理解这次结果

这次结果和之前的 feature gate 轮次有一个关键区别：

- 这次不是继续裁 feature
- 而是开始把 `databend-query` 根 crate 内的一部分代码，变成真正独立的 Cargo crate

当前已经确认的事实有三条：

1. 编译日志里会先出现 `databend-query-distributed`，再出现 `databend-query`
2. 第一刀在同口径 `-j 4` 冷编译下，墙钟时间确实出现了可见下降
3. 第二刀继续扩大边界后，冷编译收益开始明显变小，甚至略有回吐

所以至少到这一步，可以先得到一个更细一点的结论：

- `distributed` 做成真 crate，不只是“目录整理”
- 第一刀已经产生了可观测的编译收益
- 但继续把 `network/scatter` 并进去后，冷编译收益没有线性继续放大

不过这份结果还不能直接证明：

- 收益完全来自“增量复用”
- 或者下一块随便继续拆也会有同样幅度

更保守的表述应该是：

- 这条方向已经从“结构猜想”变成“有基准支持的方向”
- 可以继续沿 `query` 主体内部做真 crate 边界，而不必只盯 feature gate

## 6. 当前这两刀具体做了什么

本轮新增真实 crate：

- `src/query/distributed`

第一刀迁入的内容主要是：

- `flight_client`
- `request_builder`
- `keep_alive`
- `packets`

第二刀继续迁入：

- `network`
- `scatter`

而 `service` 侧仍保留：

- `exchange`
- `network`
- `scatter`

这意味着本轮还只是 `distributed` 的第一刀，不是最终形态。

## 7. 下一步建议

既然这次对齐基准已经证明“真 crate 边界有收益”，下一步更值得继续挖的方向是：

1. 如果继续沿 `distributed` 深挖，下一刀应该盯 `exchange` 主体，而不是再搬更边缘的小块
2. 用同样方法评估 `servers` / `runtime_managers` / 其它 query 主体模块的真 crate 拆分价值
3. 把关注点逐步从“还能裁哪个小 gate”转向“哪个中心耦合块值得做真正的 Cargo 边界”
