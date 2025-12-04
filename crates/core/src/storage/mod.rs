// SPDX-License-Identifier: MIT OR Apache-2.0
//! Storage module for PostgreSQL with pgvector

pub mod models;
pub mod postgres;

pub use models::{DiataxisType, Document, DocumentMetadata};
pub use postgres::PostgresStorage;
