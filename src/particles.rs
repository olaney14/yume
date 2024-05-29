use std::{collections::VecDeque, fmt::Debug, mem, path::PathBuf};

use json::{iterators::Members, JsonValue};
use rand::{distributions::uniform::SampleUniform, Rng};
use sdl2::{rect::Rect, render::{Canvas, RenderTarget, TextureCreator}};

use crate::{game::RenderState, texture::{self, Texture}, world::World};

#[derive(Debug)]
pub enum ParticleValue<T: SampleUniform + Copy + PartialOrd> {
    Value(T),
    RandRange(T, T),
    //RandRangeNormal(T, T)
}

impl<T: SampleUniform + Copy + PartialOrd + Debug> ParticleValue<T> {
    pub fn get(&self) -> T {
        match self {
            Self::Value(v) => *v,
            Self::RandRange(min, max) => rand::thread_rng().gen_range(*min..*max),
            //Self::RandRangeNormal(min, max) => rand::thread_rng().gen_range(range)
        }
    }
}

#[derive(Debug)]
pub struct ParticleEmitter {
    pub texture: String,
    pub pos: (i32, i32),
    pub height: i32,

    pub particles: VecDeque<Particle>,

    pub pos_offset: (ParticleValue<f32>, ParticleValue<f32>),
    pub init_vel: (ParticleValue<f32>, ParticleValue<f32>),
    pub init_acc: (ParticleValue<f32>, ParticleValue<f32>),
    pub init_tx_coord: (ParticleValue<f32>, ParticleValue<f32>),
    pub init_life: ParticleValue<u32>,
    pub init_tx_vel: (ParticleValue<f32>, ParticleValue<f32>),
    pub size: (u32, u32),
    pub freq: u32,
    pub freq_rand: i32,
    pub timer: i32
}

impl ParticleEmitter {
    pub fn draw<T: RenderTarget>(&self, canvas: &mut Canvas<T>, world: &World, state: &RenderState) {
        for particle in self.particles.iter() {
            if !particle.active { continue; }
            canvas.copy(
                &world.particle_textures.get_texture(&self.texture).unwrap().texture, 
                Rect::new(particle.tx_coord.0 as i32, particle.tx_coord.1 as i32, particle.size.0, particle.size.1), 
                Rect::new(particle.pos.0 as i32 + state.offset.0, particle.pos.1 as i32 + state.offset.1, particle.size.0, particle.size.1)
            ).unwrap()
        }
    }

    pub fn add_particle(&mut self) {
        let particle = Particle {
            active: true,
            pos: (self.pos.0 as f32 + self.pos_offset.0.get(), self.pos.1 as f32 + self.pos_offset.1.get()),
            vel: (self.init_vel.0.get(), self.init_vel.1.get()),
            acc: (self.init_acc.0.get(), self.init_acc.1.get()),
            life: self.init_life.get(),
            size: self.size,
            tx_coord: (self.init_tx_coord.0.get(), self.init_tx_coord.1.get()),
            tx_vel: (self.init_tx_vel.0.get(), self.init_tx_vel.1.get())
        };

        self.particles.push_back(particle);
    }

    pub fn update(&mut self, pos: (i32, i32)) {
        self.pos = pos;

        self.timer -= 1;
        if self.timer <= 0 {
            self.timer = self.freq as i32 + rand::thread_rng().gen_range(0..=self.freq_rand);

            self.add_particle();
        }

        if self.particles.is_empty() {
            return;
        }

        for particle in self.particles.iter_mut() {
            if particle.life == 0 { continue; }

            particle.pos.0 += particle.vel.0;
            particle.pos.1 += particle.vel.1;
            particle.vel.0 += particle.acc.0;
            particle.vel.1 += particle.acc.1;

            particle.tx_coord.0 += particle.tx_vel.0;
            particle.tx_coord.1 += particle.tx_vel.1;

            particle.life -= 1;

            if particle.life == 0 {
                particle.active = false;
            }
        }

        while let Some(particle) = self.particles.front() {
            if !particle.active {
                self.particles.pop_front();
            } else {
                break;
            }
        }
    }
}

#[derive(Debug)]
pub struct Particle {
    pub active: bool,
    pub pos: (f32, f32),
    pub vel: (f32, f32),
    pub acc: (f32, f32),
    pub life: u32,
    pub tx_coord: (f32, f32),
    pub tx_vel: (f32, f32),
    pub size: (u32, u32)
}

fn parse_particle_f32(json: &JsonValue) -> Option<ParticleValue<f32>> {
    if json.is_number() {
        return Some(ParticleValue::Value(json.as_f32().unwrap()));
    } else if json.is_string() {
        let mut split = json.as_str().unwrap().split(',');
        let first = split.next().unwrap().parse::<f32>().expect("failed to parse particle property");
        if let Some(second) = split.next() {
            let top_bound = second.parse::<f32>().expect("failed to parse particle property");
            return Some(ParticleValue::RandRange(first, top_bound));
        } else {
            return Some(ParticleValue::Value(first));
        }
    } else if json.is_array() {
        let mut members = json.members();
        let first = members.next().unwrap();
        if let Some(second) = members.next() {
            return Some(ParticleValue::RandRange(first.as_f32().unwrap(), second.as_f32().unwrap()));
        } else {
            return Some(ParticleValue::Value(first.as_f32().unwrap()));
        }
    } else if json.is_object() {
        let low = json["low"].as_f32().expect("failed to parse lower bound of particle property");
        let high = json["high"].as_f32().expect("failed to parse upper bound of particle property");

        return Some(ParticleValue::RandRange(low, high));
    }

    None
}

fn parse_particle_u32(json: &JsonValue) -> Option<ParticleValue<u32>> {
    if json.is_number() {
        return Some(ParticleValue::Value(json.as_u32().unwrap()));
    } else if json.is_string() {
        let mut split = json.as_str().unwrap().split(',');
        let first = split.next().unwrap().parse::<u32>().expect("failed to parse particle property");
        if let Some(second) = split.next() {
            let top_bound = second.parse::<u32>().expect("failed to parse particle property");
            return Some(ParticleValue::RandRange(first, top_bound));
        } else {
            return Some(ParticleValue::Value(first));
        }
    } else if json.is_array() {
        let mut members = json.members();
        let first = members.next().unwrap();
        if let Some(second) = members.next() {
            return Some(ParticleValue::RandRange(first.as_u32().unwrap(), second.as_u32().unwrap()));
        } else {
            return Some(ParticleValue::Value(first.as_u32().unwrap()));
        }
    } else if json.is_object() {
        let low = json["low"].as_u32().expect("failed to parse lower bound of particle property");
        let high = json["high"].as_u32().expect("failed to parse upper bound of particle property");

        return Some(ParticleValue::RandRange(low, high));
    }

    None
}

fn parse_particle_f32_pair(json: &JsonValue) -> Option<(ParticleValue<f32>, ParticleValue<f32>)> {
    if json.is_array() {
        let mut members = json.members();
        //expect("not enough items in particle property array")
        let x = parse_particle_f32(members.next()?).unwrap();
        let y = parse_particle_f32(members.next().expect("not enough items in particle property array")).unwrap();

        return Some((x, y));
    } else if json.is_object() {
        let x = parse_particle_f32(&json["x"]).expect("failed to parse x component of particle property pair");
        let y = parse_particle_f32(&json["y"]).expect("failed to parse y component of particle property pair");

        return Some((x, y));
    }

    None
}

fn parse_u32_pair(json: &JsonValue) -> Option<(u32, u32)> {
    if json.is_array() {
        let mut members = json.members();
        let x = members.next()?.as_u32()?;
        let y = members.next()?.as_u32()?;

        return Some((x, y));
    } else if json.is_object() {
        let x = json["x"].as_u32()?;
        let y = json["y"].as_u32()?;

        return Some((x, y));
    }

    None
}

// pub texture: String,
// pub pos: (i32, i32),

// pub particles: VecDeque<Particle>,

// pub pos_offset: (ParticleValue<f32>, ParticleValue<f32>),
// pub init_vel: (ParticleValue<f32>, ParticleValue<f32>),
// pub init_tx_coord: (ParticleValue<f32>, ParticleValue<f32>),
// pub init_life: ParticleValue<f32>,
// pub init_tx_vel: (ParticleValue<f32>, ParticleValue<f32>),
// pub freq: u32,
// pub timer: i32

// tex, offset, vel, acc, tx, tx vel, lifetime, freq (just a number)

type ParticleFloatPair = (ParticleValue<f32>, ParticleValue<f32>);

const DEFAULT_LIFETIME: ParticleValue<u32> = ParticleValue::RandRange(120, 150);
const DEFAULT_POS_OFFSET: ParticleFloatPair = (ParticleValue::Value(0.0), ParticleValue::Value(0.0));
const DEFAULT_VELOCITY: ParticleFloatPair = (ParticleValue::RandRange(-1.0, 1.0), ParticleValue::RandRange(2.0, 4.0));
const DEFAULT_ACC: ParticleFloatPair = (ParticleValue::Value(0.0), ParticleValue::Value(0.0));
const DEFAULT_TEX_COORD: ParticleFloatPair = (ParticleValue::Value(0.0), ParticleValue::Value(0.0));
const DEFAULT_TEX_VEL: ParticleFloatPair = (ParticleValue::Value(0.0), ParticleValue::Value(0.0));
const DEFAULT_FREQ: u32 = 5;

pub fn parse_particles(json: &JsonValue) -> Option<ParticleEmitter> {
    let lifetime = if !json["lifetime"].is_null() { parse_particle_u32(&json["lifetime"]).expect("failed to parse particle property `lifetime`") } else { DEFAULT_LIFETIME };
    let pos_offset = if !json["pos_offset"].is_null() { parse_particle_f32_pair(&json["pos_offset"]).expect("failed to parse particle property `pos_offset`") } else { DEFAULT_POS_OFFSET };
    let velocity = if !json["velocity"].is_null() { parse_particle_f32_pair(&json["velocity"]).expect("failed to parse particle property `velocity`") } else { DEFAULT_VELOCITY };
    let acceleration = if !json["acceleration"].is_null() { parse_particle_f32_pair(&json["acceleration"]).expect("failed to parse particle property `acceleration`") } else { DEFAULT_ACC };
    let tx_coord = if !json["tx_coord"].is_null() { parse_particle_f32_pair(&json["tx_coord"]).expect("failed to parse particle property `tx_coord`") } else { DEFAULT_TEX_COORD }; 
    let tx_vel = if !json["tx_vel"].is_null() { parse_particle_f32_pair(&json["tx_vel"]).expect("failed to parse particle property `tx_vel`") } else { DEFAULT_TEX_VEL };
    let freq = if !json["freq"].is_null() { json["freq"].as_u32().expect("failed to parse particle property `freq`") } else { DEFAULT_FREQ };
    let texture_path = if !json["texture"].is_null() { json["texture"].as_str().expect("failed to parse particle emitter texture") } else { "missing.png" };
    let size = if !json["size"].is_null() { parse_u32_pair(&json["size"]).expect("failed to parse particle property `size`") } else { (1, 1) };
    //let texture = texture::Texture::from_file(&PathBuf::from("res/textures/particle/").join(texture_path), creator).expect("failed to load particle texture");
    let height = if !json["height"].is_null() { json["height"].as_i32().unwrap() } else { 0 };
    let freq_rand = if !json["freq_rand"].is_null() { json["freq_rand"].as_i32().unwrap().abs() } else { 0 };

    let emitter = ParticleEmitter {
        freq,
        init_acc: acceleration,
        init_life: lifetime,
        init_tx_coord: tx_coord,
        init_tx_vel: tx_vel,
        init_vel: velocity,
        particles: VecDeque::new(),
        pos: (0, 0),
        pos_offset,
        texture: texture_path.to_owned(),
        timer: 0,
        size,
        height,
        freq_rand
    };

    Some(emitter)
}