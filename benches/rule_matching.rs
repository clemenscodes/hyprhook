use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use hyprhook::RuleSet;

fn build_set(rule_count: usize) -> RuleSet {
    let rules = (0..rule_count)
        .map(|index| {
            hyprhook::Rule::new(
                Some(&format!("^app-{index}$")),
                Some(&format!("^Window {index}$")),
                vec![],
                vec![],
                vec![format!("/usr/bin/cmd-{index}")],
                vec![],
            )
        })
        .collect();
    RuleSet::new(rules).unwrap()
}

fn bench_matching_no_match(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("matching/no_match");
    for rule_count in [10, 100, 1_000, 10_000] {
        let set = build_set(rule_count);
        group.bench_with_input(
            BenchmarkId::from_parameter(rule_count),
            &rule_count,
            |bencher, _| bencher.iter(|| set.matching("nonexistent", "nonexistent")),
        );
    }
    group.finish();
}

fn bench_matching_last_rule_matches(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("matching/last_rule_matches");
    for rule_count in [10, 100, 1_000, 10_000] {
        let set = build_set(rule_count);
        let last = rule_count - 1;
        let class = format!("app-{last}");
        let title = format!("Window {last}");
        group.bench_with_input(
            BenchmarkId::from_parameter(rule_count),
            &rule_count,
            |bencher, _| bencher.iter(|| set.matching(&class, &title)),
        );
    }
    group.finish();
}

fn bench_construction(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("construction");
    for rule_count in [10, 100, 1_000, 10_000] {
        group.bench_with_input(
            BenchmarkId::from_parameter(rule_count),
            &rule_count,
            |bencher, &n| bencher.iter(|| build_set(n)),
        );
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_matching_no_match,
    bench_matching_last_rule_matches,
    bench_construction,
);
criterion_main!(benches);
