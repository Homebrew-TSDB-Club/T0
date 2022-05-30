use crate::error::Error;
use crate::rosetta::{
    AggregateAction, Aggregation, Expr, Function, MatchOp, Matcher, Pipeline, Projection, Range,
    Resource,
};
use common::time::{Instant, EPOCH};
use common::LabelType;
use promql::{AggregationAction, LabelMatchOp, Node, Vector};

pub fn parse(q: &str) -> Result<Expr, Error> {
    let ast = promql::parse(q.as_ref(), false).map_err(|err| Error::InternalError {
        err: format!("{:?}", err),
    })?;
    match ast {
        Node::Vector(vector) => translate_vector(vector, None, None),
        Node::Function {
            name,
            args,
            aggregation,
        } => match args.into_iter().next().unwrap() {
            Node::Vector(vector) => {
                let aggregation = aggregation.map(|a| {
                    let action = match a.action {
                        AggregationAction::Without => AggregateAction::Without,
                        AggregationAction::By => AggregateAction::With,
                    };
                    Aggregation {
                        action,
                        labels: a.labels,
                    }
                });
                translate_vector(vector, Some(name), aggregation)
            }
            _ => {
                unimplemented!()
            }
        },
        _ => unimplemented!(),
    }
}

fn translate_vector(
    vector: Vector,
    function: Option<String>,
    aggregation: Option<Aggregation>,
) -> Result<Expr, Error> {
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
                LabelMatchOp::Eq => MatchOp::LiteralEqual,
                LabelMatchOp::Ne => MatchOp::LiteralNotEqual,
                LabelMatchOp::REq => MatchOp::RegexMatch,
                LabelMatchOp::RNe => MatchOp::RegexNotMatch,
            };
            filters.push(Matcher {
                name: label.name,
                op,
                value: Some(LabelType::String(label.value)),
            })
        }
    }

    let functions = function.map_or_else(Vec::new, |name| vec![Function { name }]);

    Ok(Expr {
        resource: Resource {
            catalog: None,
            namespace: None,
            resource: name.ok_or(Error::NoName)?,
        },
        filters,
        range,
        projection: vec![Projection {
            name: String::from("value"),
            pipeline: Pipeline {
                functions,
                breaker: None,
            },
        }],
        aggregation,
    })
}

#[cfg(test)]
mod tests {
    use crate::promql::parse;

    #[test]
    fn it_works() {
        let query = "sum by (test) (something_used{env=\"production\"}[5m])";
        let expr = parse(query).unwrap();
        println!("{:?}", expr);
    }
}
