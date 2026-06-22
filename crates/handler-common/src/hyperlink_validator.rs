//! Cross-handler allowlist for user-supplied hyperlink URI schemes.
//!
//! Mirrors `Core/HyperlinkUriValidator.cs` from the C# upstream. Without
//! this gate, OOXML handlers would happily persist `javascript:`, `data:`,
//! `vbscript:`, and other URI schemes that downstream Office products
//! either warn-or-execute on click. OOXML itself places no scheme restriction;
//! every Office product applies its own runtime UI, so we filter at write
//! time and keep the document clean.
//!
//! Handler-internal targets (PowerPoint's `ppaction://`, `slide://`, named
//! actions like `firstslide`/`nextslide`, fragment anchors like `#_ftn1`,
//! in-workbook references like `Sheet!A1`, or any non-absolute URI) are
//! resolved by the handler before this validator is consulted; callers only
//! pass strings here once they have been classified as an external URI.

/// Schemes that survive an Office "is this link safe?" prompt without
/// user warnings. `http`/`https`/`mailto` are the everyday cases;
/// `ftp`/`sms`/`tel`/`news` are the standard PowerPoint "Action button"
/// set; `ppaction` is PowerPoint's internal navigation pseudo-scheme.
///
/// `file:` is intentionally allowed — real-world documents use it to link
/// local/network resources (`file:///C:/...`). Unlike `javascript:`/`data:`
/// /`vbscript:`, it does not execute script or exfiltrate data; Office prompts
/// on follow like any external link. Allowing it lets dump→replay round-trip
/// file-target hyperlinks instead of emitting a command the batch rejects.
///
/// `javascript:` / `data:` / `vbscript:` stay rejected (omitted from the
/// allowlist).
const ALLOWED_SCHEMES: &[&str] = &[
    "http", "https", "mailto", "ftp", "ftps", "sftp", "news", "tel", "sms", "ppaction", "file",
];

/// True when `url` is an absolute URI whose scheme is in the allowlist.
///
/// Used by the HTML preview, which must not throw on an authored-in
/// `HYPERLINK()` formula but also must not emit a `javascript:` / `data:` /
/// `file:` href as an XSS sink.
pub fn is_safe_scheme(url: &str) -> bool {
    if url.is_empty() {
        return false;
    }
    let Some(scheme) = extract_scheme(url) else {
        return false;
    };
    ALLOWED_SCHEMES
        .iter()
        .any(|s| s.eq_ignore_ascii_case(scheme))
}

/// Returns the lowercase scheme prefix when `url` parses as `scheme:rest`,
/// mirroring `Uri.TryCreate(..., Absolute)` from the C# implementation.
///
/// We don't pull in a `url` crate for what is a 12-line scan: skip leading
/// whitespace, accept scheme = ALPHA *( ALPHA / DIGIT / "+" / "-" / "." ),
/// and require the colon that separates scheme from the rest.
fn extract_scheme(url: &str) -> Option<&str> {
    let trimmed = url.trim_start();
    let bytes = trimmed.as_bytes();
    if bytes.is_empty() {
        return None;
    }
    // First char must be ALPHA per RFC 3986.
    if !bytes[0].is_ascii_alphabetic() {
        return None;
    }
    // Scheme body: ALPHA / DIGIT / "+" / "-" / "." up to ':'.
    let mut i = 1;
    while i < bytes.len() {
        let c = bytes[i];
        if c == b':' {
            // Empty scheme like ":" alone is rejected; need at least one body char.
            if i == 1 {
                return None;
            }
            // The byte slice is ASCII for the scheme characters we accept.
            let scheme = &trimmed[..i];
            // Sanity-check scheme body so "java script:" or "1http:" don't slip
            // through with the i==1 fast path.
            if scheme.as_bytes()[1..]
                .iter()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'+' | b'-' | b'.'))
            {
                return Some(scheme);
            }
            return None;
        }
        // Reject characters that can't appear in a scheme body.
        if !c.is_ascii_alphanumeric() && !matches!(c, b'+' | b'-' | b'.') {
            return None;
        }
        i += 1;
    }
    // No colon → not an absolute URI.
    None
}

/// Validate an external hyperlink URI's scheme. Returns
/// `Err(message)` with a deterministic, agent-readable diagnostic when the
/// scheme is not in the allowlist. Empty input is a no-op so the caller's
/// own "missing URL" diagnostic remains the surfaced error. Non-absolute
/// URIs are also a no-op (handler-internal paths are resolved before this
/// validator is consulted).
pub fn require_safe_scheme(url: &str, context_key: &str) -> Result<(), String> {
    if url.is_empty() {
        return Ok(());
    }
    let Some(scheme) = extract_scheme(url) else {
        return Ok(());
    };
    if ALLOWED_SCHEMES
        .iter()
        .any(|s| s.eq_ignore_ascii_case(scheme))
    {
        return Ok(());
    }
    Err(format!(
        "Invalid {} URL scheme '{}:': only http, https, mailto, ftp, ftps, sftp, news, tel, sms, file, and ppaction targets are accepted. \
         javascript:, data:, vbscript:, and similar schemes are rejected to prevent click-bait redirection in shared documents.",
        context_key, scheme
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allowlist_accepts_safe_schemes() {
        assert!(is_safe_scheme("https://example.com"));
        assert!(is_safe_scheme("http://example.com"));
        assert!(is_safe_scheme("mailto:user@example.com"));
        assert!(is_safe_scheme("ftp://example.com/file"));
        assert!(is_safe_scheme("file:///C:/Users/file.txt"));
        assert!(is_safe_scheme("ppaction://action"));
        assert!(is_safe_scheme("HTTPS://UPPERCASE")); // case-insensitive
    }

    #[test]
    fn allowlist_rejects_unsafe_schemes() {
        assert!(!is_safe_scheme("javascript:alert('x')"));
        assert!(!is_safe_scheme("data:text/html,<script>"));
        assert!(!is_safe_scheme("vbscript:msgbox"));
    }

    #[test]
    fn allowlist_rejects_non_absolute() {
        assert!(!is_safe_scheme(""));
        assert!(!is_safe_scheme("relative/path"));
        assert!(!is_safe_scheme("Sheet1!A1"));
        assert!(!is_safe_scheme("#fragment"));
    }

    #[test]
    fn require_safe_returns_ok_for_safe_or_empty_or_relative() {
        assert!(require_safe_scheme("", "link").is_ok());
        assert!(require_safe_scheme("relative/path", "link").is_ok());
        assert!(require_safe_scheme("https://example.com", "link").is_ok());
    }

    #[test]
    fn require_safe_rejects_javascript_with_named_context() {
        let err = require_safe_scheme("javascript:alert(1)", "hyperlink").unwrap_err();
        assert!(err.contains("hyperlink"));
        assert!(err.contains("javascript"));
    }

    #[test]
    fn scheme_extraction_handles_plus_and_dash() {
        // Sanity: parsing the underlying rule.
        assert_eq!(extract_scheme("https://example.com"), Some("https"));
        assert_eq!(extract_scheme("a+b-c.d:e"), Some("a+b-c.d"));
        assert_eq!(extract_scheme("not-a-url"), None);
        assert_eq!(extract_scheme("://no-scheme"), None);
        assert_eq!(extract_scheme("1http:"), None); // first char must be ALPHA
    }
}
