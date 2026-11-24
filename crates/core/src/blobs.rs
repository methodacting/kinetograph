//! The payloads of a WebGL recording (packages/cli/skills/kinetograph-markup/references/SCENE.md, "glsl"): each texture,
//! patch, face, upload and buffer names its bytes by their SHA-256, a file of that name in the recording's `blobs`
//! directory. A recording carries the names, never the bytes, so one read anywhere on the machine draws the same
//! pixels with no state from the frames before it.

use std::collections::{BTreeMap, HashMap};
use std::hash::Hash;
use std::path::Path;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Deserializer};

/// The name of a blob: the SHA-256 of its bytes, written `sha256:<64 lowercase hex>`.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlobRef([u8; 32]);

impl BlobRef {
    /// The ref written `text`, or why it is not one.
    pub fn parse(text: &str) -> Result<BlobRef, String> {
        let bad = || format!("a blob is named sha256:<64 lowercase hex digits>, not {text:?}");
        let hex = text.strip_prefix("sha256:").ok_or_else(bad)?;
        if hex.len() != 64 {
            return Err(bad());
        }
        let mut hash = [0u8; 32];
        for (i, pair) in hex.as_bytes().chunks(2).enumerate() {
            let digit = |c: u8| match c {
                b'0'..=b'9' => Some(c - b'0'),
                b'a'..=b'f' => Some(c - b'a' + 10),
                _ => None,
            };
            hash[i] = (digit(pair[0]).ok_or_else(bad)? << 4) | digit(pair[1]).ok_or_else(bad)?;
        }
        Ok(BlobRef(hash))
    }

    /// Its file's name in the blob directory: the hash, in hex.
    pub fn file_name(&self) -> String {
        self.0.iter().map(|b| format!("{b:02x}")).collect()
    }
}

impl std::fmt::Display for BlobRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "sha256:{}", self.file_name())
    }
}

impl std::fmt::Debug for BlobRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}

impl<'de> Deserialize<'de> for BlobRef {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<BlobRef, D::Error> {
        let text = String::deserialize(deserializer)?;
        BlobRef::parse(&text).map_err(serde::de::Error::custom)
    }
}

/// Values by key, each counted as some number of bytes; past the budget the least recently used go, all but the one
/// used last, which is in use.
pub struct ByteLru<K, V> {
    entries: HashMap<K, (V, u64, u64)>,
    /// The keys by when they were last used.
    order: BTreeMap<u64, K>,
    clock: u64,
    bytes: u64,
    budget: u64,
}

impl<K: Clone + Eq + Hash, V> ByteLru<K, V> {
    pub fn new(budget: u64) -> ByteLru<K, V> {
        ByteLru { entries: HashMap::new(), order: BTreeMap::new(), clock: 0, bytes: 0, budget }
    }

    pub fn get(&mut self, key: &K) -> Option<&V> {
        let (_, _, used) = self.entries.get_mut(key)?;
        self.order.remove(used);
        self.clock += 1;
        *used = self.clock;
        self.order.insert(self.clock, key.clone());
        self.entries.get(key).map(|(v, _, _)| v)
    }

    pub fn insert(&mut self, key: K, value: V, bytes: u64) {
        if let Some((_, old, used)) = self.entries.remove(&key) {
            self.order.remove(&used);
            self.bytes -= old;
        }
        self.clock += 1;
        self.order.insert(self.clock, key.clone());
        self.entries.insert(key, (value, bytes, self.clock));
        self.bytes += bytes;
        self.evict();
    }

    pub fn set_budget(&mut self, budget: u64) {
        self.budget = budget;
        self.evict();
    }

    fn evict(&mut self) {
        while self.bytes > self.budget && self.entries.len() > 1 {
            let Some((_, key)) = self.order.pop_first() else { break };
            if let Some((_, bytes, _)) = self.entries.remove(&key) {
                self.bytes -= bytes;
            }
        }
    }
}

/// How many bytes of blobs `Blobs` keeps read.
pub const BLOB_BUDGET: u64 = 512 << 20;

/// The bytes of the blobs read so far, by hash, shared by the recordings a renderer draws.
pub struct Blobs(Mutex<ByteLru<BlobRef, Arc<[u8]>>>);

impl Default for Blobs {
    fn default() -> Blobs {
        Blobs(Mutex::new(ByteLru::new(BLOB_BUDGET)))
    }
}

impl Blobs {
    /// The bytes of `blob`, read from `dir` the first time.
    pub fn read(&self, dir: Option<&Path>, blob: &BlobRef) -> Result<Arc<[u8]>, String> {
        if let Some(bytes) = self.lock().get(blob) {
            return Ok(bytes.clone());
        }
        let dir = dir.ok_or_else(|| format!("the recording names the blob {blob} and no blobs directory"))?;
        let path = dir.join(blob.file_name());
        let bytes: Arc<[u8]> = std::fs::read(&path)
            .map_err(|e| format!("cannot read the blob {blob} ({}): {e}", path.display()))?
            .into();
        self.lock().insert(*blob, bytes.clone(), bytes.len() as u64);
        Ok(bytes)
    }

    pub fn set_budget(&self, budget: u64) {
        self.lock().set_budget(budget);
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, ByteLru<BlobRef, Arc<[u8]>>> {
        self.0.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

/// Where a recording's files are: the pictures its `src`s name, relative to `base_dir`, and its blobs, in `blobs`
/// (read through `cache`).
#[derive(Clone, Copy)]
pub struct Files<'a> {
    pub base_dir: &'a Path,
    pub blobs: Option<&'a Path>,
    pub cache: &'a Blobs,
}

impl Files<'_> {
    pub fn read(&self, blob: &BlobRef) -> Result<Arc<[u8]>, String> {
        self.cache.read(self.blobs, blob)
    }
}
