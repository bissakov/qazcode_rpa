use crate::constants::UiConstants;
use crate::ir::{Instruction, IrBuilder, IrProgram};
use crate::node_graph::{LogEntry, LogLevel, Project, VariableValue};
use crate::utils;
use crate::validation::ScenarioValidator;
use crate::variables::VarEvent;
use crate::{evaluator, variables};
use indexmap::IndexMap;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::time::SystemTime;

pub struct ExecutionContext {
    start_time: SystemTime,
    variable_sender: Option<Sender<VarEvent>>,
    variables: variables::Variables,
    stop_flag: Arc<AtomicBool>,
}

pub struct IrExecutor<'a, L: LogOutput> {
    program: &'a IrProgram,
    project: &'a Project,
    context: &'a mut ExecutionContext,
    log: &'a mut L,
    error_handlers: Vec<usize>,
    iteration_counts: HashMap<usize, usize>,
    scenario_call_depth: usize,
}

pub trait LogOutput {
    fn log(&mut self, entry: LogEntry);
}

impl LogOutput for Vec<LogEntry> {
    fn log(&mut self, entry: LogEntry) {
        self.push(entry);
    }
}

impl LogOutput for Sender<LogEntry> {
    fn log(&mut self, entry: LogEntry) {
        let _ = self.send(entry);
    }
}

pub fn get_timestamp(start_time: SystemTime) -> String {
    let elapsed = start_time.elapsed().unwrap_or_default();
    format!(
        "[{:02}:{:02}.{:02}]",
        elapsed.as_secs() / 60,
        elapsed.as_secs() % 60,
        elapsed.subsec_millis()
    )
}

impl ExecutionContext {
    fn new(
        start_time: SystemTime,
        sender: Sender<VarEvent>,
        variables: variables::Variables,
        stop_flag: Arc<AtomicBool>,
    ) -> Self {
        Self {
            start_time,
            variable_sender: Some(sender),
            variables,
            stop_flag,
        }
    }

    fn new_with_sender(sender: Sender<VarEvent>, stop_flag: Arc<AtomicBool>) -> Self {
        Self {
            start_time: SystemTime::now(),
            variable_sender: Some(sender),
            variables: variables::Variables::new(),
            stop_flag,
        }
    }

    pub fn is_stopped(&self) -> bool {
        self.stop_flag.load(Ordering::Relaxed)
    }
}

impl<'a, L: LogOutput> IrExecutor<'a, L> {
    pub fn new(
        program: &'a IrProgram,
        project: &'a Project,
        context: &'a mut ExecutionContext,
        log: &'a mut L,
    ) -> Self {
        Self {
            program,
            project,
            context,
            log,
            error_handlers: Vec::new(),
            iteration_counts: HashMap::new(),
            scenario_call_depth: 0,
        }
    }

    pub fn execute(&mut self) -> Result<(), String> {
        let mut pc = self.program.entry_point;

        while pc < self.program.instructions.len() {
            if self.context.is_stopped() {
                return Err("Execution stopped by user".to_string());
            }

            match self.execute_instruction(pc) {
                Ok(next_pc) => {
                    if next_pc >= self.program.instructions.len() {
                        break;
                    }
                    pc = next_pc;
                }
                Err(e) => {
                    return self.handle_error(e, pc);
                }
            }
        }

        Ok(())
    }

    fn execute_instruction(&mut self, pc: usize) -> Result<usize, String> {
        let instruction = &self.program.instructions[pc];

        match instruction {
            Instruction::Start { scenario_id } => {
                let scenario = self
                    .project
                    .scenarios
                    .iter()
                    .find(|s| s.id == *scenario_id)
                    .or_else(|| {
                        if self.project.main_scenario.id == *scenario_id {
                            Some(&self.project.main_scenario)
                        } else {
                            None
                        }
                    });

                let timestamp = get_timestamp(self.context.start_time);
                if let Some(scenario) = scenario {
                    self.log.log(LogEntry {
                        timestamp,
                        level: LogLevel::Info,
                        activity: "START".to_string(),
                        message: format!("Starting scenario: {}", scenario.name),
                    });
                    Ok(pc + 1)
                } else {
                    let error_msg = format!("Scenario with ID {} not found", scenario_id);
                    self.log.log(LogEntry {
                        timestamp,
                        level: LogLevel::Error,
                        activity: "START".to_string(),
                        message: error_msg.clone(),
                    });
                    Err(error_msg)
                }
            }
            Instruction::End { scenario_id } => {
                let scenario = self
                    .project
                    .scenarios
                    .iter()
                    .find(|s| s.id == *scenario_id)
                    .or_else(|| {
                        if self.project.main_scenario.id == *scenario_id {
                            Some(&self.project.main_scenario)
                        } else {
                            None
                        }
                    });

                let timestamp = get_timestamp(self.context.start_time);
                if let Some(scenario) = scenario {
                    self.log.log(LogEntry {
                        timestamp,
                        level: LogLevel::Info,
                        activity: "END".to_string(),
                        message: format!("Ending scenario: {}", scenario.name),
                    });
                    Ok(pc + 1)
                } else {
                    let error_msg = format!("Scenario with ID {} not found", scenario_id);
                    self.log.log(LogEntry {
                        timestamp,
                        level: LogLevel::Error,
                        activity: "END".to_string(),
                        message: error_msg.clone(),
                    });
                    Err(error_msg)
                }
            }
            Instruction::Log { level, message } => {
                let timestamp = get_timestamp(self.context.start_time);
                self.log.log(LogEntry {
                    timestamp,
                    level: level.clone(),
                    activity: "LOG".to_string(),
                    message: message.clone(),
                });
                Ok(pc + 1)
            }
            Instruction::Delay { milliseconds } => {
                let timestamp = get_timestamp(self.context.start_time);
                self.log.log(LogEntry {
                    timestamp,
                    level: LogLevel::Info,
                    activity: "DELAY".to_string(),
                    message: format!("Waiting for {} ms", milliseconds),
                });

                match utils::interruptible_sleep(*milliseconds, &self.context.stop_flag.clone()) {
                    Ok(()) => Ok(pc + 1),
                    Err(()) => Err("Aborted".into()),
                }
            }
            Instruction::SetVar { var, value } => {
                let timestamp = get_timestamp(self.context.start_time);

                if let Some(sender) = &self.context.variable_sender {
                    let _ = sender.send(VarEvent::SetId {
                        id: *var,
                        value: value.clone(),
                    });
                }

                self.log.log(LogEntry {
                    timestamp,
                    level: LogLevel::Info,
                    activity: "SET VAR".to_string(),
                    message: format!("{:?} = {}", var, value),
                });
                Ok(pc + 1)
            }
            Instruction::Evaluate { expr } => {
                let timestamp = get_timestamp(self.context.start_time);

                let result = match evaluator::eval_expr(expr, self.context.variables.values()) {
                    Ok(value) => value,
                    Err(err) => {
                        self.log.log(LogEntry {
                            timestamp,
                            level: LogLevel::Error,
                            activity: "EVALUATE".to_string(),
                            message: err.to_string(),
                        });
                        return Err(err);
                    }
                };
                println!("Evaluated result: {}", result);

                self.log.log(LogEntry {
                    timestamp,
                    level: LogLevel::Info,
                    activity: "EVALUATE".to_string(),
                    message: format!("Expression evaluated to {}", result),
                });

                Ok(pc + 1)
            }
            Instruction::Jump { target } => Ok(*target),
            Instruction::JumpIf { condition, target } => {
                let timestamp = get_timestamp(self.context.start_time);

                let (message, next_pc, level) =
                    match evaluator::eval_expr(condition, self.context.variables.values()) {
                        Ok(VariableValue::Boolean(true)) => (
                            "Condition evaluated to: true".to_string(),
                            *target,
                            LogLevel::Info,
                        ),
                        Ok(VariableValue::Boolean(false)) => (
                            "Condition evaluated to: false".to_string(),
                            pc + 1,
                            LogLevel::Info,
                        ),
                        Ok(other) => (
                            format!("Condition evaluated to non-boolean value: {:?}", other),
                            pc + 1,
                            LogLevel::Error,
                        ),
                        Err(err) => (
                            format!("Condition failed with error: {}", err),
                            pc + 1,
                            LogLevel::Error,
                        ),
                    };

                self.log.log(LogEntry {
                    timestamp,
                    level,
                    activity: "IF".to_string(),
                    message,
                });

                Ok(next_pc)
            }
            Instruction::JumpIfNot { condition, target } => {
                let timestamp = get_timestamp(self.context.start_time);

                let (message, next_pc, level) =
                    match evaluator::eval_expr(condition, self.context.variables.values()) {
                        Ok(VariableValue::Boolean(true)) => (
                            "Condition evaluated to: true".to_string(),
                            pc + 1,
                            LogLevel::Info,
                        ),
                        Ok(VariableValue::Boolean(false)) => (
                            "Condition evaluated to: false".to_string(),
                            *target,
                            LogLevel::Info,
                        ),
                        Ok(other) => (
                            format!("Condition evaluated to non-boolean value: {:?}", other),
                            pc + 1,
                            LogLevel::Error,
                        ),
                        Err(err) => (
                            format!("Condition failed with error: {}", err),
                            *target,
                            LogLevel::Error,
                        ),
                    };

                self.log.log(LogEntry {
                    timestamp,
                    level,
                    activity: "IF".to_string(),
                    message,
                });

                Ok(next_pc)
            }
            Instruction::LoopInit { index, start } => {
                if let Some(sender) = &self.context.variable_sender {
                    let _ = sender.send(VarEvent::SetId {
                        id: *index,
                        value: VariableValue::Number(*start as f64),
                    });
                }

                Ok(pc + 1)
            }
            Instruction::LoopLog {
                index: _,
                start,
                end,
                step,
            } => {
                self.log.log(LogEntry {
                    timestamp: get_timestamp(self.context.start_time),
                    level: LogLevel::Info,
                    activity: "LOOP".to_string(),
                    message: format!("Starting loop: from {} to {} step {}", start, end, step),
                });

                Ok(pc + 1)
            }
            Instruction::LoopCheck {
                index,
                end,
                step,
                body_target,
                end_target,
            } => {
                if *step == 0 {
                    self.log.log(LogEntry {
                        timestamp: get_timestamp(self.context.start_time),
                        level: LogLevel::Warning,
                        activity: "LOOP".to_string(),
                        message: "Step is 0, loop skipped".to_string(),
                    });
                    return Ok(*end_target);
                }

                let current = self
                    .context
                    .variables
                    .get(*index)
                    .as_number()
                    .map(|n| n as i64)
                    .unwrap_or(*end);

                let should_continue = if *step > 0 {
                    current < *end
                } else {
                    current > *end
                };

                if should_continue {
                    Ok(*body_target)
                } else {
                    Ok(*end_target)
                }
            }
            Instruction::LoopNext {
                index,
                step,
                check_target,
            } => {
                let current = self.context.variables.get(*index).as_number().unwrap() as i64;

                let next = current + step;

                self.context
                    .variables
                    .set(*index, VariableValue::Number(next as f64));

                if let Some(sender) = &self.context.variable_sender {
                    let _ = sender.send(VarEvent::SetId {
                        id: *index,
                        value: VariableValue::Number(next as f64),
                    });
                }

                Ok(*check_target)
            }
            Instruction::WhileCheck {
                condition,
                body_target,
                end_target,
            } => {
                let timestamp = get_timestamp(self.context.start_time);
                match evaluator::eval_expr(condition, self.context.variables.values()) {
                    Ok(VariableValue::Boolean(true)) => {
                        let iter_count = self.iteration_counts.entry(pc).or_insert(0);
                        *iter_count += 1;

                        self.log.log(LogEntry {
                            timestamp,
                            level: LogLevel::Info,
                            activity: "WHILE".to_string(),
                            message: format!("Iteration {}: condition is true", *iter_count),
                        });
                        Ok(*body_target)
                    }
                    Ok(VariableValue::Boolean(false)) => {
                        let iter_count = self.iteration_counts.get(&pc).copied().unwrap_or(0);
                        self.log.log(LogEntry {
                            timestamp,
                            level: LogLevel::Info,
                            activity: "WHILE".to_string(),
                            message: format!("Completed {} iterations", iter_count),
                        });
                        self.iteration_counts.remove(&pc);
                        Ok(*end_target)
                    }
                    Err(e) => Err(e),
                    _ => Err("Non-logical result of an expression".to_string()),
                }
            }
            Instruction::PushErrorHandler { catch_target } => {
                self.error_handlers.push(*catch_target);
                let timestamp = get_timestamp(self.context.start_time);
                self.log.log(LogEntry {
                    timestamp,
                    level: LogLevel::Info,
                    activity: "TRY-CATCH".to_string(),
                    message: "Entering try block".to_string(),
                });
                Ok(pc + 1)
            }
            Instruction::PopErrorHandler => {
                self.error_handlers.pop();
                let timestamp = get_timestamp(self.context.start_time);
                self.log.log(LogEntry {
                    timestamp,
                    level: LogLevel::Info,
                    activity: "TRY-CATCH".to_string(),
                    message: "Try block completed successfully".to_string(),
                });
                Ok(pc + 1)
            }
            Instruction::CallScenario { scenario_id } => {
                self.scenario_call_depth += 1;

                if self.scenario_call_depth > 100 {
                    return Err("Maximum scenario call depth exceeded (100)".to_string());
                }

                let scenario = self
                    .project
                    .scenarios
                    .iter()
                    .find(|s| s.id == *scenario_id)
                    .or_else(|| {
                        if self.project.main_scenario.id == *scenario_id {
                            Some(&self.project.main_scenario)
                        } else {
                            None
                        }
                    });

                if let Some(_scenario) = scenario {
                    let timestamp = get_timestamp(self.context.start_time);
                    self.log.log(LogEntry {
                        timestamp,
                        level: LogLevel::Warning,
                        activity: "CALL".to_string(),
                        message: "[TODO] CallScenario not yet implemented in new IR system"
                            .to_string(),
                    });

                    self.scenario_call_depth -= 1;
                    Ok(pc + 1)
                } else {
                    self.scenario_call_depth -= 1;
                    Err(format!("Scenario with ID {} not found", scenario_id))
                }
            }
            Instruction::RunPowershell { code: _ } => {
                let timestamp = get_timestamp(self.context.start_time);
                self.log.log(LogEntry {
                    timestamp,
                    level: LogLevel::Warning,
                    activity: "RUN PWSH".to_string(),
                    message: "[TODO] NOT IMPLEMENTED YET".to_string(),
                });
                Ok(pc + 1)
            }
            Instruction::DebugMarker { .. } => Ok(pc + 1),
        }
    }

    fn handle_error(&mut self, error: String, _pc: usize) -> Result<(), String> {
        let id = self.context.variables.id("last_error");
        self.context
            .variables
            .set(id, VariableValue::String(error.clone()));

        if let Some(sender) = &self.context.variable_sender {
            let _ = sender.send(VarEvent::SetId {
                id,
                value: VariableValue::String(error.clone()),
            });
        }

        if let Some(catch_target) = self.error_handlers.pop() {
            let timestamp = get_timestamp(self.context.start_time);
            self.log.log(LogEntry {
                timestamp,
                level: LogLevel::Warning,
                activity: "TRY-CATCH".to_string(),
                message: format!("Error caught: {}", error),
            });

            let mut pc = catch_target;
            while pc < self.program.instructions.len() {
                match self.execute_instruction(pc) {
                    Ok(next_pc) => {
                        if next_pc >= self.program.instructions.len() {
                            break;
                        }
                        pc = next_pc;
                    }
                    Err(e) => return Err(e),
                }
            }

            Ok(())
        } else {
            let timestamp = get_timestamp(self.context.start_time);
            self.log.log(LogEntry {
                timestamp,
                level: LogLevel::Error,
                activity: "ERROR".to_string(),
                message: format!("Unhandled error: {}. No error handler connected.", error),
            });
            Err(error)
        }
    }
}

pub fn execute_project_with_vars(
    project: &Project,
    log_sender: Sender<LogEntry>,
    var_sender: Sender<VarEvent>,
    initial_vars: IndexMap<String, VariableValue>,
    stop_flag: Arc<AtomicBool>,
) {
    let mut context = ExecutionContext::new_with_sender(var_sender, stop_flag);

    for (name, value) in initial_vars {
        let id = context.variables.id(&name);
        context.variables.set(id, value);
    }

    let mut log = log_sender.clone();

    let validator = ScenarioValidator::new(&project.main_scenario, project);
    let validation_result = validator.validate();

    let timestamp = get_timestamp(context.start_time);
    validation_result.log_to_output(&mut log, &timestamp);

    if !validation_result.is_valid() {
        let _ = log_sender.send(LogEntry {
            timestamp: get_timestamp(context.start_time),
            level: LogLevel::Error,
            activity: "SYSTEM".to_string(),
            message: format!(
                "Execution aborted: {} validation errors",
                validation_result.errors.len()
            ),
        });
        let _ = log_sender.send(LogEntry {
            timestamp: "[00:00.000]".to_string(),
            level: LogLevel::Info,
            activity: "SYSTEM".to_string(),
            message: UiConstants::EXECUTION_COMPLETE_MARKER.to_string(),
        });
        return;
    }

    let ir_builder = IrBuilder::new(
        &project.main_scenario,
        project,
        &validation_result.reachable_nodes,
        &mut context.variables,
    );
    let program = match ir_builder.build() {
        Ok(prog) => prog,
        Err(e) => {
            let _ = log_sender.send(LogEntry {
                timestamp: get_timestamp(context.start_time),
                level: LogLevel::Error,
                activity: "SYSTEM".to_string(),
                message: format!("IR compilation failed: {}", e),
            });
            let _ = log_sender.send(LogEntry {
                timestamp: "[00:00.000]".to_string(),
                level: LogLevel::Info,
                activity: "SYSTEM".to_string(),
                message: UiConstants::EXECUTION_COMPLETE_MARKER.to_string(),
            });
            return;
        }
    };

    let mut executor = IrExecutor::new(&program, project, &mut context, &mut log);
    if let Err(e) = executor.execute() {
        let _ = log_sender.send(LogEntry {
            timestamp: get_timestamp(context.start_time),
            level: LogLevel::Error,
            activity: "SYSTEM".to_string(),
            message: format!("Execution error: {}", e),
        });
    }

    let _ = log_sender.send(LogEntry {
        timestamp: get_timestamp(context.start_time),
        level: LogLevel::Info,
        activity: "SYSTEM".to_string(),
        message: "Execution completed.".to_string(),
    });
    let _ = log_sender.send(LogEntry {
        timestamp: get_timestamp(context.start_time),
        level: LogLevel::Info,
        activity: "SYSTEM".to_string(),
        message: UiConstants::EXECUTION_COMPLETE_MARKER.to_string(),
    });
}

pub fn execute_project_with_typed_vars(
    project: &Project,
    log_sender: Sender<LogEntry>,
    var_sender: Sender<VarEvent>,
    start_time: SystemTime,
    program: IrProgram,
    variables: variables::Variables,
    stop_flag: Arc<AtomicBool>,
) {
    let mut context = ExecutionContext::new(start_time, var_sender, variables, stop_flag);

    // // for (name, value) in initial_vars {
    // //     let id = context.variables.id(&name);
    // //     context.variables.set(id, value.clone());
    // //     // if let Some(sender) = &context.variable_sender {
    // //     //     let _ = sender.send(VarEvent::Set {
    // //     //         name: name.clone(),
    // //     //         value,
    // //     //     });
    // //     // }
    // // }
    //
    let mut log = log_sender.clone();
    //
    // let validator = ScenarioValidator::new(&project.main_scenario, project);
    // let validation_result = validator.validate();
    //
    // let timestamp = get_timestamp(context.start_time);
    // validation_result.log_to_output(&mut log, &timestamp);
    //
    // if !validation_result.is_valid() {
    //     let _ = log_sender.send(LogEntry {
    //         timestamp: get_timestamp(context.start_time),
    //         level: LogLevel::Error,
    //         activity: "SYSTEM".to_string(),
    //         message: format!(
    //             "Execution aborted: {} validation errors",
    //             validation_result.errors.len()
    //         ),
    //     });
    //     let _ = log_sender.send(LogEntry {
    //         timestamp: "[00:00.000]".to_string(),
    //         level: LogLevel::Info,
    //         activity: "SYSTEM".to_string(),
    //         message: UiConstants::EXECUTION_COMPLETE_MARKER.to_string(),
    //     });
    //     return;
    // }
    //
    // let ir_builder = IrBuilder::new(
    //     &project.main_scenario,
    //     project,
    //     &validation_result.reachable_nodes,
    //     &mut context.variables,
    // );
    // let program = match ir_builder.build() {
    //     Ok(prog) => prog,
    //     Err(e) => {
    //         let _ = log_sender.send(LogEntry {
    //             timestamp: get_timestamp(context.start_time),
    //             level: LogLevel::Error,
    //             activity: "SYSTEM".to_string(),
    //             message: format!("IR compilation failed: {}", e),
    //         });
    //         let _ = log_sender.send(LogEntry {
    //             timestamp: "[00:00.000]".to_string(),
    //             level: LogLevel::Info,
    //             activity: "SYSTEM".to_string(),
    //             message: UiConstants::EXECUTION_COMPLETE_MARKER.to_string(),
    //         });
    //         return;
    //     }
    // };

    let mut executor = IrExecutor::new(&program, project, &mut context, &mut log);
    if let Err(e) = executor.execute() {
        let _ = log_sender.send(LogEntry {
            timestamp: get_timestamp(context.start_time),
            level: LogLevel::Error,
            activity: "SYSTEM".to_string(),
            message: format!("Execution error: {}", e),
        });
    }

    let _ = log_sender.send(LogEntry {
        timestamp: get_timestamp(context.start_time),
        level: LogLevel::Info,
        activity: "SYSTEM".to_string(),
        message: "Execution completed.".to_string(),
    });
    let _ = log_sender.send(LogEntry {
        timestamp: get_timestamp(context.start_time),
        level: LogLevel::Info,
        activity: "SYSTEM".to_string(),
        message: UiConstants::EXECUTION_COMPLETE_MARKER.to_string(),
    });
}

pub fn execute_scenario_with_vars(
    project: &Project,
    log_sender: Sender<LogEntry>,
    var_sender: Sender<VarEvent>,
    start_time: SystemTime,
    program: IrProgram,
    variables: variables::Variables,
    stop_flag: Arc<AtomicBool>,
) {
    let mut context = ExecutionContext::new(start_time, var_sender, variables, stop_flag);

    let mut log = log_sender.clone();

    let mut executor = IrExecutor::new(&program, project, &mut context, &mut log);
    if let Err(e) = executor.execute() {
        let _ = log_sender.send(LogEntry {
            timestamp: get_timestamp(context.start_time),
            level: LogLevel::Error,
            activity: "SYSTEM".to_string(),
            message: format!("Execution error: {}", e),
        });
    }

    let _ = log_sender.send(LogEntry {
        timestamp: get_timestamp(context.start_time),
        level: LogLevel::Info,
        activity: "SYSTEM".to_string(),
        message: "Execution completed.".to_string(),
    });
    let _ = log_sender.send(LogEntry {
        timestamp: get_timestamp(context.start_time),
        level: LogLevel::Info,
        activity: "SYSTEM".to_string(),
        message: UiConstants::EXECUTION_COMPLETE_MARKER.to_string(),
    });
}
