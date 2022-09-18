use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct SetFaderLevelArgs {
    pub index: u16,
    pub level: u16,
}

#[derive(Serialize, Deserialize)]
pub struct SetFaderCutArgs {
    pub index: u16,
    pub isCut: bool,
}

#[derive(Serialize, Deserialize)]
pub struct SetFaderPflArgs {
    pub index: u16,
    pub isPfl: bool,
}
