use std::{collections::HashMap, str::FromStr};

use json::JsonValue;
use rand::{prelude::Distribution, distributions::Standard};
use sdl2::{keyboard::Keycode, render::{Canvas, RenderTarget}, pixels::Color};

use crate::{player::Player, world::{World, QueuedEntityAction}};

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right
}

impl Direction {
    pub fn x(&self) -> i32 {
        use Direction::*;
        match *self {
            Up | Down => 0,
            Left => -1,
            Right => 1
        }
    }

    pub fn y(&self) -> i32 {
        use Direction::*;
        match *self {
            Left | Right => 0,
            Up => -1,
            Down => 1
        }
    }

    pub fn from_key(key: &Keycode) -> Option<Self> {
        use Keycode::*;
        match key {
            Up | W => Some(Self::Up),
            Left | A => Some(Self::Left),
            Right | D => Some(Self::Right),
            Down | S => Some(Self::Down),
            _ => None
        }
    }

    pub fn to_key(&self) -> Option<Keycode> {
        use Keycode::*;
        match *self {
            Self::Up => Some(Up),
            Self::Down => Some(Down),
            Self::Left => Some(Left),
            Self::Right => Some(Right),
        }
    }

    pub fn flipped(&self) -> Direction {
        match self {
            Self::Up => Self::Down,
            Self::Down => Self::Up,
            Self::Left => Self::Right,
            Self::Right => Self::Left
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ParseDirectionError;

impl FromStr for Direction {
    // todo uhhhh
    type Err = ParseDirectionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "up" | "top" => Ok(Direction::Up),
            "down" | "bottom" => Ok(Direction::Down),
            "left" => Ok(Direction::Left),
            "right" => Ok(Direction::Right),
            _ => Err(ParseDirectionError)
        }
    }
}

impl Distribution<Direction> for Standard {
    fn sample<R: rand::Rng + ?Sized>(&self, rng: &mut R) -> Direction {
        match rng.gen_range(0..4) {
            0 => Direction::Down,
            1 => Direction::Left,
            2 => Direction::Right,
            3 => Direction::Up,
            _ => unreachable!()
        }
    }
}

#[derive(Clone, Copy)]
pub enum KeyState {
    JustPressed,
    Pressed,
    Released
}

pub struct Input {
    pub keys: HashMap<Keycode, KeyState>
}

impl Input {
    pub fn new() -> Self {
        Self {
            keys: HashMap::new()
        }
    }

    pub fn update(&mut self) {
        for (_, v) in self.keys.iter_mut() {
            match *v {
                KeyState::JustPressed => *v = KeyState::Pressed,
                _ => (),
            }
        }
    }

    /// Notify the input manager that a key has been pressed
    pub fn pressed(&mut self, key: Keycode) {
        self.keys.insert(key, KeyState::JustPressed);
    }

    /// Notify the input manager that a key has been released
    pub fn released(&mut self, key: Keycode) {
        self.keys.insert(key, KeyState::Released);
    }

    /// Returns true if `key` is pressed
    pub fn get_pressed(&self, key: Keycode) -> bool {
        matches!(self.keys.get(&key).unwrap_or(&KeyState::Released), KeyState::Pressed | KeyState::JustPressed)
    }

    /// Returns true if `key` has just been pressed
    pub fn get_just_pressed(&self, key: Keycode) -> bool {
        matches!(self.keys.get(&key).unwrap_or(&KeyState::Released), KeyState::JustPressed)
    }

    /// Returns true if `key` is released
    pub fn get_released(&self, key: Keycode) -> bool {
        matches!(self.keys.get(&key).unwrap_or(&KeyState::Released), KeyState::Released)
    }

    /// Returns the keystate of `key`
    pub fn get_keystate(&self, key: Keycode) -> KeyState {
        *self.keys.get(&key).unwrap_or(&KeyState::Released)
    }
}

pub struct RenderState {
    pub offset: (i32, i32),
    pub screen_dims: (u32, u32),
    pub zoom: (f32, f32),
    pub screen_extents: (u32, u32),
    pub clamp: (bool, bool)
}

impl RenderState {
    pub fn new(screen_dims: (u32, u32)) -> Self {
        Self {
            offset: (0, 0),
            screen_dims,
            zoom: (2.0, 2.0),
            screen_extents: (
                (screen_dims.0 as f32 / 2.0) as u32,
                (screen_dims.1 as f32 / 2.0) as u32,
            ),
            clamp: (false, false)
        }
    }

    pub fn update_zoom(&mut self, x: f32, y: f32) {
        self.zoom = (x, y);
        self.screen_extents = (
            (self.screen_dims.0 as f32 / x) as u32,
            (self.screen_dims.1 as f32 / y) as u32,
        )
    }
}

#[derive(Clone)]
pub enum TransitionType {
    Fade,
    MusicOnly
}

#[derive(Clone)]
pub struct Transition {
    pub kind: TransitionType,
    pub progress: i32,
    pub direction: i32,
    pub speed: i32,
    pub fade_music: bool
}

impl Transition {
    pub fn new(kind: TransitionType, speed: i32, fade_music: bool) -> Self {
        Self {
            direction: 1,
            progress: 0,
            fade_music, kind, speed
        }
    }

    pub fn draw<T: RenderTarget>(&self, canvas: &mut Canvas<T>) {
        match self.kind {
            TransitionType::Fade => {
                let alpha = (255.0 * (self.progress as f32 / 100.0)).clamp(0.0, 255.0) as u8;
                canvas.set_blend_mode(sdl2::render::BlendMode::Blend);
                canvas.set_draw_color(Color::RGBA(0, 0, 0, alpha));
                canvas.fill_rect(None).unwrap();
            },
            TransitionType::MusicOnly => ()
        }
    }
}

pub fn parse_action(parsed: &JsonValue) -> Result<Box<dyn Action>, String> {
    if !parsed["type"].is_string() { return Err("Invalid or no type".to_string()); }

    match parsed["type"].as_str().unwrap() {
        "warp" => {
            return WarpAction::parse(parsed);
        },
        "print" => {
            return PrintAction::parse(parsed);
        },
        "delayed" => {
            return DelayedAction::parse(parsed);
        },
        "freeze" => {
            return FreezeAction::parse(parsed);
        },
        _ => {
            return Err(format!("Unknown action \"{}\"", parsed["type"].as_str().unwrap()));
        }
    }
}

pub trait Action {
    fn act(&self, player: &mut Player, world: &mut World);
}

#[derive(Clone)]
pub enum WarpCoord {
    Match,
    Pos(i32),
    Sub(i32),
    Add(i32),
    Default
}

pub struct QueuedLoad {
    pub map: String,
    pub pos: (WarpCoord, WarpCoord),
}

pub struct WarpAction {
    pub map: Option<String>,
    pub exit: (WarpCoord, WarpCoord),
    pub transition: Option<Transition>
}

pub struct DelayedAction {
    pub after: Box<dyn Action>,
    pub delay: u32
}

pub struct FreezeAction {}

impl WarpAction {
    pub fn parse(parsed: &JsonValue) -> Result<Box<dyn Action>, String> {
        let mut map = None;
        let mut transition = None;
        let mut transition_type = None;

        // Map
        if parsed["map"].is_string() {
            map = Some(parsed["map"].as_str().unwrap());
        }

        // Transition
        if parsed["transition"].is_string() {
            transition_type = Some(
                match parsed["transition"].as_str().unwrap() {
                    "fade" => TransitionType::Fade,
                    _ => return Err("Unknown transition type".to_string())
                }
            );
        }

        if let Some(kind) = transition_type {
            let mut transition_speed = 4;
            if parsed["transition_speed"].is_number() {
                transition_speed = parsed["transition_speed"].as_i32().unwrap();
            }
            transition = Some(Transition {
                kind,
                direction: 1,
                progress: 0,
                speed: transition_speed,
                fade_music: true
            });
        }

        // Pos
        if !parsed["pos"].is_object() { return Err("Invalid or missing position".to_string()); }
        let mut pos = (WarpCoord::Pos(0), WarpCoord::Pos(0));
        if parsed["pos"]["x"].is_string() {
            pos.0 = match parsed["pos"]["x"].as_str().unwrap() {
                "match" => WarpCoord::Match,
                other => {
                    if other.starts_with("sub") {
                        WarpCoord::Sub(other[3..].parse::<i32>().unwrap())
                    } else if other.starts_with("add") {
                        WarpCoord::Add(other[3..].parse::<i32>().unwrap())
                    } else {
                        return Err("Unknown X position".to_string());
                    }
                }
            }
        } else {
            if !parsed["pos"]["x"].is_number() { return Err("Invalid X position".to_string()); }
            pos.0 = WarpCoord::Pos(parsed["pos"]["x"].as_i32().unwrap());
        }
        if parsed["pos"]["y"].is_string() {
            pos.1 = match parsed["pos"]["y"].as_str().unwrap() {
                "match" => WarpCoord::Match,
                other => {
                    if other.starts_with("sub") {
                        WarpCoord::Sub(other[3..].parse::<i32>().unwrap())
                    } else if other.starts_with("add") {
                        WarpCoord::Add(other[3..].parse::<i32>().unwrap())
                    } else {
                        return Err("Unknown Y position".to_string());
                    }
                }
            }
        } else {
            if !parsed["pos"]["y"].is_number() { return Err("Invalid Y position".to_string()); }
            pos.1 = WarpCoord::Pos(parsed["pos"]["y"].as_i32().unwrap());
        }
        return Ok(Box::new(WarpAction {
                    exit: pos,
                    map: match map {
                        Some(m) => Some(m.to_owned()),
                        None => None
                    },
                    transition
                }));
    }
}

impl DelayedAction {
    pub fn parse(parsed: &JsonValue) -> Result<Box<dyn Action>, String> {
        if !parsed["delay"].is_number() { return Err("No delay time included".to_string()); }
        if !parsed["action"].is_object() { return Err("No action included for after delay".to_string()); }
        let parsed_action = parse_action(&parsed["action"]);
        if parsed_action.is_ok() {
            return Ok(
                Box::new(DelayedAction {
                                    after: parsed_action.unwrap(),
                                    delay: parsed["delay"].as_u32().expect("Invalid delay, likely negative or too high")
                                })
            );
        }

        parsed_action
    }
}

impl FreezeAction {
    pub fn parse(parsed: &JsonValue) -> Result<Box<dyn Action>, String> {
        return Ok(Box::new(FreezeAction {}));
    }
}

impl Action for FreezeAction {
    fn act(&self, player: &mut Player, world: &mut World) {
        player.frozen = true;
    }
}

impl Action for WarpAction {
    fn act(&self, player: &mut Player, world: &mut World) {
        if let Some(map) = &self.map {
            world.queued_load = Some(QueuedLoad {
                map: String::from("res/maps/") + map.as_str(),
                pos: self.exit.clone()
            });
            world.transition = self.transition.clone();
        }
    }
}

impl Action for DelayedAction {
    fn act(&self, player: &mut Player, world: &mut World) {
        if world.special_context.delayed_run {
            self.after.act(player, world);
        } else {
            world.queued_entity_actions.push(QueuedEntityAction {
                delay: self.delay as i32,
                action_id: world.special_context.action_id,
                entity_id: world.special_context.entity_id
            })
        }
    }
}

pub struct PrintAction {
    pub message: String,
}

impl PrintAction {
    pub fn parse(parsed: &JsonValue) -> Result<Box<dyn Action>, String> {
        if !parsed["message"].is_string() { return Err("Invalid message for print".to_string()); }
        Ok(Box::new(Self {
                            message: parsed["message"].as_str().unwrap().to_string()
                        }))
    }
}

impl Action for PrintAction {
    fn act(&self, _player: &mut Player, _world: &mut World) {
        println!("{}", self.message);
    }
}

// pub enum Condition {

// }

// pub struct ConditionalAction {
//     pub 
// }