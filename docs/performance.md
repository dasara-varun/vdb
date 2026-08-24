# VDB Performance Plan

## Performance objective

VDB should feel fast for local application workloads without sacrificing correctness. The MVP optimizes the hot path for point reads, point writes, bounded filtered reads, and recovery. It does not claim to outperform every specialized database. Performance claims must be measured on documented hardware and workload fixtures.

## Language decision

The VDB core is implemented in Rust. Rust is a suitable choice for a storage engine because its ownership and type systems help catch many memory and concurrency errors at compile time while allowing native binaries without a garbage-collected runtime.[1] The trade-off is greater implementation complexity and a smaller beginner ecosystem than Python or JavaScript.

The CLI and future server can remain in Rust so the deployment is one native binary. Python, TypeScript, and other language clients should use the stable HTTP or FFI boundary rather than duplicating storage logic.

## Performance architecture

The MVP uses an append-only write-ahead log with length-prefixed CBOR records and an in-memory state index. The current prototype prioritizes a small, auditable code path. A production implementation should evolve toward segmented logs, snapshot compaction, secondary indexes, and bounded memory policies rather than growing an unbounded in-memory map.

| Hot path | MVP design | Future optimization |
|---|---|---|
| Point read | HashMap lookup after WAL replay | Sharded maps, zero-copy indexes, cache-aware document handles |
| Point write | Single write gate, CBOR encode, sync WAL append | Group commit, batched fsync, segmented WAL |
| Bounded filter | In-memory scan with explicit limit | Secondary indexes and query planner |
| Schema report | Sampled read and type aggregation | Incremental field statistics |
| Backup | WAL file copy plus SHA-256 manifest | Consistent snapshots and incremental backup segments |
| Concurrency | RwLock reads and serialized writes | Sharded locking, actor/write-log architecture, replica readers |

## Safety before speed

VDB must not trade away durability or authorization for a benchmark result. Any optimization that changes fsync behavior, ordering, concurrency, or snapshot consistency requires a fault-injection test and a documented recovery implication.

The default query limit, document-size limit, and backup verification behavior are product safety controls as well as performance controls. They prevent accidental unbounded work and keep latency predictable.

## Initial benchmark suite

The benchmark harness should measure p50, p95, and p99 latency, throughput, bytes written, WAL growth, and memory use for the following cases: 1,000 point writes; 1,000 point reads; 10,000 bounded filtered reads; mixed 80/20 read/write load; restart and replay time; and snapshot creation time. Each case should run in debug and release builds, with a documented CPU, RAM, disk, operating system, Rust version, and dataset shape.

## Targets for the local MVP

These are engineering targets rather than promises. On a modern developer laptop with small documents, the initial release should aim for sub-millisecond in-memory point reads, low single-digit millisecond local point-write latency including durability, restart replay proportional to WAL size, and bounded queries that do not exceed the configured result or scan budget. The actual README should report measured results only after the benchmark suite exists.

## Avoiding premature optimization

The project should not add lock-free structures, unsafe code, custom allocators, memory mapping, compression, or a custom binary format until profiling identifies a real bottleneck. Each optimization must include a baseline, a measured improvement, a complexity assessment, and a regression test.

## References

[1]: https://doc.rust-lang.org/book/ch16-00-concurrency.html "The Rust Programming Language - Fearless Concurrency"
