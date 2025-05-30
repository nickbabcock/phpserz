use std::hint::black_box;
use criterion::{BenchmarkId, Criterion, Throughput};

fn parser(c: &mut Criterion) {
    let awbw = include_bytes!("../../assets/corpus/awbw.txt");
    let mut group = c.benchmark_group("parser");
    group.throughput(Throughput::Bytes(awbw.len() as u64));
    group.bench_function(BenchmarkId::from_parameter("awbw"), |b| {
        b.iter(|| {
            let mut parser = phpserz::PhpParser::new(awbw.as_slice());
            let mut count = 0;
            while let Ok(Some(_)) = parser.next_token() {
                count += 1;
            }
            black_box(count);
        });
    });

    let sensors = include_bytes!("../../assets/corpus/sensors.txt");
    group.throughput(Throughput::Bytes(sensors.len() as u64));
    group.bench_function(BenchmarkId::from_parameter("sensors"), |b| {
        b.iter(|| {
            let mut parser = phpserz::PhpParser::new(sensors.as_slice());
            let mut count = 0;
            while let Ok(Some(_)) = parser.next_token() {
                count += 1;
            }
            black_box(count);
        });
    });
    group.finish();
}

criterion::criterion_group!(benches, parser);
criterion::criterion_main!(benches);
