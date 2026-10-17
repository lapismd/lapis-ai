fn main() {
    tauri_plugin::Builder::new(&[
        "targets", "list", "open", "submit", "snapshot", "attach", "detach", "ack", "close",
    ])
    .build();
}
