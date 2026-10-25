use crate::curve::Curve;
use crate::expression::Expression;
use crate::file::DriverEntry;
use crate::file::Kind;

/// A driver ready to run: its formula parsed, its curve and range kept.
#[derive(Debug, Clone)]
pub struct Driver {
    kind: Kind,
    expression: Option<Expression>,
    curve: Option<Curve>,
    range: [f32; 2],
}

impl Driver {
    pub fn new(entry: &DriverEntry) -> Result<Self, String> {
        let names: Vec<&str> = entry.variables.iter().map(|v| v.name.as_str()).collect();
        let expression = match entry.kind {
            Kind::Scripted => Some(Expression::parse(&entry.expression, &names)?),
            _ => None,
        };
        Ok(Self { kind: entry.kind, expression, curve: entry.curve.clone(), range: entry.range })
    }

    /// The shape key's value for these variable values, in the entry's order. None when it
    /// isn't a number, as when a variable isn't one.
    pub fn evaluate(&self, variables: &[f32]) -> Option<f32> {
        let values = variables.iter().map(|&v| v as f64);
        let raw = match self.kind {
            // Blender reads a formula with no answer, like a division by zero, as 0.
            Kind::Scripted => {
                let value = self.expression.as_ref()?.evaluate(variables);
                if value.is_finite() { value } else { 0.0 }
            }
            Kind::Sum => values.sum(),
            Kind::Average if variables.is_empty() => 0.0,
            Kind::Average => values.sum::<f64>() / variables.len() as f64,
            Kind::Min => values.reduce(f64::min).unwrap_or(0.0),
            Kind::Max => values.reduce(f64::max).unwrap_or(0.0),
        } as f32;
        let value = self.curve.as_ref().map_or(raw, |curve| curve.sample(raw));
        let [low, high] = self.range;
        value.is_finite().then(|| value.clamp(low.min(high), high.max(low)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::file::Source;
    use crate::file::Variable;

    fn entry(kind: Kind, expression: &str, count: usize, curve: Option<Curve>) -> DriverEntry {
        let variables = (0..count)
            .map(|i| Variable {
                name: format!("v{i}"),
                source: Source::RotationDifference { bones: [String::new(), String::new()] },
            })
            .collect();
        DriverEntry {
            object: "Body".into(),
            key: "Key".into(),
            kind,
            expression: expression.into(),
            variables,
            curve,
            range: [0.0, 1.0],
            rest: None,
        }
    }

    #[test]
    fn kinds() {
        let run = |kind, vars: &[f32]| {
            Driver::new(&entry(kind, "", vars.len(), None)).unwrap().evaluate(vars)
        };
        assert_eq!(run(Kind::Average, &[0.2, 0.4]), Some(0.3));
        assert_eq!(run(Kind::Sum, &[0.2, 0.4]), Some(0.6));
        assert_eq!(run(Kind::Min, &[0.2, 0.4]), Some(0.2));
        assert_eq!(run(Kind::Max, &[0.2, 0.4]), Some(0.4));
        assert_eq!(run(Kind::Average, &[]), Some(0.0));
    }

    #[test]
    fn curve_then_range() {
        let curve = Curve {
            from: 0.0,
            to: 1.0,
            values: vec![0.0, 2.0],
            slope_before: 0.0,
            slope_after: 2.0,
        };
        let driver = Driver::new(&entry(Kind::Scripted, "v0", 1, Some(curve))).unwrap();
        assert_eq!(driver.evaluate(&[0.25]), Some(0.5));
        assert_eq!(driver.evaluate(&[0.9]), Some(1.0));
    }

    #[test]
    fn no_answer_reads_as_zero() {
        let curve = Curve {
            from: 0.0,
            to: 1.0,
            values: vec![0.5, 1.0],
            slope_before: 0.0,
            slope_after: 0.0,
        };
        let driver = Driver::new(&entry(Kind::Scripted, "1 / v0", 1, Some(curve))).unwrap();
        assert_eq!(driver.evaluate(&[0.0]), Some(0.5));
    }

    #[test]
    fn unknown_names_are_refused() {
        assert!(Driver::new(&entry(Kind::Scripted, "d * 2", 1, None)).is_err());
    }
}
