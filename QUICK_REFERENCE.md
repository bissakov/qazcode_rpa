# Quick Reference: If Node Recursion Issues

## Critical Findings at a Glance

### Location: `crates/rpa-core/src/ir.rs` Lines 790-807

#### Problem 1: Wrong Instruction (Line 796)
```rust
// WRONG - uses JumpIf
self.program.add_instruction(Instruction::JumpIf {
    condition: expr,
    target: 0,
});

// SHOULD BE - uses JumpIfNot (like main scenario line 472)
self.program.add_instruction(Instruction::JumpIfNot {
    condition: expr,
    target: 0,
});
```

#### Problem 2: Reversed Branch Order (Lines 801-806)
```rust
// WRONG ORDER - false first, true second
if let Some(false_id) = false_next {
    self.compile_from_called_scenario(scenario, &false_id)?;
}
if let Some(true_id) = true_next {
    self.compile_from_called_scenario(scenario, &true_id)?;
}

// CORRECT ORDER - true first, false second (like main scenario lines 477-493)
if let Some(node) = true_target {
    self.compile_from_node(&node)?;
}
// ... add skip jump ...
if let Some(node) = false_target {
    self.compile_from_node(&node)?;
}
```

#### Problem 3: Missing Skip Instruction & Patching
```rust
// Main scenario (correct) has:
let jump_over_false_idx = if false_target.is_some() {
    Some(self.program.add_instruction(Instruction::Jump { target: 0 }))
} else {
    None
};
// ... plus backpatching ...

// Nested scenario (broken): MISSING ENTIRELY
```

### Other Missing Control Structures

| Structure | Main Scenario | Nested Scenario | Status |
|-----------|---------------|-----------------|--------|
| While     | ✅ compile_while_node() | ❌ Default handler | Missing |
| TryCatch  | ✅ compile_try_catch_node() | ❌ Default handler | Missing |
| Loop      | ✅ compile_loop_node() | ❌ Default handler | Missing |

### Execution-Time Protection

**Depth Limit Exists** (execution.rs:532-533):
```rust
if self.call_stack.len() >= 100 {
    return Err("Maximum scenario call depth exceeded (100)".to_string());
}
```

- Only applies to **CallScenario instruction execution**
- Does **NOT** limit compilation recursion
- Does **NOT** fix If node issues
- Only prevents infinite scenario calls at runtime

### Shared Node Tracking Issue

File: `ir.rs` Lines 150, 352, 732, 742

```rust
struct IrBuilder {
    compiled_nodes: HashSet<String>,  // Global for ALL scenarios
}

// Main scenario checks/inserts (line 352, 363)
if self.compiled_nodes.contains(node_id) { ... }
self.compiled_nodes.insert(node_id.to_string());

// Nested scenario also checks/inserts SAME map (line 732, 742)
if self.compiled_nodes.contains(node_id) { ... }
self.compiled_nodes.insert(node_id.to_string());

// Result: If main and nested both have node "if_1", 
// the nested one is never compiled (marked already done)
```

## Impact Summary

### If Nodes in Nested Scenarios
- ❌ Condition logic INVERTED (JumpIf vs JumpIfNot)
- ❌ Both branches execute regardless of condition
- ❌ Jump target never patched (remains 0 = undefined behavior)
- ❌ Can cause infinite loops at execution time

### Call Depth Safety
- ✅ Runtime limit: 100 scenario calls (CallScenario instruction)
- ❌ Compilation limit: None (only stack limit)
- ❌ Node collision risk in shared tracking

### Other Control Flow
- ❌ While nodes in nested scenarios don't execute
- ❌ TryCatch in nested scenarios doesn't work
- ❌ Loop in nested scenarios doesn't work

## Related Documentation

1. **IF_NODE_RECURSION_FINDINGS.md** - Full technical analysis with code snippets
2. **DETAILED_FINDINGS_SUMMARY.txt** - Comprehensive findings summary
3. **LOOP_IMPLEMENTATION_FINDINGS.md** - Loop/While/TryCatch analysis
4. **SEARCH_FINDINGS.md** - Initial search findings

## Test Cases to Verify

1. **Simple If in Nested Scenario**
   - Main: Start → CallScenario(Sub) → End
   - Sub: Start → If(true) → (T) Log "true" (F) Log "false" → End
   - **Expected**: Should log "true" if condition is true
   - **Actual**: Likely logs "false" or undefined behavior

2. **If with Cycle Risk**
   - Main → ScenarioA → (If-True) → ScenarioB → ScenarioA
   - **Expected**: Hits 100-call limit, error returned
   - **Actual**: Compilation may infinite loop (no explicit depth limit)

3. **Shared Node IDs**
   - Main has: Start → If → End (node "if_1")
   - Sub has: Start → If → End (also node "if_1")
   - **Expected**: Both compiled
   - **Actual**: Second one skipped (marked already compiled)
