use seraph_types::{Knowledge, Models};
use std::fs;
use std::path::Path;

pub fn read_knowledge_json(path: impl AsRef<Path>) -> Result<Knowledge, String> {
    let raw = fs::read_to_string(path.as_ref()).map_err(|err| err.to_string())?;
    serde_json::from_str(&raw).map_err(|err| err.to_string())
}

pub fn write_models_json(path: impl AsRef<Path>, models: &Models) -> Result<(), String> {
    let json = serde_json::to_string_pretty(models).map_err(|err| err.to_string())?;
    if let Some(parent) = path.as_ref().parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    fs::write(path.as_ref(), json).map_err(|err| err.to_string())
}
