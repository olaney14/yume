use std::{fs, path::{Path, PathBuf}};

use sdl2::{keyboard::Keycode, pixels::Color, rect::Rect, render::{Canvas, RenderTarget, TextureCreator}};

use crate::{audio::SoundEffectBank, game::{Input, RenderState}, texture::Texture};

enum Continue {
    Use,
    Wait(u32)
}

struct ScreenEventStep {
    cont: Continue,
    step_type: ScreenEventStepType
}

enum ScreenEventStepType {
    HideGame(u32),
    ShowGame(u32),
    Animate { from: u32, to: u32, speed: u32},
    ShowFrame(u32),
    SetTextureVisible,
    SetTextureHidden,
    PlaySound { sound: String, volume: f32, speed: f32 },
    Warn(String),
    None,
    Mute(u32),
    Unmute(u32),
    Song { song: String, volume: f32, speed: f32 }
}

pub struct ScreenEvent<'a> {
    pub texture: Texture<'a>,
    pub can_exit: bool,
    pub freeze_player: bool,
    steps: Vec<ScreenEventStep>,
    pub running: bool,
    pub init: bool,
    pub current_step: usize,
    pub timer: u32,
    pub current_frame: u32,
    pub frame_width: u32,
    pub frame_height: u32,
    pub visible: bool,

    /// warning: this is not reflective of the total number of ticks elapsed
    pub ticks: u32,
    pub fade_alpha: f32,
    pub set_song: Option<(String, f32, f32)>,
    pub set_volume: Option<f32>,
    pub has_changed_song: bool
}

impl<'a> ScreenEvent<'a> {
    pub fn reset(&mut self) {
        self.timer = 0;
        self.current_step = 0;
        self.running = false;
        self.ticks = 0;
        self.init = true;
        self.fade_alpha = 0.0;
        self.has_changed_song = false;
    } 

    pub fn tick(&mut self, sfx: &mut SoundEffectBank, input: &Input) -> bool {
        if input.get_just_pressed(Keycode::X) && self.can_exit {
            return false;
        }

        if self.timer > 0 {
            self.timer -= 1;
        }

        if self.cont(input) || self.init {
            if !self.init {
                self.current_step += 1;

                if self.current_step >= self.steps.len() {
                    return false;
                }
            } 
            self.init = false;
            if let Continue::Wait(time) = self.steps[self.current_step].cont {
                self.timer = time;
            }

            // Events that run once instantly
            match &self.steps[self.current_step].step_type {
                ScreenEventStepType::PlaySound { sound, volume, speed } => {
                    sfx.play_ex(sound, *speed, *volume);
                },
                ScreenEventStepType::SetTextureHidden => {
                    self.visible = false;
                },
                ScreenEventStepType::SetTextureVisible => {
                    self.visible = true;
                },
                ScreenEventStepType::ShowFrame(frame) => {
                    self.current_frame = *frame;
                },
                ScreenEventStepType::Warn(message) => {
                    eprintln!("{}", message)
                },
                ScreenEventStepType::Animate { .. } => {
                    self.ticks = 0;
                },
                ScreenEventStepType::Song{ song, volume, speed} => {
                    self.set_song = Some((song.clone(), *volume, *speed));
                }
                _ => ()
            }
        }

        // Events that run continuously
        match &self.steps[self.current_step].step_type {
            ScreenEventStepType::Animate { from, to, speed } => {
                self.current_frame = from + ((self.ticks / speed) % ((to - from) + 1));
            },
            ScreenEventStepType::HideGame(time) => {
                self.fade_alpha = 1.0 - ((self.timer as f32 - 1.0) / *time as f32);
            },
            ScreenEventStepType::Mute(time) => {
                self.set_volume = Some((self.timer as f32 - 1.0) / *time as f32);
            },
            ScreenEventStepType::Unmute(time) => {
                self.set_volume = Some(1.0 - ((self.timer as f32 - 1.0) / *time as f32));
            }
            ScreenEventStepType::ShowGame(time) => {
                self.fade_alpha = (self.timer as f32 - 1.0) / *time as f32;
            },
            _ => ()
        }

        self.ticks += 1;

        true
    }

    /// continue
    pub fn cont(&self, input: &Input) -> bool {
        match self.steps[self.current_step].cont {
            Continue::Use => {
                input.get_just_pressed(Keycode::Z)
            },
            Continue::Wait(_) => {
                self.timer == 0
            }
        }
    }

    pub fn draw<T: RenderTarget>(&self, canvas: &mut Canvas<T>, state: &RenderState) {
        let cx = state.screen_extents.0 / 2;
        let cy = state.screen_extents.1 / 2;

        canvas.set_draw_color(Color::RGBA(0, 0, 0, (self.fade_alpha.min(1.0).max(0.0) * 255.0) as u8));
        canvas.fill_rect(None).unwrap();

        if self.visible {
            let frames_x = self.texture.width / self.frame_width;
            // let frames_y = self.texture.height / self.frame_height;
            let frame_x = self.current_frame % frames_x;
            let frame_y = self.current_frame / frames_x;

            canvas.copy(
                &self.texture.texture, 
                Rect::new((frame_x * self.frame_width) as i32, (frame_y * self.frame_height) as i32, self.frame_width, self.frame_height), 
                Rect::new(cx as i32 - (self.frame_width / 2) as i32, cy as i32 - (self.frame_height / 2) as i32, self.frame_width, self.frame_height)
            ).unwrap()
        }
    }

    pub fn from_file<T>(path: &PathBuf, creator: &'a TextureCreator<T>) -> Self {
        let contents = fs::read_to_string(path).expect("Could not open screen event file");
        Self::parse(contents, creator)
    }

    pub fn parse<T>(from: String, creator: &'a TextureCreator<T>) -> Self {
        let mut lines = from.split(&['\n', ';']).map(|s| s.split(" ")).map(|s| s.collect::<Vec<&str>>()).collect::<Vec<Vec<&str>>>();
        lines.retain(|s| s.len() > 0);
        
        let mut ignore = Vec::new();

        let mut texture = "particle/missing.png".to_string();
        let mut can_exit = true;
        let mut freeze = true;
        let mut frame_width = None;
        let mut frame_height = None;

        // Header pass
        for (i, line) in lines.iter().enumerate() {
            if line[0].starts_with("//") {
                ignore.push(i);
            } else if line[0].starts_with("#") {
                ignore.push(i);
                match line[1].trim() {
                    "texture" => {
                        texture = line[2].trim().to_string();
                        if let Some(Ok(width)) = line.get(3).map(|s| s.trim().parse::<u32>()) {
                            frame_width = Some(width);
                        }
                        if let Some(Ok(height)) = line.get(4).map(|s| s.trim().parse::<u32>()) {
                            frame_height = Some(height);
                        }
                    },
                    "can_exit" => {
                        can_exit = line[2].trim().parse::<bool>().expect("Expected boolean value after header `can_exit`");
                    },
                    "freeze" => {
                        freeze = line[2].trim().parse::<bool>().expect("Expected boolean value after header `freeze`");
                    },
                    _ => {
                        eprintln!("Warning: Unknown header command {}", line[1].trim());
                    }
                }
            }
        }

        let mut commands = Vec::new();

        // Main pass
        for (_, line) in lines.iter().enumerate().filter(|(i, _)| !ignore.contains(i)) {
            let mut token = 0;
            
            let mut cont = Continue::Wait(0);

            if line[token].trim().len() == 0 {
                continue;
            }

            let step_type = match line[token].trim() {
                "hidden" => { 
                    token += 1;
                    ScreenEventStepType::SetTextureHidden
                },
                "visible" => {
                    token += 1;
                    ScreenEventStepType::SetTextureVisible
                },
                "hide_bg" => {
                    let time = line[token + 1].trim().parse::<u32>().expect("Expected u32 after screen event step `hide_bg`");
                    token += 2;
                    cont = Continue::Wait(time + 1);
                    ScreenEventStepType::HideGame(time)
                },
                "show_bg" => {
                    let time = line[token + 1].trim().parse::<u32>().expect("Expected u32 after screen event step `show_bg`");
                    token += 2;
                    cont = Continue::Wait(time + 1);
                    ScreenEventStepType::ShowGame(time)
                },
                "mute" => {
                    let time = line[token + 1].trim().parse::<u32>().expect("Expected u32 after screen event step `mute`");
                    token += 2;
                    cont = Continue::Wait(time + 1);
                    ScreenEventStepType::Mute(time)
                },
                "unmute" => {
                    let time = line[token + 1].trim().parse::<u32>().expect("Expected u32 after screen event step `unmute`");
                    token += 2;
                    cont = Continue::Wait(time + 1);
                    ScreenEventStepType::Unmute(time)
                },
                "song" => {
                    let song = line[token + 1].to_string();
                    let volume = line[token + 2].trim().parse::<f32>().expect("Expected f32 for 2nd argument of screen event step `play`");
                    let speed = line[token + 3].trim().parse::<f32>().expect("Expected f32 for 3rd argument of screen event step `play`");
                    token += 4;
                    ScreenEventStepType::Song { song, volume, speed }
                },
                "wait" => {
                    token += 1;
                    ScreenEventStepType::None
                },
                "play" => {
                    let sound = line[token + 1].to_string();
                    let volume = line[token + 2].trim().parse::<f32>().expect("Expected f32 for 2nd argument of screen event step `play`");
                    let speed = line[token + 3].trim().parse::<f32>().expect("Expected f32 for 3rd argument of screen event step `play`");
                    token += 4;
                    ScreenEventStepType::PlaySound { sound, volume, speed }
                },
                "animate" => {
                    let from = line[token + 1].trim().parse::<u32>().expect("Expected u32 for arg. 1 of animate");
                    let to = line[token + 2].trim().parse::<u32>().expect("Expected u32 for arg. 2 of animate");
                    let speed = line[token + 3].trim().parse::<u32>().expect("Expected u32 for arg. 3 of animate");
                    cont = Continue::Wait(((to - from) + 1) * speed);
                    token += 4;
                    ScreenEventStepType::Animate { from, to, speed }
                },
                _ => {
                    eprintln!("Warning: Unknown event step `{}`", line[token].trim());
                    token += 1;
                    ScreenEventStepType::None
                }
            };

            while token < line.len() {
                if line[token].trim().len() == 0 {
                    token += 1;
                    continue;
                }

                match line[token].trim() {
                    "until" => {
                        token += 1;
                        match step_type {
                            ScreenEventStepType::HideGame(_) | ScreenEventStepType::ShowGame(_) => {
                                // break here to prevent bad data from being parsed
                                eprintln!("`until` is not valid with this step type");
                                break;
                            },
                            _ => {
                                if let Ok(time) = line[token].trim().parse::<u32>() {
                                    cont = Continue::Wait(time);
                                    token += 1;
                                } else {
                                    match line[token].trim() {
                                        "use" => {
                                            cont = Continue::Use;
                                            token += 1;
                                        },
                                        _ => {
                                            eprintln!("Invalid token after `until`: {}", line[token]);
                                            cont = Continue::Wait(0);
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    },
                    _ => { 
                        eprintln!("Unknown command {:?} in screen event file", line[token]);
                        break;
                    }
                }
            }

            commands.push(ScreenEventStep {
                cont,
                step_type
            });
        }

        let loaded_texture = Texture::from_file(&PathBuf::from("res/textures/").join(texture), creator).expect("Failed to load screen event texture");

        Self {
            can_exit,
            freeze_player: freeze,
            current_step: 0,
            running: false,
            steps: commands,
            timer: 0,
            current_frame: 0,
            visible: false,
            fade_alpha: 0.0,
            ticks: 0,
            init: true,
            set_song: None,
            set_volume: None,
            frame_width: frame_width.unwrap_or(loaded_texture.width),
            frame_height: frame_height.unwrap_or(loaded_texture.height),
            texture: loaded_texture,
            has_changed_song: false
        }
    }
}