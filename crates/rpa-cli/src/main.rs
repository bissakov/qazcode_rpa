use clap::Parser;
use indexmap::IndexMap;
use rpa_core::{
    LogEntry, LogLevel, Project, ProjectFile, UiConstants, UiState, VariableValue,
    execute_project_with_typed_vars, execute_scenario_with_vars,
};
use rpa_core::variables::VarEvent;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

#[derive(Parser)]
#[command(name = "rpa-cli")]
#[command(about = "QazCode RPA CLI - Execute RPA projects from command line", long_about = None)]
struct Cli {
    #[arg(value_name = "FILE", help = "Path to the .rpa project file")]
    project_file: PathBuf,

    #[arg(short, long, help = "Print verbose output")]
    verbose: bool,

    #[arg(short = 's', long, help = "Run specific scenario by name")]
    scenario: Option<String>,

    #[arg(
        long,
        help = "Set variable in format NAME=VALUE",
        value_name = "VAR=VAL"
    )]
    var: Vec<String>,
}

fn main() {
    let cli = Cli::parse();

    if !cli.project_file.exists() {
        eprintln!("Error: File not found: {}", cli.project_file.display());
        std::process::exit(1);
    }

    let (project, max_iterations) = match load_project(&cli.project_file) {
        Ok((p, m)) => (p, m),
        Err(e) => {
            eprintln!("Error loading project: {}", e);
            std::process::exit(1);
        }
    };

    let initial_vars: IndexMap<String, VariableValue> = project
        .variables
        .iter()
        .filter_map(|(name, value)| {
            if !matches!(value, VariableValue::Undefined) {
                Some((name.clone(), value.clone()))
            } else {
                None
            }
        })
        .collect();

    let _cli_vars = parse_variables(&cli.var);

    println!("                                                  ");
    println!("                  ##################              ");
    println!("                  ###################             ");
    println!("           ######                ####             ");
    println!("         ########                ####             ");
    println!("         ########                ####             ");
    println!("             ####     ######     ####             ");
    println!("             ####     ######     ####             ");
    println!("             ####     ######     ####             ");
    println!("             ####                ####             ");
    println!("             ####                ####             ");
    println!("             ####                ####             ");
    println!("             ###################     ####         ");
    println!("              ##################     ####         ");
    println!("                                                  ");
    println!("QazCode RPA CLI");
    println!("==================");
    println!("Project: {}", project.name);
    println!();

    let mut log_entries = Vec::new();

    let (log_sender, log_receiver) = std::sync::mpsc::channel();
    let (var_sender, var_receiver) = std::sync::mpsc::channel();

    let project_clone = project.clone();
    let verbose = cli.verbose;
    let scenario_name = cli.scenario.clone();
    let stop_flag = Arc::new(AtomicBool::new(false));
    let stop_flag_clone = Arc::clone(&stop_flag);

    std::thread::spawn(move || {
        if let Some(scenario_name) = scenario_name {
            if let Some(scenario) = project_clone
                .scenarios
                .iter()
                .find(|s| s.name == scenario_name)
            {
                println!("Executing scenario: {}", scenario_name);
                println!();
                execute_scenario_with_vars(
                    scenario,
                    &project_clone,
                    log_sender,
                    var_sender,
                    initial_vars,
                    max_iterations,
                    stop_flag_clone,
                );
            } else {
                let _ = log_sender.send(LogEntry {
                    timestamp: "[00:00.00]".to_string(),
                    level: LogLevel::Error,
                    activity: "CLI".to_string(),
                    message: format!("Scenario '{}' not found", scenario_name),
                });
            }
        } else {
            execute_project_with_typed_vars(
                &project_clone,
                log_sender,
                var_sender,
                initial_vars,
                max_iterations,
                stop_flag_clone,
            );
        }
    });

    let mut runtime_vars = project.variables.clone();

    loop {
        match log_receiver.try_recv() {
            Ok(entry) => {
                if entry.message == UiConstants::EXECUTION_COMPLETE_MARKER {
                    break;
                }

                if verbose || entry.level != LogLevel::Info {
                    println!(
                        "{} [{}] {}: {}",
                        entry.timestamp,
                        entry.level.as_str(),
                        entry.activity,
                        entry.message
                    );
                }

                log_entries.push(entry);
            }
            Err(_) => {
                while let Ok(event) = var_receiver.try_recv() {
                    match event {
                        VarEvent::Set { name, value } => {
                            let id = runtime_vars.id(&name);
                            runtime_vars.set(id, value);
                        }
                        VarEvent::Remove { name } => {
                            let id = runtime_vars.id(&name);
                            runtime_vars.remove(id);
                        }
                    }
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        }
    }

    while let Ok(event) = var_receiver.try_recv() {
        match event {
            VarEvent::Set { name, value } => {
                let id = runtime_vars.id(&name);
                runtime_vars.set(id, value);
            }
            VarEvent::Remove { name } => {
                let id = runtime_vars.id(&name);
                runtime_vars.remove(id);
            }
        }
    }

    if verbose {
        let var_list: Vec<_> = runtime_vars
            .iter()
            .filter_map(|(name, value)| {
                if !matches!(value, VariableValue::Undefined) {
                    Some((name, value))
                } else {
                    None
                }
            })
            .collect();

        if !var_list.is_empty() {
            println!();
            println!("Final Variables:");
            println!("================");

            let max_name_len = var_list.iter().map(|(n, _)| n.len()).max().unwrap_or(10);

            for (name, value) in var_list {
                println!(
                    "  {:width$}  [{:7}]  {}",
                    name,
                    value.get_type().as_str(),
                    value,
                    width = max_name_len
                );
            }
        }
    }

    println!();
    println!("Execution Summary:");
    println!("==================");

    let info_count = log_entries
        .iter()
        .filter(|e| matches!(e.level, LogLevel::Info))
        .count();
    let warning_count = log_entries
        .iter()
        .filter(|e| matches!(e.level, LogLevel::Warning))
        .count();
    let error_count = log_entries
        .iter()
        .filter(|e| matches!(e.level, LogLevel::Error))
        .count();

    println!("  Info:     {}", info_count);
    println!("  Warnings: {}", warning_count);
    println!("  Errors:   {}", error_count);

    if error_count > 0 {
        std::process::exit(1);
    }
}

fn load_project(path: &PathBuf) -> Result<(Project, usize), String> {
    if path.extension().and_then(|s| s.to_str()) != Some("rpa") {
        return Err(format!(
            "Invalid file extension: expected .rpa, got {:?}",
            path
        ));
    }

    let contents =
        std::fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;

    let (project, max_iterations) = serde_json::from_str::<ProjectFile>(&contents)
        .map(|pf| {
            (
                pf.project,
                UiState::normalize_max_iterations(pf.ui_state.max_iterations),
            )
        })
        .or_else(|_| {
            serde_json::from_str::<Project>(&contents).map(|p| {
                (
                    p,
                    UiState::normalize_max_iterations(UiConstants::LOOP_MAX_ITERATIONS),
                )
            })
        })
        .map_err(|e| format!("Failed to parse project: {}", e))?;

    Ok((project, max_iterations))
}

fn parse_variables(var_args: &[String]) -> indexmap::IndexMap<String, String> {
    let mut vars = indexmap::IndexMap::new();

    for arg in var_args {
        if let Some((name, value)) = arg.split_once('=') {
            vars.insert(name.to_string(), value.to_string());
        } else {
            eprintln!(
                "Warning: Invalid variable format '{}', expected NAME=VALUE",
                arg
            );
        }
    }

    vars
}
