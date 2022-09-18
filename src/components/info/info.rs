use yew::prelude::*;

use crate::{components::state::stateManager::StateContext};

#[function_component(Info)]
pub fn info() -> Html {
  let state = use_context::<StateContext>().expect("no state context found");


  html!{
    <div class="desk_info">
      <p>{format!("CSCP Version {}", state.deskInfo.cscpVersion)}</p>
      <p>{format!("Name {}", &state.deskInfo.name)}</p>
    </div>
  }
}
