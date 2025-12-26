use crate::execution::LogOutput;
use crate::node_graph::{Activity, BranchType, LogEntry, LogLevel, Project, Scenario};
use std::collections::{HashMap, HashSet, hash_map::DefaultHasher};
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, PartialEq)]
pub enum ValidationLevel {
    Error,
    Warning,
}

#[derive(Debug, Clone)]
pub struct ValidationIssue {
    pub level: ValidationLevel,
    pub node_id: Option<String>,
    pub message: String,
    pub code: String,
}

#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub errors: Vec<ValidationIssue>,
    pub warnings: Vec<ValidationIssue>,
    pub reachable_nodes: HashSet<String>,
}

impl Default for ValidationResult {
    fn default() -> Self {
        Self::new()
    }
}

impl ValidationResult {
    pub fn new() -> Self {
        Self {
            errors: Vec::new(),
            warnings: Vec::new(),
            reachable_nodes: HashSet::new(),
        }
    }

    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }

    pub fn log_to_output<L: LogOutput>(&self, log: &mut L, timestamp: &str) {
        if self.errors.is_empty() && self.warnings.is_empty() {
            return;
        }

        let error_count = self.errors.len();
        let warning_count = self.warnings.len();

        if error_count > 0 || warning_count > 0 {
            log.log(LogEntry {
                timestamp: timestamp.to_string(),
                level: if error_count > 0 {
                    LogLevel::Error
                } else {
                    LogLevel::Warning
                },
                activity: "SYSTEM".to_string(),
                message: format!(
                    "Validation: {} error{}, {} warning{}",
                    error_count,
                    if error_count == 1 { "" } else { "s" },
                    warning_count,
                    if warning_count == 1 { "" } else { "s" }
                ),
            });
        }

        for error in &self.errors {
            log.log(LogEntry {
                timestamp: timestamp.to_string(),
                level: LogLevel::Error,
                activity: "SYSTEM".to_string(),
                message: format!("[{}] {}", error.code, error.message),
            });
        }

        for warning in &self.warnings {
            log.log(LogEntry {
                timestamp: timestamp.to_string(),
                level: LogLevel::Warning,
                activity: "SYSTEM".to_string(),
                message: format!("[{}] {}", warning.code, warning.message),
            });
        }
    }
}

pub struct ScenarioValidator<'a> {
    scenario: &'a Scenario,
    project: &'a Project,
}

impl<'a> ScenarioValidator<'a> {
    pub fn new(scenario: &'a Scenario, project: &'a Project) -> Self {
        Self { scenario, project }
    }

    pub fn validate(&self) -> ValidationResult {
        let mut result = ValidationResult::new();

        let structural_issues = self.validate_structure();
        for issue in structural_issues {
            match issue.level {
                ValidationLevel::Error => result.errors.push(issue),
                ValidationLevel::Warning => result.warnings.push(issue),
            }
        }

        if !result.is_valid() {
            return result;
        }

        result.reachable_nodes = self.compute_reachable_from_start();

        let control_flow_issues = self.validate_control_flow(&result.reachable_nodes);
        for issue in control_flow_issues {
            match issue.level {
                ValidationLevel::Error => result.errors.push(issue),
                ValidationLevel::Warning => result.warnings.push(issue),
            }
        }

        let data_flow_issues = self.validate_data_flow(&result.reachable_nodes);
        for issue in data_flow_issues {
            match issue.level {
                ValidationLevel::Error => result.errors.push(issue),
                ValidationLevel::Warning => result.warnings.push(issue),
            }
        }

        result
    }

    fn validate_structure(&self) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();

        issues.extend(self.check_start_end_nodes());

        if !issues.iter().any(|i| i.level == ValidationLevel::Error) {
            issues.extend(self.check_connection_integrity());
            issues.extend(self.check_dead_end_paths());
            issues.extend(self.check_pin_coverage());
        }

        issues
    }

    fn check_start_end_nodes(&self) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();

        let start_node = self
            .scenario
            .nodes
            .iter()
            .find(|n| matches!(n.activity, Activity::Start { .. }));
        if start_node.is_none() {
            issues.push(ValidationIssue {
                level: ValidationLevel::Error,
                node_id: None,
                message: format!("Scenario '{}' is missing a Start node", self.scenario.name),
                code: "E001".to_string(),
            });
        }

        let end_node = self
            .scenario
            .nodes
            .iter()
            .find(|n| matches!(n.activity, Activity::End { .. }));
        if end_node.is_none() {
            issues.push(ValidationIssue {
                level: ValidationLevel::Error,
                node_id: None,
                message: format!("Scenario '{}' is missing an End node", self.scenario.name),
                code: "E002".to_string(),
            });
        }

        issues
    }

    fn check_connection_integrity(&self) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();
        let node_ids: HashSet<String> = self.scenario.nodes.iter().map(|n| n.id.clone()).collect();

        for conn in &self.scenario.connections {
            if !node_ids.contains(&conn.from_node) {
                issues.push(ValidationIssue {
                    level: ValidationLevel::Error,
                    node_id: None,
                    message: format!(
                        "Connection references non-existent source node {}",
                        conn.from_node
                    ),
                    code: "E004".to_string(),
                });
            }
            if !node_ids.contains(&conn.to_node) {
                issues.push(ValidationIssue {
                    level: ValidationLevel::Error,
                    node_id: None,
                    message: format!(
                        "Connection references non-existent target node {}",
                        conn.to_node
                    ),
                    code: "E004".to_string(),
                });
            }
        }

        issues
    }

    fn check_dead_end_paths(&self) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();

        let reachable_from_start = self.compute_reachable_from_start();
        let can_reach_end = self.compute_can_reach_end();
        let loop_body_nodes = self.compute_loop_body_nodes();

        for node_id in reachable_from_start {
            if !can_reach_end.contains(&node_id)
                && !loop_body_nodes.contains(&node_id)
                && let Some(node) = self.scenario.get_node(&node_id)
                && !matches!(node.activity, Activity::End { .. })
            {
                let activity_name = get_activity_name(&node.activity);
                issues.push(ValidationIssue {
                    level: ValidationLevel::Error,
                    node_id: Some(node_id.clone()),
                    message: format!(
                        "Node '{}' ({}) is reachable from Start but doesn't lead to End",
                        activity_name, node_id
                    ),
                    code: "E003".to_string(),
                });
            }
        }

        issues
    }

    fn check_pin_coverage(&self) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();
        let connected_nodes = self.get_connected_nodes();

        for node in &self.scenario.nodes {
            if !connected_nodes.contains(&node.id) {
                continue;
            }

            match &node.activity {
                Activity::IfCondition { .. } => {
                    if !self.has_connection(&node.id, BranchType::TrueBranch) {
                        issues.push(ValidationIssue {
                            level: ValidationLevel::Warning,
                            node_id: Some(node.id.clone()),
                            message: format!(
                                "If node ({}) is missing True branch connection",
                                node.id
                            ),
                            code: "W001".to_string(),
                        });
                    }
                    if !self.has_connection(&node.id, BranchType::FalseBranch) {
                        issues.push(ValidationIssue {
                            level: ValidationLevel::Warning,
                            node_id: Some(node.id.clone()),
                            message: format!(
                                "If node ({}) is missing False branch connection",
                                node.id
                            ),
                            code: "W002".to_string(),
                        });
                    }
                }
                Activity::TryCatch => {
                    if !self.has_connection(&node.id, BranchType::TryBranch) {
                        issues.push(ValidationIssue {
                            level: ValidationLevel::Warning,
                            node_id: Some(node.id.clone()),
                            message: format!(
                                "Try-Catch node ({}) is missing Try branch connection",
                                node.id
                            ),
                            code: "W003".to_string(),
                        });
                    }
                    if !self.has_connection(&node.id, BranchType::CatchBranch) {
                        issues.push(ValidationIssue {
                            level: ValidationLevel::Warning,
                            node_id: Some(node.id.clone()),
                            message: format!(
                                "Try-Catch node ({}) is missing Catch branch connection",
                                node.id
                            ),
                            code: "W004".to_string(),
                        });
                    }
                }
                Activity::Loop { .. } | Activity::While { .. } => {
                    if !self.has_connection(&node.id, BranchType::LoopBody) {
                        issues.push(ValidationIssue {
                            level: ValidationLevel::Warning,
                            node_id: Some(node.id.clone()),
                            message: format!(
                                "Loop node ({}) has no body connection, loop will be skipped",
                                node.id
                            ),
                            code: "W007".to_string(),
                        });
                    }
                }
                _ => {}
            }
        }

        issues
    }

    fn validate_control_flow(&self, reachable_nodes: &HashSet<String>) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();

        issues.extend(self.check_loop_parameters(reachable_nodes));
        issues.extend(self.check_condition_syntax(reachable_nodes));
        issues.extend(self.check_scenario_references(reachable_nodes));
        issues.extend(self.check_recursive_scenarios(100));

        issues
    }

    fn check_loop_parameters(&self, reachable_nodes: &HashSet<String>) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();

        for node in &self.scenario.nodes {
            if !reachable_nodes.contains(&node.id) {
                continue;
            }

            if let Activity::Loop {
                start, end, step, ..
            } = &node.activity
            {
                if *step == 0 {
                    issues.push(ValidationIssue {
                        level: ValidationLevel::Error,
                        node_id: Some(node.id.clone()),
                        message: format!(
                            "Loop node ({}) has step = 0, which would cause infinite loop",
                            node.id
                        ),
                        code: "E101".to_string(),
                    });
                } else if *step > 0 && start >= end {
                    issues.push(ValidationIssue {
                        level: ValidationLevel::Error,
                        node_id: Some(node.id.clone()),
                        message: format!(
                            "Loop node ({}) has invalid parameters: start ({}) >= end ({}) with positive step ({})",
                            node.id, start, end, step
                        ),
                        code: "E102".to_string(),
                    });
                } else if *step < 0 && start <= end {
                    issues.push(ValidationIssue {
                        level: ValidationLevel::Error,
                        node_id: Some(node.id.clone()),
                        message: format!(
                            "Loop node ({}) has invalid parameters: start ({}) <= end ({}) with negative step ({})",
                            node.id, start, end, step
                        ),
                        code: "E102".to_string(),
                    });
                }
            }
        }

        issues
    }

    fn check_condition_syntax(&self, reachable_nodes: &HashSet<String>) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();

        for node in &self.scenario.nodes {
            if !reachable_nodes.contains(&node.id) {
                continue;
            }

            let condition = match &node.activity {
                Activity::IfCondition { condition } => Some(condition),
                Activity::While { condition } => Some(condition),
                _ => None,
            };

            if let Some(cond) = condition
                && let Err(msg) = validate_condition_syntax(cond)
            {
                issues.push(ValidationIssue {
                    level: ValidationLevel::Error,
                    node_id: Some(node.id.clone()),
                    message: format!("Invalid condition '{}': {}", cond, msg),
                    code: "E104".to_string(),
                });
            }
        }

        issues
    }

    fn check_scenario_references(&self, reachable_nodes: &HashSet<String>) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();

        for node in &self.scenario.nodes {
            if !reachable_nodes.contains(&node.id) {
                continue;
            }

            if let Activity::CallScenario { scenario_id, .. } = &node.activity {
                if !scenario_id.is_empty() {
                    let scenario_exists = self.project.scenarios.iter().any(|s| s.id == *scenario_id)
                        || self.project.main_scenario.id == *scenario_id;

                    if !scenario_exists {
                        issues.push(ValidationIssue {
                            level: ValidationLevel::Error,
                            node_id: Some(node.id.clone()),
                            message: format!(
                                "CallScenario node ({}) references non-existent scenario {}",
                                node.id, scenario_id
                            ),
                            code: "E103".to_string(),
                        });
                    }
                }
            }
        }

        issues
    }

    fn check_recursive_scenarios(&self, depth_limit: usize) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();
        let mut visited = HashSet::new();
        let mut call_stack = Vec::new();

        if let Some(cycle) = self.find_scenario_call_chain(
            &self.scenario.id,
            &mut visited,
            &mut call_stack,
            depth_limit,
        ) {
            issues.push(ValidationIssue {
                level: ValidationLevel::Warning,
                node_id: None,
                message: format!(
                    "Recursive scenario call detected: {} (depth limit: {})",
                    cycle
                        .iter()
                        .map(|id| self.get_scenario_name(id))
                        .collect::<Vec<_>>()
                        .join(" -> "),
                    depth_limit
                ),
                code: "W006".to_string(),
            });
        }

        issues
    }

    fn find_scenario_call_chain(
        &self,
        scenario_id: &str,
        visited: &mut HashSet<String>,
        call_stack: &mut Vec<String>,
        depth_limit: usize,
    ) -> Option<Vec<String>> {
        if call_stack.len() >= depth_limit {
            return Some(call_stack.clone());
        }

        if call_stack.iter().any(|id| id == scenario_id) {
            let cycle_start = call_stack.iter().position(|id| id == scenario_id).unwrap();
            return Some(call_stack[cycle_start..].to_vec());
        }

        if visited.contains(scenario_id) {
            return None;
        }

        visited.insert(scenario_id.to_string());
        call_stack.push(scenario_id.to_string());

        let scenario = if self.project.main_scenario.id == scenario_id {
            &self.project.main_scenario
        } else {
            self.project
                .scenarios
                .iter()
                .find(|s| s.id == scenario_id)?
        };

        for node in &scenario.nodes {
            if let Activity::CallScenario {
                scenario_id: called_id,
                ..
            } = &node.activity
                && let Some(cycle) =
                    self.find_scenario_call_chain(called_id, visited, call_stack, depth_limit)
            {
                return Some(cycle);
            }
        }

        call_stack.pop();

        None
    }

    fn get_scenario_name(&self, scenario_id: &str) -> String {
        if self.project.main_scenario.id == scenario_id {
            self.project.main_scenario.name.clone()
        } else {
            self.project
                .scenarios
                .iter()
                .find(|s| s.id == scenario_id)
                .map(|s| s.name.clone())
                .unwrap_or_else(|| format!("<unknown:{}>", scenario_id))
        }
    }

    fn validate_data_flow(&self, reachable_nodes: &HashSet<String>) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();

        issues.extend(self.check_variable_names(reachable_nodes));
        issues.extend(self.check_undefined_variables(reachable_nodes));

        issues
    }

    fn check_variable_names(&self, reachable_nodes: &HashSet<String>) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();

        for node in &self.scenario.nodes {
            if !reachable_nodes.contains(&node.id) {
                continue;
            }

            match &node.activity {
                Activity::SetVariable { name, .. } => {
                    if name.is_empty() {
                        issues.push(ValidationIssue {
                            level: ValidationLevel::Error,
                            node_id: Some(node.id.clone()),
                            message: format!("Variable name is empty in node ({})", node.id),
                            code: "E201".to_string(),
                        });
                    }
                }
                Activity::Loop { index, .. } => {
                    if index.is_empty() {
                        issues.push(ValidationIssue {
                            level: ValidationLevel::Error,
                            node_id: Some(node.id.clone()),
                            message: format!(
                                "Loop index variable name is empty in node ({})",
                                node.id
                            ),
                            code: "E201".to_string(),
                        });
                    }
                }
                _ => {}
            }
        }

        issues
    }

    fn check_undefined_variables(&self, reachable_nodes: &HashSet<String>) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();
        let mut defined_vars: HashSet<String> = self.project.variables.names().cloned().collect();
        let mut used_vars: HashSet<String> = HashSet::new();

        let start_node = self
            .scenario
            .nodes
            .iter()
            .find(|n| matches!(n.activity, Activity::Start { .. }));
        if let Some(start) = start_node {
            self.collect_variable_usage(
                &start.id,
                reachable_nodes,
                &mut defined_vars,
                &mut used_vars,
            );
        }

        for var in used_vars {
            if !defined_vars.contains(&var) && !self.project.variables.contains(&var) {
                issues.push(ValidationIssue {
                    level: ValidationLevel::Warning,
                    node_id: None,
                    message: format!("Variable '{}' may be used before being defined", var),
                    code: "W005".to_string(),
                });
            }
        }

        issues
    }

    fn collect_variable_usage(
        &self,
        node_id: &str,
        reachable_nodes: &HashSet<String>,
        defined_vars: &mut HashSet<String>,
        used_vars: &mut HashSet<String>,
    ) {
        if !reachable_nodes.contains(node_id) {
            return;
        }

        let node = match self.scenario.get_node(node_id) {
            Some(n) => n,
            None => return,
        };

        match &node.activity {
            Activity::SetVariable { name, value, .. } => {
                self.extract_variables_from_string(&value.to_string(), used_vars);
                defined_vars.insert(name.clone());
            }
            Activity::Log { level: _, message } => {
                self.extract_variables_from_string(message, used_vars);
            }
            Activity::IfCondition { condition } | Activity::While { condition } => {
                self.extract_variables_from_string(condition, used_vars);
            }
            Activity::Loop { index, .. } => {
                defined_vars.insert(index.clone());
            }
            _ => {}
        }

        let next_nodes: Vec<String> = self
            .scenario
            .connections
            .iter()
            .filter(|c| c.from_node == node_id)
            .map(|c| c.to_node.clone())
            .collect();

        for next_id in next_nodes {
            self.collect_variable_usage(&next_id, reachable_nodes, defined_vars, used_vars);
        }
    }

    fn extract_variables_from_string(&self, text: &str, vars: &mut HashSet<String>) {
        let mut i = 0;
        let chars: Vec<char> = text.chars().collect();

        while i < chars.len() {
            if chars[i] == '{' {
                let mut j = i + 1;
                while j < chars.len() && chars[j] != '}' {
                    j += 1;
                }
                if j < chars.len() {
                    let var_name: String = chars[i + 1..j].iter().collect();
                    if !var_name.is_empty() {
                        vars.insert(var_name);
                    }
                    i = j + 1;
                } else {
                    i += 1;
                }
            } else {
                i += 1;
            }
        }
    }

    fn compute_reachable_from_start(&self) -> HashSet<String> {
        let start_node = self
            .scenario
            .nodes
            .iter()
            .find(|n| matches!(n.activity, Activity::Start { .. }));
        if let Some(start) = start_node {
            let mut reachable = HashSet::new();
            self.compute_reachable_recursive(&start.id, &mut reachable);
            reachable
        } else {
            HashSet::new()
        }
    }

    fn compute_reachable_recursive(&self, node_id: &str, reachable: &mut HashSet<String>) {
        if reachable.contains(node_id) {
            return;
        }

        reachable.insert(node_id.to_string());

        let next_nodes: Vec<String> = self
            .scenario
            .connections
            .iter()
            .filter(|c| c.from_node == node_id)
            .map(|c| c.to_node.clone())
            .collect();

        for next_id in next_nodes {
            self.compute_reachable_recursive(&next_id, reachable);
        }
    }

    fn compute_can_reach_end(&self) -> HashSet<String> {
        let end_node = self
            .scenario
            .nodes
            .iter()
            .find(|n| matches!(n.activity, Activity::End { .. }));
        if let Some(end) = end_node {
            let mut can_reach = HashSet::new();
            self.compute_reverse_reachable(&end.id, &mut can_reach);
            can_reach
        } else {
            HashSet::new()
        }
    }

    fn compute_reverse_reachable(&self, node_id: &str, can_reach: &mut HashSet<String>) {
        if can_reach.contains(node_id) {
            return;
        }

        can_reach.insert(node_id.to_string());

        let prev_nodes: Vec<String> = self
            .scenario
            .connections
            .iter()
            .filter(|c| c.to_node == node_id)
            .map(|c| c.from_node.clone())
            .collect();

        for prev_id in prev_nodes {
            self.compute_reverse_reachable(&prev_id, can_reach);
        }
    }

    fn get_connected_nodes(&self) -> HashSet<String> {
        let mut connected = HashSet::new();

        for conn in &self.scenario.connections {
            connected.insert(conn.from_node.clone());
            connected.insert(conn.to_node.clone());
        }

        connected
    }

    fn has_connection(&self, node_id: &str, branch_type: BranchType) -> bool {
        self.scenario
            .connections
            .iter()
            .any(|c| c.from_node == node_id && c.branch_type == branch_type)
    }

    fn compute_loop_body_nodes(&self) -> HashSet<String> {
        let mut all_loop_bodies = HashSet::new();

        for node in &self.scenario.nodes {
            if matches!(
                node.activity,
                Activity::Loop { .. } | Activity::While { .. }
            ) {
                let mut this_loop_body = HashSet::new();
                let body_starts = self
                    .scenario
                    .connections
                    .iter()
                    .filter(|c| c.from_node == node.id && c.branch_type == BranchType::LoopBody)
                    .map(|c| c.to_node.clone());

                for start in body_starts {
                    self.collect_loop_body_recursive(&start, &node.id, &mut this_loop_body);
                }

                all_loop_bodies.extend(this_loop_body);
            }
        }

        all_loop_bodies
    }

    fn collect_loop_body_recursive(
        &self,
        node_id: &str,
        loop_id: &str,
        collected: &mut HashSet<String>,
    ) {
        if collected.contains(node_id) {
            return;
        }

        collected.insert(node_id.to_string());

        let next_nodes: Vec<String> = self
            .scenario
            .connections
            .iter()
            .filter(|c| c.from_node == node_id)
            .map(|c| c.to_node.clone())
            .collect();

        for next_id in next_nodes {
            if next_id != loop_id {
                self.collect_loop_body_recursive(&next_id, loop_id, collected);
            }
        }
    }
}

fn get_activity_name(activity: &Activity) -> String {
    match activity {
        Activity::Start { .. } => "Start".to_string(),
        Activity::End { .. } => "End".to_string(),
        Activity::Log { level, message } => format!("Log {} '{}'", level.as_str(), message),
        Activity::Delay { milliseconds } => format!("Delay {}ms", milliseconds),
        Activity::SetVariable { name, .. } => format!("SetVar '{}'", name),
        Activity::Evaluate { expression } => format!("Expression '{}'", expression),
        Activity::IfCondition { condition } => format!("If '{}'", condition),
        Activity::Loop { index, .. } => format!("Loop '{}'", index),
        Activity::While { condition } => format!("While '{}'", condition),
        Activity::CallScenario { .. } => "CallScenario".to_string(),
        Activity::RunPowershell { .. } => "RunPowershell".to_string(),
        Activity::Note { .. } => "Note".to_string(),
        Activity::TryCatch => "TryCatch".to_string(),
    }
}

fn validate_condition_syntax(condition: &str) -> Result<(), String> {
    let condition = condition.trim();

    if condition.is_empty() {
        return Err("Condition is empty".to_string());
    }

    let operators = ["==", "!=", ">=", "<=", ">", "<"];
    let has_operator = operators.iter().any(|op| condition.contains(op));

    if has_operator {
        for op in &operators {
            if let Some(pos) = condition.find(op) {
                let left = condition[..pos].trim();
                let right = condition[pos + op.len()..].trim();

                if left.is_empty() {
                    return Err(format!("Left side of '{}' is empty", op));
                }
                if right.is_empty() {
                    return Err(format!("Right side of '{}' is empty", op));
                }

                return Ok(());
            }
        }
    }

    Ok(())
}

pub struct ValidationCache {
    cache: HashMap<(String, u64), ValidationResult>,
}

impl Default for ValidationCache {
    fn default() -> Self {
        Self::new()
    }
}

impl ValidationCache {
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
        }
    }

    pub fn get(&self, scenario: &Scenario) -> Option<ValidationResult> {
        let hash = Self::compute_hash(scenario);
        self.cache.get(&(scenario.id.clone(), hash)).cloned()
    }

    pub fn insert(&mut self, scenario: &Scenario, result: ValidationResult) {
        let hash = Self::compute_hash(scenario);
        self.cache.insert((scenario.id.clone(), hash), result);
    }

    pub fn invalidate(&mut self, scenario_id: &str) {
        self.cache.retain(|(id, _), _| id != scenario_id);
    }

    fn compute_hash(scenario: &Scenario) -> u64 {
        let mut hasher = DefaultHasher::new();

        for node in &scenario.nodes {
            node.id.hash(&mut hasher);
            hash_activity(&node.activity, &mut hasher);
        }

        for conn in &scenario.connections {
            conn.from_node.hash(&mut hasher);
            conn.to_node.hash(&mut hasher);
            hash_branch_type(&conn.branch_type, &mut hasher);
        }

        hasher.finish()
    }
}

fn hash_activity(activity: &Activity, hasher: &mut DefaultHasher) {
    match activity {
        Activity::Start { scenario_id } => {
            0_u8.hash(hasher);
            scenario_id.hash(hasher);
        }
        Activity::End { scenario_id } => {
            1_u8.hash(hasher);
            scenario_id.hash(hasher);
        }
        Activity::Log { level, message } => {
            2_u8.hash(hasher);
            level.as_str().hash(hasher);
            message.hash(hasher);
        }
        Activity::Delay { milliseconds } => {
            3_u8.hash(hasher);
            milliseconds.hash(hasher);
        }
        Activity::SetVariable {
            name,
            value,
            var_type,
        } => {
            4_u8.hash(hasher);
            name.hash(hasher);
            value.hash(hasher);
            var_type.hash(hasher);
        }
        Activity::Evaluate { expression } => {
            6_u8.hash(hasher);
            expression.hash(hasher);
        }
        Activity::IfCondition { condition } => {
            7_u8.hash(hasher);
            condition.hash(hasher);
        }
        Activity::Loop {
            start,
            end,
            step,
            index,
        } => {
            8_u8.hash(hasher);
            start.hash(hasher);
            end.hash(hasher);
            step.hash(hasher);
            index.hash(hasher);
        }
        Activity::While { condition } => {
            9_u8.hash(hasher);
            condition.hash(hasher);
        }
        Activity::CallScenario { scenario_id, .. } => {
            10_u8.hash(hasher);
            scenario_id.hash(hasher);
        }
        Activity::RunPowershell { code } => {
            11_u8.hash(hasher);
            code.hash(hasher);
        }
        Activity::Note {
            text,
            width,
            height,
        } => {
            12_u8.hash(hasher);
            text.hash(hasher);
            width.to_bits().hash(hasher);
            height.to_bits().hash(hasher);
        }
        Activity::TryCatch => 13_u8.hash(hasher),
    }
}

fn hash_branch_type(branch_type: &BranchType, hasher: &mut DefaultHasher) {
    match branch_type {
        BranchType::Default => 0_u8.hash(hasher),
        BranchType::TrueBranch => 1_u8.hash(hasher),
        BranchType::FalseBranch => 2_u8.hash(hasher),
        BranchType::LoopBody => 3_u8.hash(hasher),
        BranchType::ErrorBranch => 4_u8.hash(hasher),
        BranchType::TryBranch => 5_u8.hash(hasher),
        BranchType::CatchBranch => 6_u8.hash(hasher),
    }
}

pub fn compute_call_graph(
    project: &Project,
) -> (HashMap<String, HashSet<String>>, HashSet<String>) {
    let mut call_graph: HashMap<String, HashSet<String>> = HashMap::new();
    let mut visited = HashSet::new();
    let mut in_progress = HashSet::new();
    let mut recursive_scenarios = HashSet::new();

    fn dfs(
        scenario_id: &str,
        project: &Project,
        call_graph: &mut HashMap<String, HashSet<String>>,
        visited: &mut HashSet<String>,
        in_progress: &mut HashSet<String>,
        recursive_scenarios: &mut HashSet<String>,
    ) {
        if visited.contains(scenario_id) {
            return;
        }

        if in_progress.contains(scenario_id) {
            recursive_scenarios.insert(scenario_id.to_string());
            return;
        }

        in_progress.insert(scenario_id.to_string());

        let scenario = if project.main_scenario.id == scenario_id {
            &project.main_scenario
        } else {
            match project.scenarios.iter().find(|s| s.id == scenario_id) {
                Some(s) => s,
                None => {
                    in_progress.remove(scenario_id);
                    visited.insert(scenario_id.to_string());
                    return;
                }
            }
        };

        let mut called_scenarios = HashSet::new();
        for node in &scenario.nodes {
            if let Activity::CallScenario { scenario_id: called_id, .. } = &node.activity {
                called_scenarios.insert(called_id.clone());
            }
        }

        call_graph.insert(scenario_id.to_string(), called_scenarios.clone());

        for called_id in called_scenarios {
            dfs(
                &called_id,
                project,
                call_graph,
                visited,
                in_progress,
                recursive_scenarios,
            );
        }

        in_progress.remove(scenario_id);
        visited.insert(scenario_id.to_string());
    }

    dfs(
        &project.main_scenario.id,
        project,
        &mut call_graph,
        &mut visited,
        &mut in_progress,
        &mut recursive_scenarios,
    );

    (call_graph, recursive_scenarios)
}

pub fn validate_variable_uniqueness(project: &Project) -> Result<(), String> {
    let mut all_variable_names = HashSet::new();

    let scenarios: Vec<&Scenario> = std::iter::once(&project.main_scenario)
        .chain(project.scenarios.iter())
        .collect();

    for scenario in scenarios {
        let mut scenario_vars = HashSet::new();

        for node in &scenario.nodes {
            match &node.activity {
                Activity::SetVariable { name, .. } => {
                    scenario_vars.insert(name.clone());
                }
                Activity::Loop { index, .. } => {
                    scenario_vars.insert(index.clone());
                }
                _ => {}
            }
        }

        for var_name in scenario_vars {
            if all_variable_names.contains(&var_name) {
                return Err(format!(
                    "Variable '{}' is defined in multiple scenarios. Variables must be unique across all reachable scenarios.",
                    var_name
                ));
            }
            all_variable_names.insert(var_name);
        }
    }

    Ok(())
}
