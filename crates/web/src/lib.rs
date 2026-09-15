#![forbid(unsafe_code)]
#![recursion_limit = "512"]

pub mod app;
pub mod client;
pub mod language;
pub mod recipe;
pub mod workspace;

#[cfg(feature = "ssr")]
pub mod server;

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    console_error_panic_hook::set_once();
    leptos::mount::hydrate_body(app::App);
}
