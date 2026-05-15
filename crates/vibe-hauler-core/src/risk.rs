use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::models::ItemKind;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub enum RiskLevel {
    Green,
    Yellow,
    Red,
    Black,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub enum Recommendation {
    Clean,
    Review,
    Protect,
    ReportOnly,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RiskInput<'a> {
    pub path: &'a Path,
    pub kind: ItemKind,
    pub evidence: &'a [String],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RiskAssessment {
    pub risk: RiskLevel,
    pub recommendation: Recommendation,
    pub reason: String,
    pub backup_required: bool,
}

pub struct RiskEngine;

impl RiskEngine {
    #[allow(clippy::too_many_lines)]
    #[must_use]
    pub fn classify(input: &RiskInput<'_>) -> RiskAssessment {
        let haystack = format!(
            "{} {}",
            input.path.to_string_lossy().to_ascii_lowercase(),
            input.evidence.join(" ").to_ascii_lowercase()
        );

        if contains_any(
            &haystack,
            &[
                "permission denied",
                "locked",
                "lock",
                "symlink escape",
                ".sqlite",
                ".sqlite3",
                ".db",
                ".ldb",
                ".sst",
                "indexeddb",
                "leveldb",
            ],
        ) || matches!(input.kind, ItemKind::Database)
        {
            return RiskAssessment {
                risk: RiskLevel::Black,
                recommendation: Recommendation::ReportOnly,
                reason: "locked, database, permission, or unknown binary data is report-only"
                    .to_owned(),
                backup_required: false,
            };
        }

        if matches!(
            input.kind,
            ItemKind::Credential | ItemKind::Config | ItemKind::Memory
        ) || contains_any(
            &haystack,
            &[
                "auth",
                "token",
                "credential",
                "secret",
                "oauth",
                "keychain",
                "api_key",
                "apikey",
                "settings",
                "config.toml",
                "config.json",
                "rules",
                "skills",
                "commands",
                "hooks",
                "memory",
                ".aider.conf",
                "claude.md",
                "agents.md",
            ],
        ) {
            return RiskAssessment {
                risk: RiskLevel::Red,
                recommendation: Recommendation::Protect,
                reason: "credentials, configuration, rules, or memory are protected".to_owned(),
                backup_required: false,
            };
        }

        if matches!(input.kind, ItemKind::Session)
            || contains_any(
                &haystack,
                &[
                    "session",
                    "transcript",
                    "history",
                    "checkpoint",
                    "chat",
                    "conversation",
                    "projects/",
                    "projects\\",
                    ".jsonl",
                    ".aider.chat.history",
                    ".aider.input.history",
                    ".aider.llm.history",
                ],
            )
        {
            return RiskAssessment {
                risk: RiskLevel::Yellow,
                recommendation: Recommendation::Review,
                reason: "sessions, transcripts, and history require review and backup".to_owned(),
                backup_required: true,
            };
        }

        if matches!(input.kind, ItemKind::Cache | ItemKind::Log)
            || contains_any(
                &haystack,
                &[
                    "cache",
                    "gpucache",
                    "tmp",
                    "temp",
                    "logs",
                    ".log",
                    "thumbnails",
                    "rebuildable",
                ],
            )
        {
            return RiskAssessment {
                risk: RiskLevel::Green,
                recommendation: Recommendation::Clean,
                reason: "rebuildable cache, log, or temporary data".to_owned(),
                backup_required: false,
            };
        }

        RiskAssessment {
            risk: RiskLevel::Black,
            recommendation: Recommendation::ReportOnly,
            reason: "unknown data is report-only until an adapter proves it safe".to_owned(),
            backup_required: false,
        }
    }
}

fn contains_any(value: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| value.contains(needle))
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{RiskEngine, RiskInput, RiskLevel};
    use crate::models::ItemKind;

    fn classify(path: &str, kind: ItemKind) -> RiskLevel {
        RiskEngine::classify(&RiskInput {
            path: Path::new(path),
            kind,
            evidence: &[],
        })
        .risk
    }

    #[test]
    fn assigns_each_required_risk_level() {
        assert_eq!(
            classify("/tmp/cache/index.bin", ItemKind::Cache),
            RiskLevel::Green
        );
        assert_eq!(
            classify("/tmp/projects/session.jsonl", ItemKind::Session),
            RiskLevel::Yellow
        );
        assert_eq!(classify("/tmp/auth.json", ItemKind::File), RiskLevel::Red);
        assert_eq!(
            classify("/tmp/state.sqlite", ItemKind::Database),
            RiskLevel::Black
        );
    }
}
