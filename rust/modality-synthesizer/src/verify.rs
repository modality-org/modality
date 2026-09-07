use modality_lang::{Formula, FormulaExpr, ModelChecker};

pub fn formulas_satisfied(model: &modality_lang::Model, formulas: &[FormulaExpr]) -> bool {
    if formulas.is_empty() {
        return false;
    }
    let checker = ModelChecker::new(model.clone());
    formulas.iter().enumerate().all(|(index, expression)| {
        let formula = Formula::new(format!("F{}", index + 1), expression.clone());
        checker.check_formula(&formula).is_satisfied
    })
}
