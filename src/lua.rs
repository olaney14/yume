use std::{collections::HashMap};

use json::JsonValue;
use mlua::{AnyUserData, IntoLua, MetaMethod, Table, UserData, Value};
use sdl2::rect::Rect;

use crate::{ai::{self, AnimationFrameData::SingleFrame}, effect, entity::Entity, game::{self, Direction}, player::Player, tiles, transitions::{Transition, TransitionType}, world::World};

const UPDATE_CALLBACK: &str = "_update";
const ONLOAD_CALLBACK: &str = "_load";
const ONUSE_CALLBACK: &str = "_use";
const ONBUMP_CALLBACK: &str = "_bump";
const ONWALK_CALLBACK: &str = "_walk";

macro_rules! field_cloned {
    ($fields:ident, $name:literal, $inner:ident, $kind:ty) => {
        $fields.add_field_method_get($name, |_, this| {
            Ok(this.$inner.clone())
        });
        $fields.add_field_method_set($name, |_, this, a: $kind| {
            this.$inner = a;
            Ok(())
        });
    }
}

macro_rules! field {
    ($fields:ident, $name:literal, $inner:ident, $kind:ty) => {
        $fields.add_field_method_get($name, |_, this| {
            Ok(this.$inner)
        });
        $fields.add_field_method_set($name, |_, this, a: $kind| {
            this.$inner = a;
            Ok(())
        });
    }
}

macro_rules! field_userdata {
    ($fields:ident, $name:literal, $inner:ident, $kind:ty) => {
        $fields.add_field_method_get($name, |_, this| {
            Ok(this.$inner)
        });
        $fields.add_field_method_set($name, |_, this, a: AnyUserData| {
            if let Ok(user) = a.borrow::<$kind>() {
                this.$inner = *user;
            }
            Ok(())
        });
    }
}

pub enum InteractionType {
    Use,
    Walk,
    Bump
}

#[derive(Clone, Debug)]
enum ScriptEvent {
    Walk(game::Direction),
    ChangeMap { map: String, transition: LuaTransition, x: i32, y: i32 },
    PlaySound { sound: String, speed: f32, volume: f32 },
    // reflection
    WalkNoclip(game::Direction),
    GiveEffect(effect::Effect),
}

// Update is not included since it is always called regardless of context
enum Callback {
    OnLoad,
    Interact { id: u32, kind: InteractionType, direction: game::Direction }
}

pub struct ScriptingContext {
    lua: mlua::Lua,
    entity_scripts: HashMap<u32, Table>,
    world_script: Option<Table>,
    world: LuaWorld,
    player: LuaPlayer,
    callback_queue: Vec<Callback>
}

impl ScriptingContext {
    /// world -> script context
    fn sync(&mut self, world: &mut World, player: &mut Player) {
        self.world.width = world.width;
        self.world.height = world.height;
        self.world.looping_x = world.loop_horizontal();
        self.world.looping_y = world.loop_vertical();

        self.world.entities.clear();

        self.world.session_random = world.random.session_random;
        self.world.level_random = world.random.level_random;

        for entity in world.entities.as_ref().unwrap().iter() {
            self.world.entities.push(LuaEntity::from_entity(entity));
        }

        if world.special_context.tiles_dirty {
            world.special_context.tiles_dirty = false;
            self.world.tilemap.sync(world);
        }

        self.player = LuaPlayer::from_player(player);
    }

    /// script edits -> world
    fn replicate(&mut self, world: &mut World, player: &mut Player) {
        let mut entity_list = world.entities.take().unwrap();
        let mut placeholder = Some(Entity::new());
        for i in 0..entity_list.len() {
            let mut entity = std::mem::replace(entity_list.get_mut(i).unwrap(), placeholder.take().unwrap());
            self.world.entities[i].replicate(&mut entity, player, &entity_list, world);
            placeholder = Some(std::mem::replace(entity_list.get_mut(i).unwrap(), entity));
        }
        world.entities = Some(entity_list);
        self.player.replicate(player);
        for event in self.world.events.drain(..) {
            match event {
                ScriptEvent::ChangeMap { map, transition, x, y } => {
                    world.queued_load = Some(game::QueuedLoad {
                        map: String::from("res/maps/") + map.as_str(),
                        pos: game::WarpPos { x: game::IntProperty::Int(x), y: game::IntProperty::Int(y) }
                    });
                    world.transition = Some(transition.as_transition());
                },
                ScriptEvent::PlaySound { sound, speed, volume } => {
                    world.special_context.play_sounds.push((sound, speed, volume));
                },
                ScriptEvent::GiveEffect(effect) => {
                    if !player.has_effect(&effect) {
                        // this one triggers the cutscene
                        world.special_context.effect_get = Some(effect);
                    }
                }
                _ => {
                    eprintln!("Warning: Script event {event:?} is not valid in a world context");
                }
            }
        }
    }

    pub fn clear(&mut self) {
        self.entity_scripts.clear();
        self.world_script = None;
    }

    pub fn add_entity_script(&mut self, id: u32, source: &str) {
        // TODO: Proper error handling on invalid script

        let chunk = self.lua.load(source);
        // Create an enclosing table to separate each script
        let script_env = self.lua.create_table().unwrap();

        // Have global function calls fallback to the default globals so user can use print, math, etc.
        let globals = self.lua.globals();
        let meta = self.lua.create_table().unwrap();
        meta.set("__index", globals).unwrap();
        script_env.set_metatable(Some(meta)).unwrap();

        let script_func = chunk.set_environment(script_env.clone()).into_function().unwrap();
        // Run the script to initialize callbacks
        script_func.call::<()>(()).unwrap();
        self.entity_scripts.insert(id, script_env);
    }

    fn call_interact(&mut self, function: &str, id: u32, direction: game::Direction, world: &mut World) {
        let target_index = world.entities.as_ref().unwrap()
            .iter().enumerate()
            .map(|e| (e.0, e.1.tiled_id))
            .filter(|(_, eid)| *eid == id).next();

        if let Some((ix, eid)) = target_index {
            if let Some(script_env) = self.entity_scripts.get(&eid) {
                if let Ok(func) = script_env.get::<mlua::Function>(function) {
                    let mut entity = std::mem::take(&mut self.world.entities[ix]);

                    self.lua.scope(|scope| {
                        let world_userdata = scope.create_userdata_ref_mut(&mut self.world).unwrap();
                        let entity_userdata = scope.create_userdata_ref_mut(&mut entity).unwrap();
                        let player_userdata = scope.create_userdata_ref_mut(&mut self.player).unwrap();

                        // this one should be valid forever
                        let direction_userdata = self.lua.create_userdata(direction).unwrap();
                        
                        if let Err(e) = func.call::<()>((world_userdata, entity_userdata, player_userdata, direction_userdata)) {
                            eprintln!("{}", e);
                        }

                        Ok(())
                    }).unwrap();

                    self.world.entities[ix] = entity;
                }
            }
        }
    }

    fn call_all(&mut self, function: &str, world: &mut World) {
        let entities_size = world.entities.as_ref().unwrap().len();
        let entity_ids: Vec<u32> = world.entities.as_ref().unwrap().iter().map(|e| e.tiled_id).collect();

        for i in 0..entities_size {
            if let Some(script_env) = self.entity_scripts.get(&entity_ids[i]) {
                if let Ok(func) = script_env.get::<mlua::Function>(function) {
                    let mut entity = std::mem::take(&mut self.world.entities[i]);

                    self.lua.scope(|scope| {
                        let world_userdata = scope.create_userdata_ref_mut(&mut self.world).unwrap();
                        let entity_userdata = scope.create_userdata_ref_mut(&mut entity).unwrap();
                        let player_userdata = scope.create_userdata_ref_mut(&mut self.player).unwrap();

                        if let Err(e) = func.call::<()>((world_userdata, entity_userdata, player_userdata)) {
                            eprintln!("{}", e);
                        }

                        Ok(())
                    }).unwrap();

                    self.world.entities[i] = entity;
                }
            }
        }
    }

    pub fn on_load(&mut self) {
        self.callback_queue.push(Callback::OnLoad);
    }

    fn on_interact(&mut self, id: u32, kind: InteractionType, direction: game::Direction) {
        self.callback_queue.push(Callback::Interact { id, kind: kind, direction });
    }

    pub fn on_update(&mut self, world: &mut World, player: &mut Player) {        
        for (kind, from, id) in std::mem::take(&mut world.special_context.push_interaction_to_scripts) {
            self.on_interact(id, kind, from);
        }
        
        self.sync(world, player);

        let callbacks = std::mem::take(&mut self.callback_queue);
        for callback in callbacks.into_iter() {
            match callback {
                Callback::OnLoad => self.call_all(ONLOAD_CALLBACK, world),
                Callback::Interact { id, kind, direction } => {
                    match kind {
                        InteractionType::Use => self.call_interact(ONUSE_CALLBACK, id, direction, world),
                        InteractionType::Bump => self.call_interact(ONBUMP_CALLBACK, id, direction, world),
                        InteractionType::Walk => self.call_interact(ONWALK_CALLBACK, id, direction, world),
                    }
                }
            }
        }

        self.call_all(UPDATE_CALLBACK, world);

        self.replicate(world, player);
    }

    fn globals(lua: &mlua::Lua) {
        let directions = lua.create_table().unwrap();
        directions.set("Up", lua.create_userdata(game::Direction::Up).unwrap()).unwrap();
        directions.set("Down", lua.create_userdata(game::Direction::Down).unwrap()).unwrap();
        directions.set("Left", lua.create_userdata(game::Direction::Left).unwrap()).unwrap();
        directions.set("Right", lua.create_userdata(game::Direction::Right).unwrap()).unwrap();

        lua.globals().set("Directions", directions).unwrap();

        let transition = lua.create_table().unwrap();

        let new_transition = lua.create_function(|lua, ()| {
            lua.create_userdata(LuaTransition::default())
        }).unwrap();

        transition.set("new", new_transition).unwrap();

        lua.globals().set("Transition", transition).unwrap();
    }

    pub fn new() -> Self {
        let lua = mlua::Lua::new();

        Self::globals(&lua);

        Self {
            lua,
            entity_scripts: HashMap::new(),
            world_script: None,
            world: LuaWorld::default(),
            player: LuaPlayer::default(),
            callback_queue: Vec::new()
        }
    }
}

#[derive(Default, Debug)]
struct LuaTileLayer {
    pub height: i32,
    pub tiles: Vec<tiles::Tile>,
    pub collision: Vec<bool>
}

#[derive(Default, Debug)]
struct LuaTileWorld {
    pub width: u32,
    pub height: u32,
    pub layers: Vec<LuaTileLayer>
}

impl LuaTileWorld {
    pub fn get_collision_at_tile(&self, x: i32, y: i32, height: i32) -> bool {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 { return true; }
        
        for layer in self.layers.iter() {
            if layer.height != height { continue; }

            if layer.collision[(y * self.width as i32 + x) as usize] { return true; }
        }

        return false;
    }

    pub fn get_collision(&self, x: i32, y: i32, height: i32) -> bool {
        self.get_collision_at_tile(x / 16, y / 16, height)
    }

    pub fn sync(&mut self, world: &World) {
        self.layers.clear();
        self.width = world.width;
        self.height = world.height;

        for layer in world.layers.iter() {
            self.layers.push(LuaTileLayer {
                height: layer.height,
                collision: layer.map.collision.clone(),
                tiles: layer.map.tiles.clone()
            });
        }
    }
}

#[derive(Default, Debug)]
struct LuaWorld {
    width: u32, // readonly write once
    height: u32, // readonly write once
    background_color: Color, // read/write replicated
    tint: Color, // read/write replicated
    level_random: f32, // readonly replicated
    session_random: f32, // readonly write once
    entities: Vec<LuaEntity>, // read/write, no direct removal or addition, replicated
    // flags: HashMap<String, i32> // i'll only add this once i need it because i forgot what it does
    // global_flags: HashMap<String, i32>, ^ but // read/write through methods, replicated
    // i believe these flags are remnants of the previous JSON-based scripting system and might just need to be deprecated
    raindrops: bool, // read/write replicated
    snow: bool, // ^
    // this might be dangerous if it pauses script updates too check that
    paused: bool, // ^
    looping_x: bool, // readonly
    looping_y: bool, // readonly
    tilemap: LuaTileWorld,
    events: Vec<ScriptEvent>,
    // play_sounds: Vec<(String, f32, f32)>

    // Lua methods to be implemented in UserData trait impl
    // all of these need to handle errors by printing warnings to the console instead of crashing
    // get_tile/tiles?() // These are a little complicated, there are tile layers, tilesets to consider
    // set_tile() // i.e. how might the script know which tile id means what? which layer to return tiles from? much to consider
    // queue_load() // needs a transition type, world path
}

impl LuaWorld {
    pub fn get_entity_collision(&self, x: i32, y: i32, height: i32) -> bool {
        for entity in self.entities.iter() {
            if entity.height != height { continue; }

            if entity.get_collision_at_point(x, y) { return true; }
        }

        false
    }

    pub fn get_entity_collision_at_tile(&self, x: i32, y: i32, height: i32) -> bool {
        for entity in self.entities.iter() {
            if entity.height != height { continue; }

            if entity.get_collision_at_tile(x, y) { return true; }
        }

        false
    }

    pub fn collide_at_point(&self, x: i32, y: i32, height: i32) -> bool {
        self.get_entity_collision(x, y, height) || self.tilemap.get_collision(x, y, height)
    }

    pub fn collide_at_tile(&self, x: i32, y: i32, height: i32) -> bool {
        self.get_entity_collision_at_tile(x, y, height) || self.tilemap.get_collision_at_tile(x, y, height)
    }

    pub fn wrap_coord(&self, mut x: i32, mut y: i32) -> (i32, i32) {
        if self.looping_x {
            x = x.rem_euclid(self.width as i32 * 16);
        }
        if self.looping_y {
            y = y.rem_euclid(self.height as i32 * 16);
        }
        (x, y)
    }

    pub fn wrap_tile_coord(&self, mut x: i32, mut y: i32) -> (i32, i32) {
        if self.looping_x {
            x = x.rem_euclid(self.width as i32);
        }
        if self.looping_y {
            y = y.rem_euclid(self.height as i32);
        }
        (x, y)
    }
}

#[derive(Clone, Debug, Default)]
struct LuaEntity {
    id: u32,
    height: i32, // read/write replicated
    draw: bool, // ^
    solid: bool, // ^
    x: i32, // ^
    y: i32, // ^
    moving: bool, // ^
    collider: (i32, i32, u32, u32),
    walk_over: bool,
    events: Vec<ScriptEvent>,
    // walk: Option<game::Direction>,
    // writing
    // is movement initialized?
    // no? are these different then default?
    // yes? init movement, set values
    // movement is initialized
    // set values
    speed: u32,
    // call this sub_speed because delay is hard to understand
    delay: u32,
    script_properties: HashMap<String, JsonValue>,
    animation_frame: u32

    // Methods
    // walk() take in direction
    // remove()
}

impl LuaEntity {
    pub fn from_entity(entity: &Entity) -> Self {
        Self {
            id: entity.tiled_id,
            height: entity.height,
            collider: entity.collider.into(),
            draw: entity.draw,
            solid: entity.solid,
            x: entity.x,
            y: entity.y,
            moving: entity.movement.as_ref().map_or(false, |m| m.moving),
            events: Vec::new(),
            speed: entity.movement.as_ref().map_or(1, |e| e.speed),
            delay: entity.movement.as_ref().map_or(0, |e| e.delay),
            script_properties: entity.script_properties.clone(),
            walk_over: entity.walk_over,
            animation_frame: entity.animator.as_ref().map(|a| a.frame).unwrap_or(0)
        }
    }

    pub fn replicate(&mut self, entity: &mut Entity, player: &Player, entity_list: &Vec<Entity>, world: &mut World) {
        entity.height = self.height;
        entity.draw = self.draw;
        entity.solid = self.solid;
        entity.walk_over = self.walk_over;
        entity.x = self.x;
        entity.y = self.y;
        
        if let Some(movement) = &mut entity.movement {
            movement.speed = self.speed;
            movement.delay = self.delay;
        } else {
            if self.speed != 1 || self.delay != 0 {
                entity.init_movement();
                entity.movement.as_mut().unwrap().speed = self.speed;
                entity.movement.as_mut().unwrap().delay = self.delay;
            }
        }

        if let Some(animator) = &mut entity.animator {
            if let SingleFrame(f) = &mut animator.frame_data {
                *f = self.animation_frame;
            }
            animator.frame = self.animation_frame;
        } else if self.animation_frame != 0 {
            // if user tries to set frame without an animator
            entity.animator = Some(ai::Animator::new(SingleFrame(self.animation_frame), entity.tileset, 1));
        }

        for event in self.events.drain(..) {
            match event {
                ScriptEvent::Walk(direction) => {
                    entity.walk(direction, world, player, entity_list);
                },
                ScriptEvent::WalkNoclip(direction) => {
                    entity.walk_noclip(direction, world, player);
                },
                _ => {
                    eprintln!("Warning: Script event {event:?} is not valid in an entity context");
                }
            }
        }
    }

    pub fn get_collision(&self, other: Rect) -> bool {
        self.solid && Rect::new(self.x + self.collider.0, self.y + self.collider.1, self.collider.2, self.collider.3).has_intersection(other)
    }

    pub fn get_collision_at_point(&self, x: i32, y: i32) -> bool {
        x >= self.x + self.collider.0 && y >= self.y + self.collider.1 &&
        x <= self.x + self.collider.0 + self.collider.2 as i32 && 
        y <= self.y + self.collider.1 + self.collider.3 as i32
    }

    pub fn get_collision_at_tile(&self, x: i32, y: i32) -> bool {
        self.get_collision(Rect::new(x as i32 * 16, y as i32 * 16, 16, 16))
    }
}

#[derive(Default, Clone, Debug)]
struct LuaPlayer {
    x: i32,
    y: i32,
    height: i32,
    facing: Direction,
    moving: bool, // readonly
    speed: u32, // readonly
    sub_speed: u32, // readonly
    // what should be able to be set is a speed_mod
    frozen: bool,
    money: u32,
    dreaming: bool, // readonly
    random: f32, // readonly
    animation_frame: u32,
    effect: String,
}

impl LuaPlayer {
    pub fn from_player(player: &Player) -> Self {
        Self {
            x: player.x,
            y: player.y,
            height: player.layer,
            facing: player.facing,
            moving: player.moving,
            speed: player.speed,
            sub_speed: player.move_delay, // right ?
            frozen: player.frozen,
            money: player.money,
            dreaming: player.dreaming,
            random: player.random,
            animation_frame: player.animation_info.frame + player.animation_info.frame_row * 3,
            effect: player.current_effect.clone().map_or("none".to_string(), |a| a.parsable().to_string())
        }
    }

    pub fn replicate(&mut self, player: &mut Player) {
        if self.x != player.x {
            player.set_x(self.x);
        }

        if self.y != player.y {
            player.set_y(self.y);
        }

        player.facing = self.facing;
        player.frozen = self.frozen;
        player.money = self.money;
        player.layer = self.height;
    }
}

/// the lua instance is needed for string interning
fn json_as_lua(json: &JsonValue, lua: &mlua::Lua) -> Option<Value> {
    match json {
        JsonValue::Boolean(b) => return Some(Value::Boolean(*b)),
        JsonValue::Null => return Some(Value::Nil),
        JsonValue::Number(n) => return Some(Value::Number((*n).into())),
        JsonValue::Short(s) => return Some(s.to_string().into_lua(lua).unwrap()),
        JsonValue::String(s) => return Some(s.clone().into_lua(lua).unwrap()),
        _ => ()
    }

    None
}

impl UserData for LuaWorld {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("width", |_, this, ()| { Ok(this.width) });
        methods.add_method("height", |_, this, ()| { Ok(this.height) });
        methods.add_method("looped_distance", |_, this, (x0, y0, x1, y1): (i32, i32, i32, i32)| {
            // TODO: this function really doesnt need to be taking in u32s
            Ok(ai::looped_manhattan_distance(x0 as u32, y0 as u32, x1 as u32, y1 as u32, this.width * 16, this.height * 16))
        });
        methods.add_method("looped_distance_x", |_, this, (x0, x1): (i32, i32)| {
            Ok(ai::looped_x_distance(x0 as u32, x1 as u32, this.width * 16))
        });  
        methods.add_method("looped_distance_y", |_, this, (y0, y1): (i32, i32)| {
            Ok(ai::looped_y_distance(y0 as u32, y1 as u32, this.height * 16))
        });
        methods.add_method("looping", |_, this, ()| Ok(this.looping_x || this.looping_y));
        methods.add_method("looping_x", |_, this, ()| Ok(this.looping_x));
        methods.add_method("looping_y", |_, this, ()| Ok(this.looping_y));
        methods.add_method("wrap", |_, this, (x, y): (i32, i32)| { Ok(this.wrap_coord(x, y)) });
        methods.add_method("wrap_tile", |_, this, (x, y): (i32, i32)| { Ok(this.wrap_tile_coord(x, y)) });
        methods.add_method("collide", |_, this, (x, y, height): (i32, i32, i32)| {
            Ok(this.collide_at_point(x, y, height))
        });
        methods.add_method("collide_tile", |_, this, (x, y, height): (i32, i32, i32)| {
            Ok(this.collide_at_tile(x, y, height))
        });
        methods.add_method_mut("play", |_, this, (sound, speed, volume): (String, f32, f32)| {
            this.events.push(ScriptEvent::PlaySound { sound, speed, volume });
            Ok(())
        });
        methods.add_method_mut("change_map", |_, this, (name, transition, x, y): (String, AnyUserData, i32, i32)| {
            if let Ok(transition) = transition.borrow::<LuaTransition>() {
                this.events.push(ScriptEvent::ChangeMap { map: name, transition: transition.clone(), x, y });
            } else {
                eprintln!("Error: change_map expected Transition for second argument");
            }
            Ok(()) 
        });
        methods.add_method("session_random", |_, this, ()| Ok(this.session_random));
        methods.add_method("level_random", |_, this, ()| Ok(this.level_random));
        methods.add_method_mut("give_effect", |_, this, effect: String| {
            if let Some(effect) = effect::Effect::parse(&effect) {
                this.events.push(ScriptEvent::GiveEffect(effect));
            } else {
                eprintln!("Error: invalid effect name `{}`", effect);
            }
            Ok(())
        });
    }
}

impl UserData for LuaEntity {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method_mut("walk", |_, this, direction: AnyUserData| {
            if let Ok(direction) = direction.borrow::<game::Direction>() {
                this.events.push(ScriptEvent::Walk(*direction));
            }

            Ok(())
        });

        methods.add_method_mut("walk_noclip", |_, this, direction: AnyUserData| {
            if let Ok(direction) = direction.borrow::<game::Direction>() {
                this.events.push(ScriptEvent::WalkNoclip(*direction));
            }

            Ok(())
        });

        methods.add_method_mut("moving", |_, this, ()| Ok(this.moving));

        methods.add_method("meta", |lua, this, key: String| {
            if let Some(prop) = this.script_properties.get(&key) {
                if let Some(value) = json_as_lua(prop, lua) {
                    return Ok(value);
                }
            }

            eprintln!("Warning: meta value `{key}` not found.");
            Ok(Value::Nil)
        });

        methods.add_method("id", |_, this, ()| { Ok(this.id) });
    }

    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        field!(fields, "speed", speed, u32);
        field!(fields, "sub_speed", delay, u32);
        field!(fields, "x", x, i32);
        field!(fields, "y", y, i32);
        field!(fields, "layer", height, i32);
        field!(fields, "solid", solid, bool);
        field!(fields, "walk_over", walk_over, bool);
        field!(fields, "frame", animation_frame, u32);
    }
}

impl UserData for LuaPlayer {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("moving", |_, this, ()| Ok(this.moving));
        methods.add_method("speed", |_, this, ()| Ok(this.speed));
        methods.add_method("sub_speed", |_, this, ()| Ok(this.sub_speed));
        methods.add_method("dreaming", |_, this, ()| Ok(this.dreaming));
        methods.add_method("random", |_, this, ()| Ok(this.random));
        methods.add_method("frame", |_, this, ()| Ok(this.animation_frame));
        methods.add_method("effect", |_, this, ()| Ok(this.effect.clone()));
    }

    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        field!(fields, "x", x, i32);
        field!(fields, "y", y, i32);
        field!(fields, "frozen", frozen, bool);
        field!(fields, "money", money, u32);
        field!(fields, "layer", height, i32);
        field_userdata!(fields, "facing", facing, Direction);
    }
}

#[derive(Default, Debug, Clone)]
struct Color {

}

impl UserData for game::Direction {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("flipped", |lua, this, ()| {
            let flipped = this.flipped();
            
            Ok(lua.create_userdata(flipped))
        });

        methods.add_method("tostring", |_, this, ()| {
            Ok(this.to_string())
        });

        methods.add_method("x", |_, this, ()| {
            Ok(this.x())
        });

        methods.add_method("y", |_, this, ()| {
            Ok(this.y())
        });

        methods.add_meta_method(MetaMethod::Eq, |_, this, other: AnyUserData| {
            if let Ok(other) = other.borrow::<Direction>() {
                return Ok(*other == *this);
            }
            Ok(false)
        });
    }
}

#[derive(Clone, Debug)]
struct LuaTransition {
    kind: String,
    speed: i32,
    delay: i32,
    fade_music: bool,
    hold: u32,
    reset_music: bool,
    scale: f32
}

impl Default for LuaTransition {
    fn default() -> Self {
        Self {
            kind: "fade".to_string(),
            speed: 8,
            fade_music: true,
            delay: 0,
            hold: 0,
            reset_music: false,
            scale: 2.0
        }
    }
}

impl LuaTransition {
    fn as_transition(&self) -> Transition {
        let mut kind = TransitionType::from_string(&self.kind).unwrap_or(TransitionType::Fade);
        if let TransitionType::Zoom(z) = &mut kind {
            *z = self.scale;
        }

        Transition::new(kind, self.speed, self.delay, self.fade_music, self.hold, self.reset_music)
    }
}

impl UserData for LuaTransition {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        field_cloned!(fields, "type", kind, String);
        field!(fields, "speed", speed, i32);
        field!(fields, "delay", delay, i32);
        field!(fields, "fade_music", fade_music, bool);
        field!(fields, "hold", hold, u32);
        field!(fields, "reset_music", reset_music, bool);
        field!(fields, "scale", scale, f32);
    }
}