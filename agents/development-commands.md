# Build, Test, and Development Commands

- `make setup`: install cargo components plus linters such as `taplo`, `shfmt`, `typos`, `machete`, and `ruff`.
- `make build` / `make build-release`: compile debug or optimized `databend-{query,meta,metactl}` binaries into `target/`.
- `make run-debug` / `make kill`: launch or stop a local standalone deployment using the latest build.
- `make unit-test`, `make stateless-test`, `make sqllogic-test`, `make metactl-test`: run focused suites.
- `make test`: run the default CI test matrix.
- `make fmt`: apply repository formatting rules.
- `make lint`: run lint checks before committing.

## Fast Query Development Compile

- For the fastest feedback while working on planner, executor, expression, and most query-service code paths, prefer:
  `cargo check -p databend-query --lib --no-default-features --features simd --target-dir target/dev-query-min`
- This avoids the default storage feature set in `src/query/service/Cargo.toml` and skips building the runnable query binary when you only need type checking.
- Use a dedicated `--target-dir` for this workflow so the smaller feature graph stays warm across repeated checks.
- When you need a runnable OSS query binary, use:
  `cargo check -p databend-binaries --bin databend-query-oss --no-default-features --features simd --target-dir target/dev-query-oss`
- Expect the runnable binary path to be noticeably heavier than the library-only check, because it pulls in the binary entrypoint and more top-level dependencies.
- Add storage features only when your change needs them. Common examples:
  `--features "simd,storage-iceberg"`
  `--features "simd,storage-delta"`
  `--features "simd,storage-hive"`
- Avoid defaulting to `jemalloc` or the full storage feature set during inner-loop development unless the code you are changing depends on them directly.
- Cold builds are still expensive because some transitive dependencies, especially `protobuf-src`, perform local C++ builds. The commands above are mainly meant to minimize repeat-check cost and keep the active dependency graph as small as practical.
