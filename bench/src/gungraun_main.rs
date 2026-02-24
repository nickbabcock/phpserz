use phpserz_bench::benchmarks::{
    deserializer::gungraun_benches::deserializer_benches,
    parser::gungraun_benches::parser_benches,
};

gungraun::main!(library_benchmark_groups = parser_benches, deserializer_benches);
