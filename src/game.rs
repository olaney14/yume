use std::{collections::HashMap, str::FromStr, path::PathBuf, f32::consts::PI};

use json::JsonValue;
use rand::{prelude::Distribution, distributions::Standard};
use sdl2::{keyboard::Keycode, render::{Canvas, RenderTarget, TextureCreator}, pixels::Color, rect::Rect};

use crate::{player::Player, world::{World, QueuedEntityAction}, effect::Effect, texture::Texture, audio::Song};

pub fn offset_floor(n: i32, to: i32, offset: i32) -> i32 {
    (n as f32 / to as f32).floor() as i32 * to + (offset.abs() % to)
}

pub fn offset_ceil(n: i32, to: i32, offset: i32) -> i32 {
    (n as f32 / to as f32).ceil() as i32 * to - (offset.abs() % to)
}

pub fn ceil(n: i32, to: i32) -> i32 {
    (n as f32 / to as f32).ceil() as i32 * to
}

pub enum Condition {
    IntEquals(IntProperty, IntProperty),
    IntGreater(IntProperty, IntProperty),
    IntLess(IntProperty, IntProperty),
    StringEquals(StringProperty, StringProperty),
    EffectEquipped(Effect),
    Negate(Box<Condition>)
}

impl Condition {
    pub fn evaluate(&self, player: Option<&Player>, world: Option<&World>) -> bool {
        match self {
            Self::IntEquals(lhs, rhs) => {
                let lh_arg = lhs.get(player, world);
                let rh_arg = rhs.get(player, world);
                return lh_arg.is_some() && rh_arg.is_some() && lh_arg.unwrap() == rh_arg.unwrap();
            },
            Self::IntGreater(lhs, rhs) => {
                let lh_arg = lhs.get(player, world);
                let rh_arg = rhs.get(player, world);
                return lh_arg.is_some() && rh_arg.is_some() && lh_arg.unwrap() > rh_arg.unwrap();
            },
            Self::IntLess(lhs, rhs) => {
                let lh_arg = lhs.get(player, world);
                let rh_arg = rhs.get(player, world);
                return lh_arg.is_some() && rh_arg.is_some() && lh_arg.unwrap() < rh_arg.unwrap();
            },
            Self::StringEquals(lhs, rhs) => {
                let lh_arg = lhs.get(player, world);
                let rh_arg = rhs.get(player, world);
                return lh_arg.is_some() && rh_arg.is_some() && lh_arg.unwrap() == rh_arg.unwrap();
            },
            Self::EffectEquipped(effect) => {
                if let Some(p) = player {
                    return p.current_effect.is_some() && p.current_effect.as_ref().unwrap() == effect
                }
                return false;
            },
            Self::Negate(cond) => {
                return !cond.evaluate(player, world);
            }
        }
    }

    pub fn parse(json: &JsonValue) -> Option<Self> {
        if !json["type"].is_string() { return None; }
        match json["type"].as_str().unwrap() {
            "int_equals" => {
                if !(json["lhs"].is_object() || json["lhs"].is_number()) || !(json["rhs"].is_object() || json["rhs"].is_number()) { return None; }
                let lhs_parsed = IntProperty::parse(&json["lhs"]);
                let rhs_parsed = IntProperty::parse(&json["rhs"]);
                if lhs_parsed.is_some() && rhs_parsed.is_some() {
                    return Some(Condition::IntEquals(lhs_parsed.unwrap(), rhs_parsed.unwrap()))
                }
                return None;
            },
            "int_greater" => {
                if !(json["lhs"].is_object() || json["lhs"].is_number()) || !(json["rhs"].is_object() || json["rhs"].is_number()) { return None; }
                let lhs_parsed = IntProperty::parse(&json["lhs"]);
                let rhs_parsed = IntProperty::parse(&json["rhs"]);
                if lhs_parsed.is_some() && rhs_parsed.is_some() {
                    return Some(Condition::IntGreater(lhs_parsed.unwrap(), rhs_parsed.unwrap()))
                }
                return None;
            },
            "int_less" => {
                if !(json["lhs"].is_object() || json["lhs"].is_number()) || !(json["rhs"].is_object() || json["rhs"].is_number()) { return None; }
                let lhs_parsed = IntProperty::parse(&json["lhs"]);
                let rhs_parsed = IntProperty::parse(&json["rhs"]);
                if lhs_parsed.is_some() && rhs_parsed.is_some() {
                    return Some(Condition::IntLess(lhs_parsed.unwrap(), rhs_parsed.unwrap()))
                }
                return None;
            },
            "string_equals" => {
                if !json["lhs"].is_object() || !json["rhs"].is_object() { return None; }
                let lhs_parsed = StringProperty::parse(&json["lhs"]);
                let rhs_parsed = StringProperty::parse(&json["rhs"]);
                if lhs_parsed.is_some() && rhs_parsed.is_some() {
                    return Some(Condition::StringEquals(lhs_parsed.unwrap(), rhs_parsed.unwrap()))
                }
                return None;
            },
            "effect_equipped" => {
                if !json["effect"].is_string() { return None; }
                let effect_parsed = Effect::parse(&json["effect"].as_str().unwrap());
                if effect_parsed.is_some() {
                    return Some(Condition::EffectEquipped(effect_parsed.unwrap()));
                }
                return None;
            },
            "negate" => {
                if !json["condition"].is_object() { return None; }
                let parsed_condition = Condition::parse(&json["condition"]);

                if let Some(condition) = parsed_condition {
                    return Some(Condition::Negate(Box::new(condition)));
                }

                return None;
            }
            _ => return None,
        }
    }
}

#[derive(Clone)]
pub enum PlayerPropertyType {
    X,
    Y,
    Height
}

impl PlayerPropertyType {
    pub fn parse(json: &JsonValue) -> Option<Self> {
        let kind;
        if json.is_string() {
            kind = json.as_str().unwrap();
        } else {
            if !json["type"].is_string() { return None; }
            kind = json["type"].as_str().unwrap();
        }
       
        match kind {
            "x" => Some(PlayerPropertyType::X),
            "y" => Some(PlayerPropertyType::Y),
            "height" => Some(PlayerPropertyType::Height),
            _ => None
        }
    }
}

#[derive(Clone)]
pub enum LevelPropertyType {
    DefaultX,
    DefaultY,
    TintR,
    TintG,
    TintB,
    TintA,
    SpecialSaveGame,
    Paused,
    BackgroundR,
    BackgroundG,
    BackgroundB
}

impl LevelPropertyType {
    pub fn parse(json: &JsonValue) -> Option<Self> {
        let mut kind = None;
        if json.is_string() {
            kind = Some(json.as_str().unwrap());
        }
        else if json["type"].is_string() {
            kind = Some(json["type"].as_str().unwrap());
        }
        
        if let Some(_type) = kind {
            return match _type {
                "default_x" => Some(LevelPropertyType::DefaultX),
                "default_y" => Some(LevelPropertyType::DefaultY),
                "tint_r" => Some(LevelPropertyType::TintR),
                "tint_g" => Some(LevelPropertyType::TintG),
                "tint_b" => Some(LevelPropertyType::TintB),
                "tint_a" => Some(LevelPropertyType::TintA),
                "special_save_game" => Some(LevelPropertyType::SpecialSaveGame),
                "paused" => Some(LevelPropertyType::Paused),
                "background_r" => Some(LevelPropertyType::BackgroundR),
                "background_g" => Some(LevelPropertyType::BackgroundG),
                "background_b" => Some(LevelPropertyType::BackgroundB),
                _ => None
            };
        }

        return None;
    }
}

#[derive(Clone)]
pub enum FlagPropertyType {
    Global(String),
    Local(String)
}

#[derive(Clone)]
pub enum BoolProperty {
    Bool(bool),
    Player(PlayerPropertyType),
    Level(LevelPropertyType),
    And(Box<BoolProperty>, Box<BoolProperty>),
    Or(Box<BoolProperty>, Box<BoolProperty>),
    Not(Box<BoolProperty>),
    Xor(Box<BoolProperty>, Box<BoolProperty>)
}

impl BoolProperty {
    pub fn get(&self, player: Option<&Player>, world: Option<&World>) -> Option<bool> {
        match self {
            BoolProperty::Bool(b) => return Some(*b),
            BoolProperty::Player(prop) => {
                if let Some(p) = player {
                    match prop {
                        _ => return None
                    }
                }
            },
            BoolProperty::Level(prop) => {
                if let Some(level) = world {
                    match prop {
                        LevelPropertyType::Paused => return Some(level.paused),
                        LevelPropertyType::SpecialSaveGame => return Some(level.special_context.save_game),
                        _ => return None
                    }
                }
            },
            BoolProperty::And(b0, b1) => {
                let (lhs, rhs) = (b0.get(player, world), b1.get(player, world));
                if lhs.is_some() && rhs.is_some() {
                    return Some(lhs.unwrap() && rhs.unwrap())
                }   return None;
            },
            BoolProperty::Or(b0, b1) => {
                let (lhs, rhs) = (b0.get(player, world), b1.get(player, world));
                if lhs.is_some() && rhs.is_some() {
                    return Some(lhs.unwrap() || rhs.unwrap())
                }   return None;
            },
            BoolProperty::Xor(b0, b1) => {
                let (lhs, rhs) = (b0.get(player, world), b1.get(player, world));
                if lhs.is_some() && rhs.is_some() {
                    return Some(lhs.unwrap() ^ rhs.unwrap())
                }   return None;
            },
            BoolProperty::Not(b) => {
                let arg = b.get(player, world);
                if arg.is_some() {
                    return Some(!arg.unwrap())
                }   return None;
            }
        }
        
        None
    }

    pub fn parse(json: &JsonValue) -> Option<Self> {
        if json.is_boolean() {
            return Some(Self::Bool(json.as_bool().unwrap()));
        }

        if !json["type"].is_string() { return None; }
        match json["type"].as_str().unwrap() {
            "bool" => return Some(BoolProperty::Bool(json["val"].as_bool().unwrap())),
            "player" => return Some(BoolProperty::Player(PlayerPropertyType::parse(&json["property"]).unwrap())),
            "level" => return Some(BoolProperty::Level(LevelPropertyType::parse(&json["property"]).unwrap())),
            "and" => {
                if !(json["lhs"].is_boolean() || json["lhs"].is_object()) || !(json["rhs"].is_boolean() || json["rhs"].is_object()) { return None; }
                let lhs = BoolProperty::parse(&json["lhs"]);
                let rhs = BoolProperty::parse(&json["rhs"]);
                if lhs.is_some() && rhs.is_some() {
                    return Some(BoolProperty::And(Box::new(lhs.unwrap()), Box::new(rhs.unwrap())));
                } return None;
            },
            "or" => {
                if !(json["lhs"].is_boolean() || json["lhs"].is_object()) || !(json["rhs"].is_boolean() || json["rhs"].is_object()) { return None; }
                let lhs = BoolProperty::parse(&json["lhs"]);
                let rhs = BoolProperty::parse(&json["rhs"]);
                if lhs.is_some() && rhs.is_some() {
                    return Some(BoolProperty::Or(Box::new(lhs.unwrap()), Box::new(rhs.unwrap())));
                } return None;
            },
            "xor" => {
                if !(json["lhs"].is_boolean() || json["lhs"].is_object()) || !(json["rhs"].is_boolean() || json["rhs"].is_object()) { return None; }
                let lhs = BoolProperty::parse(&json["lhs"]);
                let rhs = BoolProperty::parse(&json["rhs"]);
                if lhs.is_some() && rhs.is_some() {
                    return Some(BoolProperty::Xor(Box::new(lhs.unwrap()), Box::new(rhs.unwrap())));
                } return None;
            },
            "not" => {
                if !(json["val"].is_boolean() || json["val"].is_object()) { return None; }
                let val = BoolProperty::parse(&json["val"]);
                if val.is_some() {
                    return Some(BoolProperty::Not(Box::new(val.unwrap())));
                } return None;
            },
            _ => return None,
        }
    }
}

#[derive(Clone)]
pub enum FloatProperty {
    Float(f32),
    Player(PlayerPropertyType),
    Level(LevelPropertyType),
    Add(Box<FloatProperty>, Box<FloatProperty>),
    Sub(Box<FloatProperty>, Box<FloatProperty>),
    Mul(Box<FloatProperty>, Box<FloatProperty>),
    Div(Box<FloatProperty>, Box<FloatProperty>)
}

// IntProperty::Add(a, b) => {
//     let lhs = a.get(player, world);
//     let rhs = b.get(player, world);
//     if lhs.is_some() && rhs.is_some() {
//         return Some(lhs.unwrap() + rhs.unwrap());
//     }

//     return None;
// },

impl FloatProperty {
    pub fn get(&self, player: Option<&Player>, world: Option<&World>) -> Option<f32> {
        match self {
            FloatProperty::Float(f) => return Some(*f),
            FloatProperty::Player(prop) => {
                if let Some(p) = player {
                    match prop {
                        _ => return None
                    }
                } else {
                    return None;
                }
            },
            FloatProperty::Level(prop) => {
                if let Some(w) = world {
                    match prop {
                        _ => return None
                    }
                } else {
                    return None;
                }
            },
            FloatProperty::Add(a, b) => {
                let (lhs, rhs) = (a.get(player, world), b.get(player, world));
                if lhs.is_some() && rhs.is_some() { return Some(lhs.unwrap() + rhs.unwrap()); }
                return None;
            },
            FloatProperty::Sub(a, b) => {
                let (lhs, rhs) = (a.get(player, world), b.get(player, world));
                if lhs.is_some() && rhs.is_some() { return Some(lhs.unwrap() - rhs.unwrap()); }
                return None;
            },
            FloatProperty::Mul(a, b) => {
                let (lhs, rhs) = (a.get(player, world), b.get(player, world));
                if lhs.is_some() && rhs.is_some() { return Some(lhs.unwrap() * rhs.unwrap()); }
                return None;
            },
            FloatProperty::Div(a, b) => {
                let (lhs, rhs) = (a.get(player, world), b.get(player, world));
                if lhs.is_some() && rhs.is_some() { return Some(lhs.unwrap() / rhs.unwrap()); }
                return None;
            }
        }
    }

    pub fn parse(json: &JsonValue) -> Option<Self> {
        if json.is_number() {
            return Some(FloatProperty::Float(json.as_f32().unwrap()));
        }

        if !json["type"].is_string() { return None; }
        match json["type"].as_str().unwrap() {
            "float" => return Some(FloatProperty::Float(json["val"].as_f32().unwrap())),
            "player" => return Some(FloatProperty::Player(PlayerPropertyType::parse(&json["property"]).unwrap())),
            "level" => return Some(FloatProperty::Level(LevelPropertyType::parse(&json["property"]).unwrap())),
            "add" => {
                if !(json["lhs"].is_number() || json["lhs"].is_object()) || !(json["rhs"].is_number() || json["rhs"].is_object()) { return None; }
                let (left, right) = ( FloatProperty::parse(&json["lhs"]), FloatProperty::parse(&json["rhs"]) );
                if left.is_some() && right.is_some() { return Some(FloatProperty::Add(Box::new(left.unwrap()), Box::new(right.unwrap()))); }
                return None;
            },
            "sub" => {
                if !(json["lhs"].is_number() || json["lhs"].is_object()) || !(json["rhs"].is_number() || json["rhs"].is_object()) { return None; }
                let (left, right) = ( FloatProperty::parse(&json["lhs"]), FloatProperty::parse(&json["rhs"]) );
                if left.is_some() && right.is_some() { return Some(FloatProperty::Sub(Box::new(left.unwrap()), Box::new(right.unwrap()))); }
                return None;
            },
            "mul" => {
                if !(json["lhs"].is_number() || json["lhs"].is_object()) || !(json["rhs"].is_number() || json["rhs"].is_object()) { return None; }
                let (left, right) = ( FloatProperty::parse(&json["lhs"]), FloatProperty::parse(&json["rhs"]) );
                if left.is_some() && right.is_some() { return Some(FloatProperty::Mul(Box::new(left.unwrap()), Box::new(right.unwrap()))); }
                return None;
            },
            "div" => {
                if !(json["lhs"].is_number() || json["lhs"].is_object()) || !(json["rhs"].is_number() || json["rhs"].is_object()) { return None; }
                let (left, right) = ( FloatProperty::parse(&json["lhs"]), FloatProperty::parse(&json["rhs"]) );
                if left.is_some() && right.is_some() { return Some(FloatProperty::Div(Box::new(left.unwrap()), Box::new(right.unwrap()))); }
                return None;
            },
            _ => return None,
        }
    }
}

#[derive(Clone)]
pub enum IntProperty {
    Int(i32),
    Player(PlayerPropertyType),
    Flag(FlagPropertyType),
    Level(LevelPropertyType),
    Add(Box<IntProperty>, Box<IntProperty>),
    Sub(Box<IntProperty>, Box<IntProperty>),
    Mul(Box<IntProperty>, Box<IntProperty>),
    Div(Box<IntProperty>, Box<IntProperty>)
}

impl IntProperty {
    pub fn get(&self, player: Option<&Player>, world: Option<&World>) -> Option<i32> {
        match self {
            IntProperty::Int(i) => return Some(*i),
            IntProperty::Player(prop) => {
                if let Some(p) = player {  
                    match prop {
                        PlayerPropertyType::X => return Some(p.x / 16),
                        PlayerPropertyType::Y => return Some(p.y / 16),
                        PlayerPropertyType::Height => return Some(p.layer)
                    }   
                } else {
                    return None;
                }
            },
            IntProperty::Flag(flag) => {
                if let Some(w) = world {
                    match flag {
                        FlagPropertyType::Global(f) => return Some(*w.global_flags.get(f).unwrap_or(&0)),
                        FlagPropertyType::Local(f) => return Some(*w.flags.get(f).unwrap_or(&0))
                    }
                } else {
                    return None;
                }
            },
            IntProperty::Level(prop) => {
                if let Some(w) = world {
                    match prop {
                        LevelPropertyType::DefaultX => return w.default_pos.map(|f| f.0),
                        LevelPropertyType::DefaultY => return w.default_pos.map(|f| f.1),
                        LevelPropertyType::TintA => return Some(w.tint.map_or(0, |c| c.a as i32)),
                        LevelPropertyType::TintR => return Some(w.tint.map_or(0, |c| c.r as i32)),
                        LevelPropertyType::TintG => return Some(w.tint.map_or(0, |c| c.g as i32)),
                        LevelPropertyType::TintB => return Some(w.tint.map_or(0, |c| c.b as i32)),
                        LevelPropertyType::BackgroundR => return Some(w.background_color.r as i32),
                        LevelPropertyType::BackgroundG => return Some(w.background_color.g as i32),
                        LevelPropertyType::BackgroundB => return Some(w.background_color.b as i32),
                        _ => return None
                    }
                }
                return None;
            },
            IntProperty::Add(a, b) => {
                let lhs = a.get(player, world);
                let rhs = b.get(player, world);
                if lhs.is_some() && rhs.is_some() {
                    return Some(lhs.unwrap() + rhs.unwrap());
                }

                return None;
            },
            IntProperty::Sub(a, b) => {
                let lhs = a.get(player, world);
                let rhs = b.get(player, world);
                if lhs.is_some() && rhs.is_some() {
                    //dbg(lhs.as_ref().unwrap() - rha);
                    return Some(lhs.unwrap() - rhs.unwrap());
                }

                return None;
            },
            IntProperty::Mul(a, b) => {
                let lhs = a.get(player, world);
                let rhs = b.get(player, world);
                if lhs.is_some() && rhs.is_some() {
                    return Some(lhs.unwrap() * rhs.unwrap());
                }

                return None;
            },
            IntProperty::Div(a, b) => {
                let lhs = a.get(player, world);
                let rhs = b.get(player, world);
                if lhs.is_some() && rhs.is_some() {
                    return Some(lhs.unwrap() / rhs.unwrap());
                }

                return None;
            },
        }
    }

    pub fn parse(json: &JsonValue) -> Option<Self> {
        if json.is_number() {
            return Some(IntProperty::Int(json.as_i32().unwrap()));
        }

        if !json["type"].is_string() { return None; }
        match json["type"].as_str().unwrap() {
            "int" => return Some(IntProperty::Int(json["val"].as_i32().unwrap())),
            "player" => return Some(IntProperty::Player(PlayerPropertyType::parse(&json["property"]).unwrap())),
            "level" => return Some(IntProperty::Level(LevelPropertyType::parse(&json["property"]).unwrap())),
            "flag" => {
                let mut global = false;
                if json["global"].is_boolean() {
                    global = json["global"].as_bool().unwrap();
                }
                let parsed_flag = json["flag"].as_str();
                if let Some(flag) = parsed_flag {
                    if global {
                        return Some(IntProperty::Flag(FlagPropertyType::Global(flag.to_string())))
                    } else {
                        return Some(IntProperty::Flag(FlagPropertyType::Local(flag.to_string())))
                    }
                }
                return None
            },
            "add" => {
                if !(json["lhs"].is_number() || json["lhs"].is_object()) || !(json["rhs"].is_number() || json["rhs"].is_object()) {
                    return None;
                }

                let left = IntProperty::parse(&json["lhs"]);
                let right = IntProperty::parse(&json["rhs"]);
                if left.is_some() && right.is_some() {
                    return Some(IntProperty::Add(Box::new(left.unwrap()), Box::new(right.unwrap())));
                }

                return None;
            },
            "sub" => {
                if !(json["lhs"].is_number() || json["lhs"].is_object()) || !(json["rhs"].is_number() || json["rhs"].is_object()) {
                    return None;
                }

                let left = IntProperty::parse(&json["lhs"]);
                let right = IntProperty::parse(&json["rhs"]);
                if left.is_some() && right.is_some() {
                    return Some(IntProperty::Sub(Box::new(left.unwrap()), Box::new(right.unwrap())));
                }

                return None;
            },
            "mul" => {
                if !(json["lhs"].is_number() || json["lhs"].is_object()) || !(json["rhs"].is_number() || json["rhs"].is_object()) {
                    return None;
                }

                let left = IntProperty::parse(&json["lhs"]);
                let right = IntProperty::parse(&json["rhs"]);
                if left.is_some() && right.is_some() {
                    return Some(IntProperty::Mul(Box::new(left.unwrap()), Box::new(right.unwrap())));
                }

                return None;
            },
            "div" => {
                if !(json["lhs"].is_number() || json["lhs"].is_object()) || !(json["rhs"].is_number() || json["rhs"].is_object()) {
                    return None;
                }

                let left = IntProperty::parse(&json["lhs"]);
                let right = IntProperty::parse(&json["rhs"]);
                if left.is_some() && right.is_some() {
                    return Some(IntProperty::Div(Box::new(left.unwrap()), Box::new(right.unwrap())));
                }

                return None;
            }
            _ => return None
        }
    }
}

pub enum StringProperty {
    String(String),
    FromInt(IntProperty)
}

impl StringProperty {
    pub fn get(&self, player: Option<&Player>, world: Option<&World>) -> Option<String> {
        match self {
            StringProperty::String(s) => return Some(s.clone()),
            StringProperty::FromInt(int) => {
                if let Some(i) = int.get(player, world) {
                    return Some(i.to_string());
                } else {
                    return None;
                }
            }
        }
    }

    pub fn parse(json: &JsonValue) -> Option<Self> {
        if json.is_string() {
            return Some(StringProperty::String(json.as_str().unwrap().to_string()));
        }
        if !json["type"].is_string() { return None; }
        match json["type"].as_str().unwrap() {
            "string" => return Some(StringProperty::String(json["val"].as_str().unwrap().to_string())),
            "from_int" => return Some(StringProperty::FromInt(IntProperty::parse(&json["val"]).unwrap())),
            _ => return None
        }
    }
}

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

    /// Physical screen dimensions
    pub screen_dims: (u32, u32),
    pub zoom: (f32, f32),

    /// Draw space screen dimensions (scaled)
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
    MusicOnly,
    Spotlight,
    FadeScreenshot,
    Spin,
    Zoom(f32),
    Pixelate,
    Lines(u32),
    Wave(bool, u32),
    //ZoomFade(f32)
}

impl TransitionType {
    pub fn parse(json: &JsonValue) -> Option<Self> {
        let kind;

        if json.is_string() {
            kind = json.as_str().unwrap();
        } else if json.is_object() {
            kind = json["type"].as_str().unwrap();
        } else {
            return None;
        }

        match kind {
            "fade" => Some(Self::Fade),
            "music_only" => Some(Self::MusicOnly),
            "spotlight" => Some(Self::Spotlight),
            "spin" => Some(Self::Spin),
            "zoom" => Some(Self::Zoom(1.0)),
            //"zoom_fade" => Some(Self::ZoomFade(1.0)),
            "pixelate" => Some(Self::Pixelate),
            "lines" => Some(Self::Lines(1)),
            "wave" => Some(Self::Wave(false, 10)),
            _ => None
        }
    }
}

pub struct TransitionTextures<'a> {
    pub spotlight: Texture<'a>
}

impl <'a> TransitionTextures<'a> {
    pub fn new<T>(creator: &'a TextureCreator<T>) -> Result<Self, String> {
        let spotlight = Texture::from_file(&PathBuf::from("res/textures/image/spotlight.png"), creator)?;
        Ok(Self {
                    spotlight
                })
    }

    pub fn empty<T>(creator: &'a TextureCreator<T>) -> Self {
        Self {
            spotlight: Texture::empty(creator)
        }
    }
}

#[derive(Clone)]
pub struct Transition {
    pub kind: TransitionType,
    pub progress: i32,
    pub direction: i32,
    pub speed: i32,
    pub fade_music: bool,
    pub hold: u32,
    pub hold_timer: u32,
    pub holding: bool,
    pub needs_screenshot: bool
}

impl Transition {
    pub fn new(kind: TransitionType, speed: i32, fade_music: bool, hold: u32) -> Self {
        let needs_screenshot = match &kind {
            TransitionType::FadeScreenshot | TransitionType::Spin | TransitionType::Lines(..) | TransitionType::Pixelate | TransitionType::Zoom(..) | TransitionType::Wave(..) => true,
            _ => false
        };

        Self {
            direction: 1,
            progress: 0,
            fade_music, kind, speed,
            hold, holding: false, hold_timer: hold,
            needs_screenshot
        }
    }

    pub fn parse(json: &JsonValue) -> Option<Self> {
        if json.is_string() {
            if let Some(transition_type) = TransitionType::parse(json) {
                return Some(Self::new(transition_type, 8, true, 0));
            } else {
                eprintln!("Error parsing transition: invalid transition type");
                return None;
            }
        } else if json.is_object() {
            if !json["type"].is_string() { return None; }
            let speed = json["speed"].as_i32().unwrap_or(8);
            let music = json["music"].as_bool().unwrap_or(true);
            let hold = json["hold"].as_u32().unwrap_or(0);
            if let Some(parsed_type) = TransitionType::parse(&json["type"]) {
                match parsed_type {
                    TransitionType::Zoom(..) => {
                        return Some(
                            Self::new(TransitionType::Zoom(json["scale"].as_f32().unwrap_or(1.0)), speed, music, hold)
                        )
                    },
                    TransitionType::Lines(..) => {
                        return Some(
                            Self::new(TransitionType::Lines(json["height"].as_u32().unwrap_or(1)), speed, music, hold)
                        )
                    },
                    TransitionType::Wave(..) => {
                        let direction = if json["dir"].is_string() {
                            match json["dir"].as_str().unwrap() {
                                "up" | "down" | "vert" | "vertical" | "y" => true,
                                _ => false
                            }
                        } else if json["dir"].is_boolean() {
                            json["dir"].as_bool().unwrap()
                        } else {
                            false
                        };

                        return Some(
                            Self::new(TransitionType::Wave(direction, json["waves"].as_u32().unwrap_or(10)), speed, music, hold)
                        )
                    }
                    _ => return Some(Self::new(parsed_type, speed, music, hold))
                }
            } else {
                eprintln!("Error parsing transition: invalid transition type");
                return None;
            }
        } else {
            return None;
        }
    }

    pub fn draw<T: RenderTarget>(&mut self, canvas: &mut Canvas<T>, world: &mut World) {
        if self.needs_screenshot {
            world.transition_context.take_screenshot = true;
            self.needs_screenshot = false;
            return;
        }

        match self.kind {
            TransitionType::Fade => {
                let alpha = (255.0 * (self.progress as f32 / 100.0)).clamp(0.0, 255.0) as u8;
                canvas.set_blend_mode(sdl2::render::BlendMode::Blend);
                canvas.set_draw_color(Color::RGBA(0, 0, 0, alpha));
                canvas.fill_rect(None).unwrap();
            },
            TransitionType::MusicOnly => (),
            TransitionType::Spotlight => {
                let alpha = (255.0 * (self.progress as f32 / 50.0)).clamp(0.0, 255.0) as u8;
                canvas.set_blend_mode(sdl2::render::BlendMode::Blend);
                let alpha_mod = world.transitions.spotlight.texture.alpha_mod();
                world.transitions.spotlight.texture.set_alpha_mod(alpha);
                canvas.copy(&world.transitions.spotlight.texture, None, None).unwrap();
                world.transitions.spotlight.texture.set_alpha_mod(alpha_mod);

                if self.progress > 50 {
                    let fill_alpha = (255.0 * ((self.progress as f32 - 50.0) / 50.0)).clamp(0.0, 255.0) as u8;
                    canvas.set_draw_color(Color::RGBA(0, 0, 0, fill_alpha));
                    canvas.fill_rect(None).unwrap();
                }
            },
            TransitionType::FadeScreenshot => {
                canvas.set_blend_mode(sdl2::render::BlendMode::None);
                if let Some(screenshot) = &world.transition_context.screenshot {
                    canvas.set_draw_color(Color::RGBA(255, 0, 0, 255));
                    canvas.fill_rect(None).unwrap();
                    canvas.copy(&screenshot, None, None).unwrap();
                }
                
                let alpha = (255.0 * (self.progress as f32 / 100.0)).clamp(0.0, 255.0) as u8;
                canvas.set_blend_mode(sdl2::render::BlendMode::Blend);
                canvas.set_draw_color(Color::RGBA(0, 0, 0, alpha));
                canvas.fill_rect(None).unwrap();
            }
            TransitionType::Spin => {
                let progress = if self.direction == -1 {
                    100 - self.progress
                } else {
                    self.progress
                };
                let angle = 360.0 * (progress as f64 / 100.0);
                if let Some(screenshot) = &world.transition_context.screenshot {
                    canvas.set_blend_mode(sdl2::render::BlendMode::Blend);
                    canvas.set_draw_color(Color::RGBA(0, 0, 0, 255));
                    canvas.fill_rect(None).unwrap();
                    canvas.set_blend_mode(sdl2::render::BlendMode::None);
                    canvas.copy_ex(&screenshot, None, None, angle, None, false, false).unwrap();
                }
            },
            TransitionType::Zoom(scale) => {
                let progress_x = ((self.progress * 4) as f32 * scale) as i32;
                let progress_y = ((self.progress * 3) as f32 * scale) as i32;
                let dest = Rect::new(
                    0 - progress_x, 
                    0 - progress_y,
                    (400 + progress_x * 2) as u32, 
                    (300 + progress_y * 2) as u32
                );
                if let Some(screenshot) = &world.transition_context.screenshot {
                    canvas.set_blend_mode(sdl2::render::BlendMode::Blend);
                    canvas.set_draw_color(Color::RGBA(0, 0, 0, 255));
                    canvas.fill_rect(None).unwrap();
                    canvas.set_blend_mode(sdl2::render::BlendMode::None);
                    canvas.copy_ex(&screenshot, None, dest, 0.0, None, false, false).unwrap();
                }
            },
            TransitionType::Lines(height) => {
                let offset = (400.0 * (self.progress as f32 / 100.0)) as i32;
                if let Some(screenshot) = &world.transition_context.screenshot {
                    canvas.set_blend_mode(sdl2::render::BlendMode::Blend);
                    canvas.set_draw_color(Color::RGBA(0, 0, 0, 255));
                    canvas.fill_rect(None).unwrap();
                    canvas.set_blend_mode(sdl2::render::BlendMode::None);
                    for i in 0..(300 / height as i32) {
                        // laggy?
                        let src = Rect::new(0, i * height as i32, 400, height);
                        let dst = Rect::new(if i % 2 == 0 { offset } else { -offset }, i * height as i32, 400, height);
                        canvas.copy(&screenshot, src, dst).unwrap();
                    }
                }
            },
            TransitionType::Pixelate => {
                let pixelation_factor = self.progress.max(1);

                if let Some(screenshot) = &world.transition_context.screenshot {
                    canvas.set_blend_mode(sdl2::render::BlendMode::Blend);
                    canvas.set_draw_color(Color::RGBA(0, 0, 0, 255));
                    canvas.fill_rect(None).unwrap();
                    canvas.set_blend_mode(sdl2::render::BlendMode::None);
                    for y in 0..(300 / pixelation_factor) {
                        for x in 0..(400 / pixelation_factor) {
                            let src = Rect::new(x * pixelation_factor, y * pixelation_factor, 1, 1);
                            let dst = Rect::new(x * pixelation_factor, y * pixelation_factor, pixelation_factor as u32, pixelation_factor as u32);
                            canvas.copy(&screenshot, src, dst).unwrap();
                        }
                    }
                }
            },
            TransitionType::Wave(dir, waves) => {
                let progress = (200.0 * (self.progress as f32 / 100.0)) as i32;

                if let Some(screenshot) = &world.transition_context.screenshot {
                    canvas.set_blend_mode(sdl2::render::BlendMode::Blend);
                    canvas.set_draw_color(Color::RGBA(0, 0, 0, 255));
                    canvas.fill_rect(None).unwrap();
                    canvas.set_blend_mode(sdl2::render::BlendMode::None);
                    if !dir {
                        for y in 0..300 {
                            let sin = ((y as f32 / 300.0 * PI * (waves as f32)).sin() * progress as f32) as i32;
                            let src = Rect::new(0, y, 400, 1);
                            let dst = Rect::new(sin, y, 400, 1);
                            canvas.copy(&screenshot, src, dst).unwrap();
                        }
                    } else {
                        for x in 0..400 {
                            let sin = ((x as f32 / 400.0 * PI * (waves as f32)).sin() * progress as f32) as i32;
                            let src = Rect::new(x, 0, 1, 300);
                            let dst = Rect::new(x, sin, 1, 300);
                            canvas.copy(&screenshot, src, dst).unwrap();
                        }
                    }
                }
            }
        }
    }

    //pub fn parse()
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
        "give_effect" => {
            return GiveEffectAction::parse(parsed);
        },
        "set_flag" => {
            return SetFlagAction::parse(parsed);
        },
        "conditional" => {
            return ConditionalAction::parse(parsed);
        },
        "play" => {
            return PlaySoundAction::parse(parsed);
        },
        "set" => {
            return SetPropertyAction::parse(parsed);
        },
        "change_song" => {
            return ChangeSongAction::parse(parsed);
        }
        _ => {
            return Err(format!("Unknown action \"{}\"", parsed["type"].as_str().unwrap()));
        }
    }
}

pub trait Action {
    fn act(&self, player: &mut Player, world: &mut World);
}

#[deprecated = "use WarpPos"]
#[derive(Clone)]
pub enum WarpCoord {
    Match,
    Pos(i32),
    Sub(i32),
    Add(i32),
    Default
}

#[derive(Clone)]
pub struct WarpPos {
    pub x: IntProperty,
    pub y: IntProperty
}

impl WarpPos {
    pub fn parse(json: &JsonValue) -> Option<Self> {
        if !(json["x"].is_object() || json["x"].is_number()) || !(json["y"].is_object() || json["y"].is_number()) { return None; } 
        let x_parsed = IntProperty::parse(&json["x"]);
        let y_parsed = IntProperty::parse(&json["y"]);
        if x_parsed.is_some() && y_parsed.is_some() {
            return Some(Self {
                x: x_parsed.unwrap(),
                y: y_parsed.unwrap()
            });
        }

        return None;
    }
}

pub struct QueuedLoad {
    pub map: String,
    pub pos: WarpPos,
}

pub struct WarpAction {
    pub map: Option<String>,
    pub exit: WarpPos,
    pub transition: Option<Transition>
}

pub struct DelayedAction {
    pub after: Box<dyn Action>,
    pub delay: u32
}

pub struct FreezeAction {
    pub time: Option<u32>
}

pub struct GiveEffectAction {
    pub effect: String,
}

pub struct SetFlagAction {
    pub global: bool,
    pub flag: String,
    pub value: IntProperty
}

pub struct ConditionalAction {
    pub inner: Box<dyn Action>,
    pub condition: Condition
}

pub struct PlaySoundAction {
    pub sound: String,
    pub volume: f32,
    pub speed: f32
}

pub enum PropertyLocation {
    Player(PlayerPropertyType),
    World(LevelPropertyType)
}

pub struct SetPropertyAction {
    pub property: PropertyLocation,
    pub val: JsonValue
}

pub struct ChangeSongAction {
    pub new_song: Option<StringProperty>,
    pub song_speed: Option<FloatProperty>,
    pub song_volume: Option<FloatProperty>,
    pub set_defaults: BoolProperty
}

impl WarpAction {
    pub fn parse(parsed: &JsonValue) -> Result<Box<dyn Action>, String> {
        let mut map = None;
        let transition;
        //let mut transition_type = None;

        // Map
        if parsed["map"].is_string() {
            map = Some(parsed["map"].as_str().unwrap());
        }

        // Transition
        transition = Transition::parse(&parsed["transition"]);

        // Pos
        if !parsed["pos"].is_object() { return Err("Invalid or missing position".to_string()); }
        if !(parsed["pos"]["x"].is_object() || parsed["pos"]["x"].is_number()) { return Err("Missing x position".to_string()); }
        if !(parsed["pos"]["y"].is_object() || parsed["pos"]["y"].is_number()) { return Err("Missing y position".to_string()); }
        let parsed_x = IntProperty::parse(&parsed["pos"]["x"]);
        let parsed_y = IntProperty::parse(&parsed["pos"]["y"]);
        if parsed_x.is_none() { return Err("failed to parse x coord".to_string()); }
        if parsed_y.is_none() { return Err("failed to parse y coord".to_string()); }
        let pos = WarpPos {
            x: parsed_x.unwrap(),
            y: parsed_y.unwrap()
        };

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

impl ConditionalAction {
    pub fn parse(parsed: &JsonValue) -> Result<Box<dyn Action>, String> {
        if !parsed["condition"].is_object() { return Err("No condition specified for conditional action".to_string()); }
        if !parsed["action"].is_object() { return Err("No action specified for conditional action".to_string()); }
        let parsed_action = parse_action(&parsed["action"]);
        let parsed_condition = Condition::parse(&parsed["condition"]);
        if parsed_action.is_ok() && parsed_condition.is_some() {
            return Ok(
                Box::new(ConditionalAction {
                    condition: parsed_condition.unwrap(),
                    inner: parsed_action.unwrap()
                })
            );
        }

        if parsed_action.is_err() {
            return parsed_action;
        }
        if parsed_condition.is_none() {
            return Err("Condition failed to parse".to_string());
        }
        
        return Err("Unknown error in parsing conditional action".to_string());
    }
}

impl FreezeAction {
    pub fn parse(parsed: &JsonValue) -> Result<Box<dyn Action>, String> {
        let mut time = None;
        if parsed["time"].is_number() {
            time = parsed["time"].as_u32()
        }
        return Ok(Box::new(FreezeAction {
            time
        }));
    }
}

impl GiveEffectAction {
    pub fn parse(parsed: &JsonValue) -> Result<Box<dyn Action>, String> {
        if parsed["effect"].is_string() {
            return Ok(Box::new(GiveEffectAction {
                effect: parsed["effect"].as_str().unwrap().to_string()
            }));
        }

        Err("No effect specified for action".to_string())
    }
}

impl SetFlagAction {
    pub fn parse(parsed: &JsonValue) -> Result<Box<dyn Action>, String> {
        let mut global = false;
        if parsed["global"].is_boolean() { global = parsed["global"].as_bool().unwrap(); }

        if parsed["flag"].is_string() {
            if parsed["value"].is_number() {
                return Ok(Box::new(SetFlagAction {
                    flag: parsed["flag"].as_str().unwrap().to_string(),
                    global,
                    value: IntProperty::Int(parsed["value"].as_i32().unwrap())
                }));
            } else if parsed["value"].is_object() {
                let parsed_property = IntProperty::parse(&parsed["value"]);
                if let Some(property) = parsed_property {
                    return Ok(Box::new(SetFlagAction {
                        flag: parsed["flag"].as_str().unwrap().to_string(),
                        global,
                        value: property
                    }));
                } else {
                    return Err("Could not parse property for flag".to_string());
                }

            } else {
                return Err(String::from("Bad value for flag"));
            }
        } else {
            return Err(String::from("No flag specified"));
        }
    }
}

impl PlaySoundAction {
    pub fn parse(parsed: &JsonValue) -> Result<Box<dyn Action>, String> {
        if !parsed["sound"].is_string() {
            return Err("No sound specified for play action".to_string());
        }

        return Ok(
            Box::new(Self {
                            sound: parsed["sound"].as_str().unwrap().to_string(),
                            speed: parsed["speed"].as_f32().unwrap_or(1.0),
                            volume: parsed["volume"].as_f32().unwrap_or(1.0)
                        })
        )
    }
}

impl SetPropertyAction {
    pub fn parse(parsed: &JsonValue) -> Result<Box<dyn Action>, String> {
        if !parsed["in"].is_string() {
            return Err("no location for set action".to_string());
        }
        if !parsed["val"].is_string() {
            return Err("no target value for set action".to_string());
        }
        if parsed["to"].is_null() {
            return Err("no value for set action".to_string());
        }

        let mut location = None;
        
        match parsed["in"].as_str().unwrap() {
            "player" => {
                location = Some(PropertyLocation::Player(PlayerPropertyType::parse(&parsed["val"]).unwrap()));
            },
            "world" => {
                location = Some(PropertyLocation::World(LevelPropertyType::parse(&parsed["val"]).unwrap()));
            }
            _ => return Err("invalid target for set action".to_string())
        }

        if location.is_some() {
            return Ok(
                Box::new(SetPropertyAction {
                                    property: location.unwrap(),
                                    val: parsed["to"].clone()
                                })
            )
        }

        return Err(String::new());
    }
}

impl ChangeSongAction {
    pub fn parse(parsed: &JsonValue) -> Result<Box<dyn Action>, String> {
        let mut new_volume = None;
        let mut new_speed = None;
        let mut new_song = None;
        let mut set_defaults = BoolProperty::Bool(false);

        if !parsed["volume"].is_null() { new_volume = FloatProperty::parse(&parsed["volume"]); }
        if !parsed["speed"].is_null() { new_speed = FloatProperty::parse(&parsed["volume"]); }
        if !parsed["song"].is_null() { new_song = StringProperty::parse(&parsed["song"]); }
        if !parsed["set_defaults"].is_null() { set_defaults = BoolProperty::parse(&parsed["set_defaults"]).expect("failed to parse set_defaults"); }

        Ok(Box::new(Self {
                    new_song,
                    song_speed: new_speed,
                    song_volume: new_volume,
                    set_defaults
                }))
    }
}

impl Action for FreezeAction {
    fn act(&self, player: &mut Player, _world: &mut World) {
        if let Some(time) = self.time {
            player.frozen_time = time;
        } else {
            player.frozen = true;
        }
    }
}

impl Action for WarpAction {
    fn act(&self, _player: &mut Player, world: &mut World) {
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

impl Action for ConditionalAction {
    fn act(&self, player: &mut Player, world: &mut World) {
        if self.condition.evaluate(Some(player), Some(world)) {
            self.inner.act(player, world);
        }
    }
}

impl Action for GiveEffectAction {
    fn act(&self, player: &mut Player, world: &mut World) {
        if let Some(effect) = Effect::parse(self.effect.as_str()) {
            if !player.has_effect(&effect) {
                world.special_context.effect_get = Some(effect);
            }
        }
    }
}

impl Action for SetFlagAction {
    fn act(&self, player: &mut Player, world: &mut World) {
        let value_opt = self.value.get(Some(player), Some(world));
        
        if let Some(value) = value_opt {
            if self.global {
                world.global_flags.insert(self.flag.clone(), value);
            } else {
                world.flags.insert(self.flag.clone(), value);
            }
        }
    }
}

impl Action for PlaySoundAction {
    fn act(&self, _: &mut Player, world: &mut World) {
        world.special_context.play_sounds.push((self.sound.clone(), self.speed, self.volume));
    }
}

impl Action for SetPropertyAction {
    fn act(&self, player: &mut Player, world: &mut World) {
        match &self.property {
            PropertyLocation::Player(prop) => {
                match prop {
                    PlayerPropertyType::Height => { player.layer = IntProperty::parse(&self.val).unwrap().get(Some(&player), Some(&world)).unwrap() },
                    PlayerPropertyType::X => { player.set_x(IntProperty::parse(&self.val).unwrap().get(Some(&player), Some(&world)).unwrap()) },
                    PlayerPropertyType::Y => { player.set_y(IntProperty::parse(&self.val).unwrap().get(Some(&player), Some(&world)).unwrap()) }
                }
            },
            PropertyLocation::World(prop) => {
                match prop {
                    LevelPropertyType::DefaultX => { if world.default_pos.is_some() { world.default_pos.as_mut().unwrap().0 = IntProperty::parse(&self.val).unwrap().get(Some(&player), Some(&world)).unwrap(); } },
                    LevelPropertyType::DefaultY => { if world.default_pos.is_some() { world.default_pos.as_mut().unwrap().1 = IntProperty::parse(&self.val).unwrap().get(Some(&player), Some(&world)).unwrap(); } },
                    LevelPropertyType::TintA => { if world.tint.is_some() { world.tint.as_mut().unwrap().a = IntProperty::parse(&self.val).unwrap().get(Some(&player), Some(&world)).unwrap().clamp(0, 255) as u8 } },
                    LevelPropertyType::TintR => { if world.tint.is_some() { world.tint.as_mut().unwrap().r = IntProperty::parse(&self.val).unwrap().get(Some(&player), Some(&world)).unwrap().clamp(0, 255) as u8 } },
                    LevelPropertyType::TintG => { if world.tint.is_some() { world.tint.as_mut().unwrap().g = IntProperty::parse(&self.val).unwrap().get(Some(&player), Some(&world)).unwrap().clamp(0, 255) as u8 } },
                    LevelPropertyType::TintB => { if world.tint.is_some() { world.tint.as_mut().unwrap().b = IntProperty::parse(&self.val).unwrap().get(Some(&player), Some(&world)).unwrap().clamp(0, 255) as u8 } },
                    LevelPropertyType::BackgroundB => { world.background_color.b = IntProperty::parse(&self.val).unwrap().get(Some(&player), Some(&world)).unwrap().clamp(0, 255) as u8 },
                    LevelPropertyType::BackgroundG => { world.background_color.g = IntProperty::parse(&self.val).unwrap().get(Some(&player), Some(&world)).unwrap().clamp(0, 255) as u8 },
                    LevelPropertyType::BackgroundR => { world.background_color.r = IntProperty::parse(&self.val).unwrap().get(Some(&player), Some(&world)).unwrap().clamp(0, 255) as u8 },
                    LevelPropertyType::Paused => { world.paused = BoolProperty::parse(&self.val).unwrap().get(Some(&player), Some(&world)).unwrap() },
                    LevelPropertyType::SpecialSaveGame => { world.special_context.save_game = BoolProperty::parse(&self.val).unwrap().get(Some(&player), Some(&world)).unwrap() },
                }
            }
        }
    }
}

impl Action for ChangeSongAction {
    fn act(&self, player: &mut Player, world: &mut World) {
        if let Some(path) = &self.new_song {
            world.song = Some(Song::new(PathBuf::from(path.get(Some(player), Some(world)).expect("Error in getting song path"))));
            world.song.as_mut().unwrap().dirty = true;
        }
        let mut current_song_opt = world.song.take();
        if let Some(current_song) = &mut current_song_opt {
            if let Some(new_speed) = &self.song_speed {
                let new_speed_get = new_speed.get(Some(player), Some(world)).unwrap();
                current_song.speed = new_speed_get;
                if self.set_defaults.get(Some(player), Some(world)).unwrap() { current_song.default_speed = new_speed_get; }
                current_song.dirty = true;
            }
            if let Some(new_volume) = &self.song_volume {
                let new_volume_get = new_volume.get(Some(player), Some(world)).unwrap();
                current_song.volume = new_volume_get;
                if self.set_defaults.get(Some(player), Some(world)).unwrap() { current_song.default_volume = new_volume_get; }
                current_song.dirty = true;
            }  
        }
        world.song = current_song_opt;
    }
}

pub struct PrintAction {
    pub message: StringProperty,
}

impl PrintAction {
    pub fn parse(parsed: &JsonValue) -> Result<Box<dyn Action>, String> {
        if parsed["message"].is_string() {
            return Ok(Box::new(Self {
                message: StringProperty::String(parsed["message"].as_str().unwrap().to_string())
            }));
        } else if parsed["message"].is_object() {
            let parsed = StringProperty::parse(&parsed["message"]);
            if let Some(message) = parsed {
                return Ok(Box::new(Self {
                    message
                }))
            }
        }

        return Err("Invalid message for print".to_string());
    }
}

impl Action for PrintAction {
    fn act(&self, player: &mut Player, world: &mut World) {
        println!("{}", self.message.get(Some(player), Some(world)).unwrap());
    }
}