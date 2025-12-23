use crate::variables::VarId;
use crate::{
    LogLevel, VariableValue,
    node_graph::{Activity, BranchType, Project, Scenario},
    variables::Variables,
};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub enum Instruction {
    Start {
        scenario_id: Uuid,
    },
    End {
        scenario_id: Uuid,
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
    GetVar {
        var: VarId,
    },
    Evaluate {
        expression: String,
    },
    Jump {
        target: usize,
    },
    JumpIf {
        condition: String,
        target: usize,
    },
    JumpIfNot {
        condition: String,
        target: usize,
    },
    LoopInit {
        start: i64,
        end: i64,
        step: i64,
        index: VarId,
        body_target: usize,
        end_target: usize,
    },
    LoopCheck {
        index: VarId,
        end: i64,
        step: i64,
        body_target: usize,
        end_target: usize,
    },
    LoopIncrement {
        index: VarId,
        step: i64,
        check_target: usize,
    },
    WhileCheck {
        condition: String,
        body_target: usize,
        end_target: usize,
    },
    PushErrorHandler {
        catch_target: usize,
    },
    PopErrorHandler,
    CallScenario {
        scenario_id: Uuid,
    },
    RunPowershell {
        code: String,
    },
    DebugMarker {
        node_id: Uuid,
        description: String,
    },
}

#[derive(Debug)]
pub struct IrProgram {
    pub instructions: Vec<Instruction>,
    pub entry_point: usize,
    pub node_to_instruction: HashMap<Uuid, usize>,
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
            node_to_instruction: HashMap::new(),
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

    #[allow(dead_code)]
    fn format_ir(program: &IrProgram) -> String {
        let mut out = String::new();

        out.push_str(&format!("Entry point: {}\n\n", program.entry_point));

        for (i, instr) in program.instructions.iter().enumerate() {
            out.push_str(&format!("{:04}: {:?}\n", i, instr));
        }

        out.push_str("\nNode → Instruction mapping:\n");
        for (node, idx) in &program.node_to_instruction {
            out.push_str(&format!("{} -> {}\n", node, idx));
        }

        out
    }
}

#[derive(Debug)]
pub struct IrBuilder<'a> {
    scenario: &'a Scenario,
    #[allow(dead_code)]
    project: &'a Project,
    program: IrProgram,
    reachable_nodes: &'a HashSet<Uuid>,
    compiled_nodes: HashSet<Uuid>,
    node_start_index: HashMap<Uuid, usize>,
    pending_jumps: Vec<PendingJump>,
    variables: &'a mut Variables,
}

#[derive(Debug)]
struct PendingJump {
    instruction_index: usize,
    target_node: Uuid,
    #[allow(dead_code)]
    jump_type: JumpType,
}

#[derive(Debug)]
#[allow(dead_code)]
enum JumpType {
    Unconditional,
    IfTrue,
    IfFalse,
    LoopBody,
    LoopEnd,
    WhileBody,
    WhileEnd,
    TryBranch,
    CatchBranch,
}

impl<'a> IrBuilder<'a> {
    pub fn new(
        scenario: &'a Scenario,
        project: &'a Project,
        reachable_nodes: &'a HashSet<Uuid>,
        var_registry: &'a mut Variables,
    ) -> Self {
        Self {
            scenario,
            project,
            program: IrProgram::new(),
            reachable_nodes,
            compiled_nodes: HashSet::new(),
            node_start_index: HashMap::new(),
            pending_jumps: Vec::new(),
            variables: var_registry,
        }
    }

    pub fn build(mut self) -> Result<IrProgram, String> {
        let start_node = self
            .scenario
            .nodes
            .iter()
            .find(|n| matches!(n.activity, Activity::Start { .. }))
            .ok_or("No Start node found")?;

        self.compile_from_node(start_node.id)?;

        self.resolve_pending_jumps()?;

        Ok(self.program)
    }

    fn compile_from_node(&mut self, node_id: Uuid) -> Result<(), String> {
        if !self.reachable_nodes.contains(&node_id) {
            return Ok(());
        }

        if self.compiled_nodes.contains(&node_id) {
            return Ok(());
        }

        let node = self
            .scenario
            .get_node(node_id)
            .ok_or_else(|| format!("Node {} not found", node_id))?;

        let start_index = self.program.instructions.len();
        self.node_start_index.insert(node_id, start_index);
        self.compiled_nodes.insert(node_id);

        self.program.add_instruction(Instruction::DebugMarker {
            node_id,
            description: format!("{:?}", node.activity),
        });

        match &node.activity {
            Activity::Start { scenario_id } => {
                self.program.add_instruction(Instruction::Start {
                    scenario_id: *scenario_id,
                });
                let next_nodes = self.get_next_nodes(node_id, BranchType::Default);
                if let Some(next) = next_nodes.first() {
                    self.compile_from_node(*next)?;
                }
            }
            Activity::End { scenario_id } => {
                self.program.add_instruction(Instruction::End {
                    scenario_id: *scenario_id,
                });
            }
            Activity::Log { level, message } => {
                self.program.add_instruction(Instruction::Log {
                    level: level.clone(),
                    message: message.clone(),
                });
                let next_nodes = self.get_next_nodes(node_id, BranchType::Default);
                if let Some(next) = next_nodes.first() {
                    self.compile_from_node(*next)?;
                }
            }
            Activity::Delay { milliseconds } => {
                self.program.add_instruction(Instruction::Delay {
                    milliseconds: *milliseconds,
                });
                let next_nodes = self.get_next_nodes(node_id, BranchType::Default);
                if let Some(next) = next_nodes.first() {
                    self.compile_from_node(*next)?;
                }
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
                let next_nodes = self.get_next_nodes(node_id, BranchType::Default);
                if let Some(next) = next_nodes.first() {
                    self.compile_from_node(*next)?;
                }
            }
            Activity::GetVariable { name } => {
                let var_id = self.variables.id(name);
                self.program
                    .add_instruction(Instruction::GetVar { var: var_id });
                let next_nodes = self.get_next_nodes(node_id, BranchType::Default);
                if let Some(next) = next_nodes.first() {
                    self.compile_from_node(*next)?;
                }
            }
            Activity::Evaluate { expression } => {
                self.program.add_instruction(Instruction::Evaluate {
                    expression: expression.clone(),
                });
                let next_nodes = self.get_next_nodes(node_id, BranchType::Default);
                if let Some(next) = next_nodes.first() {
                    self.compile_from_node(*next)?;
                }
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
            Activity::CallScenario { scenario_id } => {
                self.program.add_instruction(Instruction::CallScenario {
                    scenario_id: *scenario_id,
                });
                let next_nodes = self.get_next_nodes(node_id, BranchType::Default);
                if let Some(next) = next_nodes.first() {
                    self.compile_from_node(*next)?;
                }
            }
            Activity::RunPowershell { code } => {
                self.program
                    .add_instruction(Instruction::RunPowershell { code: code.clone() });
                let next_nodes = self.get_next_nodes(node_id, BranchType::Default);
                if let Some(next) = next_nodes.first() {
                    self.compile_from_node(*next)?;
                }
            }
            Activity::Note { .. } => {}
        }

        Ok(())
    }

    // fn compile_if_node(&mut self, node_id: Uuid, condition: &str) -> Result<(), String> {
    //     let true_branch = self.get_next_nodes(node_id, BranchType::TrueBranch);
    //     let false_branch = self.get_next_nodes(node_id, BranchType::FalseBranch);
    //
    //     let jump_if_instr_index = self.program.add_instruction(Instruction::JumpIf {
    //         condition: condition.to_string(),
    //         target: 0,
    //     });
    //
    //     if let Some(false_target) = false_branch.first() {
    //         self.pending_jumps.push(PendingJump {
    //             instruction_index: jump_if_instr_index,
    //             target_node: *false_target,
    //             jump_type: JumpType::IfFalse,
    //         });
    //         self.compile_from_node(*false_target)?;
    //     }
    //
    //     let true_branch_start = self.program.instructions.len();
    //
    //     if let Some(true_target) = true_branch.first() {
    //         self.compile_from_node(*true_target)?;
    //     }
    //
    //     if let Instruction::JumpIf { target, .. } =
    //         &mut self.program.instructions[jump_if_instr_index]
    //     {
    //         *target = true_branch_start;
    //     }
    //
    //     Ok(())
    // }

    fn compile_if_node(&mut self, node_id: Uuid, condition: &str) -> Result<(), String> {
        let true_branch = self.get_next_nodes(node_id, BranchType::TrueBranch);
        let false_branch = self.get_next_nodes(node_id, BranchType::FalseBranch);

        let jump_if_idx = self.program.add_instruction(Instruction::JumpIf {
            condition: condition.to_string(),
            target: 0,
        });

        if let Some(false_target) = false_branch.first() {
            self.compile_from_node(*false_target)?;
        }

        let jump_over_true_idx = self
            .program
            .add_instruction(Instruction::Jump { target: 0 });

        let true_branch_start = self.program.instructions.len();

        if let Some(true_target) = true_branch.first() {
            self.compile_from_node(*true_target)?;
        }

        let after_if = self.program.instructions.len();

        if let Instruction::JumpIf { target, .. } = &mut self.program.instructions[jump_if_idx] {
            *target = true_branch_start;
        }

        if let Instruction::Jump { target } = &mut self.program.instructions[jump_over_true_idx] {
            *target = after_if;
        }

        Ok(())
    }

    fn compile_loop_node(
        &mut self,
        node_id: Uuid,
        start: i64,
        end: i64,
        step: i64,
        index: &str,
    ) -> Result<(), String> {
        let body_nodes = self.get_next_nodes(node_id, BranchType::LoopBody);
        let after_loop = self.get_next_nodes(node_id, BranchType::Default);

        if body_nodes.is_empty() {
            if let Some(after_node) = after_loop.first() {
                self.compile_from_node(*after_node)?;
            }
            return Ok(());
        }

        let index_var = self.variables.id(index);

        let loop_init_idx = self.program.add_instruction(Instruction::LoopInit {
            start,
            end,
            step,
            index: index_var,
            body_target: 0,
            end_target: 0,
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

        if let Some(body_node) = body_nodes.first() {
            self.compile_from_node(*body_node)?;
        }

        self.program.add_instruction(Instruction::LoopIncrement {
            index: index_var,
            step,
            check_target: check_idx,
        });

        let after_loop_start = self.program.instructions.len();

        if let Some(after_node) = after_loop.first() {
            self.compile_from_node(*after_node)?;
        }

        if let Instruction::LoopInit {
            body_target,
            end_target,
            ..
        } = &mut self.program.instructions[loop_init_idx]
        {
            *body_target = body_start;
            *end_target = after_loop_start;
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

    fn compile_while_node(&mut self, node_id: Uuid, condition: &str) -> Result<(), String> {
        let body_nodes = self.get_next_nodes(node_id, BranchType::LoopBody);
        let after_loop = self.get_next_nodes(node_id, BranchType::Default);

        if body_nodes.is_empty() {
            if let Some(after_node) = after_loop.first() {
                self.compile_from_node(*after_node)?;
            }
            return Ok(());
        }

        let check_idx = self.program.instructions.len();

        let while_check_idx = self.program.add_instruction(Instruction::WhileCheck {
            condition: condition.to_string(),
            body_target: 0,
            end_target: 0,
        });

        let body_start = self.program.instructions.len();

        if let Some(body_node) = body_nodes.first() {
            self.compile_from_node(*body_node)?;
        }

        self.program
            .add_instruction(Instruction::Jump { target: check_idx });

        let after_loop_start = self.program.instructions.len();

        if let Some(after_node) = after_loop.first() {
            self.compile_from_node(*after_node)?;
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

    fn compile_try_catch_node(&mut self, node_id: Uuid) -> Result<(), String> {
        let try_nodes = self.get_next_nodes(node_id, BranchType::TryBranch);
        let catch_nodes = self.get_next_nodes(node_id, BranchType::CatchBranch);

        let push_handler_idx = self
            .program
            .add_instruction(Instruction::PushErrorHandler { catch_target: 0 });

        if let Some(try_node) = try_nodes.first() {
            self.compile_from_node(*try_node)?;
        }

        self.program.add_instruction(Instruction::PopErrorHandler);

        let jump_after_catch_idx = self
            .program
            .add_instruction(Instruction::Jump { target: 0 });

        let catch_start = self.program.instructions.len();

        if let Some(catch_node) = catch_nodes.first() {
            self.compile_from_node(*catch_node)?;
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

    fn get_next_nodes(&self, node_id: Uuid, branch: BranchType) -> Vec<Uuid> {
        self.scenario
            .connections
            .iter()
            .filter(|c| c.from_node == node_id && c.branch_type == branch)
            .map(|c| c.to_node)
            .collect()
    }

    fn resolve_pending_jumps(&mut self) -> Result<(), String> {
        for pending in &self.pending_jumps {
            let target_index = self
                .node_start_index
                .get(&pending.target_node)
                .ok_or_else(|| format!("Target node {} not compiled", pending.target_node))?;

            match &mut self.program.instructions[pending.instruction_index] {
                Instruction::Jump { target } => *target = *target_index,
                Instruction::JumpIf { target, .. } => *target = *target_index,
                Instruction::JumpIfNot { target, .. } => *target = *target_index,
                _ => return Err("Invalid pending jump instruction".to_string()),
            }
        }

        Ok(())
    }
}
