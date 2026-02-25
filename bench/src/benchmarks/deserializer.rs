use serde::Deserialize;
use std::collections::BTreeMap;

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct GamePayload {
    id: i64,
    name: String,
    password: Option<String>,
    creator: i64,
    start_date: String,
    end_date: Option<String>,
    activity_date: String,
    maps_id: i64,
    weather_type: String,
    weather_start: Option<i64>,
    weather_code: String,
    win_condition: Option<String>,
    turn: i64,
    day: i64,
    active: String,
    funds: i64,
    capture_win: i64,
    fog: String,
    comment: Option<String>,
    #[serde(rename = "type")]
    game_type: String,
    boot_interval: i64,
    starting_funds: i64,
    official: String,
    min_rating: i64,
    max_rating: Option<i64>,
    league: Option<String>,
    team: String,
    aet_interval: i64,
    aet_date: String,
    use_powers: String,
    players: BTreeMap<u32, PlayerPayload>,
    buildings: BTreeMap<u32, BuildingPayload>,
    units: BTreeMap<u32, UnitPayload>,
    timers_initial: Option<i64>,
    timers_increment: i64,
    timers_max_turn: i64,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct PlayerPayload {
    id: i64,
    users_id: i64,
    games_id: i64,
    countries_id: i64,
    co_id: i64,
    funds: i64,
    turn: Option<String>,
    email: Option<String>,
    uniq_id: Option<String>,
    eliminated: String,
    last_read: String,
    last_read_broadcasts: Option<String>,
    emailpress: Option<String>,
    signature: Option<String>,
    co_power: i64,
    co_power_on: String,
    order: i64,
    accept_draw: String,
    co_max_power: i64,
    co_max_spower: i64,
    co_image: Option<String>,
    team: String,
    aet_count: i64,
    turn_start: String,
    turn_clock: i64,
    tags_co_id: Option<i64>,
    tags_co_power: Option<i64>,
    tags_co_max_power: Option<i64>,
    tags_co_max_spower: Option<i64>,
    interface: String,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct BuildingPayload {
    id: i64,
    games_id: i64,
    terrain_id: i64,
    x: i64,
    y: i64,
    capture: i64,
    last_capture: i64,
    last_updated: String,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct UnitPayload {
    id: i64,
    games_id: i64,
    players_id: i64,
    name: String,
    movement_points: i64,
    vision: i64,
    fuel: i64,
    fuel_per_turn: i64,
    sub_dive: String,
    ammo: i64,
    short_range: i64,
    long_range: i64,
    second_weapon: String,
    symbol: String,
    cost: i64,
    movement_type: String,
    x: i64,
    y: i64,
    moved: i64,
    capture: i64,
    fired: i64,
    hit_points: f64,
    cargo1_units_id: i64,
    cargo2_units_id: i64,
    carried: String,
}

pub mod criterion_benches {
    use super::*;
    use criterion::{BenchmarkId, Criterion, Throughput};
    use std::hint::black_box;

    fn deserializer(c: &mut Criterion) {
        let awbw = include_bytes!("../../../assets/corpus/awbw.txt");
        let mut group = c.benchmark_group("deserializer");
        group.throughput(Throughput::Bytes(awbw.len() as u64));
        group.bench_function(BenchmarkId::from_parameter("game-awbw"), |b| {
            b.iter(|| {
                let mut deserializer = phpserz::PhpDeserializer::new(awbw.as_slice());
                let game: GamePayload = Deserialize::deserialize(&mut deserializer)
                    .expect("to deserialize game payload");
                black_box(game);
            });
        });
        group.finish();
    }

    criterion::criterion_group!(deserializer_benches, deserializer);
}

#[cfg(not(target_family = "wasm"))]
pub mod gungraun_benches {
    use super::*;
    use gungraun::{library_benchmark, library_benchmark_group};

    #[library_benchmark]
    #[bench::game_awbw(include_bytes!("../../../assets/corpus/awbw.txt").as_slice())]
    fn deserialize_game(data: &[u8]) -> GamePayload {
        let mut deserializer = phpserz::PhpDeserializer::new(data);
        Deserialize::deserialize(&mut deserializer).expect("to deserialize game payload")
    }

    library_benchmark_group!(name = deserializer_benches, benchmarks = [deserialize_game,]);
}
