# Databend Query 最小 feature 基线下的逐 gate 单独打开编译对比

在当前最小开发编译基线之上，**每次只额外打开一个 gate**，`databend-query --lib` 的冷编译时间会增加多少。

目的是避免之前“混用不同并发参数、混用不同 feature 组合”的比较口径，改成单变量对比。

## 测量口径

所有 case 都使用同一套命令，只改变 `--features`：

```bash
ionice -c3 nice -n 15 cargo check \
  -p databend-query \
  --lib \
  --quiet \
  --no-default-features \
  -j 2 \
  --features "<case features>" \
  --target-dir "<独立 target dir>"
```

说明：

- 基线 feature：`simd`
- 每个 case 都使用独立 `--target-dir`
- 每个 case 都是冷编译
- 所有结果都来自同一天、同机器、同并发配置
- 时间文件位于 `target/bench-logs/query-lib-gate-*-j2-20260330.time`

## 原始结果

基线：

- feature：`simd`
- elapsed：`557.23s`
- max_rss_kb：`2507272`

逐个单独打开后的结果：

| gate | features | elapsed | 相对基线 | max_rss_kb | 相对基线 | 初步判断 |
| --- | --- | ---: | ---: | ---: | ---: | --- |
| baseline | `simd` | `557.23s` | `+0.00s` (`+0.00%`) | `2507272` | `+0` (`+0.00%`) | 基线 |
| `storage-stage` | `simd,storage-stage` | `813.16s` | `+255.93s` (`+45.93%`) | `2571092` | `+63820` (`+2.55%`) | 明显更重 |
| `script-udf` | `simd,script-udf` | `549.01s` | `-8.22s` (`-1.48%`) | `2582880` | `+75608` (`+3.02%`) | 时间差接近噪音 |
| `cloud-control` | `simd,cloud-control` | `518.50s` | `-38.73s` (`-6.95%`) | `2578832` | `+71560` (`+2.85%`) | 时间差接近噪音 |
| `virtual-column` | `simd,virtual-column` | `521.87s` | `-35.36s` (`-6.35%`) | `2550224` | `+42952` (`+1.71%`) | 时间差接近噪音 |
| `sql-script` | `simd,sql-script` | `536.18s` | `-21.05s` (`-3.78%`) | `2604776` | `+97504` (`+3.89%`) | 时间差接近噪音 |
| `storage-stream` | `simd,storage-stream` | `563.80s` | `+6.57s` (`+1.18%`) | `2518448` | `+11176` (`+0.45%`) | 基本贴着基线 |
| `fuse-management` | `simd,fuse-management` | `548.31s` | `-8.92s` (`-1.60%`) | `2620432` | `+113160` (`+4.51%`) | 时间差接近噪音 |

## 公平解读

### 1. 只有 `storage-stage` 显示出明确、可见的冷编译成本

`storage-stage` 单独打开后：

- 时间增加 `255.93s`
- 增幅 `45.93%`

这已经远大于本轮其它 case 的波动范围，所以可以认为：

- `storage-stage` gate 对最小开发编译路径的裁剪收益是**真实存在**的
- 这条 gate 不是“感觉上有用”，而是单独拿出来也能测到明显差异

### 2. 其余 gate 在这轮单次冷编译里都没有证明自己“单独很重”

除了 `storage-stage` 之外：

- `storage-stream` 只比基线慢 `6.57s`
- `script-udf` / `cloud-control` / `virtual-column` / `sql-script` / `fuse-management` 甚至都比基线更快

这**不能**说明“打开这些 gate 会让编译更快”，更合理的解释是：

- 单次冷编译本身存在波动
- 这些 gate 的单独成本还没有大到能稳定压过这层噪音

所以公平结论应该是：

- 目前没有证据表明这些 gate 单独打开时会显著拖慢最小开发编译
- 也就没有证据表明“仅靠裁掉它们”能带来显著的冷编译收益

### 3. 如果只从“编译时间收益”评判，大部分 gate 现在都不够硬

如果只看这轮统一口径结果：

- **明确值得保留的编译优化 gate**：`storage-stage`
- **编译收益暂时证据不足的 gate**：
  - `script-udf`
  - `cloud-control`
  - `virtual-column`
  - `sql-script`
  - `storage-stream`
  - `fuse-management`

这里的“证据不足”不是说这些 gate 没价值，而是说：

- 它们的价值目前更多来自领域边界、依赖图整洁度、最小功能面控制
- 不是来自本轮已经被测出来的明确冷编译时间收益

## 维护成本上的新信号

这轮“逐个单独打开 gate”还额外暴露了两个 feature 组合问题：

1. `sql-script` 单开时，`src/query/service/src/interpreters/util.rs` 缺少 `TableContext` trait import
2. `fuse-management` 单开时，`src/query/service/src/interpreters/hook/hook.rs` 缺少 `TableContext` trait import

这说明当前 gate 方案除了编译时间问题，还有一个现实成本：

- 单独 feature 组合是否真的可编过，需要持续维护
- gate 越多，类似“默认全开没事，单开才炸”的问题越容易出现

## 现阶段结论

如果现在要做“公平评判”，我会给出下面这个判断：

1. `storage-stage` 应继续保留，它的编译收益已经被这轮单变量测试明确证明。
2. 其余 gate 暂时都不能仅凭“编译更快”作为强理由。
3. 如果后面要回退一部分 gate 以降低维护复杂度，优先级不应再靠主观印象，而应更多参考：
   - 这轮单 gate 编译收益是否明显
   - 之前统计的改动文件数是否过大
   - 该 gate 是否带来了额外的 feature 组合维护负担
4. 结合之前的改动面统计文档看，最值得继续审视的仍然是：
   - `storage-stream`
   - `fuse-management`

原因不是它们“单独最重”，而是：

- 这轮没有测出明显的单独编译收益
- 但它们已经带来了额外 gate 维护复杂度

## 相关文件

- 原始 time 文件：
  - `target/bench-logs/query-lib-gate-baseline-j2-20260330.time`
  - `target/bench-logs/query-lib-gate-storage-stage-j2-20260330.time`
  - `target/bench-logs/query-lib-gate-script-udf-j2-20260330.time`
  - `target/bench-logs/query-lib-gate-cloud-control-j2-20260330.time`
  - `target/bench-logs/query-lib-gate-virtual-column-j2-20260330.time`
  - `target/bench-logs/query-lib-gate-sql-script-j2-20260330.time`
  - `target/bench-logs/query-lib-gate-storage-stream-j2-20260330.time`
  - `target/bench-logs/query-lib-gate-fuse-management-j2-20260330.time`
- 改动面盘点：
  - `benchmark/query-min-gate-cost-review-2026-03-30.md`
