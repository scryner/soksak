use gpui::{AssetSource, Result, SharedString};
use std::path::PathBuf;
use std::{fs, str};

pub struct Assets {
    base: PathBuf,
}

impl Assets {
    pub fn new(base: PathBuf) -> Self {
        Self { base }
    }
}

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<std::borrow::Cow<'static, [u8]>>> {
        let path = self.base.join(path);
        match fs::read(&path) {
            Ok(data) => Ok(Some(data.into())),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(anyhow::Error::new(e)),
        }
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let path = self.base.join(path);
        let mut results = Vec::new();
        if let Ok(entries) = fs::read_dir(&path) {
            for entry in entries {
                if let Ok(entry) = entry {
                    if let Ok(name) = entry.file_name().into_string() {
                        results.push(name.into());
                    }
                }
            }
        }
        Ok(results)
    }
}
