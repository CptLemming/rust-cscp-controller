#![allow(non_snake_case, non_camel_case_types)]
use std::sync::Arc;

use num_derive::FromPrimitive;
use slab::Slab;

#[derive(Debug, Clone, Copy, PartialEq, FromPrimitive, serde::Serialize, serde::Deserialize)]
pub enum AudioType {
  U,
  CH,
  GP,
  VCA_MASTER,
  VCA_MASTER_CH,
  VCA_MASTER_GP,
  MN,
  VCA_MASTER_MN,
  TK,
  VCA_MASTER_TK,
  AUX,
  VCA_MASTER_AUX,
}

#[derive(Debug, Clone, Copy, PartialEq, FromPrimitive, serde::Serialize, serde::Deserialize)]
pub enum AudioWidth {
  NP,
  M,
  ST,
  UNUSED1,
  UNUSED2,
  UNUSED3,
  SU,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Fader {
  pub index: u16,
  pub label: String,
  pub level: u16,
  pub isCut: bool,
  pub isPfl: bool,
  pub pathType: AudioType,
  pub format: AudioWidth,
}

impl Fader {
  pub fn new(index: u16) -> Fader {
    Fader { index, label: String::from(""), level: 0, isCut: false, isPfl: false, pathType: AudioType::U, format: AudioWidth::NP }
  }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct DeskInfo {
  pub cscpVersion: u16,
  pub numFaders: u16,
  pub numMains: u16,
  pub name: String,
}

impl DeskInfo {
  pub fn default() -> DeskInfo {
    DeskInfo { cscpVersion:  0, numFaders: 0, numMains: 0, name: String::new() }
  }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DB {
  pub deskInfo: DeskInfo,
  pub faders: Vec<Fader>,
}

impl DB {
  pub fn default() -> DB {
    DB {
      deskInfo: DeskInfo::default(),
      faders: vec![],
    }
  }
}
