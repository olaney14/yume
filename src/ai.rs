use std::{collections::VecDeque, str::FromStr};

use json::JsonValue;
use rand::Rng;

use crate::{entity::Entity, game::Direction, player::Player, world::{self, Interaction, World}};

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

pub struct FollowAnimationData {
    pub follow_vec: (i32, i32),
    pub easing: u32,
    pub center: u32,
    pub axes: world::Axis
}

pub enum AnimationFrameData {
    SingleFrame(u32),
    FrameSequence{start: u32, idle: u32, len: u32, advance: AnimationAdvancementType},
    Directional(DirectionalAnimationData),
    Follow(FollowAnimationData),
}

pub struct Animator {
    pub frame_data: AnimationFrameData,
    pub tileset: u32,

    /// this is only Some if frame_data is Follow
    pub tileset_width: Option<u32>,
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
            AnimationFrameData::FrameSequence { start, .. } => { *start },
            AnimationFrameData::Directional(data) => { data.down * data.frames_per_direction + (data.frames_per_direction / 2) },
            AnimationFrameData::Follow(data) => { data.center }
        };

        Self {
            frame_data: data,
            tileset,
            tileset_width: None,
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
            AnimationFrameData::FrameSequence { start, .. } => { *start },
            AnimationFrameData::Directional(data) => { data.down * data.frames_per_direction + (data.frames_per_direction / 2) },
            AnimationFrameData::Follow(data) => { data.center }
        };

        self.frame = beginning_frame;
        self.timer = self.speed as i32;
        match &mut self.frame_data {
            AnimationFrameData::FrameSequence { advance, .. } => {
                match advance {
                    AnimationAdvancementType::Cycle(direction) => {
                        *direction = 1;
                    },
                    _ => ()
                }
            },
            AnimationFrameData::Directional(dir) => {
                match &mut dir.advance {
                    AnimationAdvancementType::Cycle(direction) => {
                        *direction = 1;
                    },
                    _ => ()
                }
            }
            _ => ()
        }
    }

    pub fn step(&mut self) -> u32 {
        self.timer -= 1;
        if self.timer == 0 {
            self.timer = self.speed as i32;
            match &mut self.frame_data {
                AnimationFrameData::SingleFrame(frame) => { self.frame = *frame },
                AnimationFrameData::FrameSequence { start, len, advance, .. } => {
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
                },
                AnimationFrameData::Follow(data) => {
                    assert!(self.tileset_width.is_some());
                    // TODO: add easing
                    let look_offset = match &data.axes {
                        &world::Axis::Horizontal => {
                            
                            (data.follow_vec.0, 0)
                        }
                        &world::Axis::Vertical => {
                            (0, data.follow_vec.1)
                        }
                        &world::Axis::All => {
                            data.follow_vec
                        }
                    };

                    self.frame = (data.center as i32 + look_offset.0 + (look_offset.1 * self.tileset_width.unwrap() as i32)).max(0) as u32;
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
    pub speed: u32,
    pub move_delay: u32,
}

pub struct MoveStraight {
    pub direction: Direction,
}

impl Ai for Wander {
    fn act(&mut self, entity: &mut Entity, world: &mut World, player: &Player, entity_list: &Vec<Entity>) {
        if entity.movement.is_none() {
            entity.init_movement();
            entity.movement.as_mut().unwrap().speed = self.speed;
            entity.movement.as_mut().unwrap().delay = self.move_delay;
        }

        self.timer = (self.timer - 1).max(0);
        //dbg!(self.timer);
        if self.timer == 0 {
            
            if (rand::random::<f32>() * self.frequency as f32).round() as i32 == 0 {
                entity.walk(rand::random::<Direction>(), world, player, entity_list);
                self.timer = self.delay;
            }
        }
    }
}

impl Ai for MoveStraight {
    fn act(&mut self, entity: &mut Entity, world: &mut World, player: &Player, entity_list: &Vec<Entity>) {
        entity.walk(self.direction, world, player, entity_list);
    }
}

pub enum PathfinderType {
    AStar,
    WalkTowards,
    Erratic
}

impl PathfinderType {
    pub fn parse(input: &str) -> Option<Self> {
        match input.to_lowercase().as_str() {
            "astar" | "a_star" | "a*" => return Some(Self::AStar),
            "walk_towards" | "walktowards" => return Some(Self::WalkTowards),
            "erratic" => return Some(Self::Erratic),
            _ => return None
        }
    }

    pub fn initialize(&self, world: &World) -> Pathfinder {
        match self {
            Self::AStar => return Pathfinder::a_star(world),
            Self::WalkTowards => return Pathfinder::walk_towards(),
            Self::Erratic => return Pathfinder::erratic()
        }
    }
}

pub struct Chaser {
    pub speed: u32,
    pub pathfinder_type: PathfinderType,
    pub pathfinder: Option<Pathfinder>,
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

pub struct Bird {
    pub speed: u32,
    init: bool,
    cur_direction: Direction
}

impl Ai for Chaser {
    fn act(&mut self, entity: &mut Entity, world: &mut World, player: &Player, entity_list: &Vec<Entity>) {
        let player_pos = player.get_standing_tile();
        let player_in_range = looped_manhattan_distance(player_pos.0, player_pos.1, entity.collision_x().max(0) as u32 / 16, entity.collision_y().max(0) as u32 / 16, world.width, world.height) <= self.detection_radius;

        if !self.init {
            self.init = true;
            self.pathfinder = Some(self.pathfinder_type.initialize(world));
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

        // Update calculated pathfinder
        if self.pathfinder.as_ref().unwrap().is_calculated() {
            if player_in_range && self.needs_recalculation && !entity.movement.as_ref().unwrap().moving {
                self.needs_recalculation = false;
                let mut pathfinder_container = self.pathfinder.take().unwrap();
                let pathfinder = pathfinder_container.get_calculated().unwrap();
                let x = (entity.collision_x() / 16).rem_euclid(world.width as i32) as u32;
                let y = (entity.collision_y() / 16).rem_euclid(world.height as i32) as u32;
                if pathfinder.pathfind_to(x, y, player.x / 16, (player.y + 16) / 16, 0, player, world, entity_list).is_ok() {
                    self.following_path = true;
                } else {
                    self.needs_recalculation = false;
                }
                self.pathfinder = Some(pathfinder_container);
            }

            if player_in_range && self.following_path && !entity.movement.as_ref().unwrap().moving {
                if let Some(direction) = self.pathfinder.as_mut().unwrap().get_calculated().as_ref().unwrap().get_step() {
                    let walk_pos = (entity.collision_x() / 16, entity.collision_y() / 16);
                    if entity.walk(direction, world, player, entity_list) {
                        self.pathfinder.as_mut().unwrap().get_calculated().as_mut().unwrap().advance_step();
                    }
                    if entity.would_bump_player(direction, player) && self.last_walk_pos != walk_pos {
                        world.player_bump(entity.collision_x() / 16, entity.collision_y() / 16);
                    }
                    self.last_walk_pos = walk_pos;
                }
            }
        } else { // Update polled pathfinder
            if !entity.movement.as_ref().unwrap().moving {
                let x = (entity.collision_x() / 16).rem_euclid(world.width as i32) as u32;
                let y = (entity.collision_y() / 16).rem_euclid(world.height as i32) as u32;
                if player_in_range {
                    if let Some(direction) = self.pathfinder.as_mut().unwrap().get_polled().as_mut().unwrap()
                        .poll(x, y, player.x / 16, (player.y + 16) / 16, 0, player, world, entity_list) {
                        let walk_pos = (entity.collision_x() / 16, entity.collision_y() / 16);
                        entity.walk(direction, world, player, entity_list);
                        if entity.would_bump_player(direction, player) && self.last_walk_pos != walk_pos {
                            world.player_bump(entity.collision_x() / 16, entity.collision_y() / 16);
                        }
                        self.last_walk_pos = walk_pos;
                    }
                } else {
                    if let Some(direction) = self.pathfinder.as_mut().unwrap().get_polled().as_mut().unwrap()
                        .idle(x, y, 0, player, world, entity_list) {
                        let walk_pos = (entity.collision_x() / 16, entity.collision_y() / 16);
                        entity.walk(direction, world, player, entity_list);
                        if entity.would_bump_player(direction, player) && self.last_walk_pos != walk_pos {
                            world.player_bump(entity.collision_x() / 16, entity.collision_y() / 16);
                        }
                        self.last_walk_pos = walk_pos;
                    }
                }
            }
        }
    }
}

impl Ai for Pushable {
    fn act(&mut self, entity: &mut Entity, world: &mut World, player: &Player, entity_list: &Vec<Entity>) {
        if !self.init {
            self.init = true;
            entity.init_movement();
            entity.movement.as_mut().unwrap().speed = self.speed;
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

impl Ai for Bird {
    fn act(&mut self, entity: &mut Entity, world: &mut World, player: &Player, entity_list: &Vec<Entity>) {
        if !self.init {
            self.init = true;
            entity.init_movement();
            entity.movement.as_mut().unwrap().speed = self.speed;
            self.cur_direction = if rand::thread_rng().gen::<bool>() {Direction::Left} else {Direction::Right};
        }

        if !entity.movement.as_ref().unwrap().moving {
            if rand::thread_rng().gen_range(0.0..1.0) < 0.025 {
                if rand::thread_rng().gen::<bool>() {
                    entity.walk(Direction::Up, world, player, entity_list);
                } else {
                    entity.walk(Direction::Down, world, player, entity_list);
                }
            } else if entity.can_move_in_direction_looping(self.cur_direction, world, player, entity_list) {
                entity.walk(self.cur_direction, world, player, entity_list);
            } else {
                self.cur_direction = self.cur_direction.flipped();
            }
        }
    }
}

impl Ai for AnimateOnInteract {
    fn act(&mut self, entity: &mut Entity, _world: &mut World, player: &Player, _entity_list: &Vec<Entity>) {
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
            let speed = parsed["speed"].as_u32().unwrap_or(2);
            let move_delay = parsed["move_delay"].as_u32().unwrap_or(0);
            return Ok(Box::new(Wander {
                            frequency,
                            delay,
                            speed,
                            timer: delay,
                            move_delay
                        }));
        }, // TODO: add speed ?????
        "move_straight" => {
            let direction = Direction::from_str(parsed["direction"].as_str().expect("Direction must be a string")).expect("Invalid direction");
            return Ok(Box::new(MoveStraight {
                direction
            }));
        },
        "chaser" => {
            let speed = parsed["speed"].as_u32().unwrap_or(1);
            let path_max = parsed["path_max"].as_u32().unwrap_or(ASTAR_MAX_STEPS);
            let detection_radius = parsed["detection_radius"].as_u32().unwrap_or(16);
            let pathfinder = parsed["pathfinder"].as_str().unwrap_or("walk_towards");
            return Ok(Box::new(
                Chaser {
                    speed,
                    pathfinder_type: PathfinderType::parse(pathfinder).expect("Invalid pathfinder type"),
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
        },
        "bird" => {
            let speed = parsed["speed"].as_u32().unwrap_or(2);
            return Ok(Box::new(Bird {
                cur_direction: Direction::Left,
                init: false,
                speed
            }));
        }
        _ => return Err("Unknown ai type")
    }
}

pub const DEFAULT_ANIMATION_SPEED: u32 = 5;

pub fn parse_animator(parsed: &JsonValue, tileset: u32, tileset_width: u32) -> Result<Animator, &str> {
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
                tileset_width: Some(tileset_width),
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
                tileset_width: Some(tileset_width),
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

            return Ok(
                Animator {
                    frame: down * frames + (frames / 2),
                    speed,
                    tileset,
                    tileset_width: Some(tileset_width),
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
        "follow" => {
            let center = parsed["center"].as_u32().expect("Expected u32 for follow animation center.");
            let axes = world::Axis::parse(parsed["axes"].as_str().expect("Expected string for follow animation axes.")).expect("Could not parse axes for follow animation");
            let speed = parsed["speed"].as_u32().unwrap_or(DEFAULT_ANIMATION_SPEED);

            return Ok(
                Animator {
                    frame: center,
                    speed,
                    tileset,
                    tileset_width: Some(tileset_width),
                    timer: speed as i32,
                    frame_data: AnimationFrameData::Follow(FollowAnimationData {
                        axes,
                        center: center,
                        easing: 0,
                        follow_vec: (0, 0)
                    }),
                    manual,
                    on_move
                }
            )
        }
        _ => return Err("Unrecognized animation type")
    }
}

pub enum Pathfinder {
    Calculated(Box<dyn CalculatedPathfinder>),
    Polled(Box<dyn PolledPathfinder>)
}

impl Pathfinder {
    pub fn a_star(world: &World) -> Self {
        Self::Calculated(Box::new(
            AStarPathfinder::new(world)
        ))
    }

    pub fn walk_towards() -> Self {
        Self::Polled(Box::new(
            WalkTowardsPathfinder {}
        ))
    }

    pub fn erratic() -> Self {
        Self::Polled(Box::new(
            ErraticPathfinder {}
        ))
    }

    pub fn is_polled(&self) -> bool {
        matches!(self, Self::Polled(..))
    }

    pub fn is_calculated(&self) -> bool {
        matches!(self, Self::Calculated(..))
    }

    pub fn get_polled(&mut self) -> Option<&mut Box<dyn PolledPathfinder>> {
        match self {
            Self::Polled(p) => return Some(p),
            _ => return None
        }
    }

    pub fn get_calculated(&mut self) -> Option<&mut Box<dyn CalculatedPathfinder>> {
        match self {
            Self::Calculated(c) => return Some(c),
            _ => return None
        }
    }
}

pub trait CalculatedPathfinder {
    fn pathfind_to(&mut self, x0: u32, y0: u32, x1: i32, y1: i32, height: i32, _player: &Player, world: &mut World, entity_list: &Vec<Entity>) -> Result<(), ()>;
    fn get_step(&self) -> Option<Direction>;
    fn advance_step(&mut self) -> Option<Direction>;
}

pub trait PolledPathfinder {
    fn poll(&mut self, x0: u32, y0: u32, x1: i32, y1: i32, height: i32, player: &Player, world: &mut World, entity_list: &Vec<Entity>) -> Option<Direction>;
    fn idle(&mut self, x: u32, y: u32, height: i32, player: &Player, world: &mut World, entity_list: &Vec<Entity>) -> Option<Direction> {
        None
    }
}

pub fn manhattan_dist(x0: u32, y0: u32, x1: u32, y1: u32) -> u32 {
    x0.abs_diff(x1) + y0.abs_diff(y1)
}

pub fn manhattan_looped_dist(x0: u32, y0: u32, x1: u32, y1: u32, width: u32, height: u32) -> u32 {
    let xbase0 = (x0 as i32 - x1 as i32).rem_euclid(width as i32) as u32;
    let ybase0 = (y0 as i32 - y1 as i32).rem_euclid(height as i32) as u32;
    let xbase1 = (x1 as i32 - x0 as i32).rem_euclid(width as i32) as u32;
    let ybase1 = (y1 as i32 - y0 as i32).rem_euclid(height as i32) as u32;

    return manhattan_dist(x0, y0, x1, y1)
        .min(manhattan_dist(0, 0, xbase0, ybase0))
        .min(manhattan_dist(0, 0, xbase1, ybase1));
}

const ASTAR_MAX_STEPS: u32 = 10000;

pub struct AStarPathfinder {
    // g, h costs
    costs: Vec<AStarPathfinderTile>,
    pub cur_path: VecDeque<Direction>
}

#[derive(Debug)]
struct AStarPathfinderTile {
    pub g_cost: u32,
    pub h_cost: u32,
    pub direction: Option<Direction>,
    pub checked: bool
}

impl CalculatedPathfinder for AStarPathfinder {
    fn get_step(&self) -> Option<Direction> {
        self.cur_path.front().copied()
    }

    fn advance_step(&mut self) -> Option<Direction> {
        self.cur_path.pop_front()
    }

    // TODO: !!!!! you can limit the radius of search to the entity's search radius
    // TODO: the thingy still sometimes "hesitates" at looping boundaries but it kinda works now
    fn pathfind_to(&mut self, x0: u32, y0: u32, x1: i32, y1: i32, height: i32, _player: &Player, world: &mut World, entity_list: &Vec<Entity>) -> Result<(), ()> {
        self.clear();
        let mut i = 0;
        let mut x = x0;
        let mut y = y0;
        if x1 < 0 || y1 < 0 || x1 >= world.width as i32 || y1 >= world.height as i32 {
            return Err(());
        }

        while i < ASTAR_MAX_STEPS {
            use Direction::*;

            //let mut time = Instant::now();
            for dir in [Up, Down, Left, Right].into_iter() {
                let mut check_x = x as i32 + dir.x();
                let mut check_y = y as i32 + dir.y();

                if world.looping {
                    if world.loop_horizontal() && (check_x < 0 || check_x >= world.width as i32) {
                        check_x = check_x.rem_euclid(world.width as i32);
                    }

                    if world.loop_vertical() && (check_y < 0 || check_y >= world.height as i32) {
                        check_y = check_y.rem_euclid(world.height as i32);
                    }
                }

                if check_x == x1 as i32 && check_y == y1 as i32 {
                    self.costs[(check_y * world.width as i32 + check_x) as usize].direction = Some(dir.flipped());
                    if self.calc_path(x0, y0, x1 as u32, y1 as u32, world) {
                        return Ok(());
                    } else {
                        return Err(());
                    }
                }

                // if the coordinate is either out of bounds or blocked, either do nothing or keep the value
                if !world.looping {
                    if check_x < 0 || check_y < 0 || check_x >= world.width as i32 || check_y >= world.height as i32 {
                        continue;
                    }
                }

                if world.collide_entity_at_tile_with_list(check_x as u32, check_y as u32, None, height, entity_list) {
                    continue;
                }

                let index = (check_y * world.width as i32 + check_x) as usize;
                let last_g = self.costs[index].g_cost;
                let last_h = self.costs[index].h_cost;
                let new_g = if world.looping { 
                    manhattan_looped_dist(x0, y0, check_x as u32, check_y as u32, world.width, world.height)
                } else {
                    manhattan_dist(x0, y0, check_x as u32, check_y as u32)
                };
                let new_h = if world.looping { 
                    manhattan_looped_dist(x1 as u32, y1 as u32, check_x as u32, check_y as u32, world.width, world.height)
                } else {
                    manhattan_dist(x1 as u32, y1 as u32, check_x as u32, check_y as u32)
                };
                if new_g < last_g {
                    self.costs[index].g_cost = new_g;
                    self.costs[index].direction = Some(dir.flipped());
                }
                if new_h < last_h {
                    self.costs[index].h_cost = new_h;
                }
            }

            //println!("Check time: {:?}", Instant::now() - time);
            //time = Instant::now();
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
            //println!("Find min time: {:?}", Instant::now() - time);
            //time = Instant::now();
            if let Some((index, _)) = min {
                if self.costs[index].checked || self.costs[index].direction == None {
                    // Break if we've repeated a check (this means there is nothing new to check)
                    eprintln!("bye bye! with {} loops", i);
                    return Err(());
                }
                self.costs[(y * world.width + x) as usize].checked = true;
                x = index as u32 % world.width;
                y = index as u32 / world.width;
            } else {
                eprintln!("No min?");
                break;
            }
            //println!("Final check time: {:?}", Instant::now() - time);

            i += 1;
        }

        println!("Overrun");
        Err(())
    }
}

impl AStarPathfinder {
    pub fn new(world: &World) -> Self {
        let mut costs = Vec::with_capacity((world.width * world.height) as usize);
        for _ in 0..world.height * world.width {
            costs.push(AStarPathfinderTile {
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
                if world.loop_horizontal() {
                    x = (x as i32 + dir.x()).rem_euclid(world.width as i32) as u32;
                } else {
                    x = (x as i32 + dir.x()) as u32;
                }

                if world.loop_vertical() {
                    y = (y as i32 + dir.y()).rem_euclid(world.height as i32) as u32;
                } else {
                    y = (y as i32 + dir.y()) as u32;
                }
            } else {
                return false;
            }
        }
        steps = steps.into_iter().rev().collect();
        self.cur_path = steps.into();
        return true;
    }
}

// pub fn manhattan_dist(x0: u32, y0: u32, x1: u32, y1: u32) -> u32 {
//     x0.abs_diff(x1) + y0.abs_diff(y1)
// }

// pub fn manhattan_looped_dist(x0: u32, y0: u32, x1: u32, y1: u32, width: u32, height: u32) -> u32 {
//     let xbase0 = (x0 as i32 - x1 as i32).rem_euclid(width as i32) as u32;
//     let ybase0 = (y0 as i32 - y1 as i32).rem_euclid(height as i32) as u32;
//     let xbase1 = (x1 as i32 - x0 as i32).rem_euclid(width as i32) as u32;
//     let ybase1 = (y1 as i32 - y0 as i32).rem_euclid(height as i32) as u32;

//     return manhattan_dist(x0, y0, x1, y1)
//         .min(manhattan_dist(0, 0, xbase0, ybase0))
//         .min(manhattan_dist(0, 0, xbase1, ybase1));
// }

pub fn looped_x_distance(x0: u32, x1: u32, width: u32) -> u32 {
    let xbase0 = (x0 as i32 - x1 as i32).rem_euclid(width as i32) as u32;
    let xbase1 = (x1 as i32 - x0 as i32).rem_euclid(width as i32) as u32;

    x0.abs_diff(x1).min(xbase0).min(xbase1)
}

pub fn looped_y_distance(y0: u32, y1: u32, height: u32) -> u32 {
    let ybase0 = (y0 as i32 - y1 as i32).rem_euclid(height as i32) as u32;
    let ybase1 = (y1 as i32 - y0 as i32).rem_euclid(height as i32) as u32;

    y0.abs_diff(y1).min(ybase0).min(ybase1)
}

pub fn looped_manhattan_distance(x0: u32, y0: u32, x1: u32, y1: u32, width: u32, height: u32) -> u32 {
    looped_x_distance(x0, x1, width) + looped_y_distance(y0, y1, height)
} 

pub struct WalkTowardsPathfinder;

impl PolledPathfinder for WalkTowardsPathfinder {
    fn poll(&mut self, x0: u32, y0: u32, x1: i32, y1: i32, height: i32, _: &Player, world: &mut World, entity_list: &Vec<Entity>) -> Option<Direction> {
        let diff_x = looped_x_distance(x0, x1 as u32, world.width);
        let diff_y = looped_y_distance(y0, y1 as u32, world.height);

        let mut suggested_direction;

        // this is RIDICULOUS
        // HOW does this work
        if diff_x > diff_y {
            if diff_x == 0 { return None; }
            let mut direction;
            if (x1 - x0 as i32) > 0 { direction = Direction::Right; }
            else { direction = Direction::Left; }
            if diff_x != x1.abs_diff(x0 as i32) { direction = direction.flipped() }
            suggested_direction = direction;
        } else {
            if diff_y == 0 { return None; }
            let mut direction;
            if (y1 - y0 as i32) > 0 { direction = Direction::Down; }
            else { direction = Direction::Up; }
            if diff_y != y1.abs_diff(y0 as i32) { direction = direction.flipped() }
            suggested_direction = direction;
        }

        let check_x = ((x0 as i32) + suggested_direction.x()).rem_euclid(world.width as i32);
        let check_y = ((y0 as i32) + suggested_direction.y()).rem_euclid(world.height as i32);
        if world.collide_entity_at_tile_with_list(check_x as u32, check_y as u32, None, height, entity_list) {
            match suggested_direction {
                Direction::Left | Direction::Right => {
                    if diff_y == 0 { return None; }
                    if (y1 - y0 as i32) > 0 { suggested_direction = Direction::Up }
                    else { suggested_direction = Direction::Down }
                    if y1.abs_diff(y0 as i32) != diff_x { suggested_direction = suggested_direction.flipped() }
                },
                _ => {
                    if diff_x == 0 { return None; }
                    if (x1 - x0 as i32) > 0 { suggested_direction = Direction::Right }
                    else { suggested_direction = Direction::Left }
                    if x1.abs_diff(x0 as i32) != diff_x { suggested_direction = suggested_direction.flipped() } 
                }
            }
        }
        
        return Some(suggested_direction);
    }
}

pub struct ErraticPathfinder {

}

impl PolledPathfinder for ErraticPathfinder {
    fn poll(&mut self, x0: u32, y0: u32, x1: i32, y1: i32, _: i32, _: &Player, world: &mut World, _: &Vec<Entity>) -> Option<Direction> {
        // taken from above
        let diff_x = looped_x_distance(x0, x1 as u32, world.width);
        let diff_y = looped_y_distance(y0, y1 as u32, world.height);

        let mut suggested_direction;

        if diff_x > diff_y {
            if diff_x == 0 { return None; }
            let mut direction;
            if (x1 - x0 as i32) > 0 { direction = Direction::Right; }
            else { direction = Direction::Left; }
            if diff_x != x1.abs_diff(x0 as i32) { direction = direction.flipped() }
            suggested_direction = direction;
        } else {
            if diff_y == 0 { return None; }
            let mut direction;
            if (y1 - y0 as i32) > 0 { direction = Direction::Down; }
            else { direction = Direction::Up; }
            if diff_y != y1.abs_diff(y0 as i32) { direction = direction.flipped() }
            suggested_direction = direction;
        }

        if rand::thread_rng().gen_range(0.0..1.0) < 0.1 {
            suggested_direction = rand::thread_rng().gen::<Direction>();
        }

        return Some(suggested_direction);
    }

    fn idle(&mut self, _: u32, _: u32, _: i32, _: &Player, _: &mut World, _: &Vec<Entity>) -> Option<Direction> {
        if rand::thread_rng().gen_range(0.0..1.0) < 0.005 {
            return Some(rand::thread_rng().gen::<Direction>());
        }

        None
    }
}