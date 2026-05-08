use crate::error::SigilError;
use rhai::{AST, Dynamic, Engine, Scope as RhaiScope};

#[derive(Debug, Clone)]
pub struct CompiledExpr {
    pub source: ExprSource,
    pub ast: AST,
}

#[derive(Debug, Clone)]
pub struct ExprSource {
    pub expr: String,
    pub line: usize,
    pub col: usize,
}

impl CompiledExpr {
    pub fn eval(&self, vars: &[(String, f64)]) -> Result<f64, SigilError> {
        let engine = Engine::new();
        let mut scope = RhaiScope::new();
        for (name, value) in vars {
            scope.push_constant(name.as_str(), *value);
        }
        let result: Dynamic = engine
            .eval_ast_with_scope(&mut scope, &self.ast)
            .map_err(|err| SigilError::render(err.to_string()))?;
        result
            .as_float()
            .map_err(|_| SigilError::render("expression did not evaluate to number"))
    }
}

pub fn compile_expr(expr: &str, line: usize, col: usize) -> Result<CompiledExpr, SigilError> {
    let engine = Engine::new();
    let ast = engine
        .compile_expression(expr)
        .map_err(|err| SigilError::construction(err.to_string(), line, col))?;
    Ok(CompiledExpr {
        source: ExprSource {
            expr: expr.to_string(),
            line,
            col,
        },
        ast,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn malformed_expression_loud_at_construct() {
        let err = compile_expr("sqrt(urgency +", 1, 1).unwrap_err();
        matches!(err, SigilError::Construction { .. });
    }

    #[test]
    fn missing_attribute_on_one_node_graceful() {
        let compiled = compile_expr("sqrt(urgency) + 1", 1, 1).unwrap();
        let err = compiled.eval(&[("other".into(), 1.0)]).unwrap_err();
        matches!(err, SigilError::Render { .. });
    }
}
