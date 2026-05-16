mod api;
mod components;
pub mod history;
mod sse;
mod state;

use components::chat::ChatView;
use components::settings::SettingsModal;
use components::sidebar::Sidebar;
use leptos::prelude::*;
use state::AppState;
use wasm_bindgen::prelude::*;

#[component]
pub fn App() -> impl IntoView {
    AppState::provide();

    view! {
        <div class="app-layout">
            <Sidebar />
            <main class="chat-main">
                <ChatView />
            </main>
            <SettingsModal />
        </div>
    }
}

#[wasm_bindgen(start)]
pub fn run() {
    leptos::mount::mount_to_body(App);
}
