use lsp_types::Url;

use thiserror::Error;

use std::fmt::Display;

use std::num::ParseIntError;
use std::result;

use serde_json::Error;

pub type Result<T> = result::Result<T, ProtocolError>;

#[derive(Error, Clone, Debug)]
pub enum ProtocolError {
    IDParse(#[from] ParseIntError),
    JSONParse(String),
    Other(String),
}

impl Display for ProtocolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{:?}", self))
    }
}

impl From<Error> for ProtocolError {
    fn from(e: Error) -> Self {
        ProtocolError::JSONParse(format!("{}", e))
    }
}

impl From<anyhow::Error> for ProtocolError {
    fn from(e: anyhow::Error) -> Self {
        ProtocolError::Other(format!("{}", e))
    }
}

#[derive(Clone)]
pub struct Element {
    pub id: u64,
    pub el_type: String,
    pub label: String,
    pub payload: Option<Payload>,
}

#[derive(Clone)]
pub enum Payload {
    Edge(Edge),
    MetaData(MetaData),
    Range(Range),
    Document(Url),
    ResultSet(ResultSet),
    Moniker(Moniker),
    PackageInformation(PackageInformation),
    Diagnostics(Vec<Diagnostic>),
}

#[derive(Clone)]
pub struct Edge {
    pub out_v: u64,
    pub in_v: u64,
    pub in_vs: Vec<u64>,
    pub document: u64,
}

#[derive(Clone)]
pub struct MetaData {
    pub version: String,
    pub project_root: String,
}

#[derive(Clone)]
pub struct Range {
    pub start_line: u32,
    pub start_character: u32,
    pub end_line: u32,
    pub end_character: u32,
}

#[derive(Clone)]
pub struct ResultSet {}

#[derive(Clone)]
pub struct Moniker {
    pub kind: String,
    pub scheme: String,
    pub identifier: String,
}

#[derive(Clone)]
pub struct PackageInformation {
    pub name: String,
    pub version: String,
}

#[derive(Clone)]
pub struct Diagnostic {
    pub severity: u64,
    pub code: String,
    pub message: String,
    pub source: String,
    pub start_line: u64,
    pub start_character: u64,
    pub end_line: u64,
    pub end_character: u64,
}
