# If Node Handling in Nested Scenarios - Infinite Recursion Analysis

## EXECUTIVE SUMMARY

The If node (IfCondition) handling in nested scenarios has **critical architectural flaws** that can lead to infinite recursion:

1. **Incorrect Jump Instruction** (Line 796): Uses `JumpIf` instead of `JumpIfNot`
2. **Reversed Branch Order** (Lines 801-806): False branch compiled BEFORE true branch
3. **Missing Jump Patching** (Line 796-806): No target addresses calculated or patched
4. **No Cycle Detection**: Nested If nodes can create infinite recursion during compilation
5. **Shared Node Tracking**: The `compiled_nodes` HashSet tracks **all scenarios**, not per-scenario

---

## 1. IF CONDITION COMPILATION - MAIN SCENARIO vs NESTED SCENARIO

### Main Scenario: compile_if_node() (Lines 461-510)

**Correct Implementation:**

```rust
fn compile_if_node(&mut self, node_id: &str, condition: &str) -> Result<(), String> {
    let true_target = self.first_next_node(node_id, BranchType::TrueBranch);
    let false_target = self.first_next_node(node_id, BranchType::FalseBranch);
    
    let expr = parse_expr(condition, self.variables)?;
    
    // Line 472-475: Add JumpIfNot instruction (CORRECT!)
    let jump_if_not_idx = self.program.add_instruction(Instruction::JumpIfNot {
        condition: expr,
        target: 0,  // Will be patched
    });
    
    // Line 477-479: Compile TRUE branch first
    if let Some(node) = true_target {
        self.compile_from_node(&node)?;
    }
    
    // Line 481-488: Add unconditional jump over false branch
    let jump_over_false_idx = if false_target.is_some() {
        Some(self.program.add_instruction(Instruction::Jump { target: 0 }))
    } else {
        None
    };
    
    // Line 490-493: Compile FALSE branch second
    let false_start = self.program.instructions.len();
    if let Some(node) = false_target {
        self.compile_from_node(&node)?;
    }
    
    // Line 495-507: Backpatch jump targets
    let after_if = self.program.instructions.len();
    
    if let Instruction::JumpIfNot { target, .. } =
        &mut self.program.instructions[jump_if_not_idx]
    {
        *target = false_start;  // JumpIfNot -> false branch
    }
    
    if let Some(idx) = jump_over_false_idx
        && let Instruction::Jump { target } = &mut self.program.instructions[idx]
    {
        *target = after_if;  // Jump from true -> after if
    }
    
    Ok(())
}
```

**Key Points:**
- Uses `JumpIfNot` (condition false → skip true branch)
- Compiles TRUE branch first
- Places jump instruction to skip false branch
- Compiles FALSE branch second
- Backpatches both jump targets with correct addresses
- Targets are calculated AFTER compilation (know exact addresses)

---

### Nested Scenario: compile_from_called_scenario() (Lines 790-807)

**BROKEN Implementation:**

```rust
Activity::IfCondition { condition } => {
    let true_next = self.first_next_node_called(scenario, node_id, BranchType::TrueBranch);
    let false_next = self.first_next_node_called(scenario, node_id, BranchType::FalseBranch);
    
    let expr = parse_expr(condition, self.variables)?;
    
    // Line 796-799: PROBLEM #1 - Uses JumpIf instead of JumpIfNot!
    self.program.add_instruction(Instruction::JumpIf {
        condition: expr,
        target: 0,  // Target is 0 and NEVER patched!
    });
    
    // Line 801-803: PROBLEM #2 - FALSE branch first (wrong order!)
    if let Some(false_id) = false_next {
        self.compile_from_called_scenario(scenario, &false_id)?;
    }
    
    // Line 804-806: TRUE branch second
    if let Some(true_id) = true_next {
        self.compile_from_called_scenario(scenario, &true_id)?;
    }
    // PROBLEM #3 - NO jump instruction to skip between branches!
    // PROBLEM #4 - NO target patching!
}
```

**Critical Issues:**

1. **Wrong Instruction (Line 796)**: Uses `JumpIf` instead of `JumpIfNot`
   - `JumpIf`: condition TRUE → jump to target
   - Should be: `JumpIfNot`: condition FALSE → jump to false branch
   - This inverts the logic!

2. **Reversed Branch Order (Lines 801-806)**
   - False branch compiled first, then true branch
   - With wrong JumpIf semantics: creates incorrect control flow
   - Execution order doesn't match intent

3. **Missing Jump Instruction**
   - Main scenario adds a `Jump` to skip over false branch after true branch compiles
   - Nested scenario has NO skip instruction
   - Both branches execute sequentially regardless of condition

4. **Never-Patched Target (Line 798)**
   - JumpIf instruction created with `target: 0`
   - In main scenario, targets are backpatched AFTER branch compilation
   - In nested scenario: NO backpatching occurs
   - Target remains 0 (likely causing jump to start or undefined behavior)

5. **No Cycle Prevention**
   - Unlike main scenario using `compiled_nodes` check before recursion
   - Nested scenario can revisit same nodes if they share connections

---

## 2. COMPILED_NODES TRACKING - GLOBAL vs PER-SCENARIO

### Issue: Single HashSet for All Scenarios

**IrBuilder Structure (Lines 144-156):**

```rust
pub struct IrBuilder<'a> {
    scenario: &'a Scenario,
    project: &'a Project,
    program: IrProgram,
    reachable_nodes: &'a HashSet<String>,
    compiled_nodes: HashSet<String>,    // <-- SINGLE instance!
    node_start_index: HashMap<String, usize>,
    variables: &'a mut Variables,
    compiled_scenarios: HashSet<String>,
    call_graph: HashMap<String, HashSet<String>>,
    recursive_scenarios: HashSet<String>,
}
```

**The Problem:**

1. **compile_from_node()** (Lines 347-459)
   - Called only for MAIN scenario
   - Checks: `if self.compiled_nodes.contains(node_id)` (Line 352)
   - Inserts: `self.compiled_nodes.insert(node_id.to_string())` (Line 363)

2. **compile_from_called_scenario()** (Lines 731-821)
   - Called for NESTED scenarios
   - ALSO checks: `if self.compiled_nodes.contains(node_id)` (Line 732)
   - ALSO inserts: `self.compiled_nodes.insert(node_id.to_string())` (Line 742)

3. **Node ID Collision Risk**
   - If main scenario has node "if_1" 
   - And nested scenario also has node "if_1"
   - The HashSet treats them as IDENTICAL
   - Second scenario's "if_1" is never compiled (marked already compiled)

**Impact on If Nodes:**

- If node is visited in main scenario during compilation
- Same node ID in nested scenario is skipped entirely
- Leads to incomplete IR generation
- Can cause: infinite loops in execution, missed branches, incorrect logic

---

## 3. DEPTH LIMIT AND RECURSION PATTERNS

### Execution-Time Depth Limit (execution.rs, Line 532-533)

```rust
Instruction::CallScenario { scenario_id, parameters } => {
    if scenario_id.is_empty() {
        return Ok(pc + 1);
    }
    
    if self.call_stack.len() >= 100 {
        return Err("Maximum scenario call depth exceeded (100)".to_string());
    }
```

**Key Points:**
- Limit: 100 nested CallScenario calls
- Checked at EXECUTION time, not compilation
- Prevents infinite recursion during execution
- Does NOT prevent compilation-time infinite loops

### Compilation-Time Recursion Paths

**Path 1: Main Scenario (compile_from_node)**

```
compile_from_node(start_node)
  ├─ compile_from_node(next_node)
  │   └─ compile_default_next() checks compiled_nodes
  │       └─ if already compiled: return (prevents cycles)
  │       └─ else: recurse to next
  └─ ... continues sequentially
```

**Path 2: Nested Scenario (compile_from_called_scenario)**

```
compile_from_called_scenario(scenario_A, start)
  ├─ compile_from_called_scenario(scenario_A, next)
  │   └─ compile_default_next_called() calls compile_from_called_scenario()
  │       └─ same scenario_A, different node
  └─ If node processes:
      ├─ compile_from_called_scenario(scenario_A, true_branch)
      └─ compile_from_called_scenario(scenario_A, false_branch)
```

**Potential Infinite Recursion in Nested If Nodes:**

If nested scenario has cyclic connection like:
```
If Node A -> (True) -> If Node B -> (True) -> If Node A
```

Then:
1. `compile_from_called_scenario(A)` checks `compiled_nodes`
2. Inserts A in `compiled_nodes`
3. Processes If, calls `compile_from_c
