use yew::prelude::*;

use crate::components::state::stateManager::StateContext;
use super::fader::*;

#[function_component(RenderFaders)]
pub fn faders() -> Html {
  let state = use_context::<StateContext>().expect("no state context found");

  html!{
    <div class="faders">
      {
          for state.faders.iter().map(|fader| {
              html!{
                <RenderFader fader={fader.clone()} />
              }
          })
      }
    </div>
  }
}
