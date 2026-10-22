use std::collections::BTreeSet;

use crate::QueryError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetOperator {
    Union,
    Intersect,
    Subtract,
}

pub fn evaluate_set_operation(
    operator: SetOperator,
    operands: &[BTreeSet<String>],
) -> Result<BTreeSet<String>, QueryError> {
    let result = match operator {
        SetOperator::Union => operands.iter().flat_map(|set| set.iter().cloned()).collect(),
        SetOperator::Intersect => operands
            .split_first()
            .map(|(first, rest)| {
                first
                    .iter()
                    .filter(|key| rest.iter().all(|set| set.contains(*key)))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default(),
        SetOperator::Subtract => operands
            .split_first()
            .map(|(first, rest)| {
                first
                    .iter()
                    .filter(|key| rest.iter().all(|set| !set.contains(*key)))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default(),
    };
    Ok(result)
}
