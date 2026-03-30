# Databend Query 真 Crate 拆分实验记录

日期：2026-03-30

这份文档不讨论 feature gate，而是专门记录 `databend-query` 主体内部做“真 crate 拆分”的实验结果。

目标不是一次性决定最终要拆哪些，而是采用下面这套策略：

- 每次只拆一块
- 每次都跑同口径基准
- 先看收益，再决定这块是否值得继续深挖或保留

也就是说，当前路线是：

- 逐个拆解
- 逐个验证
- 最后根据收益决定优先保留哪些拆分

## 1. 统一口径

所有正式对比都尽量使用同一套冷编译口径：

```bash
ionice -c3 nice -n 15 /usr/bin/time -f 'elapsed=%e\nmax_rss_kb=%M' \
  cargo check -j 4 -p databend-query --lib --no-default-features --features simd \
  --target-dir <独立冷 target-dir> --quiet
```

比较原则：

- 优先比较墙钟时间
- 同时记录峰值 RSS
- 每一刀都使用独立 `target-dir`
- 不把不同并发度、不同 feature 组合的数据混在一起

## 2. 当前实验结果

| 实验 | 拆分内容 | 正式结果 | 相对上一基线 | 初步判断 |
| --- | --- | ---: | ---: | --- |
| 基线 | 上一轮最小正式基准 | `423.33s` / `2499436 KB` | `+0.00s` | 对照组 |
| `distributed` 第一刀 | `flight_client`、`request_builder`、`keep_alive`、`packets` | `376.08s` / `2518936 KB` | `-47.25s` (`-11.16%`) | 收益明显，值得保留 |
| `distributed` 第二刀 | 继续并进 `network`、`scatter` | `379.93s` / `2500872 KB` | 相对第一刀 `+3.85s` (`+1.02%`) | 冷编译收益不明显 |

说明：

- 第一刀相对基线收益很明确，说明“真 crate 边界”这条方向本身成立
- 第二刀继续扩大 `distributed` 边界后，冷编译没有继续明显变快，已经出现明显收益递减

## 3. 当前阶段性结论

从目前已测出来的数据看：

1. `distributed` 拆成真 crate 这件事本身是有效的。
2. 但不是拆进去的每一块都会继续线性带来收益。
3. 所以后续不应该凭感觉一路拆到底，而应该继续坚持“拆一块，测一块”的节奏。

换句话说，目前更合理的做法不是：

- 先假设整条路线都值得拆

而是：

- 把每一刀都当成独立实验
- 只有收益足够清晰的拆分，才继续往下扩

## 4. 当前保留标准

后面是否继续保留或深挖某一块拆分，优先参考这几个信号：

1. 同口径冷编译时间是否有明确改善。
2. 是否形成了真实的 Cargo crate 边界，而不只是目录整理。
3. 是否降低了后续进一步拆分的门槛。
4. 改动复杂度是否和收益匹配。

## 5. 下一步使用方式

后面每做一刀新的 crate 拆分，都追加一条记录：

- 拆了哪一块
- 改动范围是什么
- 正式基准是多少
- 相对上一状态变化多少
- 是否继续保留 / 是否值得继续深挖

这样最后我们就不是靠直觉选方向，而是可以回头看完整实验表，再决定：

- 哪些拆分值得继续
- 哪些拆分收益太小，不值得复杂化代码结构

## 6. 相关文档

- [query-distributed-split-benchmark-2026-03-30.md](/home/kould/RustroverProjects/databend/benchmark/query-distributed-split-benchmark-2026-03-30.md)
- [query-core-split-candidates-2026-03-30.md](/home/kould/RustroverProjects/databend/benchmark/query-core-split-candidates-2026-03-30.md)
