use std::{path::PathBuf, fmt::Debug};

use sdl2::{surface::Surface, render::TextureCreator, image::LoadSurface};

pub struct Texture<'a> {
    pub texture: sdl2::render::Texture<'a>,
    pub width: u32,
    pub height: u32,
}

impl Debug for Texture<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "texture with dims: {}x{}", self.width, self.height)
    }
}

impl<'a> Texture<'a> {
    pub fn new<T>(surface: Surface<'a>, creator: &'a TextureCreator<T>) -> Self {
        let surf_width: u32 = surface.width();
        let surf_height: u32 = surface.height();
        
        Self {
            texture: creator.create_texture_from_surface(surface).map_err(|e| format!("failed to load texture: {}", e)).unwrap(),
            height: surf_height,
            width: surf_width
        }
    }

    pub fn from_file<T>(file: &PathBuf, creator: &'a TextureCreator<T>) -> Result<Self, String> {
        let surface = Surface::from_file(file);
        if let Ok(surf) = surface {
            Ok(Self::new(surf, creator))
        } else {
            Err(surface.err().unwrap_or("failed to load texture".to_string()))
        }
    }

    pub fn empty<T>(creator: &'a TextureCreator<T>) -> Self {
        Self {
            texture: creator.create_texture(None, sdl2::render::TextureAccess::Static, 1, 1).unwrap(),
            width: 0,
            height: 0
        }
    }
}