//! Bootloader and flasher stub resources
//!
//! Resources are looked up by file name, first in the resource directory and
//! then among the copies bundled into the binary (with the
//! `bundled-resources` feature). The resource directory mirrors the layout of
//! `espflash/resources/`, i.e. it contains `bootloaders/` and `stubs/`
//! subdirectories.
//!
//! The resource directory is taken from the `ESPFLASH_RESOURCE_DIR` environment
//! variable at runtime or, if that is unset, from the same variable at build
//! time.

use std::{
    borrow::Cow,
    env,
    fs,
    path::{Path, PathBuf},
};

use crate::Error;

/// Environment variable (runtime or build time) naming the resource directory
pub const RESOURCE_DIR_ENV: &str = "ESPFLASH_RESOURCE_DIR";

/// Kind of resource, which determines its subdirectory
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ResourceKind {
    Bootloader,
    #[cfg_attr(not(feature = "serialport"), allow(dead_code))]
    Stub,
}

impl ResourceKind {
    fn subdir(self) -> &'static str {
        match self {
            ResourceKind::Bootloader => "bootloaders",
            ResourceKind::Stub => "stubs",
        }
    }
}

/// Returns the resource directory, if one is configured.
pub fn resource_dir() -> Option<PathBuf> {
    env::var_os(RESOURCE_DIR_ENV)
        .map(PathBuf::from)
        .or_else(|| option_env!("ESPFLASH_RESOURCE_DIR").map(PathBuf::from))
}

/// Loads a resource, preferring the resource directory over bundled copies.
///
/// Returns `None` if the resource is neither in the resource directory nor
/// bundled.
pub(crate) fn load(kind: ResourceKind, name: &str) -> Result<Option<Cow<'static, [u8]>>, Error> {
    load_from(resource_dir().as_deref(), kind, name)
}

fn load_from(
    dir: Option<&Path>,
    kind: ResourceKind,
    name: &str,
) -> Result<Option<Cow<'static, [u8]>>, Error> {
    if let Some(dir) = dir {
        let path = dir.join(kind.subdir()).join(name);
        if path.is_file() {
            let data =
                fs::read(&path).map_err(|e| Error::FileOpenError(path.display().to_string(), e))?;
            return Ok(Some(Cow::Owned(data)));
        }
    }

    Ok(bundled(kind, name).map(Cow::Borrowed))
}

#[cfg(feature = "bundled-resources")]
pub(crate) fn bundled(kind: ResourceKind, name: &str) -> Option<&'static [u8]> {
    macro_rules! bundled {
        ($dir:literal: $($file:literal),* $(,)?) => {
            match name {
                $($file => Some(include_bytes!(concat!("../resources/", $dir, "/", $file)).as_slice()),)*
                _ => None,
            }
        };
    }

    match kind {
        ResourceKind::Bootloader => bundled!("bootloaders":
            "esp32_26-bootloader.bin",
            "esp32-bootloader.bin",
            "esp32c2_26-bootloader.bin",
            "esp32c2-bootloader.bin",
            "esp32c3-bootloader.bin",
            "esp32c5-bootloader.bin",
            "esp32c6-bootloader.bin",
            "esp32c61-bootloader.bin",
            "esp32h2-bootloader.bin",
            "esp32h4-bootloader.bin",
            "esp32p4-v0-bootloader.bin",
            "esp32p4-v3-bootloader.bin",
            "esp32s2-bootloader.bin",
            "esp32s3-bootloader.bin",
            "esp32s31-bootloader.bin",
        ),
        ResourceKind::Stub => bundled!("stubs":
            "esp32.toml",
            "esp32c2.toml",
            "esp32c3.toml",
            "esp32c5.toml",
            "esp32c6.toml",
            "esp32c61.toml",
            "esp32h2.toml",
            "esp32h4.toml",
            "esp32p4.toml",
            "esp32p4rc1.toml",
            "esp32s2.toml",
            "esp32s3.toml",
            "esp32s31.toml",
        ),
    }
}

#[cfg(not(feature = "bundled-resources"))]
pub(crate) fn bundled(_kind: ResourceKind, _name: &str) -> Option<&'static [u8]> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resource_dir_takes_precedence() {
        let dir = env::temp_dir().join(format!("espflash-resources-{}", std::process::id()));
        fs::create_dir_all(dir.join("bootloaders")).unwrap();
        fs::write(dir.join("bootloaders/esp32s3-bootloader.bin"), b"custom").unwrap();

        let data = load_from(
            Some(&dir),
            ResourceKind::Bootloader,
            "esp32s3-bootloader.bin",
        );
        fs::remove_dir_all(&dir).unwrap();

        assert_eq!(data.unwrap().as_deref(), Some(b"custom".as_slice()));
    }

    #[test]
    fn missing_resource_is_none() {
        let data = load_from(None, ResourceKind::Stub, "does-not-exist.toml").unwrap();
        assert!(data.is_none());
    }

    #[cfg(feature = "bundled-resources")]
    #[test]
    fn bundled_fallback() {
        let dir = env::temp_dir().join("espflash-resources-does-not-exist");
        let data = load_from(Some(&dir), ResourceKind::Stub, "esp32s3.toml").unwrap();
        assert!(matches!(data, Some(Cow::Borrowed(_))));
    }
}
