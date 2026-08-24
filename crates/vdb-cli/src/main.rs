#![forbid(unsafe_code)]

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use serde_json::{Map, Value};
use std::path::PathBuf;
use vdb_core::{VdbOptions, VdbStore, DEFAULT_MAX_WAL_BYTES, MAX_CONFIGURED_WAL_BYTES};

#[derive(Debug, Parser)]
#[command(name = "vdb", version, about = "Fast, local-first document database")]
struct Cli {
    #[arg(long, default_value = "vdb.vdb", global = true)]
    path: PathBuf,
    #[arg(
        long,
        default_value_t = DEFAULT_MAX_WAL_BYTES,
        value_parser = clap::value_parser!(u64).range(1..=MAX_CONFIGURED_WAL_BYTES),
        global = true,
        help = "Maximum WAL size in bytes before new writes are rejected"
    )]
    max_wal_bytes: u64,
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Init,
    Collections {
        #[command(subcommand)]
        command: CollectionCommand,
    },
    Put {
        collection: String,
        id: String,
        document: String,
        #[arg(long)]
        expected_version: Option<u64>,
    },
    Get {
        collection: String,
        id: String,
    },
    Query {
        collection: String,
        #[arg(long, default_value = "{}")]
        where_json: String,
        #[arg(long, default_value_t = 100)]
        limit: usize,
    },
    Delete {
        collection: String,
        id: String,
        #[arg(long)]
        expected_version: Option<u64>,
    },
    Schema {
        collection: String,
        #[arg(long, default_value_t = 100)]
        sample_limit: usize,
    },
    IndexCreate {
        collection: String,
        field: String,
    },
    IndexList {
        collection: String,
    },
    Health,
    Steward {
        #[arg(long)]
        collection: Option<String>,
    },
    Backup {
        destination: PathBuf,
    },
    BackupVerify {
        destination: PathBuf,
    },
    Restore {
        source: PathBuf,
        destination: PathBuf,
    },
    Export {
        destination: PathBuf,
    },
    Import {
        source: PathBuf,
    },
    Compact,
}

#[derive(Debug, Subcommand)]
enum CollectionCommand {
    List,
    Create { name: String },
}

fn print_json<T: serde::Serialize>(value: &T) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let options = VdbOptions {
        max_wal_bytes: cli.max_wal_bytes,
        ..VdbOptions::default()
    };
    if matches!(cli.command, Command::Init) {
        let _store = VdbStore::open_with_options(&cli.path, options).context("initialize VDB")?;
        return print_json(&serde_json::json!({"status": "initialized", "path": cli.path}));
    }
    if let Command::Restore {
        source,
        destination,
    } = cli.command
    {
        let health = VdbStore::restore_backup(source, destination).context("restore backup")?;
        return print_json(&health);
    }

    let store = VdbStore::open_with_options(&cli.path, options).context("open VDB")?;
    match cli.command {
        Command::Init => unreachable!(),
        Command::Collections { command } => match command {
            CollectionCommand::List => print_json(&store.list_collections()),
            CollectionCommand::Create { name } => {
                store.create_collection(&name)?;
                print_json(&serde_json::json!({"created": name}))
            }
        },
        Command::Put {
            collection,
            id,
            document,
            expected_version,
        } => {
            let value: Value =
                serde_json::from_str(&document).context("document must be valid JSON")?;
            print_json(&store.put(&collection, id, value, expected_version)?)
        }
        Command::Get { collection, id } => print_json(&store.get(&collection, &id)?),
        Command::Query {
            collection,
            where_json,
            limit,
        } => {
            let value: Value =
                serde_json::from_str(&where_json).context("--where-json must be valid JSON")?;
            let map: Option<Map<String, Value>> = value.as_object().cloned();
            if value.is_object() || value.is_null() {
                print_json(&store.query(&collection, map.as_ref(), limit)?)
            } else {
                anyhow::bail!("--where-json must be a JSON object")
            }
        }
        Command::Delete {
            collection,
            id,
            expected_version,
        } => {
            store.delete(&collection, &id, expected_version)?;
            print_json(&serde_json::json!({"deleted": format!("{collection}/{id}")}))
        }
        Command::Schema {
            collection,
            sample_limit,
        } => print_json(&store.schema_report(&collection, sample_limit)?),
        Command::IndexCreate { collection, field } => {
            store.create_index(&collection, &field)?;
            print_json(&serde_json::json!({"created": format!("{collection}.{field}")}))
        }
        Command::IndexList { collection } => print_json(&store.list_indexes(&collection)?),
        Command::Health => print_json(&store.health()),
        Command::Steward { collection } => {
            let findings = if let Some(collection) = collection {
                let schema = store.schema_report(&collection, 100)?;
                let mixed = schema
                    .fields
                    .iter()
                    .filter(|(_, types)| types.len() > 1)
                    .count();
                serde_json::json!({
                    "mode": "observe",
                    "actions": [],
                    "health": store.health(),
                    "findings": if mixed > 0 { vec![serde_json::json!({
                        "kind": "SCHEMA_DRIFT",
                        "severity": "medium",
                        "collection": collection,
                        "evidence": format!("{mixed} field(s) have multiple observed types"),
                        "approval_required": true,
                    })] } else { Vec::new() },
                })
            } else {
                serde_json::json!({
                    "mode": "observe",
                    "actions": [],
                    "health": store.health(),
                    "findings": [],
                })
            };
            print_json(&findings)
        }
        Command::Backup { destination } => print_json(&store.backup(destination)?),
        Command::BackupVerify { destination } => print_json(&VdbStore::verify_backup(destination)?),
        Command::Restore { .. } => unreachable!(),
        Command::Export { destination } => {
            let count = store.export_jsonl(destination)?;
            print_json(&serde_json::json!({"exported_documents": count}))
        }
        Command::Import { source } => {
            let count = store.import_jsonl(source)?;
            print_json(&serde_json::json!({"imported_documents": count}))
        }
        Command::Compact => print_json(&serde_json::json!({"wal_bytes": store.compact()?})),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_custom_wal_quota() {
        let cli = Cli::try_parse_from([
            "vdb",
            "--path",
            "data.vdb",
            "--max-wal-bytes",
            "1048576",
            "health",
        ])
        .unwrap();
        assert_eq!(cli.path, PathBuf::from("data.vdb"));
        assert_eq!(cli.max_wal_bytes, 1_048_576);
        assert!(matches!(cli.command, Command::Health));
    }

    #[test]
    fn rejects_zero_wal_quota() {
        let result = Cli::try_parse_from(["vdb", "--max-wal-bytes", "0", "health"]);
        assert!(result.is_err());
    }

    #[test]
    fn rejects_wal_quota_above_supported_maximum() {
        let result = Cli::try_parse_from([
            "vdb",
            "--max-wal-bytes",
            &(MAX_CONFIGURED_WAL_BYTES + 1).to_string(),
            "health",
        ]);
        assert!(result.is_err());
    }

    #[test]
    fn parses_backup_command_and_destination() {
        let cli = Cli::try_parse_from(["vdb", "backup", "backups/data.vdb"]).unwrap();
        assert!(
            matches!(cli.command, Command::Backup { destination } if destination.as_path() == std::path::Path::new("backups/data.vdb"))
        );
    }

    #[test]
    fn parses_restore_command_and_paths() {
        let cli = Cli::try_parse_from(["vdb", "restore", "backup.vdb", "restored.vdb"]).unwrap();
        assert!(matches!(
            cli.command,
            Command::Restore { source, destination }
                if source == PathBuf::from("backup.vdb")
                    && destination == PathBuf::from("restored.vdb")
        ));
    }
}
