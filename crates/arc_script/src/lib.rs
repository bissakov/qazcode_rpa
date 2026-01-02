mod ast;
mod evaluator;
mod lexer;
mod parser;
mod token;
mod value;
mod variable_type;

pub use ast::Expr;
pub use evaluator::eval_expr;
pub use lexer::Lexer;
pub use parser::parse_expr;
pub use token::Token;
pub use value::{Value, VariableResolver};
pub use variable_type::VariableType;

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    struct MockResolver {
        vars: HashMap<String, Value>,
    }

    impl VariableResolver for MockResolver {
        fn resolve(&self, name: &str) -> Result<Value, String> {
            self.vars
                .get(name)
                .cloned()
                .ok_or_else(|| format!("Undefined variable: {}", name))
        }
    }

    #[test]
    fn test_arithmetic() {
        let resolver = MockResolver {
            vars: HashMap::new(),
        };

        {
            let expression = "2 + 3";
            let expr = parse_expr(expression).unwrap();
            assert_eq!(eval_expr(&expr, &resolver).unwrap(), Value::Number(5.0));
        }

        {
            let expression = "10 - 4";
            let expr = parse_expr(expression).unwrap();
            assert_eq!(eval_expr(&expr, &resolver).unwrap(), Value::Number(6.0));
        }

        {
            let expression = "3 * 4";
            let expr = parse_expr(expression).unwrap();
            assert_eq!(eval_expr(&expr, &resolver).unwrap(), Value::Number(12.0));
        }

        {
            let expression = "15 / 3";
            let expr = parse_expr(expression).unwrap();
            assert_eq!(eval_expr(&expr, &resolver).unwrap(), Value::Number(5.0));
        }

        {
            let expression = "10 % 3";
            let expr = parse_expr(expression).unwrap();
            assert_eq!(eval_expr(&expr, &resolver).unwrap(), Value::Number(1.0));
        }
    }

    #[test]
    fn test_parentheses() {
        let resolver = MockResolver {
            vars: HashMap::new(),
        };

        {
            let expr = parse_expr("(2 + 3) * 4").unwrap();
            assert_eq!(eval_expr(&expr, &resolver).unwrap(), Value::Number(20.0));
        }

        {
            let expr = parse_expr("2 + (3 * 4)").unwrap();
            assert_eq!(eval_expr(&expr, &resolver).unwrap(), Value::Number(14.0));
        }
    }

    #[test]
    fn test_comparison() {
        let resolver = MockResolver {
            vars: HashMap::new(),
        };

        for (expr_str, expected) in [
            ("5 > 3", true),
            ("5 < 3", false),
            ("5 >= 5", true),
            ("5 <= 4", false),
            ("5 == 5", true),
            ("5 != 3", true),
        ] {
            let expr = parse_expr(expr_str).unwrap();
            assert_eq!(
                eval_expr(&expr, &resolver).unwrap(),
                Value::Boolean(expected)
            );
        }
    }

    #[test]
    fn test_boolean() {
        let resolver = MockResolver {
            vars: HashMap::new(),
        };

        for (expr_str, expected) in [
            ("true && true", true),
            ("true && false", false),
            ("true || false", true),
            ("!true", false),
            ("!false", true),
        ] {
            let expr = parse_expr(expr_str).unwrap();
            assert_eq!(
                eval_expr(&expr, &resolver).unwrap(),
                Value::Boolean(expected)
            );
        }
    }

    #[test]
    fn test_boolean_uppercase() {
        let resolver = MockResolver {
            vars: HashMap::new(),
        };

        for (expr_str, expected) in [
            ("true AND true", true),
            ("true OR false", true),
            ("NOT false", true),
        ] {
            let expr = parse_expr(expr_str).unwrap();
            assert_eq!(
                eval_expr(&expr, &resolver).unwrap(),
                Value::Boolean(expected)
            );
        }
    }

    #[test]
    fn test_variables() {
        let mut vars = HashMap::new();
        vars.insert("x".to_string(), Value::Number(10.0));
        vars.insert("y".to_string(), Value::Number(5.0));

        let resolver = MockResolver { vars };

        {
            let expr = parse_expr("@x + @y").unwrap();
            assert_eq!(eval_expr(&expr, &resolver).unwrap(), Value::Number(15.0));
        }

        {
            let expr = parse_expr("@x > @y").unwrap();
            assert_eq!(eval_expr(&expr, &resolver).unwrap(), Value::Boolean(true));
        }
    }

    #[test]
    fn test_complex() {
        let mut vars = HashMap::new();
        vars.insert("a".to_string(), Value::Number(10.0));
        vars.insert("b".to_string(), Value::Number(5.0));

        let resolver = MockResolver { vars };

        let expr = parse_expr("(@a + @b) * 2 > 20").unwrap();
        assert_eq!(eval_expr(&expr, &resolver).unwrap(), Value::Boolean(true));
    }

    #[test]
    fn test_errors() {
        let resolver = MockResolver {
            vars: HashMap::new(),
        };

        assert!(parse_expr("").is_err());
        assert!(parse_expr("2 +").is_err());
        assert!(parse_expr("(2 + 3").is_err());
        assert!(parse_expr("2 + 3)").is_err());

        let expr = parse_expr("10 / 0").unwrap();
        assert!(eval_expr(&expr, &resolver).is_err());

        let expr = parse_expr("@undefined").unwrap();
        assert!(eval_expr(&expr, &resolver).is_err());
    }

    #[test]
    fn test_boolean_strict() {
        for expr_str in ["yes", "and", "or", "not"] {
            assert!(parse_expr(expr_str).is_err());
        }
    }

    #[test]
    fn test_string_literals_double_quotes_only() {
        let resolver = MockResolver {
            vars: HashMap::new(),
        };

        let expr = parse_expr("\"hello\"").unwrap();
        assert_eq!(
            eval_expr(&expr, &resolver).unwrap(),
            Value::String("hello".to_string())
        );
    }

    #[test]
    fn test_string_literals_single_quotes_rejected() {
        assert!(parse_expr("'hello'").is_err());
        assert!(parse_expr("'123'").is_err());
    }

    #[test]
    fn test_string_no_numeric_coercion() {
        let resolver = MockResolver {
            vars: HashMap::new(),
        };

        let expr = parse_expr("\"123\"").unwrap();
        assert_eq!(
            eval_expr(&expr, &resolver).unwrap(),
            Value::String("123".to_string())
        );

        let expr = parse_expr("\"123\" + 1").unwrap();
        assert_eq!(
            eval_expr(&expr, &resolver).unwrap(),
            Value::String("1231".to_string())
        );
    }

    #[test]
    fn test_string_equality_strict() {
        let resolver = MockResolver {
            vars: HashMap::new(),
        };

        let expr = parse_expr("\"abc\" == \"abc\"").unwrap();
        assert_eq!(eval_expr(&expr, &resolver).unwrap(), Value::Boolean(true));

        let expr = parse_expr("\"abc\" != \"def\"").unwrap();
        assert_eq!(eval_expr(&expr, &resolver).unwrap(), Value::Boolean(true));
    }

    #[test]
    fn test_string_number_equality_rejected() {
        let resolver = MockResolver {
            vars: HashMap::new(),
        };

        let expr = parse_expr("\"1\" == 1").unwrap();
        assert!(eval_expr(&expr, &resolver).is_err());

        let expr = parse_expr("1 != \"1\"").unwrap();
        assert!(eval_expr(&expr, &resolver).is_err());
    }

    #[test]
    fn test_string_in_boolean_context_rejected() {
        let resolver = MockResolver {
            vars: HashMap::new(),
        };

        let expr = parse_expr("!\"true\"").unwrap();
        assert!(eval_expr(&expr, &resolver).is_err());

        let expr = parse_expr("\"true\" && true").unwrap();
        assert!(eval_expr(&expr, &resolver).is_err());
    }

    #[test]
    fn test_string_comparisons_rejected() {
        let resolver = MockResolver {
            vars: HashMap::new(),
        };

        for expr_str in [
            "\"a\" > \"b\"",
            "\"a\" < \"b\"",
            "\"a\" >= \"b\"",
            "\"a\" <= \"b\"",
        ] {
            let expr = parse_expr(expr_str).unwrap();
            assert!(eval_expr(&expr, &resolver).is_err());
        }
    }

    #[test]
    fn test_string_concatenation() {
        let resolver = MockResolver {
            vars: HashMap::new(),
        };

        let expr = parse_expr("\"hello\" + \" world\"").unwrap();
        assert_eq!(
            eval_expr(&expr, &resolver).unwrap(),
            Value::String("hello world".to_string())
        );
    }

    #[test]
    fn test_string_plus_number() {
        let resolver = MockResolver {
            vars: HashMap::new(),
        };

        let expr = parse_expr("\"value: \" + 42").unwrap();
        assert_eq!(
            eval_expr(&expr, &resolver).unwrap(),
            Value::String("value: 42".to_string())
        );
    }

    #[test]
    fn test_string_plus_boolean() {
        let resolver = MockResolver {
            vars: HashMap::new(),
        };

        let expr = parse_expr("\"flag: \" + true").unwrap();
        assert_eq!(
            eval_expr(&expr, &resolver).unwrap(),
            Value::String("flag: true".to_string())
        );

        let expr = parse_expr("\"flag: \" + false").unwrap();
        assert_eq!(
            eval_expr(&expr, &resolver).unwrap(),
            Value::String("flag: false".to_string())
        );
    }

    #[test]
    fn test_string_plus_variable() {
        let mut vars = HashMap::new();
        vars.insert("name".to_string(), Value::String("Alice".to_string()));
        let resolver = MockResolver { vars };

        let expr = parse_expr("\"Hello \" + @name").unwrap();
        assert_eq!(
            eval_expr(&expr, &resolver).unwrap(),
            Value::String("Hello Alice".to_string())
        );
    }

    #[test]
    fn test_string_plus_number_variable() {
        let mut vars = HashMap::new();
        vars.insert("count".to_string(), Value::Number(5.0));
        let resolver = MockResolver { vars };

        let expr = parse_expr("\"items: \" + @count").unwrap();
        assert_eq!(
            eval_expr(&expr, &resolver).unwrap(),
            Value::String("items: 5".to_string())
        );
    }

    #[test]
    fn test_string_with_decimal() {
        let resolver = MockResolver {
            vars: HashMap::new(),
        };

        let expr = parse_expr("\"value: \" + 3.14").unwrap();
        assert_eq!(
            eval_expr(&expr, &resolver).unwrap(),
            Value::String("value: 3.14".to_string())
        );
    }

    #[test]
    fn test_number_addition_unchanged() {
        let resolver = MockResolver {
            vars: HashMap::new(),
        };

        let expr = parse_expr("5 + 3").unwrap();
        assert_eq!(eval_expr(&expr, &resolver).unwrap(), Value::Number(8.0));
    }

    #[test]
    fn test_number_plus_boolean_coercion() {
        let resolver = MockResolver {
            vars: HashMap::new(),
        };

        let expr = parse_expr("5 + true").unwrap();
        assert_eq!(eval_expr(&expr, &resolver).unwrap(), Value::Number(6.0));

        let expr = parse_expr("5 + false").unwrap();
        assert_eq!(eval_expr(&expr, &resolver).unwrap(), Value::Number(5.0));
    }

    #[test]
    fn test_chained_string_concatenation() {
        let resolver = MockResolver {
            vars: HashMap::new(),
        };

        let expr = parse_expr("\"a\" + \"b\" + \"c\"").unwrap();
        assert_eq!(
            eval_expr(&expr, &resolver).unwrap(),
            Value::String("abc".to_string())
        );
    }

    #[test]
    fn test_chained_string_with_variables() {
        let mut vars = HashMap::new();
        vars.insert("x".to_string(), Value::Number(10.0));
        let resolver = MockResolver { vars };

        let expr = parse_expr("\"x=\" + @x + \" is the answer\"").unwrap();
        assert_eq!(
            eval_expr(&expr, &resolver).unwrap(),
            Value::String("x=10 is the answer".to_string())
        );
    }

    #[test]
    fn test_number_plus_string_error() {
        let resolver = MockResolver {
            vars: HashMap::new(),
        };

        let expr = parse_expr("5 + \"hello\"").unwrap();
        assert!(eval_expr(&expr, &resolver).is_err());
    }

    #[test]
    fn test_boolean_plus_error() {
        let resolver = MockResolver {
            vars: HashMap::new(),
        };

        let expr = parse_expr("true + 5").unwrap();
        assert!(eval_expr(&expr, &resolver).is_err());

        let expr = parse_expr("false + \"hello\"").unwrap();
        assert!(eval_expr(&expr, &resolver).is_err());
    }

    #[test]
    fn test_string_plus_undefined() {
        let resolver = MockResolver {
            vars: HashMap::new(),
        };

        let expr = parse_expr("\"hello\" + @undefined").unwrap();
        assert!(eval_expr(&expr, &resolver).is_err());
    }

    #[test]
    fn test_interpolation_simple_variable() {
        let mut vars = HashMap::new();
        vars.insert("name".to_string(), Value::String("Alice".to_string()));
        let resolver = MockResolver { vars };

        let expr = parse_expr("\"Hello {@name}\"").unwrap();
        assert_eq!(
            eval_expr(&expr, &resolver).unwrap(),
            Value::String("Hello Alice".to_string())
        );
    }

    #[test]
    fn test_interpolation_number_variable() {
        let mut vars = HashMap::new();
        vars.insert("num".to_string(), Value::Number(42.0));
        let resolver = MockResolver { vars };

        let expr = parse_expr("\"Number: {@num}\"").unwrap();
        assert_eq!(
            eval_expr(&expr, &resolver).unwrap(),
            Value::String("Number: 42".to_string())
        );
    }

    #[test]
    fn test_interpolation_boolean_variable() {
        let mut vars = HashMap::new();
        vars.insert("flag".to_string(), Value::Boolean(true));
        let resolver = MockResolver { vars };

        let expr = parse_expr("\"Flag: {@flag}\"").unwrap();
        assert_eq!(
            eval_expr(&expr, &resolver).unwrap(),
            Value::String("Flag: true".to_string())
        );
    }

    #[test]
    fn test_interpolation_arithmetic_expression() {
        let mut vars = HashMap::new();
        vars.insert("x".to_string(), Value::Number(5.0));
        vars.insert("y".to_string(), Value::Number(3.0));
        let resolver = MockResolver { vars };

        let expr = parse_expr("\"Sum: {@x + @y}\"").unwrap();
        assert_eq!(
            eval_expr(&expr, &resolver).unwrap(),
            Value::String("Sum: 8".to_string())
        );
    }

    #[test]
    fn test_interpolation_multiplication_expression() {
        let mut vars = HashMap::new();
        vars.insert("x".to_string(), Value::Number(4.0));
        vars.insert("y".to_string(), Value::Number(5.0));
        let resolver = MockResolver { vars };

        let expr = parse_expr("\"Product: {@x * @y}\"").unwrap();
        assert_eq!(
            eval_expr(&expr, &resolver).unwrap(),
            Value::String("Product: 20".to_string())
        );
    }

    #[test]
    fn test_interpolation_comparison_expression() {
        let mut vars = HashMap::new();
        vars.insert("x".to_string(), Value::Number(10.0));
        vars.insert("y".to_string(), Value::Number(5.0));
        let resolver = MockResolver { vars };

        let expr = parse_expr("\"Comparison: {@x > @y}\"").unwrap();
        assert_eq!(
            eval_expr(&expr, &resolver).unwrap(),
            Value::String("Comparison: true".to_string())
        );
    }

    #[test]
    fn test_interpolation_complex_expression() {
        let mut vars = HashMap::new();
        vars.insert("x".to_string(), Value::Number(5.0));
        vars.insert("y".to_string(), Value::Number(3.0));
        let resolver = MockResolver { vars };

        let expr = parse_expr("\"Result: {(@x + @y) * 2}\"").unwrap();
        assert_eq!(
            eval_expr(&expr, &resolver).unwrap(),
            Value::String("Result: 16".to_string())
        );
    }

    #[test]
    fn test_interpolation_multiple_variables() {
        let mut vars = HashMap::new();
        vars.insert("first".to_string(), Value::String("John".to_string()));
        vars.insert("last".to_string(), Value::String("Doe".to_string()));
        let resolver = MockResolver { vars };

        let expr = parse_expr("\"{@first} and {@last}\"").unwrap();
        assert_eq!(
            eval_expr(&expr, &resolver).unwrap(),
            Value::String("John and Doe".to_string())
        );
    }

    #[test]
    fn test_interpolation_multiple_segments() {
        let mut vars = HashMap::new();
        vars.insert("a".to_string(), Value::Number(1.0));
        vars.insert("b".to_string(), Value::Number(2.0));
        vars.insert("c".to_string(), Value::Number(3.0));
        let resolver = MockResolver { vars };

        let expr = parse_expr("\"{@a}-{@b}-{@c}\"").unwrap();
        assert_eq!(
            eval_expr(&expr, &resolver).unwrap(),
            Value::String("1-2-3".to_string())
        );
    }

    #[test]
    fn test_interpolation_escape_opening_brace() {
        let resolver = MockResolver {
            vars: HashMap::new(),
        };

        let expr = parse_expr("\"Use {{}} for braces\"").unwrap();
        assert_eq!(
            eval_expr(&expr, &resolver).unwrap(),
            Value::String("Use {} for braces".to_string())
        );
    }

    #[test]
    fn test_interpolation_escape_with_variable() {
        let mut vars = HashMap::new();
        vars.insert("name".to_string(), Value::String("Alice".to_string()));
        let resolver = MockResolver { vars };

        let expr = parse_expr("\"{{@name}}\"").unwrap();
        assert_eq!(
            eval_expr(&expr, &resolver).unwrap(),
            Value::String("{@name}".to_string())
        );
    }

    #[test]
    fn test_interpolation_multiple_escape_braces() {
        let resolver = MockResolver {
            vars: HashMap::new(),
        };

        let expr = parse_expr("\"{{{{\"").unwrap();
        assert_eq!(
            eval_expr(&expr, &resolver).unwrap(),
            Value::String("{{".to_string())
        );
    }

    #[test]
    fn test_interpolation_whitespace_around_expression() {
        let mut vars = HashMap::new();
        vars.insert("x".to_string(), Value::Number(5.0));
        vars.insert("y".to_string(), Value::Number(3.0));
        let resolver = MockResolver { vars };

        let expr = parse_expr("\"Result: { @x + @y }\"").unwrap();
        assert_eq!(
            eval_expr(&expr, &resolver).unwrap(),
            Value::String("Result: 8".to_string())
        );
    }

    #[test]
    fn test_interpolation_whitespace_both_sides() {
        let mut vars = HashMap::new();
        vars.insert("x".to_string(), Value::Number(5.0));
        vars.insert("y".to_string(), Value::Number(3.0));
        let resolver = MockResolver { vars };

        let expr = parse_expr("\"{ @x }{ @y }\"").unwrap();
        assert_eq!(
            eval_expr(&expr, &resolver).unwrap(),
            Value::String("53".to_string())
        );
    }

    #[test]
    fn test_interpolation_error_undefined_variable() {
        let resolver = MockResolver {
            vars: HashMap::new(),
        };

        let expr = parse_expr("\"Hello {@undefined}\"").unwrap();
        assert!(eval_expr(&expr, &resolver).is_err());
    }

    #[test]
    fn test_interpolation_error_unclosed_brace() {
        let expr = parse_expr("\"Hello {@x");
        assert!(expr.is_err());
    }

    #[test]
    fn test_interpolation_error_empty_expression() {
        let expr = parse_expr("\"Hello {}\"");
        assert!(expr.is_err());
    }

    #[test]
    fn test_interpolation_no_interpolation_plain_string() {
        let resolver = MockResolver {
            vars: HashMap::new(),
        };

        let expr = parse_expr("\"Plain string\"").unwrap();
        assert_eq!(
            eval_expr(&expr, &resolver).unwrap(),
            Value::String("Plain string".to_string())
        );
    }

    #[test]
    fn test_interpolation_decimal_number() {
        let mut vars = HashMap::new();
        vars.insert("pi".to_string(), Value::Number(3.14));
        let resolver = MockResolver { vars };

        let expr = parse_expr("\"Pi is approximately {@pi}\"").unwrap();
        assert_eq!(
            eval_expr(&expr, &resolver).unwrap(),
            Value::String("Pi is approximately 3.14".to_string())
        );
    }
}
