#![allow(clippy::missing_errors_doc)]

use std::path::PathBuf;

use vibe_hauler_core::{CleanManifest, CleanPlan};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExecutionMode {
    DryRun,
    Execute,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RestoreRequest {
    pub manifest_path: PathBuf,
    pub force: bool,
}

pub trait PlanExecutor {
    type Error;

    fn execute(&self, plan: &CleanPlan, mode: ExecutionMode) -> Result<CleanManifest, Self::Error>;
}

pub trait Restorer {
    type Error;

    fn restore(&self, request: RestoreRequest) -> Result<(), Self::Error>;
}
