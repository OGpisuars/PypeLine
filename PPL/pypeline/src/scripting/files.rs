//! Player script files on disk.
//!
//! Scripts live in the per-user data folder (`~/.local/share/PypeLine` on
//! Linux, `%APPDATA%\PypeLine` on Windows, `~/Library/Application
//! Support/PypeLine` on macOS) as plain `.py` files. Writes are atomic: the
//! new text goes to a temp file that is then renamed over the old one, so a
//! crash mid-save never leaves a half-written script (roadmap Part 4 A). The
//! previous version is kept next to it as `main.py.bak`.

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use bevy::prelude::Resource;

#[derive(Resource, Debug, Clone)]
pub struct ScriptStore {
    dir: PathBuf,
}

impl ScriptStore {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    /// The standard per-user location, if the OS has one.
    pub fn default_location() -> Option<Self> {
        dirs::data_dir().map(|data| Self::new(data.join("PypeLine").join("scripts")))
    }

    /// The folder the scripts live in.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn main_path(&self) -> PathBuf {
        self.dir.join("main.py")
    }

    /// The saved main.py, or None if there is none yet.
    pub fn load_main(&self) -> io::Result<Option<String>> {
        match fs::read_to_string(self.main_path()) {
            Ok(text) => Ok(Some(text)),
            Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(err) => Err(err),
        }
    }

    /// Save main.py, keeping the previous version as main.py.bak.
    pub fn save_main(&self, source: &str) -> io::Result<()> {
        self.save("main.py", source)
    }

    /// Save a script file, keeping the previous version as NAME.py.bak.
    pub fn save(&self, file: &str, source: &str) -> io::Result<()> {
        fs::create_dir_all(&self.dir)?;
        let path = self.dir.join(file);
        if path.exists() {
            fs::copy(&path, path.with_extension("py.bak"))?;
        }
        write_atomic(&path, source)
    }

    /// Every `.py` file in the folder, sorted by name.
    pub fn load_all(&self) -> io::Result<Vec<(String, String)>> {
        let entries = match fs::read_dir(&self.dir) {
            Ok(entries) => entries,
            Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(err) => return Err(err),
        };
        let mut files = Vec::new();
        for entry in entries {
            let path = entry?.path();
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            if path.extension().is_some_and(|e| e == "py") {
                files.push((name.to_owned(), fs::read_to_string(&path)?));
            }
        }
        files.sort();
        Ok(files)
    }

    /// Delete a script file. Its last version stays as NAME.py.bak.
    pub fn delete(&self, file: &str) -> io::Result<()> {
        let path = self.dir.join(file);
        if path.exists() {
            fs::copy(&path, path.with_extension("py.bak"))?;
            fs::remove_file(path)?;
        }
        Ok(())
    }
}

/// Write `contents` to `path` so that readers only ever see the old file or
/// the complete new one.
pub fn write_atomic(path: &Path, contents: &str) -> io::Result<()> {
    let tmp = path.with_extension("tmp");
    {
        let mut file = fs::File::create(&tmp)?;
        file.write_all(contents.as_bytes())?;
        file.sync_all()?;
    }
    fs::rename(&tmp, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_store(name: &str) -> ScriptStore {
        let dir = std::env::temp_dir().join(format!("pypeline-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        ScriptStore::new(dir)
    }

    #[test]
    fn save_and_load_round_trip() {
        let store = temp_store("roundtrip");
        assert_eq!(store.load_main().unwrap(), None);
        store.save_main("print('one')\n").unwrap();
        store.save_main("print('two')\n").unwrap();
        assert_eq!(
            store.load_main().unwrap().as_deref(),
            Some("print('two')\n")
        );
        let backup = fs::read_to_string(store.main_path().with_extension("py.bak")).unwrap();
        assert_eq!(backup, "print('one')\n");
        assert!(!store.main_path().with_extension("tmp").exists());
        store.save("PPL.py", "x = 1\n").unwrap();
        let names: Vec<String> = store
            .load_all()
            .unwrap()
            .into_iter()
            .map(|(n, _)| n)
            .collect();
        assert_eq!(names, vec!["PPL.py", "main.py"]);
        store.delete("PPL.py").unwrap();
        assert_eq!(store.load_all().unwrap().len(), 1);
        fs::remove_dir_all(&store.dir).unwrap();
    }
}
