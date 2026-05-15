# Redaction Module

Cargo package: `vibe-hauler-redaction`

## Responsibility

Redaction protects terminal output, JSON reports, previews, logs, and debug traces from leaking secrets.

It owns:

- secret detectors;
- masking policy;
- preview truncation;
- raw output guardrails.

It does not parse full app sessions beyond preview text.

## Default Detectors

| Secret Type | Examples |
|---|---|
| OpenAI keys | `sk-...` |
| Anthropic keys | provider-specific API keys |
| GitHub tokens | `ghp_`, `github_pat_` |
| Google/AWS credentials | common key formats |
| Bearer tokens | `Authorization: Bearer ...` |
| JWTs | three-part base64url tokens |
| URL credentials | `https://user:pass@host` |
| SSH private keys | PEM blocks |
| Email/phone | optional user-configured masking |

## Masking Rules

- Keep short prefixes only when useful for recognition.
- Never print complete token values.
- Default preview length is 200 characters.
- `--raw` requires explicit user intent.
- JSON output should include `redacted: true` metadata where masking happened.

## APIs

```rust
pub struct RedactionPolicy {
    pub mask_api_keys: bool,
    pub mask_bearer_tokens: bool,
    pub mask_url_credentials: bool,
    pub mask_pii: bool,
    pub max_preview_chars: usize,
}

pub fn redact_text(input: &str, policy: &RedactionPolicy) -> RedactedText;
```

## Testing

- one unit test per detector;
- false-positive tests for ordinary code strings;
- snapshot tests for session previews;
- raw mode tests proving explicit bypass is visible and auditable.

