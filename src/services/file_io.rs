use anyhow::Result;
use std::path::Path;

pub fn load_file(path: &Path) -> Result<String> {
    let bytes = std::fs::read(path)?;
    // Try UTF-8 first (fast path)
    if let Ok(content) = String::from_utf8(bytes.clone()) {
        return Ok(content);
    }
    // Check for BOM
    if let Some((encoding, _)) = encoding_rs::Encoding::for_bom(&bytes) {
        let (content, _, _) = encoding.decode(&bytes);
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
