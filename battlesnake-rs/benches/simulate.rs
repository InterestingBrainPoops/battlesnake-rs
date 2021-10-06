use battlesnake_game_types::compact_representation::CellBoard4Snakes11x11;
use battlesnake_game_types::types::*;
use battlesnake_game_types::wire_representation::Game;

use battlesnake_rs::devious_devin::Instruments;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use pprof::criterion::{Output, PProfProfiler};

pub fn criterion_benchmark(c: &mut Criterion) {
    let mut g = c.benchmark_group("simulate");
    g.bench_function("compact start_of_game", |b| {
        let game_json = include_str!("../fixtures/start_of_game.json");
        let game: Game = serde_json::from_str(game_json).unwrap();

        let id_map = build_snake_id_map(&game);
        let game = CellBoard4Snakes11x11::convert_from_game(game, &id_map).unwrap();

        b.iter(|| {
            let game = black_box(&game);
            game.simulate(&Instruments, game.get_snake_ids())
        })
    });

    g.bench_function("compact a-prime-food-maze", |b| {
        let game_json = include_str!("../fixtures/a-prime-food-maze.json");
        let game: Game = serde_json::from_str(game_json).unwrap();

        let id_map = build_snake_id_map(&game);
        let game = CellBoard4Snakes11x11::convert_from_game(game, &id_map).unwrap();

        b.iter(|| {
            let game = black_box(&game);
            game.simulate(&Instruments, game.get_snake_ids())
        })
    });
}

criterion_group! {
    name = benches;
    config = Criterion::default().with_profiler(PProfProfiler::new(100, Output::Flamegraph(None)));
    targets = criterion_benchmark
}
criterion_main!(benches);
