pub mod criterion_benches {
    use criterion::{BenchmarkId, Criterion, Throughput};
    use std::hint::black_box;

    fn parser(c: &mut Criterion) {
        let awbw = include_bytes!("../../../assets/corpus/awbw.txt");
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

        let sensors = include_bytes!("../../../assets/corpus/sensors.txt");
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

    criterion::criterion_group!(parser_benches, parser);
}

#[cfg(not(target_family = "wasm"))]
pub mod gungraun_benches {
    use gungraun::{library_benchmark, library_benchmark_group};

    #[library_benchmark]
    #[bench::awbw(include_bytes!("../../../assets/corpus/awbw.txt").as_slice())]
    #[bench::sensors(include_bytes!("../../../assets/corpus/sensors.txt").as_slice())]
    fn parse_tokens(data: &[u8]) -> usize {
        let mut parser = phpserz::PhpParser::new(data);
        let mut count = 0;
        while let Ok(Some(_)) = parser.next_token() {
            count += 1;
        }
        count
    }

    library_benchmark_group!(name = parser_benches, benchmarks = [parse_tokens,]);
}
