use crate::constants::UiConstants;
use crate::evaluator;
use crate::ir::{Instruction, IrBuilder, IrProgram};
use crate::node_graph::{LogEntry, LogLevel, Project, Scenario, VariableValue};
use crate::utils;
use crate::validation::ScenarioValidator;
use crate::variables::{VarEvent, Variables};
use indexmap::IndexMap;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::time::SystemTime;

pub struct ExecutionContext {
    pub variables: Variables,
    start_time: SystemTime,
    variable_sender: Option<Sender<VarEvent>>,
    max_iterations: usize,
    stop_flag: Arc<AtomicBool>,
}

pub struct IrExecutor<'a, L: LogOutput> {
    program: &'a IrProgram,
    project: &'a Project,
    context: &'a mut ExecutionContext,
    log: &'a mut L,
    error_handlers: Vec<usize>,
    iteration_counts: HashMap<String, usize>,
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

pub fn get_timestamp(context: &ExecutionContext) -> String {
    let elapsed = context.start_time.elapsed().unwrap_or_default();
    format!(
        "[{:02}:{:02}.{:02}]",
        elapsed.as_secs() / 60,
        elapsed.as_secs() % 60,
        elapsed.subsec_millis()
    )
}

impl ExecutionContext {
    fn new_with_sender(
        sender: Sender<VarEvent>,
        max_iterations: usize,
        stop_flag: Arc<AtomicBool>,
    ) -> Self {
        Self {
            variables: Variables::new(),
            start_time: SystemTime::now(),
            variable_sender: Some(sender),
            max_iterations,
            stop_flag,
        }
    }

    fn max_iterations_enabled(&self) -> bool {
        self.max_iterations != 0
    }

    pub fn is_stopped(&self) -> bool {
        self.stop_flag.load(Ordering::Relaxed)
    }

    // fn set_variable(&mut self, name: String, value: VariableValue) {
    //     self.variables.insert(name, value);
    //     if let Some(sender) = &self.variable_sender {
    //         let _ = sender.send(self.variables.clone());
    //     }
    // }
    //
    // fn get_variable(&self, name: &str) -> Option<&VariableValue> {
    //     self.variables.get(name)
    // }

    // fn compare(&self, op: &str, l: VariableValue, r: VariableValue) -> bool {
    //     match (l, r) {
    //         (VariableValue::Number(a), VariableValue::Number(b)) => match op {
    //             "==" => a == b,
    //             "!=" => a != b,
    //             ">" => a > b,
    //             "<" => a < b,
    //             ">=" => a >= b,
    //             "<=" => a <= b,
    //             _ => false,
    //         },
    //
    //         (VariableValue::Boolean(a), VariableValue::Boolean(b)) => match op {
    //             "==" => a == b,
    //             "!=" => a != b,
    //             _ => false,
    //         },
    //
    //         (VariableValue::String(a), VariableValue::String(b)) => match op {
    //             "==" => a == b,
    //             "!=" => a != b,
    //             _ => false,
    //         },
    //
    //         _ => false,
    //     }
    // }
    //
    // fn eval(&self, expr: &str) -> Option<VariableValue> {
    //     let expr = expr.trim();
    //
    //     if (expr.starts_with('"') && expr.ends_with('"'))
    //         || (expr.starts_with('\'') && expr.ends_with('\''))
    //     {
    //         return Some(VariableValue::String(expr[1..expr.len() - 1].to_string()));
    //     }
    //
    //     if let Ok(n) = expr.parse::<f64>() {
    //         return Some(VariableValue::Number(n));
    //     }
    //
    //     if expr == "true" {
    //         return Some(VariableValue::Boolean(true));
    //     }
    //     if expr == "false" {
    //         return Some(VariableValue::Boolean(false));
    //     }
    //
    //     self.get_variable(expr).cloned()
    // }
    //
    // pub fn evaluate_condition(&self, expression: &str) -> bool {
    //     let resolved = self.resolve_value(&expression.to_string());
    //     let condition = resolved.trim();
    //
    //     println!("execution.rs:120 - {}", condition);
    //
    //     for op in ["==", "!=", ">=", "<=", ">", "<"] {
    //         if let Some((l, r)) = condition.split_once(op) {
    //             let left = self.eval(l.trim());
    //             let right = self.eval(r.trim());
    //
    //             if let (Some(lv), Some(rv)) = (left, right) {
    //                 return self.compare(op, lv, rv);
    //             }
    //
    //             return false;
    //         }
    //     }
    //
    //     match self.eval(condition) {
    //         Some(VariableValue::Boolean(b)) => b,
    //         Some(VariableValue::Number(n)) => n != 0.0,
    //         Some(VariableValue::String(s)) => !s.is_empty(),
    //         _ => false,
    //     }
    // }

    fn resolve_value(&mut self, value: &String) -> String {
        if (value.starts_with('"') && value.ends_with('"'))
            || (value.starts_with('\'') && value.ends_with('\''))
        {
            return value[1..value.len() - 1].to_string();
        }

        let mut result = value.to_string();
        let mut start_idx = 0;

        while let Some(open_pos) = result[start_idx..].find(UiConstants::VARIABLE_PLACEHOLDER_OPEN)
        {
            let actual_open = start_idx + open_pos;
            if let Some(close_pos) =
                result[actual_open..].find(UiConstants::VARIABLE_PLACEHOLDER_CLOSE)
            {
                let actual_close = actual_open + close_pos;
                let var_name = &result[actual_open + 1..actual_close];

                let id = self.variables.id(var_name);
                let var_value = self.variables.get(id);
                if !matches!(var_value, VariableValue::Undefined) {
                    let var_string = var_value.to_string();
                    result.replace_range(actual_open..=actual_close, &var_string);
                    start_idx = actual_open + var_string.len();
                } else {
                    start_idx = actual_close + 1;
                }
            } else {
                break;
            }
        }

        // let id = self.registry.get_or_create(var_name);
        // if !value.contains(UiConstants::VARIABLE_PLACEHOLDER_OPEN)
        //     && !value.contains(UiConstants::VARIABLE_PLACEHOLDER_CLOSE)
        //     && let Some(var_value) = self.get_variable(value)
        // {
        //     return var_value.to_string().clone();
        // }

        result
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

                let timestamp = get_timestamp(self.context);
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

                let timestamp = get_timestamp(self.context);
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
                let resolved_message = self.context.resolve_value(message);
                let timestamp = get_timestamp(self.context);
                self.log.log(LogEntry {
                    timestamp,
                    level: level.clone(),
                    activity: "LOG".to_string(),
                    message: resolved_message,
                });
                Ok(pc + 1)
            }
            Instruction::Delay { milliseconds } => {
                let timestamp = get_timestamp(self.context);
                self.log.log(LogEntry {
                    timestamp,
                    level: LogLevel::Info,
                    activity: "DELAY".to_string(),
                    message: format!("Waiting for {} ms", milliseconds),
                });

                match utils::interruptible_sleep(5_000, &self.context.stop_flag.clone()) {
                    Ok(()) => Ok(pc + 1),
                    Err(()) => Err("Aborted".into()),
                }
            }
            Instruction::SetVar { var, value } => {
                let timestamp = get_timestamp(self.context);

                self.context.variables.set(*var, value.clone());
                let name = self.context.variables.name(*var);

                self.log.log(LogEntry {
                    timestamp,
                    level: LogLevel::Info,
                    activity: "SET VAR".to_string(),
                    message: format!("{} = {}", name, value),
                });
                Ok(pc + 1)
            }
            Instruction::GetVar { var } => {
                let timestamp = get_timestamp(self.context);

                let name = self.context.variables.name(*var);
                let value = self.context.variables.get(*var);
                if !matches!(value, VariableValue::Undefined) {
                    self.log.log(LogEntry {
                        timestamp,
                        level: LogLevel::Info,
                        activity: "GET VAR".to_string(),
                        message: format!("{} = {}", name, value),
                    });
                } else {
                    self.log.log(LogEntry {
                        timestamp,
                        level: LogLevel::Warning,
                        activity: "GET VAR".to_string(),
                        message: format!("Variable '{}' not found", name),
                    });
                }
                Ok(pc + 1)
            }
            Instruction::Evaluate { expression } => {
                let timestamp = get_timestamp(self.context);

                let result = match evaluator::evaluate(expression, &mut self.context.variables) {
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
                let timestamp = get_timestamp(self.context);

                let (message, next_pc, level) =
                    match evaluator::evaluate(condition, &mut self.context.variables) {
                        Ok(VariableValue::Boolean(true)) => (
                            format!("Condition '{}' evaluated to: true", condition),
                            *target,
                            LogLevel::Info,
                        ),
                        Ok(VariableValue::Boolean(false)) => (
                            format!("Condition '{}' evaluated to: false", condition),
                            pc + 1,
                            LogLevel::Info,
                        ),
                        Ok(other) => (
                            format!(
                                "Condition '{}' evaluated to non-boolean value: {:?}",
                                condition, other
                            ),
                            pc + 1,
                            LogLevel::Error,
                        ),
                        Err(err) => (
                            format!("Condition '{}' failed with error: {}", condition, err),
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
                let timestamp = get_timestamp(self.context);

                let (message, next_pc, level) =
                    match evaluator::evaluate(condition, &mut self.context.variables) {
                        Ok(VariableValue::Boolean(true)) => (
                            format!("Condition '{}' evaluated to: true", condition),
                            pc + 1,
                            LogLevel::Info,
                        ),
                        Ok(VariableValue::Boolean(false)) => (
                            format!("Condition '{}' evaluated to: false", condition),
                            *target,
                            LogLevel::Info,
                        ),
                        Ok(other) => (
                            format!(
                                "Condition '{}' evaluated to non-boolean value: {:?}",
                                condition, other
                            ),
                            pc + 1,
                            LogLevel::Error,
                        ),
                        Err(err) => (
                            format!("Condition '{}' failed with error: {}", condition, err),
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
            Instruction::LoopInit {
                start,
                end,
                step,
                index,
                body_target: _,
                end_target,
            } => {
                let timestamp = get_timestamp(self.context);
                let index_name = self.context.variables.name(*index).to_string();

                self.log.log(LogEntry {
                    timestamp,
                    level: LogLevel::Info,
                    activity: "LOOP".to_string(),
                    message: format!(
                        "Starting loop: {} from {} to {} step {}",
                        index_name, start, end, step
                    ),
                });

                if *step == 0 {
                    let timestamp = get_timestamp(self.context);
                    self.log.log(LogEntry {
                        timestamp,
                        level: LogLevel::Warning,
                        activity: "LOOP".to_string(),
                        message: "Step is 0, loop skipped".to_string(),
                    });
                    Ok(*end_target)
                } else {
                    self.context
                        .variables
                        .set(*index, VariableValue::Number(*start as f64));
                    self.iteration_counts.insert(index_name, 0);
                    Ok(pc + 1)
                }
            }
            Instruction::LoopCheck {
                index,
                end,
                step,
                body_target,
                end_target,
            } => {
                let index_name = self.context.variables.name(*index).to_string();
                let current_val = self
                    .context
                    .variables
                    .get(*index)
                    .as_number()
                    .map(|n| n as i64)
                    .unwrap_or(*end);

                let should_continue = if *step > 0 {
                    current_val < *end
                } else {
                    current_val > *end
                };

                if should_continue {
                    let iter_count = self.iteration_counts.get(&index_name).copied().unwrap_or(0);

                    if self.context.max_iterations_enabled()
                        && iter_count >= self.context.max_iterations
                    {
                        let timestamp = get_timestamp(self.context);
                        self.log.log(LogEntry {
                            timestamp,
                            level: LogLevel::Warning,
                            activity: "LOOP".to_string(),
                            message: format!(
                                "Max iterations limit ({}) reached, loop terminated",
                                self.context.max_iterations
                            ),
                        });

                        self.context.variables.remove(*index);
                        if let Some(sender) = &self.context.variable_sender {
                            let _ = sender.send(VarEvent::Remove { name: index_name.clone() });
                        }
                        self.iteration_counts.remove(&index_name);

                        Ok(*end_target)
                    } else {
                        let timestamp = get_timestamp(self.context);
                        self.log.log(LogEntry {
                            timestamp,
                            level: LogLevel::Info,
                            activity: "LOOP".to_string(),
                            message: format!(
                                "Iteration {}: {} = {}",
                                iter_count + 1,
                                index_name,
                                current_val
                            ),
                        });

                        Ok(*body_target)
                    }
                } else {
                    let iter_count = self.iteration_counts.get(&index_name).copied().unwrap_or(0);
                    let timestamp = get_timestamp(self.context);
                    self.log.log(LogEntry {
                        timestamp,
                        level: LogLevel::Info,
                        activity: "LOOP".to_string(),
                        message: format!("Completed {} iterations", iter_count),
                    });

                    self.context.variables.remove(*index);
                    if let Some(sender) = &self.context.variable_sender {
                        let _ = sender.send(VarEvent::Remove { name: index_name.clone() });
                    }
                    self.iteration_counts.remove(&index_name);

                    Ok(*end_target)
                }
            }
            Instruction::LoopIncrement {
                index,
                step,
                check_target,
            } => {
                let index_name = self.context.variables.name(*index).to_string();
                let current_val = self
                    .context
                    .variables
                    .get(*index)
                    .as_number()
                    .map(|n| n as i64)
                    .unwrap_or(0);

                let new_val = current_val + step;
                self.context
                    .variables
                    .set(*index, VariableValue::Number(new_val as f64));
                if let Some(sender) = &self.context.variable_sender {
                    let _ = sender.send(VarEvent::Set {
                        name: index_name.clone(),
                        value: VariableValue::Number(new_val as f64),
                    });
                }

                let iter_count = self.iteration_counts.entry(index_name).or_insert(0);
                *iter_count += 1;

                Ok(*check_target)
            }
            Instruction::WhileCheck {
                condition,
                body_target,
                end_target,
            } => {
                let timestamp = get_timestamp(self.context);
                match evaluator::evaluate(condition, &mut self.context.variables) {
                    Ok(VariableValue::Boolean(true)) => {
                        let iter_count =
                            self.iteration_counts.entry(condition.clone()).or_insert(0);
                        *iter_count += 1;

                        if self.context.max_iterations_enabled()
                            && *iter_count > self.context.max_iterations
                        {
                            self.log.log(LogEntry {
                                timestamp,
                                level: LogLevel::Warning,
                                activity: "WHILE".to_string(),
                                message: format!(
                                    "Max iterations limit ({}) reached, loop terminated",
                                    self.context.max_iterations
                                ),
                            });
                            self.iteration_counts.remove(condition);
                            Ok(*end_target)
                        } else {
                            self.log.log(LogEntry {
                                timestamp,
                                level: LogLevel::Info,
                                activity: "WHILE".to_string(),
                                message: format!("Iteration {}: condition is true", *iter_count),
                            });
                            Ok(*body_target)
                        }
                    }
                    Ok(VariableValue::Boolean(false)) => {
                        let iter_count = self.iteration_counts.get(condition).copied().unwrap_or(0);
                        self.log.log(LogEntry {
                            timestamp,
                            level: LogLevel::Info,
                            activity: "WHILE".to_string(),
                            message: format!("Completed {} iterations", iter_count),
                        });
                        self.iteration_counts.remove(condition);
                        Ok(*end_target)
                    }
                    Err(e) => Err(e),
                    _ => Err("Non-logical result of an expression".to_string()),
                }
            }
            Instruction::PushErrorHandler { catch_target } => {
                self.error_handlers.push(*catch_target);
                let timestamp = get_timestamp(self.context);
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
                let timestamp = get_timestamp(self.context);
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

                if let Some(scenario) = scenario {
                    let timestamp = get_timestamp(self.context);
                    self.log.log(LogEntry {
                        timestamp,
                        level: LogLevel::Info,
                        activity: "CALL".to_string(),
                        message: format!("Calling scenario: {}", scenario.name),
                    });

                    let current_vars: IndexMap<String, VariableValue> = self
                        .context
                        .variables
                        .iter()
                        .map(|(name, value)| (name.clone(), value.clone()))
                        .collect();

                    let (log_sender, log_receiver) = std::sync::mpsc::channel();
                    let (var_sender, _var_receiver) = std::sync::mpsc::channel();

                    execute_scenario_with_vars(
                        scenario,
                        self.project,
                        log_sender,
                        var_sender,
                        current_vars,
                        self.context.max_iterations,
                        self.context.stop_flag.clone(),
                    );

                    for log_entry in log_receiver.iter() {
                        if log_entry.message == UiConstants::EXECUTION_COMPLETE_MARKER {
                            break;
                        }
                        self.log.log(log_entry);
                    }

                    self.scenario_call_depth -= 1;
                    Ok(pc + 1)
                } else {
                    self.scenario_call_depth -= 1;
                    Err(format!("Scenario with ID {} not found", scenario_id))
                }
            }
            Instruction::RunPowershell { code: _ } => {
                let timestamp = get_timestamp(self.context);
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

        if let Some(catch_target) = self.error_handlers.pop() {
            let timestamp = get_timestamp(self.context);
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
            let timestamp = get_timestamp(self.context);
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
    max_iterations: usize,
    stop_flag: Arc<AtomicBool>,
) {
    let mut context = ExecutionContext::new_with_sender(var_sender, max_iterations, stop_flag);

    for (name, value) in initial_vars {
        let id = context.variables.id(&name);
        context.variables.set(id, value);
    }

    let mut log = log_sender.clone();

    let validator = ScenarioValidator::new(&project.main_scenario, project);
    let validation_result = validator.validate();

    let timestamp = get_timestamp(&context);
    validation_result.log_to_output(&mut log, &timestamp);

    if !validation_result.is_valid() {
        let _ = log_sender.send(LogEntry {
            timestamp: get_timestamp(&context),
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
                timestamp: get_timestamp(&context),
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
            timestamp: get_timestamp(&context),
            level: LogLevel::Error,
            activity: "SYSTEM".to_string(),
            message: format!("Execution error: {}", e),
        });
    }

    let _ = log_sender.send(LogEntry {
        timestamp: get_timestamp(&context),
        level: LogLevel::Info,
        activity: "SYSTEM".to_string(),
        message: "Execution completed.".to_string(),
    });
    let _ = log_sender.send(LogEntry {
        timestamp: get_timestamp(&context),
        level: LogLevel::Info,
        activity: "SYSTEM".to_string(),
        message: UiConstants::EXECUTION_COMPLETE_MARKER.to_string(),
    });
}

pub fn execute_project_with_typed_vars(
    project: &Project,
    log_sender: Sender<LogEntry>,
    var_sender: Sender<VarEvent>,
    initial_vars: indexmap::IndexMap<String, VariableValue>,
    max_iterations: usize,
    stop_flag: Arc<AtomicBool>,
) {
    let mut context = ExecutionContext::new_with_sender(var_sender, max_iterations, stop_flag);

    for (name, value) in initial_vars {
        let id = context.variables.id(&name);
        context.variables.set(id, value.clone());
        if let Some(sender) = &context.variable_sender {
            let _ = sender.send(VarEvent::Set { name: name.clone(), value });
        }
    }

    let mut log = log_sender.clone();

    let validator = ScenarioValidator::new(&project.main_scenario, project);
    let validation_result = validator.validate();

    let timestamp = get_timestamp(&context);
    validation_result.log_to_output(&mut log, &timestamp);

    if !validation_result.is_valid() {
        let _ = log_sender.send(LogEntry {
            timestamp: get_timestamp(&context),
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
                timestamp: get_timestamp(&context),
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
            timestamp: get_timestamp(&context),
            level: LogLevel::Error,
            activity: "SYSTEM".to_string(),
            message: format!("Execution error: {}", e),
        });
    }

    let _ = log_sender.send(LogEntry {
        timestamp: get_timestamp(&context),
        level: LogLevel::Info,
        activity: "SYSTEM".to_string(),
        message: "Execution completed.".to_string(),
    });
    let _ = log_sender.send(LogEntry {
        timestamp: get_timestamp(&context),
        level: LogLevel::Info,
        activity: "SYSTEM".to_string(),
        message: UiConstants::EXECUTION_COMPLETE_MARKER.to_string(),
    });
}

pub fn execute_scenario_with_vars(
    scenario: &Scenario,
    project: &Project,
    log_sender: Sender<LogEntry>,
    var_sender: Sender<VarEvent>,
    initial_vars: indexmap::IndexMap<String, VariableValue>,
    max_iterations: usize,
    stop_flag: Arc<AtomicBool>,
) {
    let mut context = ExecutionContext::new_with_sender(var_sender, max_iterations, stop_flag);

    for (name, value) in initial_vars {
        let id = context.variables.id(&name);
        context.variables.set(id, value.clone());
        if let Some(sender) = &context.variable_sender {
            let _ = sender.send(VarEvent::Set { name: name.clone(), value });
        }
    }

    let mut log = log_sender.clone();

    let validator = ScenarioValidator::new(scenario, project);
    let validation_result = validator.validate();

    let timestamp = get_timestamp(&context);
    validation_result.log_to_output(&mut log, &timestamp);

    if !validation_result.is_valid() {
        let _ = log_sender.send(LogEntry {
            timestamp: get_timestamp(&context),
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
        scenario,
        project,
        &validation_result.reachable_nodes,
        &mut context.variables,
    );
    let program = match ir_builder.build() {
        Ok(prog) => prog,
        Err(e) => {
            let _ = log_sender.send(LogEntry {
                timestamp: get_timestamp(&context),
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
            timestamp: get_timestamp(&context),
            level: LogLevel::Error,
            activity: "SYSTEM".to_string(),
            message: format!("Execution error: {}", e),
        });
    }

    let _ = log_sender.send(LogEntry {
        timestamp: get_timestamp(&context),
        level: LogLevel::Info,
        activity: "SYSTEM".to_string(),
        message: "Execution completed.".to_string(),
    });
    let _ = log_sender.send(LogEntry {
        timestamp: get_timestamp(&context),
        level: LogLevel::Info,
        activity: "SYSTEM".to_string(),
        message: UiConstants::EXECUTION_COMPLETE_MARKER.to_string(),
    });
}

// fn execute_scenario_with_context_old<L: LogOutput>(
//     scenario: &Scenario,
//     project: &Project,
//     context: &mut ExecutionContext,
//     log: &mut L,
// ) {
//     let timestamp = get_timestamp(context);
//     log.log(LogEntry {
//         timestamp,
//         level: LogLevel::Info,
//         activity: "SCENARIO".to_string(),
//         message: format!("Executing scenario: {}", scenario.name),
//     });
//
//     let start_node = scenario
//         .nodes
//         .iter()
//         .find(|n| matches!(n.activity, Activity::Start { .. }));
//     if start_node.is_none() {
//         let timestamp = get_timestamp(context);
//         log.log(LogEntry {
//             timestamp,
//             level: LogLevel::Error,
//             activity: "SCENARIO".to_string(),
//             message: format!(
//                 "Scenario '{}' does not have a Start node. Execution aborted.",
//                 scenario.name
//             ),
//         });
//         return;
//     }
//
//     let start_id = start_node.unwrap().id;
//
//     execute_from_node(scenario, start_id, project, context, log);
//
//     let timestamp = get_timestamp(context);
//     log.log(LogEntry {
//         timestamp,
//         level: LogLevel::Info,
//         activity: "SCENARIO".to_string(),
//         message: format!("Scenario '{}' completed.", scenario.name),
//     });
// }

// fn execute_from_node<L: LogOutput>(
//     scenario: &Scenario,
//     node_id: Uuid,
//     project: &Project,
//     context: &mut ExecutionContext,
//     log: &mut L,
// ) {
//     if context.is_stopped() {
//         return;
//     }
//
//     let node = match scenario.get_node(node_id) {
//         Some(n) => n,
//         None => return,
//     };
//
//     if let Activity::IfCondition { condition } = &node.activity {
//         let timestamp = get_timestamp(context);
//         let result = evaluator::evaluate(condition, &context.variables);
//
//         let (message, branch_type) = match result {
//             Ok(VariableValue::Boolean(true)) => (
//                 format!("Condition '{}' evaluated to: true", condition),
//                 BranchType::TrueBranch,
//             ),
//             Ok(VariableValue::Boolean(false)) => (
//                 format!("Condition '{}' evaluated to: false", condition),
//                 BranchType::FalseBranch,
//             ),
//             Ok(other) => (
//                 format!(
//                     "Condition '{}' evaluated to non-boolean value: {:?}",
//                     condition, other
//                 ),
//                 BranchType::FalseBranch,
//             ),
//             Err(err) => (
//                 format!("Condition '{}' failed with error: {}", condition, err),
//                 BranchType::FalseBranch,
//             ),
//         };
//
//         log.log(LogEntry {
//             timestamp,
//             level: LogLevel::Info,
//             activity: "IF".to_string(),
//             message: message,
//         });
//
//         let next_nodes: Vec<Uuid> = scenario
//             .connections
//             .iter()
//             .filter(|c| c.from_node == node_id && c.branch_type == branch_type)
//             .map(|c| c.to_node)
//             .collect();
//
//         for next_id in next_nodes {
//             execute_from_node(scenario, next_id, project, context, log);
//         }
//         return;
//     }
//
//     if let Activity::While { condition } = &node.activity {
//         let timestamp = get_timestamp(context);
//         log.log(LogEntry {
//             timestamp,
//             level: LogLevel::Info,
//             activity: "WHILE".to_string(),
//             message: format!("Starting while loop: {}", condition),
//         });
//
//         let loop_body: Vec<Uuid> = scenario
//             .connections
//             .iter()
//             .filter(|c| c.from_node == node_id && c.branch_type == BranchType::LoopBody)
//             .map(|c| c.to_node)
//             .collect();
//
//         let next_nodes: Vec<Uuid> = scenario
//             .connections
//             .iter()
//             .filter(|c| c.from_node == node_id && c.branch_type == BranchType::Default)
//             .map(|c| c.to_node)
//             .collect();
//
//         let mut iteration_count = 0;
//
//         while let Ok(VariableValue::Boolean(true)) =
//             evaluator::evaluate(condition, &context.variables)
//         {
//             if context.is_stopped() {
//                 break;
//             }
//
//             iteration_count += 1;
//
//             if context.max_iterations_enabled() && iteration_count > context.max_iterations {
//                 let timestamp = get_timestamp(context);
//                 log.log(LogEntry {
//                     timestamp,
//                     level: LogLevel::Warning,
//                     activity: "WHILE".to_string(),
//                     message: format!(
//                         "Max iterations limit ({}) reached, loop terminated",
//                         context.max_iterations
//                     ),
//                 });
//                 break;
//             }
//
//             let timestamp = get_timestamp(context);
//             log.log(LogEntry {
//                 timestamp,
//                 level: LogLevel::Info,
//                 activity: "WHILE".to_string(),
//                 message: format!("Iteration {}: condition is true", iteration_count),
//             });
//
//             for body_node_id in &loop_body {
//                 execute_from_node(scenario, *body_node_id, project, context, log);
//             }
//         }
//
//         let timestamp = get_timestamp(context);
//         log.log(LogEntry {
//             timestamp,
//             level: LogLevel::Info,
//             activity: "WHILE".to_string(),
//             message: format!("Completed {} iterations", iteration_count),
//         });
//
//         for next_id in next_nodes {
//             execute_from_node(scenario, next_id, project, context, log);
//         }
//         return;
//     }
//
//     if let Activity::Loop {
//         start,
//         end,
//         step,
//         index,
//     } = &node.activity
//     {
//         let timestamp = get_timestamp(context);
//         let total_iterations = if *step == 0 {
//             0
//         } else {
//             ((*end - *start) / *step).abs()
//         };
//
//         log.log(LogEntry {
//             timestamp,
//             level: LogLevel::Info,
//             activity: "LOOP".to_string(),
//             message: format!(
//                 "Starting loop: {} from {} to {} step {}",
//                 index, start, end, step
//             ),
//         });
//
//         let loop_body: Vec<Uuid> = scenario
//             .connections
//             .iter()
//             .filter(|c| c.from_node == node_id && c.branch_type == BranchType::LoopBody)
//             .map(|c| c.to_node)
//             .collect();
//
//         let next_nodes: Vec<Uuid> = scenario
//             .connections
//             .iter()
//             .filter(|c| c.from_node == node_id && c.branch_type == BranchType::Default)
//             .map(|c| c.to_node)
//             .collect();
//
//         if *step == 0 {
//             let timestamp = get_timestamp(context);
//             log.log(LogEntry {
//                 timestamp,
//                 level: LogLevel::Warning,
//                 activity: "LOOP".to_string(),
//                 message: "Step is 0, loop skipped".to_string(),
//             });
//         } else {
//             let mut iteration_count = 0;
//             let mut current = *start;
//
//             while if *step > 0 {
//                 current < *end
//             } else {
//                 current > *end
//             } {
//                 if context.is_stopped() {
//                     break;
//                 }
//
//                 iteration_count += 1;
//
//                 if context.max_iterations_enabled() && iteration_count > context.max_iterations {
//                     let timestamp = get_timestamp(context);
//                     log.log(LogEntry {
//                         timestamp,
//                         level: LogLevel::Warning,
//                         activity: "LOOP".to_string(),
//                         message: format!(
//                             "Max iterations limit ({}) reached, loop terminated",
//                             context.max_iterations
//                         ),
//                     });
//                     break;
//                 }
//
//                 let timestamp = get_timestamp(context);
//                 log.log(LogEntry {
//                     timestamp,
//                     level: LogLevel::Info,
//                     activity: "LOOP".to_string(),
//                     message: format!(
//                         "Iteration {}/{}: {} = {}",
//                         iteration_count, total_iterations, index, current
//                     ),
//                 });
//
//                 context.set_variable(index.clone(), VariableValue::Number(current as f64));
//
//                 for body_node_id in &loop_body {
//                     execute_from_node(scenario, *body_node_id, project, context, log);
//                 }
//
//                 current += *step;
//             }
//
//             context.variables.shift_remove(index);
//             if let Some(sender) = &context.variable_sender {
//                 let _ = sender.send(context.variables.clone());
//             }
//
//             let timestamp = get_timestamp(context);
//             log.log(LogEntry {
//                 timestamp,
//                 level: LogLevel::Info,
//                 activity: "LOOP".to_string(),
//                 message: format!("Completed {} iterations", iteration_count),
//             });
//         }
//
//         for next_id in next_nodes {
//             execute_from_node(scenario, next_id, project, context, log);
//         }
//         return;
//     }
//
//     if let Activity::TryCatch = &node.activity {
//         let try_nodes: Vec<Uuid> = scenario
//             .connections
//             .iter()
//             .filter(|c| c.from_node == node_id && c.branch_type == BranchType::TryBranch)
//             .map(|c| c.to_node)
//             .collect();
//
//         let catch_nodes: Vec<Uuid> = scenario
//             .connections
//             .iter()
//             .filter(|c| c.from_node == node_id && c.branch_type == BranchType::CatchBranch)
//             .map(|c| c.to_node)
//             .collect();
//
//         let timestamp = get_timestamp(context);
//         log.log(LogEntry {
//             timestamp,
//             level: LogLevel::Info,
//             activity: "TRY-CATCH".to_string(),
//             message: "Entering try block".to_string(),
//         });
//
//         let mut error_occurred = false;
//
//         for try_node_id in &try_nodes {
//             if let Some(try_node) = scenario.get_node(*try_node_id) {
//                 match execute_activity(&try_node.activity, project, context, log) {
//                     Ok(()) => {
//                         let next_from_try: Vec<Uuid> = scenario
//                             .connections
//                             .iter()
//                             .filter(|c| c.from_node == *try_node_id)
//                             .map(|c| c.to_node)
//                             .collect();
//
//                         for next_id in next_from_try {
//                             execute_from_node(scenario, next_id, project, context, log);
//                         }
//                     }
//                     Err(error_msg) => {
//                         error_occurred = true;
//                         context.set_variable(
//                             "last_error".to_string(),
//                             VariableValue::String(error_msg.clone()),
//                         );
//
//                         let timestamp = get_timestamp(context);
//                         log.log(LogEntry {
//                             timestamp,
//                             level: LogLevel::Warning,
//                             activity: "TRY-CATCH".to_string(),
//                             message: format!("Error caught: {}", error_msg),
//                         });
//
//                         for catch_node_id in &catch_nodes {
//                             execute_from_node(scenario, *catch_node_id, project, context, log);
//                         }
//                         break;
//                     }
//                 }
//             }
//         }
//
//         if !error_occurred {
//             let timestamp = get_timestamp(context);
//             log.log(LogEntry {
//                 timestamp,
//                 level: LogLevel::Info,
//                 activity: "TRY-CATCH".to_string(),
//                 message: "Try block completed successfully".to_string(),
//             });
//         }
//
//         return;
//     }
//
//     match execute_activity(&node.activity, project, context, log) {
//         Ok(()) => {
//             // Success: follow default/success branch
//             let next_nodes: Vec<Uuid> = scenario
//                 .connections
//                 .iter()
//                 .filter(|c| c.from_node == node_id && c.branch_type == BranchType::Default)
//                 .map(|c| c.to_node)
//                 .collect();
//
//             for next_id in next_nodes {
//                 execute_from_node(scenario, next_id, project, context, log);
//             }
//         }
//         Err(error_msg) => {
//             // Error: follow error branch if it exists
//             context.set_variable(
//                 "last_error".to_string(),
//                 VariableValue::String(error_msg.clone()),
//             );
//
//             let error_nodes: Vec<Uuid> = scenario
//                 .connections
//                 .iter()
//                 .filter(|c| c.from_node == node_id && c.branch_type == BranchType::ErrorBranch)
//                 .map(|c| c.to_node)
//                 .collect();
//
//             if error_nodes.is_empty() {
//                 let timestamp = get_timestamp(context);
//                 log.log(LogEntry {
//                     timestamp,
//                     level: LogLevel::Error,
//                     activity: "ERROR".to_string(),
//                     message: format!(
//                         "Unhandled error: {}. No error handler connected.",
//                         error_msg
//                     ),
//                 });
//             } else {
//                 for next_id in error_nodes {
//                     execute_from_node(scenario, next_id, project, context, log);
//                 }
//             }
//         }
//     }
// }

// fn execute_activity<L: LogOutput>(
//     activity: &Activity,
//     project: &Project,
//     context: &mut ExecutionContext,
//     log: &mut L,
// ) -> Result<(), String> {
//     let timestamp = get_timestamp(context);
//
//     match activity {
//         Activity::Start { scenario_id } => {
//             if let Some(scenario) = project.scenarios.iter().find(|s| s.id == *scenario_id) {
//                 log.log(LogEntry {
//                     timestamp,
//                     level: LogLevel::Info,
//                     activity: "START".to_string(),
//                     message: format!("Starting scenario: {}", scenario.name),
//                 });
//             } else {
//                 let error_msg = format!("Scenario with ID {} not found", scenario_id);
//                 log.log(LogEntry {
//                     timestamp: get_timestamp(context),
//                     level: LogLevel::Error,
//                     activity: "START".to_string(),
//                     message: error_msg.clone(),
//                 });
//                 return Err(error_msg);
//             }
//         }
//         Activity::End { scenario_id } => {
//             if let Some(scenario) = project.scenarios.iter().find(|s| s.id == *scenario_id) {
//                 log.log(LogEntry {
//                     timestamp,
//                     level: LogLevel::Info,
//                     activity: "END".to_string(),
//                     message: format!("Ending scenario: {}", scenario.name),
//                 });
//             } else {
//                 let error_msg = format!("Scenario with ID {} not found", scenario_id);
//                 log.log(LogEntry {
//                     timestamp: get_timestamp(context),
//                     level: LogLevel::Error,
//                     activity: "END".to_string(),
//                     message: error_msg.clone(),
//                 });
//                 return Err(error_msg);
//             }
//         }
//         Activity::Log { level, message } => {
//             let resolved_message = context.resolve_value(message);
//             log.log(LogEntry {
//                 timestamp,
//                 level: level.clone(),
//                 activity: "LOG".to_string(),
//                 message: resolved_message,
//             });
//         }
//         Activity::Delay { milliseconds } => {
//             log.log(LogEntry {
//                 timestamp,
//                 level: LogLevel::Info,
//                 activity: "DELAY".to_string(),
//                 message: format!("Waiting for {} ms", milliseconds),
//             });
//             std::thread::sleep(std::time::Duration::from_millis(*milliseconds));
//         }
//         Activity::SetVariable {
//             name,
//             value,
//             var_type,
//         } => {
//             let final_value = if value.contains(UiConstants::VARIABLE_PLACEHOLDER_OPEN) {
//                 VariableValue::String(context.resolve_value(value))
//             } else if let Some(v) = context.get_variable(value) {
//                 v.clone()
//             } else {
//                 match VariableValue::from_string(value, var_type) {
//                     Ok(v) => v,
//                     Err(e) => {
//                         log.log(LogEntry {
//                             timestamp: timestamp.clone(),
//                             level: LogLevel::Warning,
//                             activity: "SET VAR".to_string(),
//                             message: format!(
//                                 "Type conversion failed for '{}': {}. Storing as string.",
//                                 name, e
//                             ),
//                         });
//                         VariableValue::String(value.clone())
//                     }
//                 }
//             };
//
//             context.set_variable(name.clone(), final_value.clone());
//
//             log.log(LogEntry {
//                 timestamp,
//                 level: LogLevel::Info,
//                 activity: "SET VAR".to_string(),
//                 message: format!("{} = {}", name, final_value),
//             });
//         }
//         Activity::GetVariable { name } => {
//             if let Some(value) = context.get_variable(name) {
//                 log.log(LogEntry {
//                     timestamp,
//                     level: LogLevel::Info,
//                     activity: "GET VAR".to_string(),
//                     message: format!("{} = {}", name, value),
//                 });
//             } else {
//                 log.log(LogEntry {
//                     timestamp,
//                     level: LogLevel::Warning,
//                     activity: "GET VAR".to_string(),
//                     message: format!("Variable '{}' not found", name),
//                 });
//             }
//         }
//         Activity::Evaluate { .. } => {}
//         Activity::IfCondition { .. } => {}
//         Activity::Loop { .. } => {}
//         Activity::While { .. } => {}
//         Activity::CallScenario { scenario_id } => {
//             if let Some(scenario) = project.scenarios.iter().find(|s| s.id == *scenario_id) {
//                 log.log(LogEntry {
//                     timestamp,
//                     level: LogLevel::Info,
//                     activity: "CALL".to_string(),
//                     message: format!("Calling scenario: {}", scenario.name),
//                 });
//                 execute_scenario_with_context_old(scenario, project, context, log);
//             } else {
//                 let error_msg = format!("Scenario with ID {} not found", scenario_id);
//                 log.log(LogEntry {
//                     timestamp: get_timestamp(context),
//                     level: LogLevel::Error,
//                     activity: "CALL".to_string(),
//                     message: error_msg.clone(),
//                 });
//                 return Err(error_msg);
//             }
//         }
//         Activity::RunPowershell { code: _ } => {
//             log.log(LogEntry {
//                 timestamp,
//                 level: LogLevel::Warning,
//                 activity: "RUN PWSH".to_string(),
//                 message: "[TODO] NOT IMPLEMENTED YET".to_string(),
//             });
//         }
//         Activity::Note { .. } => {}
//         Activity::TryCatch => {}
//     }
//
//     Ok(())
// }
