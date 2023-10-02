use std::{path::PathBuf, u8, collections::HashMap, fs, io::Read, ffi::OsString};

use json::JsonValue;
use sdl2::{render::{TextureCreator, TextureAccess}, pixels::{PixelFormatEnum, Color}, rect::Rect};
use tiled::{Loader, Orientation, LayerType, TileLayer, PropertyValue, TilesetLocation};

use crate::{world::{World, Layer, ImageLayer}, tiles::{Tilemap, Tileset, Tile, SpecialTile}, texture::Texture, game::{self, parse_action}, audio::Song, entity::{Entity, parse_trigger, TriggeredAction}, ai::{self, parse_animator}};

impl<'a> World<'a> {
    pub fn load_from_file<T>(file: &String, creator: &'a TextureCreator<T>, old_world: &mut Option<World<'a>>) -> World<'a> {
        let mut loader = Loader::new();
        let map = loader.load_tmx_map(file).unwrap();

        let mut world = if let Some(old) = old_world {
            World::with_old(old, creator)
        } else {
            World::new(creator)
        };
        //let mut world = World::new(creator);
        world.name = PathBuf::from(file).file_stem().unwrap_or(&OsString::from("none")).to_str().unwrap_or("none").to_string();

        if let Some(color) = map.background_color {
            world.background_color = sdl2::pixels::Color::RGBA(color.red, color.green, color.blue, color.alpha);
        }

        // Loading - Map Properties

        if let Some(prop) = map.properties.get("clampCamera") {
            if let PropertyValue::BoolValue(clamp_camera) = prop {
                world.clamp_camera = *clamp_camera;
            }
        }
        if let Some(prop) = map.properties.get("clamp_camera") {
            if let PropertyValue::BoolValue(clamp_camera) = prop {
                world.clamp_camera = *clamp_camera;
            }
        }

        let mut default_pos = map.properties.get("defaultPos");
        if default_pos.is_none() {
            default_pos = map.properties.get("default_pos");
        }

        if let Some(prop) = default_pos {
            if let PropertyValue::StringValue(default_pos) = prop {
                let mut split = default_pos.split(',');
                world.default_pos = Some(
                    (
                        split.next().unwrap_or("0").parse::<i32>().unwrap_or(0),
                        split.next().unwrap_or("0").parse::<i32>().unwrap_or(0)
                    )
                );
            }
        }

        if let Some(prop) = map.properties.get("edges") {
            if let PropertyValue::StringValue(edges) = prop {
                let parsed = json::parse(&edges.as_str()).unwrap();
                if !parsed["down"].is_null() {
                    world.side_actions[1] = (false, Some(game::parse_action(&parsed["down"]).expect("failed to parse down screen transition action")));
                }
                if !parsed["up"].is_null() {
                    world.side_actions[0] = (false, Some(game::parse_action(&parsed["up"]).expect("failed to parse up screen action")));
                }
                if !parsed["left"].is_null() {
                    world.side_actions[2] = (false, Some(game::parse_action(&parsed["left"]).expect("failed to parse left screen transition action")));
                }
                if !parsed["right"].is_null() {
                    world.side_actions[3] = (false, Some(game::parse_action(&parsed["right"]).expect("failed to parse right screen action")));
                }
            }
        }

        if let Some(prop) = map.properties.get("looping") {
            if let PropertyValue::BoolValue(looping) = prop {
                world.looping = *looping;
            }
        }

        if let Some(prop) = map.properties.get("music") {
            if let PropertyValue::StringValue(song) = prop {
                if old_world.is_some() && old_world.as_ref().unwrap().song.is_some() && old_world.as_ref().unwrap().song.as_ref().unwrap().path == PathBuf::from(song) {
                    world.song = old_world.as_mut().unwrap().song.take();
                    world.song.as_mut().unwrap().default_speed = 1.0;
                    world.song.as_mut().unwrap().default_volume = 1.0;
                    world.song.as_mut().unwrap().dirty = true;
                } else {
                    world.song = Some(
                        Song::new(PathBuf::from(song))
                    );
                }
            }
        }

        if let Some(prop) = map.properties.get("music_speed") {
            if let PropertyValue::FloatValue(speed) = prop {
                if let Some(song) = &mut world.song {
                    song.speed = *speed;
                    song.default_speed = *speed;
                    song.dirty = true;
                }
            }
        }

        if let Some(prop) = map.properties.get("music_volume") {
            if let PropertyValue::FloatValue(volume) = prop {
                if let Some(song) = &mut world.song {
                    song.volume = *volume;
                    song.default_volume = *volume;
                    song.dirty = true;
                }
            }
        }

        if let Some(prop) = map.properties.get("tint") {
            if let PropertyValue::StringValue(tint_color) = prop {
                let mut color = tint_color.split(',');
                world.tint = Some(Color::RGBA(
                    color.next().unwrap().parse::<u8>().unwrap(), 
                    color.next().unwrap().parse::<u8>().unwrap(), 
                    color.next().unwrap().parse::<u8>().unwrap(), 
                    color.next().unwrap().parse::<u8>().unwrap(), 
                ));
            }
        }

        assert!(!map.infinite(), "Infinite maps not supported");
        assert!(matches!(map.orientation, Orientation::Orthogonal), "Non-orthogonal orientations not supported");

        for tileset in map.tilesets().iter() {
            let mut ts = Tileset::new_with_tile_size(
                Texture::from_file(&tileset.as_ref().image.as_ref().expect("tileset has no source image").source, creator).expect("failed to load tileset texture"),
                tileset.tile_width, tileset.tile_height
            );
            ts.name = Some(tileset.name.clone());
            world.tilesets.push(ts);
        }

        for layer in map.layers().into_iter() {
            match layer.layer_type() {
                LayerType::Tiles(tile_layer) => {
                    if let TileLayer::Finite(finite_tile_layer) = tile_layer {
                        let mut tilemap = Tilemap::new(map.width, map.height);
                        for j in 0..map.height {
                            for i in 0..map.width {
                                let tile_opt = finite_tile_layer.get_tile(i as i32, j as i32);
                                if let Some(tile) = tile_opt {
                                    if tile.get_tile().is_none() { continue; }
                                    if let Some(prop) = tile.get_tile().unwrap().properties.get("animation") {
                                        if let PropertyValue::StringValue(animation) = prop {
                                            match parse_animator(&json::parse(&animation).expect("failed to parse tile animator json"), tile.tileset_index() as u32) {
                                                Ok(animator) => {
                                                    let mut entity = Entity::new();
                                                    entity.animator = Some(animator);
                                                    if let Some(prop) = tile.get_tile().unwrap().properties.get("blocking") {
                                                        if let PropertyValue::BoolValue(blocking) = prop {
                                                            entity.solid = *blocking;
                                                        }
                                                    }
                                                    entity.x = i as i32 * 16;
                                                    entity.y = j as i32 * 16 - 16;
                                                    entity.tileset = tile.tileset_index() as u32;
                                                    entity.id = tile.id();
                                                    entity.draw = true;
                                                    world.add_entity(entity);
                                                },
                                                Err(e) => {
                                                    eprintln!("{}", e);
                                                }
                                            }
                                            continue;
                                        }
                                    }

                                    tilemap.set_tile(i, j, Tile::from_tiled(tile)).unwrap();
                                    if let Some(prop) = tile.get_tile().unwrap().properties.get("blocking") {
                                        if let PropertyValue::BoolValue(blocking) = prop {
                                            tilemap.set_collision(i, j, *blocking);
                                        }
                                    }

                                    if let Some(prop) = tile.get_tile().unwrap().properties.get("step") {
                                        if let PropertyValue::StringValue(step) = prop {
                                            tilemap.set_special(i, j, SpecialTile::Step(step.clone(), 0.25));
                                        }
                                    }

                                    if let Some(prop) = tile.get_tile().unwrap().properties.get("step_volume") {
                                        if let PropertyValue::FloatValue(step_volume) = prop {
                                            let sound = tilemap.get_special(i, j).map(|f| {
                                                if let SpecialTile::Step(step, _) = f {
                                                    return step.clone()
                                                } else {
                                                    return "step".to_string()
                                                }
                                            }).unwrap_or("step".to_string());
                                            let new_tile = SpecialTile::Step(sound, *step_volume);
                                            tilemap.set_special(i, j, new_tile);
                                        }
                                    }

                                    if let Some(prop) = tile.get_tile().unwrap().properties.get("stairs") {
                                        if let PropertyValue::BoolValue(stairs) = prop {
                                            if *stairs {
                                                tilemap.set_special(i, j, SpecialTile::Stairs);
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // Loading - Layer Properties

                        let mut world_layer = Layer::new(tilemap);
                        if let Some(prop) = layer.properties.get("height") {
                            if let PropertyValue::IntValue(height) = prop {
                                world_layer.height = *height;
                            } 
                        }

                        if let Some(prop) = layer.properties.get("draw") {
                            if let PropertyValue::BoolValue(draw) = prop {
                                world_layer.draw = *draw;
                            }
                        }

                        if let Some(prop) = layer.properties.get("collide") {
                            if let PropertyValue::BoolValue(collide) = prop {
                                world_layer.collide = *collide;
                            }
                        }

                        world_layer.name = layer.name.clone();
                        if let Some(prop) = layer.properties.get("name")  {
                            if let PropertyValue::StringValue(name) = prop {
                                world_layer.name = name.clone();
                            }
                        }

                        world.add_layer(world_layer);
                    } else {
                        eprintln!("Infinite layers not supported");
                    }
                },
                LayerType::Objects(object_layer) => {
                    for object in object_layer.objects().into_iter() {
                        if let Some(tile_obj) = object.get_tile() {
                            if let TilesetLocation::Map(tileset_id) = tile_obj.tileset_location() {
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
                                    walk_behind: true,
                                    ai: None,
                                    animator: None,
                                    movement: None,
                                    interaction: None
                                };

                                // justification for clone call - i need to mutate the properties and this shouldnt be called more than like 100 times
                                let mut properties = object.properties.clone();

                                let filename = if let Some(prop) = properties.get("file") { if let PropertyValue::StringValue(file) = prop { Some(file) } else { None } } else { None };

                                if let Some(properties_filename) = filename {
                                    let file = fs::File::open(properties_filename);
                                    match file {
                                        Ok(mut f) => {
                                            let mut source = String::new();
                                            f.read_to_string(&mut source).unwrap();
                                            match json::parse(&source) {
                                                Ok(mut v) => {
                                                    json_to_properties(&mut properties, &mut v);
                                                },
                                                Err(e) => {
                                                    eprintln!("Error parsing properties file: {}", e);
                                                }
                                            }
                                        },
                                        Err(e) => {
                                            eprintln!("Error opening properties file: {}", e);
                                        }
                                    }
                                }
                                

                                if let Some(prop) = properties.get("height") { if let PropertyValue::IntValue(height) = prop { entity.height = *height; } }
                                if let Some(prop) = properties.get("solid") { if let PropertyValue::BoolValue(solid) = prop { entity.solid = *solid; } }
                                if let Some(prop) = properties.get("draw") { if let PropertyValue::BoolValue(draw) = prop { entity.draw = *draw; } }
                                if let Some(prop) = properties.get("walk_behind") { if let PropertyValue::BoolValue(walk_behind) = prop { 
                                    entity.walk_behind = *walk_behind; 
                                    
                                    // The entity is raised one layer for drawing above the player
                                    // and if the entity is in the top layer, it would otherwise not be drawn
                                    // so the world's depth is changed to accommodate
                                    world.layer_max = world.layer_max.max(entity.height + 1);
                                } }
                                if let Some(prop) = properties.get("collider") { if let PropertyValue::StringValue(collider) = prop { entity.collider = parse_rect(&json::parse(collider).unwrap()) } }
                                if let Some(prop) = properties.get("ai") { if let PropertyValue::StringValue(ai) = prop { entity.ai = Some(ai::parse_ai(&json::parse(ai).unwrap()).unwrap()) } }
                                if let Some(prop) = properties.get("animation") { if let PropertyValue::StringValue(animation) = prop { entity.animator = Some(ai::parse_animator(&json::parse(&animation).unwrap(), *tileset_id as u32).unwrap()) } }

                                let mut actions_vec = Vec::new();
                                if let Some(prop) = properties.get("actions") {
                                    if let PropertyValue::StringValue(actions) = prop {
                                        let mut parsed = json::parse(actions).expect("failed to parse actions");
                                        if parsed.is_array() {

                                            let mut cur_action = parsed.pop();
                                            while !cur_action.is_null() {
                                                let mut trigger = None;
                                                let mut action = None;

                                                if cur_action["trigger"].is_object() {
                                                    trigger = Some(parse_trigger(&mut cur_action["trigger"]).expect("failed to parse trigger"));
                                                }
                                                if cur_action["action"].is_object() {
                                                    action = Some(parse_action(&cur_action["action"]).expect("failed to parse action"));
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

                                                cur_action = parsed.pop();
                                            }
                                        } else {
                                            eprintln!("Warning: Object actions property is not an array");
                                        }
                                    }
                                }

                                entity.actions = actions_vec;

                                world.add_entity(entity);
                            }
                        }
                    }
                },
                LayerType::Image(image_layer) => {
                    if let Some(image) = &image_layer.image {
                        let mut world_image_layer = ImageLayer::load_from_file(&image.source, creator);
                        if let Some(prop) = layer.properties.get("looping") { if let PropertyValue::BoolValue(_b) = prop { world_image_layer.looping_x = true; world_image_layer.looping_y = true; } };
                        if let Some(prop) = layer.properties.get("looping_x") { if let PropertyValue::BoolValue(_b) = prop { world_image_layer.looping_x = true; } };
                        if let Some(prop) = layer.properties.get("looping_y") { if let PropertyValue::BoolValue(_b) = prop { world_image_layer.looping_y = true; } };
                        if let Some(prop) = layer.properties.get("scroll_x") { if let PropertyValue::IntValue(i) = prop { world_image_layer.scroll_x = *i; } };
                        if let Some(prop) = layer.properties.get("scroll_y") { if let PropertyValue::IntValue(i) = prop { world_image_layer.scroll_y = *i; } };
                        if let Some(prop) = layer.properties.get("x") { if let PropertyValue::IntValue(i) = prop { world_image_layer.x = *i; } };
                        if let Some(prop) = layer.properties.get("y") { if let PropertyValue::IntValue(i) = prop { world_image_layer.y = *i; } };
                        if let Some(prop) = layer.properties.get("delay_x") { if let PropertyValue::IntValue(i) = prop { world_image_layer.delay_x = *i as u32; world_image_layer.timer_x = *i; } };
                        if let Some(prop) = layer.properties.get("delay_y") { if let PropertyValue::IntValue(i) = prop { world_image_layer.delay_y = *i as u32; world_image_layer.timer_y = *i; } };
                        if let Some(prop) = layer.properties.get("mismatch") { if let PropertyValue::BoolValue(b) = prop { if *b { world_image_layer.timer_x /= 2; } } }
                        if let Some(prop) = layer.properties.get("parallax_x") { if let PropertyValue::IntValue(i) = prop { world_image_layer.parallax_x = *i; } };
                        if let Some(prop) = layer.properties.get("parallax_y") { if let PropertyValue::IntValue(i) = prop { world_image_layer.parallax_y = *i; } };
                        world.image_layers.push(world_image_layer);
                    }
                }
                _ => println!("Unsupported layer type")
            }
        }

        if world.looping {
            world.render_texture = Some(creator.create_texture(Some(PixelFormatEnum::RGBA8888), TextureAccess::Target, world.width * 16, world.height * 16).expect("failed to create render texture for looping level"));
            world.render_texture.as_mut().unwrap().set_blend_mode(sdl2::render::BlendMode::Blend);
        }

        return world;
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
        PropertyValue::BoolValue(b) => { return JsonValue::Boolean(*b) },
        PropertyValue::ColorValue(c) => { 
            let mut color = JsonValue::new_object();
            color["r"] = JsonValue::Number(c.red.into());
            color["g"] = JsonValue::Number(c.green.into());
            color["b"] = JsonValue::Number(c.blue.into());
            color["a"] = JsonValue::Number(c.alpha.into());
            return color
        },
        PropertyValue::FileValue(f) => { return JsonValue::String(f.clone()); },
        PropertyValue::FloatValue(n) => { return JsonValue::Number((*n).into()); },
        PropertyValue::IntValue(n) => { return JsonValue::Number((*n).into()); },
        PropertyValue::ObjectValue(obj) => { return JsonValue::Number((*obj).into()); },
        PropertyValue::StringValue(s) => { return JsonValue::String(s.clone()); }
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

    return None;
}