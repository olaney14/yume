use core::fmt;
use std::path::PathBuf;

use sdl2::{render::{Canvas, TextureCreator, RenderTarget}, rect::Rect};
use serde_derive::{Deserialize, Serialize};
use crate::{game::Direction, texture::Texture};

#[derive(Debug)]
pub struct Tileset<'a> {
    pub texture: Texture<'a>,
    pub tiles_width: u32,
    pub tiles_height: u32,
    pub total_tiles: u32,
    pub tile_width: u32,
    pub tile_height: u32,
    pub name: Option<String>,
}

impl<'a> Tileset<'a> {
    pub fn new(texture: Texture<'a>) -> Self {
        let width = texture.width;
        let height = texture.height;
        Self {
            texture,
            tiles_width: width / 16,
            tiles_height: height / 16,
            total_tiles: (width * height) / 256,
            tile_height: 16,
            tile_width: 16,
            name: None
        }
    }

    pub fn new_with_tile_size(texture: Texture<'a>, tile_width: u32, tile_height: u32) -> Self {
        let width = texture.width;
        let height = texture.height;
        
        Self {
            texture,
            tiles_width: width / tile_width,
            tiles_height: height / tile_height,
            tile_width,
            tile_height,
            total_tiles: (width / tile_width) * (height / tile_height),
            name: None
        }
    }

    pub fn load_from_file<T>(file: &PathBuf, creator: &'a TextureCreator<T>) -> Self {
        let texture = 
            Texture::from_file(file, creator).map_err(|e| format!("failed to load tileset image: {}", e)).unwrap();
        let mut tileset = Tileset::new(texture);
        if let Some(stem) = file.file_stem() {
            tileset.name = Some(stem.to_str().unwrap().to_string());
        }
        tileset
    }

    pub fn draw_tile<T: RenderTarget>(&self, canvas: &mut Canvas<T>, tile: u32, pos: (i32, i32)) {
        let tile_x = tile % self.tiles_width;
        let tile_y = tile / self.tiles_width;
        canvas.copy(&self.texture.texture, Rect::new(tile_x as i32 * 16, tile_y as i32 * 16, 16, 16), Rect::new(pos.0, pos.1, 16, 16)).unwrap();
    }

    pub fn draw_tile_sized<T: RenderTarget>(&self, canvas: &mut Canvas<T>, tile: u32, pos: (i32, i32)) {
        let tile_x = tile % self.tiles_width;
        let tile_y = tile / self.tiles_width;
        canvas.copy(
            &self.texture.texture,
            Rect::new((tile_x * self.tile_width) as i32, (tile_y * self.tile_height) as i32, self.tile_width, self.tile_height),
            Rect::new(pos.0, pos.1, self.tile_width, self.tile_height)
        ).unwrap();
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub enum SpecialTile {
    Stairs,
    Step(String, f32),
    NoRain,
    SpeedMod(i32),
    Ladder,
    Exits(TileExits)
}

#[derive(Clone, Serialize, Deserialize)]
pub enum TileExits {
    Up,
    Down,
    Left,
    Right,
    Horizontal,
    Vertical,
    NotUp,
    NotDown,
    NotLeft,
    NotRight,
    ULCorner,
    URCorner,
    DLCorner,
    DRCorner,
    All,
    None
}

impl TileExits {
    pub fn parse(from: &str) -> Self {
        match from.to_lowercase().as_str() {
            "up" | "top" => Self::Up,
            "down" | "bottom" => Self::Down,
            "left" => Self::Left,
            "right" => Self::Right,
            "horizontal" | "horiz" | "x" => Self::Horizontal,
            "vertical" | "vert" | "y" => Self::Vertical,
            "notup" | "not_up" | "nottop" | "not_top" => Self::NotUp,
            "notdown" | "not_down" | "notbottom" | "not_bottom" => Self::NotDown,
            "notleft" | "not_left" => Self::NotLeft,
            "notright" | "not_right" => Self::NotRight,
            "any" | "all" => Self::All,
            "none" => Self::None,
            "corner_ul" => Self::ULCorner,
            "corner_ur" => Self::URCorner,
            "corner_dl" => Self::DLCorner,
            "corner_dr" => Self::DRCorner,
            _ => {
                eprintln!("Unknown tile exits `{}`", from);
                Self::All
            }
        }
    }

    pub fn can_pass(&self, direction: &Direction) -> bool {
        match self {
            Self::Up => matches!(direction, Direction::Up),
            Self::Down => matches!(direction, Direction::Down),
            Self::Left => matches!(direction, Direction::Left),
            Self::Right => matches!(direction, Direction::Right),
            Self::Horizontal => matches!(direction, Direction::Left | Direction::Right),
            Self::Vertical => matches!(direction, Direction::Up | Direction::Down),
            Self::All => true,
            Self::None => false,
            Self::NotUp => !matches!(direction, Direction::Up),
            Self::NotDown => !matches!(direction, Direction::Down),
            Self::NotLeft => !matches!(direction, Direction::Left),
            Self::NotRight => !matches!(direction, Direction::Right),
            Self::ULCorner => matches!(direction, Direction::Down | Direction::Right),
            Self::URCorner => matches!(direction, Direction::Down | Direction::Left),
            Self::DLCorner => matches!(direction, Direction::Up | Direction::Right),
            Self::DRCorner => matches!(direction, Direction::Up | Direction::Left),
        }
    }
}

pub struct Tilemap {
    pub width: u32,
    pub height: u32,
    pub tiles: Vec<Tile>,
    pub collision: Vec<bool>,
    pub special: Vec<Option<SpecialTile>>
}

#[derive(Debug, Clone, Copy)]
pub struct Tile {
    pub id: i32,
    pub tileset: i32,
}

impl Tile {
    pub fn new(id: i32, tileset: i32) -> Self {
        Self { id, tileset }
    }

    pub fn from_tiled(tile: tiled::LayerTile) -> Self {
        Self { id: tile.id() as i32, tileset: tile.tileset_index() as i32 }
    }
}

impl Tilemap {
    pub fn new(width: u32, height: u32) -> Self {
        let mut tiles = Vec::with_capacity((width * height).try_into().expect("tilemap too large"));
        let mut collision = Vec::with_capacity((width * height).try_into().unwrap());
        let mut special = Vec::with_capacity((width * height).try_into().unwrap());
        for _ in 0..(width * height) {
            tiles.push(Tile::new(-1, -1));
            collision.push(false);
            special.push(None);
        }

        Self {
            width,
            height,
            tiles,
            collision,
            special
        }
    }
    
    pub fn set_tile(&mut self, x: u32, y: u32, tile: Tile) -> Result<(), TileError> {
        if x >= self.width || y >= self.height {
            return Err(TileError::OutOfBounds(x, y));
        }
        
        self.tiles[(y * self.width + x) as usize] = tile;

        Ok(())
    }

    pub fn get_tile(&mut self, x: u32, y: u32) -> Result<Tile, TileError> {
        if x >= self.width || y >= self.height {
            return Err(TileError::OutOfBounds(x, y));
        }

        Ok(self.tiles[(y * self.width + x) as usize])
    }

    pub fn get_collision(&self, x: u32, y: u32) -> bool {
        if x >= self.width || y >= self.height {
            return true;
        }

        return self.collision[(y * self.width + x) as usize];
    }

    pub fn get_special(&self, x: u32, y: u32) -> Option<&SpecialTile> {
        if x >= self.width || y >= self.height {
            return None;
        }

        return self.special[(y * self.width + x) as usize].as_ref();
    }

    pub fn get_collision_with_rect(&self, rect: Rect) -> bool {
        // inefficient but more complexity isnt really necessary
        for y in 0..self.height {
            for x in 0..self.width {
                if self.collision[(y * self.width + x) as usize] {
                    let tile_rect = Rect::new(x as i32 * 16, y as i32 * 16, 16, 16);
                    if rect.has_intersection(tile_rect) { return true; }
                }
            }
        }

        return false;
    }

    pub fn set_collision(&mut self, x: u32, y: u32, state: bool) {
        if !(x >= self.width || y >= self.height) {
            self.collision[(y * self.width + x) as usize] = state;
        }
    }

    pub fn set_special(&mut self, x: u32, y: u32, special: SpecialTile) {
        if !(x >= self.width || y >= self.height) {
            self.special[(y * self.width + x) as usize] = Some(special);
        }
    }
}

#[derive(Debug)]
pub enum TileError {
    OutOfBounds(u32, u32),
}

impl fmt::Display for TileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            TileError::OutOfBounds(x, y) => write!(f, "Out of Bounds at ({}, {})", x, y)
        }
    }
}