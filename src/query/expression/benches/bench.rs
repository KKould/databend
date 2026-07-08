// Copyright 2021 Datafuse Labs
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use arrow_buffer::BooleanBuffer;
use arrow_buffer::ScalarBuffer;
use databend_common_base::vec_ext::VecExt;
use databend_common_column::bitmap::Bitmap;
use databend_common_column::buffer::Buffer;
use databend_common_expression::Column;
use databend_common_expression::DataBlock;
use databend_common_expression::FromData;
use databend_common_expression::RepeatIndex;
use databend_common_expression::arrow::deserialize_column;
use databend_common_expression::arrow::serialize_column;
use databend_common_expression::types::BinaryType;
use databend_common_expression::types::DecimalType;
use databend_common_expression::types::ReturnType;
use databend_common_expression::types::StringType;
use rand::Rng;
use rand::SeedableRng;
use rand::rngs::StdRng;

fn main() {
    // Run registered benchmarks.
    divan::main();
}

// bench                    fastest       │ slowest       │ median        │ mean          │ samples │ iters
// ├─ concat_string_offset                │               │               │               │         │
// │  ├─ 12                 22.58 ms      │ 31.43 ms      │ 24.05 ms      │ 24.83 ms      │ 100     │ 100
// │  ├─ 20                 23.32 ms      │ 29.58 ms      │ 26.87 ms      │ 26.44 ms      │ 100     │ 100
// │  ╰─ 500                295.2 ms      │ 314.2 ms      │ 301.4 ms      │ 302 ms        │ 100     │ 100
// ╰─ concat_string_view                  │               │               │               │         │
//    ├─ 12                 23.68 ms      │ 25.96 ms      │ 24.42 ms      │ 24.46 ms      │ 100     │ 100
//    ├─ 20                 26.27 ms      │ 27.79 ms      │ 26.85 ms      │ 26.85 ms      │ 100     │ 100
//    ╰─ 500                118.8 ms      │ 247.2 ms      │ 121.5 ms      │ 123.3 ms      │ 100     │ 100
#[divan::bench(args = [12, 20, 500])]
fn concat_string_offset(bencher: divan::Bencher, length: usize) {
    let mut rng = StdRng::seed_from_u64(0);
    let (_s, b) = generate_random_string_data(&mut rng, length);
    let bin_col = (0..5).map(|_| BinaryType::from_data(b.clone()));

    bencher.bench(|| {
        Column::concat_columns(bin_col.clone()).unwrap();
    });
}

#[divan::bench(args = [12, 20, 500])]
fn concat_string_view(bencher: divan::Bencher, length: usize) {
    let mut rng = StdRng::seed_from_u64(0);
    let (s, _b) = generate_random_string_data(&mut rng, length);
    let str_col = (0..5).map(|_| StringType::from_data(s.clone()));
    bencher.bench(|| {
        Column::concat_columns(str_col.clone()).unwrap();
    });
}

#[divan::bench(args = [12, 20, 500])]
fn take_compact_string_offset(bencher: divan::Bencher, length: usize) {
    let mut rng = StdRng::seed_from_u64(0);
    let (s, b) = generate_random_string_data(&mut rng, length);
    let block_bin = DataBlock::new_from_columns(vec![BinaryType::from_data(b.clone())]);
    let indices: Vec<RepeatIndex> = (0..s.len())
        .filter(|x| x % 10 == 0)
        .map(|x| RepeatIndex {
            row: x as u32,
            count: 1000,
        })
        .collect();
    let num_rows = indices.len() * 1000;
    bencher.bench(|| {
        block_bin
            .take_compacted_indices(&indices, num_rows)
            .unwrap();
    });
}

#[divan::bench(args = [12, 20, 500])]
fn take_compact_string_view(bencher: divan::Bencher, length: usize) {
    let mut rng = StdRng::seed_from_u64(0);
    let (s, _b) = generate_random_string_data(&mut rng, length);
    let block_view = DataBlock::new_from_columns(vec![StringType::from_data(s.clone())]);
    let indices: Vec<RepeatIndex> = (0..s.len())
        .filter(|x| x % 10 == 0)
        .map(|x| RepeatIndex {
            row: x as u32,
            count: 1000,
        })
        .collect();
    let num_rows = indices.len() * 1000;
    bencher.bench(|| {
        block_view
            .take_compacted_indices(&indices, num_rows)
            .unwrap();
    });
}

// bench                       fastest       │ slowest       │ median        │ mean          │ samples │ iters
// ├─ serialize_string_offset                │               │               │               │         │
// │  ├─ 12                    3.057 ms      │ 4.628 ms      │ 3.194 ms      │ 3.265 ms      │ 100     │ 100
// │  ├─ 20                    4.651 ms      │ 6.266 ms      │ 4.857 ms      │ 4.911 ms      │ 100     │ 100
// │  ╰─ 500                   50.15 ms      │ 58.9 ms       │ 52.54 ms      │ 53 ms         │ 100     │ 100
// ╰─ serialize_string_view                  │               │               │               │         │
//    ├─ 12                    3.221 ms      │ 3.79 ms       │ 3.335 ms      │ 3.331 ms      │ 100     │ 100
//    ├─ 20                    3.838 ms      │ 4.502 ms      │ 3.932 ms      │ 3.977 ms      │ 100     │ 100
//    ╰─ 500                   69.78 ms      │ 74.67 ms      │ 70.88 ms      │ 71.05 ms      │ 100     │ 100
#[divan::bench(args = [12, 20, 500])]
fn serialize_string_offset(bencher: divan::Bencher, length: usize) {
    let mut rng = StdRng::seed_from_u64(0);
    let (_s, b) = generate_random_string_data(&mut rng, length);
    let b_c = BinaryType::from_data(b.clone());

    bencher.bench(|| {
        let bs = serialize_column(&b_c);
        deserialize_column(&bs).unwrap();
    });
}

#[divan::bench(args = [12, 20, 500])]
fn serialize_string_view(bencher: divan::Bencher, length: usize) {
    let mut rng = StdRng::seed_from_u64(0);
    let (s, _b) = generate_random_string_data(&mut rng, length);
    let s_c = StringType::from_data(s.clone());

    bencher.bench(|| {
        let bs = serialize_column(&s_c);
        deserialize_column(&bs).unwrap();
    });
}

// bench                                               fastest       │ slowest       │ median        │ mean          │ samples │ iters
// ├─ function_buffer_index_unchecked_iterator                       │               │               │               │         │
// │  ├─ 10240                                         18.17 µs      │ 76.9 µs       │ 18.54 µs      │ 19.24 µs      │ 100     │ 100
// │  ╰─ 102400                                        183.1 µs      │ 508.8 µs      │ 186.8 µs      │ 194.7 µs      │ 100     │ 100
// ├─ function_buffer_index_unchecked_push                           │               │               │               │         │
// │  ├─ 10240                                         18.52 µs      │ 20.83 µs      │ 18.55 µs      │ 18.64 µs      │ 100     │ 100
// │  ╰─ 102400                                        187.6 µs      │ 439.7 µs      │ 191.2 µs      │ 192.8 µs      │ 100     │ 100
// ├─ function_buffer_scalar_index_unchecked_iterator                │               │               │               │         │
// │  ├─ 10240                                         11.58 µs      │ 12.94 µs      │ 11.6 µs       │ 11.63 µs      │ 100     │ 100
// │  ╰─ 102400                                        115.9 µs      │ 492.3 µs      │ 118.3 µs      │ 122.3 µs      │ 100     │ 100
// ├─ function_iterator_iterator_ref                                 │               │               │               │         │
// │  ├─ 10240                                         6.301 µs      │ 6.859 µs      │ 6.318 µs      │ 6.325 µs      │ 100     │ 100
// │  ╰─ 102400                                        77.07 µs      │ 390.6 µs      │ 77.27 µs      │ 81.39 µs      │ 100     │ 100
// ├─ function_iterator_iterator_v1                                  │               │               │               │         │
// │  ├─ 10240                                         9.502 µs      │ 14.74 µs      │ 9.535 µs      │ 9.694 µs      │ 100     │ 100
// │  ╰─ 102400                                        100.9 µs      │ 344.6 µs      │ 101 µs        │ 103.9 µs      │ 100     │ 100
// ╰─ function_iterator_iterator_v2                                  │               │               │               │         │
//    ├─ 10240                                         6.307 µs      │ 6.447 µs      │ 6.322 µs      │ 6.324 µs      │ 100     │ 100
//    ╰─ 102400                                        77.49 µs      │ 317.6 µs      │ 77.73 µs      │ 80.28 µs      │ 100     │ 100
#[divan::bench(args = [10240, 102400])]
fn function_iterator_iterator_v1(bencher: divan::Bencher, length: usize) {
    let mut rng = StdRng::seed_from_u64(0);
    let (left, right) = generate_random_i128_data(&mut rng, length);

    bencher.bench(|| {
        let left = left.clone();
        let right = right.clone();

        divan::black_box(
            left.into_iter()
                .zip(right)
                .map(|(a, b)| a * b)
                .collect::<Vec<i128>>(),
        )
    });
}

#[divan::bench(args = [10240, 102400])]
fn function_iterator_iterator_ref(bencher: divan::Bencher, length: usize) {
    let mut rng = StdRng::seed_from_u64(0);
    let (left, right) = generate_random_i128_data(&mut rng, length);

    bencher.bench(|| {
        divan::black_box(
            left.iter()
                .zip(right.iter())
                .map(|(a, b)| *a * *b)
                .collect::<Vec<i128>>(),
        )
    });
}

#[divan::bench(args = [10240, 102400])]
fn function_iterator_iterator_v2(bencher: divan::Bencher, length: usize) {
    let mut rng = StdRng::seed_from_u64(0);
    let (left, right) = generate_random_i128_data(&mut rng, length);

    bencher.bench(|| {
        let iter = left
            .iter()
            .cloned()
            .zip(right.iter().cloned())
            .map(|(a, b)| a * b);
        divan::black_box(DecimalType::<i128>::column_from_iter(iter, &[]))
    });
}

#[divan::bench(args = [10240, 102400])]
fn function_buffer_index_unchecked_iterator(bencher: divan::Bencher, length: usize) {
    let mut rng = StdRng::seed_from_u64(0);
    let (left, right) = generate_random_i128_data(&mut rng, length);

    bencher.bench(|| {
        divan::black_box(
            (0..length)
                .map(|i| unsafe { left.get_unchecked(i) * right.get_unchecked(i) })
                .collect::<Vec<i128>>(),
        )
    });
}

#[divan::bench(args = [10240, 102400])]
fn function_buffer_index_unchecked_push(bencher: divan::Bencher, length: usize) {
    let mut rng = StdRng::seed_from_u64(0);
    let (left, right) = generate_random_i128_data(&mut rng, length);

    bencher.bench(|| {
        let mut c = Vec::with_capacity(length);
        for i in 0..length {
            unsafe { c.push_unchecked(left.get_unchecked(i) * right.get_unchecked(i)) };
        }
    });
}

#[divan::bench(args = [10240, 102400])]
fn function_buffer_scalar_index_unchecked_iterator(bencher: divan::Bencher, length: usize) {
    let mut rng = StdRng::seed_from_u64(0);
    let (left, right) = generate_random_i128_data(&mut rng, length);
    let left_scalar = ScalarBuffer::from_iter(left.iter().cloned());
    let right_scalar = ScalarBuffer::from_iter(right.iter().cloned());

    bencher.bench(|| {
        divan::black_box(
            (0..length)
                .map(|i| unsafe { left_scalar.get_unchecked(i) * right_scalar.get_unchecked(i) })
                .collect::<Vec<i128>>(),
        )
    });
}

// Timer precision: 10 ns
// bench                               fastest       │ slowest       │ median        │ mean          │ samples │ iters
// ├─ bitmap_from_arrow1_collect_bool                │               │               │               │         │
// │  ├─ 10240                         216.8 ns      │ 4.312 µs      │ 218.8 ns      │ 263.7 ns      │ 100     │ 100
// │  ╰─ 102400                        1.425 µs      │ 1.673 µs      │ 1.433 µs      │ 1.44 µs       │ 100     │ 100
// ├─ bitmap_from_arrow2                             │               │               │               │         │
// │  ├─ 10240                         4.427 µs      │ 6.18 µs       │ 4.572 µs      │ 4.855 µs      │ 100     │ 100
// │  ╰─ 102400                        43.84 µs      │ 62.38 µs      │ 54.28 µs      │ 53.52 µs      │ 100     │ 100
// ╰─ bitmap_from_arrow2_collect_bool                │               │               │               │         │
//    ├─ 10240                         175.6 ns      │ 195.2 ns      │ 179.8 ns      │ 180.3 ns      │ 100     │ 800
//    ╰─ 102400                        1.487 µs      │ 1.6 µs        │ 1.501 µs      │ 1.504 µs      │ 100     │ 100
#[divan::bench(args = [10240, 102400])]
fn bitmap_from_arrow1_collect_bool(bencher: divan::Bencher, length: usize) {
    bencher.bench(|| {
        let buffer = collect_bool(length, false, |x| x % 2 == 0);
        assert!(buffer.count_set_bits() == length / 2);
    });
}

#[divan::bench(args = [10240, 102400])]
fn bitmap_from_arrow2_collect_bool(bencher: divan::Bencher, length: usize) {
    bencher.bench(|| {
        let nulls = Bitmap::collect_bool(length, |x| x % 2 == 0);
        assert!(nulls.null_count() == length / 2);
    });
}

#[divan::bench(args = [10240, 102400])]
fn bitmap_from_arrow2(bencher: divan::Bencher, length: usize) {
    bencher.bench(|| {
        let nulls = Bitmap::from_trusted_len_iter((0..length).map(|x| x % 2 == 0));
        assert!(nulls.null_count() == length / 2);
    });
}

#[divan::bench_group(max_time = 1)]
mod skew_hash_hot_key {
    use std::cmp::Ordering;
    use std::fmt;

    use databend_common_expression::Scalar;
    use databend_common_expression::ScalarRef;
    use databend_common_expression::types::UInt64Type;
    use databend_common_expression::types::number::NumberScalar;

    use super::*;

    #[derive(Clone, Copy, Debug)]
    enum KeyKind {
        UInt64,
        String,
    }

    impl KeyKind {
        fn as_str(self) -> &'static str {
            match self {
                KeyKind::UInt64 => "u64",
                KeyKind::String => "str",
            }
        }
    }

    #[derive(Clone, Copy, Debug)]
    struct HotKeyCase {
        key_kind: KeyKind,
        rows: usize,
        hot_key_count: usize,
        hot_percent: usize,
    }

    impl fmt::Display for HotKeyCase {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(
                f,
                "{}_rows{}_keys{}_hot{}",
                self.key_kind.as_str(),
                self.rows,
                self.hot_key_count,
                self.hot_percent
            )
        }
    }

    const CASES: &[HotKeyCase] = &[
        HotKeyCase {
            key_kind: KeyKind::UInt64,
            rows: 8192,
            hot_key_count: 1,
            hot_percent: 0,
        },
        HotKeyCase {
            key_kind: KeyKind::UInt64,
            rows: 8192,
            hot_key_count: 8,
            hot_percent: 50,
        },
        HotKeyCase {
            key_kind: KeyKind::UInt64,
            rows: 65536,
            hot_key_count: 8,
            hot_percent: 50,
        },
        HotKeyCase {
            key_kind: KeyKind::UInt64,
            rows: 65536,
            hot_key_count: 8,
            hot_percent: 100,
        },
        HotKeyCase {
            key_kind: KeyKind::UInt64,
            rows: 65536,
            hot_key_count: 64,
            hot_percent: 50,
        },
        HotKeyCase {
            key_kind: KeyKind::String,
            rows: 8192,
            hot_key_count: 1,
            hot_percent: 0,
        },
        HotKeyCase {
            key_kind: KeyKind::String,
            rows: 8192,
            hot_key_count: 8,
            hot_percent: 50,
        },
        HotKeyCase {
            key_kind: KeyKind::String,
            rows: 65536,
            hot_key_count: 8,
            hot_percent: 50,
        },
        HotKeyCase {
            key_kind: KeyKind::String,
            rows: 65536,
            hot_key_count: 8,
            hot_percent: 100,
        },
        HotKeyCase {
            key_kind: KeyKind::String,
            rows: 65536,
            hot_key_count: 64,
            hot_percent: 50,
        },
    ];

    struct ScatterLayout {
        scatter_size: usize,
        node_partitions: Vec<(usize, usize)>,
        bucket_count: usize,
    }

    struct BenchInput {
        column: Column,
        hot_keys: Vec<Scalar>,
        hashes: Vec<u64>,
        layout: ScatterLayout,
        rows: usize,
    }

    impl BenchInput {
        fn new(case: HotKeyCase) -> Self {
            let hot_keys = build_hot_keys(case);
            let column = build_column(case);
            let hashes = (0..case.rows).map(hash_row).collect();

            Self {
                column,
                hot_keys,
                hashes,
                layout: ScatterLayout {
                    scatter_size: 16,
                    node_partitions: vec![(0, 4), (4, 4), (8, 4), (12, 4)],
                    bucket_count: 4,
                },
                rows: case.rows,
            }
        }
    }

    fn build_hot_keys(case: HotKeyCase) -> Vec<Scalar> {
        match case.key_kind {
            KeyKind::UInt64 => (0..case.hot_key_count)
                .map(|value| Scalar::Number(NumberScalar::UInt64(value as u64)))
                .collect(),
            KeyKind::String => (0..case.hot_key_count)
                .map(|value| Scalar::String(hot_string(value)))
                .collect(),
        }
    }

    fn build_column(case: HotKeyCase) -> Column {
        match case.key_kind {
            KeyKind::UInt64 => {
                let values = (0..case.rows)
                    .map(|row| {
                        if is_hot_row(case, row) {
                            (row % case.hot_key_count) as u64
                        } else {
                            (case.hot_key_count + row + 1) as u64
                        }
                    })
                    .collect();
                UInt64Type::from_data(values)
            }
            KeyKind::String => {
                let values = (0..case.rows)
                    .map(|row| {
                        if is_hot_row(case, row) {
                            hot_string(row % case.hot_key_count)
                        } else {
                            format!("cold_{row:08}")
                        }
                    })
                    .collect::<Vec<_>>();
                StringType::from_data(values)
            }
        }
    }

    fn hot_string(value: usize) -> String {
        format!("hot_{value:04}")
    }

    fn is_hot_row(case: HotKeyCase, row: usize) -> bool {
        match case.hot_percent {
            0 => false,
            100 => true,
            hot_percent => row % 100 < hot_percent,
        }
    }

    fn hash_row(row: usize) -> u64 {
        (row as u64)
            .wrapping_mul(11_400_714_819_323_198_485)
            .rotate_left(13)
    }

    fn is_hot_scalar_ref(hot_keys: &[Scalar], scalar: ScalarRef<'_>) -> bool {
        if matches!(scalar, ScalarRef::Null) {
            return false;
        }

        match hot_keys.binary_search_by(|hot_key| hot_key.as_ref().cmp(&scalar)) {
            Ok(index) => hot_keys[index].as_ref().partial_cmp(&scalar) == Some(Ordering::Equal),
            Err(_) => false,
        }
    }

    fn is_hot_key(input: &BenchInput, row: usize) -> bool {
        let scalar = unsafe { input.column.index_unchecked(row) };
        is_hot_scalar_ref(&input.hot_keys, scalar)
    }

    fn hot_key_bitmap(input: &BenchInput) -> Bitmap {
        Bitmap::from_trusted_len_iter((0..input.rows).map(|row| is_hot_key(input, row)))
    }

    fn normal_partition(layout: &ScatterLayout, hash: u64) -> u64 {
        hash % layout.scatter_size as u64
    }

    fn skew_partition(layout: &ScatterLayout, hash: u64, salt: usize) -> u64 {
        let node_count = layout.node_partitions.len();
        let node_index = ((hash % node_count as u64) as usize + salt) % node_count;
        let (partition_start, partition_count) = layout.node_partitions[node_index];
        let local_partition = (hash % partition_count as u64) as usize;
        (partition_start + local_partition) as u64
    }

    fn probe_indices_binary_search(input: &BenchInput) -> Vec<u64> {
        let mut salt = 0;
        (0..input.rows)
            .map(|row| {
                let hash = input.hashes[row];
                if is_hot_key(input, row) {
                    let partition = skew_partition(&input.layout, hash, salt);
                    salt = (salt + 1) % input.layout.bucket_count;
                    partition
                } else {
                    normal_partition(&input.layout, hash)
                }
            })
            .collect()
    }

    fn probe_indices_bitmap(input: &BenchInput) -> Vec<u64> {
        let hot_keys = hot_key_bitmap(input);
        let hot_count = hot_keys.true_count();
        let mut salt = 0;

        if hot_count == 0 {
            return input
                .hashes
                .iter()
                .map(|hash| normal_partition(&input.layout, *hash))
                .collect();
        }

        if hot_count == input.rows {
            return input
                .hashes
                .iter()
                .map(|hash| {
                    let partition = skew_partition(&input.layout, *hash, salt);
                    salt = (salt + 1) % input.layout.bucket_count;
                    partition
                })
                .collect();
        }

        (0..input.rows)
            .map(|row| {
                let hash = input.hashes[row];
                if hot_keys.get_bit(row) {
                    let partition = skew_partition(&input.layout, hash, salt);
                    salt = (salt + 1) % input.layout.bucket_count;
                    partition
                } else {
                    normal_partition(&input.layout, hash)
                }
            })
            .collect()
    }

    fn build_rows_binary_search(input: &BenchInput) -> Vec<Vec<u32>> {
        let mut partition_rows = vec![Vec::<u32>::new(); input.layout.scatter_size];
        for row in 0..input.rows {
            let hash = input.hashes[row];
            if is_hot_key(input, row) {
                for salt in 0..input.layout.bucket_count {
                    let target = skew_partition(&input.layout, hash, salt) as usize;
                    partition_rows[target].push(row as u32);
                }
            } else {
                let target = normal_partition(&input.layout, hash) as usize;
                partition_rows[target].push(row as u32);
            }
        }
        partition_rows
    }

    fn build_rows_bitmap(input: &BenchInput) -> Vec<Vec<u32>> {
        let hot_keys = hot_key_bitmap(input);
        let hot_count = hot_keys.true_count();
        let mut partition_rows = vec![Vec::<u32>::new(); input.layout.scatter_size];

        if hot_count == 0 {
            for row in 0..input.rows {
                let target = normal_partition(&input.layout, input.hashes[row]) as usize;
                partition_rows[target].push(row as u32);
            }
            return partition_rows;
        }

        if hot_count == input.rows {
            for row in 0..input.rows {
                let hash = input.hashes[row];
                for salt in 0..input.layout.bucket_count {
                    let target = skew_partition(&input.layout, hash, salt) as usize;
                    partition_rows[target].push(row as u32);
                }
            }
            return partition_rows;
        }

        for row in 0..input.rows {
            let hash = input.hashes[row];
            if hot_keys.get_bit(row) {
                for salt in 0..input.layout.bucket_count {
                    let target = skew_partition(&input.layout, hash, salt) as usize;
                    partition_rows[target].push(row as u32);
                }
            } else {
                let target = normal_partition(&input.layout, hash) as usize;
                partition_rows[target].push(row as u32);
            }
        }
        partition_rows
    }

    #[divan::bench(args = CASES)]
    fn probe_binary_search(bencher: divan::Bencher, case: HotKeyCase) {
        let input = BenchInput::new(case);
        bencher.bench_local(|| {
            divan::black_box(probe_indices_binary_search(divan::black_box(&input)));
        });
    }

    #[divan::bench(args = CASES)]
    fn probe_bitmap(bencher: divan::Bencher, case: HotKeyCase) {
        let input = BenchInput::new(case);
        bencher.bench_local(|| {
            divan::black_box(probe_indices_bitmap(divan::black_box(&input)));
        });
    }

    #[divan::bench(args = CASES)]
    fn build_binary_search(bencher: divan::Bencher, case: HotKeyCase) {
        let input = BenchInput::new(case);
        bencher.bench_local(|| {
            divan::black_box(build_rows_binary_search(divan::black_box(&input)));
        });
    }

    #[divan::bench(args = CASES)]
    fn build_bitmap(bencher: divan::Bencher, case: HotKeyCase) {
        let input = BenchInput::new(case);
        bencher.bench_local(|| {
            divan::black_box(build_rows_bitmap(divan::black_box(&input)));
        });
    }
}

fn generate_random_string_data(rng: &mut StdRng, length: usize) -> (Vec<String>, Vec<Vec<u8>>) {
    let iter_str: Vec<_> = (0..102400)
        .map(|_| {
            let random_string: String = (0..length)
                .map(|_| {
                    // Generate a random character (ASCII printable characters)
                    rng.gen_range(32..=126) as u8 as char
                })
                .collect();
            random_string
        })
        .collect();

    let iter_binary: Vec<_> = iter_str
        .iter()
        .map(|x| x.clone().as_bytes().to_vec())
        .collect();

    (iter_str, iter_binary)
}

fn generate_random_i128_data(rng: &mut StdRng, length: usize) -> (Buffer<i128>, Buffer<i128>) {
    let s: Buffer<i128> = (0..length).map(|_| rng.gen_range(-1000..1000)).collect();
    let b: Buffer<i128> = (0..length).map(|_| rng.gen_range(-1000..1000)).collect();
    (s, b)
}

/// Invokes `f` with values `0..len` collecting the boolean results into a new `BooleanBuffer`
///
/// This is similar to [`MutableBuffer::collect_bool`] but with
/// the option to efficiently negate the result
fn collect_bool(len: usize, neg: bool, f: impl Fn(usize) -> bool) -> BooleanBuffer {
    let mut buffer = arrow_buffer::MutableBuffer::new(arrow_buffer::bit_util::ceil(len, 64) * 8);

    let chunks = len / 64;
    let remainder = len % 64;
    for chunk in 0..chunks {
        let mut packed = 0;
        for bit_idx in 0..64 {
            let i = bit_idx + chunk * 64;
            packed |= (f(i) as u64) << bit_idx;
        }
        if neg {
            packed = !packed
        }

        // SAFETY: Already allocated sufficient capacity
        unsafe { buffer.push_unchecked(packed) }
    }

    if remainder != 0 {
        let mut packed = 0;
        for bit_idx in 0..remainder {
            let i = bit_idx + chunks * 64;
            packed |= (f(i) as u64) << bit_idx;
        }
        if neg {
            packed = !packed
        }

        // SAFETY: Already allocated sufficient capacity
        unsafe { buffer.push_unchecked(packed) }
    }
    BooleanBuffer::new(buffer.into(), 0, len)
}
