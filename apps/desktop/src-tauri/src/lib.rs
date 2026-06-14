mod commands;
mod db;
mod services;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let app_data_dir = app.path().app_data_dir()?;
            let db_state = db::initialize(app_data_dir)?;
            app.manage(db_state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::projects::get_project,
            commands::agents::list_agents,
            commands::agent_runs::list_agent_runs,
            commands::agent_runs::list_runtime_events,
            commands::tasks::list_tasks,
            commands::tasks::create_task,
            commands::tasks::update_task_status,
            commands::approvals::list_approvals,
            commands::approvals::create_approval,
            commands::approvals::approve_approval,
            commands::approvals::reject_approval,
            commands::approvals::patch_only_approval
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
