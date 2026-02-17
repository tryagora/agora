fn main() {
    tauri::builder::default()
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
