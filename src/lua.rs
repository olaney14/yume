use std::collections::HashMap;

use mlua::{Table, UserData};

use crate::{entity::Entity, world::World};

const UPDATE_CALLBACK: &str = "_update";
const ONLOAD_CALLBACK: &str = "_onload";

pub struct ScriptingContext {
    lua: mlua::Lua,
    entity_scripts: HashMap<u32, Table>,
    world_script: Option<Table>
}

impl ScriptingContext {
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

    pub fn on_update(&mut self, world: &mut World) {
        self.lua.scope(|scope| {
            let entities_size = world.entities.as_ref().unwrap().len();
            let entity_ids: Vec<u32> = world.entities.as_ref().unwrap().iter().map(|e| e.id).collect();

            let world_wrapper = WorldWrapper { world };
            let lua_world_userdata = scope.create_userdata(world_wrapper).unwrap();
            // let mut placeholder = Some(Entity::new());
            for i in 0..entities_size {
                let id: u32 = entity_ids[i];
                let script_env = self.entity_scripts.get(&id);

                if let Some(script_env) = script_env {
                    if let Ok(func) = script_env.get::<mlua::Function>(UPDATE_CALLBACK) {
                        // If the entity has a script and a valid update function
                        // let mut entity = std::mem::replace(world_wrapper.world.entities.as_mut().unwrap().get_mut(i).unwrap(), placeholder.take().unwrap());
                        // let entity_ref = world.entities.as_mut().unwrap().get_mut(i).unwrap();
                        // let entity_wrapper = EntityWrapper { entity: entity_ref };
                        // let lua_entity_userdata = scope.create_userdata(entity_wrapper).unwrap();

                        // TODO: proper runtime error handling
                        func.call::<()>((&lua_world_userdata, id)).unwrap();

                        // placeholder = Some(std::mem::replace(world_wrapper.world.entities.as_mut().unwrap().get_mut(i).unwrap(), entity));
                    }
                }  
            }

            Ok(())
        }).unwrap();
    }

    pub fn on_load(&mut self, world: &mut World) {
        self.lua.scope(|scope| {
            let entities_size = world.entities.as_ref().unwrap().len();
            let entity_ids: Vec<u32> = world.entities.as_ref().unwrap().iter().map(|e| e.id).collect();

            let world_wrapper = WorldWrapper { world };
            let lua_world_userdata = scope.create_userdata(world_wrapper).unwrap();
            for i in 0..entities_size {
                let id: u32 = entity_ids[i];
                let script_env = self.entity_scripts.get(&id);

                if let Some(script_env) = script_env {
                    if let Ok(func) = script_env.get::<mlua::Function>(ONLOAD_CALLBACK) {
                        // TODO: proper runtime error handling
                        func.call::<()>((&lua_world_userdata, id)).unwrap();
                    }
                }  
            }

            Ok(())
        }).unwrap();
    }

    pub fn new() -> Self {
        Self {
            lua: mlua::Lua::new(),
            entity_scripts: HashMap::new(),
            world_script: None
        }
    }
}

struct WorldWrapper<'a, 'w> {
    world: &'a mut World<'w>
}

// Update this is never yused beasdcue its abd and abd anmd bad
// /// This is only ever used for scripted entities on themselves
// struct EntityWrapper<'a> {
//     entity: &'a mut Entity
// }

impl UserData for WorldWrapper<'_, '_> {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method_mut("test", |_, this, ()| {
            this.world.snow.enabled = true;
            Ok(())
        });
    }
}

// impl UserData for EntityWrapper<'_> {
//     fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
//         methods.add_method_mut("test", |_, this, ()| {
//             this.entity.draw = !this.entity.draw;
//             Ok(())
//         });
//     }
// }