mod commands;
mod infrastructure;

use danxi_core::{CampusLifeService, CampusSession, SessionManager};
use infrastructure::token_store::FileSecretStore;
use std::sync::Arc;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    dotenvy::dotenv().ok();
    let secrets = Arc::new(
        FileSecretStore::open()
            .unwrap_or_else(|error| panic!("failed to open secret store: {error}")),
    );
    let session_manager = SessionManager::new(secrets.clone())
        .unwrap_or_else(|error| panic!("failed to initialize session manager: {error}"));
    let campus_session = CampusSession::new(secrets.clone())
        .unwrap_or_else(|error| panic!("failed to initialize campus session: {error}"));
    let campus_life = CampusLifeService::new()
        .unwrap_or_else(|error| panic!("failed to initialize campus services: {error}"));

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(session_manager)
        .manage(campus_session)
        .manage(campus_life)
        .invoke_handler(tauri::generate_handler![
            commands::session::community_login,
            commands::session::community_register,
            commands::session::community_check_email,
            commands::session::community_send_verify_code,
            commands::session::community_logout,
            commands::session::campus_login,
            commands::session::campus_logout,
            commands::session::load_timetable,
            commands::session::load_library_occupancy,
            commands::session::load_dining_crowdedness,
            commands::session::begin_dining_enhanced_auth,
            commands::session::load_campus_buses,
            commands::session::load_empty_classrooms,
            commands::session::session_status,
            commands::session::load_forum_holes,
            commands::session::load_forum_thread,
            commands::session::search_evaluation_courses,
            commands::session::load_evaluation_course,
            commands::session::load_random_evaluation_review
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
