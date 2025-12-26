# Loop Node Implementation & Nested Scenario Execution - Search Findings

## EXECUTIVE SUMMARY

The codebase shows **asymmetric compilation support** between main and nested scenarios:
- **Main Scenario**: Full IR compilation including Loop, While, TryCatch
- **Nested Scenarios**: INCOMPLETE compilation - Loop/While/TryCatch silently skipped

This is a critical architectural issue where control flow structures in called scenarios don't execute.

---

## 1. LOOP NODE DEFINITION AND PROPERTIES

### File: `crates/rpa-core/src/node_graph.rs`

**Activity enum variant (Lines 589-593):**
```rust
Loop {
    start: i64,
    end: i64,
    step: i64,
    index: String,
}
```

**Pin Configuration (Lines 414, 497-506, 534-539):**
- Output count: 2 pins
  - Pin 0: LoopBody (body execution branch)
  - Pin 1: Default (normal exit/next branch)
- Used in Node::get_output_pin_count() and get_pin_index_for_branch()

**Branch Type (Line 659):**
```rust
enum BranchType {
    Default,
    TrueBranch,
    FalseBranch,
    LoopBody,      // <-- Loop body connection
    ErrorBranch,
    TryBranch,
    CatchBranch,
}
```

---

## 2. LOOP IR COMPILATION (Main Scenario Only)

### File: `crates/rpa-core/src/ir.rs`

**Entry Point in compile_from_node() (Lines 429-435):**
```rust
Activity::Loop {
    start,
    end,
    step,
    index,
} => {
    self.compile_loop_node(node_id, *start, *end, *step, index)?;
}
```

**Instruction Types Defined (Lines 44-65):**
```rust
enum Instruction {
    // ... other types ...
    LoopInit { index: VarId, start: i64 },
    LoopLog { index: VarId, start: i64, end: i64, step: i64 },
    LoopCheck { 
        index: VarId,
        end: i64,
        step: i64,
        body_target: usize,    // Patched later
        end_target: usize,     // Patched later
    },
    LoopNext {
        index: VarId,
        step: i64,
        check_target: usize,
    },
    // ... other types ...
}
```

**compile_loop_node() Function (Lines 512-580):**

Execution sequence:
1. Line 530: Get loop body node via BranchType::LoopBody
2. Line 534-537: Add LoopInit instruction
3. Line 539-544: Add LoopLog instruction
4. Line 546-553: Add LoopCheck with placeholder targets
5. Line 556: Recursively compile loop body via compile_from_node(&body_node)
6. Line 558-562: Add LoopNext instruction
7. Line 565-567: Compile after-loop node via BranchType::Default
8. Line 569-577: Backpatch LoopCheck targets

Key handling (Lines 523-527):
```rust
if body_node.is_none() {
    if let Some(n) = after_node {
        self.compile_from_node(&n)?;  // Skip loop if no body
    }
    return Ok(());
}
```

---

## 3. CRITICAL FINDING: Missing Loop Support in Nested Scenarios

### File: `crates/rpa-core/src/ir.rs` - Lines 731-821

**Function: compile_from_called_scenario()**

Pattern match handles (Lines 749-818):
- ✅ Activity::Start (750-751)
- ✅ Activity::End (753-756)
- ✅ Activity::Log (758-764)
- ✅ Activity::Delay (766-770)
- ✅ Activity::SetVariable (772-783)
- ✅ Activity::Evaluate (785-788)
- ✅ Activity::IfCondition (790-806)
- ✅ Activity::CallScenario (808-813)

**Missing handlers (CRITICAL):**
- ❌ Activity::Loop (no case - uses default)
- ❌ Activity::While (no case - uses default)
- ❌ Activity::TryCatch (no case - uses default)
- ❌ Activity::RunPowershell (no case - uses default)

**Default Case (Lines 815-817):**
```rust
_ => {
    self.compile_default_next_called(scenario, node_id)?;
}
```

This means:
- Loop nodes in nested scenarios are SILENTLY SKIPPED
- No error raised
- Only the next connection is compiled
- Loop body never executes

### Contrast with Main Scenario (Lines 347-459)

**compile_from_node()** handles all activity types:
```rust
match &node.activity {
    Activity::Start { .. } => { ... }
    Activity::End { .. } => { ... }
    Activity::Log { .. } => { ... }
    Activity::Delay { .. } => { ... }
    Activity::SetVariable { .. } => { ... }
    Activity::Evaluate { .. } => { ... }
    Activity::IfCondition { .. } => { ... }
    Activity::Loop { .. } => {  // <-- SUPPORTED
        self.compile_loop_node(node_id, *start, *end, *step, index)?;
    }
    Activity::While { .. } => {  // <-- SUPPORTED
        self.compile_while_node(node_id, condition)?;
    }
    Activity::TryCatch => {  // <-- SUPPORTED
        self.compile_try_catch_node(node_id)?;
    }
    Activity::CallScenario { .. } => { ... }
    Activity::RunPowershell { .. } => { ... }
    Activity::Note { .. } => {}
}
```

---

## 4. LOOP EXECUTION

### File: `crates/rpa-core/src/execution.rs`

**IrExecutor Field (Line 34-35):**
```rust
pub struct IrExecutor<'a, L: LogOutput> {
    // ...
    iteration_counts: HashMap<usize, usize>,  // For While loops
    call_stack: Vec<CallFrame>,                // For scenario calls
}
```

**LoopInit Execution (Lines 388-397):**
- Sets index variable to start value
- Broadcasts VarEvent::SetId if listener connected
- No state changes to loop mechanism

**LoopLog Execution (Lines 398-411):**
- Logs: "Starting loop: from X to Y step Z"
- No control flow changes

**LoopCheck Execution (Lines 413-448):**
```rust
if *step == 0 {
    self.log.log("Step is 0, loop skipped");
    return Ok(*end_target);
}

let current = self.context.variables.get(*index).as_number()...;
let should_continue = if *step > 0 {
    current < *end
} else {
    current > *end
};

if should_continue {
    Ok(*body_target)  // Jump to loop body
} else {
    Ok(*end_target)   // Jump past loop
}
```

**LoopNext Execution (Lines 449-470):**
- Increments index: `next = current + step`
- Sets updated value in variables
- Broadcasts VarEvent::SetId
- Returns `Ok(*check_target)` to jump back to LoopCheck

**Loop Execution Flow:**
```
LoopInit -> LoopLog -> [LoopCheck -> body -> LoopNext]* -> after-loop
                            |
                            +---(false condition)--> end_target
```

---

## 5. VARIABLE SCOPE AND LOOP INDICES

### File: `crates/rpa-core/src/execution.rs` - Lines 527-599

**CallScenario Execution:**

Parameter binding (Lines 558-581):
```rust
for binding in parameters {
    match binding.direction {
        In | InOut => {
            let source_value = self.context.variables.get(binding.source_var_id);
            self.context.variables.set(binding.param_var_id, source_value);
        }
        Out => {
            self.context.variables.set(binding.param_var_id, VariableValue::Undefined);
        }
    }
}
```

**Key Points:**
1. Loop indices are created via `Variables::id(name)` in IR compilation
2. Same Variables instance shared across ALL scenarios
3. No variable scope isolation between scenarios
4. Loop indices are NOT typically exposed as scenario parameters
5. Multiple scenarios with same index name = name collision in global namespace

**End Scenario Handling (Lines 211-227):**
Only Out/InOut parameters are copied back when scenario ends:
```rust
for binding in &frame.parameter_bindings {
    match binding.direction {
        Out | InOut => {
            let param_value = self.context.variables.get(binding.param_var_id);
            self.context.variables.set(binding.source_var_id, param_value);
        }
        In => {}
    }
}
```

---

## 6. LOOP VALIDATION

### File: `crates/rpa-core/src/validation.rs`

**Loop Parameter Validation (Lines 337-384):**

Checks performed (any scenario):
- Line 349: Step == 0 → Error "would cause infinite loop"
- Line 359: step > 0 AND start >= end → Error "invalid parameters"
- Line 369: step < 0 AND start <= end → Error "invalid parameters"

**Loop Body Validation (Lines 224-228, 306-314):**
- Checks for LoopBody connection existence
- If missing: "Loop node has no body connection, loop will be skipped"

**Loop Body Node Collection (Lines 778-802):**
```rust
fn compute_loop_body_nodes(&self) -> HashSet<String> {
    let mut all_loop_bodies = HashSet::new();
    
    for node in &self.scenario.nodes {
        if matches!(node.activity, Activity::Loop { .. } | Activity::While { .. }) {
            let mut this_loop_body = HashSet::new();
            // Get LoopBody connections and recursively collect
            self.collect_loop_body_recursive(&start, &node.i
