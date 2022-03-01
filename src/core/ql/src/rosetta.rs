use common::time::Instant;
use common::LabelType;

#[derive(Debug)]
pub struct Resource {
    pub catalog: Option<String>,
    pub namespace: Option<String>,
    pub resource: String,
}

#[derive(Debug, Clone)]
pub struct Function {
    pub name: String,
}

#[derive(Debug)]
pub enum Projection {
    Specific {
        name: String,
        pipeline: Vec<Function>,
    },
    Rest,
}

#[derive(Debug, Copy, Clone)]
pub struct Range {
    pub start: Option<Instant>,
    pub end: Option<Instant>,
}

#[derive(Debug)]
pub struct Expr<'a> {
    pub resource: Resource,
    pub filters: Vec<Matcher<'a>>,
    pub range: Range,
    pub projection: Vec<Projection>,
}

#[derive(Debug)]
pub struct Matcher<'a> {
    pub name: &'a str,
    pub op: MatcherOp,
    pub value: Option<LabelType<&'a str>>,
}

#[derive(Debug, Copy, Clone)]
pub enum MatcherOp {
    LiteralEqual,
    LiteralNotEqual,
    RegexMatch,
    RegexNotMatch,
}
