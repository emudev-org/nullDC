use criterion::{criterion_group, criterion_main, Criterion, Throughput};

#[path = "../src/core.rs"]
mod core;

fn bench_tls_vs_param(c: &mut Criterion) {
    let mut g = c.benchmark_group("tls-vs-param");
    let n = 10_000;
    g.throughput(Throughput::Elements(n));

    unsafe {
        // allocate/bind once if you have such helper
        // core::ctx_alloc_bind(core::Ctx{ a:1, b:2, c:3 });
    }

    g.bench_function("param_sum", |b| {
        b.iter(|| unsafe {
            // call core::param_sum(...)
        })
    });

    g.finish();
}

criterion_group!(benches, bench_tls_vs_param);
criterion_main!(benches);
