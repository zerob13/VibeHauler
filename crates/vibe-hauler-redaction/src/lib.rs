#![allow(
    clippy::missing_errors_doc,
    clippy::module_name_repetitions,
    clippy::struct_excessive_bools
)]

use std::sync::LazyLock;

use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RedactionPolicy {
    pub mask_api_keys: bool,
    pub mask_bearer_tokens: bool,
    pub mask_url_credentials: bool,
    pub mask_pii: bool,
    pub max_preview_chars: usize,
}

impl Default for RedactionPolicy {
    fn default() -> Self {
        Self {
            mask_api_keys: true,
            mask_bearer_tokens: true,
            mask_url_credentials: true,
            mask_pii: false,
            max_preview_chars: 200,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RedactedText {
    pub value: String,
    pub redacted: bool,
}

pub trait Redactor {
    type Error;

    fn redact(&self, input: &str, policy: &RedactionPolicy) -> Result<RedactedText, Self::Error>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct DefaultRedactor;

impl Redactor for DefaultRedactor {
    type Error = std::convert::Infallible;

    fn redact(&self, input: &str, policy: &RedactionPolicy) -> Result<RedactedText, Self::Error> {
        Ok(redact_text(input, policy))
    }
}

#[must_use]
pub fn redact_text(input: &str, policy: &RedactionPolicy) -> RedactedText {
    let mut value = input.to_owned();
    let mut redacted = false;

    if policy.mask_api_keys {
        for regex in [&*OPENAI_KEY, &*ANTHROPIC_KEY, &*GITHUB_TOKEN, &*AWS_KEY] {
            redacted |= regex.is_match(&value);
            value = regex.replace_all(&value, "$1...[redacted]").into_owned();
        }
        redacted |= SSH_PRIVATE_KEY.is_match(&value);
        value = SSH_PRIVATE_KEY
            .replace_all(&value, "-----BEGIN [redacted] PRIVATE KEY-----")
            .into_owned();
    }

    if policy.mask_bearer_tokens {
        redacted |= BEARER.is_match(&value);
        value = BEARER
            .replace_all(&value, "Authorization: Bearer ...[redacted]")
            .into_owned();
        redacted |= JWT.is_match(&value);
        value = JWT.replace_all(&value, "$1...[redacted].$3").into_owned();
    }

    if policy.mask_url_credentials {
        redacted |= URL_CREDENTIALS.is_match(&value);
        value = URL_CREDENTIALS
            .replace_all(&value, "${scheme}://[redacted]@")
            .into_owned();
    }

    if policy.mask_pii {
        redacted |= EMAIL.is_match(&value);
        value = EMAIL.replace_all(&value, "[email redacted]").into_owned();
        redacted |= PHONE.is_match(&value);
        value = PHONE.replace_all(&value, "[phone redacted]").into_owned();
    }

    let truncated = truncate_chars(&value, policy.max_preview_chars);
    redacted |= truncated.len() != value.len();
    RedactedText {
        value: truncated,
        redacted,
    }
}

#[must_use]
pub fn truncate_preview(input: &str, max_chars: usize) -> RedactedText {
    let value = truncate_chars(input, max_chars);
    RedactedText {
        redacted: value.len() != input.len(),
        value,
    }
}

fn truncate_chars(input: &str, max_chars: usize) -> String {
    if input.chars().count() <= max_chars {
        return input.to_owned();
    }
    let keep = max_chars.saturating_sub(3);
    let mut output = input.chars().take(keep).collect::<String>();
    output.push_str("...");
    output
}

static OPENAI_KEY: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b(sk-[A-Za-z0-9_\-]{4})[A-Za-z0-9_\-]{8,}").expect("valid regex")
});
static ANTHROPIC_KEY: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b(sk-ant-[A-Za-z0-9_\-]{4})[A-Za-z0-9_\-]{8,}").expect("valid regex")
});
static GITHUB_TOKEN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b((?:ghp|gho|ghu|ghs|ghr|github_pat)_[A-Za-z0-9_]{4})[A-Za-z0-9_]{8,}")
        .expect("valid regex")
});
static AWS_KEY: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b(AKIA[0-9A-Z]{4})[0-9A-Z]{8,}\b").expect("valid regex"));
static BEARER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)Authorization:\s*Bearer\s+[A-Za-z0-9._\-]+").expect("valid regex")
});
static JWT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b([A-Za-z0-9_\-]{8,})\.([A-Za-z0-9_\-]{8,})\.([A-Za-z0-9_\-]{8,})\b")
        .expect("valid regex")
});
static URL_CREDENTIALS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?P<scheme>https?)://[^/\s:@]+:[^@\s/]+@").expect("valid regex"));
static SSH_PRIVATE_KEY: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"-----BEGIN [A-Z ]*PRIVATE KEY-----[\s\S]*?-----END [A-Z ]*PRIVATE KEY-----")
        .expect("valid regex")
});
static EMAIL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b[\w.%+\-]+@[\w.\-]+\.[A-Za-z]{2,}\b").expect("valid regex"));
static PHONE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b\+?[0-9][0-9 .()\-]{7,}[0-9]\b").expect("valid regex"));

#[cfg(test)]
mod tests {
    use super::{RedactionPolicy, redact_text};

    #[test]
    fn masks_common_secret_shapes() {
        let policy = RedactionPolicy::default();
        let output = redact_text(
            "Authorization: Bearer abc.def.ghi and sk-1234567890abcdef",
            &policy,
        );
        assert!(output.redacted);
        assert!(!output.value.contains("1234567890abcdef"));
        assert!(!output.value.contains("abc.def.ghi"));
    }

    #[test]
    fn masks_url_credentials_and_truncates_preview() {
        let policy = RedactionPolicy {
            max_preview_chars: 20,
            ..RedactionPolicy::default()
        };
        let output = redact_text("fetch https://user:pass@example.com/path please", &policy);
        assert!(output.redacted);
        assert!(output.value.contains("[redacted]") || output.value.ends_with("..."));
    }

    #[test]
    fn ordinary_code_is_not_a_secret() {
        let output = redact_text(
            "let token_count = 3; let sketch = true;",
            &RedactionPolicy::default(),
        );
        assert!(!output.redacted);
    }
}
