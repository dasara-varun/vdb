#![forbid(unsafe_code)]

use chrono::{DateTime, Utc};
use parking_lot::{Mutex, RwLock};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use thiserror::Error;

const MAX_WAL_RECORD_BYTES: usize = 64 * 1024 * 1024;

#[derive(Debug, Error)]
pub enum VdbError {
    #[error("invalid collection: {0}")]
    InvalidCollection(String),
    #[error("invalid document: {0}")]
    InvalidDocument(String),
    #[error("collection not found: {0}")]
    CollectionNotFound(String),
    #[error("document not found: {collection}/{document_id}")]
    DocumentNotFound {
        collection: String,
        document_id: String,
    },
    #[error(
        "version conflict on {collection}/{document_id}: expected {expected:?}, current {current}"
    )]
    VersionConflict {
        collection: String,
        document_id: String,
        expected: Option<u64>,
        current: u64,
    },
    #[error("query limit must be between 1 and 1000")]
    InvalidLimit,
    #[error("storage error: {0}")]
    Io(#[from] std::io::Error),
    #[error("serialization error: {0}")]
    Serialization(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Document {
    pub id: String,
    pub version: u64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub data: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum WalRecord {
    CreateCollection {
        name: String,
    },
    Put {
        collection: String,
        document: Document,
    },
    Delete {
        collection: String,
        document_id: String,
    },
}

#[derive(Debug, Clone, Serialize)]
pub struct Health {
    pub status: &'static str,
    pub collections: usize,
    pub documents: usize,
    pub payload_bytes: usize,
    pub wal_bytes: u64,
    pub max_document_bytes: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct SchemaReport {
    pub collection: String,
    pub sampled_documents: usize,
    pub fields: BTreeMap<String, Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupManifest {
    pub source: String,
    pub destination: String,
    pub sha256: String,
    pub bytes: u64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
struct State {
    collections: HashMap<String, HashMap<String, Document>>,
}

pub struct VdbStore {
    path: PathBuf,
    wal: Mutex<File>,
    state: RwLock<State>,
    write_gate: Mutex<()>,
    max_document_bytes: usize,
}

impl VdbStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, VdbError> {
        Self::open_with_limit(path, 1_048_576)
    }

    pub fn open_with_limit(
        path: impl AsRef<Path>,
        max_document_bytes: usize,
    ) -> Result<Self, VdbError> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let state = replay_wal(&path)?;
        let wal = OpenOptions::new()
            .create(true)
            .read(true)
            .append(true)
            .open(&path)?;
        Ok(Self {
            path,
            wal: Mutex::new(wal),
            state: RwLock::new(state),
            write_gate: Mutex::new(()),
            max_document_bytes,
        })
    }

    pub fn create_collection(&self, name: &str) -> Result<(), VdbError> {
        validate_collection(name)?;
        let _gate = self.write_gate.lock();
        if self.state.read().collections.contains_key(name) {
            return Ok(());
        }
        self.append(&WalRecord::CreateCollection {
            name: name.to_string(),
        })?;
        self.state
            .write()
            .collections
            .insert(name.to_string(), HashMap::new());
        Ok(())
    }

    pub fn list_collections(&self) -> Vec<String> {
        let mut names: Vec<_> = self.state.read().collections.keys().cloned().collect();
        names.sort();
        names
    }

    pub fn put(
        &self,
        collection: &str,
        document_id: impl Into<String>,
        data: Value,
        expected_version: Option<u64>,
    ) -> Result<Document, VdbError> {
        self.require_collection(collection)?;
        validate_document(&data, self.max_document_bytes)?;
        let document_id = document_id.into();
        if document_id.is_empty() || document_id.len() > 256 {
            return Err(VdbError::InvalidDocument(
                "document id must contain 1-256 bytes".to_string(),
            ));
        }
        let _gate = self.write_gate.lock();
        let current = self
            .state
            .read()
            .collections
            .get(collection)
            .and_then(|docs| docs.get(&document_id))
            .cloned();
        let current_version = current.as_ref().map_or(0, |doc| doc.version);
        if expected_version.is_some() && expected_version != Some(current_version) {
            return Err(VdbError::VersionConflict {
                collection: collection.to_string(),
                document_id,
                expected: expected_version,
                current: current_version,
            });
        }
        let now = Utc::now();
        let document = Document {
            id: document_id,
            version: current_version + 1,
            created_at: current.as_ref().map_or(now, |doc| doc.created_at),
            updated_at: now,
            data,
        };
        self.append(&WalRecord::Put {
            collection: collection.to_string(),
            document: document.clone(),
        })?;
        self.state
            .write()
            .collections
            .get_mut(collection)
            .expect("collection checked before write")
            .insert(document.id.clone(), document.clone());
        Ok(document)
    }

    pub fn get(&self, collection: &str, document_id: &str) -> Result<Document, VdbError> {
        self.require_collection(collection)?;
        self.state
            .read()
            .collections
            .get(collection)
            .and_then(|docs| docs.get(document_id))
            .cloned()
            .ok_or_else(|| VdbError::DocumentNotFound {
                collection: collection.to_string(),
                document_id: document_id.to_string(),
            })
    }

    pub fn query(
        &self,
        collection: &str,
        where_filter: Option<&Map<String, Value>>,
        limit: usize,
    ) -> Result<Vec<Document>, VdbError> {
        self.require_collection(collection)?;
        if !(1..=1000).contains(&limit) {
            return Err(VdbError::InvalidLimit);
        }
        let mut documents: Vec<_> = self
            .state
            .read()
            .collections
            .get(collection)
            .expect("collection checked before query")
            .values()
            .filter(|document| matches_filter(&document.data, where_filter))
            .cloned()
            .collect();
        documents.sort_by(|left, right| right.updated_at.cmp(&left.updated_at));
        documents.truncate(limit);
        Ok(documents)
    }

    pub fn delete(
        &self,
        collection: &str,
        document_id: &str,
        expected_version: Option<u64>,
    ) -> Result<(), VdbError> {
        self.require_collection(collection)?;
        let _gate = self.write_gate.lock();
        let current = self.get(collection, document_id)?;
        if expected_version.is_some() && expected_version != Some(current.version) {
            return Err(VdbError::VersionConflict {
                collection: collection.to_string(),
                document_id: document_id.to_string(),
                expected: expected_version,
                current: current.version,
            });
        }
        self.append(&WalRecord::Delete {
            collection: collection.to_string(),
            document_id: document_id.to_string(),
        })?;
        self.state
            .write()
            .collections
            .get_mut(collection)
            .expect("collection checked before delete")
            .remove(document_id);
        Ok(())
    }

    pub fn schema_report(&self, collection: &str, limit: usize) -> Result<SchemaReport, VdbError> {
        let documents = self.query(collection, None, limit.clamp(1, 1000))?;
        let mut fields: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for document in &documents {
            if let Value::Object(map) = &document.data {
                for (key, value) in map {
                    fields
                        .entry(key.clone())
                        .or_default()
                        .insert(json_type(value).to_string());
                }
            }
        }
        Ok(SchemaReport {
            collection: collection.to_string(),
            sampled_documents: documents.len(),
            fields: fields
                .into_iter()
                .map(|(key, types)| (key, types.into_iter().collect()))
                .collect(),
        })
    }

    pub fn health(&self) -> Health {
        let state = self.state.read();
        let documents = state.collections.values().map(HashMap::len).sum::<usize>();
        let payload_bytes = state
            .collections
            .values()
            .flat_map(HashMap::values)
            .filter_map(|document| serde_cbor::to_vec(&document.data).ok())
            .map(|payload| payload.len())
            .sum();
        Health {
            status: "healthy",
            collections: state.collections.len(),
            documents,
            payload_bytes,
            wal_bytes: fs::metadata(&self.path).map_or(0, |metadata| metadata.len()),
            max_document_bytes: self.max_document_bytes,
        }
    }

    pub fn backup(&self, destination: impl AsRef<Path>) -> Result<BackupManifest, VdbError> {
        let _gate = self.write_gate.lock();
        self.wal.lock().sync_data()?;
        let destination = destination.as_ref().to_path_buf();
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(&self.path, &destination)?;
        let bytes = fs::read(&destination)?;
        let digest = Sha256::digest(&bytes);
        let manifest = BackupManifest {
            source: self.path.display().to_string(),
            destination: destination.display().to_string(),
            sha256: format!("{digest:x}"),
            bytes: bytes.len() as u64,
            created_at: Utc::now(),
        };
        let manifest_path = PathBuf::from(format!("{}.manifest.json", destination.display()));
        fs::write(manifest_path, serde_json::to_vec_pretty(&manifest).unwrap())?;
        Ok(manifest)
    }

    pub fn verify_backup(destination: impl AsRef<Path>) -> Result<Health, VdbError> {
        let destination = destination.as_ref();
        let bytes = fs::read(destination)?;
        if bytes.len() < 4 {
            return Err(VdbError::Serialization(
                "backup is too small to contain a WAL record".to_string(),
            ));
        }
        let manifest_path = PathBuf::from(format!("{}.manifest.json", destination.display()));
        if manifest_path.exists() {
            let manifest: BackupManifest = serde_json::from_slice(&fs::read(manifest_path)?)
                .map_err(|error| VdbError::Serialization(error.to_string()))?;
            let digest = Sha256::digest(&bytes);
            if manifest.sha256 != format!("{digest:x}") || manifest.bytes != bytes.len() as u64 {
                return Err(VdbError::Serialization(
                    "backup checksum or size does not match manifest".to_string(),
                ));
            }
        }
        let restored = VdbStore::open(destination)?;
        Ok(restored.health())
    }

    fn require_collection(&self, name: &str) -> Result<(), VdbError> {
        validate_collection(name)?;
        if self.state.read().collections.contains_key(name) {
            Ok(())
        } else {
            Err(VdbError::CollectionNotFound(name.to_string()))
        }
    }

    fn append(&self, record: &WalRecord) -> Result<(), VdbError> {
        let payload = serde_cbor::to_vec(record)
            .map_err(|error| VdbError::Serialization(error.to_string()))?;
        if payload.len() > MAX_WAL_RECORD_BYTES {
            return Err(VdbError::InvalidDocument(
                "WAL record is too large".to_string(),
            ));
        }
        let length = (payload.len() as u32).to_le_bytes();
        let checksum = Sha256::digest(&payload);
        let mut wal = self.wal.lock();
        wal.write_all(&length)?;
        wal.write_all(&payload)?;
        wal.write_all(&checksum)?;
        wal.sync_data()?;
        Ok(())
    }
}

fn validate_collection(name: &str) -> Result<(), VdbError> {
    let valid = !name.is_empty()
        && name.len() <= 63
        && name.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
    if valid {
        Ok(())
    } else {
        Err(VdbError::InvalidCollection(name.to_string()))
    }
}

fn validate_document(data: &Value, max_bytes: usize) -> Result<(), VdbError> {
    let Value::Object(map) = data else {
        return Err(VdbError::InvalidDocument(
            "documents must be JSON-like objects".to_string(),
        ));
    };
    for reserved in ["_id", "_version", "_created_at", "_updated_at"] {
        if map.contains_key(reserved) {
            return Err(VdbError::InvalidDocument(format!(
                "reserved field is not allowed: {reserved}"
            )));
        }
    }
    let encoded =
        serde_cbor::to_vec(data).map_err(|error| VdbError::Serialization(error.to_string()))?;
    if encoded.len() > max_bytes {
        return Err(VdbError::InvalidDocument(format!(
            "document is {} bytes; maximum is {max_bytes}",
            encoded.len()
        )));
    }
    Ok(())
}

fn matches_filter(data: &Value, filter: Option<&Map<String, Value>>) -> bool {
    let Some(filter) = filter else { return true };
    let Value::Object(document) = data else {
        return false;
    };
    filter
        .iter()
        .all(|(key, expected)| document.get(key) == Some(expected))
}

fn json_type(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Number(number) if number.is_i64() || number.is_u64() => "integer",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

fn replay_wal(path: &Path) -> Result<State, VdbError> {
    let mut bytes = Vec::new();
    if path.exists() {
        File::open(path)?.read_to_end(&mut bytes)?;
    }
    let mut state = State {
        collections: HashMap::new(),
    };
    let mut offset = 0usize;
    let mut valid_end = 0usize;
    while offset + 4 <= bytes.len() {
        let length = u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap()) as usize;
        if length > MAX_WAL_RECORD_BYTES || offset + 4 + length + 32 > bytes.len() {
            break;
        }
        let payload = &bytes[offset + 4..offset + 4 + length];
        let expected_checksum = &bytes[offset + 4 + length..offset + 4 + length + 32];
        let actual_checksum = Sha256::digest(payload);
        if expected_checksum != actual_checksum.as_slice() {
            return Err(VdbError::Serialization(
                "WAL checksum mismatch; storage recovery is required".to_string(),
            ));
        }
        let record: WalRecord = serde_cbor::from_slice(payload)
            .map_err(|error| VdbError::Serialization(error.to_string()))?;
        apply_record(&mut state, record);
        offset += 4 + length + 32;
        valid_end = offset;
    }
    if valid_end < bytes.len() {
        let file = OpenOptions::new().write(true).open(path)?;
        file.set_len(valid_end as u64)?;
    }
    Ok(state)
}

fn apply_record(state: &mut State, record: WalRecord) {
    match record {
        WalRecord::CreateCollection { name } => {
            state.collections.entry(name).or_default();
        }
        WalRecord::Put {
            collection,
            document,
        } => {
            state
                .collections
                .entry(collection)
                .or_default()
                .insert(document.id.clone(), document);
        }
        WalRecord::Delete {
            collection,
            document_id,
        } => {
            if let Some(documents) = state.collections.get_mut(&collection) {
                documents.remove(&document_id);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn round_trip_survives_reopen() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("data.vdb");
        {
            let store = VdbStore::open(&path).unwrap();
            store.create_collection("users").unwrap();
            let document = serde_json::json!({"name": "Asha", "plan": "pro"});
            store.put("users", "u1", document.clone(), None).unwrap();
            assert_eq!(store.get("users", "u1").unwrap().data, document);
        }
        let store = VdbStore::open(&path).unwrap();
        assert_eq!(store.get("users", "u1").unwrap().version, 1);
    }

    #[test]
    fn version_conflicts_are_rejected() {
        let directory = tempdir().unwrap();
        let store = VdbStore::open(directory.path().join("data.vdb")).unwrap();
        store.create_collection("users").unwrap();
        store
            .put("users", "u1", serde_json::json!({"name": "Asha"}), None)
            .unwrap();
        store
            .put(
                "users",
                "u1",
                serde_json::json!({"name": "Asha", "plan": "pro"}),
                Some(1),
            )
            .unwrap();
        let error = store
            .put("users", "u1", serde_json::json!({"name": "bad"}), Some(1))
            .unwrap_err();
        assert!(matches!(
            error,
            VdbError::VersionConflict { current: 2, .. }
        ));
    }

    #[test]
    fn backup_can_be_verified_and_reopened() {
        let directory = tempdir().unwrap();
        let source = directory.path().join("data.vdb");
        let destination = directory.path().join("backup.vdb");
        let store = VdbStore::open(source).unwrap();
        store.create_collection("notes").unwrap();
        store
            .put("notes", "n1", serde_json::json!({"text": "hello"}), None)
            .unwrap();
        store.backup(&destination).unwrap();
        let health = VdbStore::verify_backup(&destination).unwrap();
        assert_eq!(health.collections, 1);
        assert_eq!(health.documents, 1);
    }

    #[test]
    fn query_and_schema_are_bounded() {
        let directory = tempdir().unwrap();
        let store = VdbStore::open(directory.path().join("data.vdb")).unwrap();
        store.create_collection("events").unwrap();
        store
            .put(
                "events",
                "e1",
                serde_json::json!({"kind": "login", "n": 1}),
                None,
            )
            .unwrap();
        store
            .put(
                "events",
                "e2",
                serde_json::json!({"kind": "logout", "n": "2"}),
                None,
            )
            .unwrap();
        let filter = serde_json::json!({"kind": "login"}).as_object().cloned();
        assert_eq!(store.query("events", filter.as_ref(), 10).unwrap().len(), 1);
        let schema = store.schema_report("events", 10).unwrap();
        assert_eq!(schema.fields["n"], vec!["integer", "string"]);
        assert!(matches!(
            store.query("events", None, 0),
            Err(VdbError::InvalidLimit)
        ));
    }
}
