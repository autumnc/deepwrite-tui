use anyhow::Result;
use std::path::Path;

pub fn load_file(path: &Path) -> Result<String> {
    let bytes = std::fs::read(path)?;
    // Check for BOM
    if let Some((encoding, bom_len)) = encoding_rs::Encoding::for_bom(&bytes) {
        let (content, _, _) = encoding.decode(&bytes[bom_len..]);
        return Ok(content.to_string());
    }
    // Try UTF-8 first (fast path)
    if let Ok(content) = std::str::from_utf8(&bytes) {
        return Ok(content.to_string());
    }
    // Fallback: auto-detect encoding via chardetng
    let mut detector = chardetng::EncodingDetector::new();
    detector.feed(&bytes, true);
    let encoding = detector.guess(None, true);
    let (content, _, had_errors) = encoding.decode(&bytes);
    if had_errors {
        anyhow::bail!("Failed to decode file: lossy conversion detected");
    }
    Ok(content.to_string())
}

pub fn save_file(path: &Path, content: &str) -> Result<()> {
    let dir = path.parent().unwrap_or(Path::new("."));
    let temp_path = dir.join(format!(".deepwrite-tmp-{}", std::process::id()));
    std::fs::write(&temp_path, content.as_bytes())?;
    std::fs::rename(&temp_path, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn load_file_strips_utf8_bom() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("bom.md");
        fs::write(&path, [0xEF, 0xBB, 0xBF, b'a', b'b', b'c']).unwrap();

        assert_eq!(load_file(&path).unwrap(), "abc");
    }

    #[test]
    fn load_file_decodes_utf16le_bom_without_marker() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("utf16.md");
        let bytes = [0xFF, 0xFE, b'a', 0x00, b'b', 0x00, b'c', 0x00];
        fs::write(&path, bytes).unwrap();

        assert_eq!(load_file(&path).unwrap(), "abc");
    }
}
