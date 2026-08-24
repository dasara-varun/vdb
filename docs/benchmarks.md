# VDB Benchmark Baseline

## Purpose

This document records the first repeatable benchmark for the Rust MVP. It exists to make performance measurable and to prevent unsupported speed claims.

## Environment

The benchmark was run in the Manus Ubuntu sandbox on 24 August 2026 using Rust 1.75.0, a release build, a temporary local filesystem, and 10,000 small documents. The exact host CPU, storage device, and contention profile were not isolated, so these numbers are an engineering baseline rather than a product guarantee.

## Command

```bash
cargo run -p vdb-core --example bench_local --release --offline
```

## Result

| Operation | Workload | Elapsed | Approximate throughput |
|---|---:|---:|---:|
| Durable point writes | 10,000 documents | 1.172268571 seconds | 8,530 operations/second |
| Point reads | 10,000 documents | 6.770153 milliseconds | 1,477,071 operations/second |

The resulting health report contained one collection, 10,000 documents, approximately 329,720 bytes of CBOR payload data, and approximately 2,138,640 bytes of WAL data after adding per-record checksums. The write result includes a synchronous WAL flush per record, which is intentionally conservative for durability.

## Interpretation

The benchmark demonstrates that the Rust prototype has a fast in-memory read path and a durable but intentionally serialized write path. It does not measure concurrent clients, large documents, filtered queries at scale, memory pressure, compaction, encryption overhead, replication, or crash recovery. Those measurements are required before any production performance claim.

## Next benchmark work

The next suite should include p50, p95, and p99 latency; concurrent readers; batched writes; restart replay time; WAL growth; snapshot time; document-size sensitivity; query selectivity; memory use; and fault-injection recovery. Each result must record the hardware, operating system, compiler, dataset shape, build profile, and database configuration.
