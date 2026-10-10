use std::path::Path;

use crate::error::AppError;

/// Detects character encoding and decodes raw bytes into a UTF-8 string with normalized newlines.
///
/// Implements FR-RAW-01 and FR-RAW-02:
/// - Handles UTF-8 with or without BOM.
/// - Falls back to Japanese encoding detection (Shift_JIS/CP932).
/// - Normalizes `\r\n` and `\r` line endings to `\n`.
pub fn decode_bytes(bytes: &[u8]) -> Result<String, AppError> {
    // 1. Check for UTF-8 Byte Order Mark (BOM: 0xEF, 0xBB, 0xBF)
    let (data_without_bom, has_bom) = if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        (&bytes[3..], true)
    } else {
        (bytes, false)
    };

    // 2. Try fast path: valid UTF-8
    if let Ok(utf8_str) = std::str::from_utf8(data_without_bom) {
        return Ok(normalize_newlines(utf8_str));
    }

    // If UTF-8 BOM was present but the rest wasn't valid UTF-8, still attempt detection
    let bytes_to_detect = if has_bom { data_without_bom } else { bytes };

    // 3. Fallback: chardetng encoding detector
    let mut detector = chardetng::EncodingDetector::new();
    detector.feed(bytes_to_detect, true);
    let encoding = detector.guess(None, true);

    let (decoded, _, had_errors) = encoding.decode(bytes_to_detect);
    if !had_errors {
        return Ok(normalize_newlines(&decoded));
    }

    // 4. Fallback explicitly to Shift_JIS / Windows-31J (CP932)
    let (sjis_decoded, _, sjis_errors) = encoding_rs::SHIFT_JIS.decode(bytes_to_detect);
    if !sjis_errors {
        return Ok(normalize_newlines(&sjis_decoded));
    }

    // If both had errors, return the best effort from detector if not empty, or error
    if !decoded.is_empty() {
        Ok(normalize_newlines(&decoded))
    } else {
        Err(AppError::Internal {
            message: "Failed to decode raw content with UTF-8 or Shift_JIS".to_string(),
        })
    }
}

/// Reads a raw file into memory without ever modifying the original file on disk.
pub fn read_raw_file(path: &Path) -> Result<String, AppError> {
    let bytes = std::fs::read(path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            AppError::FileNotFound {
                path: path.to_string_lossy().to_string(),
            }
        } else if e.kind() == std::io::ErrorKind::PermissionDenied {
            AppError::PermissionDenied {
                path: path.to_string_lossy().to_string(),
            }
        } else {
            AppError::Internal {
                message: format!("Failed to read raw file {}: {e}", path.display()),
            }
        }
    })?;

    decode_bytes(&bytes)
}

/// Normalizes CRLF and CR newlines into standard LF.
fn normalize_newlines(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\r' {
            if chars.peek() == Some(&'\n') {
                chars.next();
            }
            result.push('\n');
        } else {
            result.push(c);
        }
    }
    result
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn test_utf8_without_bom() {
        let text = "健太と美咲\n第一話\n";
        let decoded = decode_bytes(text.as_bytes()).unwrap();
        assert_eq!(decoded, text);
    }

    #[test]
    fn test_utf8_with_bom() {
        let text = "健太と美咲\n第一話\n";
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(text.as_bytes());

        let decoded = decode_bytes(&bytes).unwrap();
        assert_eq!(decoded, text);
    }

    #[test]
    fn test_shift_jis_decoding() {
        let text = "健太と美咲\n第一話\n";
        let (sjis_bytes, _, _) = encoding_rs::SHIFT_JIS.encode(text);

        let decoded = decode_bytes(&sjis_bytes).unwrap();
        assert_eq!(decoded, text);
    }

    #[test]
    fn test_newline_normalization() {
        let crlf = "line 1\r\nline 2\rline 3\nline 4";
        let decoded = decode_bytes(crlf.as_bytes()).unwrap();
        assert_eq!(decoded, "line 1\nline 2\nline 3\nline 4");
    }
}
