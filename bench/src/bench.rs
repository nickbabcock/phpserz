use criterion::{BenchmarkId, Criterion, Throughput};

fn parser(c: &mut Criterion) {
    let data = include_bytes!("../../assets/corpus/awbw.txt");
    let mut group = c.benchmark_group("parser");
    group.throughput(Throughput::Bytes(data.len() as u64));
    group.bench_function(BenchmarkId::from_parameter("awbw"), |b| {
        b.iter(|| {
            let mut parser = phpserz::PhpParser::new(data.as_slice());
            let mut storage = Vec::new();
            while let Ok(Some(_)) = parser.next_token(&mut storage) {}
        });
    });
    group.finish();
}

criterion::criterion_group!(benches, parser);
criterion::criterion_main!(benches);
