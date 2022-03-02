use crate::error::Error;
use crate::rosetta::{Expr, Function, Matcher, MatcherOp, Projection, Range, Resource};
use common::time::{Instant, EPOCH};
use common::LabelType;
use promql::{LabelMatchOp, Node, Vector};

pub fn parse(q: &str) -> Result<Expr, Error> {
    let ast = promql::parse(q.as_ref(), false).map_err(|err| Error::InternalError {
        err: format!("{:?}", err),
    })?;
    match ast {
        Node::Vector(vector) => translate_vector(vector, vec![]),
        Node::Function { name, args, .. } => match args.into_iter().next().unwrap() {
            Node::Vector(vector) => translate_vector(vector, vec![&name]),
            _ => unimplemented!(),
        },
        _ => unimplemented!(),
    }
}

fn translate_vector(vector: Vector, functions: Vec<&str>) -> Result<Expr, Error> {
    let range = Range {
        start: vector
            .range
            .map(|sec| EPOCH + (Instant::now() - Instant::from_millis(sec as i64 * 1000))),
        end: None,
    };
    let mut name = None;
    let mut filters = Vec::with_capacity(vector.labels.len() - 1);
    for label in vector.labels {
        if label.name == "__name__" {
            name = Some(label.value);
        } else {
            let op = match label.op {
                LabelMatchOp::Eq => MatcherOp::LiteralEqual,
                LabelMatchOp::Ne => MatcherOp::LiteralNotEqual,
                LabelMatchOp::REq => MatcherOp::RegexMatch,
                LabelMatchOp::RNe => MatcherOp::RegexNotMatch,
            };
            filters.push(Matcher {
                name: label.name,
                op,
                value: Some(LabelType::String(label.value)),
            })
        }
    }

    Ok(Expr {
        resource: Resource {
            catalog: None,
            namespace: None,
            resource: name.ok_or(Error::NoName)?,
        },
        filters,
        range,
        projection: vec![Projection::Specific {
            name: String::from("value"),
            pipeline: functions
                .into_iter()
                .map(|name| Function { name: name.into() })
                .collect(),
        }],
    })
}

#[cfg(test)]
mod tests {
    use crate::promql::parse;

    #[test]
    fn it_works() {
        let query = "sum(something_used{env=\"production\"}[5m])";
        let expr = parse(query).unwrap();
        println!("{:?}", expr);
    }
}
