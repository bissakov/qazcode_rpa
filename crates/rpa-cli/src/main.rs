use clap::Parser;
use rpa_core::variables::VarEvent;
use rpa_core::{
    IrBuilder, LogEntry, LogLevel, Project, ProjectFile, ScenarioValidator, UiConstants,
    VariableValue, execute_project_with_typed_vars,
};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::SystemTime;

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

    let project = match load_project(&cli.project_file) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Error loading project: {}", e);
            std::process::exit(1);
        }
    };

    let mut variables = project.variables.clone();

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

    let verbose = cli.verbose;
    let scenario_name = cli.scenario.clone();
    let stop_flag = Arc::new(AtomicBool::new(false));
    let stop_flag_clone = Arc::clone(&stop_flag);

    let start_time = SystemTime::now();
    let validator = ScenarioValidator::new(&project.main_scenario, &project);
    let validation_result = validator.validate();

    if !validation_result.is_valid() {
        eprintln!("Execution aborted: {} validation errors", validation_result.errors.len());
        for error in &validation_result.errors {
            eprintln!("  ERROR: {:?}", error);
        }
        std::process::exit(1);
    }

    let ir_builder = IrBuilder::new(
        &project.main_scenario,
        &project,
        &validation_result.reachable_nodes,
        &mut variables,
    );
    let program = match ir_builder.build() {
        Ok(prog) => prog,
        Err(e) => {
            eprintln!("IR compilation failed: {}", e);
            std::process::exit(1);
        }
    };

    let project_clone = project.clone();
    let variables_clone = variables.clone();

    std::thread::spawn(move || {
        if let Some(_scenario_name) = scenario_name {
            eprintln!("Error: Specific scenario execution not supported in new IR-based architecture");
            let _ = log_sender.send(LogEntry {
                timestamp: "[00:00.000]".to_string(),
                level: LogLevel::Error,
                activity: "CLI".to_string(),
                message: "Scenario-specific execution not supported".to_string(),
            });
            let _ = log_sender.send(LogEntry {
                timestamp: "[00:00.000]".to_string(),
                level: LogLevel::Info,
                activity: "SYSTEM".to_string(),
                message: UiConstants::EXECUTION_COMPLETE_MARKER.to_string(),
            });
        } else {
            execute_project_with_typed_vars(
                &project_clone,
                log_sender,
                var_sender,
                start_time,
                program,
                variables_clone,
                stop_flag_clone,
            );
        }
    });

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
                            let id = variables.id(&name);
                            variables.set(id, value);
                        }
                        VarEvent::Remove { name } => {
                            let id = variables.id(&name);
                            variables.remove(id);
                        }
                        VarEvent::SetId { id, value } => {
                            variables.set(id, value);
                        }
                        VarEvent::RemoveId { id } => {
                            variables.remove(id);
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
                let id = variables.id(&name);
                variables.set(id, value);
            }
            VarEvent::Remove { name } => {
                let id = variables.id(&name);
                variables.remove(id);
            }
            VarEvent::SetId { id, value } => {
                variables.set(id, value);
            }
            VarEvent::RemoveId { id } => {
                variables.remove(id);
            }
        }
    }

    if verbose {
        let var_list: Vec<(String, VariableValue)> = variables
            .iter()
            .filter_map(|(name, value)| {
                if !matches!(value, VariableValue::Undefined) {
                    Some((name.clone(), value.clone()))
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

fn load_project(path: &PathBuf) -> Result<Project, String> {
    if path.extension().and_then(|s| s.to_str()) != Some("rpa") {
        return Err(format!(
            "Invalid file extension: expected .rpa, got {:?}",
            path
        ));
    }

    let contents =
        std::fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;

    let project = serde_json::from_str::<ProjectFile>(&contents)
        .map(|pf| pf.project)
        .or_else(|_| serde_json::from_str::<Project>(&contents))
        .map_err(|e| format!("Failed to parse project: {}", e))?;

    Ok(project)
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
