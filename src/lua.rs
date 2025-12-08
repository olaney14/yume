use std::{cell::RefCell, collections::HashMap, rc::Rc, str::FromStr};

use mlua::{AnyUserData, Table, UserData, Value};

use crate::{common::Slot, entity::Entity, game, player::Player, world::World};

const UPDATE_CALLBACK: &str = "_update";
const ONLOAD_CALLBACK: &str = "_load";
const ONUSE_CALLBACK: &str = "_use";
const ONBUMP_CALLBACK: &str = "_bump";
const ONWALK_CALLBACK: &str = "_walk";

pub enum InteractionType {
    Use,
    Walk,
    Bump
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
    callback_queue: Vec<Callback>
}

impl ScriptingContext {
    /// world -> script context
    fn sync(&mut self, world: &mut World, player: &mut Player) {
        self.world.width = world.width;
        self.world.height = world.height;

        self.world.entities.clear();

        for entity in world.entities.as_ref().unwrap().iter() {
            self.world.entities.push(LuaEntity::from_entity(entity));
        }
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
            .map(|e| (e.0, e.1.id))
            .filter(|(ix, eid)| *eid == id).next();

        if let Some((ix, eid)) = target_index {
            if let Some(script_env) = self.entity_scripts.get(&eid) {
                if let Ok(func) = script_env.get::<mlua::Function>(function) {
                    let mut entity = std::mem::take(&mut self.world.entities[ix]);

                    self.lua.scope(|scope| {
                        let world_userdata = scope.create_userdata_ref_mut(&mut self.world).unwrap();
                        let entity_userdata = scope.create_userdata_ref_mut(&mut entity).unwrap();

                        // this one should be valid forever
                        let direction_userdata = self.lua.create_userdata(direction).unwrap();
                        
                        if let Err(e) = func.call::<()>((world_userdata, entity_userdata, direction_userdata)) {
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
        let entity_ids: Vec<u32> = world.entities.as_ref().unwrap().iter().map(|e| e.id).collect();

        for i in 0..entities_size {
            if let Some(script_env) = self.entity_scripts.get(&entity_ids[i]) {
                if let Ok(func) = script_env.get::<mlua::Function>(function) {
                    let mut entity = std::mem::take(&mut self.world.entities[i]);

                    self.lua.scope(|scope| {
                        let world_userdata = scope.create_userdata_ref_mut(&mut self.world).unwrap();
                        let entity_userdata = scope.create_userdata_ref_mut(&mut entity).unwrap();

                        if let Err(e) = func.call::<()>((world_userdata, entity_userdata)) {
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

    pub fn new() -> Self {
        let lua = mlua::Lua::new();

        let directions = lua.create_table().unwrap();
        directions.set("Up", lua.create_userdata(game::Direction::Up).unwrap()).unwrap();
        directions.set("Down", lua.create_userdata(game::Direction::Down).unwrap()).unwrap();
        directions.set("Left", lua.create_userdata(game::Direction::Left).unwrap()).unwrap();
        directions.set("Right", lua.create_userdata(game::Direction::Right).unwrap()).unwrap();

        lua.globals().set("Direction", directions).unwrap();

        Self {
            lua,
            entity_scripts: HashMap::new(),
            world_script: None,
            world: LuaWorld::default(),
            callback_queue: Vec::new()
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

    // Lua methods to be implemented in UserData trait impl
    // all of these need to handle errors by printing warnings to the console instead of crashing
    // get_tile/tiles?() // These are a little complicated, there are tile layers, tilesets to consider
    // set_tile() // i.e. how might the script know which tile id means what? which layer to return tiles from? much to consider
    // queue_load() // needs a transition type, world path
}

#[derive(Clone, Debug, Default)]
struct LuaEntity {
    id: u32,
    // this is engine terminology for the layer the entity is on
    height: i32, // read/write replicated
    draw: bool, // ^
    solid: bool, // ^
    x: i32, // ^
    y: i32, // ^
    moving: bool, // ^
    walk: Option<game::Direction>

    // Methods
    // walk() take in direction
    // remove()
}

impl LuaEntity {
    pub fn from_entity(entity: &Entity) -> Self {
        Self {
            id: entity.id,
            height: entity.height,
            draw: entity.draw,
            solid: entity.solid,
            x: entity.x,
            y: entity.y,
            moving: entity.movement.as_ref().map_or(false, |m| m.moving),
            walk: None
        }
    }

    pub fn replicate(&mut self, entity: &mut Entity, player: &Player, entity_list: &Vec<Entity>, world: &mut World) {
        entity.height = self.height;
        entity.draw = self.draw;
        entity.solid = self.solid;
        entity.x = self.x;
        entity.y = self.y;
        
        if let Some(walk) = self.walk {
            entity.walk(walk, world, player, entity_list);
        }
        self.walk = None;
    }
}

impl UserData for LuaWorld {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("width", |_, this, ()| {
            Ok(this.width)
        });
        methods.add_method("height", |_, this, ()| {
            Ok(this.width)
        });
    }
}

impl UserData for LuaEntity {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method_mut("walk", |_, this, direction: AnyUserData| {
            if let Ok(direction) = direction.borrow::<game::Direction>() {
                this.walk = Some(*direction);
            }

            Ok(())
        });
        
        // methods.add_method_mut("walk", |_, this, ()| {
        //     // if let Ok(direction) = direction.borrow::<game::Direction>() {
        //     //     this.walk = Some(*direction);
        //     // }
        //     this.walk = Some(game::Direction::Up);

        //     Ok(())
        // });
    }
}

#[derive(Default, Debug, Clone)]
struct Color {

}

#[derive(Default, Debug, Clone)]
struct Vec2i {

}

impl UserData for game::Direction {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("flipped", |lua, this, ()| {
            let flipped = this.flipped();
            
            Ok(lua.create_userdata(flipped))
        });

        methods.add_method("tostring", |lua, this, ()| {
            Ok(this.to_string())
        });
    }
}