mod api;
mod components;

use components::chat::ChatView;
use leptos::prelude::*;
use wasm_bindgen::prelude::*;

#[component]
pub fn App() -> impl IntoView {
    view! {
        <div class="app-root">
            <ChatView />
        </div>
    }
}

#[wasm_bindgen(start)]
pub fn run() {
    leptos::mount::mount_to_body(App);
}
