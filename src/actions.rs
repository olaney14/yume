use std::path::PathBuf;

use json::JsonValue;

use crate::{ai::Animator, audio::Song, effect::Effect, entity::{Entity, VariableValue}, game::{BoolProperty, Condition, EntityPropertyType, FloatProperty, IntProperty, LevelPropertyType, PlayerPropertyType, PropertyLocation, QueuedLoad, StringProperty, WarpPos}, player::Player, transitions::Transition, world::{QueuedEntityAction, World}};

pub fn parse_action(parsed: &JsonValue) -> Result<Box<dyn Action>, String> {
    if parsed.is_array() {
        return MultipleAction::parse(parsed);
    }

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
        },
        "set_animation_frame" => {
            return SetAnimationFrameAction::parse(parsed);
        },
        "multiple" => {
            return MultipleAction::parse(parsed);
        },
        "set_variable" | "set_var" => {
            return SetVariableAction::parse(parsed);
        },
        "sit" => {
            return SitAction::parse(parsed);
        },
        "lay_down" => {
            return LayDownAction::parse(parsed);
        },
        "remove" => {
            return RemoveEntityAction::parse(parsed);
        } 
        _ => {
            return Err(format!("Unknown action \"{}\"", parsed["type"].as_str().unwrap()));
        }
    }
}

pub trait Action {
    fn act(&self, player: &mut Player, world: &mut World);
}

pub struct WarpAction {
    pub map: Option<String>,
    pub exit: WarpPos,
    pub transition: Option<Transition>
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

pub struct DelayedAction {
    pub after: Box<dyn Action>,
    pub delay: u32
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

impl Action for DelayedAction {
    fn act(&self, player: &mut Player, world: &mut World) {
        if world.special_context.delayed_run {
            self.after.act(player, world);
        } else {
            world.queued_entity_actions.push(QueuedEntityAction {
                delay: self.delay as i32,
                action_id: world.special_context.action_id,
                entity_id: world.special_context.entity_id,
                multiple_action_id: world.special_context.multiple_action_index
            });
            //world.special_context.multiple_action_index = None;
        }
    }
}

pub struct FreezeAction {
    pub time: Option<u32>
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

impl Action for FreezeAction {
    fn act(&self, player: &mut Player, _world: &mut World) {
        if let Some(time) = self.time {
            player.frozen_time = time;
        } else {
            player.frozen = true;
        }
    }
}

pub struct GiveEffectAction {
    pub effect: String,
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

impl Action for GiveEffectAction {
    fn act(&self, player: &mut Player, world: &mut World) {
        if let Some(effect) = Effect::parse(self.effect.as_str()) {
            if !player.has_effect(&effect) {
                world.special_context.effect_get = Some(effect);
            }
        }
    }
}

pub struct SetFlagAction {
    pub global: bool,
    pub flag: StringProperty,
    pub value: IntProperty
}

impl SetFlagAction {
    pub fn parse(parsed: &JsonValue) -> Result<Box<dyn Action>, String> {
        let mut global = false;
        if parsed["global"].is_boolean() { global = parsed["global"].as_bool().unwrap(); }

        let flag_name = if parsed["flag"].is_string() {
            StringProperty::String(parsed["flag"].as_str().unwrap().to_string())
        } else {
            StringProperty::parse(&parsed["flag"])?
        };

        if parsed["val"].is_number() {
            return Ok(Box::new(SetFlagAction {
                flag: flag_name,
                global,
                value: IntProperty::Int(parsed["val"].as_i32().unwrap())
            }));
        } else if parsed["val"].is_object() {
            let parsed_property = IntProperty::parse(&parsed["val"]);
            if let Some(property) = parsed_property {
                return Ok(Box::new(SetFlagAction {
                    flag: flag_name,
                    global,
                    value: property
                }));
            } else {
                return Err("Could not parse property for flag".to_string());
            }
        } else {
            return Err(String::from("Bad value for flag"));
        }
    }
}

impl Action for SetFlagAction {
    fn act(&self, player: &mut Player, world: &mut World) {
        let value_opt = self.value.get(Some(player), Some(world));
        
        if let Some(value) = value_opt {
            if self.global {
                world.global_flags.insert(self.flag.get(Some(player), Some(world)).unwrap(), value);
            } else {
                world.flags.insert(self.flag.get(Some(player), Some(world)).unwrap(), value);
            }
        }
    }
}

pub struct ConditionalAction {
    pub inner: Box<dyn Action>,
    pub condition: Condition
}

impl ConditionalAction {
    pub fn parse(parsed: &JsonValue) -> Result<Box<dyn Action>, String> {
        if !parsed["condition"].is_object() { return Err("No condition specified for conditional action".to_string()); }
        if !parsed["action"].is_object() && !parsed["action"].is_array() { return Err("No action specified for conditional action".to_string()); }
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

impl Action for ConditionalAction {
    fn act(&self, player: &mut Player, world: &mut World) {
        if self.condition.evaluate(Some(player), Some(world)) {
            self.inner.act(player, world);
        }
    }
}

pub struct PlaySoundAction {
    pub sound: String,
    pub volume: f32,
    pub speed: f32
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

impl Action for PlaySoundAction {
    fn act(&self, _: &mut Player, world: &mut World) {
        world.special_context.play_sounds.push((self.sound.clone(), self.speed, self.volume));
    }
}

pub struct SetPropertyAction {
    pub property: PropertyLocation,
    pub val: JsonValue
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

        let location;
        
        match parsed["in"].as_str().unwrap() {
            "player" => {
                location = Some(PropertyLocation::Player(PlayerPropertyType::parse(&parsed["val"]).unwrap()));
            },
            "world" => {
                location = Some(PropertyLocation::World(LevelPropertyType::parse(&parsed["val"]).unwrap()));
            },
            "entity" => {
                location = Some(PropertyLocation::Entity(EntityPropertyType::parse(&parsed["val"]).unwrap()))
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

impl Action for SetPropertyAction {
    fn act(&self, player: &mut Player, world: &mut World) {
        match &self.property {
            PropertyLocation::Player(prop) => {
                match prop {
                    PlayerPropertyType::Height => { player.layer = IntProperty::parse(&self.val).unwrap().get(Some(&player), Some(&world)).unwrap() },
                    PlayerPropertyType::X => { player.set_x(IntProperty::parse(&self.val).unwrap().get(Some(&player), Some(&world)).unwrap()) },
                    PlayerPropertyType::Y => { player.set_y(IntProperty::parse(&self.val).unwrap().get(Some(&player), Some(&world)).unwrap()) },
                    PlayerPropertyType::Dreaming => { player.dreaming = BoolProperty::parse(&self.val).unwrap().get(Some(&player), Some(&world)).unwrap() },
                    PlayerPropertyType::Layer => { player.layer = IntProperty::parse(&self.val).unwrap().get(Some(&player), Some(&world)).unwrap() },
                    PlayerPropertyType::CheckWalkable => { player.check_walkable_on_next_frame = BoolProperty::parse(&self.val).unwrap().get(Some(&player), Some(&world)).unwrap() }
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
            },
            PropertyLocation::Entity(prop) => {
                if world.special_context.entity_context.entity_call {
                    world.special_context.entity_context.set_properties.push((prop.clone(), self.val.clone()));
                } else {
                    eprintln!("Warning: SetPropertyAction called on entity from outside entity call context");
                }
            }
        }
    }
}

pub struct ChangeSongAction {
    pub new_song: Option<StringProperty>,
    pub song_speed: Option<FloatProperty>,
    pub song_volume: Option<FloatProperty>,
    pub set_defaults: BoolProperty
}

impl ChangeSongAction {
    pub fn parse(parsed: &JsonValue) -> Result<Box<dyn Action>, String> {
        let mut new_volume = None;
        let mut new_speed = None;
        let mut new_song = None;
        let mut set_defaults = BoolProperty::Bool(false);

        if !parsed["volume"].is_null() { new_volume = FloatProperty::parse(&parsed["volume"]); }
        if !parsed["speed"].is_null() { new_speed = FloatProperty::parse(&parsed["speed"]); }
        if !parsed["song"].is_null() { new_song = StringProperty::parse(&parsed["song"]).map_or(None, |v| Some(v)); }
        if !parsed["set_defaults"].is_null() { set_defaults = BoolProperty::parse(&parsed["set_defaults"]).expect("failed to parse set_defaults"); }

        Ok(Box::new(Self {
                    new_song,
                    song_speed: new_speed,
                    song_volume: new_volume,
                    set_defaults
                }))
    }
}

impl Action for ChangeSongAction {
    fn act(&self, player: &mut Player, world: &mut World) {
        if let Some(path) = &self.new_song {
            world.song = Some(Song::new(PathBuf::from(path.get(Some(player), Some(world)).expect("Error in getting song path"))));
            world.song.as_mut().unwrap().dirty = true;
            world.song.as_mut().unwrap().reload = true;
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
            if let Ok(message) = parsed {
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

pub enum AnimationFrameTarget {
    This,
    Other(IntProperty)
}

pub struct SetAnimationFrameAction {
    pub frame: IntProperty,
    pub target: AnimationFrameTarget
}

impl SetAnimationFrameAction {
    pub fn parse(json: &JsonValue) -> Result<Box<dyn Action>, String> {
        let frame = IntProperty::parse(&json["val"]);
        let target = if json["target"].is_string() {
            match json["target"].as_str().unwrap() {
                "self" | "this" => Some(AnimationFrameTarget::This),
                _ => None
            }
        } else {
            if let Some(prop) = IntProperty::parse(&json["target"]) {
                Some(AnimationFrameTarget::Other(prop))
            } else {
                None
            }
        };

        if !frame.is_some() { return Err(String::from("Error parsing frame for set_animation_frame action frame")); }
        if !target.is_some() { return Err(String::from("Error parsing set_animation_frame action target")); }
        Ok(Box::new(Self {
                            frame: frame.unwrap(),
                            target: target.unwrap()
                        }))
    }
}

// TODO: on calling this action, the entities list has an entity removed from it and using
// id is completely invalid so use special context or somethign to fix itplease
impl Action for SetAnimationFrameAction {
    fn act(&self, player: &mut Player, world: &mut World) {
        if let Some(frame) = self.frame.get(Some(player), Some(world)) {
            let target = match &self.target {
                AnimationFrameTarget::This => {
                    if !world.special_context.entity_context.entity_call {
                        eprintln!("Warning: attemped set_animation_frame action on `this` without a valid caller");
                        None
                    } else {
                        let id = world.special_context.entity_context.id;
                        world.entities.as_mut().unwrap().get_mut(id as usize)
                    }
                },
                AnimationFrameTarget::Other(id) => {
                    if let Some(id) = id.get(Some(player), Some(world)) {
                        world.entities.as_mut().unwrap().get_mut(id as usize)
                    } else {
                        None
                    }
                }
            };

            world.defer_entity_action(Box::new(move |entity: &mut Entity| {
                entity.animator = Some(Animator::new(crate::ai::AnimationFrameData::SingleFrame(frame as u32), entity.tileset, 0));
            }));
        }
    }
}

pub struct MultipleAction {
    pub actions: Vec<Box<dyn Action>>
}

impl MultipleAction {
    pub fn parse(json: &JsonValue) -> Result<Box<dyn Action>, String> {
        let mut actions = Vec::new();
        if json.is_array() {
            for action in json.members() {
                actions.push(parse_action(action)?);
            }
        } else if json["actions"].is_array() {
            for action in json["actions"].members() {
                actions.push(parse_action(action)?);
            }
        } else {
            return Err(String::from("No actions list provided for `Multiple` action"));
        }

        Ok(Box::new(
            Self {
                actions
            }
        ))
    }
}

impl Action for MultipleAction {
    fn act(&self, player: &mut Player, world: &mut World) {
        if let Some(index) = world.special_context.multiple_action_index {
            self.actions[index].act(player, world);
            world.special_context.multiple_action_index = None;
        } else {
            for (i, action) in self.actions.iter().enumerate() {
                world.special_context.multiple_action_index = Some(i);
                action.act(player, world);
                world.special_context.multiple_action_index = None;
            }
        }
    }
}

enum AnyProperty {
    Int(IntProperty),
    Float(FloatProperty),
    Bool(BoolProperty),
    String(StringProperty)
}

impl AnyProperty {
    fn to_variable_value(&self, store: bool, world: Option<&World>, player: Option<&Player>) -> VariableValue {
        match self {
            Self::Int(i) => {
                if store {
                    VariableValue::LitInt(i.get(player, world).unwrap())
                } else {
                    VariableValue::Int(i.clone())
                }
            },
            Self::Float(f) => {
                if store {
                    VariableValue::LitFloat(f.get(player, world).unwrap())
                } else {
                    VariableValue::Float(f.clone())
                }
            },
            Self::Bool(b) => {
                if store {
                    VariableValue::LitBool(b.get(player, world).unwrap())
                } else {
                    VariableValue::Bool(b.clone())
                }
            },
            Self::String(s) => {
                if store {
                    VariableValue::LitString(s.get(player, world).unwrap())
                } else {
                    VariableValue::String(s.clone())
                }
            }
        }
    }
}

pub struct SetVariableAction {
    pub variable: StringProperty,
    value: AnyProperty,
    pub store: bool
}

impl SetVariableAction {
    pub fn parse(json: &JsonValue) -> Result<Box<dyn Action>, String> {
        let store = json["store"].as_bool().unwrap_or(true);
        if !json["var_type"].is_string() { return Err("No variable type specified".to_string()); }
        let kind = json["var_type"].as_str().unwrap();
        if json["val"].is_null() { return Err("No variable value specified".to_string()); }
        if json["name"].is_null() { return Err("No variable name specified".to_string()); }
        let name = StringProperty::parse(&json["name"]).unwrap();

        let value;
        match kind {
            "int" => {
                value = IntProperty::parse(&json["val"]).map(|p| AnyProperty::Int(p));
            },
            "float" => {
                value = FloatProperty::parse(&json["val"]).map(|p| AnyProperty::Float(p));
            },
            "bool" | "boolean" => {
                value = BoolProperty::parse(&json["val"]).map(|p| AnyProperty::Bool(p));
            },
            "string" => {
                value = StringProperty::parse(&json["val"]).map(|p| AnyProperty::String(p)).ok();
            },
            _ => value = None
        };

        if let Some(value) = value {
            return Ok(Box::new(Self {
                                        store,
                                        value,
                                        variable: name
                                    }));
        }

        return Err("Error in set variable action parsing, invalid value?".to_string());
    }
}

impl Action for SetVariableAction {
    fn act(&self, player: &mut Player, world: &mut World) {
        if world.special_context.entity_context.entity_call {
            let name = self.variable.get(Some(player), Some(world)).unwrap();
            let variable_value = self.value.to_variable_value(self.store, Some(world), Some(player));
            world.defer_entity_action(Box::new(move |entity| {
                // i dont like this clone call
                entity.set_variable(name.clone(), variable_value.clone());
            }));
        } else {
            eprintln!("Set variable called outside of entity context");
        }
    }
}

enum RemoveEntityTarget {
    This,
    Other(Box<IntProperty>)
}

pub struct RemoveEntityAction {
    target: RemoveEntityTarget
}

impl RemoveEntityAction {
    pub fn parse(json: &JsonValue) -> Result<Box<dyn Action>, String> {
        if json["target"].is_null() { return Err("Error parsing RemoveEntityAction: no target specified".to_string()); }
        let mut target = None;
        if json["target"].is_string() {
            match json["target"].as_str().unwrap() {
                "self" | "this" => target = Some(RemoveEntityTarget::This),
                _ => ()
            }
        } else {
            if let Some(id) = IntProperty::parse(&json["target"]) {
                target = Some(RemoveEntityTarget::Other(Box::new(id)));
            }
        }

        if let Some(target) = target {
            return Ok(Box::new(Self {
                target
            }));
        }

        Err("Error parsing RemoveEntityAction".to_string())
    }
}

impl Action for RemoveEntityAction {
    fn act(&self, player: &mut Player, world: &mut World) {
        match &self.target {
            RemoveEntityTarget::Other(id) => {
                if let Some(id) = id.get(Some(player), Some(world)) {
                    if id >= 0 {
                        world.special_context.entity_removal_queue.push(id as usize);
                    }
                }
                
            },
            RemoveEntityTarget::This => {
                if world.special_context.entity_context.entity_call { 
                    let id_self = world.special_context.entity_context.id;
                    world.special_context.entity_removal_queue.push(id_self as usize);
                } else {
                    eprintln!("Warning: RemoveEntityTarget::This used outside of entity call");
                }
            }
        }
    }
}

pub struct SitAction {}

impl SitAction {
    pub fn parse(_: &JsonValue) -> Result<Box<dyn Action>, String> {
        Ok(Box::new(Self {}))
    }
}

impl Action for SitAction {
    fn act(&self, player: &mut Player, world: &mut World) {
        player.do_sit(world);
    }
}

pub struct LayDownAction {}

impl LayDownAction {
    pub fn parse(_: &JsonValue) -> Result<Box<dyn Action>, String> {
        Ok(Box::new(Self {}))
    }
}

impl Action for LayDownAction {
    fn act(&self, player: &mut Player, world: &mut World) {
        player.do_lay_down(world);
    }
}