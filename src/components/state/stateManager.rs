use std::{collections::HashMap, rc::Rc, cmp::Ordering};

use common::{Fader, DB, DeskInfo, AudioWidth};
use yew::prelude::*;
use wasm_bindgen_futures::spawn_local;
use wasm_bindgen::prelude::*;

use crate::{external::{log, listen, invoke}, includes::events::FaderChangedEvent};

#[derive(Properties, PartialEq)]
pub struct AgentProps {
    #[prop_or_default]
    pub children: Children,
}

struct FadersState {
  faders: HashMap<u16, Fader>,
}

enum FaderAction {
  INSERT(u16, Fader),
  INSERT_BULK(Vec<Fader>),
}

impl Default for FadersState {
  fn default() -> Self {
      Self { faders: HashMap::default() }
  }
}

impl Reducible for FadersState {
  /// Reducer Action Type
  type Action = FaderAction;

  /// Reducer Function
  fn reduce(self: Rc<Self>, action: Self::Action) -> Rc<Self> {
      let next_faders = match action {
        FaderAction::INSERT(id, fader) => {
          let mut nextFaders: HashMap<u16, Fader> = HashMap::default();

          for (id, fader) in self.faders.iter() {
            nextFaders.insert(id.clone(), fader.clone());
          }

          nextFaders.insert(id, fader);
          nextFaders
        },
        FaderAction::INSERT_BULK(faders) => {
          let mut nextFaders: HashMap<u16, Fader> = HashMap::default();
          
          for fader in faders.iter() {
            nextFaders.insert(fader.index, fader.clone());
          }

          nextFaders
        }
      };

      Self { faders: next_faders }.into()
  }
}


struct DeskInfoState {
  deskInfo: DeskInfo,
}

enum DeskInfoAction {
  INSERT(DeskInfo),
}

impl Default for DeskInfoState {
  fn default() -> Self {
      Self { deskInfo: DeskInfo::default() }
  }
}

impl Reducible for DeskInfoState {
  /// Reducer Action Type
  type Action = DeskInfoAction;

  /// Reducer Function
  fn reduce(self: Rc<Self>, action: Self::Action) -> Rc<Self> {
      let next_desk_info = match action {
        DeskInfoAction::INSERT(deskInfo) => {
          deskInfo
        }
      };

      Self { deskInfo: next_desk_info }.into()
  }
}

#[derive(Clone, Debug, PartialEq)]
pub struct StateContext {
    pub faders: Vec<Fader>,
    pub deskInfo: DeskInfo,
}

#[function_component(StateManager)]
pub fn stateManager(props: &AgentProps) -> Html {
  let faders = use_reducer(FadersState::default);
  let deskInfo = use_reducer(DeskInfoState::default);

  let handler_faders = faders.clone();
  let fader_changed_handler_ref = use_ref(|| Closure::new(move |ev: JsValue| {
      let fader_event: FaderChangedEvent = JsValue::into_serde(&ev).unwrap();
      log(format!("Fader event :: {} faderNum={} level={}", fader_event.event, fader_event.payload.index, fader_event.payload.level).to_string().as_str());

      handler_faders.dispatch(FaderAction::INSERT(fader_event.payload.index, fader_event.payload));
  }));

  {
    let deskInfo = deskInfo.clone();
    let faders = faders.clone();
    use_effect_with_deps(move |_| {
      let deskInfo = deskInfo.clone();
      let faders = faders.clone();
        spawn_local(async move {
            listen("fader::changed", &fader_changed_handler_ref).await;
        });

        spawn_local(async move {
          log("Send get DB");
          let new_msg = invoke(
              "getDatabase",
              JsValue::default(),
          )
          .await;
          let db: DB = JsValue::into_serde(&new_msg).unwrap();
          // log(&db.faders.len().to_string());

          log("Set new desk info");
          deskInfo.dispatch(DeskInfoAction::INSERT(db.deskInfo));

          log("Set new faders");
          faders.dispatch(FaderAction::INSERT_BULK(db.faders));
        });

        || {}
    }, 0);
  }

  let mut faders: Vec<Fader> = faders.faders
    .iter()
    .filter(|(_, fader)| !matches!(fader.format, AudioWidth::NP))
    .map(|(_, fader)| fader.clone())
    .collect();
  faders.sort_by(|a, b| b.index.cmp(&a.index));
  faders.sort_by(|a, b| {
      if a.index < b.index {
          Ordering::Less
      } else if a.index == b.index {
          Ordering::Equal
      } else {
          Ordering::Greater
      }
  });
  let state = StateContext { deskInfo: deskInfo.deskInfo.clone(), faders };

  html! {
    <ContextProvider<StateContext> context={state.clone()}>
      { for props.children.iter() }
    </ContextProvider<StateContext>>
  }
}
