use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;
use common::{Fader, AudioType};

use crate::{external::invoke, includes::commands::{SetFaderLevelArgs, SetFaderCutArgs, SetFaderPflArgs}};

#[derive(Properties, PartialEq)]
pub struct RenderFaderProps {
    pub fader: Fader,
}

#[function_component(RenderFader)]
pub fn fader(props: &RenderFaderProps) -> Html {
  let index = props.fader.index;
  let setFaderHigh = {
      Callback::from(move |_| {
          spawn_local(async move {
              invoke(
                  "setFaderLevel",
                  JsValue::from_serde(&SetFaderLevelArgs { index, level: 1023 }).unwrap(),
              )
              .await;
          });
      })
  };

  let setFaderLow = {
      Callback::from(move |_| {
          spawn_local(async move {
              invoke(
                  "setFaderLevel",
                  JsValue::from_serde(&SetFaderLevelArgs { index, level: 0 }).unwrap(),
              )
              .await;
          });
      })
  };

  let toggleFaderCut = {
    let isCut = props.fader.isCut;
      Callback::from(move |_| {
          spawn_local(async move {
              invoke(
                  "setFaderCut",
                  JsValue::from_serde(&SetFaderCutArgs { index, isCut: !isCut }).unwrap(),
              )
              .await;
          });
      })
  };

  let toggleFaderPfl = {
    let isPfl = props.fader.isPfl;
      Callback::from(move |_| {
          spawn_local(async move {
              invoke(
                  "setFaderPfl",
                  JsValue::from_serde(&SetFaderPflArgs { index, isPfl: !isPfl }).unwrap(),
              )
              .await;
          });
      })
  };

  let mut pflButtonClasses = classes!("pfl");

  if props.fader.isPfl {
    pflButtonClasses.push("pfl__active");
  }

  let mut cutButtonClasses = classes!("cut");

  if !props.fader.isCut {
    cutButtonClasses.push("cut__active");
  }

  html!{
      <div class="fader">
        <p>{format!("F{}", props.fader.index + 1)}</p>
        <p>{&props.fader.label}</p>
        <p>{&props.fader.level}</p>
        <div class="fader__controls">
          <button type="button" onclick={setFaderHigh}>{"HIGH"}</button>
          <button type="button" onclick={setFaderLow}>{"LOW"}</button>
          <button type="button" class={pflButtonClasses} onclick={toggleFaderPfl}>{"PFL"}</button>
          if !matches!(props.fader.pathType, AudioType::MN) { <button type="button" class={cutButtonClasses} onclick={toggleFaderCut}>{"CUT"}</button>} 
        </div>
      </div>
  }
}
