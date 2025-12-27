use crate::constants::UiConstants;
use crate::ir::{Instruction, IrBuilder, IrProgram};
use crate::node_graph::{LogEntry, LogLevel, Project, VariableDirection, VariableValue};
use crate::utils;
use crate::validation::ScenarioValidator;
use crate::variables::{VarEvent, Variables};
use crate::{evaluator, variables};
use indexmap::IndexMap;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::time::SystemTime;

pub struct CallFrame {
    pub scenario_id: String,
    pub return_address: usize,
    pub var_bindings: Vec<crate::node_graph::VariablesBinding>,
    pub saved_scenario_variables: variables::Variables,
}

pub struct ExecutionContext {
    start_time: SystemTime,
    variable_sender: Option<Sender<VarEvent>>,
    pub global_variables: variables::Variables,
    pub scenario_variables: variables::Variables,
    pub current_scenario_id: String,
    stop_flag: Arc<AtomicBool>,
}

pub struct IrExecutor<'a, L: LogOutput> {
    program: &'a IrProgram,
    project: &'a Project,
    context: &'a mut ExecutionContext,
    log: &'a mut L,
    error_handlers: Vec<usize>,
    iteration_counts: HashMap<usize, usize>,
    call_stack: Vec<CallFrame>,
    current_scenario_id: String,
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
        global_variables: variables::Variables,
        scenario_variables: variables::Variables,
        current_scenario_id: String,
        stop_flag: Arc<AtomicBool>,
    ) -> Self {
        Self {
            start_time,
            variable_sender: Some(sender),
            global_variables,
            scenario_variables,
            current_scenario_id,
            stop_flag,
        }
    }

    fn new_with_sender(sender: Sender<VarEvent>, stop_flag: Arc<AtomicBool>) -> Self {
        Self {
            start_time: SystemTime::now(),
            variable_sender: Some(sender),
            global_variables: variables::Variables::new(),
            scenario_variables: variables::Variables::new(),
            current_scenario_id: String::new(),
            stop_flag,
        }
    }

    pub fn new_without_sender(
        start_time: SystemTime,
        global_variables: variables::Variables,
        scenario_variables: variables::Variables,
        current_scenario_id: String,
        stop_flag: Arc<AtomicBool>,
    ) -> Self {
        Self {
            start_time,
            variable_sender: None,
            global_variables,
            scenario_variables,
            current_scenario_id,
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
        let current_scenario_id = project.main_scenario.id.clone();
        Self {
            program,
            project,
            context,
            log,
            error_handlers: Vec::new(),
            iteration_counts: HashMap::new(),
            call_stack: Vec::new(),
            current_scenario_id,
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

    fn resolve_variables_runtime(&self, template: &str) -> String {
        let mut out = String::with_capacity(template.len());
        let mut chars = template.char_indices().peekable();

        while let Some((i, c)) = chars.next() {
            if c == UiConstants::VARIABLE_PLACEHOLDER_OPEN {
                let start = i + c.len_utf8();
                let mut end = None;
                for (j, c2) in chars.by_ref() {
                    if c2 == UiConstants::VARIABLE_PLACEHOLDER_CLOSE {
                        end = Some(j);
                        break;
                    }
                }

                if let Some(end) = end {
                    let var_name = &template[start..end];
                    let var_value = self.get_variable_value(var_name);
                    if let Some(s) = var_value.as_str() {
                        out.push_str(s);
                    } else if !matches!(var_value, VariableValue::Undefined) {
                        use std::fmt::Write;
                        write!(out, "{}", var_value).unwrap();
                    }
                } else {
                    out.push(c);
                    break;
                }
            } else {
                out.push(c);
            }
        }

        out
    }

    fn get_variable_value(&self, name: &str) -> VariableValue {
        if let Some(val) = self.context.scenario_variables.get(name) {
            val.clone()
        } else if let Some(val) = self.context.global_variables.get(name) {
            val.clone()
        } else {
            VariableValue::Undefined
        }
    }

    fn get_combined_variables(&self) -> Variables {
        let mut combined = self.context.global_variables.clone();
        for (name, val, scope) in self.context.scenario_variables.iter() {
            combined.set(name, val.clone());
            combined.set_scope(name, scope.clone());
        }
        combined
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
                        message: format!("Starting scenario: {}", &scenario.name),
                    });
                    Ok(pc + 1)
                } else {
                    let error_msg = format!("Scenario with ID {scenario_id} not found");
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
                        message: format!("Ending scenario: {}", &scenario.name),
                    });

                    if let Some(frame) = self.call_stack.pop() {
                        for binding in &frame.var_bindings {
                            match binding.direction {
                                VariableDirection::Out | VariableDirection::InOut => {
                                    if let Some(param_value) = self
                                        .context
                                        .scenario_variables
                                        .get(&binding.target_var_name)
                                    {
                                        self.context
                                            .global_variables
                                            .set(&binding.source_var_name, param_value.clone());
                                        if let Some(ref sender) = self.context.variable_sender {
                                            let _ = sender.send(VarEvent::Set {
                                                name: binding.source_var_name.clone(),
                                                value: param_value.clone(),
                                            });
                                        }
                                    }
                                }
                                VariableDirection::In => {}
                            }
                        }
                        self.context.scenario_variables = frame.saved_scenario_variables;
                        self.current_scenario_id = frame.scenario_id.clone();
                        self.context.current_scenario_id = frame.scenario_id.clone();
                        Ok(frame.return_address)
                    } else {
                        Ok(self.program.instructions.len())
                    }
                } else {
                    let error_msg = format!("Scenario with ID {scenario_id} not found");
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
                let resolved_message = self.resolve_variables_runtime(message);
                self.log.log(LogEntry {
                    timestamp,
                    level: level.clone(),
                    activity: "LOG".to_string(),
                    message: resolved_message,
                });
                Ok(pc + 1)
            }
            Instruction::Delay { milliseconds } => {
                let timestamp = get_timestamp(self.context.start_time);
                self.log.log(LogEntry {
                    timestamp,
                    level: LogLevel::Info,
                    activity: "DELAY".to_string(),
                    message: format!("Waiting for {milliseconds} ms"),
                });

                if utils::interruptible_sleep(*milliseconds, &self.context.stop_flag.clone()) {
                    Ok(pc + 1)
                } else {
                    Err("Aborted".into())
                }
            }
            Instruction::SetVar { var, value, scope } => {
                let timestamp = get_timestamp(self.context.start_time);

                match scope {
                    crate::variables::VariableScope::Global => {
                        self.context.global_variables.set(var, value.clone());
                    }
                    crate::variables::VariableScope::Scenario => {
                        self.context.scenario_variables.set(var, value.clone());
                    }
                }

                if let Some(sender) = &self.context.variable_sender {
                    let _ = sender.send(VarEvent::Set {
                        name: var.clone(),
                        value: value.clone(),
                    });
                }

                self.log.log(LogEntry {
                    timestamp,
                    level: LogLevel::Info,
                    activity: "SET VAR".to_string(),
                    message: format!("{var:?} = {value}"),
                });
                Ok(pc + 1)
            }
            Instruction::Evaluate { expr } => {
                let timestamp = get_timestamp(self.context.start_time);

                let combined_vars = self.get_combined_variables();
                let result = match evaluator::eval_expr(expr, &combined_vars) {
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

                self.log.log(LogEntry {
                    timestamp,
                    level: LogLevel::Info,
                    activity: "EVALUATE".to_string(),
                    message: format!("Expression evaluated to {result}"),
                });

                Ok(pc + 1)
            }
            Instruction::Jump { target } => Ok(*target),
            Instruction::JumpIf { condition, target } => {
                let timestamp = get_timestamp(self.context.start_time);

                let combined_vars = self.get_combined_variables();
                let (message, next_pc, level) =
                    match evaluator::eval_expr(condition, &combined_vars) {
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
                            format!("Condition failed with error: {err}"),
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

                let combined_vars = self.get_combined_variables();
                let (message, next_pc, level) =
                    match evaluator::eval_expr(condition, &combined_vars) {
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
                            format!("Condition failed with error: {err}"),
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
                    let _ = sender.send(VarEvent::Set {
                        name: index.clone(),
                        value: VariableValue::Number(*start as f64),
                    });
                }

                self.context
                    .scenario_variables
                    .set(index, VariableValue::Number(*start as f64));

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
                    message: format!("Starting loop: from {start} to {end} step {step}"),
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
                    .scenario_variables
                    .get(index)
                    .and_then(|v| v.as_number())
                    .map_or(*end, |n| n as i64);

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
                let current = self
                    .context
                    .scenario_variables
                    .get(index)
                    .and_then(|v| v.as_number())
                    .unwrap() as i64;

                let next = current + step;

                self.context
                    .scenario_variables
                    .set(index, VariableValue::Number(next as f64));

                if let Some(sender) = &self.context.variable_sender {
                    let _ = sender.send(VarEvent::Set {
                        name: index.clone(),
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
                let combined_vars = self.get_combined_variables();
                match evaluator::eval_expr(condition, &combined_vars) {
                    Ok(VariableValue::Boolean(true)) => {
                        let iter_count = self.iteration_counts.entry(pc).or_insert(0);
                        *iter_count += 1;

                        self.log.log(LogEntry {
                            timestamp,
                            level: LogLevel::Info,
                            activity: "WHILE".to_string(),
                            message: format!("Iteration {iter_count}: condition is true"),
                        });
                        Ok(*body_target)
                    }
                    Ok(VariableValue::Boolean(false)) => {
                        let iter_count = self.iteration_counts.get(&pc).copied().unwrap_or(0);
                        self.log.log(LogEntry {
                            timestamp,
                            level: LogLevel::Info,
                            activity: "WHILE".to_string(),
                            message: format!("Completed {iter_count} iterations"),
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
            Instruction::CallScenario {
                scenario_id,
                parameters,
            } => {
                if scenario_id.is_empty() {
                    return Ok(pc + 1);
                }

                if self.call_stack.len() >= 100 {
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
                        level: LogLevel::Info,
                        activity: "CALL".to_string(),
                        message: format!("Entering scenario: {}", _scenario.name),
                    });

                    for binding in parameters {
                        match binding.direction {
                            VariableDirection::In | VariableDirection::InOut => {
                                let source_value = if binding.source_scope
                                    == Some(crate::variables::VariableScope::Global)
                                {
                                    self.context
                                        .global_variables
                                        .get(&binding.source_var_name)
                                        .cloned()
                                } else {
                                    self.context
                                        .scenario_variables
                                        .get(&binding.source_var_name)
                                        .cloned()
                                };
                                if let Some(val) = source_value {
                                    self.context
                                        .scenario_variables
                                        .set(&binding.target_var_name, val.clone());
                                    if let Some(ref sender) = self.context.variable_sender {
                                        let _ = sender.send(VarEvent::Set {
                                            name: binding.target_var_name.clone(),
                                            value: val.clone(),
                                        });
                                    }
                                }
                            }
                            VariableDirection::Out => {
                                self.context
                                    .scenario_variables
                                    .set(&binding.target_var_name, VariableValue::Undefined);
                                if let Some(ref sender) = self.context.variable_sender {
                                    let _ = sender.send(VarEvent::Set {
                                        name: binding.target_var_name.clone(),
                                        value: VariableValue::Undefined,
                                    });
                                }
                            }
                        }
                    }

                    let return_address = pc + 1;
                    let saved_scenario_variables = self.context.scenario_variables.clone();
                    self.context.scenario_variables = _scenario.variables.clone();

                    self.call_stack.push(CallFrame {
                        scenario_id: self.current_scenario_id.clone(),
                        return_address,
                        var_bindings: parameters.clone(),
                        saved_scenario_variables,
                    });
                    self.current_scenario_id = scenario_id.clone();
                    self.context.current_scenario_id = scenario_id.clone();

                    if let Some(&start_index) = self.program.scenario_start_index.get(scenario_id) {
                        Ok(start_index)
                    } else {
                        self.call_stack.pop();
                        Err(format!("Scenario {} not found in IR program", scenario_id))
                    }
                } else {
                    Err(format!(
                        "Scenario with ID {scenario_id} not found in project"
                    ))
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
        self.context
            .global_variables
            .set("last_error", VariableValue::String(error.clone()));

        if let Some(sender) = &self.context.variable_sender {
            let _ = sender.send(VarEvent::Set {
                name: "last_error".to_string(),
                value: VariableValue::String(error.clone()),
            });
        }

        if let Some(catch_target) = self.error_handlers.pop() {
            let timestamp = get_timestamp(self.context.start_time);
            self.log.log(LogEntry {
                timestamp,
                level: LogLevel::Warning,
                activity: "TRY-CATCH".to_string(),
                message: format!("Error caught: {error}"),
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
                message: format!("Unhandled error: {error}. No error handler connected."),
            });
            Err(error)
        }
    }
}

pub fn execute_project_with_vars(
    project: &Project,
    log_sender: &Sender<LogEntry>,
    var_sender: &Sender<VarEvent>,
    initial_vars: IndexMap<String, VariableValue>,
    stop_flag: Arc<AtomicBool>,
) {
    let mut context = ExecutionContext::new_with_sender(var_sender.clone(), stop_flag);

    for (name, value) in initial_vars {
        context.global_variables.set(&name, value);
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
            timestamp: "[00:00.00]".to_string(),
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
        &mut context.global_variables,
    );
    let program = match ir_builder.build() {
        Ok(prog) => prog,
        Err(e) => {
            let _ = log_sender.send(LogEntry {
                timestamp: get_timestamp(context.start_time),
                level: LogLevel::Error,
                activity: "SYSTEM".to_string(),
                message: format!("Execution error: {e}"),
            });
            let _ = log_sender.send(LogEntry {
                timestamp: "[00:00.00]".to_string(),
                level: LogLevel::Info,
                activity: "SYSTEM".to_string(),
                message: UiConstants::EXECUTION_COMPLETE_MARKER.to_string(),
            });
            return;
        }
    };

    context.current_scenario_id = project.main_scenario.id.clone();
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
    log_sender: &Sender<LogEntry>,
    var_sender: &Sender<VarEvent>,
    start_time: SystemTime,
    program: &IrProgram,
    global_variables: variables::Variables,
    stop_flag: Arc<AtomicBool>,
) {
    let scenario_variables = program.scenario_variables.clone();
    let current_scenario_id = project.main_scenario.id.clone();

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut context = ExecutionContext::new(
            start_time,
            var_sender.clone(),
            global_variables,
            scenario_variables,
            current_scenario_id,
            stop_flag,
        );

        let mut log = log_sender.clone();

        let mut executor = IrExecutor::new(program, project, &mut context, &mut log);
        if let Err(e) = executor.execute() {
            let _ = log_sender.send(LogEntry {
                timestamp: get_timestamp(context.start_time),
                level: LogLevel::Error,
                activity: "SYSTEM".to_string(),
                message: format!("Execution error: {e}"),
            });
        }
    }));

    if let Err(panic) = result {
        let panic_msg = if let Some(s) = panic.downcast_ref::<String>() {
            s.clone()
        } else if let Some(s) = panic.downcast_ref::<&str>() {
            s.to_string()
        } else {
            "Unknown panic occurred".to_string()
        };

        let _ = log_sender.send(LogEntry {
            timestamp: get_timestamp(start_time),
            level: LogLevel::Error,
            activity: "SYSTEM".to_string(),
            message: format!("Execution interrupted: {panic_msg}"),
        });
    }

    let _ = log_sender.send(LogEntry {
        timestamp: get_timestamp(start_time),
        level: LogLevel::Info,
        activity: "SYSTEM".to_string(),
        message: "Execution completed.".to_string(),
    });
    let _ = log_sender.send(LogEntry {
        timestamp: get_timestamp(start_time),
        level: LogLevel::Info,
        activity: "SYSTEM".to_string(),
        message: UiConstants::EXECUTION_COMPLETE_MARKER.to_string(),
    });
}

pub fn execute_scenario_with_vars(
    project: &Project,
    log_sender: &Sender<LogEntry>,
    var_sender: &Sender<VarEvent>,
    start_time: SystemTime,
    program: &IrProgram,
    global_variables: variables::Variables,
    stop_flag: Arc<AtomicBool>,
) {
    let scenario_variables = program.scenario_variables.clone();
    let current_scenario_id = project.main_scenario.id.clone();

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut context = ExecutionContext::new(
            start_time,
            var_sender.clone(),
            global_variables,
            scenario_variables,
            current_scenario_id,
            stop_flag,
        );

        let mut log = log_sender.clone();

        let mut executor = IrExecutor::new(program, project, &mut context, &mut log);
        if let Err(e) = executor.execute() {
            let _ = log_sender.send(LogEntry {
                timestamp: get_timestamp(context.start_time),
                level: LogLevel::Error,
                activity: "SYSTEM".to_string(),
                message: format!("Execution error: {e}"),
            });
        }
    }));

    if let Err(panic) = result {
        let panic_msg = if let Some(s) = panic.downcast_ref::<String>() {
            s.clone()
        } else if let Some(s) = panic.downcast_ref::<&str>() {
            s.to_string()
        } else {
            "Unknown panic occurred".to_string()
        };

        let _ = log_sender.send(LogEntry {
            timestamp: get_timestamp(start_time),
            level: LogLevel::Error,
            activity: "SYSTEM".to_string(),
            message: format!("Execution interrupted: {panic_msg}"),
        });
    }

    let _ = log_sender.send(LogEntry {
        timestamp: get_timestamp(start_time),
        level: LogLevel::Info,
        activity: "SYSTEM".to_string(),
        message: "Execution completed.".to_string(),
    });
    let _ = log_sender.send(LogEntry {
        timestamp: get_timestamp(start_time),
        level: LogLevel::Info,
        activity: "SYSTEM".to_string(),
        message: UiConstants::EXECUTION_COMPLETE_MARKER.to_string(),
    });
}
