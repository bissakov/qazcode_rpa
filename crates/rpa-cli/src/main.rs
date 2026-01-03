use arc_script::Value;
use clap::Parser;
use rpa_core::execution::{ExecutionContext, IrExecutor, LogOutput, ScopeFrame};
use rpa_core::log::{LogEntry, LogLevel};
use rpa_core::{IrBuilder, Project, ProjectFile, ScenarioValidator, StopControl};
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
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

struct CliLogOutput {
    verbose: bool,
    entries: Vec<LogEntry>,
}

impl LogOutput for CliLogOutput {
    fn log(&mut self, entry: LogEntry) {
        if self.verbose || entry.level != LogLevel::Info {
            println!(
                "{} [{}] {}: {}",
                entry.timestamp,
                entry.level.as_str(),
                entry.activity.as_str(),
                entry.message
            );
        }
        self.entries.push(entry);
    }
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
    println!("                  ####################            ");
    println!("           ######                 ####            ");
    println!("         ########                 ####            ");
    println!("         ########                 ####            ");
    println!("             ####     ######      ####            ");
    println!("             ####     ######      ####            ");
    println!("             ####     ######      ####            ");
    println!("             ####                 ####            ");
    println!("             #### QazCode RPA CLI ####            ");
    println!("             ####                 ####            ");
    println!("             ###################     ####         ");
    println!("              ##################     ####         ");
    println!("                                                  ");
    println!("Project: {}", project.name);
    println!();

    let verbose = cli.verbose;
    let stop_control = StopControl::new();
    let start_time = SystemTime::now();
    let validator = ScenarioValidator::new(&project.main_scenario, &project);
    let validation_result = validator.validate();

    if !validation_result.is_valid() {
        eprintln!(
            "Execution aborted: {} validation errors",
            validation_result.errors.len()
        );
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

    if let Some(_scenario_name) = &cli.scenario {
        eprintln!("Error: Specific scenario execution not supported in new IR-based architecture");
        std::process::exit(1);
    }

    let scope_stack = vec![ScopeFrame {
        scenario_id: project.main_scenario.id.clone(),
        variables: project.main_scenario.variables.clone(),
    }];

    let context = Arc::new(RwLock::new(ExecutionContext::new_without_sender(
        start_time,
        scope_stack,
        variables,
        stop_control,
    )));

    let mut log_output = CliLogOutput {
        verbose,
        entries: Vec::new(),
    };

    let mut executor = IrExecutor::new(&program, &project, context.clone(), &mut log_output);
    if let Err(e) = executor.execute() {
        eprintln!("Execution error: {}", e);
        std::process::exit(1);
    }

    if verbose {
        let ctx = context.read().unwrap();
        let var_list: Vec<(String, Value)> = ctx
            .global_variables
            .iter()
            .filter_map(|(name, value, _)| {
                if !matches!(value, Value::Undefined) {
                    Some((name.to_string(), value.clone()))
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

    let info_count = log_output
        .entries
        .iter()
        .filter(|e| matches!(e.level, LogLevel::Info))
        .count();
    let warning_count = log_output
        .entries
        .iter()
        .filter(|e| matches!(e.level, LogLevel::Warning))
        .count();
    let error_count = log_output
        .entries
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
