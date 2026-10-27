use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{Read, Write},
    path::{Component, Path, PathBuf},
};

pub const MAX_INPUT: usize = 256 * 1024;
pub const MAX_LINE: usize = 64 * 1024;
pub const MAX_LINES: usize = 4096;
pub const MAX_STORE: u64 = 4 * 1024 * 1024;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Queue {
    pub schema: u8,
    pub revision: u64,
    pub last_wall_ms: u64,
    pub original: Option<String>,
    pub active: bool,
    pub next: usize,
    pub produced: Option<String>,
    pub owner: Option<String>,
    pub pending: Option<Pending>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pending {
    pub text: String,
    pub index: usize,
}

pub fn split(text: &str) -> Result<Vec<String>, String> {
    if text.len() > MAX_INPUT {
        return Err(
            "Clipboard exceeds 256 KiB; copy a smaller list. Existing queue retained.".into(),
        );
    }
    // Strip CR only when immediately followed by LF (a final lone CR is data).
    let mut parts: Vec<String> = text.split('\n').map(str::to_owned).collect();
    let end = parts.len() - 1;
    for part in &mut parts[..end] {
        if part.ends_with('\r') {
            part.pop();
        }
    }
    if parts.len() > MAX_LINES || parts.iter().any(|s| s.len() > MAX_LINE) {
        return Err(
            "Clipboard exceeds 4096 lines or 64 KiB per line; existing queue retained.".into(),
        );
    }
    Ok(parts)
}
impl Queue {
    pub fn fresh() -> Self {
        Self {
            schema: 1,
            ..Self::default()
        }
    }
    pub fn validate(&self) -> Result<(), String> {
        if (!self.active && self.next != 0)
            || self.schema != 1
            || self.revision == u64::MAX
            || self.last_wall_ms > 253402300799999
        {
            return Err("Unsupported queue metadata".into());
        }
        if self.produced.as_ref().is_some_and(|s| s.len() > MAX_LINE)
            || self
                .owner
                .as_ref()
                .is_some_and(|s| uuid::Uuid::parse_str(s).is_err())
            || self.produced.is_some() != self.owner.is_some()
        {
            return Err("Invalid ownership metadata".into());
        }
        match &self.original {
            None if self.active
                || self.next != 0
                || self.pending.is_some()
                || self.produced.is_some() =>
            {
                return Err("Queue has no source list".into())
            }
            Some(text) => {
                if text.is_empty() {
                    return Err("Empty source list".into());
                }
                let lines = split(text)?;
                if self.next >= lines.len() {
                    return Err("Queue index outside source list".into());
                }
                if let Some(p) = &self.pending {
                    if !self.active || p.index != self.next || lines.get(p.index) != Some(&p.text) {
                        return Err("Invalid pending effect".into());
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }
    pub fn advance(&mut self, owner: &str) -> Result<(), String> {
        let pending = self.pending.take().ok_or("No pending paste")?;
        let lines = split(self.original.as_deref().ok_or("No source list")?)?;
        self.produced = Some(pending.text);
        self.owner = Some(owner.into());
        if pending.index + 1 == lines.len() {
            self.active = false;
            self.next = 0;
        } else {
            self.next = pending.index + 1;
        }
        Ok(())
    }
}

// Every component, including existing ancestors, must be a real directory/file.
// Host grants an absolute private directory. No migration or other profile reads.
fn no_links(path: &Path) -> Result<(), String> {
    if !path.is_absolute()
        || path
            .components()
            .any(|c| matches!(c, Component::ParentDir | Component::CurDir))
    {
        return Err("Host data path must be absolute without traversal".into());
    }
    let mut cursor = PathBuf::new();
    for c in path.components() {
        cursor.push(c);
        match fs::symlink_metadata(&cursor) {
            Ok(m) if m.file_type().is_symlink() => {
                return Err("Symlink in queue path; use a private real directory".into())
            }
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return Err("Cannot inspect private queue path".into()),
        }
    }
    Ok(())
}
#[derive(Clone)]
pub struct Store {
    dir: PathBuf,
}
impl Store {
    pub fn new(data: &Path) -> Result<Self, String> {
        no_links(data)?;
        if !data.is_dir() {
            return Err("Host private data directory is missing".into());
        }
        let dir = data.join("clipboard-sequential-paste-v2");
        no_links(&dir)?;
        if !dir.exists() {
            let mut builder = fs::DirBuilder::new();
            #[cfg(unix)]
            {
                use std::os::unix::fs::DirBuilderExt;
                builder.mode(0o700);
            }
            builder
                .create(&dir)
                .map_err(|_| "Cannot create private queue directory")?;
        }
        if !dir.is_dir() {
            return Err("Queue path is not a directory".into());
        }
        Ok(Self { dir })
    }
    pub fn load(&self) -> Result<Queue, String> {
        let path = self.dir.join("queue.json");
        no_links(&path)?;
        let meta = match fs::symlink_metadata(&path) {
            Ok(m) => m,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Queue::fresh()),
            Err(_) => return Err("Cannot read queue metadata; retained without changes".into()),
        };
        if !meta.is_file() || meta.len() > MAX_STORE {
            return Err("Queue is not a bounded regular file; retained. Inspect private queue.json offline.".into());
        }
        let mut bytes = Vec::new();
        fs::File::open(path)
            .map_err(|_| "Cannot open queue")?
            .take(MAX_STORE + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| "Cannot read queue")?;
        if bytes.len() as u64 > MAX_STORE {
            return Err("Queue grew beyond storage limit".into());
        }
        let q: Queue = serde_json::from_slice(&bytes).map_err(|_| {
            "Corrupt queue.json retained. Back it up and repair offline; nothing pasted."
        })?;
        q.validate().map_err(|_| {
            "Invalid queue.json retained. Back it up and repair offline; nothing pasted."
        })?;
        Ok(q)
    }
    pub fn save(&self, q: &mut Queue) -> Result<(), String> {
        q.validate()?;
        let revision = q
            .revision
            .checked_add(1)
            .filter(|n| *n != u64::MAX)
            .ok_or("Queue revision exhausted")?;
        let mut next = q.clone();
        next.revision = revision;
        let bytes = serde_json::to_vec(&next).map_err(|_| "Cannot encode queue")?;
        if bytes.len() as u64 > MAX_STORE {
            return Err("Queue exceeds storage bound".into());
        }
        no_links(&self.dir.join("queue.json"))?;
        let mut file = tempfile::NamedTempFile::new_in(&self.dir)
            .map_err(|_| "Cannot create atomic queue checkpoint; nothing retried")?;
        file.write_all(&bytes)
            .and_then(|_| file.as_file().sync_all())
            .map_err(|_| "Cannot sync queue checkpoint")?;
        file.persist(self.dir.join("queue.json"))
            .map_err(|_| "Cannot replace queue checkpoint; reopen to inspect pending state")?;
        #[cfg(unix)]
        fs::File::open(&self.dir)
            .and_then(|f| f.sync_all())
            .map_err(|_| {
                "Queue replaced but directory sync failed; reopen to inspect pending state"
            })?;
        *q = next;
        Ok(())
    }
}
