use crate::error::{Error, Result};

const MAX_ID_LEN: usize = 256;
const MAX_VEC_LEN: usize = 10_000;
const MAX_TEXT_LEN: usize = 10 * 1024 * 1024; // 10MB

/// Validate an iTerm2 object identifier (session, tab, window ID).
/// Rejects null bytes and excessively long values.
pub fn identifier(id: &str, kind: &str) -> Result<()> {
    if id.len() > MAX_ID_LEN {
        return Err(Error::Api(format!(
            "{kind} ID too long ({} bytes, max {MAX_ID_LEN})",
            id.len()
        )));
    }
    if id.contains('\0') {
        return Err(Error::Api(format!(
            "{kind} ID contains null byte"
        )));
    }
    Ok(())
}

/// Validate a vector parameter is within bounds.
pub fn vec_len<T>(v: &[T], param: &str) -> Result<()> {
    if v.len() > MAX_VEC_LEN {
        return Err(Error::Api(format!(
            "{param} too many elements ({}, max {MAX_VEC_LEN})",
            v.len()
        )));
    }
    Ok(())
}

/// Validate text length for send operations.
pub fn text_len(text: &str) -> Result<()> {
    if text.len() > MAX_TEXT_LEN {
        return Err(Error::Api(format!(
            "Text too long ({} bytes, max {MAX_TEXT_LEN})",
            text.len()
        )));
    }
    Ok(())
}

/// Validate that a string is syntactically valid JSON.
pub fn json_value(value: &str) -> Result<()> {
    serde_json::from_str::<serde_json::Value>(value).map_err(|e| {
        Error::Api(format!("Invalid JSON value: {e}"))
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_identifier() {
        identifier("session-abc-123", "session").unwrap();
        identifier("active", "session").unwrap();
        identifier("all", "session").unwrap();
    }

    #[test]
    fn identifier_with_null_byte() {
        let err = identifier("session\0id", "session").unwrap_err();
        assert!(err.to_string().contains("null byte"));
    }

    #[test]
    fn identifier_too_long() {
        let long_id = "x".repeat(MAX_ID_LEN + 1);
        let err = identifier(&long_id, "session").unwrap_err();
        assert!(err.to_string().contains("too long"));
    }

    #[test]
    fn vec_within_bounds() {
        vec_len(&vec![1, 2, 3], "ids").unwrap();
    }

    #[test]
    fn vec_exceeds_bounds() {
        let big: Vec<i32> = vec![0; MAX_VEC_LEN + 1];
        let err = vec_len(&big, "ids").unwrap_err();
        assert!(err.to_string().contains("too many"));
    }

    #[test]
    fn text_within_bounds() {
        text_len("hello world").unwrap();
    }

    #[test]
    fn text_exceeds_bounds() {
        let big = "x".repeat(MAX_TEXT_LEN + 1);
        let err = text_len(&big).unwrap_err();
        assert!(err.to_string().contains("too long"));
    }

    #[test]
    fn valid_json_values() {
        json_value(r#""hello""#).unwrap();
        json_value("42").unwrap();
        json_value("true").unwrap();
        json_value("null").unwrap();
        json_value(r#"{"key": "value"}"#).unwrap();
        json_value(r#"[1, 2, 3]"#).unwrap();
    }

    #[test]
    fn invalid_json_value() {
        let err = json_value("not valid json").unwrap_err();
        assert!(err.to_string().contains("Invalid JSON"));
    }

    #[test]
    fn empty_string_is_invalid_json() {
        let err = json_value("").unwrap_err();
        assert!(err.to_string().contains("Invalid JSON"));
    }
}
