use common::Fader;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct FaderChangedEvent {
    pub event: String,
    pub payload: Fader,
}
