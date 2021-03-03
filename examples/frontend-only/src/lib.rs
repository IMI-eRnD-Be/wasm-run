// This is needed only of the this in wasm-run.
// You should remove this as it prevents the autocompletion to work properly.
#![cfg(target_arch = "wasm32")]

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(message: &str);
}

#[wasm_bindgen(start)]
pub fn run_app() -> Result<(), JsValue> {
    log("Hello World!");

    Ok(())
}
