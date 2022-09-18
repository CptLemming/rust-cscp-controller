#![allow(non_snake_case, non_camel_case_types)]

use yew::prelude::*;

use crate::components::state::stateManager::*;
use crate::components::faders::faders::*;
use crate::components::info::info::*;

#[function_component(App)]
pub fn app() -> Html {
    html! {
        <StateManager>
            <div class="app">
                <Info />
                <RenderFaders />
            </div>
        </StateManager>
    }
}
