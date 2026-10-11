//! Simple Motion inside SimpleHub. The page mounts the app on a canvas it owns and hands over a
//! bridge object (`open(purpose): Promise<{name, bytes}|null>`, `save(name, bytes): Promise<void>`)
//! that moves files to and from Simple Cloud: File ▸ Open / Import and every save, export and render
//! output go through it instead of the browser's file picker and downloads. Browser storage (the
//! session snapshot, imported media, auto-saves) stays as in the standalone build.
//!
//! `open` receives what the app is asking for as its only argument, so the host may filter file
//! types: `open` (a project), `footage` (media to import), `preset`, `animation`, `timeline`,
//! `template`, `script`, `plugin`, `model`, `guides`, `data` or `file`. Whatever comes back is put
//! in the file table and routed by extension (`.ecproj` opens a project, anything else imports).

use std::cell::RefCell;

use wasm_bindgen::JsCast as _;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

#[wasm_bindgen(module = "/js/host.js")]
extern "C" {
    #[wasm_bindgen(js_name = moduleBase)]
    fn module_base() -> String;
    #[wasm_bindgen(js_name = setWorkerGlue)]
    fn set_worker_glue(name: &str);
}

/// The wasm-bindgen glue as Trunk names it (the workers import it next to `worker.js`).
const GLUE: &str = "effectcraft-web.js";

thread_local! {
    static BRIDGE: RefCell<Option<JsValue>> = const { RefCell::new(None) };
}

fn bridge() -> Option<JsValue> {
    BRIDGE.with(|b| b.borrow().clone())
}

/// A running instance. SimpleHub calls `destroy` when the module unmounts.
#[wasm_bindgen]
pub struct MotionHandle {
    runner: eframe::WebRunner,
}

#[wasm_bindgen]
impl MotionHandle {
    pub fn destroy(&self) {
        self.runner.destroy();
    }

    pub fn has_panicked(&self) -> bool {
        self.runner.has_panicked()
    }
}

/// Starts Simple Motion on `canvas`, with `bridge` providing open and save.
#[wasm_bindgen]
pub async fn start_simple_motion(canvas: web_sys::HtmlCanvasElement, bridge: JsValue) -> Result<MotionHandle, JsValue> {
    eframe::WebLogger::init(log::LevelFilter::Info).ok();
    BRIDGE.with(|b| *b.borrow_mut() = Some(bridge));
    set_worker_glue(GLUE);
    // The workers instantiate this compiled module instead of fetching the .wasm again.
    if let Some(w) = web_sys::window() {
        let _ = js_sys::Reflect::set(&w, &JsValue::from_str("effectcraftModule"), &wasm_bindgen::module());
    }
    let runner = crate::run(canvas, crate::Boot { empty: false, home: false }).await?;
    Ok(MotionHandle { runner })
}

/// Where `worker.js`, `audio-worklet.js` and the glue live: an optional `base` on the bridge, else
/// the folder the glue was served from (the page's own URL is SimpleHub's, not the bundle's).
pub(crate) fn base_url() -> String {
    bridge()
        .and_then(|b| js_sys::Reflect::get(&b, &JsValue::from_str("base")).ok())
        .and_then(|v| v.as_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(module_base)
}

/// What the app is asking for, from the extensions a file dialog would filter on.
pub(crate) fn purpose_for(exts: &[&str]) -> &'static str {
    let has = |e: &str| exts.iter().any(|x| x.eq_ignore_ascii_case(e));
    if has("ecproj") {
        "open"
    } else if has("mp4") || has("png") || has("wav") {
        "footage"
    } else if has("ecpreset") {
        "preset"
    } else if has("lottie") {
        "animation"
    } else if has("fcpxml") || has("otio") || has("edl") {
        "timeline"
    } else if has("ectemplate") {
        "template"
    } else if has("jsx") {
        "script"
    } else if has("wasm") || has("wat") {
        "plugin"
    } else if has("pt") || has("task") {
        "model"
    } else if has("csv") {
        "data"
    } else if exts.len() == 1 && has("json") {
        "guides"
    } else {
        "file"
    }
}

/// Ask the host for one file; `then` gets its table path once the bytes are stored. Nothing
/// happens when the user cancels.
pub(crate) fn open(purpose: &str, then: impl FnOnce(Vec<String>) + 'static) {
    let Some(bridge) = bridge() else {
        log::warn!("SimpleHub bridge not installed: cannot open");
        return;
    };
    let purpose = purpose.to_string();
    wasm_bindgen_futures::spawn_local(async move {
        match call_open(&bridge, &purpose).await {
            Ok(Some((name, bytes))) => {
                let path = crate::files::path_for(&name);
                crate::files::put(&path, bytes.into());
                then(vec![path]);
            }
            Ok(None) => {}
            Err(e) => log::error!("SimpleHub open ({purpose}) failed: {e:?}"),
        }
    });
}

/// Hand `bytes` to the host as the file `name` (fire and forget; a rejected promise is logged).
pub(crate) fn save(name: &str, bytes: &[u8]) -> Result<(), JsValue> {
    let bridge = bridge().ok_or_else(|| JsValue::from_str("SimpleHub bridge not installed"))?;
    let name = name.to_string();
    let bytes = js_sys::Uint8Array::from(bytes);
    wasm_bindgen_futures::spawn_local(async move {
        if let Err(e) = call_save(&bridge, &name, &bytes).await {
            log::error!("SimpleHub save of {name} failed: {e:?}");
        }
    });
    Ok(())
}

fn method(bridge: &JsValue, name: &str) -> Result<js_sys::Function, JsValue> {
    js_sys::Reflect::get(bridge, &JsValue::from_str(name))?
        .dyn_into::<js_sys::Function>()
        .map_err(|_| JsValue::from_str(&format!("bridge.{name} is not a function")))
}

async fn call_open(bridge: &JsValue, purpose: &str) -> Result<Option<(String, Vec<u8>)>, JsValue> {
    let promise: js_sys::Promise = method(bridge, "open")?.call1(bridge, &JsValue::from_str(purpose))?.dyn_into()?;
    let result = JsFuture::from(promise).await?;
    if result.is_null() || result.is_undefined() {
        return Ok(None);
    }
    let name = js_sys::Reflect::get(&result, &JsValue::from_str("name"))?.as_string().unwrap_or_else(|| "Untitled Project.ecproj".into());
    let bytes: js_sys::Uint8Array = js_sys::Reflect::get(&result, &JsValue::from_str("bytes"))?.dyn_into()?;
    Ok(Some((name, bytes.to_vec())))
}

async fn call_save(bridge: &JsValue, name: &str, bytes: &js_sys::Uint8Array) -> Result<(), JsValue> {
    let ret = method(bridge, "save")?.call2(bridge, &JsValue::from_str(name), bytes)?;
    if let Ok(promise) = ret.dyn_into::<js_sys::Promise>() {
        JsFuture::from(promise).await?;
    }
    Ok(())
}
