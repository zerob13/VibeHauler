#![allow(clippy::missing_errors_doc)]

use std::path::PathBuf;

use vibe_hauler_core::AgentSession;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParserInput {
    pub path: PathBuf,
    pub app_hint: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ParserSupport {
    Unsupported,
    Maybe,
    Supported,
}

pub trait SessionParser: Send + Sync {
    fn name(&self) -> &'static str;

    fn supports(&self, input: &ParserInput) -> ParserSupport;

    fn parse(&self, input: ParserInput) -> anyhow::Result<Vec<AgentSession>>;
}

pub mod indexeddb;
pub mod jsonl;
pub mod leveldb;
pub mod markdown;
pub mod sqlite;
