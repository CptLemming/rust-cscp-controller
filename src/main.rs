mod app;
mod includes;
mod external;
mod components;

use app::App;

fn main() {
    yew::start_app::<App>();
}
