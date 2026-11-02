mod carts;
mod runtime;

use serde::Serialize;
use std::{fs, path::PathBuf};
use tauri::{http, ipc::{InvokeBody, Request, Response}, AppHandle, Manager};
use tauri_plugin_opener::OpenerExt;

const PNG_MAGIC: &[u8] = b"\x89PNG\r\n\x1a\n";

#[derive(Serialize)]
struct CartInfo {
    id: String,
    size: u64,
}

fn carts_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let d = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("carts");
    fs::create_dir_all(&d).map_err(|e| e.to_string())?;
    Ok(d)
}

// Lexaloffle's runtime is not bundled; the user imports it once.
fn runtime_path(app: &AppHandle) -> Result<PathBuf, String> {
    let d = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("runtime");
    fs::create_dir_all(&d).map_err(|e| e.to_string())?;
    Ok(d.join("runtime.js"))
}

#[derive(Serialize)]
struct RuntimeInfo {
    size: u64,
}

#[tauri::command]
fn runtime_status(app: AppHandle) -> Result<Option<RuntimeInfo>, String> {
    let p = runtime_path(&app)?;
    Ok(fs::metadata(p).ok().map(|m| RuntimeInfo { size: m.len() }))
}

// Raw body instead of JSON: the file is ~1.6 MB.
#[tauri::command]
fn import_runtime(app: AppHandle, request: Request<'_>) -> Result<(), String> {
    let data = match request.body() {
        InvokeBody::Raw(d) => d,
        _ => return Err("expected the raw bytes of the runtime file".into()),
    };
    runtime::validate(data)?;
    fs::write(runtime_path(&app)?, data).map_err(|e| e.to_string())
}

#[tauri::command]
fn read_runtime(app: AppHandle) -> Result<Response, String> {
    let p = runtime_path(&app)?;
    Ok(Response::new(fs::read(p).map_err(|_| "runtime not imported yet".to_string())?))
}

#[tauri::command]
fn list_carts(app: AppHandle) -> Result<Vec<CartInfo>, String> {
    let d = carts_dir(&app)?;
    Ok(carts::list(&d)
        .into_iter()
        .map(|(id, size)| CartInfo { id, size })
        .collect())
}

#[tauri::command]
fn read_cart(app: AppHandle, id: String) -> Result<Response, String> {
    let d = carts_dir(&app)?;
    let p = carts::resolve(&d, &id).ok_or_else(|| format!("no cart '{id}'"))?;
    Ok(Response::new(fs::read(p).map_err(|e| e.to_string())?))
}

#[tauri::command]
fn import_cart(app: AppHandle, name: String, data: Vec<u8>) -> Result<String, String> {
    if !data.starts_with(PNG_MAGIC) {
        return Err(format!("{name}: not a PNG cart (.p8.png)"));
    }
    let id = carts::safe_id(&name).ok_or_else(|| format!("bad file name: {name}"))?;
    let d = carts_dir(&app)?;
    fs::write(d.join(format!("{id}{}", carts::EXT)), data).map_err(|e| e.to_string())?;
    Ok(id)
}

#[tauri::command]
fn delete_cart(app: AppHandle, id: String) -> Result<(), String> {
    let d = carts_dir(&app)?;
    let p = carts::resolve(&d, &id).ok_or_else(|| format!("no cart '{id}'"))?;
    fs::remove_file(p).map_err(|e| e.to_string())
}

#[tauri::command]
fn open_carts_dir(app: AppHandle) -> Result<(), String> {
    let d = carts_dir(&app)?;
    app.opener()
        .open_path(d.to_string_lossy().to_string(), None::<&str>)
        .map_err(|e| e.to_string())
}

fn reply(status: u16, ctype: &str, body: Vec<u8>) -> http::Response<Vec<u8>> {
    http::Response::builder()
        .status(status)
        .header("Content-Type", ctype)
        .header("Access-Control-Allow-Origin", "*")
        .header("Cache-Control", "no-store")
        .body(body)
        .unwrap()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        // The page points the runtime's get_cart.php requests here; Windows uses http://p8cart.localhost/.
        .register_uri_scheme_protocol("p8cart", |ctx, request| {
            let lid = request
                .uri()
                .query()
                .and_then(|q| {
                    url::form_urlencoded::parse(q.as_bytes())
                        .find(|(k, _)| k == "lid")
                        .map(|(_, v)| v.into_owned())
                })
                .unwrap_or_default();
            let found = carts_dir(ctx.app_handle())
                .ok()
                .and_then(|d| carts::resolve(&d, &lid))
                .and_then(|p| fs::read(p).ok());
            match found {
                Some(bytes) => reply(200, "image/png", bytes),
                None => {
                    eprintln!("cart NOT FOUND: lid={lid:?}");
                    reply(404, "text/plain; charset=utf-8", format!("No cart for lid={lid}\n").into_bytes())
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            list_carts,
            read_cart,
            import_cart,
            delete_cart,
            open_carts_dir,
            runtime_status,
            import_runtime,
            read_runtime
        ])
        .run(tauri::generate_context!())
        .expect("error while running PICO-8 Player");
}
