#![forbid(unsafe_code)]

use chrono::{DateTime, Utc};
use parking_lot::{Mutex, RwLock};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use thiserror::Error;

const MAX_WAL_RECORD_BYTES: usize = 64 * 1024 * 1024;
const FILE_MAGIC: &[u8; 4] = b"VDB1";
const FORMAT_VERSION: u16 = 1;
const FILE_HEADER_LEN: usize = 6;

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
    #[error("database instance is already locked: {0}")]
    InstanceLocked(PathBuf),
    #[error("unsupported or corrupt VDB file format")]
    UnsupportedFormat,
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
pub struct ExportRecord {
    pub collection: String,
    pub id: String,
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
    CreateIndex {
        collection: String,
        field: String,
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

#[derive(Debug, Clone, Serialize)]
pub struct IndexInfo {
    pub collection: String,
    pub field: String,
    pub indexed_values: usize,
    pub indexed_documents: usize,
}

#[derive(Debug, Clone)]
struct State {
    collections: HashMap<String, HashMap<String, Document>>,
    index_fields: HashMap<String, BTreeSet<String>>,
    indexes: HashMap<String, HashMap<String, HashMap<String, BTreeSet<String>>>>,
}

pub struct VdbStore {
    path: PathBuf,
    lock_path: PathBuf,
    lock_file: File,
    wal: Mutex<Option<File>>,
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
        let lock_path = PathBuf::from(format!("{}.lock", path.display()));
        let lock_file = match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&lock_path)
        {
            Ok(mut file) => {
                writeln!(file, "pid={}", std::process::id())?;
                file.sync_data()?;
                file
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                return Err(VdbError::InstanceLocked(lock_path));
            }
            Err(error) => return Err(error.into()),
        };
        if let Err(error) = ensure_header(&path) {
            let _ = fs::remove_file(&lock_path);
            return Err(error);
        }
        let state = match replay_wal(&path) {
            Ok(state) => state,
            Err(error) => {
                let _ = fs::remove_file(&lock_path);
                return Err(error);
            }
        };
        let wal = match OpenOptions::new()
            .create(true)
            .read(true)
            .append(true)
            .open(&path)
        {
            Ok(wal) => wal,
            Err(error) => {
                let _ = fs::remove_file(&lock_path);
                return Err(error.into());
            }
        };
        Ok(Self {
            path,
            lock_path,
            lock_file,
            wal: Mutex::new(Some(wal)),
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
        let mut state = self.state.write();
        refresh_document_indexes(&mut state, collection, current.as_ref(), &document);
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

    pub fn create_index(&self, collection: &str, field: &str) -> Result<(), VdbError> {
        self.require_collection(collection)?;
        validate_field(field)?;
        let _gate = self.write_gate.lock();
        if self
            .state
            .read()
            .index_fields
            .get(collection)
            .is_some_and(|fields| fields.contains(field))
        {
            return Ok(());
        }
        self.append(&WalRecord::CreateIndex {
            collection: collection.to_string(),
            field: field.to_string(),
        })?;
        let mut state = self.state.write();
        state
            .index_fields
            .entry(collection.to_string())
            .or_default()
            .insert(field.to_string());
        let documents: Vec<_> = state
            .collections
            .get(collection)
            .expect("collection checked before index")
            .values()
            .cloned()
            .collect();
        let index = state
            .indexes
            .entry(collection.to_string())
            .or_default()
            .entry(field.to_string())
            .or_default();
        for document in documents {
            if let Some(key) = index_key(&document.data, field) {
                index.entry(key).or_default().insert(document.id);
            }
        }
        Ok(())
    }

    pub fn list_indexes(&self, collection: &str) -> Result<Vec<IndexInfo>, VdbError> {
        self.require_collection(collection)?;
        let state = self.state.read();
        let mut result = Vec::new();
        if let Some(indexes) = state.indexes.get(collection) {
            for (field, values) in indexes {
                result.push(IndexInfo {
                    collection: collection.to_string(),
                    field: field.clone(),
                    indexed_values: values.len(),
                    indexed_documents: values.values().map(BTreeSet::len).sum(),
                });
            }
        }
        result.sort_by(|left, right| left.field.cmp(&right.field));
        Ok(result)
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
        let state = self.state.read();
        let collection_documents = state
            .collections
            .get(collection)
            .expect("collection checked before query");
        let candidate_ids = where_filter.and_then(|filter| {
            state.indexes.get(collection).and_then(|indexes| {
                filter.iter().find_map(|(field, expected)| {
                    let key = serde_json::to_string(expected).ok()?;
                    indexes.get(field)?.get(&key).cloned()
                })
            })
        });
        let mut documents: Vec<_> = match candidate_ids {
            Some(ids) => ids
                .iter()
                .filter_map(|id| collection_documents.get(id))
                .filter(|document| matches_filter(&document.data, where_filter))
                .cloned()
                .collect(),
            None => collection_documents
                .values()
                .filter(|document| matches_filter(&document.data, where_filter))
                .cloned()
                .collect(),
        };
        documents.sort_by_key(|document| std::cmp::Reverse(document.updated_at));
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
        let mut state = self.state.write();
        remove_document_from_indexes(&mut state, collection, &current);
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

    pub fn compact(&self) -> Result<u64, VdbError> {
        let _gate = self.write_gate.lock();
        let state = self.state.read().clone();
        let mut records = Vec::new();
        let mut collections: Vec<_> = state.collections.keys().cloned().collect();
        collections.sort();
        for collection in collections {
            records.push(WalRecord::CreateCollection {
                name: collection.clone(),
            });
            if let Some(fields) = state.index_fields.get(&collection) {
                for field in fields {
                    records.push(WalRecord::CreateIndex {
                        collection: collection.clone(),
                        field: field.clone(),
                    });
                }
            }
            let mut documents: Vec<_> = state
                .collections
                .get(&collection)
                .expect("collection exists")
                .values()
                .cloned()
                .collect();
            documents.sort_by(|left, right| left.id.cmp(&right.id));
            records.extend(documents.into_iter().map(|document| WalRecord::Put {
                collection: collection.clone(),
                document,
            }));
        }
        let temporary_path = PathBuf::from(format!(
            "{}.compact.{}.tmp",
            self.path.display(),
            std::process::id()
        ));
        let mut temporary = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&temporary_path)?;
        temporary.write_all(FILE_MAGIC)?;
        temporary.write_all(&FORMAT_VERSION.to_le_bytes())?;
        for record in records {
            write_wal_record(&mut temporary, &record)?;
        }
        temporary.sync_data()?;
        drop(temporary);

        let mut wal = self.wal.lock();
        let old_wal = wal.take();
        drop(old_wal);
        if let Err(error) = fs::rename(&temporary_path, &self.path) {
            *wal = Some(
                OpenOptions::new()
                    .read(true)
                    .append(true)
                    .open(&self.path)?,
            );
            return Err(error.into());
        }
        let new_wal = OpenOptions::new()
            .read(true)
            .append(true)
            .open(&self.path)?;
        let bytes = new_wal.metadata()?.len();
        *wal = Some(new_wal);
        Ok(bytes)
    }

    pub fn export_jsonl(&self, destination: impl AsRef<Path>) -> Result<usize, VdbError> {
        let destination = destination.as_ref();
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        let file = File::create(destination)?;
        let mut writer = BufWriter::new(file);
        let state = self.state.read();
        let mut count = 0usize;
        let mut collections: Vec<_> = state.collections.keys().cloned().collect();
        collections.sort();
        for collection in collections {
            let mut documents: Vec<_> = state
                .collections
                .get(&collection)
                .expect("collection exists")
                .values()
                .cloned()
                .collect();
            documents.sort_by(|left, right| left.id.cmp(&right.id));
            for document in documents {
                let record = ExportRecord {
                    collection: collection.clone(),
                    id: document.id,
                    data: document.data,
                };
                serde_json::to_writer(&mut writer, &record)
                    .map_err(|error| VdbError::Serialization(error.to_string()))?;
                writer.write_all(b"\n")?;
                count += 1;
            }
        }
        writer.flush()?;
        Ok(count)
    }

    pub fn import_jsonl(&self, source: impl AsRef<Path>) -> Result<usize, VdbError> {
        let file = File::open(source)?;
        let reader = BufReader::new(file);
        let mut count = 0usize;
        for (line_number, line) in reader.lines().enumerate() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let record: ExportRecord = serde_json::from_str(&line).map_err(|error| {
                VdbError::Serialization(format!(
                    "invalid JSON Lines record {}: {error}",
                    line_number + 1
                ))
            })?;
            self.create_collection(&record.collection)?;
            self.put(&record.collection, record.id, record.data, None)?;
            count += 1;
        }
        Ok(count)
    }

    pub fn backup(&self, destination: impl AsRef<Path>) -> Result<BackupManifest, VdbError> {
        let _gate = self.write_gate.lock();
        self.wal
            .lock()
            .as_mut()
            .expect("WAL handle is available while store is open")
            .sync_data()?;
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
        if bytes.len() < FILE_HEADER_LEN {
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
        let mut wal = self.wal.lock();
        let wal = wal
            .as_mut()
            .expect("WAL handle is available while store is open");
        write_wal_record(wal, record)?;
        wal.sync_data()?;
        Ok(())
    }
}

impl Drop for VdbStore {
    fn drop(&mut self) {
        let _ = self.lock_file.sync_data();
        let _ = fs::remove_file(&self.lock_path);
    }
}

fn ensure_header(path: &Path) -> Result<(), VdbError> {
    if !path.exists() || fs::metadata(path)?.len() == 0 {
        let mut file = OpenOptions::new().create(true).append(true).open(path)?;
        file.write_all(FILE_MAGIC)?;
        file.write_all(&FORMAT_VERSION.to_le_bytes())?;
        file.sync_data()?;
        return Ok(());
    }
    let mut file = File::open(path)?;
    let mut header = [0u8; FILE_HEADER_LEN];
    file.read_exact(&mut header)
        .map_err(|_| VdbError::UnsupportedFormat)?;
    if &header[..4] != FILE_MAGIC || u16::from_le_bytes([header[4], header[5]]) != FORMAT_VERSION {
        return Err(VdbError::UnsupportedFormat);
    }
    Ok(())
}

fn write_wal_record(writer: &mut impl Write, record: &WalRecord) -> Result<(), VdbError> {
    let payload =
        serde_cbor::to_vec(record).map_err(|error| VdbError::Serialization(error.to_string()))?;
    if payload.len() > MAX_WAL_RECORD_BYTES {
        return Err(VdbError::InvalidDocument(
            "WAL record is too large".to_string(),
        ));
    }
    let length = (payload.len() as u32).to_le_bytes();
    let checksum = Sha256::digest(&payload);
    writer.write_all(&length)?;
    writer.write_all(&payload)?;
    writer.write_all(&checksum)?;
    Ok(())
}

fn validate_field(field: &str) -> Result<(), VdbError> {
    if field.is_empty()
        || field.len() > 128
        || !field.chars().all(|character| {
            character.is_ascii_alphanumeric() || character == '_' || character == '.'
        })
    {
        return Err(VdbError::InvalidDocument(format!(
            "index field must contain only letters, numbers, underscores, or dots: {field}"
        )));
    }
    Ok(())
}

fn index_key(data: &Value, field: &str) -> Option<String> {
    let mut value = data;
    for segment in field.split('.') {
        value = value.get(segment)?;
    }
    if value.is_object() || value.is_array() {
        return None;
    }
    serde_json::to_string(value).ok()
}

fn remove_document_from_indexes(state: &mut State, collection: &str, document: &Document) {
    if let Some(indexes) = state.indexes.get_mut(collection) {
        for (field, values) in indexes {
            if let Some(key) = index_key(&document.data, field) {
                if let Some(ids) = values.get_mut(&key) {
                    ids.remove(&document.id);
                    if ids.is_empty() {
                        values.remove(&key);
                    }
                }
            }
        }
    }
}

fn refresh_document_indexes(
    state: &mut State,
    collection: &str,
    previous: Option<&Document>,
    current: &Document,
) {
    if let Some(previous) = previous {
        remove_document_from_indexes(state, collection, previous);
    }
    if let Some(indexes) = state.indexes.get_mut(collection) {
        for (field, values) in indexes {
            if let Some(key) = index_key(&current.data, field) {
                values.entry(key).or_default().insert(current.id.clone());
            }
        }
    }
}

fn rebuild_indexes(state: &mut State) {
    let fields_by_collection = state.index_fields.clone();
    for (collection, fields) in fields_by_collection {
        let mut indexes = HashMap::new();
        for field in fields {
            let mut values: HashMap<String, BTreeSet<String>> = HashMap::new();
            if let Some(documents) = state.collections.get(&collection) {
                for document in documents.values() {
                    if let Some(key) = index_key(&document.data, &field) {
                        values.entry(key).or_default().insert(document.id.clone());
                    }
                }
            }
            indexes.insert(field, values);
        }
        state.indexes.insert(collection, indexes);
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
        index_fields: HashMap::new(),
        indexes: HashMap::new(),
    };
    if bytes.len() < FILE_HEADER_LEN
        || &bytes[..4] != FILE_MAGIC
        || u16::from_le_bytes([bytes[4], bytes[5]]) != FORMAT_VERSION
    {
        return Err(VdbError::UnsupportedFormat);
    }
    let mut offset = FILE_HEADER_LEN;
    let mut valid_end = FILE_HEADER_LEN;
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
    rebuild_indexes(&mut state);
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
        WalRecord::CreateIndex { collection, field } => {
            state
                .index_fields
                .entry(collection)
                .or_default()
                .insert(field);
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
    fn process_lock_prevents_concurrent_instances() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("data.vdb");
        let first = VdbStore::open(&path).unwrap();
        let error = match VdbStore::open(&path) {
            Ok(_) => panic!("second instance unexpectedly opened"),
            Err(error) => error,
        };
        assert!(matches!(error, VdbError::InstanceLocked(_)));
        drop(first);
        assert!(VdbStore::open(&path).is_ok());
    }

    #[test]
    fn checksum_mismatch_requires_recovery() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("data.vdb");
        {
            let store = VdbStore::open(&path).unwrap();
            store.create_collection("users").unwrap();
            store
                .put("users", "u1", serde_json::json!({"name": "Asha"}), None)
                .unwrap();
        }
        let mut bytes = std::fs::read(&path).unwrap();
        let payload_offset = FILE_HEADER_LEN + 4 + 1;
        bytes[payload_offset] ^= 0x01;
        std::fs::write(&path, bytes).unwrap();
        let error = match VdbStore::open(&path) {
            Ok(_) => panic!("tampered WAL unexpectedly opened"),
            Err(error) => error,
        };
        assert!(error.to_string().contains("checksum mismatch"));
    }

    #[test]
    fn compaction_preserves_data_and_indexes() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("data.vdb");
        let store = VdbStore::open(&path).unwrap();
        store.create_collection("events").unwrap();
        store.create_index("events", "kind").unwrap();
        for index in 0..20 {
            let id = format!("e{index}");
            store
                .put(
                    "events",
                    id.clone(),
                    serde_json::json!({"kind": "login", "n": index}),
                    None,
                )
                .unwrap();
            store
                .put(
                    "events",
                    id,
                    serde_json::json!({"kind": "login", "n": index + 100}),
                    Some(1),
                )
                .unwrap();
        }
        let before = store.health().wal_bytes;
        let after = store.compact().unwrap();
        assert!(after < before);
        assert!(!PathBuf::from(format!(
            "{}.compact.{}.tmp",
            path.display(),
            std::process::id()
        ))
        .exists());
        drop(store);
        let reopened = VdbStore::open(&path).unwrap();
        let filter = serde_json::json!({"kind": "login"}).as_object().cloned();
        assert_eq!(
            reopened
                .query("events", filter.as_ref(), 100)
                .unwrap()
                .len(),
            20
        );
        assert_eq!(
            reopened.list_indexes("events").unwrap()[0].indexed_documents,
            20
        );
    }

    #[test]
    fn equality_index_accelerates_queries_and_survives_reopen() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("data.vdb");
        {
            let store = VdbStore::open(&path).unwrap();
            store.create_collection("users").unwrap();
            store
                .put("users", "u1", serde_json::json!({"plan": "pro"}), None)
                .unwrap();
            store
                .put("users", "u2", serde_json::json!({"plan": "free"}), None)
                .unwrap();
            store.create_index("users", "plan").unwrap();
            let filter = serde_json::json!({"plan": "pro"}).as_object().cloned();
            assert_eq!(store.query("users", filter.as_ref(), 10).unwrap().len(), 1);
            assert_eq!(store.list_indexes("users").unwrap()[0].field, "plan");
        }
        let store = VdbStore::open(&path).unwrap();
        let filter = serde_json::json!({"plan": "free"}).as_object().cloned();
        assert_eq!(
            store.query("users", filter.as_ref(), 10).unwrap()[0].id,
            "u2"
        );
        assert_eq!(store.list_indexes("users").unwrap()[0].indexed_documents, 2);
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
