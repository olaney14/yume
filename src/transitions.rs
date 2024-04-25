use std::{f32::consts::PI, path::PathBuf};

use json::JsonValue;
use sdl2::{pixels::Color, rect::Rect, render::{Canvas, RenderTarget, TextureCreator}};

use crate::{game::RenderState, player::Player, texture::Texture, world::World};

#[derive(Clone)]
pub enum TransitionType {
    Fade,
    FadeToColor(u32, u32, u32),
    MusicOnly,
    Spotlight,
    FadeScreenshot,
    Spin,
    Zoom(f32),
    Pixelate,
    Lines(u32),
    Wave(bool, u32),
    GridCycle,
    PlayerFall
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
            "fade_to_color" => Some(Self::FadeToColor(0, 0, 0)),
            "music_only" => Some(Self::MusicOnly),
            "spotlight" => Some(Self::Spotlight),
            "spin" => Some(Self::Spin),
            "zoom" => Some(Self::Zoom(1.0)),
            //"zoom_fade" => Some(Self::ZoomFade(1.0)),
            "pixelate" => Some(Self::Pixelate),
            "lines" => Some(Self::Lines(1)),
            "wave" => Some(Self::Wave(false, 10)),
            "grid_cycle" => Some(Self::GridCycle),
            "player_fall" => Some(Self::PlayerFall),
            _ => None
        }
    }
}

pub struct TransitionTextures<'a> {
    pub spotlight: Texture<'a>,
    // TODO: move this outta here
    pub raindrop: Texture<'a>
}

impl <'a> TransitionTextures<'a> {
    pub fn new<T>(creator: &'a TextureCreator<T>) -> Result<Self, String> {
        let spotlight = Texture::from_file(&PathBuf::from("res/textures/image/spotlight.png"), creator)?;
        let raindrop = Texture::from_file(&PathBuf::from("res/textures/misc/drop.png"), creator)?;
        Ok(Self {
                    spotlight,
                    raindrop
                })
    }

    pub fn empty<T>(creator: &'a TextureCreator<T>) -> Self {
        Self {
            spotlight: Texture::empty(creator),
            raindrop: Texture::empty(creator)
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
    pub draw_player: bool,
}

impl Transition {
    pub fn new(kind: TransitionType, speed: i32, delay: i32, fade_music: bool, hold: u32) -> Self {
        let needs_screenshot = match &kind {
            TransitionType::FadeScreenshot | TransitionType::Spin | TransitionType::Lines(..) | TransitionType::Pixelate | TransitionType::Zoom(..) | TransitionType::Wave(..) => true,
            _ => false
        };

        let draw_player = match &kind {
            TransitionType::PlayerFall => false,
            _ => true
        };

        Self {
            direction: 1,
            progress: 0,
            fade_music, kind, speed,
            hold, holding: false, hold_timer: hold,
            needs_screenshot,
            delay, delay_timer: 0,
            draw_player
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
                    },
                    TransitionType::FadeToColor(..) => {
                        let r = json["r"].as_u32().expect("no `r` value for fade to color transition");
                        let g = json["g"].as_u32().expect("no `g` value for fade to color transition");
                        let b = json["b"].as_u32().expect("no `b` value for fade to color transition");

                        return Some(
                            Self::new(TransitionType::FadeToColor(r, g, b), speed, 0, music, hold)
                        )
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

    pub fn draw<T: RenderTarget>(&mut self, canvas: &mut Canvas<T>, world: &mut World, player: &Player, state: &RenderState) {
        if self.needs_screenshot {
            world.transition_context.take_screenshot = true;
            self.needs_screenshot = false;
            return;
        }

        if !self.draw_player {
            world.draw_player = false;
        }

        match self.kind {
            TransitionType::Fade => {
                let alpha = (255.0 * (self.progress as f32 / 100.0)).clamp(0.0, 255.0) as u8;
                canvas.set_blend_mode(sdl2::render::BlendMode::Blend);
                canvas.set_draw_color(Color::RGBA(0, 0, 0, alpha));
                canvas.fill_rect(None).unwrap();
            },
            TransitionType::FadeToColor(r, g, b) => {
                let alpha = (255.0 * (self.progress as f32 / 100.0)).clamp(0.0, 255.0) as u8;
                canvas.set_blend_mode(sdl2::render::BlendMode::Blend);
                canvas.set_draw_color(Color::RGBA(r.clamp(0, 255) as u8, g.clamp(0, 255) as u8, b.clamp(0, 255) as u8, alpha));
                canvas.fill_rect(None).unwrap();
            }
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
            },
            TransitionType::PlayerFall => {
                let progress = (100.0 * (self.progress as f32 / 100.0)) as i32;

                let x;
                let mut y;

                let source = (16, 32);

                if state.clamp.0 {
                    x = player.x + state.offset.0;
                } else {
                    x = (state.screen_extents.0 as i32 / 2) - 8;
                }
        
                if self.direction == 1 {
                    if state.clamp.1 {
                        y = player.y + state.offset.1;
                    } else {
                        y = (state.screen_extents.1 as i32 / 2) - 16;
                    }

                    y += ((progress as f32 / 100.0) * (state.screen_extents.1 as f32 / 2.0) * 1.5) as i32;
                } else {
                    if state.clamp.1 {
                        y = player.y + state.offset.1;
                    } else {
                        y = (state.screen_extents.1 as i32 / 2) - 16;
                    }
                    
                    y -= ((state.screen_extents.1 as f32 / 2.0) * 1.5) as i32;
                    y += ((1.0 - (progress as f32 / 100.0)) * (state.screen_extents.1 as f32 / 2.0) * 1.5) as i32;
                }
                
                if player.current_effect.is_some() {
                    if let Some(texture) = player.effect_textures.get(player.current_effect.as_ref().unwrap()) {
                        canvas.copy(&texture.texture, Rect::new(source.0 as i32, source.1 as i32, 16, 32), Rect::new(x, y, 16, 32)).unwrap();
                    } else {
                        canvas.copy(&player.texture.texture, Rect::new(source.0 as i32, source.1 as i32, 16, 32), Rect::new(x, y, 16, 32)).unwrap();
                    }
                    
                } else {
                    canvas.copy(&player.texture.texture, Rect::new(source.0 as i32, source.1 as i32, 16, 32), Rect::new(x, y, 16, 32)).unwrap();
                }

                let alpha = (255.0 * (self.progress as f32 / 100.0)).clamp(0.0, 255.0) as u8;
                canvas.set_blend_mode(sdl2::render::BlendMode::Blend);
                canvas.set_draw_color(Color::RGBA(0, 0, 0, alpha));
                canvas.fill_rect(None).unwrap();
            }
        }
    }
}