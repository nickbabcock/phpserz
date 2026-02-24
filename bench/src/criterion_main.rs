use criterion::criterion_main;
use phpserz_bench::benchmarks::{deserializer, parser};

criterion_main!(
    parser::criterion_benches::parser_benches,
    deserializer::criterion_benches::deserializer_benches,
);
