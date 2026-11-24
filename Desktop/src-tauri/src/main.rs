#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod community;
mod engine;
mod hid;
mod model;
mod protocol;
mod startup;
mod audio;
mod spectrum;
mod streaming;
use community::{Config, Library};
use engine::{Controller, Snapshot};
use model::{Cached, CachedSummary, Sequence};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc, Mutex,
};
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    webview::NewWindowResponse,
    Manager, WebviewUrl, WebviewWindowBuilder,
};

struct Service {
    config: Config,
    library: Library,
    request: AtomicU64,
}
#[tauri::command]
async fn hid_begin(app: tauri::AppHandle, window: tauri::WebviewWindow) -> Result<u64, String> {
    tauri::async_runtime::spawn_blocking(move || {
        app.state::<Controller>().hid_begin(window.label().into())
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn hid_send(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    token: u64,
    raw: Vec<u8>,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        app.state::<Controller>()
            .hid_send(window.label().into(), token, raw)
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
fn hid_end(
    c: tauri::State<Controller>,
    window: tauri::WebviewWindow,
    token: u64,
) -> Result<(), String> {
    c.hid_end(window.label().into(), token)
}
#[tauri::command]
fn hid_poll(c: tauri::State<Controller>, window: tauri::WebviewWindow, token: u64) -> Vec<Vec<u8>> {
    c.hid_poll(window.label(), token)
}
#[tauri::command]
fn hid_detach(c: tauri::State<Controller>, window: tauri::WebviewWindow) {
    c.hid_detach(window.label().into());
}
#[tauri::command]
async fn community_request(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    path: String,
    method: String,
    body: String,
    headers: std::collections::BTreeMap<String, String>,
) -> Result<community::HttpResponse, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let s = app.state::<Service>();
        let url = tauri::Url::parse(&format!("{}{path}", s.config.api_base))
            .map_err(|e| e.to_string())?;
        let cookies = window
            .cookies_for_url(url)
            .map_err(|e| e.to_string())?
            .iter()
            .map(|c| format!("{}={}", c.name(), c.value()))
            .collect::<Vec<_>>()
            .join("; ");
        community::proxy(
            &s.config.api_base,
            &path,
            &method,
            &body,
            &headers,
            &cookies,
        )
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
fn snapshot(c: tauri::State<Controller>) -> Snapshot {
    c.snapshot()
}
#[derive(serde::Serialize)]
struct AudioStatus {
    active: bool,
    fps: f32,
    packets: usize,
    latency_ms: f32,
    idle: bool,
    colors: std::collections::BTreeMap<u8, [u8; 3]>,
    connected: bool,
    error: Option<String>,
}
#[tauri::command]
fn audio_start(c: tauri::State<Controller>, color: Option<[u8; 3]>, fps: Option<u32>, gain: Option<f32>, keep_sending: Option<bool>) -> Result<(), String> {
    let fps = fps.unwrap_or(30);
    let gain = gain.unwrap_or(3.0);
    if !(1..=60).contains(&fps) || !gain.is_finite() || !(0.3..=8.0).contains(&gain) { return Err("音频参数超出范围".into()); }
    c.audio_start(audio::AudioOptions { color: color.unwrap_or([0, 170, 255]), fps, gain, keep_sending: keep_sending.unwrap_or(false) })
}
#[tauri::command]
fn audio_stop(c: tauri::State<Controller>) -> Result<(), String> {
    c.audio_stop()
}
#[tauri::command]
fn audio_status(c: tauri::State<Controller>) -> AudioStatus {
    let (active, error) = c.audio_status();
    let snapshot = c.snapshot();
    let colors = model::keys().iter().enumerate().map(|(i, k)| (k.index,
        if active && snapshot.audio { snapshot.colors.get(i).copied().unwrap_or([0, 0, 0]) } else { [0, 0, 0] }
    )).collect();
    AudioStatus { active, colors, idle: snapshot.audio_idle, fps: snapshot.fps, packets: snapshot.packets, latency_ms: snapshot.audio_latency_ms, connected: snapshot.connected, error }
}
#[tauri::command]
fn list_cached(s: tauri::State<Service>) -> Vec<CachedSummary> {
    s.library.list()
}
#[tauri::command]
fn stop_playback(c: tauri::State<Controller>, s: tauri::State<Service>) -> Result<(), String> {
    s.request.fetch_add(1, Ordering::SeqCst);
    c.stop()
}
#[tauri::command]
fn hide_window(window: tauri::WebviewWindow) -> Result<(), String> {
    window.hide().map_err(|e| e.to_string())
}
#[tauri::command]
fn quit(app: tauri::AppHandle, c: tauri::State<Controller>) {
    c.shutdown();
    app.exit(0);
}
async fn fetch(
    app: tauri::AppHandle,
    code: Option<String>,
    id: Option<String>,
    play: bool,
) -> Result<Cached, String> {
    let revision = app
        .state::<Service>()
        .request
        .fetch_add(1, Ordering::SeqCst)
        + 1;
    tauri::async_runtime::spawn_blocking(move || {
        let s = app.state::<Service>();
        let animation = community::download(&s.config.api_base, code.as_deref(), id.as_deref())?;
        s.library.save(&animation)?;
        if play {
            if s.request.load(Ordering::SeqCst) != revision {
                return Err("已取消过期的播放请求，作品已保存在本机".into());
            }
            app.state::<Controller>().play(animation.clone())?;
        }
        Ok(animation)
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn play_share(app: tauri::AppHandle, code: String) -> Result<Cached, String> {
    fetch(app, Some(code), None, true).await
}
#[tauri::command]
async fn download_share(app: tauri::AppHandle, code: String) -> Result<Cached, String> {
    fetch(app, Some(code), None, false).await
}
#[tauri::command]
async fn play_public(app: tauri::AppHandle, id: String) -> Result<Cached, String> {
    fetch(app, None, Some(id), true).await
}
#[tauri::command]
fn play_sequence(
    c: tauri::State<Controller>,
    s: tauri::State<Service>,
    name: String,
    sequence: Sequence,
) -> Result<(), String> {
    s.request.fetch_add(1, Ordering::SeqCst);
    let a = community::draft(name, sequence)?;
    s.library.save(&a)?;
    c.play(a)
}
#[tauri::command]
fn play_cached(
    c: tauri::State<Controller>,
    s: tauri::State<Service>,
    key: String,
) -> Result<Cached, String> {
    s.request.fetch_add(1, Ordering::SeqCst);
    let a = s.library.read(&key)?;
    c.play(a.clone())?;
    Ok(a)
}
fn show(app: &tauri::AppHandle, label: &str) {
    if let Some(w) = app.get_webview_window(label) {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}
fn bridge_script(config: &Config) -> String {
    format!(
        "window.__F87_DESKTOP_PAGE__={};window.F87_COMMUNITY_API_BASE={};\n{}\n{}\n{}",
        serde_json::json!({"origin":config.frontend.origin().ascii_serialization(),"path":config.frontend.path(),"apiBase":config.api_base}),
        serde_json::json!(config.api_base),
        include_str!("../../ui/hid-bridge.js"),
        include_str!("../../ui/online-bridge.js"),
        include_str!("../../ui/audio-bridge.js")
    )
}
fn popup(
    app: &tauri::AppHandle,
    url: tauri::Url,
    features: tauri::webview::NewWindowFeatures,
) -> NewWindowResponse<tauri::Wry> {
    if url.as_str() != "about:blank" && url.scheme() != "https" {
        return NewWindowResponse::Deny;
    }
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let label = format!("web-popup-{}", NEXT.fetch_add(1, Ordering::SeqCst));
    match WebviewWindowBuilder::new(app, label, WebviewUrl::External(url))
        .window_features(features)
        .title("F87 Pro Desktop")
        .on_navigation(|u| {
            u.scheme() == "https" || u.as_str() == "about:blank"
        })
        .build()
    {
        Ok(window) => NewWindowResponse::Create { window },
        Err(_) => NewWindowResponse::Deny,
    }
}
fn create_driver(app: &tauri::AppHandle) -> tauri::Result<()> {
    if app.get_webview_window("main").is_some() {
        show(app, "main");
        return Ok(());
    }
    let script = bridge_script(&app.state::<Service>().config);
    let popup_app = app.clone();
    WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
        // Register the window and its close handler before it can receive user input.
        .visible(false)
        .title("F87 Pro Desktop")
        .inner_size(1080., 720.)
        .min_inner_size(900., 600.)
        .initialization_script(script)
        .on_navigation(|u| {
            matches!(u.host_str(), Some("tauri.localhost") | Some("localhost"))
                && ["http", "tauri"].contains(&u.scheme())
        })
        .on_new_window(move |u, f| popup(&popup_app, u, f))
        .build()?;
    show(app, "main");
    Ok(())
}
#[tauri::command]
fn open_driver(app: tauri::AppHandle) -> Result<(), String> {
    create_driver(&app).map_err(|e| e.to_string())
}
fn main() {
    let options = startup::LaunchOptions::from_args(std::env::args().skip(1));
    let instance = match startup::Instance::acquire(options).expect("single-instance setup") {
        Some(instance) => instance,
        None => return,
    };
    let open_event = instance.event_value();
    let listener = Arc::new(Mutex::new(None));
    let setup_listener = listener.clone();
    let app = tauri::Builder::default()
        .manage(Controller::new(options.preview_only))
        .manage(Service {
            config: Config::new(),
            library: Library::new(),
            request: AtomicU64::new(0),
        })
        .invoke_handler(tauri::generate_handler![
            snapshot,
            list_cached,
            stop_playback,
            hide_window,
            quit,
            play_share,
            download_share,
            play_public,
            play_sequence,
            play_cached,
            open_driver,
            hid_begin,
            hid_send,
            hid_end,
            hid_poll,
            hid_detach,
            audio_start,
            audio_stop,
            audio_status,
            community_request
        ])
        .setup(move |app| {
            let driver = MenuItem::with_id(app, "driver", "打开F87ProDesktop", true, None::<&str>)?;
            let stop = MenuItem::with_id(app, "stop", "停止后台播放", true, None::<&str>)?;
            let exit = MenuItem::with_id(app, "exit", "退出", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&driver, &stop, &exit])?;
            TrayIconBuilder::new()
                .icon(
                    app.default_window_icon()
                        .expect("embedded receiver icon")
                        .clone(),
                )
                .tooltip("F87 Pro Desktop")
                .menu(&menu)
                .show_menu_on_left_click(true)
                .on_menu_event(|app, e| match e.id.as_ref() {
                    "driver" => {
                        let _ = create_driver(app);
                    }
                    "stop" => {
                        app.state::<Service>()
                            .request
                            .fetch_add(1, Ordering::SeqCst);
                        let _ = app.state::<Controller>().stop();
                    }
                    "exit" => {
                        app.state::<Controller>().shutdown();
                        app.exit(0);
                    }
                    _ => {}
                })
                .build(app)?;
            *setup_listener.lock().unwrap() = Some(startup::SignalListener::start(
                open_event,
                app.handle().clone(),
            ));
            if !options.background {
                create_driver(app.handle())?;
            }
            Ok(())
        })
        .on_window_event(|w, e| {
            if w.label() == "main" {
                if let tauri::WindowEvent::CloseRequested { api, .. } = e {
                    api.prevent_close();
                    let _ = w.hide();
                }
            }
            if let tauri::WindowEvent::Destroyed = e {
                w.state::<Controller>().hid_detach(w.label().into());
            }
        })
        .build(tauri::generate_context!())
        .expect("start F87 desktop");
    app.run(|app, event| {
        if let tauri::RunEvent::Exit = event {
            app.state::<Controller>().shutdown();
        }
    });
    listener.lock().unwrap().take();
    drop(instance);
}
