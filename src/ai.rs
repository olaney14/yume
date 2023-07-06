use std::{collections::VecDeque};

use json::JsonValue;

use crate::{entity::{Entity, TriggeredAction, EntityMovementInfo}, world::{World, Interaction}, game::Direction, player::Player};

pub enum AnimationAdvancementType {
    Cycle(i32),
    Loop
}

pub struct DirectionalAnimationData {
    //pub idle: u32,
    pub frames_per_direction: u32,
    pub up: u32,
    pub down: u32,
    pub left: u32,
    pub right: u32,
    pub direction: Direction,
    pub advance: AnimationAdvancementType
}

pub enum AnimationFrameData {
    SingleFrame(u32),
    FrameSequence{start: u32, idle: u32, len: u32, advance: AnimationAdvancementType},
    Directional(DirectionalAnimationData)
}

pub struct Animator {
    pub frame_data: AnimationFrameData,
    pub tileset: u32,
    pub frame: u32,
    pub speed: u32,
    pub timer: i32,
    pub on_move: bool,
    pub manual: bool
}

impl Animator {
    pub fn new(data: AnimationFrameData, tileset: u32, speed: u32) -> Self {
        let beginning_frame = match &data {
            AnimationFrameData::SingleFrame(frame) => { *frame },
            AnimationFrameData::FrameSequence { start, idle, len, advance } => { *start },
            AnimationFrameData::Directional(data) => { data.down * data.frames_per_direction + (data.frames_per_direction / 2) }
        };

        Self {
            frame_data: data,
            tileset,
            speed,
            timer: speed as i32,
            frame: beginning_frame,
            on_move: false,
            manual: false
        }
    }

    pub fn reset(&mut self) {
        let beginning_frame = match &self.frame_data {
            AnimationFrameData::SingleFrame(frame) => { *frame },
            AnimationFrameData::FrameSequence { start, idle, len, advance } => { *start },
            AnimationFrameData::Directional(data) => { data.down * data.frames_per_direction + (data.frames_per_direction / 2) }
        };

        self.frame = beginning_frame;
    }

    pub fn step(&mut self) -> u32 {
        self.timer -= 1;
        if self.timer == 0 {
            self.timer = self.speed as i32;
            match &mut self.frame_data {
                AnimationFrameData::SingleFrame(frame) => { self.frame = *frame },
                AnimationFrameData::FrameSequence { start, idle, len, advance } => {
                    match advance {
                        AnimationAdvancementType::Loop => {
                            self.frame += 1;
                            if self.frame == *start + *len {
                                self.frame = *start;
                            }
                        },
                        AnimationAdvancementType::Cycle(direction) => {
                            let advanced = self.frame as i32 + *direction;
                            if advanced <= *start as i32 || advanced >= (*start + *len - 1) as i32 {
                                *direction *= -1;
                            }
                            self.frame = advanced as u32;
                        }
                    }
                },
                AnimationFrameData::Directional(data) => {
                    let row = match data.direction {
                        Direction::Down => data.down,
                        Direction::Up => data.up,
                        Direction::Left => data.left,
                        Direction::Right => data.right
                    };

                    match &mut data.advance {
                        AnimationAdvancementType::Loop => {
                            self.frame += 1;
                            if self.frame == ((row + 1) * data.frames_per_direction as u32) {
                                self.frame = row * data.frames_per_direction as u32;
                            }
                        },
                        AnimationAdvancementType::Cycle(direction) => {
                            let advanced = self.frame as i32 + *direction;
                            if advanced <= (row * data.frames_per_direction as u32) as i32 || advanced >= ((row + 1) * data.frames_per_direction as u32) as i32 - 1 {
                                *direction *= -1;
                            }
                            self.frame = advanced as u32;
                        }
                    }
                }
            }
        }

        return self.frame;
    }
}

pub trait Ai {
    fn act(&mut self, entity: &mut Entity, world: &mut World, player: &Player, entity_list: &Vec<Entity>);
}

pub struct Wander {
    pub frequency: i32,
    pub delay: i32,
    pub timer: i32,
}

impl Ai for Wander {
    fn act(&mut self, entity: &mut Entity, world: &mut World, player: &Player, entity_list: &Vec<Entity>) {
        self.timer = (self.timer - 1).max(0);

        if self.timer == 0 {
            if (rand::random::<f32>() * self.frequency as f32).round() as i32 == 0 {
                entity.walk(rand::random::<Direction>(), world, player, entity_list);
                self.timer = self.delay;
            }
        }
    }
}

pub struct Chaser {
    pub speed: u32,
    pub pathfinder: Option<AStarPathfinder>,
    pub following_path: bool,
    pub init: bool,
    pub path_max: u32,
    pub detection_radius: u32,
    needs_recalculation: bool,
    player_last_pos: (u32, u32),
    last_walk_pos: (i32, i32)
}

pub struct Pushable {
    pub speed: u32,
    init: bool
}

pub struct AnimateOnInteract {
    pub frames: u32,
    timer: u32,
    takes_use: bool,
    takes_bump: bool,
    takes_walk: bool,
    counter: u32,
    side: Option<Direction>
}

impl Ai for Chaser {
    fn act(&mut self, entity: &mut Entity, world: &mut World, player: &Player, entity_list: &Vec<Entity>) {
        let player_pos = player.get_standing_tile();
        let player_in_range = manhattan_dist(player_pos.0, player_pos.1, entity.collision_x().max(0) as u32 / 16, entity.collision_y().max(0) as u32 / 16) <= self.detection_radius;

        if !self.init {
            self.init = true;
            self.pathfinder = Some(AStarPathfinder::new(world));
            entity.init_movement();
            entity.movement.as_mut().unwrap().speed = self.speed;
            self.player_last_pos = player_pos;
            self.last_walk_pos = (entity.collision_x() / 16, entity.collision_y() / 16);
        } else {
            if self.player_last_pos != player_pos {
                self.player_last_pos = player_pos;
                self.needs_recalculation = true;
            }
        }
        
        if player_in_range && self.needs_recalculation && !entity.movement.as_ref().unwrap().moving {
            self.needs_recalculation = false;
            let mut pathfinder = self.pathfinder.take().unwrap();
            if pathfinder.pathfind_to(entity.collision_x() as u32 / 16, entity.collision_y() as u32 / 16, player.x / 16, (player.y + 16) / 16, 0, player, world, entity_list).is_ok() {
                self.following_path = true;
            } else {
                self.needs_recalculation = false;
            }
            self.pathfinder = Some(pathfinder);
        }

        if player_in_range && self.following_path && !entity.movement.as_ref().unwrap().moving {
            if let Some(direction) = self.pathfinder.as_ref().unwrap().get_step() {
                let walk_pos = (entity.collision_x() / 16, entity.collision_y() / 16);
                if entity.walk(direction, world, player, entity_list) {
                    self.pathfinder.as_mut().unwrap().advance_step();
                }
                if entity.would_bump_player(direction, player) && self.last_walk_pos != walk_pos {
                    world.player_bump(entity.collision_x() / 16, entity.collision_y() / 16);
                }
                self.last_walk_pos = walk_pos;
            }
        }
    }
}

impl Ai for Pushable {
    fn act(&mut self, entity: &mut Entity, world: &mut World, player: &Player, entity_list: &Vec<Entity>) {
        if !self.init {
            self.init = true;
        }

        if entity.interaction.is_some() {
            if matches!(entity.interaction.as_ref().unwrap().0, Interaction::Use(_, _)) {
                let direction = entity.interaction.as_ref().unwrap().1.flipped();
                entity.walk(direction, world, player, entity_list);
                entity.interaction = None;
            }
        }
    }
}

impl Ai for AnimateOnInteract {
    fn act(&mut self, entity: &mut Entity, world: &mut World, player: &Player, entity_list: &Vec<Entity>) {
        if entity.interaction.is_some() {
            let mut fulfullled = false;
            if self.takes_use && matches!(entity.interaction.as_ref().unwrap().0, Interaction::Use(_, _)) {
                fulfullled = true;
            }
            if self.takes_bump && matches!(entity.interaction.as_ref().unwrap().0, Interaction::Bump(_, _)) {
                fulfullled = true;
            }
            if self.takes_walk && matches!(entity.interaction.as_ref().unwrap().0, Interaction::Walk(_, _)) {
                fulfullled = true;
            }
            if let Some(direction) = self.side {
                if player.facing.flipped() != direction {
                    fulfullled = false;
                }
            }
            if fulfullled && self.counter == 0 {
                self.counter = self.frames;
                if let Some(animator) = &mut entity.animator {
                    animator.reset();
                    self.timer = animator.speed;
                }
            }
            entity.interaction = None;
        }

        if self.counter != 0 {
            if self.timer != 0 {
                self.timer -= 1;
                if let Some(animator) = &mut entity.animator {
                    animator.step();
                }

                if self.timer == 0 {
                    self.counter -= 1;
                    if let Some(animator) = &entity.animator {
                        self.timer = animator.speed;
                    }
                }
            }
        }
    }
}

pub fn parse_ai(parsed: &JsonValue) -> Result<Box::<dyn Ai>, &str> {
    if !parsed["type"].is_string() { return Err("No ai type"); }
    
    match parsed["type"].as_str().unwrap() {
        "wander" => {
            let frequency = parsed["frequency"].as_i32().unwrap_or(100);
            let delay = parsed["delay"].as_i32().unwrap_or(25);
            return Ok(Box::new(Wander {
                            frequency,
                            delay,
                            timer: delay
                        }));
        },
        "chaser" => {
            let speed = parsed["speed"].as_u32().unwrap_or(2);
            let path_max = parsed["path_max"].as_u32().unwrap_or(ASTAR_MAX_STEPS);
            let detection_radius = parsed["detection_radius"].as_u32().unwrap_or(16);
            return Ok(Box::new(
                Chaser {
                    speed,
                    pathfinder: None,
                    following_path: false,
                    init: false,
                    needs_recalculation: true,
                    detection_radius,
                    path_max,
                    player_last_pos: (0, 0),
                    last_walk_pos: (0, 0)
                }
            ))
        },
        "push" => {
            let speed = parsed["speed"].as_u32().unwrap_or(2);
            return Ok(Box::new(
                Pushable {
                    init: false,
                    speed
                }
            ))
        },
        "animate_on_interact" => {
            let frames = parsed["frames"].as_u32().unwrap_or(1);
            let takes_use = parsed["use"].as_bool().unwrap_or(false);
            let takes_bump = parsed["bump"].as_bool().unwrap_or(false);
            let takes_walk = parsed["walk"].as_bool().unwrap_or(false);
            
            let mut side = None;

            if parsed["side"].is_string() {
                let dir = parsed["side"].as_str().unwrap().parse::<Direction>();
                if let Ok(direction) = dir {
                    side = Some(direction);
                }
            }

            return Ok(Box::new(
                AnimateOnInteract {
                    counter: 0,
                    frames,
                    timer: 0,
                    takes_bump,
                    takes_use,
                    takes_walk,
                    side
                }
            ));
        }
        _ => return Err("Unknown ai type")
    }
}

pub const DEFAULT_ANIMATION_SPEED: u32 = 5;

pub fn parse_animator(parsed: &JsonValue, tileset: u32) -> Result<Animator, &str> {
    if !parsed["type"].is_string() { return Err("No animation type") }
    let repeat = match parsed["repeat"].as_str() {
        Some(v) => {
            match v {
                "cycle" => AnimationAdvancementType::Cycle(1),
                "loop" => AnimationAdvancementType::Loop,
                _ => return Err("Invalid animation repeat type")
            }
        },
        None => AnimationAdvancementType::Loop
    };
    let on_move = parsed["on_move"].as_bool().unwrap_or(false);
    let manual = parsed["manual"].as_bool().unwrap_or(false);

    match parsed["type"].as_str().unwrap() {
        "still" => {
            if !parsed["frame"].is_number() { return Err("No frame specified for still animation"); }

            return Ok(Animator { 
                frame_data: AnimationFrameData::SingleFrame(parsed["frame"].as_u32().unwrap()), 
                tileset, 
                frame: 0, 
                speed: 0, 
                timer: 0,
                on_move,
                manual
            });
        },
        "sequence" => {
            if !parsed["start"].is_number() { return Err("No starting frame specified for frame sequence"); }
            if !parsed["length"].is_number() { return Err("No length specified for frame sequence"); }
            let start = parsed["start"].as_u32().unwrap();
            let speed = parsed["speed"].as_u32().unwrap_or(DEFAULT_ANIMATION_SPEED);
            let length = parsed["length"].as_u32().unwrap();

            return Ok(Animator {
                frame: start,
                speed,
                tileset,
                timer: speed as i32,
                frame_data: AnimationFrameData::FrameSequence { 
                    start, 
                    idle: parsed["idle"].as_u32().unwrap_or((2 * start + length) / 2),
                    len: length, 
                    advance: repeat 
                },
                on_move,
                manual
            });
        },
        "directional" => {
            let up = parsed["up"].as_u32().unwrap_or(0);
            let down = parsed["down"].as_u32().unwrap_or(1);
            let left = parsed["left"].as_u32().unwrap_or(2);
            let right = parsed["right"].as_u32().unwrap_or(3);
            if !parsed["frames"].is_number() { return Err("No frames length specified for directional sequence"); }
            let frames = parsed["frames"].as_u32().unwrap();
            let speed = parsed["speed"].as_u32().unwrap_or(DEFAULT_ANIMATION_SPEED);
            //let idle = parsed["idle"].as_u32().unwrap_or(down * frames + (frames / 2));

            return Ok(
                Animator {
                    frame: down * frames + (frames / 2),
                    speed,
                    tileset,
                    timer: speed as i32,
                    frame_data: AnimationFrameData::Directional(DirectionalAnimationData {
                        advance: repeat,
                        direction: Direction::Down,
                        frames_per_direction: frames,
                        down,
                        left,
                        right,
                        up
                    }),
                    on_move,
                    manual
                }
            )
        },
        _ => return Err("Unrecognized animation type")
    }
}

pub fn manhattan_dist(x0: u32, y0: u32, x1: u32, y1: u32) -> u32 {
    x0.abs_diff(x1) + y0.abs_diff(y1)
}

const ASTAR_MAX_STEPS: u32 = 10000;

pub struct AStarPathfinder {
    // g, h costs
    costs: Vec<PathfinderTile>,
    pub cur_path: VecDeque<Direction>
}

#[derive(Debug)]
struct PathfinderTile {
    pub g_cost: u32,
    pub h_cost: u32,
    pub direction: Option<Direction>,
    pub checked: bool
}

impl AStarPathfinder {
    pub fn new(world: &World) -> Self {
        let mut costs = Vec::with_capacity((world.width * world.height) as usize);
        for _ in 0..world.height * world.width {
            costs.push(PathfinderTile {
                checked: false,
                direction: None,
                g_cost: u32::MAX / 2 - 1,
                h_cost: u32::MAX / 2 - 1
            });
        }
        Self {
            costs,
            cur_path: VecDeque::new()
        }
    }

    pub fn clear(&mut self) {
        for tile in self.costs.iter_mut() {
            tile.checked = false;
            tile.direction = None;
            tile.g_cost = u32::MAX / 2 - 1;
            tile.h_cost = u32::MAX / 2 - 1;
        }
        self.cur_path.clear();
    }

    /// Only call if a valid path was found, hangs forever or panics if else <br>
    /// pwease
    pub fn calc_path(&mut self, x0: u32, y0: u32, x1: u32, y1: u32, world: &mut World) -> bool {
        // Start at the end and retrace steps back to the beginning to find the path
        let mut steps = Vec::new();
        let mut x = x1;
        let mut y = y1;

        while !(x == x0 && y == y0) {
            let direction = self.costs[(y * world.width + x) as usize].direction;
            if let Some(dir) = direction {
                steps.push(dir.flipped());
                x = (x as i32 + dir.x()) as u32;
                y = (y as i32 + dir.y()) as u32;
            } else {
                return false;
            }
        }
        steps = steps.into_iter().rev().collect();
        self.cur_path = steps.into();
        return true;
    }

    pub fn get_step(&self) -> Option<Direction> {
        self.cur_path.front().copied()
    }

    pub fn advance_step(&mut self) -> Option<Direction> {
        self.cur_path.pop_front()
    }

    pub fn pathfind_to(&mut self, x0: u32, y0: u32, x1: i32, y1: i32, height: i32, player: &Player, world: &mut World, entity_list: &Vec<Entity>) -> Result<(), ()> {
        self.clear();
        let mut i = 0;
        let mut x = x0;
        let mut y = y0;
        if x1 < 0 || y1 < 0 || x1 >= world.width as i32 || y1 >= world.height as i32 {
            return Err(());
        }
        while i < ASTAR_MAX_STEPS {
            use Direction::*;
            for dir in [Up, Down, Left, Right].into_iter() {
                let check_x = x as i32 + dir.x();
                let check_y = y as i32 + dir.y();

                if check_x == x1 as i32 && check_y == y1 as i32 {
                    self.costs[(check_y * world.width as i32 + check_x) as usize].direction = Some(dir.flipped());
                    // for y in 0..world.height {
                    //     for x in 0..world.width {
                    //         print!("{}", match self.costs[(y * world.width + x) as usize].direction {
                    //             Some(Direction::Down) => "v", Some(Direction::Up) => "^", Some(Direction::Left) => "<", Some(Direction::Right) => ">",
                    //             None => "."
                    //         });
                    //     }
                    //     println!("");
                    // }
                    // println!("");
                    if self.calc_path(x0, y0, x1 as u32, y1 as u32, world) {
                        return Ok(());
                    } else {
                        return Err(());
                    }
                }

                // if the coordinate is either out of bounds or blocked, either do nothing or keep the value
                if check_x < 0 || check_y < 0 || check_x >= world.width as i32 || check_y >= world.height as i32
                || world.collide_entity_at_tile_with_list(check_x as u32, check_y as u32, None, height, entity_list) {
                    continue;
                }

                let index = (check_y * world.width as i32 + check_x) as usize;
                let last_g = self.costs[index].g_cost;
                let last_h = self.costs[index].h_cost;
                let new_g = manhattan_dist(x0, y0, check_x as u32, check_y as u32);
                let new_h = manhattan_dist(x1 as u32, y1 as u32, check_x as u32, check_y as u32);
                if new_g < last_g {
                    self.costs[index].g_cost = new_g;
                    self.costs[index].direction = Some(dir.flipped());
                }  
                if new_h < last_h {
                    self.costs[index].h_cost = new_h;
                }
            }

            let min = self.costs.iter().enumerate().min_by(|(_, a), (_, b)| {
                let f0 = a.g_cost + a.h_cost;
                let f1 = b.g_cost + b.h_cost;
    
                if a.checked && !b.checked {
                    return std::cmp::Ordering::Greater;
                } else if !a.checked && b.checked {
                    return std::cmp::Ordering::Less;
                }

                let cmp = f0.cmp(&f1);
                match cmp {
                    std::cmp::Ordering::Equal => return a.h_cost.cmp(&b.h_cost),
                    _ => return cmp
                }
            });
    
            if let Some((index, _)) = min {
                if self.costs[index].checked || self.costs[index].direction == None {
                    // TODO: this is questionable and might break something
                    // but the lag reduction makes the game playable
                    return Err(()); 
                }
                self.costs[(y * world.width + x) as usize].checked = true;
                x = index as u32 % world.width;
                y = index as u32 / world.width;
            } else {
                eprintln!("No min?");
                break;
            }

            i += 1;
        }

        println!("Overrun");
        Err(())
    }
}