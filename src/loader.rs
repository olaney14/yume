use std::{any::Any, cell::RefCell, collections::HashMap, ffi::OsString, fs::{self, File}, io::Read, path::{Path, PathBuf}, rc::Rc, u8};

use json::JsonValue;
use sdl2::{render::{TextureCreator, TextureAccess}, pixels::{PixelFormatEnum, Color}, rect::Rect};
use tiled::{Loader, Orientation, LayerType, TileLayer, PropertyValue, TilesetLocation};

use crate::{actions::{self, MultipleAction}, ai::{self, parse_animator}, audio::Song, entity::{self, parse_trigger, Entity, TriggeredAction}, game::RenderState, particles, screen_event::ScreenEvent, texture::Texture, tiles::{SpecialTile, Tile, TileExits, Tilemap, Tileset}, world::{self, ImageLayer, Layer, World}};

impl<'a> World<'a> {
    pub fn load_from_file<T, P: AsRef<Path>>(file: P, creator: &'a TextureCreator<T>, old_world: Option<World<'a>>, state: &RenderState) -> Result<World<'a>, Box<dyn std::error::Error>> {
        let mut loader = Loader::new();
        let map = loader.load_tmx_map(&file)?;

        let mut world = match old_world {
            Some(old) => World::with_old(old, creator),
            None => World::new(creator, state)
        };

        world.source_file = file.as_ref().to_path_buf();
        world.name = world.source_file.file_stem().unwrap_or(&OsString::from("none")).to_str().unwrap_or("none").to_string();

        if let Some(color) = map.background_color {
            world.background_color = sdl2::pixels::Color::RGBA(color.red, color.green, color.blue, color.alpha);
        }

        // Loading - Map Properties

        if let Some(PropertyValue::BoolValue(clamp_camera)) = map.properties.get("clampCamera") {
            world.clamp_camera = *clamp_camera;
            world.clamp_camera_axes = Some(world::Axis::All);
        }

        if let Some(PropertyValue::BoolValue(clamp_camera)) = map.properties.get("clamp_camera") {
            world.clamp_camera = *clamp_camera;
            world.clamp_camera_axes = Some(world::Axis::All);
        }

        if let Some(PropertyValue::StringValue(axis)) = map.properties.get("clamp_camera_axis") {
            world.clamp_camera_axes = world::Axis::parse(axis);
        }

        let mut default_pos = map.properties.get("defaultPos");
        if default_pos.is_none() {
            default_pos = map.properties.get("default_pos");
        }

        if let Some(PropertyValue::StringValue(default_pos)) = default_pos {
            let mut split = default_pos.split(',');
            world.default_pos = Some((
                split.next().map(|x| x.parse::<i32>().unwrap_or(0)).unwrap_or(0),
                split.next().map(|y| y.parse::<i32>().unwrap_or(0)).unwrap_or(0),
            ));
        }

        if let Some(PropertyValue::StringValue(edges)) = map.properties.get("edges") {
            let parsed = json::parse(edges).unwrap();
            if !parsed["up"].is_null() {
                world.side_actions[0] = (false, Some(actions::parse_action(&parsed["up"]).map_err(|err| { format!("in up screen action: {}", err) })?));
            }
            if !parsed["down"].is_null() {
                world.side_actions[1] = (false, Some(actions::parse_action(&parsed["down"]).map_err(|err| { format!("in down screen action: {}", err) })?));
            }
            if !parsed["left"].is_null() {
                world.side_actions[2] = (false, Some(actions::parse_action(&parsed["left"]).map_err(|err| { format!("in left screen action: {}", err) })?));
            }
            if !parsed["right"].is_null() {
                world.side_actions[3] = (false, Some(actions::parse_action(&parsed["right"]).map_err(|err| { format!("in right screen action: {}", err) })?));
            }
        }

        if let Some(PropertyValue::BoolValue(looping)) = map.properties.get("looping") {
            world.looping = *looping;
        }

        if let Some(PropertyValue::StringValue(axis)) = map.properties.get("looping_axis") {
            world.looping_axes = world::Axis::parse(axis);
        }

        if let Some(PropertyValue::StringValue(song)) = map.properties.get("music") {
            if let Some(old_song) = &mut world.song && old_song.path == PathBuf::from(song) {
                // same song, reuse
                old_song.default_speed = 1.0;
                old_song.default_volume = 1.0;
                old_song.dirty = true;
            } else {
                world.song = Some(Song::new(song));
            }
        } else {
            world.song = None;
        }

        if let Some(PropertyValue::FloatValue(speed)) = map.properties.get("music_speed") {
            if let Some(song) = &mut world.song {
                song.speed = *speed;
                song.default_speed = *speed;
                song.dirty = true;
            }
        }

        if let Some(PropertyValue::FloatValue(volume)) = map.properties.get("music_volume") {
            if let Some(song) = &mut world.song {
                song.volume = *volume;
                song.default_volume = *volume;
                song.dirty = true;
            }
        }

        if let Some(PropertyValue::StringValue(tint_color)) = map.properties.get("tint") {
            let mut color = tint_color.split(',');
            world.tint = Some(Color::RGBA(
                color.next().ok_or("tint property missing r channel (ex: r,g,b,a)")?.parse::<u8>()?, 
                color.next().ok_or("tint property missing g channel (ex: r,g,b,a)")?.parse::<u8>()?, 
                color.next().ok_or("tint property missing b channel (ex: r,g,b,a)")?.parse::<u8>()?, 
                color.next().ok_or("tint property missing a channel (ex: r,g,b,a)")?.parse::<u8>()?, 
            ));
        }

        if let Some(PropertyValue::BoolValue(raindrops)) = map.properties.get("raindrops") {
            world.raindrops.enabled = *raindrops;
        }

        if let Some(PropertyValue::BoolValue(snow)) = map.properties.get("snow") {
            world.snow.enabled = *snow;
        }

        if map.infinite() { return Err("infinite maps not supported".into()) }
        if !matches!(map.orientation, Orientation::Orthogonal) { return Err("non-orthogonal maps not supported".into()) }

        for tileset in map.tilesets() {
            let mut ts = Tileset::new_with_tile_size(
                Texture::from_file(&tileset.as_ref().image.as_ref().ok_or("tileset has no source image")?.source, creator).map_err(|err| { format!("failed to load tileset texture: {}", err) })?,
                tileset.tile_width, tileset.tile_height
            );
            ts.name = Some(tileset.name.clone());
            world.tilesets.push(ts);
        }

        for layer in map.layers().into_iter() {
            match layer.layer_type() {
                LayerType::Tiles(TileLayer::Finite(finite_tile_layer)) => {
                    let mut tilemap = Tilemap::new(map.width, map.height);

                    let layer_height = if let Some(PropertyValue::IntValue(height)) = layer.properties.get("height") {
                        *height
                    } else { 0 };

                    for j in 0..map.height {
                        for i in 0..map.width {
                            let tile_opt = finite_tile_layer.get_tile(i as i32, j as i32);

                            if let Some(tile) = tile_opt {
                                if tile.get_tile().is_none() { continue; }
                                let tileset_width = tile.get_tileset().columns;

                                // animated tiles are implemented as entities
                                if let Some(PropertyValue::StringValue(animation)) = tile.get_tile().unwrap().properties.get("animation") {
                                    match parse_animator(&json::parse(&animation).map_err(|err| { format!("failed to parse tile animator json: {}", err) })?, tile.tileset_index() as u32, tileset_width) {
                                        Ok(animator) => {
                                            let mut entity = Entity::new();
                                            entity.animator = Some(animator);
                                            if let Some(PropertyValue::BoolValue(blocking)) = tile.get_tile().ok_or("failed to get tile for animation parsing")?.properties.get("blocking") {
                                                entity.solid = *blocking;
                                            }
                                            entity.x = i as i32 * 16;
                                            entity.y = j as i32 * 16;
                                            entity.tileset = tile.tileset_index() as u32;
                                            entity.id = tile.id();
                                            entity.draw = true;
                                            entity.walk_over = true;
                                            entity.height = layer_height;
                                            world.add_entity(entity);
                                        },
                                        Err(e) => {
                                            eprintln!("{}", e);
                                        }
                                    }
                                    continue;
                                }

                                tilemap.set_tile(i, j, Tile::from_tiled(tile)).map_err(|err| { err.to_string() })?;
                                let ref_tile = tile.get_tile().ok_or("failed to get tile for property parsing")?;
                                if let Some(PropertyValue::BoolValue(blocking)) = ref_tile.properties.get("blocking") {
                                    tilemap.set_collision(i, j, *blocking);
                                }

                                if let Some(PropertyValue::StringValue(step)) = ref_tile.properties.get("step") {
                                    tilemap.set_special(i, j, SpecialTile::Step(step.clone(), 0.25));
                                }

                                if let Some(PropertyValue::FloatValue(step_volume)) = ref_tile.properties.get("step_volume") {
                                    // this property is only valid if defined along with step
                                    if let Some(SpecialTile::Step(_, vol)) = tilemap.get_special_mut(i, j) {
                                        *vol = *step_volume;
                                    }
                                }

                                if let Some(PropertyValue::BoolValue(stairs)) = ref_tile.properties.get("stairs") {
                                    if *stairs {
                                        tilemap.set_special(i, j, SpecialTile::Stairs);
                                    }
                                }

                                if let Some(PropertyValue::BoolValue(no_rain)) = ref_tile.properties.get("no_rain") {
                                    if *no_rain {
                                        tilemap.set_special(i, j, SpecialTile::NoRain);
                                    }
                                }

                                if let Some(PropertyValue::IntValue(speed_mod)) = ref_tile.properties.get("speed_mod") {
                                    tilemap.set_special(i, j, SpecialTile::SpeedMod(*speed_mod));
                                }

                                if let Some(PropertyValue::BoolValue(ladder)) = ref_tile.properties.get("ladder") {
                                    if *ladder {
                                        tilemap.set_special(i, j, SpecialTile::Ladder);
                                    }
                                }

                                if let Some(PropertyValue::StringValue(exits)) = ref_tile.properties.get("exits") {
                                    let exit_type = TileExits::parse(&exits);
                                    tilemap.set_special(i, j, SpecialTile::Exits(exit_type));
                                }
                            }
                        }
                    }

                    // Loading - Layer Properties

                    let mut world_layer = Layer::new(tilemap);
                    world_layer.height = layer_height;

                    if let Some(PropertyValue::BoolValue(draw)) = layer.properties.get("draw") {
                        world_layer.draw = *draw;
                    }

                    if let Some(PropertyValue::BoolValue(collide)) = layer.properties.get("collide") {
                        world_layer.collide = *collide;
                    }
                    
                    if let Some(PropertyValue::StringValue(name)) = layer.properties.get("name")  {
                        world_layer.name = name.clone();
                    } else {
                        world_layer.name = layer.name.clone();
                    }

                    world.add_layer(world_layer);
                },
                LayerType::Tiles(TileLayer::Infinite(_)) => eprintln!("Infinite tile layers not supported"),
                LayerType::Objects(object_layer) => {
                    for object in object_layer.objects().into_iter() {
                        if let Some(tile_obj) = object.get_tile() {
                            if let TilesetLocation::Map(tileset_id) = tile_obj.tileset_location() {
                                let tileset_width = tile_obj.get_tileset().columns;

                                let mut entity = Entity {
                                    actions: Vec::new(),
                                    height: 0,
                                    id: tile_obj.id(),
                                    tileset: *tileset_id as u32,
                                    solid: true,
                                    collider: Rect::new(0, 0, world.tilesets[*tileset_id].tile_width, world.tilesets[*tileset_id].tile_height),
                                    x: object.x as i32,
                                    y: object.y as i32 - world.tilesets[*tileset_id].tile_height as i32,
                                    draw: true,
                                    walk_over: false,
                                    ai: None,
                                    animator: None,
                                    movement: None,
                                    interaction: None,
                                    variables: Rc::new(RefCell::new(HashMap::new())),
                                    particle_emitter: None,
                                    killable: false,
                                    script: None,
                                    tiled_id: object.id(),
                                    script_properties: HashMap::new()
                                };

                                // is this clone call necessary?
                                let mut properties = object.properties.clone();

                                if let Some(PropertyValue::StringValue(file)) = properties.get("file") {
                                    let mut file = fs::File::open(file)?;
                                    let mut source = String::new();
                                    file.read_to_string(&mut source).unwrap();
                                    json_to_properties(&mut properties, &mut json::parse(&source)?);
                                }
                                
                                if let Some(PropertyValue::IntValue(height)) = properties.get("height") { entity.height = *height; }
                                if let Some(PropertyValue::BoolValue(solid)) = properties.get("solid") { entity.solid = *solid; }
                                if let Some(PropertyValue::BoolValue(draw)) = properties.get("draw") { entity.draw = *draw; }
                                if let Some(PropertyValue::BoolValue(walk_over)) = properties.get("walk_over") { entity.walk_over = *walk_over; }

                                if let Some(prop) = properties.get("killable") { if let PropertyValue::BoolValue(killable) = prop { 
                                    entity.killable = *killable; 

                                    entity.actions.push(TriggeredAction {
                                        run_on_next_loop: false,
                                        trigger: entity::Trigger::Use,
                                        action: Box::new(MultipleAction {
                                            actions: vec![
                                                // TODO:::: THIS
                                                // TODO:::: THIS
                                                // TODO:::: THIS
                                                // TODO:::: THIS
                                                // TODO:::: THIS
                                                // STILL
                                            ]
                                        })
                                    });
                                } }
                                if let Some(PropertyValue::StringValue(collider)) = properties.get("collider") { entity.collider = parse_rect(&json::parse(collider)?) }
                                if let Some(PropertyValue::StringValue(ai)) = properties.get("ai") { entity.ai = Some(ai::parse_ai(&json::parse(ai)?)?) }
                                if let Some(PropertyValue::StringValue(animation)) = properties.get("animation") { entity.animator = Some(ai::parse_animator(&json::parse(&animation)?, *tileset_id as u32, tileset_width)?) }
                                if let Some(PropertyValue::StringValue(particles)) = properties.get("particles") { 
                                    entity.particle_emitter = Some(particles::parse_particles(&json::parse(&particles)?).ok_or("error parsing particles")?);

                                    let texture = &entity.particle_emitter.as_ref().unwrap().texture;

                                    if !world.particle_textures.textures.contains_key(texture) {
                                        world.particle_textures.add_texture(texture, creator);
                                    }
                                }

                                let mut actions_vec = Vec::new();
                                if let Some(PropertyValue::StringValue(actions)) = properties.get("actions") {
                                    let parsed = json::parse(actions).map_err(|err| { format!("error parsing actions: {}", err) })?;

                                    if parsed.is_array() {
                                        for cur_action in parsed.members() {
                                            let mut trigger = None;
                                            let mut action = None;

                                            if cur_action["trigger"].is_object() || cur_action["trigger"].is_string() || cur_action["trigger"].is_array() {
                                                trigger = Some(parse_trigger(&cur_action["trigger"]).ok_or("failed to parse trigger")?);
                                            } else {
                                                eprintln!("Invalid type for trigger: {:?}", cur_action["trigger"].type_id());
                                            }
                                            if cur_action["action"].is_object() || cur_action["action"].is_array() {
                                                action = Some(actions::parse_action(&cur_action["action"]).map_err(|err| { format!("error parsing action: {}", err) })?);

                                                action_preload(&cur_action["action"], &mut world, creator);
                                            }

                                            if trigger.is_some() && action.is_some() {
                                                actions_vec.push(
                                                    TriggeredAction {
                                                        action: action.unwrap(),
                                                        trigger: trigger.unwrap(),
                                                        run_on_next_loop: false
                                                    }
                                                );
                                            }
                                        }
                                    } else {
                                        eprintln!("Warning: Object actions property is not an array");
                                    }
                                }

                                entity.actions = actions_vec;

                                if let Some(PropertyValue::StringValue(path)) = properties.get("script") {
                                    let path = PathBuf::from("res/scripts/").join(path);
                                    let file = File::open(&path);
                                    if let Ok(mut file) = file {
                                        let mut source = String::new();
                                        file.read_to_string(&mut source).expect("Failed to read script source");
                                        entity.script = Some(source);
                                    } else {
                                        eprintln!("Script file \"{:?}\" not found", &path);
                                    }
                                }

                                if let Some(PropertyValue::StringValue(src)) = properties.get("meta") {
                                    match json::parse(src) {
                                        Ok(parsed) => {
                                            for (key, value) in parsed.entries() {
                                                entity.script_properties.insert(key.to_string(), value.clone());
                                            }
                                        },
                                        Err(e) => {
                                            eprintln!("Error parsing script properties: {e}");
                                        }
                                    }
                                }

                                world.add_entity(entity);
                            }
                        }
                    }
                },
                LayerType::Image(image_layer) => {
                    if let Some(image) = &image_layer.image {
                        let mut world_image_layer = ImageLayer::load_from_file(&image.source, creator);

                        if let Some(PropertyValue::BoolValue(b)) = layer.properties.get("looping") { world_image_layer.looping_x = *b; world_image_layer.looping_y = *b; };
                        if let Some(PropertyValue::BoolValue(b)) = layer.properties.get("looping_x") { world_image_layer.looping_x = *b; };
                        if let Some(PropertyValue::BoolValue(b)) = layer.properties.get("looping_y") { world_image_layer.looping_y = *b; };
                        if let Some(PropertyValue::IntValue(i)) = layer.properties.get("scroll_x") { world_image_layer.scroll_x = *i; };
                        if let Some(PropertyValue::IntValue(i)) = layer.properties.get("scroll_y") { world_image_layer.scroll_y = *i; };
                        if let Some(PropertyValue::IntValue(i)) = layer.properties.get("x") { world_image_layer.x = *i; };
                        if let Some(PropertyValue::IntValue(i)) = layer.properties.get("y") { world_image_layer.y = *i; };
                        if let Some(PropertyValue::IntValue(i)) = layer.properties.get("delay_x") { world_image_layer.delay_x = *i as u32; world_image_layer.timer_x = *i; };
                        if let Some(PropertyValue::IntValue(i)) = layer.properties.get("delay_y") { world_image_layer.delay_y = *i as u32; world_image_layer.timer_y = *i; };
                        if let Some(PropertyValue::BoolValue(b)) = layer.properties.get("mismatch") { if *b { world_image_layer.timer_x /= 2; } }
                        if let Some(PropertyValue::IntValue(i)) = layer.properties.get("parallax_x") { world_image_layer.parallax_x = *i; };
                        if let Some(PropertyValue::IntValue(i)) = layer.properties.get("parallax_y") { world_image_layer.parallax_y = *i; };
                        if let Some(PropertyValue::IntValue(i)) = layer.properties.get("height") { world_image_layer.height = *i; };
                        if let Some(PropertyValue::BoolValue(b)) = layer.properties.get("draw") { world_image_layer.draw = *b; };
                        if let Some(PropertyValue::BoolValue(b)) = layer.properties.get("center") { world_image_layer.center = *b; };

                        world_image_layer.name = layer.name.clone();

                        if world_image_layer.height > world.layer_max {
                            world.layer_max = world_image_layer.height;
                        }
                        world.image_layers.push(world_image_layer);
                    }
                }
                _ => eprintln!("Unsupported layer type")
            }
        }

        if world.looping {
            world.render_texture = Some(creator.create_texture(Some(PixelFormatEnum::RGBA8888), TextureAccess::Target, world.width * 16, world.height * 16).map_err(|err| { format!("failed to create render texture for looping level: {}", err) })?);
            world.render_texture.as_mut().unwrap().set_blend_mode(sdl2::render::BlendMode::Blend);
        }

        Ok(world)
    }
}

pub fn parse_rect(parsed: &JsonValue) -> Rect {
    let x = parsed["x"].as_i32().unwrap();
    let y = parsed["y"].as_i32().unwrap();
    let w = parsed["w"].as_u32().unwrap();
    let h = parsed["h"].as_u32().unwrap();
    Rect::new(x, y, w, h)
}

/// recursively replace json string `$<property>` with properties from the tiled entity
pub fn replace_json_vars(properties: &mut HashMap<String, PropertyValue>, parsed: &mut JsonValue) {
    for (_, field) in parsed.entries_mut() {
        if field.is_string() {
            let replace = field.as_str().unwrap();
            if replace.starts_with("$") {
                let property = &replace[1..];
                if properties.contains_key(property) {
                    *field = property_to_json(properties.get(property).unwrap());
                } else {
                    eprintln!("Variable field {} not specified", replace);
                }
            }
        } else if field.is_object() {
            replace_json_vars(properties, field);
        } else if field.is_array() {
            for i in 0..field.len() {
                replace_json_vars(properties, &mut field[i]);
            }
        }
    }
}

pub fn json_to_properties(properties: &mut HashMap<String, PropertyValue>, parsed: &mut JsonValue) {
    replace_json_vars(properties, parsed);

    for (name, field) in parsed.entries_mut() {
        if !properties.contains_key(name) {
            if let Some(property) = json_to_property(field) {
                properties.insert(name.to_string(), property);
            } else {
                eprintln!("Error parsing property \"{}\" in property file", name);
            }
        }
    }
}

pub fn property_to_json(property: &PropertyValue) -> JsonValue {
    match property {
        PropertyValue::BoolValue(b) => { JsonValue::Boolean(*b) },
        PropertyValue::ColorValue(c) => { 
            let mut color = JsonValue::new_object();
            color["r"] = JsonValue::Number(c.red.into());
            color["g"] = JsonValue::Number(c.green.into());
            color["b"] = JsonValue::Number(c.blue.into());
            color["a"] = JsonValue::Number(c.alpha.into());
            color
        },
        PropertyValue::FileValue(f) => { JsonValue::String(f.clone()) },
        PropertyValue::FloatValue(n) => { JsonValue::Number((*n).into()) },
        PropertyValue::IntValue(n) => { JsonValue::Number((*n).into()) },
        PropertyValue::ObjectValue(obj) => { JsonValue::Number((*obj).into()) },
        PropertyValue::StringValue(s) => { JsonValue::String(s.clone()) }
    }
}

pub fn json_to_property(parsed: &JsonValue) -> Option<PropertyValue> {
    if parsed.is_object() || parsed.is_array() {
        return Some(PropertyValue::StringValue(parsed.to_string()));
    }
    if parsed.is_boolean() {
        return Some(PropertyValue::BoolValue(parsed.as_bool().unwrap()));
    }
    if parsed.is_null() {
        return None; 
    }
    if parsed.is_string() {
        let string = parsed.as_str().unwrap();
        if string.ends_with('f') {
            if let Ok(float) = string[0..string.len() - 2].parse::<f32>() {
                return Some(PropertyValue::FloatValue(float));
            }
        } else {
            if let Ok(int) = string.parse::<i32>() {
                return Some(PropertyValue::IntValue(int));
            }
        }
        return Some(PropertyValue::StringValue(string.to_string()));
    }
    // We will assume that all json numbers passed are floats for now
    // TODO use the property name to assume better
    if parsed.is_number() {
        return Some(PropertyValue::FloatValue(parsed.as_f32().unwrap()));
    }

    None
}

fn action_preload<'a, T>(action: &JsonValue, world: &mut World<'a>, creator: &'a TextureCreator<T>) {
    // Preload screen events
    if action.is_object() {
        if action["type"].as_str().unwrap() == "play_event" {
            let screen_event = ScreenEvent::from_file(&PathBuf::from("res/data/event/").join(format!("{}.svt", action["event"].as_str().unwrap())), creator);
        
            world.screen_events.insert(action["event"].as_str().unwrap().to_string(), screen_event);
        }
    } else {
        for sub_action in action.members() {
            action_preload(sub_action, world, creator);
        }
    }
}