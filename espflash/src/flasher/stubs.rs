//! Stub loader module.

use std::{fs, path::Path, time::Duration};

use base64::{Engine as _, engine::general_purpose};
use serde::{Deserialize, Serialize};

use crate::{
    Error,
    resources::{self, ResourceKind},
    target::Chip,
};

/// Flash stub object (deserialized from TOML, converted from JSON as used by
/// `esptool.py`)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FlashStub {
    /// Entry point (address)
    entry: u32,
    /// Text (base64 encoded)
    text: String,
    /// Start of text section address
    text_start: u32,
    /// Data
    data: String,
    /// Start of data section address
    data_start: u32,
}

pub(crate) const CHIP_DETECT_MAGIC_REG_ADDR: u32 = 0x40001000;
pub(crate) const DEFAULT_TIMEOUT: Duration = Duration::from_secs(3);
pub(crate) const EXPECTED_STUB_HANDSHAKE: &str = "OHAI";

impl FlashStub {
    /// Fetch the bundled flash stub for the provided chip
    #[cfg(feature = "bundled-resources")]
    #[allow(dead_code)]
    #[deprecated(note = "Use get_with_rev instead.")]
    pub fn get(chip: Chip) -> FlashStub {
        Self::get_with_rev(chip, None)
    }

    /// Fetch the bundled flash stub for the provided chip and revision
    ///
    /// For ESP32-P4, if `revision` is provided and < 300, uses the RC1 stub.
    /// Otherwise uses the default stub for the chip.
    #[cfg(feature = "bundled-resources")]
    pub fn get_with_rev(chip: Chip, revision: Option<u32>) -> FlashStub {
        let s = resources::bundled(ResourceKind::Stub, Self::file_name(chip, revision)).unwrap();

        Self::from_toml(std::str::from_utf8(s).unwrap()).unwrap()
    }

    /// Find the flash stub for the provided chip and revision, in the resource
    /// directory or among the bundled stubs
    ///
    /// Returns `None` if no stub is available.
    pub fn find(chip: Chip, revision: Option<u32>) -> Result<Option<FlashStub>, Error> {
        let name = Self::file_name(chip, revision);
        let Some(data) = resources::load(ResourceKind::Stub, name)? else {
            return Ok(None);
        };
        let s =
            std::str::from_utf8(&data).map_err(|e| Error::InvalidStub(format!("{name}: {e}")))?;

        Self::from_toml(s)
            .map(Some)
            .map_err(|e| Error::InvalidStub(format!("{name}: {e}")))
    }

    /// Parse a flash stub from its TOML representation
    pub fn from_toml(s: &str) -> Result<FlashStub, Error> {
        toml::from_str(s).map_err(|e| Error::InvalidStub(e.to_string()))
    }

    /// Load a flash stub from a TOML file
    pub fn from_file(path: &Path) -> Result<FlashStub, Error> {
        let s = fs::read_to_string(path)
            .map_err(|e| Error::FileOpenError(path.display().to_string(), e))?;

        Self::from_toml(&s).map_err(|e| Error::InvalidStub(format!("{}: {e}", path.display())))
    }

    /// File name of the stub for the provided chip and revision
    fn file_name(chip: Chip, revision: Option<u32>) -> &'static str {
        match chip {
            Chip::Esp32 => "esp32.toml",
            Chip::Esp32c2 => "esp32c2.toml",
            Chip::Esp32c3 => "esp32c3.toml",
            Chip::Esp32c5 => "esp32c5.toml",
            Chip::Esp32c6 => "esp32c6.toml",
            Chip::Esp32c61 => "esp32c61.toml",
            Chip::Esp32h2 => "esp32h2.toml",
            Chip::Esp32h4 => "esp32h4.toml",
            Chip::Esp32p4 => {
                // For ESP32-P4, use RC1 stub if revision < 300 (matching
                // esptool behavior)
                if revision.unwrap_or(300) < 300 {
                    "esp32p4rc1.toml"
                } else {
                    "esp32p4.toml"
                }
            }
            Chip::Esp32s2 => "esp32s2.toml",
            Chip::Esp32s3 => "esp32s3.toml",
            Chip::Esp32s31 => "esp32s31.toml",
        }
    }

    /// Fetch stub entry point
    pub fn entry(&self) -> u32 {
        self.entry
    }

    /// Fetch text start address and bytes
    pub fn text(&self) -> (u32, Vec<u8>) {
        let v = general_purpose::STANDARD.decode(&self.text).unwrap();
        (self.text_start, v)
    }

    /// Fetch data start address and bytes
    pub fn data(&self) -> (u32, Vec<u8>) {
        let v = general_purpose::STANDARD.decode(&self.data).unwrap();
        (self.data_start, v)
    }
}

#[cfg(all(test, feature = "bundled-resources"))]
mod tests {
    use strum::IntoEnumIterator;

    use super::FlashStub;
    use crate::target::Chip;

    #[test]
    fn check_stub_encodings() {
        for c in Chip::iter() {
            // Stub must be valid TOML:
            let s = FlashStub::get_with_rev(c, None);

            // Data decoded from b64
            let _ = s.text();
            let _ = s.data();
        }
    }

    #[test]
    fn check_esp32p4_rc1_stub() {
        // Test RC1 stub for ESP32-P4
        let s = FlashStub::get_with_rev(Chip::Esp32p4, Some(200));
        let _ = s.text();
        let _ = s.data();
    }
}
