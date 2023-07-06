use std::{path::PathBuf, fs::File, io::BufReader, sync::Arc, thread, collections::HashMap, process::Output};

use rodio::{Sink, Decoder, Source, source::{Repeat, Buffered}, dynamic_mixer::{DynamicMixerController, self, DynamicMixer}, OutputStreamHandle};

pub struct SoundEffectBank {
    pub sound_effects: HashMap<String, SoundEffect>,
    pub output_handle: Arc<OutputStreamHandle>
}

impl SoundEffectBank {
    pub fn new(output_handle: Arc<OutputStreamHandle>) -> Self {
        Self {
            sound_effects: HashMap::new(),
            output_handle
        }
    }

    pub fn play(&mut self, name: &str) {
        if self.sound_effects.contains_key(name) {
            self.sound_effects.get(name).unwrap().play(&self.output_handle);
        } else {
            if let Ok(file) = File::open(PathBuf::from("res/audio/sfx/".to_owned() + name + ".mp3")) {
                let source = rodio::Decoder::new(BufReader::new(file)).unwrap().buffered();

                self.sound_effects.insert(name.to_string().clone(), SoundEffect { speed: 1.0, volume: 1.0, source });
                self.sound_effects.get(name).unwrap().play(&self.output_handle);
            } else {
                eprintln!("Could not play sound effect {}", name);
            }
        }
    }

    pub fn load(&mut self, name: &String, volume: f32, speed: f32) {
        if let Ok(file) = File::open(PathBuf::from("res/audio/sfx/".to_owned() + name + ".mp3")) {
            let source = rodio::Decoder::new(BufReader::new(file)).unwrap().buffered();

            self.sound_effects.insert(name.clone(), SoundEffect { speed, volume, source });
        } else {
            eprintln!("Could not load sound effect {}", name);
        }
    }
}

pub struct SoundEffect {
    pub speed: f32,
    pub volume: f32,
    pub source: Buffered<Decoder<BufReader<File>>>,
}

impl SoundEffect {
    pub fn new(path: PathBuf) -> Self {
        let file = File::open(&path).expect(format!("Failed to load song {}", path.as_os_str().to_str().unwrap()).as_str());
        let source = rodio::Decoder::new(BufReader::new(file)).unwrap().buffered();

        Self {
            speed: 1.0,
            volume: 1.0,
            source
        }
    }

    pub fn play(&self, output_handle: &Arc<OutputStreamHandle>) {
        let sound_sink = Sink::try_new(&output_handle).unwrap();
        let cloned_source = self.source.clone();
        let speed = self.speed;
        let volume = self.volume;
        thread::spawn(move || {
            sound_sink.set_speed(speed);
            sound_sink.set_volume(volume);
            sound_sink.append(cloned_source);
            sound_sink.sleep_until_end();
        });
    }
}

pub struct Song {
    pub speed: f32,
    pub volume: f32, 
    pub dirty: bool,
    pub source: Option<Repeat<Decoder<BufReader<File>>>>,
    pub playing: bool,
    pub path: PathBuf,
    pub default_speed: f32,
    pub default_volume: f32,
}

impl Song {
    pub fn new(path: PathBuf) -> Self {
        let file = File::open(&path).expect(format!("Failed to load song {}", path.as_os_str().to_str().unwrap()).as_str());
        let source = rodio::Decoder::new(BufReader::new(file)).unwrap().repeat_infinite();

        Self {
            path,
            source: Some(source),
            speed: 1.0,
            volume: 1.0,
            dirty: true,
            playing: false,
            default_speed: 1.0,
            default_volume: 1.0
        }
    }

    pub fn play(&mut self, sink: &Sink) {
        if !self.playing && self.source.is_some() {
            if !sink.empty() {
                sink.clear();
            }
            sink.set_speed(self.speed);
            sink.set_volume(self.volume);
            sink.append(self.source.take().unwrap());
            self.playing = true;
            self.dirty = false;
            sink.play();
        }
    }

    /// This method only needs to be called if `dirty` is true but you do you
    pub fn update(&self, sink: &Sink) {
        sink.set_speed(self.speed);
        sink.set_volume(self.volume);
    }
}