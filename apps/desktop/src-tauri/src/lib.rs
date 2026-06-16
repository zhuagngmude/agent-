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
            commands::model_gateway::request_project_plan_model_draft,
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
            commands::approvals::patch_only_approval,
            commands::auto_swarm::auto_run_swarm_idea,
            commands::project_plan::create_project_plan_draft,
            commands::project_plan::approve_project_plan,
            commands::project_plan::auto_generate_project_plan_tasks,
            commands::project_plan::list_project_plan_drafts,
            commands::project_plan::delete_project_plan_draft,
            commands::project_plan::list_runner_requests,
            commands::project_plan::save_project_plan_model_draft,
            commands::project_plan::list_project_plan_task_templates,
            commands::project_plan::update_project_plan_task_template,
            commands::project_plan::get_project_plan_execution_preview,
            commands::runner_preflight::create_runner_preflight_review,
            commands::runner_preflight::list_runner_preflight_reviews,
            commands::runner_execution_gate::create_runner_execution_gate,
            commands::runner_execution_gate::list_runner_execution_gates,
            commands::runner_execution_gate::revoke_runner_execution_gate,
            commands::runner_dry_run::create_runner_dry_run,
            commands::runner_dry_run::list_runner_dry_runs,
            commands::runner_dry_run::revoke_runner_dry_run,
            commands::runner_execution_lock::create_runner_execution_lock,
            commands::runner_execution_lock::list_runner_execution_locks,
            commands::runner_execution_lock::revoke_runner_execution_lock,
            commands::runner_minimal_run::create_runner_minimal_run,
            commands::runner_minimal_run::list_runner_minimal_runs,
            commands::model_catalog::list_project_plan_models,
            commands::model_catalog::update_project_plan_model_enabled,
            commands::idea_guidance::create_idea_guidance_questions,
            commands::idea_guidance::generate_project_seed,
            commands::idea_guidance::save_guidance_answers,
            commands::idea_guidance::list_project_seeds,
            commands::project_intake::classify_project_intake,
            commands::project_intake::list_project_intakes
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
