use crate::constants::UiConstants;
use crate::ir::{Instruction, IrProgram};
use crate::node_graph::{LogEntry, LogLevel, Project, VariableDirection};
use crate::stop_control::StopControl;
use crate::variables::{VariableScope, Variables};
use arc_script::{Value, eval_expr, parse_expr};
use std::collections::HashMap;
use std::sync::mpsc::Sender;
use std::sync::{Arc, RwLock};
use std::time::SystemTime;

#[derive(Debug, Clone)]
pub struct ScopeFrame {
    pub scenario_id: String,
    pub variables: Variables,
}

pub struct CallFrame {
    pub scenario_id: String,
    pub return_address: usize,
    pub var_bindings: Vec<crate::node_graph::VariablesBinding>,
}

pub struct ExecutionContext {
    pub start_time: SystemTime,
    pub global_variables: Variables,
    pub scope_stack: Vec<ScopeFrame>,
    pub stop_control: StopControl,
}

pub struct IrExecutor<'a, L: LogOutput> {
    program: &'a IrProgram,
    project: &'a Project,
    context: Arc<RwLock<ExecutionContext>>,
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
        scope_stack: Vec<ScopeFrame>,
        global_variables: Variables,
        stop_control: StopControl,
    ) -> Self {
        Self {
            start_time,
            global_variables,
            scope_stack,
            stop_control,
        }
    }

    pub fn new_without_sender(
        start_time: SystemTime,
        scope_stack: Vec<ScopeFrame>,
        global_variables: Variables,
        stop_control: StopControl,
    ) -> Self {
        Self {
            start_time,
            global_variables,
            scope_stack,
            stop_control,
        }
    }

    pub fn current_scenario_id(&self) -> Option<&str> {
        self.scope_stack.last().map(|f| f.scenario_id.as_str())
    }

    pub fn get_scenario_variables(&self) -> Option<&Variables> {
        self.scope_stack.last().map(|f| &f.variables)
    }

    pub fn get_scenario_variables_mut(&mut self) -> Option<&mut Variables> {
        self.scope_stack.last_mut().map(|f| &mut f.variables)
    }

    pub fn find_scenario_variables(
        &self,
        scenario_id: &crate::node_graph::NanoId,
    ) -> Option<&Variables> {
        self.scope_stack
            .iter()
            .find(|f| f.scenario_id == scenario_id.as_str())
            .map(|f| &f.variables)
    }

    pub fn resolve_variable(&self, name: &str) -> Option<Value> {
        if let Some(frame) = self.scope_stack.last()
            && let Some(val) = frame.variables.get(name)
        {
            return Some(val.clone());
        }
        self.global_variables.get(name).cloned()
    }

    pub fn set_variable(&mut self, name: &str, value: Value, scope: VariableScope) {
        match scope {
            VariableScope::Global => {
                self.global_variables.set(name, value, scope);
            }
            VariableScope::Scenario => {
                if let Some(frame) = self.scope_stack.last_mut() {
                    frame.variables.set(name, value, scope);
                }
            }
        }
    }

    pub fn is_stopped(&self) -> bool {
        self.stop_control.is_stopped()
    }
}

impl<'a, L: LogOutput> IrExecutor<'a, L> {
    pub fn new(
        program: &'a IrProgram,
        project: &'a Project,
        context: Arc<RwLock<ExecutionContext>>,
        log: &'a mut L,
    ) -> Self {
        let current_scenario_id = project.main_scenario.id.as_str().to_string();
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
            if self.context.read().unwrap().is_stopped() {
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

    fn get_combined_variables(&self) -> Variables {
        let ctx = self.context.read().unwrap();
        let mut combined = ctx.global_variables.clone();
        if let Some(vars) = ctx.get_scenario_variables() {
            for (name, val, scope) in vars.iter() {
                combined.set(name, val.clone(), scope.clone());
            }
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

                let timestamp = get_timestamp(self.context.read().unwrap().start_time);
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

                let timestamp = get_timestamp(self.context.read().unwrap().start_time);
                if let Some(scenario) = scenario {
                    self.log.log(LogEntry {
                        timestamp,
                        level: LogLevel::Info,
                        activity: "END".to_string(),
                        message: format!("Ending scenario: {}", &scenario.name),
                    });

                    if let Some(frame) = self.call_stack.pop() {
                        let mut param_values: Vec<(String, Value)> = Vec::new();
                        for binding in &frame.var_bindings {
                            if matches!(
                                binding.direction,
                                VariableDirection::Out | VariableDirection::InOut
                            ) && let Some(val) = self
                                .context
                                .read()
                                .unwrap()
                                .resolve_variable(&binding.target_var_name)
                            {
                                param_values.push((binding.source_var_name.clone(), val));
                            }
                        }

                        self.context.write().unwrap().scope_stack.pop();

                        for (var_name, value) in param_values {
                            self.context.write().unwrap().set_variable(
                                &var_name,
                                value,
                                VariableScope::Scenario,
                            );
                        }

                        self.current_scenario_id = frame.scenario_id.clone();
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
                let timestamp = get_timestamp(self.context.read().unwrap().start_time);
                let combined_vars = self.get_combined_variables();

                match parse_expr(message) {
                    Ok(expr) => {
                        match eval_expr(&expr, &combined_vars) {
                            Ok(value) => {
                                self.log.log(LogEntry {
                                    timestamp,
                                    level: level.clone(),
                                    activity: "LOG".to_string(),
                                    message: value.to_string(),
                                });
                                Ok(pc + 1)
                            }
                            Err(e) => Err(e),
                        }
                    }
                    Err(e) => Err(e),
                }
            }
            Instruction::Delay { milliseconds } => {
                let timestamp = get_timestamp(self.context.read().unwrap().start_time);
                self.log.log(LogEntry {
                    timestamp,
                    level: LogLevel::Info,
                    activity: "DELAY".to_string(),
                    message: format!("Waiting for {milliseconds} ms"),
                });

                if self
                    .context
                    .read()
                    .unwrap()
                    .stop_control
                    .sleep_interruptible(*milliseconds)
                {
                    Ok(pc + 1)
                } else {
                    Err("Execution stopped by user".into())
                }
            }
            Instruction::SetVar { var, value, scope } => {
                let timestamp = get_timestamp(self.context.read().unwrap().start_time);

                match scope {
                    VariableScope::Global => {
                        self.context.write().unwrap().global_variables.set(
                            var,
                            value.clone(),
                            scope.clone(),
                        );
                    }
                    VariableScope::Scenario => {
                        self.context.write().unwrap().set_variable(
                            var,
                            value.clone(),
                            scope.clone(),
                        );
                    }
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
                let timestamp = get_timestamp(self.context.read().unwrap().start_time);

                let combined_vars = self.get_combined_variables();
                let result = match eval_expr(expr, &combined_vars) {
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
                let timestamp = get_timestamp(self.context.read().unwrap().start_time);

                let combined_vars = self.get_combined_variables();
                let (message, next_pc, level) = match eval_expr(condition, &combined_vars) {
                    Ok(Value::Boolean(true)) => (
                        "Condition evaluated to: true".to_string(),
                        *target,
                        LogLevel::Info,
                    ),
                    Ok(Value::Boolean(false)) => (
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
                let timestamp = get_timestamp(self.context.read().unwrap().start_time);

                let combined_vars = self.get_combined_variables();
                let (message, next_pc, level) = match eval_expr(condition, &combined_vars) {
                    Ok(Value::Boolean(true)) => (
                        "Condition evaluated to: true".to_string(),
                        pc + 1,
                        LogLevel::Info,
                    ),
                    Ok(Value::Boolean(false)) => (
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
                self.context.write().unwrap().set_variable(
                    index,
                    Value::Number(*start as f64),
                    VariableScope::Scenario,
                );

                Ok(pc + 1)
            }
            Instruction::LoopLog {
                index: _,
                start,
                end,
                step,
            } => {
                self.log.log(LogEntry {
                    timestamp: get_timestamp(self.context.read().unwrap().start_time),
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
                        timestamp: get_timestamp(self.context.read().unwrap().start_time),
                        level: LogLevel::Warning,
                        activity: "LOOP".to_string(),
                        message: "Step is 0, loop skipped".to_string(),
                    });
                    return Ok(*end_target);
                }

                let current = {
                    let ctx = self.context.read().unwrap();
                    ctx.resolve_variable(index)
                        .and_then(|v: Value| v.as_number())
                        .map_or(*end, |n| n as i64)
                };

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
                let current = {
                    let ctx = self.context.read().unwrap();
                    ctx.resolve_variable(index)
                        .and_then(|v: Value| v.as_number())
                        .unwrap() as i64
                };

                let next = current + step;

                self.context.write().unwrap().set_variable(
                    index,
                    Value::Number(next as f64),
                    VariableScope::Scenario,
                );

                Ok(*check_target)
            }
            Instruction::WhileCheck {
                condition,
                body_target,
                end_target,
            } => {
                let timestamp = get_timestamp(self.context.read().unwrap().start_time);
                let combined_vars = self.get_combined_variables();
                match eval_expr(condition, &combined_vars) {
                    Ok(Value::Boolean(true)) => {
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
                    Ok(Value::Boolean(false)) => {
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
            Instruction::LoopContinue { check_target } => {
                let timestamp = get_timestamp(self.context.read().unwrap().start_time);
                self.log.log(LogEntry {
                    timestamp,
                    level: LogLevel::Info,
                    activity: "LOOP".to_string(),
                    message: "Continue to next iteration".to_string(),
                });
                Ok(*check_target)
            }
            Instruction::LoopBreak { end_target } => {
                let timestamp = get_timestamp(self.context.read().unwrap().start_time);
                self.log.log(LogEntry {
                    timestamp,
                    level: LogLevel::Info,
                    activity: "LOOP".to_string(),
                    message: "Breaking out of loop".to_string(),
                });
                Ok(*end_target)
            }
            Instruction::PushErrorHandler { catch_target } => {
                self.error_handlers.push(*catch_target);
                let timestamp = get_timestamp(self.context.read().unwrap().start_time);
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
                let timestamp = get_timestamp(self.context.read().unwrap().start_time);
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
                    let timestamp = get_timestamp(self.context.read().unwrap().start_time);
                    self.log.log(LogEntry {
                        timestamp,
                        level: LogLevel::Info,
                        activity: "CALL".to_string(),
                        message: format!("Entering scenario: {}", _scenario.name),
                    });

                    let mut child_scope = ScopeFrame {
                        scenario_id: scenario_id.as_str().to_string(),
                        variables: _scenario.variables.clone(),
                    };

                    for binding in parameters {
                        match binding.direction {
                            VariableDirection::In | VariableDirection::InOut => {
                                let source_value = {
                                    let ctx = self.context.read().unwrap();
                                    if binding.source_scope
                                        == Some(crate::variables::VariableScope::Global)
                                    {
                                        ctx.global_variables.get(&binding.source_var_name).cloned()
                                    } else {
                                        ctx.resolve_variable(&binding.source_var_name)
                                    }
                                };
                                if let Some(val) = source_value {
                                    child_scope.variables.set(
                                        &binding.target_var_name,
                                        val,
                                        VariableScope::Scenario,
                                    );
                                }
                            }
                            VariableDirection::Out => {
                                child_scope.variables.set(
                                    &binding.target_var_name,
                                    Value::Undefined,
                                    VariableScope::Scenario,
                                );
                            }
                        }
                    }

                    self.context.write().unwrap().scope_stack.push(child_scope);

                    self.call_stack.push(CallFrame {
                        scenario_id: self.current_scenario_id.clone(),
                        return_address: pc + 1,
                        var_bindings: parameters.clone(),
                    });

                    self.current_scenario_id = scenario_id.as_str().to_string();

                    if let Some(&start_index) = self.program.scenario_start_index.get(scenario_id) {
                        Ok(start_index)
                    } else {
                        self.call_stack.pop();
                        self.context.write().unwrap().scope_stack.pop();
                        Err(format!("Scenario {} not found in IR program", scenario_id))
                    }
                } else {
                    Err(format!(
                        "Scenario with ID {scenario_id} not found in project"
                    ))
                }
            }
            Instruction::RunPowershell { code: _ } => {
                let timestamp = get_timestamp(self.context.read().unwrap().start_time);
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
        if error == "Execution stopped by user" {
            let timestamp = get_timestamp(self.context.read().unwrap().start_time);
            self.log.log(LogEntry {
                timestamp,
                level: LogLevel::Info,
                activity: "SYSTEM".to_string(),
                message: "Execution stopped by user".to_string(),
            });
            return Err(error);
        }

        self.context.write().unwrap().global_variables.set(
            "last_error",
            Value::String(error.clone()),
            VariableScope::Global,
        );

        if let Some(catch_target) = self.error_handlers.pop() {
            let timestamp = get_timestamp(self.context.read().unwrap().start_time);
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
            let timestamp = get_timestamp(self.context.read().unwrap().start_time);
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

pub fn execute_project_with_typed_vars(
    project: &Project,
    log_sender: &Sender<LogEntry>,
    start_time: SystemTime,
    program: &IrProgram,
    global_variables: Variables,
    stop_control: StopControl,
) {
    let scope_stack = vec![ScopeFrame {
        scenario_id: project.main_scenario.id.as_str().to_string(),
        variables: project.main_scenario.variables.clone(),
    }];

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let context = Arc::new(RwLock::new(ExecutionContext::new(
            start_time,
            scope_stack,
            global_variables,
            stop_control,
        )));

        let mut log = log_sender.clone();

        let mut executor = IrExecutor::new(program, project, context.clone(), &mut log);
        if let Err(e) = executor.execute()
            && e != "Execution stopped by user"
        {
            let _ = log_sender.send(LogEntry {
                timestamp: get_timestamp(context.read().unwrap().start_time),
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
