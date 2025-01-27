use battlesnake_game_types::compact_representation::{CellBoard, CellIndex, CellNum};
use battlesnake_game_types::types::*;
use battlesnake_game_types::wire_representation::{Game, Position};

use rustc_hash::FxHashMap;
use std::cmp::Ordering;
use std::collections::BinaryHeap;

const NEIGHBOR_DISTANCE: i32 = 1;
const HAZARD_PENALTY: i32 = 1;
const HEURISTIC_MAX: i32 = 500;

pub struct APrimeResult<T> {
    best_cost: i32,
    paths_from: FxHashMap<T, Option<T>>,
    best_target: T,
}

pub struct APrimeOptions {
    pub food_penalty: i32,
}

pub trait APrimeNextDirection: APrimeCalculable {
    fn shortest_path_next_direction(
        &self,
        start: &Self::NativePositionType,
        targets: &[Self::NativePositionType],
        options: Option<APrimeOptions>,
    ) -> Option<Move>;
}

impl APrimeNextDirection for Game {
    fn shortest_path_next_direction(
        &self,
        start: &Self::NativePositionType,
        targets: &[Self::NativePositionType],
        options: Option<APrimeOptions>,
    ) -> Option<Move> {
        let shortest_path = self.shortest_path(start, targets, options);
        let next_coordinate = shortest_path.get(1);
        let start_vec = start.to_vector();

        next_coordinate.map(|c| Move::from_vector(c.sub_vec(start_vec).to_vector()))
    }
}

pub trait APrimeCalculable: PositionGettableGame + NeighborDeterminableGame {
    fn shortest_distance(
        &self,
        start: &Self::NativePositionType,
        targets: &[Self::NativePositionType],
        options: Option<APrimeOptions>,
    ) -> Option<i32> {
        self.a_prime_inner(start, targets, options)
            .map(|r| r.best_cost)
    }

    fn shortest_path(
        &self,
        start: &Self::NativePositionType,
        targets: &[Self::NativePositionType],
        options: Option<APrimeOptions>,
    ) -> Vec<Self::NativePositionType> {
        let result = self.a_prime_inner(start, targets, options);

        let mut path = vec![];

        if let Some(result) = result {
            let mut paths_from = result.paths_from;
            let mut current: Option<Self::NativePositionType> = Some(result.best_target);

            while let Some(c) = current {
                current = paths_from.remove(&c).expect(
                    "Somehow we didn't look at this node during a-prime, but its still in the path",
                );

                path.push(c);
            }
        }

        path.reverse();

        path
    }

    fn a_prime_inner(
        &self,
        start: &Self::NativePositionType,
        targets: &[Self::NativePositionType],
        options: Option<APrimeOptions>,
    ) -> Option<APrimeResult<Self::NativePositionType>>;
}

// The priority queue depends on `Ord`.
// Explicitly implement the trait so the queue becomes a min-heap
// instead of a max-heap.
impl<T: Eq + Ord> Ord for Node<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        // Notice that the we flip the ordering on costs.
        // In case of a tie we compare positions - this step is necessary
        // to make implementations of `PartialEq` and `Ord` consistent.
        other
            .cost
            .cmp(&self.cost)
            .then_with(|| self.coordinate.cmp(&other.coordinate))
    }
}

// `PartialOrd` needs to be implemented as well.
impl<T: Eq + Ord> PartialOrd for Node<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Copy, Clone, Eq, PartialEq)]
struct Node<T> {
    cost: i32,
    coordinate: T,
}

impl<T: CellNum, const BOARD_SIZE: usize, const MAX_SNAKES: usize> APrimeCalculable
    for CellBoard<T, BOARD_SIZE, MAX_SNAKES>
{
    fn a_prime_inner(
        &self,
        start: &Self::NativePositionType,
        targets: &[Self::NativePositionType],
        options: Option<APrimeOptions>,
    ) -> Option<APrimeResult<Self::NativePositionType>> {
        let options = options.unwrap_or(APrimeOptions { food_penalty: 0 });
        let mut paths_from: FxHashMap<Self::NativePositionType, Option<Self::NativePositionType>> =
            FxHashMap::default();

        if targets.is_empty() {
            return None;
        }

        let mut to_search: BinaryHeap<Node<Self::NativePositionType>> = BinaryHeap::new();

        let mut known_score: FxHashMap<Self::NativePositionType, i32> = FxHashMap::default();

        to_search.push(Node {
            cost: 0,
            coordinate: *start,
        });
        known_score.insert(*start, 0);
        paths_from.insert(*start, None);

        while let Some(Node { cost, coordinate }) = to_search.pop() {
            if targets.contains(&coordinate) {
                return Some(APrimeResult {
                    best_cost: cost,
                    paths_from,
                    best_target: coordinate,
                });
            }

            let neighbor_distance = if self.cell_is_hazard(coordinate) {
                HAZARD_PENALTY + NEIGHBOR_DISTANCE
            } else if self.cell_is_food(coordinate) {
                NEIGHBOR_DISTANCE + options.food_penalty
            } else {
                NEIGHBOR_DISTANCE
            };

            let tentative = known_score.get(&coordinate).unwrap_or(&i32::MAX) + neighbor_distance;
            let neighbors = self.neighbors(&coordinate);
            for neighbor in neighbors
                .into_iter()
                .filter(|n| targets.contains(n) || !self.cell_is_snake_body_piece(coordinate))
            {
                if &tentative < known_score.get(&neighbor).unwrap_or(&i32::MAX) {
                    known_score.insert(neighbor, tentative);
                    paths_from.insert(neighbor, Some(coordinate));
                    to_search.push(Node {
                        coordinate: neighbor,
                        cost: tentative + hueristic(&neighbor, targets).unwrap_or(HEURISTIC_MAX),
                    });
                }
            }
        }

        None
    }
}

pub fn dist_between_new<T: PositionGettableGame>(
    game: &T,
    a: &T::NativePositionType,
    b: &T::NativePositionType,
) -> i32 {
    dist_between(
        &game.position_from_native(a.clone()),
        &game.position_from_native(b.clone()),
    )
}

pub fn dist_between(a: &Position, b: &Position) -> i32 {
    (a.x - b.x).abs() + (a.y - b.y).abs()
}

fn hueristic<T: CellNum>(start: &CellIndex<T>, targets: &[CellIndex<T>]) -> Option<i32> {
    let width = ((11 * 11) as f32).sqrt() as u8;

    targets
        .iter()
        .map(|coor| dist_between(&coor.into_position(width), &start.into_position(width)))
        .min()
}

fn hueristic_wire(start: &Position, targets: &[Position]) -> Option<i32> {
    targets.iter().map(|coor| dist_between(coor, start)).min()
}

impl APrimeCalculable for Game {
    fn a_prime_inner(
        &self,
        start: &Position,
        targets: &[Position],
        options: Option<APrimeOptions>,
    ) -> Option<APrimeResult<Position>> {
        let options = options.unwrap_or(APrimeOptions { food_penalty: 0 });
        let mut paths_from: FxHashMap<Position, Option<Position>> = FxHashMap::default();

        if targets.is_empty() {
            return None;
        }

        let mut to_search: BinaryHeap<Node<Position>> = BinaryHeap::new();

        let mut known_score: FxHashMap<Position, i32> = FxHashMap::default();

        to_search.push(Node {
            cost: 0,
            coordinate: *start,
        });
        known_score.insert(*start, 0);
        paths_from.insert(*start, None);

        while let Some(Node { cost, coordinate }) = to_search.pop() {
            if targets.contains(&coordinate) {
                return Some(APrimeResult {
                    best_cost: cost,
                    paths_from,
                    best_target: coordinate,
                });
            }

            let neighbor_distance = if self.board.hazards.contains(&coordinate) {
                HAZARD_PENALTY + NEIGHBOR_DISTANCE
            } else if self.board.food.contains(&coordinate) {
                NEIGHBOR_DISTANCE + options.food_penalty
            } else {
                NEIGHBOR_DISTANCE
            };

            let tentative = known_score.get(&coordinate).unwrap_or(&i32::MAX) + neighbor_distance;
            for neighbor in self.neighbors(&coordinate).into_iter().filter(|n| {
                targets.contains(n)
                    || self
                        .board
                        .snakes
                        .iter()
                        .all(|snake| !snake.body.contains(n))
            }) {
                if &tentative < known_score.get(&neighbor).unwrap_or(&i32::MAX) {
                    known_score.insert(neighbor, tentative);
                    paths_from.insert(neighbor, Some(coordinate));
                    to_search.push(Node {
                        coordinate: neighbor,
                        cost: tentative
                            + hueristic_wire(&neighbor, targets).unwrap_or(HEURISTIC_MAX),
                    });
                }
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use battlesnake_game_types::{
        compact_representation::CellBoard4Snakes11x11, wire_representation::Game,
    };

    fn cell_index_from_position_default_width(pos: Position) -> CellIndex<u8> {
        let width = ((11 * 11) as f32).sqrt() as u8;

        CellIndex::new(pos, width)
    }

    #[test]
    fn test_heuristic() {
        assert_eq!(
            hueristic(
                &cell_index_from_position_default_width(Position { x: 1, y: 1 }),
                &[cell_index_from_position_default_width(Position {
                    x: 2,
                    y: 2
                })]
            ),
            Some(2)
        );
    }

    #[test]
    fn test_multi_target_heuristic() {
        assert_eq!(
            hueristic(
                &cell_index_from_position_default_width(Position { x: 1, y: 1 }),
                &[
                    cell_index_from_position_default_width(Position { x: 3, y: 3 }),
                    cell_index_from_position_default_width(Position { x: 4, y: 4 }),
                    cell_index_from_position_default_width(Position { x: 5, y: 5 }),
                ]
            ),
            Some(4)
        );
    }

    #[test]
    fn test_basic_a_prime() {
        let json = b"{\"game\":{\"id\":\"\",\"ruleset\":{\"name\":\"royale\",\"version\":\"v1.0.17\"},\"timeout\":500},\"turn\":60,\"board\":{\"height\":11,\"width\":11,\"snakes\":[{\"id\":\"\",\"name\":\"\",\"latency\":\"100\",\"health\":86,\"body\":[{\"x\":10,\"y\":4}],\"head\":{\"x\":10,\"y\":4},\"length\":1,\"shout\":\"\"}],\"food\":[],\"hazards\":[]},\"you\":{\"id\":\"\",\"name\":\"\",\"latency\":\"100\",\"health\":86,\"body\":[{\"x\":10,\"y\":4}],\"head\":{\"x\":10,\"y\":4},\"length\":1,\"shout\":\"\"}}";
        let game: Game = serde_json::from_slice(json).unwrap();
        let id_map = battlesnake_game_types::types::build_snake_id_map(&game);

        assert_eq!(
            game.shortest_distance(
                &Position { x: 1, y: 1 },
                &[
                    Position { x: 3, y: 3 },
                    Position { x: 4, y: 4 },
                    Position { x: 5, y: 5 },
                ],
                None
            ),
            Some(4)
        );

        let compact: CellBoard4Snakes11x11 =
            battlesnake_game_types::compact_representation::CellBoard::convert_from_game(
                game, &id_map,
            )
            .unwrap();

        assert_eq!(
            compact.shortest_distance(
                &cell_index_from_position_default_width(Position { x: 1, y: 1 }),
                &[
                    cell_index_from_position_default_width(Position { x: 3, y: 3 }),
                    cell_index_from_position_default_width(Position { x: 4, y: 4 }),
                    cell_index_from_position_default_width(Position { x: 5, y: 5 }),
                ],
                None
            ),
            Some(4)
        );
    }

    #[test]
    fn test_real_example() {
        let board_json = r#"{"game":{"id":"","ruleset":{"name":"royale","version":"v1.0.17"},"timeout":500},"turn":60,"board": {"height":11,"width":11,"food":[],"hazards":[],"snakes":[{"id":"","name":"","health":93,"body":[{"x":7,"y":10},{"x":6,"y":10},{"x":5,"y":10},{"x":4,"y":10}],"latency":84,"head":{"x":7,"y":10},"length":4,"shout":"","squad":""},{"id":"","name":"","health":99,"body":[{"x":5,"y":4},{"x":5,"y":5},{"x":4,"y":5},{"x":3,"y":5},{"x":2,"y":5}],"latency":327,"head":{"x":5,"y":4},"length":4,"shout":"","squad":""}]},"you":{"id":"","name":"","health":99,"body":[{"x":5,"y":4},{"x":5,"y":5},{"x":4,"y":5},{"x":3,"y":5},{"x":2,"y":5}],"latency":327,"head":{"x":5,"y":4},"length":4,"shout":"","squad":""}}"#;
        let game: Game = serde_json::from_str(board_json).unwrap();
        let id_map = battlesnake_game_types::types::build_snake_id_map(&game);

        assert_eq!(
            game.shortest_distance(&Position { x: 5, y: 4 }, &[Position { x: 7, y: 10 },], None),
            Some(8)
        );

        let compact: CellBoard4Snakes11x11 =
            battlesnake_game_types::compact_representation::CellBoard::convert_from_game(
                game, &id_map,
            )
            .unwrap();

        assert_eq!(
            compact.shortest_distance(
                &cell_index_from_position_default_width(Position { x: 5, y: 4 }),
                &[cell_index_from_position_default_width(Position {
                    x: 7,
                    y: 10
                }),],
                None
            ),
            Some(8)
        );
    }

    #[test]
    fn test_start_of_game_path() {
        let board_json = include_str!("../fixtures/start_of_game.json");
        let game: Game = serde_json::from_str(board_json).unwrap();
        let id_map = build_snake_id_map(&game);

        let wire_path = game.shortest_path(
            &game.get_head_as_native_position(game.you_id()),
            &game.get_all_food_as_native_positions(),
            None,
        );

        let compact: CellBoard4Snakes11x11 =
            battlesnake_game_types::compact_representation::CellBoard::convert_from_game(
                game, &id_map,
            )
            .unwrap();

        let compact_path = compact.shortest_path(
            &compact.get_head_as_native_position(compact.you_id()),
            &compact.get_all_food_as_native_positions(),
            None,
        );

        dbg!(&compact_path);
        let width = ((11 * 11) as f32).sqrt() as u8;
        let compact_path_as_wire: Vec<Position> = compact_path
            .into_iter()
            .map(|x| x.into_position(width))
            .collect();
        assert_eq!(wire_path, compact_path_as_wire);
    }
}
