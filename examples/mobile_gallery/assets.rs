use gpui::{AssetSource, SharedString};
use std::{borrow::Cow, path::Path, sync::OnceLock};
#[path = "../gallery/assets.rs"]
mod base;
#[path = "embedded.rs"]
mod embedded;
pub use base::CUSTOM_ICON;
pub struct GalleryAssets;
static FILE_ROOT: OnceLock<std::path::PathBuf> = OnceLock::new();

/// Call once with an app-private writable directory before rendering specimens.
/// No install path is compiled into the binary; all bytes are bundled offline.
pub fn prepare_files(root: &Path) -> anyhow::Result<()> {
    if let Some(existing) = FILE_ROOT.get() {
        anyhow::ensure!(
            existing == root,
            "gallery assets already initialized at another root"
        );
        return Ok(());
    }
    for (name, bytes) in embedded::FILES {
        let path = root.join(name);
        std::fs::create_dir_all(path.parent().expect("asset parent"))?;
        std::fs::write(path, bytes)?;
    }
    FILE_ROOT
        .set(root.to_path_buf())
        .map_err(|_| anyhow::anyhow!("gallery asset initialization raced"))?;
    Ok(())
}

/// Resolve legacy string-based media APIs which require a filesystem path.
pub fn file(key: &str) -> SharedString {
    let Some(name) = key.strip_prefix("gallery-assets/") else {
        return key.to_owned().into();
    };
    FILE_ROOT
        .get()
        .expect("call assets::prepare_files before gallery init")
        .join(name)
        .to_string_lossy()
        .into_owned()
        .into()
}

impl AssetSource for GalleryAssets {
    fn load(&self, path: &str) -> gpui::Result<Option<Cow<'static, [u8]>>> {
        if let Some(name) = path.strip_prefix("gallery-assets/") {
            return Ok(embedded::FILES
                .iter()
                .find(|(key, _)| *key == name)
                .map(|(_, bytes)| Cow::Borrowed(*bytes)));
        }
        base::GalleryAssets.load(path)
    }
    fn list(&self, path: &str) -> gpui::Result<Vec<SharedString>> {
        let mut result = base::GalleryAssets.list(path)?;
        result.extend(
            embedded::FILES
                .iter()
                .map(|(name, _)| format!("gallery-assets/{name}"))
                .filter(|name| name.starts_with(path))
                .map(Into::into),
        );
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixture_files_are_removed_on_drop_and_can_be_recreated() {
        for _ in 0..2 {
            let fixture = prepare_test_files();
            let root = fixture.root.clone();
            assert!(root.is_dir());
            drop(fixture);
            let _owner = FIXTURE_OWNER
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            assert!(!root.exists(), "private gallery fixture leaked");
        }
    }

    #[test]
    fn cleanup_failure_during_unwind_preserves_the_original_panic() {
        let result = std::panic::catch_unwind(|| {
            let fixture = prepare_test_files();
            // Removing our directory first makes Drop's removal fail without leaving files.
            std::fs::remove_dir_all(&fixture.root).expect("remove owned failure fixture");
            panic!("original gallery test failure");
        });
        let original = result.expect_err("the original panic must propagate");
        assert_eq!(
            original.downcast_ref::<&str>(),
            Some(&"original gallery test failure")
        );
    }

    #[test]
    fn bundled_catalog_is_nonempty_unique_and_loadable() {
        for (index, (name, bytes)) in embedded::FILES.iter().enumerate() {
            assert!(!bytes.is_empty(), "{name}");
            assert!(
                !embedded::FILES[..index]
                    .iter()
                    .any(|(prior, _)| prior == name)
            );
            assert_eq!(
                GalleryAssets
                    .load(&format!("gallery-assets/{name}"))
                    .unwrap()
                    .unwrap()
                    .as_ref(),
                *bytes
            );
        }
        assert!(
            GalleryAssets
                .load("gallery-assets/missing.png")
                .unwrap()
                .is_none()
        );
    }
}

#[cfg(test)]
static FIXTURE_OWNER: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(test)]
#[must_use = "hold the fixture until the test finishes"]
pub(crate) struct TestFiles {
    root: std::path::PathBuf,
    _owner: std::sync::MutexGuard<'static, ()>,
}

#[cfg(test)]
impl Drop for TestFiles {
    fn drop(&mut self) {
        let result = std::fs::remove_dir_all(&self.root).and_then(|()| {
            if self.root.try_exists()? {
                Err(std::io::Error::other(
                    "private gallery fixture still exists",
                ))
            } else {
                Ok(())
            }
        });
        if std::thread::panicking() {
            if let Err(error) = result {
                use std::io::Write as _;
                let _ = writeln!(
                    std::io::stderr().lock(),
                    "gallery fixture cleanup failed at {}: {error}",
                    self.root.display()
                );
            }
        } else {
            assert!(
                result.is_ok(),
                "gallery fixture cleanup failed at {}: {result:?}",
                self.root.display()
            );
        }
    }
}

/// Serialize filesystem fixtures because the native file resolver is process-global.
#[cfg(test)]
pub(crate) fn prepare_test_files() -> TestFiles {
    static ROOT: OnceLock<std::path::PathBuf> = OnceLock::new();
    let guard = FIXTURE_OWNER
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let root = ROOT
        .get_or_init(|| {
            let nonce = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            std::env::temp_dir().join(format!("ely-gallery-test-{}-{nonce}", std::process::id()))
        })
        .clone();
    std::fs::create_dir(&root).expect("create private gallery fixture");
    let fixture = TestFiles {
        root: root.clone(),
        _owner: guard,
    };
    prepare_files(&root).expect("initialize gallery file resolver");
    // A repeated test uses the same OnceLock address after the previous cleanup.
    for (name, bytes) in embedded::FILES {
        let path = root.join(name);
        std::fs::create_dir_all(path.parent().unwrap()).expect("fixture asset directory");
        std::fs::write(&path, bytes).expect("fixture asset write");
        assert_eq!(
            std::fs::read(path).unwrap(),
            *bytes,
            "fixture bytes: {name}"
        );
    }
    fixture
}
