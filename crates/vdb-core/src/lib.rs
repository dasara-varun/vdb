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
const MAX_IMPORT_LINE_BYTES: usize = 2 * 1024 * 1024;
const MAX_IMPORT_BYTES: usize = 64 * 1024 * 1024;
pub const DEFAULT_MAX_WAL_BYTES: u64 = 512 * 1024 * 1024;
pub const MAX_CONFIGURED_WAL_BYTES: u64 = 16 * 1024 * 1024 * 1024;
const DEFAULT_MAX_DOCUMENT_BYTES: usize = 1_048_576;
const MAX_CONFIGURED_DOCUMENT_BYTES: usize = 64 * 1024 * 1024;
const FILE_MAGIC: &[u8; 4] = b"VDB1";
const MIN_SUPPORTED_FORMAT_VERSION: u16 = 1;
const FORMAT_VERSION: u16 = 2;
const FILE_HEADER_LEN: usize = 6;

#[derive(Debug, Error)]
pub enum VdbError {
    #[error("invalid collection: {0}")]
    InvalidCollection(String),
    #[error("invalid document: {0}")]
    InvalidDocument(String),
    #[error("invalid database path: {0}")]
    InvalidPath(String),
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
    #[error("storage handle is unavailable")]
    StorageUnavailable,
    #[error("WAL storage quota exceeded: current {current} bytes + requested {requested} bytes > limit {limit} bytes")]
    StorageQuotaExceeded {
        current: u64,
        requested: u64,
        limit: u64,
    },
    #[error("serialization error: {0}")]
    Serialization(String),
    #[error("JSON Lines import exceeds the {max_bytes} byte batch limit")]
    ImportTooLarge { max_bytes: usize },
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
    SnapshotPut {
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
    pub max_wal_bytes: u64,
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
pub struct VdbOptions {
    pub max_document_bytes: usize,
    pub max_wal_bytes: u64,
}

impl Default for VdbOptions {
    fn default() -> Self {
        Self {
            max_document_bytes: DEFAULT_MAX_DOCUMENT_BYTES,
            max_wal_bytes: DEFAULT_MAX_WAL_BYTES,
        }
    }
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
    max_wal_bytes: u64,
}

impl VdbStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, VdbError> {
        Self::open_with_options(path, VdbOptions::default())
    }

    pub fn open_with_limit(
        path: impl AsRef<Path>,
        max_document_bytes: usize,
    ) -> Result<Self, VdbError> {
        Self::open_with_options(
            path,
            VdbOptions {
                max_document_bytes,
                ..VdbOptions::default()
            },
        )
    }

    pub fn open_with_options(
        path: impl AsRef<Path>,
        options: VdbOptions,
    ) -> Result<Self, VdbError> {
        if !(1..=MAX_CONFIGURED_DOCUMENT_BYTES).contains(&options.max_document_bytes) {
            return Err(VdbError::InvalidDocument(format!(
                "maximum document size must be between 1 and {MAX_CONFIGURED_DOCUMENT_BYTES} bytes"
            )));
        }
        if !(1..=MAX_CONFIGURED_WAL_BYTES).contains(&options.max_wal_bytes) {
            return Err(VdbError::InvalidDocument(format!(
                "maximum WAL size must be between 1 and {MAX_CONFIGURED_WAL_BYTES} bytes"
            )));
        }
        let max_document_bytes = options.max_document_bytes;
        let path = path.as_ref().to_path_buf();
        reject_symlink(&path)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let lock_path = PathBuf::from(format!("{}.lock", path.display()));
        let lock_file = acquire_instance_lock(&lock_path)?;
        if let Err(error) = ensure_header(&path).and_then(|_| restrict_file_permissions(&path)) {
            drop(lock_file);
            let _ = fs::remove_file(&lock_path);
            return Err(error);
        }
        // Replay uses the format-wide cap so a lower write-time limit cannot make existing data unreadable.
        let state = match replay_wal(&path, MAX_CONFIGURED_DOCUMENT_BYTES) {
            Ok(state) => state,
            Err(error) => {
                drop(lock_file);
                let _ = fs::remove_file(&lock_path);
                return Err(error);
            }
        };
        let mut wal_options = OpenOptions::new();
        wal_options.create(true).read(true).append(true);
        secure_create_options(&mut wal_options);
        let wal = match wal_options.open(&path) {
            Ok(wal) => wal,
            Err(error) => {
                drop(lock_file);
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
            max_wal_bytes: options.max_wal_bytes,
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
        validate_document_id(&document_id)?;
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
        let mut state = self.state.write();
        state
            .collections
            .get_mut(collection)
            .ok_or_else(|| VdbError::CollectionNotFound(collection.to_string()))?
            .insert(document.id.clone(), document.clone());
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
            .ok_or_else(|| VdbError::CollectionNotFound(collection.to_string()))?
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
            .ok_or_else(|| VdbError::CollectionNotFound(collection.to_string()))?;
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
            .ok_or_else(|| VdbError::CollectionNotFound(collection.to_string()))?
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
            max_wal_bytes: self.max_wal_bytes,
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
                .ok_or_else(|| VdbError::CollectionNotFound(collection.clone()))?
                .values()
                .cloned()
                .collect();
            documents.sort_by(|left, right| left.id.cmp(&right.id));
            records.extend(
                documents
                    .into_iter()
                    .map(|document| WalRecord::SnapshotPut {
                        collection: collection.clone(),
                        document,
                    }),
            );
        }
        let temporary_path = PathBuf::from(format!(
            "{}.compact.{}.tmp",
            self.path.display(),
            std::process::id()
        ));
        reject_new_output(&temporary_path)?;
        let mut temporary_options = OpenOptions::new();
        temporary_options.create_new(true).write(true);
        secure_create_options(&mut temporary_options);
        let mut temporary = temporary_options.open(&temporary_path)?;
        temporary.write_all(FILE_MAGIC)?;
        temporary.write_all(&FORMAT_VERSION.to_le_bytes())?;
        for record in records {
            write_wal_record(&mut temporary, &record)?;
        }
        temporary.sync_all()?;
        let compacted_bytes = temporary.metadata()?.len();
        if compacted_bytes > self.max_wal_bytes {
            drop(temporary);
            let _ = fs::remove_file(&temporary_path);
            return Err(VdbError::StorageQuotaExceeded {
                current: 0,
                requested: compacted_bytes,
                limit: self.max_wal_bytes,
            });
        }
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
        self.reject_managed_output_path(destination)?;
        reject_new_output(destination)?;
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut export_options = OpenOptions::new();
        export_options.create_new(true).write(true);
        secure_create_options(&mut export_options);
        let file = export_options.open(destination)?;
        let mut writer = BufWriter::new(file);
        let state = self.state.read();
        let mut count = 0usize;
        let mut collections: Vec<_> = state.collections.keys().cloned().collect();
        collections.sort();
        for collection in collections {
            let mut documents: Vec<_> = state
                .collections
                .get(&collection)
                .ok_or_else(|| VdbError::CollectionNotFound(collection.clone()))?
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
        drop(writer);
        restrict_file_permissions(destination)?;
        Ok(count)
    }

    pub fn import_jsonl(&self, source: impl AsRef<Path>) -> Result<usize, VdbError> {
        let records = read_export_records(source)?;
        if records.is_empty() {
            return Ok(0);
        }

        let _gate = self.write_gate.lock();
        let mut staged_state = self.state.read().clone();
        let mut wal_records = Vec::with_capacity(records.len() * 2);
        for record in &records {
            validate_collection(&record.collection)?;
            validate_document_id(&record.id)?;
            validate_document(&record.data, self.max_document_bytes)?;
            if !staged_state.collections.contains_key(&record.collection) {
                staged_state
                    .collections
                    .insert(record.collection.clone(), HashMap::new());
                wal_records.push(WalRecord::CreateCollection {
                    name: record.collection.clone(),
                });
            }
            let current = staged_state
                .collections
                .get(&record.collection)
                .and_then(|documents| documents.get(&record.id))
                .cloned();
            let now = Utc::now();
            let document = Document {
                id: record.id.clone(),
                version: current.as_ref().map_or(1, |previous| previous.version + 1),
                created_at: current.as_ref().map_or(now, |previous| previous.created_at),
                updated_at: now,
                data: record.data.clone(),
            };
            wal_records.push(WalRecord::Put {
                collection: record.collection.clone(),
                document: document.clone(),
            });
            let documents = staged_state
                .collections
                .get_mut(&record.collection)
                .ok_or_else(|| VdbError::CollectionNotFound(record.collection.clone()))?;
            documents.insert(document.id.clone(), document.clone());
            refresh_document_indexes(
                &mut staged_state,
                &record.collection,
                current.as_ref(),
                &document,
            );
        }

        let mut encoded_batch = Vec::new();
        for record in &wal_records {
            write_wal_record(&mut encoded_batch, record)?;
        }
        let mut wal = self.wal.lock();
        let wal = wal.as_mut().ok_or(VdbError::StorageUnavailable)?;
        let current_bytes = wal.metadata()?.len();
        let requested_bytes = encoded_batch.len() as u64;
        if current_bytes.saturating_add(requested_bytes) > self.max_wal_bytes {
            return Err(VdbError::StorageQuotaExceeded {
                current: current_bytes,
                requested: requested_bytes,
                limit: self.max_wal_bytes,
            });
        }
        if let Err(error) = wal.write_all(&encoded_batch).and_then(|_| wal.sync_all()) {
            let rollback_result = wal.set_len(current_bytes).and_then(|_| wal.sync_all());
            if let Err(rollback_error) = rollback_result {
                return Err(VdbError::Io(std::io::Error::new(
                    rollback_error.kind(),
                    format!("import failed ({error}); WAL rollback failed: {rollback_error}"),
                )));
            }
            return Err(error.into());
        }
        *self.state.write() = staged_state;
        Ok(records.len())
    }

    pub fn backup(&self, destination: impl AsRef<Path>) -> Result<BackupManifest, VdbError> {
        let _gate = self.write_gate.lock();
        self.wal
            .lock()
            .as_mut()
            .ok_or(VdbError::StorageUnavailable)?
            .sync_all()?;
        let destination = destination.as_ref().to_path_buf();
        self.reject_managed_output_path(&destination)?;
        reject_new_output(&destination)?;
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut source_file = File::open(&self.path)?;
        let mut destination_options = OpenOptions::new();
        destination_options.create_new(true).write(true);
        secure_create_options(&mut destination_options);
        let mut destination_file = destination_options.open(&destination)?;
        std::io::copy(&mut source_file, &mut destination_file)?;
        destination_file.sync_all()?;
        drop(destination_file);
        restrict_file_permissions(&destination)?;
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
        let manifest_bytes = serde_json::to_vec_pretty(&manifest)
            .map_err(|error| VdbError::Serialization(error.to_string()))?;
        write_secure_file(&manifest_path, &manifest_bytes)?;
        Ok(manifest)
    }

    pub fn verify_backup(destination: impl AsRef<Path>) -> Result<Health, VdbError> {
        let destination = destination.as_ref();
        reject_symlink(destination)?;
        let bytes = fs::read(destination)?;
        if bytes.len() < FILE_HEADER_LEN {
            return Err(VdbError::Serialization(
                "backup is too small to contain a WAL record".to_string(),
            ));
        }
        let manifest_path = PathBuf::from(format!("{}.manifest.json", destination.display()));
        reject_symlink(&manifest_path)?;
        let manifest: BackupManifest = serde_json::from_slice(&fs::read(&manifest_path)?)
            .map_err(|error| VdbError::Serialization(error.to_string()))?;
        let digest = Sha256::digest(&bytes);
        if manifest.sha256 != format!("{digest:x}") || manifest.bytes != bytes.len() as u64 {
            return Err(VdbError::Serialization(
                "backup checksum or size does not match manifest".to_string(),
            ));
        }
        let restored = VdbStore::open(destination)?;
        Ok(restored.health())
    }

    fn reject_managed_output_path(&self, destination: &Path) -> Result<(), VdbError> {
        if destination == self.path || destination == self.lock_path {
            return Err(VdbError::InvalidPath(format!(
                "output path is managed by the active database: {}",
                destination.display()
            )));
        }
        Ok(())
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
        let wal = wal.as_mut().ok_or(VdbError::StorageUnavailable)?;
        let encoded = encode_wal_record(record)?;
        let current = wal.metadata()?.len();
        let requested = encoded.len() as u64;
        if current.saturating_add(requested) > self.max_wal_bytes {
            return Err(VdbError::StorageQuotaExceeded {
                current,
                requested,
                limit: self.max_wal_bytes,
            });
        }
        wal.write_all(&encoded)?;
        wal.sync_all()?;
        Ok(())
    }
}

impl Drop for VdbStore {
    fn drop(&mut self) {
        let _ = self.lock_file.sync_all();
        let _ = fs::remove_file(&self.lock_path);
    }
}

#[cfg(unix)]
fn acquire_instance_lock(lock_path: &Path) -> Result<File, VdbError> {
    use rustix::fs::{flock, FlockOperation};

    reject_symlink(lock_path)?;
    let mut options = OpenOptions::new();
    options.read(true).write(true).create(true);
    secure_create_options(&mut options);
    let mut file = options.open(lock_path)?;
    if let Err(error) = flock(&file, FlockOperation::NonBlockingLockExclusive) {
        if error.kind() == std::io::ErrorKind::WouldBlock {
            return Err(VdbError::InstanceLocked(lock_path.to_path_buf()));
        }
        return Err(std::io::Error::from_raw_os_error(error.raw_os_error()).into());
    }
    file.set_len(0)?;
    writeln!(file, "pid={}", std::process::id())?;
    file.sync_all()?;
    Ok(file)
}

#[cfg(not(unix))]
fn acquire_instance_lock(lock_path: &Path) -> Result<File, VdbError> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    secure_create_options(&mut options);
    let mut file = match options.open(lock_path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            return Err(VdbError::InstanceLocked(lock_path.to_path_buf()));
        }
        Err(error) => return Err(error.into()),
    };
    writeln!(file, "pid={}", std::process::id())?;
    file.sync_all()?;
    Ok(file)
}

fn ensure_header(path: &Path) -> Result<(), VdbError> {
    if !path.exists() || fs::metadata(path)?.len() == 0 {
        let mut options = OpenOptions::new();
        options.create(true).append(true);
        secure_create_options(&mut options);
        let mut file = options.open(path)?;
        file.write_all(FILE_MAGIC)?;
        file.write_all(&FORMAT_VERSION.to_le_bytes())?;
        file.sync_all()?;
        return Ok(());
    }
    let mut file = File::open(path)?;
    let mut header = [0u8; FILE_HEADER_LEN];
    file.read_exact(&mut header)
        .map_err(|_| VdbError::UnsupportedFormat)?;
    let format_version = u16::from_le_bytes([header[4], header[5]]);
    if &header[..4] != FILE_MAGIC || !is_supported_format_version(format_version) {
        return Err(VdbError::UnsupportedFormat);
    }
    Ok(())
}

fn encode_wal_record(record: &WalRecord) -> Result<Vec<u8>, VdbError> {
    let payload =
        serde_cbor::to_vec(record).map_err(|error| VdbError::Serialization(error.to_string()))?;
    if payload.len() > MAX_WAL_RECORD_BYTES {
        return Err(VdbError::InvalidDocument(
            "WAL record is too large".to_string(),
        ));
    }
    let length = (payload.len() as u32).to_le_bytes();
    let checksum = Sha256::digest(&payload);
    let mut encoded = Vec::with_capacity(4 + payload.len() + checksum.len());
    encoded.extend_from_slice(&length);
    encoded.extend_from_slice(&payload);
    encoded.extend_from_slice(&checksum);
    Ok(encoded)
}

fn write_wal_record(writer: &mut impl Write, record: &WalRecord) -> Result<(), VdbError> {
    writer.write_all(&encode_wal_record(record)?)?;
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
    let value = value_at_path(data, field)?;
    if value.is_object() || value.is_array() {
        return None;
    }
    serde_json::to_string(value).ok()
}

fn value_at_path<'a>(data: &'a Value, field: &str) -> Option<&'a Value> {
    let mut value = data;
    for segment in field.split('.') {
        value = value.get(segment)?;
    }
    Some(value)
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

fn validate_document_id(document_id: &str) -> Result<(), VdbError> {
    if document_id.is_empty() || document_id.len() > 256 {
        return Err(VdbError::InvalidDocument(
            "document id must contain 1-256 bytes".to_string(),
        ));
    }
    Ok(())
}

fn read_bounded_line(
    reader: &mut impl BufRead,
    max_bytes: usize,
) -> Result<Option<Vec<u8>>, VdbError> {
    let mut line = Vec::new();
    let bytes_read = reader
        .take(max_bytes.saturating_add(1) as u64)
        .read_until(b'\n', &mut line)?;
    if bytes_read == 0 {
        return Ok(None);
    }
    if line.len() > max_bytes {
        return Err(VdbError::Serialization(format!(
            "JSON Lines record exceeds {max_bytes} bytes"
        )));
    }
    Ok(Some(line))
}

fn read_export_records(source: impl AsRef<Path>) -> Result<Vec<ExportRecord>, VdbError> {
    let file = File::open(source)?;
    let mut reader = BufReader::new(file);
    let mut records = Vec::new();
    let mut total_bytes = 0usize;
    let mut line_number = 0usize;
    while let Some(mut line) = read_bounded_line(&mut reader, MAX_IMPORT_LINE_BYTES)? {
        line_number += 1;
        total_bytes = total_bytes
            .checked_add(line.len())
            .ok_or(VdbError::ImportTooLarge {
                max_bytes: MAX_IMPORT_BYTES,
            })?;
        if total_bytes > MAX_IMPORT_BYTES {
            return Err(VdbError::ImportTooLarge {
                max_bytes: MAX_IMPORT_BYTES,
            });
        }
        if line.last() == Some(&b'\n') {
            line.pop();
        }
        if line.last() == Some(&b'\r') {
            line.pop();
        }
        if line.iter().all(|byte| byte.is_ascii_whitespace()) {
            continue;
        }
        let line = String::from_utf8(line).map_err(|error| {
            VdbError::Serialization(format!(
                "invalid UTF-8 in JSON Lines record {line_number}: {error}"
            ))
        })?;
        let record: ExportRecord = serde_json::from_str(&line).map_err(|error| {
            VdbError::Serialization(format!("invalid JSON Lines record {line_number}: {error}"))
        })?;
        records.push(record);
    }
    Ok(records)
}

fn reject_new_output(path: &Path) -> Result<(), VdbError> {
    match fs::symlink_metadata(path) {
        Ok(_) => Err(VdbError::InvalidPath(format!(
            "refusing to overwrite existing output path: {}",
            path.display()
        ))),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

fn reject_symlink(path: &Path) -> Result<(), VdbError> {
    if let Ok(metadata) = fs::symlink_metadata(path) {
        if metadata.file_type().is_symlink() {
            return Err(VdbError::InvalidPath(format!(
                "symbolic links are not accepted for database files: {}",
                path.display()
            )));
        }
    }
    Ok(())
}

fn restrict_file_permissions(path: &Path) -> Result<(), VdbError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(path)?.permissions();
        permissions.set_mode(0o600);
        fs::set_permissions(path, permissions)?;
    }
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}

fn write_secure_file(path: &Path, bytes: &[u8]) -> Result<(), VdbError> {
    reject_new_output(path)?;
    let mut options = OpenOptions::new();
    options.create_new(true).write(true);
    secure_create_options(&mut options);
    let mut file = options.open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    restrict_file_permissions(path)?;
    Ok(())
}

fn secure_create_options(options: &mut OpenOptions) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
}

fn is_supported_format_version(version: u16) -> bool {
    (MIN_SUPPORTED_FORMAT_VERSION..=FORMAT_VERSION).contains(&version)
}

fn validate_wal_record(record: &WalRecord, max_document_bytes: usize) -> Result<(), VdbError> {
    match record {
        WalRecord::CreateCollection { name } => validate_collection(name),
        WalRecord::Put {
            collection,
            document,
        }
        | WalRecord::SnapshotPut {
            collection,
            document,
        } => {
            validate_collection(collection)?;
            validate_document_id(&document.id)?;
            validate_document(&document.data, max_document_bytes)?;
            if document.version == 0 || document.created_at > document.updated_at {
                return Err(VdbError::Serialization(
                    "invalid document metadata in WAL".to_string(),
                ));
            }
            Ok(())
        }
        WalRecord::Delete {
            collection,
            document_id,
        } => {
            validate_collection(collection)?;
            validate_document_id(document_id)
        }
        WalRecord::CreateIndex { collection, field } => {
            validate_collection(collection)?;
            validate_field(field)
        }
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
    let Value::Object(_) = data else {
        return false;
    };
    filter
        .iter()
        .all(|(key, expected)| value_at_path(data, key) == Some(expected))
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

fn replay_wal(path: &Path, max_document_bytes: usize) -> Result<State, VdbError> {
    let file_length = fs::metadata(path)?.len();
    let mut reader = BufReader::new(File::open(path)?);
    let mut header = [0u8; FILE_HEADER_LEN];
    reader
        .read_exact(&mut header)
        .map_err(|_| VdbError::UnsupportedFormat)?;
    let format_version = u16::from_le_bytes([header[4], header[5]]);
    if &header[..4] != FILE_MAGIC || !is_supported_format_version(format_version) {
        return Err(VdbError::UnsupportedFormat);
    }
    let mut state = State {
        collections: HashMap::new(),
        index_fields: HashMap::new(),
        indexes: HashMap::new(),
    };
    let mut valid_end = FILE_HEADER_LEN as u64;
    loop {
        let mut length_bytes = [0u8; 4];
        match reader.read_exact(&mut length_bytes) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(error) => return Err(error.into()),
        }
        let length = u32::from_le_bytes(length_bytes) as usize;
        if length > MAX_WAL_RECORD_BYTES {
            return Err(VdbError::Serialization(format!(
                "WAL record length {length} exceeds the {MAX_WAL_RECORD_BYTES} byte limit"
            )));
        }
        let mut payload = vec![0u8; length];
        match reader.read_exact(&mut payload) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(error) => return Err(error.into()),
        }
        let mut expected_checksum = [0u8; 32];
        match reader.read_exact(&mut expected_checksum) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(error) => return Err(error.into()),
        }
        let actual_checksum = Sha256::digest(&payload);
        if expected_checksum != actual_checksum.as_slice() {
            return Err(VdbError::Serialization(
                "WAL checksum mismatch; storage recovery is required".to_string(),
            ));
        }
        let record: WalRecord = serde_cbor::from_slice(&payload)
            .map_err(|error| VdbError::Serialization(error.to_string()))?;
        validate_wal_record(&record, max_document_bytes)?;
        apply_record(&mut state, record, format_version)?;
        valid_end += (4 + length + 32) as u64;
    }
    if valid_end < file_length {
        let file = OpenOptions::new().write(true).open(path)?;
        file.set_len(valid_end)?;
        file.sync_all()?;
    }
    rebuild_indexes(&mut state);
    Ok(state)
}

fn apply_record(state: &mut State, record: WalRecord, format_version: u16) -> Result<(), VdbError> {
    match record {
        WalRecord::CreateCollection { name } => {
            state.collections.entry(name).or_default();
        }
        WalRecord::Put {
            collection,
            document,
        } => {
            let documents = state.collections.get_mut(&collection).ok_or_else(|| {
                VdbError::Serialization(format!(
                    "WAL put references missing collection: {collection}"
                ))
            })?;
            if let Some(previous) = documents.get(&document.id) {
                if document.version != previous.version.saturating_add(1) {
                    return Err(VdbError::Serialization(format!(
                        "invalid version sequence for {collection}/{}",
                        document.id
                    )));
                }
            } else if document.version != 1 {
                return Err(VdbError::Serialization(format!(
                    "first version for {collection}/{} must be 1",
                    document.id
                )));
            }
            documents.insert(document.id.clone(), document);
        }
        WalRecord::SnapshotPut {
            collection,
            document,
        } => {
            if format_version < 2 {
                return Err(VdbError::UnsupportedFormat);
            }
            let documents = state.collections.get_mut(&collection).ok_or_else(|| {
                VdbError::Serialization(format!(
                    "WAL snapshot references missing collection: {collection}"
                ))
            })?;
            if documents.contains_key(&document.id) {
                return Err(VdbError::Serialization(format!(
                    "WAL snapshot duplicates document: {collection}/{}",
                    document.id
                )));
            }
            documents.insert(document.id.clone(), document);
        }
        WalRecord::Delete {
            collection,
            document_id,
        } => {
            let documents = state.collections.get_mut(&collection).ok_or_else(|| {
                VdbError::Serialization(format!(
                    "WAL delete references missing collection: {collection}"
                ))
            })?;
            if documents.remove(&document_id).is_none() {
                return Err(VdbError::Serialization(format!(
                    "WAL delete references missing document: {collection}/{document_id}"
                )));
            }
        }
        WalRecord::CreateIndex { collection, field } => {
            if !state.collections.contains_key(&collection) {
                return Err(VdbError::Serialization(format!(
                    "WAL index references missing collection: {collection}"
                )));
            }
            state
                .index_fields
                .entry(collection)
                .or_default()
                .insert(field);
        }
    }
    Ok(())
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
    fn legacy_format_version_one_remains_readable() {
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
        bytes[4..6].copy_from_slice(&1u16.to_le_bytes());
        std::fs::write(&path, bytes).unwrap();
        let store = VdbStore::open(&path).unwrap();
        assert_eq!(store.get("users", "u1").unwrap().version, 1);
    }

    #[test]
    fn unsupported_format_version_is_rejected() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("data.vdb");
        let _store = VdbStore::open(&path).unwrap();
        drop(_store);
        let mut bytes = std::fs::read(&path).unwrap();
        bytes[4..6].copy_from_slice(&3u16.to_le_bytes());
        std::fs::write(&path, bytes).unwrap();
        let error = match VdbStore::open(&path) {
            Ok(_) => panic!("unsupported format version unexpectedly opened"),
            Err(error) => error,
        };
        assert!(matches!(error, VdbError::UnsupportedFormat));
    }

    #[test]
    fn backup_rejects_active_database_path() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("data.vdb");
        let store = VdbStore::open(&path).unwrap();
        let error = store.backup(&path).unwrap_err();
        assert!(matches!(error, VdbError::InvalidPath(_)));
    }

    #[test]
    fn output_files_are_not_overwritten() {
        let directory = tempdir().unwrap();
        let database = directory.path().join("data.vdb");
        let backup = directory.path().join("backup.vdb");
        let export = directory.path().join("records.jsonl");
        let store = VdbStore::open(database).unwrap();
        std::fs::write(&backup, b"existing backup").unwrap();
        std::fs::write(&export, b"existing export").unwrap();
        assert!(matches!(
            store.backup(&backup),
            Err(VdbError::InvalidPath(_))
        ));
        assert!(matches!(
            store.export_jsonl(&export),
            Err(VdbError::InvalidPath(_))
        ));
        assert_eq!(std::fs::read(&backup).unwrap(), b"existing backup");
        assert_eq!(std::fs::read(&export).unwrap(), b"existing export");
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
    fn backup_verification_requires_an_intact_manifest() {
        let directory = tempdir().unwrap();
        let source = directory.path().join("data.vdb");
        let destination = directory.path().join("backup.vdb");
        let manifest = PathBuf::from(format!("{}.manifest.json", destination.display()));
        let store = VdbStore::open(source).unwrap();
        store.create_collection("notes").unwrap();
        store
            .put("notes", "n1", serde_json::json!({"text": "hello"}), None)
            .unwrap();
        store.backup(&destination).unwrap();
        drop(store);

        let manifest_bytes = std::fs::read(&manifest).unwrap();
        std::fs::remove_file(&manifest).unwrap();
        assert!(VdbStore::verify_backup(&destination).is_err());

        std::fs::write(&manifest, manifest_bytes).unwrap();
        std::fs::write(&manifest, b"{}").unwrap();
        assert!(VdbStore::verify_backup(&destination).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn backup_verification_rejects_symlinked_backup_and_manifest() {
        use std::os::unix::fs::symlink;

        let directory = tempdir().unwrap();
        let source = directory.path().join("data.vdb");
        let destination = directory.path().join("backup.vdb");
        let destination_link = directory.path().join("backup-link.vdb");
        let manifest = PathBuf::from(format!("{}.manifest.json", destination.display()));
        let manifest_target = directory.path().join("manifest-copy.json");
        let store = VdbStore::open(source).unwrap();
        store.create_collection("notes").unwrap();
        store.backup(&destination).unwrap();
        drop(store);

        symlink(&destination, &destination_link).unwrap();
        assert!(matches!(
            VdbStore::verify_backup(&destination_link),
            Err(VdbError::InvalidPath(_))
        ));

        std::fs::copy(&manifest, &manifest_target).unwrap();
        std::fs::remove_file(&manifest).unwrap();
        symlink(&manifest_target, &manifest).unwrap();
        assert!(matches!(
            VdbStore::verify_backup(&destination),
            Err(VdbError::InvalidPath(_))
        ));
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
    fn configured_document_limit_is_bounded() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("data.vdb");
        let low = match VdbStore::open_with_limit(&path, 0) {
            Ok(_) => panic!("zero document limit unexpectedly accepted"),
            Err(error) => error,
        };
        assert!(matches!(low, VdbError::InvalidDocument(_)));
        let high = VdbStore::open_with_limit(&path, MAX_CONFIGURED_DOCUMENT_BYTES + 1);
        assert!(matches!(high, Err(VdbError::InvalidDocument(_))));
        let invalid_wal = VdbStore::open_with_options(
            &path,
            VdbOptions {
                max_wal_bytes: 0,
                ..VdbOptions::default()
            },
        );
        assert!(matches!(invalid_wal, Err(VdbError::InvalidDocument(_))));
        assert!(!path.exists());
    }

    #[test]
    fn wal_quota_rejects_write_without_partial_append() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("data.vdb");
        let store = VdbStore::open(&path).unwrap();
        store.create_collection("users").unwrap();
        let current_bytes = store.health().wal_bytes;
        drop(store);

        let constrained = VdbStore::open_with_options(
            &path,
            VdbOptions {
                max_wal_bytes: current_bytes + 1,
                ..VdbOptions::default()
            },
        )
        .unwrap();
        let error = constrained
            .put("users", "u1", serde_json::json!({"name": "Asha"}), None)
            .unwrap_err();
        assert!(matches!(error, VdbError::StorageQuotaExceeded { .. }));
        assert!(matches!(
            constrained.get("users", "u1"),
            Err(VdbError::DocumentNotFound { .. })
        ));
        assert_eq!(constrained.health().wal_bytes, current_bytes);
    }

    #[test]
    fn compaction_quota_failure_preserves_original_database() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("data.vdb");
        {
            let store = VdbStore::open(&path).unwrap();
            store.create_collection("users").unwrap();
            store
                .put("users", "u1", serde_json::json!({"name": "Asha"}), None)
                .unwrap();
        }

        let constrained = VdbStore::open_with_options(
            &path,
            VdbOptions {
                max_wal_bytes: FILE_HEADER_LEN as u64,
                ..VdbOptions::default()
            },
        )
        .unwrap();
        let error = constrained.compact().unwrap_err();
        assert!(matches!(error, VdbError::StorageQuotaExceeded { .. }));
        drop(constrained);

        let reopened = VdbStore::open(&path).unwrap();
        assert_eq!(
            reopened.get("users", "u1").unwrap().data,
            serde_json::json!({"name": "Asha"})
        );
        assert!(!PathBuf::from(format!(
            "{}.compact.{}.tmp",
            path.display(),
            std::process::id()
        ))
        .exists());
    }

    #[test]
    fn larger_configured_document_remains_readable_on_default_reopen() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("data.vdb");
        let data = serde_json::json!({"payload": "x".repeat(1_500_000)});
        {
            let store = VdbStore::open_with_limit(&path, 2 * 1024 * 1024).unwrap();
            store.create_collection("large").unwrap();
            store.put("large", "d1", data.clone(), None).unwrap();
        }
        let reopened = VdbStore::open(&path).unwrap();
        assert_eq!(reopened.get("large", "d1").unwrap().data, data);
    }

    #[test]
    fn truncated_trailing_record_recovers_valid_prefix() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("data.vdb");
        {
            let store = VdbStore::open(&path).unwrap();
            store.create_collection("users").unwrap();
            store
                .put("users", "u1", serde_json::json!({"name": "Asha"}), None)
                .unwrap();
        }

        let complete_length = std::fs::metadata(&path).unwrap().len();
        let mut bytes = std::fs::read(&path).unwrap();
        bytes.pop();
        std::fs::write(&path, bytes).unwrap();

        let reopened = VdbStore::open(&path).unwrap();
        assert_eq!(reopened.list_collections(), vec![String::from("users")]);
        assert!(matches!(
            reopened.get("users", "u1"),
            Err(VdbError::DocumentNotFound { .. })
        ));
        assert!(std::fs::metadata(&path).unwrap().len() < complete_length);
        assert_eq!(
            std::fs::metadata(&path).unwrap().len(),
            reopened.health().wal_bytes
        );
        drop(reopened);

        let reopened_again = VdbStore::open(&path).unwrap();
        assert_eq!(
            reopened_again.list_collections(),
            vec![String::from("users")]
        );
    }

    #[test]
    fn jsonl_import_reads_each_record_independently() {
        let directory = tempdir().unwrap();
        let database = directory.path().join("data.vdb");
        let source = directory.path().join("records.jsonl");
        let store = VdbStore::open(database).unwrap();
        std::fs::write(
            &source,
            b"{\"collection\":\"users\",\"id\":\"u1\",\"data\":{\"name\":\"Asha\"}}\n{\"collection\":\"users\",\"id\":\"u2\",\"data\":{\"name\":\"Lin\"}}\n",
        )
        .unwrap();

        assert_eq!(store.import_jsonl(&source).unwrap(), 2);
        assert_eq!(store.get("users", "u1").unwrap().data["name"], "Asha");
        assert_eq!(store.get("users", "u2").unwrap().data["name"], "Lin");
    }

    #[test]
    fn jsonl_import_is_atomic_on_invalid_record() {
        let directory = tempdir().unwrap();
        let database = directory.path().join("data.vdb");
        let source = directory.path().join("records.jsonl");
        let store = VdbStore::open(database).unwrap();
        std::fs::write(
            &source,
            b"{\"collection\":\"users\",\"id\":\"u1\",\"data\":{\"name\":\"Asha\"}}\nnot-json\n",
        )
        .unwrap();

        assert!(store.import_jsonl(&source).is_err());
        assert!(store.list_collections().is_empty());
        assert_eq!(store.health().wal_bytes, FILE_HEADER_LEN as u64);
    }

    #[test]
    fn jsonl_import_is_atomic_on_quota_failure() {
        let directory = tempdir().unwrap();
        let database = directory.path().join("data.vdb");
        let source = directory.path().join("records.jsonl");
        {
            let store = VdbStore::open(&database).unwrap();
            store.create_collection("users").unwrap();
        }
        let current_bytes = std::fs::metadata(&database).unwrap().len();
        let constrained = VdbStore::open_with_options(
            &database,
            VdbOptions {
                max_wal_bytes: current_bytes + 1,
                ..VdbOptions::default()
            },
        )
        .unwrap();
        std::fs::write(
            &source,
            b"{\"collection\":\"users\",\"id\":\"u1\",\"data\":{\"name\":\"Asha\"}}\n",
        )
        .unwrap();

        assert!(matches!(
            constrained.import_jsonl(&source),
            Err(VdbError::StorageQuotaExceeded { .. })
        ));
        assert!(matches!(
            constrained.get("users", "u1"),
            Err(VdbError::DocumentNotFound { .. })
        ));
        assert_eq!(constrained.health().wal_bytes, current_bytes);
    }

    #[test]
    fn oversized_jsonl_record_is_rejected_before_import() {
        let directory = tempdir().unwrap();
        let database = directory.path().join("data.vdb");
        let source = directory.path().join("records.jsonl");
        let store = VdbStore::open(database).unwrap();
        std::fs::write(&source, vec![b' '; MAX_IMPORT_LINE_BYTES + 1]).unwrap();
        let error = store.import_jsonl(&source).unwrap_err();
        assert!(error.to_string().contains("exceeds"));
        assert!(store.list_collections().is_empty());
    }

    #[test]
    fn every_truncated_final_record_boundary_recovers_valid_prefix() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("data.vdb");
        {
            let store = VdbStore::open(&path).unwrap();
            store.create_collection("users").unwrap();
            store
                .put("users", "u1", serde_json::json!({"name": "Asha"}), None)
                .unwrap();
        }
        let complete = std::fs::read(&path).unwrap();
        let collection_record = encode_wal_record(&WalRecord::CreateCollection {
            name: String::from("users"),
        })
        .unwrap();
        let prefix_length = FILE_HEADER_LEN + collection_record.len();
        assert!(prefix_length < complete.len());

        for cut in prefix_length..complete.len() {
            std::fs::write(&path, &complete[..cut]).unwrap();
            let reopened = VdbStore::open(path.clone()).unwrap();
            assert_eq!(reopened.list_collections(), vec![String::from("users")]);
            assert!(matches!(
                reopened.get("users", "u1"),
                Err(VdbError::DocumentNotFound { .. })
            ));
            drop(reopened);
            assert_eq!(
                std::fs::metadata(&path).unwrap().len(),
                prefix_length as u64
            );
        }
    }

    #[test]
    fn oversized_wal_length_fails_closed() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("data.vdb");
        {
            let store = VdbStore::open(&path).unwrap();
            drop(store);
        }
        let mut file = OpenOptions::new().append(true).open(&path).unwrap();
        file.write_all(&((MAX_WAL_RECORD_BYTES as u32) + 1).to_le_bytes())
            .unwrap();
        file.sync_all().unwrap();

        let error = match VdbStore::open(&path) {
            Ok(_) => panic!("oversized WAL length unexpectedly opened"),
            Err(error) => error,
        };
        assert!(error.to_string().contains("exceeds"));
        assert_eq!(
            std::fs::metadata(&path).unwrap().len(),
            (FILE_HEADER_LEN + 4) as u64
        );
    }

    #[test]
    fn invalid_wal_version_sequence_is_rejected() {
        let now = Utc::now();
        let mut state = State {
            collections: HashMap::from([(String::from("users"), HashMap::new())]),
            index_fields: HashMap::new(),
            indexes: HashMap::new(),
        };
        let record = WalRecord::Put {
            collection: String::from("users"),
            document: Document {
                id: String::from("u1"),
                version: 2,
                created_at: now,
                updated_at: now,
                data: serde_json::json!({"name": "Asha"}),
            },
        };
        let error = apply_record(&mut state, record, 1).unwrap_err();
        assert!(error.to_string().contains("must be 1"));

        let snapshot = WalRecord::SnapshotPut {
            collection: String::from("users"),
            document: Document {
                id: String::from("u1"),
                version: 1,
                created_at: now,
                updated_at: now,
                data: serde_json::json!({"name": "Asha"}),
            },
        };
        let mut snapshot_state = State {
            collections: HashMap::from([(String::from("users"), HashMap::new())]),
            index_fields: HashMap::new(),
            indexes: HashMap::new(),
        };
        apply_record(&mut snapshot_state, snapshot.clone(), 2).unwrap();
        let duplicate = apply_record(&mut snapshot_state, snapshot, 2).unwrap_err();
        assert!(duplicate.to_string().contains("duplicates document"));
    }

    #[cfg(unix)]
    #[test]
    fn database_and_lock_files_are_private_by_default() {
        use std::os::unix::fs::PermissionsExt;
        let directory = tempdir().unwrap();
        let path = directory.path().join("data.vdb");
        let store = VdbStore::open(&path).unwrap();
        assert_eq!(
            std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        let lock_path = PathBuf::from(format!("{}.lock", path.display()));
        assert_eq!(
            std::fs::metadata(lock_path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        let backup_path = directory.path().join("backup.vdb");
        let manifest_path = PathBuf::from(format!("{}.manifest.json", backup_path.display()));
        store.backup(&backup_path).unwrap();
        assert_eq!(
            std::fs::metadata(&backup_path)
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
        assert_eq!(
            std::fs::metadata(manifest_path)
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
        drop(store);
    }

    #[cfg(unix)]
    #[test]
    fn database_symlink_is_rejected() {
        use std::os::unix::fs::symlink;
        let directory = tempdir().unwrap();
        let target = directory.path().join("target.vdb");
        let path = directory.path().join("data.vdb");
        symlink(&target, &path).unwrap();
        let error = match VdbStore::open(&path) {
            Ok(_) => panic!("database symlink unexpectedly opened"),
            Err(error) => error,
        };
        assert!(matches!(error, VdbError::InvalidPath(_)));
        assert!(!target.exists());
    }

    #[cfg(not(unix))]
    #[test]
    fn locked_new_path_is_not_created() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("new.vdb");
        let lock_path = PathBuf::from(format!("{}.lock", path.display()));
        std::fs::write(lock_path, b"pid=someone-else\n").unwrap();
        let error = match VdbStore::open(&path) {
            Ok(_) => panic!("locked path unexpectedly opened"),
            Err(error) => error,
        };
        assert!(matches!(error, VdbError::InstanceLocked(_)));
        assert!(!path.exists());
    }

    #[cfg(unix)]
    #[test]
    fn stale_lock_file_does_not_block_unix_reopen() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("new.vdb");
        let lock_path = PathBuf::from(format!("{}.lock", path.display()));
        std::fs::write(&lock_path, b"pid=previous-process\n").unwrap();

        let store = VdbStore::open(&path).unwrap();
        assert!(path.exists());
        assert!(store.list_collections().is_empty());
        drop(store);
        assert!(!lock_path.exists());
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
        let inserted = reopened
            .put(
                "events",
                "e20",
                serde_json::json!({"kind": "logout", "n": 20}),
                None,
            )
            .unwrap();
        assert_eq!(inserted.version, 1);
        assert_eq!(reopened.query("events", None, 100).unwrap().len(), 21);
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
    fn nested_equality_query_matches_with_and_without_index() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("data.vdb");
        let store = VdbStore::open(path).unwrap();
        store.create_collection("users").unwrap();
        store
            .put(
                "users",
                "u1",
                serde_json::json!({"profile": {"plan": "pro"}}),
                None,
            )
            .unwrap();
        store
            .put(
                "users",
                "u2",
                serde_json::json!({"profile": {"plan": "free"}}),
                None,
            )
            .unwrap();

        let filter = serde_json::json!({"profile.plan": "pro"})
            .as_object()
            .cloned();
        assert_eq!(
            store.query("users", filter.as_ref(), 10).unwrap()[0].id,
            "u1"
        );

        store.create_index("users", "profile.plan").unwrap();
        assert_eq!(
            store.query("users", filter.as_ref(), 10).unwrap()[0].id,
            "u1"
        );
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
