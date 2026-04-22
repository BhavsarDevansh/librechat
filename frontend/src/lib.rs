use leptos::prelude::*;
use wasm_bindgen::prelude::*;

#[component]
pub fn App() -> impl IntoView {
    view! {
        <div class="app-root">
            <p>"Hello from Leptos!"</p>
        </div>
    }
}

#[wasm_bindgen(start)]
pub fn run() {
    leptos::mount::mount_to_body(App);
}
