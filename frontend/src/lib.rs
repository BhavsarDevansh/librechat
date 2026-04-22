use leptos::prelude::*;
use wasm_bindgen::prelude::*;

#[component]
pub fn App() -> impl IntoView {
    view! {
        <p>"Hello from Leptos!"</p>
    }
}

#[wasm_bindgen(start)]
pub fn run() {
    leptos::mount::mount_to_body(App);
}
