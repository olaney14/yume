use std::{collections::HashMap, error::Error, fmt::Debug, fs::File, io::BufReader, path::{Path, PathBuf}, sync::Arc, thread};

use rodio::{Sink, Decoder, Source, source::Buffered, OutputStreamHandle};

pub struct SoundEffectBank {
    pub sound_effects: HashMap<String, SoundEffect>,
    pub output_handle: Arc<OutputStreamHandle>
}

const ACCEPTED_SFX_EXTENSIONS: [&str; 3] = [
    "mp3", "wav", "ogg"
];

const CROSSFADE_LOOP_MS: u32 = 10;
const FADE_IN_MS: u32 = 25;

impl SoundEffectBank {
    pub fn new(output_handle: Arc<OutputStreamHandle>) -> Self {
        Self {
            sound_effects: HashMap::new(),
            output_handle
        }
    }

    fn try_insert_sfx_with_extension(&mut self, name: &str, ext: &str, speed: f32, volume: f32) -> Result<(), Box<dyn Error>> {
        let file = File::open(PathBuf::from(format!("res/audio/sfx/{}.{}", name, ext)))?;
        let source = rodio::Decoder::new(BufReader::new(file)).unwrap().buffered();
        self.sound_effects.insert(name.to_string(), SoundEffect {
            speed, volume, source
        });
        Ok(())
    }

    pub fn try_load(&mut self, name: &str, speed: f32, volume: f32) -> bool {
        for extension in ACCEPTED_SFX_EXTENSIONS.iter() {
            match self.try_insert_sfx_with_extension(name, extension, speed, volume) {
                Ok(()) => return true,
                Err(_e) => {
                    //eprintln!("{:?} at res/audio/sfx/{}.{}", e, name, extension);
                }
            }
        }

        false
    }

    pub fn play(&mut self, name: &str) {
        if self.sound_effects.contains_key(name) {
            self.sound_effects.get(name).unwrap().play(&self.output_handle);
        } else {
            if self.try_load(name, 1.0, 1.0) {
                self.play(name);
            } else {
                eprintln!("Could not play sound effect {}", name);
            }
        }
    }

    pub fn play_ex(&mut self, name: &str, speed: f32, volume: f32) {
        if self.sound_effects.contains_key(name) {
            self.sound_effects.get(name).unwrap().play_ex(&self.output_handle, speed, volume);
        } else {
            if self.try_load(name, speed, volume) {
                self.play_ex(name, speed, volume);
            } else {
                eprintln!("Could not play sound effect {}", name);
            }
        }
    }
}

pub struct SoundEffect {
    pub speed: f32,
    pub volume: f32,
    pub source: Buffered<Decoder<BufReader<File>>>,
}

impl SoundEffect {
    pub fn play(&self, output_handle: &Arc<OutputStreamHandle>) {
        self.play_ex(output_handle, self.speed, self.volume);
    }

    pub fn play_ex(&self, output_handle: &Arc<OutputStreamHandle>, speed: f32, volume: f32) {
        let sound_sink = Sink::try_new(&output_handle).unwrap();
        let cloned_source = self.source.clone();
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
    pub reload: bool,
    pub source: Option<CrossfadedLoop>,
    pub playing: bool,
    pub path: PathBuf,
    pub default_speed: f32,
    pub default_volume: f32,
    pub name: String,
}

impl Song {
    pub fn new(path: PathBuf) -> Self {
        // let file = File::open(&path).expect(format!("Failed to load song {}", path.as_os_str().to_str().unwrap()).as_str());
        let name = path.file_stem().unwrap().to_str().unwrap().to_owned();
        // let source = rodio::Decoder::new(BufReader::new(file)).unwrap().repeat_infinite();
        let source = CrossfadedLoop::new(&path, CROSSFADE_LOOP_MS);

        Self {
            path,
            source: Some(source),
            speed: 1.0,
            volume: 1.0,
            dirty: true,
            playing: false,
            default_speed: 1.0,
            default_volume: 1.0,
            reload: false,
            name
        }
    }

    pub fn play(&mut self, sink: &Sink) {
        if !self.playing && self.source.is_some() {
            if !sink.empty() {
                sink.clear();
            }
            sink.append(self.source.take().unwrap());
            sink.set_speed(self.speed);
            sink.set_volume(self.volume);
            self.playing = true;
            self.dirty = false;
            sink.play();
        }
    }

    /// This method only needs to be called if `dirty` is true but you do you
    pub fn update(&mut self, sink: &Sink) {
        sink.set_speed(self.speed);
        sink.set_volume(self.volume.max(0.0));

        if self.reload {
            sink.clear();
            sink.append(self.source.take().unwrap());
            self.playing = true;
            self.reload = false;
            sink.play();
        }
    }

    pub fn reload(&mut self, sink: &Sink) {
        // let file = File::open(&self.path).expect(format!("Failed to load song {}", self.path.as_os_str().to_str().unwrap()).as_str());
        //self.source = Some(rodio::Decoder::new(BufReader::new(file)).unwrap().repeat_infinite());
        self.source = Some(CrossfadedLoop::new(&self.path, CROSSFADE_LOOP_MS));
        self.reload = true;
        self.update(sink);
    }
}

fn crossfade_loop_buffer(
    samples: Vec<f32>,
    channels: u16,
    sample_rate: u32,
    fade_ms: u32
) -> Vec<f32> {
    let fade_frames = (sample_rate * fade_ms / 1000) as usize;
    let fade_samples = fade_frames * channels as usize;

    // not enough to crossfade loop
    if samples.len() < fade_samples * 2 {
        return samples;
    }

    let total = samples.len();
    // this looks bad but is so so cheap compared to actually loading the song from disk
    let mut output = samples.clone();

    for i in 0..fade_samples {
        let a = i as f32 / fade_samples as f32;

        // sample index from the end to apply crossfade
        let tail_ix = total - fade_samples + i;
        // fade out end
        output[tail_ix] *= 1.0 - a;

        // fade in beginning
        output[tail_ix] += samples[i] * a;
    }

    // trim beginning to avoid double playing
    output[fade_samples..].to_vec()
    // output
}

pub struct CrossfadedLoop {
    samples: Vec<f32>,
    pos: usize,
    channels: u16,
    sample_rate: u32,
    fade_in: usize,
    fade_in_samples: usize
}

impl CrossfadedLoop {
    pub fn new<P: AsRef<Path> + Debug>(path: P, fade_ms: u32) -> Self {
        let file = File::open(&path)
            .unwrap_or_else(|_| panic!("Failed to load song {:?}", &path));

        let decoder = Decoder::new(BufReader::new(file)).unwrap();
        let channels = decoder.channels();
        let sample_rate = decoder.sample_rate();

        let samples = decoder
            .convert_samples::<f32>()
            .collect();

        let looping_samples = crossfade_loop_buffer(samples, channels, sample_rate, fade_ms);
    
        let fade_frames = (sample_rate * FADE_IN_MS / 1000) as usize;
        let fade_samples = fade_frames * channels as usize;

        Self {
            samples: looping_samples,
            pos: 0,
            channels,
            sample_rate,
            fade_in: fade_samples,
            fade_in_samples: fade_samples
        }
    }
}

impl Iterator for CrossfadedLoop {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.samples.is_empty() {
            return None;
        }
        let a = if self.fade_in == 0 {
            1.0
        } else {
            let t = (self.fade_in_samples - self.fade_in) as f32 / self.fade_in_samples as f32;
            t.sqrt()
        };
        let sample = self.samples[self.pos] * a;
        self.pos = (self.pos + 1) % self.samples.len();
        if self.fade_in > 0 {
            self.fade_in -= 1;
        }
        Some(sample)
    }
}

impl Source for CrossfadedLoop {
    fn current_frame_len(&self) -> Option<usize> { None }
    fn channels(&self) -> u16 { self.channels }
    fn sample_rate(&self) -> u32 { self.sample_rate }
    fn total_duration(&self) -> Option<std::time::Duration> { None }
}