// SPDX-License-Identifier: MIT OR Apache-2.0
//! Diataxis reorganization module

pub mod executor;
pub mod planner;
pub mod scanner;

pub use executor::{ExecutionError, MoveResult, ReorgExecutor};
pub use planner::{FileMove, PlannerError, ReorgPlan, ReorgPlanner};
pub use scanner::{RepositoryScanner, ScanResult, ScannerError};
