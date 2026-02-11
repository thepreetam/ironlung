# IronLung Benchmark Matrix (Campaign C, Phase C4)

Performance results for I/O and allocator benchmarks across the distro matrix. Captured by CI; see `.github/workflows/distro-matrix.yml`.

## Benchmarks

- **I/O (io_bench)**: 10k iterations, 4K buffer, pipe read+write. Measures ops/s.
- **Alloc (alloc_bench)**: 10k iterations, 4 threads. Measures malloc/free throughput.

## Results Template

| Distro | Glibc | I/O baseline | I/O IronLung | Alloc baseline | Alloc IronLung |
|--------|-------|--------------|--------------|----------------|----------------|
| ubuntu2204 | 2.35 | (ops/s) | (ops/s) | (ops/s) | (ops/s) |
| debian12 | 2.36 | | | | |
| fedora40 | 2.38 | | | | |
| centos7 | 2.17 | | | | |
| alpine | musl | | | | |

## Outlier Notes

- **CentOS 7**: Older kernel; may show different behavior.
- **Alpine (musl)**: Non-glibc; results may vary.
- **Arch**: Rolling; values can shift between runs.

## Target

IronLung I/O overhead ≤ 1% vs baseline (see [BENCHMARKS.md](BENCHMARKS.md)).
