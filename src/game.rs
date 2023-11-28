use std::{collections::HashMap, str::FromStr, path::PathBuf, f32::consts::PI};

use json::JsonValue;
use rand::{prelude::Distribution, distributions::Standard};
use sdl2::{keyboard::Keycode, render::{Canvas, RenderTarget, TextureCreator}, pixels::Color, rect::Rect};

use crate::{player::Player, world::{World, QueuedEntityAction}, effect::Effect, texture::Texture, audio::Song, entity::VariableValue};

pub fn offset_floor(n: i32, to: i32, offset: i32) -> i32 {
    (n as f32 / to as f32).floor() as i32 * to + (offset.abs() % to)
}

pub fn offset_ceil(n: i32, to: i32, offset: i32) -> i32 {
    (n as f32 / to as f32).ceil() as i32 * to - (offset.abs() % to)
}

pub fn ceil(n: i32, to: i32) -> i32 {
    (n as f32 / to as f32).ceil() as i32 * to
}

#[derive(Clone)]
pub enum Condition {
    IntEquals(IntProperty, IntProperty),
    IntGreater(IntProperty, IntProperty),
    IntLess(IntProperty, IntProperty),
    StringEquals(StringProperty, StringProperty),
    EffectEquipped(Effect),
    Negate(Box<Condition>),
    Bool(Box<BoolProperty>),
    Variable(Box<StringProperty>)
}

// fn try_get_variable<'a>(world: &'a World, name: &String) -> Option<&'a VariableValue> {
//     if let Some(variables_list) = &world.special_context.entity_context.entity_variables {
//         return variables_list.borrow().get(name);
//     }

//     None
// }

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
            },
            Self::Bool(bool) => {
                bool.get(player, world).unwrap_or(false)
            },
            Self::Variable(name) => {
                if let Some(world) = world {
                    if let Some(name) = name.get(player, Some(world)) {
                        if world.special_context.entity_context.entity_call {
                            if let Some(variables_list) = &world.special_context.entity_context.entity_variables {
                                if let Some(variable) = variables_list.borrow().get(&name) {
                                    if variable.is_bool() {
                                        return variable.as_bool(Some(world), player).unwrap_or(false);
                                    }
                                } else {
                                    eprintln!("Warning: Variable {} not found", &name);
                                }
                            }
                        } else {
                            eprintln!("Warning: Variable get called outside of entity context (as condition)");
                        }
                    }
                }

                return false;
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
                if lhs_parsed.is_ok() && rhs_parsed.is_ok() {
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
            },
            "bool" => {
                if json["val"].is_null() { return None; }
                
                if let Some(bool) = BoolProperty::parse(&json["val"]) {
                    return Some(Self::Bool(Box::new(bool)));
                }

                return None;
            },
            "variable" | "var" => {
                if json["name"].is_null() { return None; }

                if let Ok(name) = StringProperty::parse(&json["name"]) {
                    return Some(Self::Variable(Box::new(name)));
                }

                return None;
            }
            _ => return None,
        }
    }
}

#[derive(Clone)]
pub enum EntityPropertyType {
    X,
    Y,
    ID
}

impl EntityPropertyType {
    pub fn parse(json: &JsonValue) -> Option<Self> {
        let kind = if json.is_string() {
            json.as_str().unwrap()
        } else {
            if !json["type"].is_string() { return None; }
            json["type"].as_str().unwrap()
        };

        match kind {
            "x" => Some(EntityPropertyType::X),
            "y" => Some(EntityPropertyType::Y),
            "id" => Some(EntityPropertyType::ID),
            _ => None
        }
    }
}

#[derive(Clone)]
pub enum PlayerPropertyType {
    X,
    Y,
    Height,
    Dreaming
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
            "dreaming" => Some(PlayerPropertyType::Dreaming),
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
    Global(Box<StringProperty>),
    Local(Box<StringProperty>)
}

#[derive(Clone)]
pub enum BoolProperty {
    Bool(bool),
    Player(PlayerPropertyType),
    Level(LevelPropertyType),
    And(Box<BoolProperty>, Box<BoolProperty>),
    Or(Box<BoolProperty>, Box<BoolProperty>),
    Not(Box<BoolProperty>),
    Xor(Box<BoolProperty>, Box<BoolProperty>),
    FromCondition(Box<Condition>),
    Variable(Box<StringProperty>)
}

impl BoolProperty {
    pub fn get(&self, player: Option<&Player>, world: Option<&World>) -> Option<bool> {
        match self {
            BoolProperty::Bool(b) => return Some(*b),
            BoolProperty::Player(prop) => {
                if let Some(p) = player {
                    match prop {
                        PlayerPropertyType::Dreaming => return Some(p.dreaming),
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
            },
            BoolProperty::Variable(name) => {
                if let Some(world) = world {
                    if let Some(name) = name.get(player, Some(world)) {
                        if world.special_context.entity_context.entity_call {
                            if let Some(variables_list) = &world.special_context.entity_context.entity_variables {
                                if let Some(variable) = variables_list.borrow().get(&name) {
                                    if variable.is_bool() {
                                        return variable.as_bool(Some(world), player);
                                    }
                                }
                            }
                        } else {
                            eprintln!("Warning: Variable get called outside of entity context");
                        }
                    }
                }

                return None;
            },
            BoolProperty::FromCondition(condition) => {
                return Some(condition.evaluate(player, world));
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
            "variable" | "var" => {
                if !json["name"].is_null() {
                    if let Ok(name) = StringProperty::parse(&json["name"]) {
                        return Some(Self::Variable(Box::new(name)));
                    }
                }

                return None;
            },
            "condition" | "from_condition" | "conditional" => {
                if !json["condition"].is_null() {
                    if let Some(cond) = Condition::parse(&json["condition"]) {
                        return Some(Self::FromCondition(Box::new(cond)));
                    }
                }

                return None;
            }
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
    Div(Box<FloatProperty>, Box<FloatProperty>),
    Variable(Box<StringProperty>)
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
            },
            FloatProperty::Variable(name) => {
                if let Some(world) = world {
                    if let Some(name) = name.get(player, Some(world)) {
                        if world.special_context.entity_context.entity_call {
                            if let Some(variables_list) = &world.special_context.entity_context.entity_variables {
                                if let Some(variable) = variables_list.borrow().get(&name) {
                                    if variable.is_float() {
                                        return variable.as_f32(Some(world), player);
                                    }
                                }
                            }
                        } else {
                            eprintln!("Warning: Variable get called outside of entity context");
                        }
                    }
                }

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
            "variable" | "var" => {
                if !json["name"].is_null() {
                    if let Ok(name) = StringProperty::parse(&json["name"]) {
                        return Some(Self::Variable(Box::new(name)));
                    }
                }

                return None;
            }
            _ => return None,
        }
    }
}

#[derive(Clone)]
pub enum IntProperty {
    Int(i32),
    Entity(EntityPropertyType),
    Player(PlayerPropertyType),
    Flag(FlagPropertyType),
    Level(LevelPropertyType),
    Add(Box<IntProperty>, Box<IntProperty>),
    Sub(Box<IntProperty>, Box<IntProperty>),
    Mul(Box<IntProperty>, Box<IntProperty>),
    Div(Box<IntProperty>, Box<IntProperty>),
    Variable(Box<StringProperty>)
}

impl IntProperty {
    pub fn get(&self, player: Option<&Player>, world: Option<&World>) -> Option<i32> {
        match self {
            IntProperty::Int(i) => return Some(*i),
            IntProperty::Entity(prop) => {
                if let Some(world) = world {
                    if world.special_context.entity_context.entity_call {
                        match prop {
                            EntityPropertyType::ID => return Some(world.special_context.entity_context.id),
                            EntityPropertyType::X => return Some(world.special_context.entity_context.x),
                            EntityPropertyType::Y => return Some(world.special_context.entity_context.y),
                            _ => return None
                        }
                    }
                }

                return None
            },
            IntProperty::Player(prop) => {
                if let Some(p) = player {  
                    match prop {
                        PlayerPropertyType::X => return Some(p.x / 16),
                        PlayerPropertyType::Y => return Some(p.y / 16),
                        PlayerPropertyType::Height => return Some(p.layer),
                        _ => return None
                    }   
                } else {
                    return None;
                }
            },
            IntProperty::Flag(flag) => {
                if let Some(w) = world {
                    match flag {
                        FlagPropertyType::Global(f) => return Some(*w.global_flags.get(f.get(player, world).unwrap().as_str()).unwrap_or(&0)),
                        FlagPropertyType::Local(f) => return Some(*w.flags.get(f.get(player, world).unwrap().as_str()).unwrap_or(&0))
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
            IntProperty::Variable(name) => {
                if let Some(world) = world {
                    if let Some(name) = name.get(player, Some(world)) {
                        if world.special_context.entity_context.entity_call {
                            if let Some(variables_list) = &world.special_context.entity_context.entity_variables {
                                if let Some(variable) = variables_list.borrow().get(&name) {
                                    if variable.is_int() {
                                        return variable.as_i32(Some(world), player);
                                    }
                                }
                            }
                        } else {
                            eprintln!("Warning: Variable get called outside of entity context");
                        }
                    }
                }

                return None;
            }
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
            "entity" => return Some(IntProperty::Entity(EntityPropertyType::parse(&json["property"]).unwrap())),
            "level" => return Some(IntProperty::Level(LevelPropertyType::parse(&json["property"]).unwrap())),
            "flag" => {
                let mut global = false;
                if json["global"].is_boolean() {
                    global = json["global"].as_bool().unwrap();
                }

                let flag_name = if json["flag"].is_string() {
                    let string = json["flag"].as_str();
                    if let Some(s) = string {
                        Some(StringProperty::String(s.to_string()))
                    } else {
                        None
                    }
                } else {
                    StringProperty::parse(&json["flag"]).map_or(None, |v| { Some(v) })
                };
                if let Some(flag) = flag_name {
                    if global {
                        return Some(IntProperty::Flag(FlagPropertyType::Global(Box::new(flag))))
                    } else {
                        return Some(IntProperty::Flag(FlagPropertyType::Local(Box::new(flag))))
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
            },
            "variable" | "var" => {
                if json["name"].is_null() {
                    return None;
                }

                if let Ok(name) = StringProperty::parse(&json["name"]) {
                    return Some(IntProperty::Variable(Box::new(name)));
                }

                return None;
            }
            _ => return None
        }
    }
}

#[derive(Clone)]
pub enum StringProperty {
    String(String),
    FromInt(IntProperty),
    Concatenate(Box<StringProperty>, Box<StringProperty>),
    Variable(Box<StringProperty>),
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
            },
            StringProperty::Concatenate(l, r) => {
                let l = l.get(player, world);
                let r = r.get(player, world);
                if l.is_some() && r.is_some() {
                    let mut left = l.unwrap();
                    left.extend(r.unwrap().chars());
                    return Some(left);
                } else {
                    return None;
                }
            },
            StringProperty::Variable(name) => {
                if let Some(world) = world {
                    if let Some(name) = name.get(player, Some(world)) {
                        if world.special_context.entity_context.entity_call {
                            if let Some(variables_list) = &world.special_context.entity_context.entity_variables {
                                if let Some(variable) = variables_list.borrow().get(&name) {
                                    if variable.is_string() {
                                        return variable.as_string(Some(world), player);
                                    }
                                } else {
                                    eprintln!("Warning: variable not found: {}", &name);
                                }
                            }
                        } else {
                            eprintln!("Warning: Variable get called outside of entity context");
                        }
                    }
                }

                return None;
            }
        }
    }

    pub fn parse(json: &JsonValue) -> Result<Self, String> {
        if json.is_string() {
            return Ok(StringProperty::String(json.as_str().unwrap().to_string()));
        }
        if !json["type"].is_string() { return Err("no type for string property".to_string()); }
        match json["type"].as_str().unwrap() {
            "string" => return Ok(StringProperty::String(json["val"].as_str().unwrap().to_string())),
            "from_int" => return Ok(StringProperty::FromInt(IntProperty::parse(&json["val"]).unwrap())),
            "concatenate" => return Ok(StringProperty::Concatenate(Box::new(StringProperty::parse(&json["lhs"]).unwrap()), Box::new(StringProperty::parse(&json["rhs"]).unwrap()))),
            "variable" | "var" => {
                if !json["name"].is_null() {
                    if let Ok(name) = StringProperty::parse(&json["name"]) {
                        return Ok(StringProperty::Variable(Box::new(name)));
                    } else {
                        return Err("Could not parse name field of string variable get".to_string());
                    }
                }

                return Err("No name specified for variable get".to_string());
            }
            _ => return Err("unknown type for string property".to_string())
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
    pub clamp: (bool, bool),
    pub fullscreen: bool
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
            clamp: (false, false),
            fullscreen: false
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
    GridCycle
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
            "grid_cycle" => Some(Self::GridCycle),
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
    pub needs_screenshot: bool,
    pub delay: i32,
    pub delay_timer: i32,
}

impl Transition {
    pub fn new(kind: TransitionType, speed: i32, delay: i32, fade_music: bool, hold: u32) -> Self {
        let needs_screenshot = match &kind {
            TransitionType::FadeScreenshot | TransitionType::Spin | TransitionType::Lines(..) | TransitionType::Pixelate | TransitionType::Zoom(..) | TransitionType::Wave(..) => true,
            _ => false
        };

        Self {
            direction: 1,
            progress: 0,
            fade_music, kind, speed,
            hold, holding: false, hold_timer: hold,
            needs_screenshot,
            delay, delay_timer: 0
        }
    }

    pub fn parse(json: &JsonValue) -> Option<Self> {
        if json.is_string() {
            if let Some(transition_type) = TransitionType::parse(json) {
                return Some(Self::new(transition_type, 8, 0, true, 0));
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
                            Self::new(TransitionType::Zoom(json["scale"].as_f32().unwrap_or(1.0)), speed, 0, music, hold)
                        )
                    },
                    TransitionType::Lines(..) => {
                        return Some(
                            Self::new(TransitionType::Lines(json["height"].as_u32().unwrap_or(1)), speed, 0, music, hold)
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
                            Self::new(TransitionType::Wave(direction, json["waves"].as_u32().unwrap_or(10)), speed, 0, music, hold)
                        )
                    }, TransitionType::GridCycle => {
                        return Some(
                            Self::new(TransitionType::GridCycle, speed, 0, music, hold)
                        );
                    }
                    _ => return Some(Self::new(parsed_type, speed, 0, music, hold))
                }
            } else {
                eprintln!("Error parsing transition: invalid transition type");
                return None;
            }
        } else {
            return None;
        }
    }

    pub fn draw<T: RenderTarget>(&mut self, canvas: &mut Canvas<T>, world: &mut World, state: &RenderState) {
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
                    (state.screen_extents.0 as i32 + progress_x * 2) as u32, 
                    (state.screen_extents.1 as i32 + progress_y * 2) as u32
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
                let offset = (state.screen_extents.0 as f32 * (self.progress as f32 / 100.0)) as i32;
                if let Some(screenshot) = &world.transition_context.screenshot {
                    canvas.set_blend_mode(sdl2::render::BlendMode::Blend);
                    canvas.set_draw_color(Color::RGBA(0, 0, 0, 255));
                    canvas.fill_rect(None).unwrap();
                    canvas.set_blend_mode(sdl2::render::BlendMode::None);
                    for i in 0..(state.screen_extents.1 as i32 / height as i32) {
                        // laggy?
                        let src = Rect::new(0, i * height as i32, state.screen_extents.0, height);
                        let dst = Rect::new(if i % 2 == 0 { offset } else { -offset }, i * height as i32, state.screen_extents.0, height);
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
                    for y in 0..(state.screen_extents.1 as i32 / pixelation_factor) {
                        for x in 0..(state.screen_extents.0 as i32 / pixelation_factor) {
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
                        for y in 0..(state.screen_extents.1 as i32) {
                            let sin = ((y as f32 / (state.screen_extents.1 as f32) * PI * (waves as f32)).sin() * progress as f32) as i32;
                            let src = Rect::new(0, y, state.screen_extents.0, 1);
                            let dst = Rect::new(sin, y, state.screen_extents.0, 1);
                            canvas.copy(&screenshot, src, dst).unwrap();
                        }
                    } else {
                        for x in 0..state.screen_extents.0 as i32 {
                            let sin = ((x as f32 / (state.screen_extents.0 as f32) * PI * (waves as f32)).sin() * progress as f32) as i32;
                            let src = Rect::new(x, 0, 1, state.screen_extents.1);
                            let dst = Rect::new(x, sin, 1, state.screen_extents.1);
                            canvas.copy(&screenshot, src, dst).unwrap();
                        }
                    }
                }
            },
            TransitionType::GridCycle => {
                let progress = (100.0 * (self.progress as f32 / 100.0)) as i32;

                if let Some(screenshot) = &world.transition_context.screenshot {
                    canvas.set_blend_mode(sdl2::render::BlendMode::Blend);
                    canvas.set_draw_color(Color::RGBA(0, 0, 0, 255));
                    canvas.fill_rect(None).unwrap();
                    canvas.set_blend_mode(sdl2::render::BlendMode::None);
                    let width = state.screen_extents.0 as i32 / 20;
                    let height = state.screen_extents.1 as i32 / 20;
                    let radius = 50.0;
                    for y in 0..20 {
                        for x in 0..20 {
                            let i = (((y * width + x) as f32 / (width * height) as f32) - 0.5) * 4.0 * PI + (progress as f32 / 10.0);
                            let src = Rect::new(x * width, y * height, width as u32, height as u32);
                            let start = (src.x as f32, src.y as f32);
                            let target = (i.cos() * radius + (state.screen_extents.0 as f32 / 2.0), i.sin() * radius + (state.screen_extents.1 as f32 / 2.0));
                            let a = (progress as f32 / 50.0).min(1.0);
                            let dest = Rect::new(
                                (start.0 * (1.0 - a) + (target.0 * a)) as i32,
                                (start.1 * (1.0 - a) + (target.1 * a)) as i32,
                                width as u32, height as u32
                            );
                            canvas.copy(&screenshot, src, dest).unwrap();
                        }
                    }
                }
            }
        }
    }

    //pub fn parse()
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

pub enum PropertyLocation {
    Player(PlayerPropertyType),
    World(LevelPropertyType)
}