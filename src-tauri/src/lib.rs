//! BloxClient native backend.

mod auth;
mod autostart;
mod commands;
mod db;
mod deployments;
mod error;
mod fflags;
mod library;
mod paths;
mod performance;
mod roblox;
mod settings;
mod state;
mod tray;
mod vault;
mod window;

use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Duration;

use tauri::{App, Manager, WindowEvent};
use tauri_plugin_log::{RotationStrategy, Target, TargetKind};

use auth::AuthManager;
use db::Database;
use deployments::DeploymentManager;
use error::AppError;
use fflags::FlagManager;
use library::Library;
use paths::AppPaths;
use performance::PerformanceMonitor;
use roblox::{spawn_player_watcher, Catalog, PeopleManager, PlayHistorySync, SessionManager};
use settings::SettingsStore;
use state::AppState;

/// How long the window may stay hidden if the frontend never reports ready.
const REVEAL_FALLBACK: Duration = Duration::from_secs(5);
const MAX_LOG_BYTES: u128 = 1_000_000;

pub fn run() {
    let result = tauri::Builder::default()
        // Must be registered first so a second launch exits before doing any work.
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            window::focus_main(app);
            auth::handle_launch_args(app, &args);
        }))
        .plugin(log_plugin())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            if let Err(error) = initialize(app) {
                log::error!("startup failed: {error}");
                window::show_fatal_error(&error.user_message());
                return Err(Box::new(error));
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            match event {
                WindowEvent::CloseRequested { api, .. } if window::should_hide_on_close(window) => {
                    api.prevent_close();
                    if let Err(error) = window.hide() {
                        log::warn!("could not hide window: {error}");
                    }
                    window::sync_animation(window.app_handle());
                }
                WindowEvent::Resized(_) | WindowEvent::Focused(_) => {
                    window::sync_animation(window.app_handle());
                }
                _ => {}
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::settings::get_bootstrap,
            commands::settings::update_settings,
            commands::settings::reset_settings,
            commands::auth::get_auth_status,
            commands::auth::set_oauth_client_id,
            commands::auth::start_sign_in,
            commands::auth::cancel_sign_in,
            commands::auth::sign_out,
            commands::auth::open_oauth_dashboard,
            commands::auth::dismiss_welcome,
            commands::deployments::list_deployments,
            commands::deployments::validate_deployment,
            commands::deployments::install_deployment,
            commands::deployments::cancel_deployment_install,
            commands::deployments::remove_deployment,
            commands::deployments::launch_deployment,
            commands::fflags::get_fflags,
            commands::fflags::set_active_fflag_profile,
            commands::fflags::create_fflag_profile,
            commands::fflags::rename_fflag_profile,
            commands::fflags::delete_fflag_profile,
            commands::fflags::save_fflag,
            commands::fflags::set_fflag_enabled,
            commands::fflags::remove_fflag,
            commands::fflags::import_fflags,
            commands::fflags::get_fflags_json,
            commands::fflags::export_fflags_file,
            commands::fflags::reset_fflags,
            commands::fflags::restore_fflag_backup,
            commands::library::get_library,
            commands::library::set_game_favorite,
            commands::library::set_game_pinned,
            commands::library::create_collection,
            commands::library::rename_collection,
            commands::library::delete_collection,
            commands::library::set_game_in_collection,
            commands::library::clear_library,
            commands::people::get_people,
            commands::people::add_local_profile,
            commands::people::remove_local_profile,
            commands::people::set_active_local_profile,
            commands::people::add_person,
            commands::people::remove_person,
            commands::people::clear_people,
            commands::people::open_user_profile,
            commands::performance::get_performance_history,
            commands::performance::sample_performance,
            commands::performance::check_network,
            commands::roblox::get_discover_feed,
            commands::roblox::search_experiences,
            commands::roblox::get_experience,
            commands::roblox::get_roblox_install,
            commands::roblox::launch_experience,
            commands::roblox::get_play_session,
            commands::roblox::list_recent_experiences,
            commands::roblox::sync_play_history,
            commands::roblox::clear_recent_experiences,
            commands::system::app_ready,
            commands::system::get_background_animation,
            commands::system::get_diagnostics,
            commands::system::open_app_folder,
        ])
        .run(tauri::generate_context!());

    if let Err(error) = result {
        log::error!("application error: {error}");
    }
}

fn log_plugin() -> tauri::plugin::TauriPlugin<tauri::Wry> {
    let file_name = Some("bloxclient".to_string());
    let file_target = match paths::debug_data_override() {
        Some(dir) => TargetKind::Folder {
            path: dir.join("logs"),
            file_name,
        },
        None => TargetKind::LogDir { file_name },
    };

    let mut builder = tauri_plugin_log::Builder::new()
        .clear_targets()
        .target(Target::new(file_target))
        .level(log::LevelFilter::Info)
        .level_for("tao", log::LevelFilter::Warn)
        .level_for("wry", log::LevelFilter::Warn)
        .max_file_size(MAX_LOG_BYTES)
        .rotation_strategy(RotationStrategy::KeepSome(3));

    if cfg!(debug_assertions) {
        builder = builder.target(Target::new(TargetKind::Stdout));
    }
    builder.build()
}

fn initialize(app: &mut App) -> Result<(), AppError> {
    let paths = AppPaths::resolve(app.handle())?;
    let db = Arc::new(Database::open(&paths.database)?);
    if let Err(error) = roblox::history::migrate_plaintext(&db) {
        log::error!("could not encrypt existing play history: {error}");
    }
    let settings = SettingsStore::load(Arc::clone(&db))?;

    // Registered values go stale when the app moves, and users can remove the
    // entry from Windows. Only development builds skip this check.
    if cfg!(not(debug_assertions)) {
        autostart::reconcile(&settings, &app.package_info().version.to_string())?;
    }

    let general = settings.get().general;
    tray::sync(app.handle(), general.minimize_to_tray)?;

    let deployments = Arc::new(DeploymentManager::new(
        Arc::clone(&db),
        paths.deployments.clone(),
        paths.roblox_versions.clone(),
    )?);
    let fflags = Arc::new(FlagManager::new(Arc::clone(&db)));
    let library = Library::new(Arc::clone(&db));
    let people = PeopleManager::new(Arc::clone(&db))?;
    let performance = Arc::new(PerformanceMonitor::new()?);
    let catalog = Catalog::new()?;
    let sessions = Arc::new(SessionManager::new(Arc::clone(&db)));
    let play_history = PlayHistorySync::new(Arc::clone(&db), paths.roblox_logs.clone());
    let auth = Arc::new(AuthManager::new(Arc::clone(&db))?);

    app.manage(AppState {
        db,
        settings,
        deployments,
        fflags,
        library,
        people,
        performance,
        catalog,
        sessions,
        play_history,
        auth,
        paths,
        revealed: AtomicBool::new(false),
    });
    spawn_player_watcher(app.handle().clone());

    let handle = app.handle().clone();
    std::thread::spawn(move || {
        std::thread::sleep(REVEAL_FALLBACK);
        window::reveal_on_launch(&handle);
    });

    log::info!("BloxClient {} started", app.package_info().version);
    Ok(())
}
