pub struct CoreConstants;

impl CoreConstants {
    pub const EXECUTION_COMPLETE_MARKER: &'static str = "__EXECUTION_COMPLETE__";

    pub const VARIABLE_PLACEHOLDER_OPEN: char = '{';
    pub const VARIABLE_PLACEHOLDER_CLOSE: char = '}';
    pub const VARIABLE_SIGIL: char = '@';

    pub const DEFAULT_LOG_ENTRIES: usize = 100;
    pub const MAX_LOG_ENTRIES: usize = 10_000;

    pub const NANOID_LENGTH: usize = 10;

    pub const IR_COMPILATION_MAX_DEPTH: usize = 1000;

    pub const MAX_CALL_STACK_DEPTH: usize = 100;
    pub const MAX_RECURSION_DEPTH: usize = 100;

    pub const ERROR_VARIABLE_NAME: &'static str = "last_error";
    pub const TIMESTAMP_FORMAT_MINUTES: u64 = 60;
}

pub struct ActivityCategories;

impl ActivityCategories {
    pub const SCENARIOS: &'static str = "activity_groups.scenarios";
    pub const BASIC: &'static str = "activity_groups.basic";
    pub const CONTROL_FLOW: &'static str = "activity_groups.control_flow";
    pub const SCRIPTING: &'static str = "activity_groups.scripting";
    pub const DOCUMENTATION: &'static str = "activity_groups.documentation";
}

pub struct ActivityDefaults;

impl ActivityDefaults {
    pub const LOG_MESSAGE: &'static str = "default_values.log_message";
    pub const DELAY_MS: u64 = 1000;
    pub const VARIABLE_NAME: &'static str = "default_values.variable_name";
    pub const VARIABLE_VALUE: &'static str = "default_values.variable_value";
    pub const CONDITION_EXAMPLE: &'static str = "default_values.condition_example";
    pub const LOOP_START: i64 = 0;
    pub const LOOP_END: i64 = 10;
    pub const LOOP_STEP: i64 = 1;
    pub const LOOP_INDEX: &'static str = "i";
    pub const POWERSHELL_CODE: &'static str = "";
    pub const NOTE_TEXT: &'static str = "default_values.note_text";
}

pub struct ValidationConstants;

impl ValidationConstants {
    pub const COMPARISON_OPERATORS: &'static [&'static str] = &["==", "!=", ">=", "<=", ">", "<"];
}
