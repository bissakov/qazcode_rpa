use crate::UiConstants;
use crate::evaluator::{Expr, parse_expr};
use crate::variables::VarId;
use crate::{
    LogLevel, VariableValue,
    node_graph::{Activity, BranchType, Project, Scenario},
    variables::Variables,
};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub enum Instruction {
    Start {
        scenario_id: String,
    },
    End {
        scenario_id: String,
    },
    Log {
        level: LogLevel,
        message: String,
    },
    Delay {
        milliseconds: u64,
    },
    SetVar {
        var: VarId,
        value: VariableValue,
    },
    Evaluate {
        expr: Expr,
    },
    Jump {
        target: usize,
    },
    JumpIf {
        condition: Expr,
        target: usize,
    },
    JumpIfNot {
        condition: Expr,
        target: usize,
    },
    LoopInit {
        index: VarId,
        start: i64,
    },
    LoopLog {
        index: VarId,
        start: i64,
        end: i64,
        step: i64,
    },
    LoopCheck {
        index: VarId,
        end: i64,
        step: i64,
        body_target: usize,
        end_target: usize,
    },
    LoopNext {
        index: VarId,
        step: i64,
        check_target: usize,
    },
    WhileCheck {
        condition: Expr,
        body_target: usize,
        end_target: usize,
    },
    PushErrorHandler {
        catch_target: usize,
    },
    PopErrorHandler,
    CallScenario {
        scenario_id: String,
        parameters: Vec<crate::node_graph::ParameterBinding>,
    },
    RunPowershell {
        code: String,
    },
    DebugMarker {
        node_id: String,
        description: String,
    },
}

#[derive(Debug)]
pub struct IrProgram {
    pub instructions: Vec<Instruction>,
    pub entry_point: usize,
    pub scenario_start_index: HashMap<String, usize>,
    pub scenario_call_graph: HashMap<String, HashSet<String>>,
    pub recursive_scenarios: HashSet<String>,
}

impl Default for IrProgram {
    fn default() -> Self {
        Self::new()
    }
}

impl IrProgram {
    pub fn new() -> Self {
        Self {
            instructions: Vec::new(),
            entry_point: 0,
            scenario_start_index: HashMap::new(),
            scenario_call_graph: HashMap::new(),
            recursive_scenarios: HashSet::new(),
        }
    }

    pub fn add_instruction(&mut self, instr: Instruction) -> usize {
        let index = self.instructions.len();
        self.instructions.push(instr);
        index
    }

    pub fn get_instruction(&self, index: usize) -> Option<&Instruction> {
        self.instructions.get(index)
    }

    // #[allow(dead_code)]
    // fn format_ir(program: &IrProgram) -> String {
    //     let mut out = String::new();
    //
    //     out.push_str(&format!("Entry point: {}\n\n", program.entry_point));
    //
    //     for (i, instr) in program.instructions.iter().enumerate() {
    //         out.push_str(&format!("{:04}: {:?}\n", i, instr));
    //     }
    //
    //     out.push_str("\nNode → Instruction mapping:\n");
    //     for (node, idx) in &program.node_to_instruction {
    //         out.push_str(&format!("{} -> {}\n", node, idx));
    //     }
    //
    //     out
    // }
}

#[derive(Debug)]
pub struct IrBuilder<'a> {
    scenario: &'a Scenario,
    #[allow(dead_code)]
    project: &'a Project,
    program: IrProgram,
    reachable_nodes: &'a HashSet<String>,
    compiled_nodes: HashSet<String>,
    node_start_index: HashMap<String, usize>,
    variables: &'a mut Variables,
    compiled_scenarios: HashSet<String>,
    call_graph: HashMap<String, HashSet<String>>,
    recursive_scenarios: HashSet<String>,
}

// #[derive(Debug)]
// struct PendingJump {
//     instruction_index: usize,
//     target_node: Uuid,
//     #[allow(dead_code)]
//     jump_type: JumpType,
// }

// #[derive(Debug)]
// #[allow(dead_code)]
// enum JumpType {
//     Unconditional,
//     IfTrue,
//     IfFalse,
//     LoopBody,
//     LoopEnd,
//     WhileBody,
//     WhileEnd,
//     TryBranch,
//     CatchBranch,
// }

impl<'a> IrBuilder<'a> {
    pub fn new(
        scenario: &'a Scenario,
        project: &'a Project,
        reachable_nodes: &'a HashSet<String>,
        variables: &'a mut Variables,
    ) -> Self {
        let (call_graph, recursive_scenarios) = crate::validation::compute_call_graph(project);
        Self {
            scenario,
            project,
            program: IrProgram::new(),
            reachable_nodes,
            compiled_nodes: HashSet::new(),
            node_start_index: HashMap::new(),
            variables,
            compiled_scenarios: HashSet::new(),
            call_graph,
            recursive_scenarios,
        }
    }

    fn resolve_value(&mut self, value: &str) -> String {
        if (value.starts_with('"') && value.ends_with('"'))
            || (value.starts_with('\'') && value.ends_with('\''))
        {
            return value[1..value.len() - 1].to_owned();
        }

        let mut out = String::with_capacity(value.len());
        let mut chars = value.char_indices().peekable();

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
                    let var_name = &value[start..end];
                    let id = self.variables.id(var_name);
                    let var_value = self.variables.get(id);

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

    // fn resolve_value(&mut self, value: &String) -> String {
    //     if (value.starts_with('"') && value.ends_with('"'))
    //         || (value.starts_with('\'') && value.ends_with('\''))
    //     {
    //         return value[1..value.len() - 1].to_string();
    //     }
    //
    //     let mut result = value.to_string();
    //     let mut start_idx = 0;
    //
    //     while let Some(open_pos) = result[start_idx..].find(UiConstants::VARIABLE_PLACEHOLDER_OPEN)
    //     {
    //         let actual_open = start_idx + open_pos;
    //         if let Some(close_pos) =
    //             result[actual_open..].find(UiConstants::VARIABLE_PLACEHOLDER_CLOSE)
    //         {
    //             let actual_close = actual_open + close_pos;
    //             let var_name = &result[actual_open + 1..actual_close];
    //
    //             let id = self.variables.id(var_name);
    //             let var_value = self.variables.get(id);
    //             if !matches!(var_value, VariableValue::Undefined) {
    //                 let var_string = var_value.to_string();
    //                 result.replace_range(actual_open..=actual_close, &var_string);
    //                 start_idx = actual_open + var_string.len();
    //             } else {
    //                 start_idx = actual_close + 1;
    //             }
    //         } else {
    //             break;
    //         }
    //     }
    //
    //     result
    // }

    pub fn build(mut self) -> Result<IrProgram, String> {
        self.program.scenario_call_graph = self.call_graph.clone();
        self.program.recursive_scenarios = self.recursive_scenarios.clone();

        let start_node = self
            .scenario
            .nodes
            .iter()
            .find(|n| matches!(n.activity, Activity::Start { .. }))
            .ok_or("No Start node found")?;

        self.program.entry_point = self.program.instructions.len();
        self.program.scenario_start_index.insert(
            self.scenario.id.clone(),
            self.program.entry_point,
        );
        self.compiled_scenarios.insert(self.scenario.id.clone());

        self.compile_from_node(&start_node.id)?;

        // Compile all reachable scenarios that were referenced but not yet compiled
        self.compile_all_called_scenarios()?;

        Ok(self.program)
    }

    fn compile_all_called_scenarios(&mut self) -> Result<(), String> {
        let mut scenarios_to_compile = Vec::new();

        // Collect all scenarios that need to be compiled
        for scenario in std::iter::once(&self.project.main_scenario)
            .chain(self.project.scenarios.iter())
        {
            if !self.compiled_scenarios.contains(&scenario.id) {
                // Check if this scenario is reachable
                if self.call_graph.contains_key(&scenario.id) {
                    scenarios_to_compile.push(scenario.id.clone());
                }
            }
        }

        // Compile each scenario
        for scenario_id in scenarios_to_compile {
            self.compile_called_scenario(&scenario_id)?;
        }

        Ok(())
    }

    fn first_next_node(&self, node_id: &str, branch: BranchType) -> Option<String> {
        self.scenario
            .connections
            .iter()
            .find(|c| c.from_node == node_id && c.branch_type == branch)
            .map(|c| c.to_node.clone())
    }

    fn compile_default_next(&mut self, node_id: &str) -> Result<(), String> {
        if let Some(next) = self.first_next_node(node_id, BranchType::Default) {
            self.compile_from_node(&next)?;
        }
        Ok(())
    }

    fn compile_from_node(&mut self, node_id: &str) -> Result<(), String> {
        if !self.reachable_nodes.contains(node_id) {
            return Ok(());
        }

        if self.compiled_nodes.contains(node_id) {
            return Ok(());
        }

        let node = self
            .scenario
            .get_node(node_id)
            .ok_or_else(|| format!("Node {} not found", node_id))?;

        let start_index = self.program.instructions.len();
        self.node_start_index.insert(node_id.to_string(), start_index);
        self.compiled_nodes.insert(node_id.to_string());

        self.program.add_instruction(Instruction::DebugMarker {
            node_id: node_id.to_string(),
            description: format!("{:?}", node.activity),
        });

        match &node.activity {
            Activity::Start { scenario_id } => {
                let id = self.variables.id("last_error");
                self.variables.set(id, VariableValue::Undefined);
                self.program.add_instruction(Instruction::Start {
                    scenario_id: scenario_id.clone(),
                });
                self.compile_default_next(node_id)?;
            }
            Activity::End { scenario_id } => {
                self.program.add_instruction(Instruction::End {
                    scenario_id: scenario_id.clone(),
                });
            }
            Activity::Log { level, message } => {
                let resolved = self.resolve_value(message);
                self.program.add_instruction(Instruction::Log {
                    level: level.clone(),
                    message: resolved,
                });
                self.compile_default_next(node_id)?;
            }
            Activity::Delay { milliseconds } => {
                self.program.add_instruction(Instruction::Delay {
                    milliseconds: *milliseconds,
                });
                self.compile_default_next(node_id)?;
            }
            Activity::SetVariable {
                name,
                value,
                var_type,
            } => {
                let var_value = match VariableValue::from_string(value, var_type) {
                    Ok(v) => v,
                    Err(_) => VariableValue::String(value.to_string()),
                };

                let var_id = self.variables.id(name);
                self.program.add_instruction(Instruction::SetVar {
                    var: var_id,
                    value: var_value,
                });
                self.compile_default_next(node_id)?;
            }
            Activity::Evaluate { expression } => {
                let expr = parse_expr(expression, self.variables).map_err(|e| {
                    format!(
                        "Error in node {} while parsing expression '{}': {}",
                        node_id, expression, e
                    )
                })?;

                self.program.add_instruction(Instruction::Evaluate { expr });
                self.compile_default_next(node_id)?;
            }
            Activity::IfCondition { condition } => {
                self.compile_if_node(node_id, condition)?;
            }
            Activity::Loop {
                start,
                end,
                step,
                index,
            } => {
                self.compile_loop_node(node_id, *start, *end, *step, index)?;
            }
            Activity::While { condition } => {
                self.compile_while_node(node_id, condition)?;
            }
            Activity::TryCatch => {
                self.compile_try_catch_node(node_id)?;
            }
            Activity::CallScenario { scenario_id, parameters } => {
                self.program.add_instruction(Instruction::CallScenario {
                    scenario_id: scenario_id.clone(),
                    parameters: parameters.clone(),
                });
                self.compile_default_next(node_id)?;
            }
            Activity::RunPowershell { code } => {
                self.program
                    .add_instruction(Instruction::RunPowershell { code: code.clone() });
                self.compile_default_next(node_id)?;
            }
            Activity::Note { .. } => {}
        }

        Ok(())
    }

    fn compile_if_node(&mut self, node_id: &str, condition: &str) -> Result<(), String> {
        let true_target = self.first_next_node(node_id, BranchType::TrueBranch);
        let false_target = self.first_next_node(node_id, BranchType::FalseBranch);

        let expr = parse_expr(condition, self.variables).map_err(|e| {
            format!(
                "Error in node {} while parsing expression '{}': {}",
                node_id, condition, e
            )
        })?;

        let jump_if_not_idx = self.program.add_instruction(Instruction::JumpIfNot {
            condition: expr,
            target: 0,
        });

        if let Some(node) = true_target {
            self.compile_from_node(&node)?;
        }

        let jump_over_false_idx = if false_target.is_some() {
            Some(
                self.program
                    .add_instruction(Instruction::Jump { target: 0 }),
            )
        } else {
            None
        };

        let false_start = self.program.instructions.len();
        if let Some(node) = false_target {
            self.compile_from_node(&node)?;
        }

        let after_if = self.program.instructions.len();

        if let Instruction::JumpIfNot { target, .. } =
            &mut self.program.instructions[jump_if_not_idx]
        {
            *target = false_start;
        }

        if let Some(idx) = jump_over_false_idx
            && let Instruction::Jump { target } = &mut self.program.instructions[idx]
        {
            *target = after_if;
        }

        Ok(())
    }

    fn compile_loop_node(
        &mut self,
        node_id: &str,
        start: i64,
        end: i64,
        step: i64,
        index: &str,
    ) -> Result<(), String> {
        let body_node = self.first_next_node(node_id, BranchType::LoopBody);
        let after_node = self.first_next_node(node_id, BranchType::Default);

        if body_node.is_none() {
            if let Some(n) = after_node {
                self.compile_from_node(&n)?;
            }
            return Ok(());
        }

        let index_var = self.variables.id(index);
        self.variables
            .set(index_var, VariableValue::Number(start as f64));

        self.program.add_instruction(Instruction::LoopInit {
            index: index_var,
            start,
        });

        self.program.add_instruction(Instruction::LoopLog {
            index: index_var,
            start,
            end,
            step,
        });

        let check_idx = self.program.instructions.len();
        let loop_check_idx = self.program.add_instruction(Instruction::LoopCheck {
            index: index_var,
            end,
            step,
            body_target: 0,
            end_target: 0,
        });

        let body_start = self.program.instructions.len();
        self.compile_from_node(&body_node.unwrap())?;

        self.program.add_instruction(Instruction::LoopNext {
            index: index_var,
            step,
            check_target: check_idx,
        });

        let after_loop_start = self.program.instructions.len();
        if let Some(n) = after_node {
            self.compile_from_node(&n)?;
        }

        if let Instruction::LoopCheck {
            body_target,
            end_target,
            ..
        } = &mut self.program.instructions[loop_check_idx]
        {
            *body_target = body_start;
            *end_target = after_loop_start;
        }

        Ok(())
    }

    fn compile_while_node(&mut self, node_id: &str, condition: &str) -> Result<(), String> {
        let body_nodes = self.get_next_nodes(node_id, BranchType::LoopBody);
        let after_loop = self.get_next_nodes(node_id, BranchType::Default);

        if body_nodes.is_empty() {
            if let Some(after_node) = after_loop.first() {
                self.compile_from_node(after_node)?;
            }
            return Ok(());
        }

        let check_idx = self.program.instructions.len();

        let expr = parse_expr(condition, self.variables).map_err(|e| {
            format!(
                "Error in node {} while parsing expression '{}': {}",
                node_id, condition, e
            )
        })?;

        let while_check_idx = self.program.add_instruction(Instruction::WhileCheck {
            condition: expr,
            body_target: 0,
            end_target: 0,
        });

        let body_start = self.program.instructions.len();

        if let Some(body_node) = body_nodes.first() {
            self.compile_from_node(body_node)?;
        }

        self.program
            .add_instruction(Instruction::Jump { target: check_idx });

        let after_loop_start = self.program.instructions.len();

        if let Some(after_node) = after_loop.first() {
            self.compile_from_node(after_node)?;
        }

        if let Instruction::WhileCheck {
            body_target,
            end_target,
            ..
        } = &mut self.program.instructions[while_check_idx]
        {
            *body_target = body_start;
            *end_target = after_loop_start;
        }

        Ok(())
    }

    fn compile_try_catch_node(&mut self, node_id: &str) -> Result<(), String> {
        let try_nodes = self.get_next_nodes(node_id, BranchType::TryBranch);
        let catch_nodes = self.get_next_nodes(node_id, BranchType::CatchBranch);

        let push_handler_idx = self
            .program
            .add_instruction(Instruction::PushErrorHandler { catch_target: 0 });

        if let Some(try_node) = try_nodes.first() {
            self.compile_from_node(try_node)?;
        }

        self.program.add_instruction(Instruction::PopErrorHandler);

        let jump_after_catch_idx = self
            .program
            .add_instruction(Instruction::Jump { target: 0 });

        let catch_start = self.program.instructions.len();

        if let Some(catch_node) = catch_nodes.first() {
            self.compile_from_node(catch_node)?;
        }

        let after_catch = self.program.instructions.len();

        if let Instruction::PushErrorHandler { catch_target } =
            &mut self.program.instructions[push_handler_idx]
        {
            *catch_target = catch_start;
        }

        if let Instruction::Jump { target } = &mut self.program.instructions[jump_after_catch_idx] {
            *target = after_catch;
        }

        Ok(())
    }

    fn get_next_nodes(&self, node_id: &str, branch: BranchType) -> Vec<String> {
        self.scenario
            .connections
            .iter()
            .filter(|c| c.from_node == node_id && c.branch_type == branch)
            .map(|c| c.to_node.clone())
            .collect()
    }

    // fn resolve_pending_jumps(&mut self) -> Result<(), String> {
    //     for pending in &self.pending_jumps {
    //         let target_index = self
    //             .node_start_index
    //             .get(&pending.target_node)
    //             .ok_or_else(|| format!("Target node {} not compiled", pending.target_node))?;
    //
    //         match &mut self.program.instructions[pending.instruction_index] {
    //             Instruction::Jump { target } => *target = *target_index,
    //             Instruction::JumpIf { target, .. } => *target = *target_index,
    //             Instruction::JumpIfNot { target, .. } => *target = *target_index,
    //             _ => return Err("Invalid pending jump instruction".to_string()),
    //         }
    //     }
    //
    //     Ok(())
    // }

    fn compile_called_scenario(&mut self, scenario_id: &str) -> Result<(), String> {
        let scenario = self.project.scenarios
            .iter()
            .find(|s| s.id == scenario_id)
            .ok_or_else(|| format!("Scenario {} not found", scenario_id))?
            .clone();

        let start_node = scenario
            .nodes
            .iter()
            .find(|n| matches!(n.activity, Activity::Start { .. }))
            .ok_or(format!("No Start node found in scenario {}", scenario_id))?
            .clone();

        self.program.scenario_start_index.insert(
            scenario_id.to_string(),
            self.program.instructions.len(),
        );
        self.compiled_scenarios.insert(scenario_id.to_string());

        self.program.add_instruction(Instruction::Start {
            scenario_id: scenario_id.to_string(),
        });

        self.compile_from_called_scenario(&scenario, &start_node.id)?;

        Ok(())
    }

    fn compile_from_called_scenario(&mut self, scenario: &Scenario, node_id: &str) -> Result<(), String> {
        if self.compiled_nodes.contains(node_id) {
            return Ok(());
        }

        let node = scenario
            .get_node(node_id)
            .ok_or_else(|| format!("Node {} not found", node_id))?;

        let start_index = self.program.instructions.len();
        self.node_start_index.insert(node_id.to_string(), start_index);
        self.compiled_nodes.insert(node_id.to_string());

        self.program.add_instruction(Instruction::DebugMarker {
            node_id: node_id.to_string(),
            description: format!("{:?}", node.activity),
        });

        match &node.activity {
            Activity::Start { .. } => {
                self.compile_default_next_called(scenario, node_id)?;
            }
            Activity::End { .. } => {
                self.program.add_instruction(Instruction::End {
                    scenario_id: scenario.id.clone(),
                });
            }
            Activity::Log { level, message } => {
                let resolved = self.resolve_value(message);
                self.program.add_instruction(Instruction::Log {
                    level: level.clone(),
                    message: resolved,
                });
                self.compile_default_next_called(scenario, node_id)?;
            }
            Activity::Delay { milliseconds } => {
                self.program.add_instruction(Instruction::Delay {
                    milliseconds: *milliseconds,
                });
                self.compile_default_next_called(scenario, node_id)?;
            }
            Activity::SetVariable { name, value, var_type } => {
                let var_value = match VariableValue::from_string(value, var_type) {
                    Ok(v) => v,
                    Err(_) => VariableValue::String(value.to_string()),
                };

                let var_id = self.variables.id(name);
                self.program.add_instruction(Instruction::SetVar {
                    var: var_id,
                    value: var_value,
                });
                self.compile_default_next_called(scenario, node_id)?;
            }
            Activity::Evaluate { expression } => {
                let expr = parse_expr(expression, self.variables)?;
                self.program.add_instruction(Instruction::Evaluate { expr });
                self.compile_default_next_called(scenario, node_id)?;
            }
            Activity::IfCondition { condition } => {
                let true_next = self.first_next_node_called(scenario, node_id, BranchType::TrueBranch);
                let false_next = self.first_next_node_called(scenario, node_id, BranchType::FalseBranch);

                let expr = parse_expr(condition, self.variables)?;

                self.program.add_instruction(Instruction::JumpIf {
                    condition: expr,
                    target: 0,
                });

                if let Some(false_id) = false_next {
                    self.compile_from_called_scenario(scenario, &false_id)?;
                }
                if let Some(true_id) = true_next {
                    self.compile_from_called_scenario(scenario, &true_id)?;
                }
            }
            Activity::CallScenario { scenario_id, parameters } => {
                self.program.add_instruction(Instruction::CallScenario {
                    scenario_id: scenario_id.clone(),
                    parameters: parameters.clone(),
                });
                self.compile_default_next_called(scenario, node_id)?;
            }
            _ => {
                self.compile_default_next_called(scenario, node_id)?;
            }
        }

        Ok(())
    }

    fn first_next_node_called(&self, scenario: &Scenario, node_id: &str, branch: BranchType) -> Option<String> {
        scenario
            .connections
            .iter()
            .find(|c| c.from_node == node_id && c.branch_type == branch)
            .map(|c| c.to_node.clone())
    }

    fn compile_default_next_called(&mut self, scenario: &Scenario, node_id: &str) -> Result<(), String> {
        if let Some(next) = self.first_next_node_called(scenario, node_id, BranchType::Default) {
            self.compile_from_called_scenario(scenario, &next)?;
        }
        Ok(())
    }
}
