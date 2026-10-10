use super::*;
use crate::{
    config::GeneralConfig,
    protocol::{CellInput, CellKind, CellOutput, CellStatus, Dialect, HostPlatform},
};
use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Metadata {
    pub version: u32,
    pub binding: CheckpointBinding,
    pub general: serde_json::Value,
    pub system_language: crate::config::Language,
    pub platform: HostPlatform,
    pub output_serial: u64,
    pub title: String,
    pub cells: Vec<CellData>,
    pub owners: Vec<(u32, String)>,
    pub contexts: Vec<ContextData>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CellData {
    pub input: InputData,
    pub output: Option<serde_json::Value>,
    pub status: CellStatus,
    pub defines: Vec<u32>,
    pub uses: Vec<u32>,
    pub exec_count: Option<u32>,
    pub records: Vec<RecordData>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct InputData {
    pub id: String,
    pub kind: CellKind,
    pub source: String,
    pub dialect: Dialect,
}
impl InputData {
    pub fn source_input(&self) -> CellInput {
        CellInput {
            id: self.id.clone(),
            kind: self.kind,
            source: self.source.clone(),
            dialect: self.dialect,
        }
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RecordData {
    pub input: u32,
    pub value: u32,
    pub steps: Option<om_solve::checkpoint::StepsData>,
    pub suppressed: bool,
    pub out_index: u32,
    pub solver: Option<SolverData>,
    pub scientific: Option<ScientificData>,
    pub view_id: String,
    pub exploration: Option<u32>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SolverData {
    pub name: String,
    pub source: u32,
    pub variables: Vec<u32>,
    pub value: u32,
    pub set: om_solve::checkpoint::SolutionsData,
    pub domain: u8,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ScientificData {
    pub name: String,
    pub value: u32,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ContextData {
    pub expression: u32,
    pub controls: Vec<ControlData>,
    pub offset: u32,
    pub length: u32,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ControlData {
    pub name: String,
    pub lower: u64,
    pub upper: u64,
    pub initial: u64,
}
pub(crate) fn general(value: &serde_json::Value) -> Result<GeneralConfig, CheckpointError> {
    let decoded: GeneralConfig =
        serde_json::from_value(value.clone()).map_err(|_| CheckpointError::Invalid)?;
    if serde_json::to_value(&decoded).map_err(|_| CheckpointError::Invalid)? != *value {
        return Err(CheckpointError::Invalid);
    }
    Ok(decoded)
}
pub(crate) fn output(value: serde_json::Value) -> Result<CellOutput, CheckpointError> {
    let decoded: CellOutput =
        serde_json::from_value(value.clone()).map_err(|_| CheckpointError::Invalid)?;
    if serde_json::to_value(&decoded).map_err(|_| CheckpointError::Invalid)? != value {
        return Err(CheckpointError::Invalid);
    }
    Ok(decoded)
}
