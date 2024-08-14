use std::{collections::HashMap, ffi::OsStr, fs::{self, File}, os, path::{Path, PathBuf}, rc::Rc, sync::Arc};

use json::object;
use sdl2::{image::{LoadSurface, SaveSurface}, rect::Rect, render::TextureCreator, surface::Surface};
use serde::{de::Visitor, ser::SerializeTuple, Deserialize, Serialize};
use serde_derive::{Deserialize, Serialize};
use tiled::{ImageLayer, Layer, LayerType, Loader, PropertyValue, TilesetLocation};

use crate::{entity::Entity, game::RenderState, texture, tiles::{self, SpecialTile, Tileset}, world::{self, World}};

#[derive(Serialize, Deserialize)]
enum SerializablePropertyValue {
    BoolValue(bool),
    FloatValue(f32),
    IntValue(i32),
    ColorValue([u8; 4]),
    StringValue(String),
    FileValue(String),
    ObjectValue(u32)
}

#[derive(Serialize, Deserialize)]
struct OptimizedEntity {
    id: u32,
    tileset: u32,
    x: i32,
    y: i32,
    properties: Vec<(String, SerializablePropertyValue)>
}

#[derive(Serialize, Deserialize)]
struct OptimizedSong {
    speed: f32,
    volume: f32,
    path: String,
}

#[derive(Serialize, Deserialize)]
struct OptimizedImageLayer {
    image: String,
    x: i32,
    y: i32,
    looping_x: bool,
    looping_y: bool,
    scroll_x: i32,
    scroll_y: i32,
    height: i32,
    draw: bool,
    delay_x: u32,
    delay_y: u32,
    timer_x: i32,
    timer_y: i32,
    parallax_x: i32,
    parallax_y: i32,
    parallax_mode: bool
}

#[derive(Serialize, Deserialize)]
struct OptimizedTilemap {
    width: u32,
    height: u32,
    tiles: Vec<i32>,
    collision: Vec<bool>,
    special: Vec<Option<SpecialTile>>
}

#[derive(Serialize, Deserialize)]
struct OptimizedLayer {
    height: i32,
    map: OptimizedTilemap,
    draw: bool,
    collide: bool,
    name: String,
}

#[derive(Serialize, Deserialize)]
struct OptimizedMap {
    layers: Vec<OptimizedLayer>,
    image_layers: Vec<OptimizedImageLayer>,
    tilesets: Vec<String>,
    layer_min: i32,
    layer_max: i32,
    width: u32,
    height: u32,
    background_color: [u8; 4],
    clamp_camera: bool,
    clamp_camera_axes: Option<world::Axis>,
    side_actions: Option<String>, // json
    looping: bool,
    looping_axes: Option<world::Axis>,
    song: Option<OptimizedSong>,
    tint: Option<[u8; 4]>,
    entities: Vec<OptimizedEntity>,
    default_pos: Option<(i32, i32)>,
    name: String,
    raindrops: bool,
    snow: bool,
    source_file: String
}

#[derive(Serialize, Deserialize)]
struct OptimizedTileset {
    texture: PathBuf,
    tile_width: u32,
    tile_height: u32,
    name: String
}

fn tile_pixel(x: u32, y: u32, tile_x: u32, tile_y: u32, tile_width: u32, tile_height: u32, image_width: u32) -> usize {
    ((((tile_y * tile_height) + y) * image_width + ((tile_x * tile_width) + x)) * 4) as usize
}

fn tiles_equal(data: &[u8], tile_x0: u32, tile_y0: u32, tile_x1: u32, tile_y1: u32, tile_width: u32, tile_height: u32, image_width: u32) -> bool {
    for y in 0..tile_height {
        for x in 0..tile_width {
            let px1 = tile_pixel(x, y, tile_x0, tile_y0, tile_width, tile_height, image_width);
            let px2 = tile_pixel(x, y, tile_x1, tile_y1, tile_width, tile_height, image_width);
            let data1 = ((data[px1] as u32) << 24) + ((data[px1 + 1] as u32) << 16) + ((data[px1 + 2] as u32) << 8) + (data[px1 + 3] as u32);
            let data2 = ((data[px2] as u32) << 24) + ((data[px2 + 1] as u32) << 16) + ((data[px2 + 2] as u32) << 8) + (data[px2 + 3] as u32);
            if data1 != data2 {
                return false;
            }
        }
    }

    true
}

fn tile_empty(data: &[u8], tile_x: u32, tile_y: u32, tile_width: u32, tile_height: u32, image_width: u32) -> bool {
    for y in 0..tile_height {
        for x in 0..tile_width {
            let px = tile_pixel(x, y, tile_x, tile_y, tile_width, tile_height, image_width);
            if data[px + 3] != 0x00 {
                return false;
            }
        }
    }

    return true
}

pub fn optimize_tileset(to: &PathBuf, tiled_tileset: &Arc<tiled::Tileset>) -> Result<HashMap<u32, i32>, Box<dyn std::error::Error>> {
    let surface = Surface::from_file(&tiled_tileset.image.as_ref().unwrap().source)?;
    //let new_surface = Surface::new(surface.width(), surface.height(), surface.pixel_format_enum())?;
    let image_width = surface.width();
    let image_height = surface.height();
    let width = image_width / tiled_tileset.tile_width;
    let height = image_height / tiled_tileset.tile_height;
    
    let mut copy = Vec::new();
    let mut update_map = HashMap::new();

    surface.with_lock(|f| {
        for tile_y in 0..height {
            'tile: for tile_x in 0..width {
                if tile_empty(f, tile_x, tile_y, tiled_tileset.tile_width, tiled_tileset.tile_height, image_width) {
                    update_map.insert(tile_y * width + tile_x, -1);
                    continue;
                }

                for compare_tile_y in 0..=tile_y {
                    for compare_tile_x in 0..tile_x {
                        if tiles_equal(f, tile_x, tile_y, compare_tile_x, compare_tile_y, tiled_tileset.tile_width, tiled_tileset.tile_height, image_width) {
                            update_map.insert(tile_y * width + tile_x, (compare_tile_y * width + compare_tile_x) as i32);
                            continue 'tile;
                        }
                    }
                }

                copy.push((tile_x, tile_y));
            }
        }
    });

    for (i, copy_tile) in copy.iter().enumerate() {
        let id = copy_tile.1 * width + copy_tile.0;
        update_map.insert(id, i as i32);
    }

    // (dividend + divisor - 1) / divisor
    let mut new_surface = Surface::new(surface.width(), ((copy.len() as u32 + (width - 1)) / width) * tiled_tileset.tile_height, surface.pixel_format_enum())?;

    let mut next_slot = 0;
    for copy_tile in copy.iter() {
        let src = Rect::new((copy_tile.0 * tiled_tileset.tile_width) as i32, (copy_tile.1 * tiled_tileset.tile_height) as i32, tiled_tileset.tile_width, tiled_tileset.tile_height);
        let dest = Rect::new(((next_slot % width) * tiled_tileset.tile_width) as i32, ((next_slot / width) * tiled_tileset.tile_height) as i32, tiled_tileset.tile_width, tiled_tileset.tile_height);
        surface.blit(src, &mut new_surface, dest)?;
        next_slot += 1;
    }

    let mut image_path = PathBuf::from("res/textures/tiles/optimized/");
    image_path.push(format!("{}.png", tiled_tileset.name));
    new_surface.save(&image_path)?;

    let optimized_tileset = OptimizedTileset {
        name: tiled_tileset.name.clone(),
        texture: image_path,
        tile_width: tiled_tileset.tile_width,
        tile_height: tiled_tileset.tile_height
    };

    std::fs::create_dir_all(to.parent().unwrap())?;
    let mut tileset_file = File::create(to)?;
    serde_cbor::to_writer(&mut tileset_file, &optimized_tileset)?;

    Ok(update_map)
}

pub fn optimize_map<T>(to: &PathBuf, map: &PathBuf, optimized_tilesets: &mut HashMap<String, Rc<HashMap<u32, i32>>>, creator: &TextureCreator<T>) -> Result<(), Box<dyn std::error::Error>> {
    let mut loader = Loader::new();
    let tiled_map = loader.load_tmx_map(map).unwrap();
    let state = RenderState::new((2, 2));
    let world = World::load_from_file(&map.as_os_str().to_str().unwrap().to_owned(), creator, &mut None, &state)?;

    let tiled_tilesets = tiled_map.tilesets();

    //let mut new_tilesets = Vec::new();
    let mut tilesets = Vec::new();
    let mut tileset_update_maps: Vec<Rc<HashMap<u32, i32>>> = Vec::new();

    for world_tileset in world.tilesets.iter() {
        if optimized_tilesets.contains_key(world_tileset.name.as_ref().unwrap()) {
            tileset_update_maps.push(optimized_tilesets.get(world_tileset.name.as_ref().unwrap()).unwrap().clone());
            tilesets.push(format!("res/maps/optimized/tilesets/{}.set", world_tileset.name.as_ref().unwrap()));
            continue;
        }

        if let Some(tiled_tileset) = tiled_tilesets.iter().find(|t| { *t.name == *world_tileset.name.as_ref().unwrap() }) {
            let mut path = PathBuf::from("res/maps/optimized/tilesets/");
            path.push(format!("{}.set", tiled_tileset.name));
            //new_tilesets.push(tileset_update_maps.len());
            tilesets.push(path.as_os_str().to_str().unwrap().to_owned());
            let optimized_tileset = Rc::new(optimize_tileset(&path, tiled_tileset)?);
            tileset_update_maps.push(optimized_tileset.clone());
            optimized_tilesets.insert(tiled_tileset.name.clone(), optimized_tileset.clone());
        } else {
            return Err("could not find matching tiled tileset".into());
        }
    }

    let mut layers = Vec::new();

    for world_layer in world.layers.iter() {
        layers.push(OptimizedLayer::from_layer(world_layer, &tileset_update_maps));
    }

    let mut image_layers = Vec::new();

    for (world_image_layer, tiled_image_layer) in world.image_layers.iter().zip(tiled_map.layers().filter(|l| matches!(l.layer_type(), LayerType::Image(..)))) {
        image_layers.push(OptimizedImageLayer::from_image_layer(world_image_layer, tiled_image_layer.name.clone()));
    }

    let side_actions = if !world.side_actions.is_empty() {
        if let Some(PropertyValue::StringValue(edges)) = tiled_map.properties.get("edges") {
            Some(edges.clone())
        } else {
            None
        }
    } else {
        None
    }; 

    let song = if let Some(song) = world.song {
        Some(OptimizedSong {
            speed: song.default_speed,
            volume: song.default_volume,
            path: song.path.as_os_str().to_str().unwrap().into()            
        })
    } else {
        None
    };

    let mut entities = Vec::new();

    for entity_layer in tiled_map.layers().filter_map(|l| l.as_object_layer()) {
        for entity in entity_layer.objects() {
            if let Some(tile_obj) = entity.get_tile() {
                if let TilesetLocation::Map(tileset_id) = tile_obj.tileset_location() {
                    entities.push(OptimizedEntity {
                        id: entity.id(),
                        tileset: *tileset_id as u32,
                        x: entity.x as i32,
                        y: entity.y as i32,
                        properties: entity.properties.clone().into_iter().map(|(key, val)| {
                            (key, SerializablePropertyValue::from_tiled(val))
                        }).collect::<Vec<(String, SerializablePropertyValue)>>()
                    });
                }
                
            } 
        }
    }

    let World {
        background_color,
        clamp_camera,
        clamp_camera_axes,
        default_pos,
        height,
        layer_max,
        layer_min,
        looping,
        looping_axes,
        name,
        raindrops,
        snow,
        source_file,
        tint,
        width,
        ..
    } = world;

    let optimized = OptimizedMap {
        background_color: [background_color.r, background_color.g, background_color.b, background_color.a],
        clamp_camera,
        clamp_camera_axes,
        default_pos,
        entities,
        layers,
        image_layers,
        height,
        layer_max,
        layer_min,
        looping,
        looping_axes,
        name,
        raindrops: raindrops.enabled,
        snow: snow.enabled,
        side_actions,
        song,
        source_file: source_file.as_os_str().to_str().unwrap().to_owned(),
        tilesets,
        tint: tint.map(|c| { [c.r, c.b, c.g, c.a] }),
        width
    };

    std::fs::create_dir_all(to.parent().unwrap())?;
    let mut map_file = File::create(to)?;
    serde_cbor::to_writer(&mut map_file, &optimized)?;

    Ok(())
}

const OPT_PREFIX: &str = "OPTIMIZATION |";

pub fn optimize_all<T>(dir: &PathBuf, creator: &TextureCreator<T>) -> Result<(), Box<dyn std::error::Error>> {
    let mut optimized_tilesets = HashMap::new();

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let name = path.file_stem().unwrap_or(OsStr::new("error")).to_str().unwrap_or("error");

        if entry.file_type()?.is_dir() {
            optimize_all(&path, creator)?;
        } else {
            if let Some(extension) = path.extension() {
                if extension.to_str().unwrap_or("") == "tmx" {
                    let mut new_path = path.clone();
                    new_path = new_path.components().skip(2).collect::<PathBuf>();
                    new_path = PathBuf::from("res/maps/optimized/").join(new_path);
                    new_path.set_extension("map");

                    match optimize_map(&new_path, &path, &mut optimized_tilesets, creator).map_err(|err| {
                        format!("<{}> {}", name, err)
                    }) {
                        Ok(_) => {
                            println!("{} <{}> finished", OPT_PREFIX, name);
                        },
                        Err(e) => {
                            println!("{} <{}> ERROR: {}", OPT_PREFIX, name, e);
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

// struct Tiles<const N: usize> {
//     data: [i32; N]
// }

// impl Serialize for tiles::Tile {
//     fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
//         where
//             S: serde::Serializer {
//         let mut tuple = serializer.serialize_tuple(2)?;
//         tuple.serialize_element(&self.tileset)?;
//         tuple.serialize_element(&self.id)?;
//         tuple.end()
//     }
// }

// struct TileVisitor;

// impl<'de> Visitor<'de> for TileVisitor {
//     type Value = tiles::Tile;

//     fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
//         formatter.write_str("a 2 element tuple of i32s")
//     }

//     fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
//         where
//             A: serde::de::SeqAccess<'de>, {
//             Ok(tiles::Tile {
//                 tileset: seq.next_element::<i32>()?.unwrap(),
//                 id: seq.next_element::<i32>()?.unwrap()
//             })
//     }
// }

// impl<'de> Deserialize<'de> for tiles::Tile {
//     fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
//         where
//             D: serde::Deserializer<'de> {
//         deserializer.deserialize_seq(TileVisitor {})
//     }
// }

impl OptimizedTileset {
    fn into_tileset<T>(self, creator: &TextureCreator<T>) -> Result<tiles::Tileset, Box<dyn std::error::Error>> {
        let OptimizedTileset {
            name,
            texture,
            tile_height,
            tile_width
        } = self;

        let texture = texture::Texture::from_file(&texture, creator)?;
        let width = texture.width / tile_width;
        let height = texture.height / tile_height;

        Ok(tiles::Tileset {
            texture,
            tile_width: tile_width,
            tile_height: tile_height,
            tiles_width: width,
            tiles_height: height,
            total_tiles: width * height,
            name: Some(name)
        })
    }
}

// tile format:
// -1 at all means none
// TODO: rle
// main format is
// <tileset id> [<tile>; (width * height)]

impl OptimizedLayer {
    fn from_layer(layer: &world::Layer, tileset_update_maps: &Vec<Rc<HashMap<u32, i32>>>) -> Self {
        let mut map = OptimizedTilemap {
            width: layer.map.width,
            height: layer.map.height,
            collision: layer.map.collision.clone(),
            special: layer.map.special.clone(),
            tiles: Vec::new()
        };

        let mut tiles_raw = Vec::new();
        let mut last_tileset = -2;

        for tile in layer.map.tiles.iter() {
            if tile.id == -1 || tile.tileset == -1 {
                // map.tiles.push(tiles::Tile {
                //     id: -1, tileset: -1
                // });
                tiles_raw.push(-1);
                continue;
            }
            let mapped_id = *tileset_update_maps[tile.tileset as usize].get(&(tile.id as u32)).unwrap();
            if mapped_id == -1 {
                // map.tiles.push(tiles::Tile {
                //     id: -1, tileset: -1
                // });
                tiles_raw.push(-1);
                continue;
            }
            // let new_tile = tiles::Tile {
            //     tileset: tile.tileset,
            //     id: mapped_id
            // };

            // map.tiles.push(tile.tileset as i32);
            // map.tiles.push(mapped_id);
        }
        
        Self {
            collide: layer.collide,
            draw: layer.draw,
            height: layer.height,
            name: layer.name.clone(),
            map
        }
    }
}

impl OptimizedImageLayer {
    fn from_image_layer(layer: &world::ImageLayer, image: String) -> Self {
        Self {
            delay_x: layer.delay_x,
            delay_y: layer.delay_y,
            draw: layer.draw,
            height: layer.height,
            image,
            looping_x: layer.looping_x,
            looping_y: layer.looping_y,
            parallax_mode: layer.parallax_mode,
            parallax_x: layer.parallax_x,
            parallax_y: layer.parallax_y,
            scroll_x: layer.scroll_x,
            scroll_y: layer.scroll_y,
            timer_x: layer.scroll_x,
            timer_y: layer.timer_y,
            x: layer.x,
            y: layer.y
        }
    }
}

impl SerializablePropertyValue {
    fn from_tiled(property_value: PropertyValue) -> Self {
        match property_value {
            PropertyValue::BoolValue(b) => Self::BoolValue(b),
            PropertyValue::ColorValue(c) => { Self::ColorValue([c.red, c.green, c.blue, c.alpha]) },
            PropertyValue::FileValue(s) => { Self::FileValue(s) },
            PropertyValue::FloatValue(f) => { Self::FloatValue(f) },
            PropertyValue::IntValue(i) => { Self::IntValue(i) },
            PropertyValue::ObjectValue(o) => { Self::ObjectValue(o) },
            PropertyValue::StringValue(s) => { Self::StringValue(s) }
        }
    }
}