use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn promql_parse(c: &mut Criterion) {
    let query = b"rate(something_used{env=~\"production\"}[5m])";
    c.bench_function("promql", |b| {
        b.iter(|| promql::parse(black_box(query), false).unwrap())
    });
}

fn translate(c: &mut Criterion) {
    let query = "rate(something_used{env=~\"production\"}[5m])";
    c.bench_function("translate", |b| {
        b.iter(|| ql::promql::parse(black_box(query)).unwrap())
    });
}

criterion_group!(benches, promql_parse, translate);
criterion_main!(benches);
