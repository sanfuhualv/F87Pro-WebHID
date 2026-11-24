fn main() {
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "snapshot",
            "play_share",
            "download_share",
            "play_public",
            "play_sequence",
            "play_cached",
            "list_cached",
            "stop_playback",
            "hide_window",
            "quit",
            "open_driver",
            "open_github",
            "hid_begin",
            "hid_send",
            "hid_end",
            "hid_poll",
            "hid_detach",
            "community_request",
            "audio_start",
            "audio_stop",
            "audio_status",
        ]),
    ))
    .expect("build desktop permissions");
}
