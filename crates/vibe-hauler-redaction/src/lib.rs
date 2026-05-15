#![allow(clippy::missing_errors_doc, clippy::struct_excessive_bools)]

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RedactionPolicy {
    pub mask_api_keys: bool,
    pub mask_bearer_tokens: bool,
    pub mask_url_credentials: bool,
    pub mask_pii: bool,
    pub max_preview_chars: usize,
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
