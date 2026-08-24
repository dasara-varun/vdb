use serde_json::json;
use std::time::Instant;
use tempfile::tempdir;
use vdb_core::VdbStore;

fn main() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("bench.vdb");
    let store = VdbStore::open(path).expect("open store");
    store.create_collection("bench").expect("create collection");

    let writes = 10_000u64;
    let start = Instant::now();
    for index in 0..writes {
        store
            .put(
                "bench",
                format!("doc-{index}"),
                json!({"index": index, "kind": "benchmark", "active": true}),
                None,
            )
            .expect("write document");
    }
    let write_elapsed = start.elapsed();

    let start = Instant::now();
    for index in 0..writes {
        let _ = store
            .get("bench", &format!("doc-{index}"))
            .expect("read document");
    }
    let read_elapsed = start.elapsed();

    println!(
        "writes: {writes} in {:?} ({:.0} ops/s)",
        write_elapsed,
        writes as f64 / write_elapsed.as_secs_f64()
    );
    println!(
        "reads: {writes} in {:?} ({:.0} ops/s)",
        read_elapsed,
        writes as f64 / read_elapsed.as_secs_f64()
    );
    println!(
        "health: {}",
        serde_json::to_string_pretty(&store.health()).unwrap()
    );
}
