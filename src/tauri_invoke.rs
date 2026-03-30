use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::JsFuture;

/// Invoke a Tauri command and return the raw JsValue.
/// Returns Err(String) on any failure.
pub async fn invoke(js_cmd: String, args: &js_sys::Object) -> Result<JsValue, String> {
    let window = web_sys::window()
        .ok_or_else(|| "failed to get window".to_string())?;

    let tauri = js_sys::Reflect::get(&window, &"__TAURI__".into())
        .map_err(|e| format!("__TAURI__ error: {:?}", e))?;

    if tauri.is_undefined() {
        return Err("Only available in Tauri desktop app".to_string());
    }

    let core = js_sys::Reflect::get(&tauri, &"core".into())
        .map_err(|e| format!("core error: {:?}", e))?;
    let invoke_fn = js_sys::Reflect::get(&core, &"invoke".into())
        .map_err(|e| format!("invoke error: {:?}", e))?;

    let args_arr = js_sys::Array::new();
    args_arr.push(&js_cmd.into());
    args_arr.push(args);

    let promise: js_sys::Promise = js_sys::Reflect::apply(&invoke_fn.into(), &JsValue::UNDEFINED, &args_arr)
        .map_err(|e| format!("apply error: {:?}", e))?
        .dyn_into()
        .map_err(|e| format!("not a promise: {:?}", e))?;

    JsFuture::from(promise).await.map_err(|e| format!("promise error: {:?}", e))
}