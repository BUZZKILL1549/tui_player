use std::collections::VecDeque;
use std::fs::File;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, atomic::AtomicBool};
use std::thread;
use std::time::{Duration, Instant};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use symphonia::core::{
    audio::SampleBuffer, codecs::DecoderOptions, formats::FormatOptions, io::MediaSourceStream,
    meta::MetadataOptions, probe::Hint,
};

pub static EXIT_NOW: AtomicBool = AtomicBool::new(false);

pub struct AudioPlayer {
    _stream: Option<cpal::Stream>,
    audio_buffer: Arc<Mutex<VecDeque<f32>>>,
    thread_handle: Option<thread::JoinHandle<()>>,
    should_stop: Arc<Mutex<bool>>,
    current_position: Arc<Mutex<Duration>>,
    total_duration: Arc<Mutex<Duration>>,
    playback_started: Arc<Mutex<Option<Instant>>>,
    is_playing: Arc<Mutex<bool>>,
    is_paused: Arc<Mutex<bool>>,
    current_song: Arc<Mutex<Option<String>>>,
    current_volume: Arc<Mutex<f32>>,
    current_path: Arc<Mutex<Option<PathBuf>>>,
    underruns: Arc<Mutex<u32>>,
}

impl AudioPlayer {
    pub fn new() -> Self {
        AudioPlayer {
            _stream: None,
            audio_buffer: Arc::new(Mutex::new(VecDeque::new())),
            thread_handle: None,
            should_stop: Arc::new(Mutex::new(false)),
            current_position: Arc::new(Mutex::new(Duration::from_secs(0))),
            total_duration: Arc::new(Mutex::new(Duration::from_secs(0))),
            playback_started: Arc::new(Mutex::new(None)),
            is_playing: Arc::new(Mutex::new(false)),
            is_paused: Arc::new(Mutex::new(false)),
            current_song: Arc::new(Mutex::new(None)),
            current_volume: Arc::new(Mutex::new(1.0)),
            current_path: Arc::new(Mutex::new(None)),
            underruns: Arc::new(Mutex::new(0)),
        }
    }

    pub fn play_song(&mut self, file_path: Option<PathBuf>) {
        self.stop();

        if let Some(path) = file_path.clone() {
            *self.current_position.lock().unwrap() = Duration::from_secs(0);
            *self.is_playing.lock().unwrap() = true;
            *self.is_paused.lock().unwrap() = false;
            *self.playback_started.lock().unwrap() = Some(Instant::now());
            *self.current_path.lock().unwrap() = file_path;
            *self.underruns.lock().unwrap() = 0;

            if let Some(filename) = path.file_name() {
                if let Some(name) = filename.to_str() {
                    *self.current_song.lock().unwrap() = Some(name.to_owned());
                }
            }

            *self.should_stop.lock().unwrap() = false;

            let should_stop_clone = Arc::clone(&self.should_stop);
            let path_clone = path.clone();
            let total_duration_clone = Arc::clone(&self.total_duration);
            let is_playing_clone = Arc::clone(&self.is_playing);
            let volume_clone = Arc::clone(&self.current_volume);
            let audio_buffer_clone = Arc::clone(&self.audio_buffer);
            let is_paused_clone = Arc::clone(&self.is_paused);
            let underruns_clone = Arc::clone(&self.underruns);

            self.thread_handle = Some(thread::spawn(move || {
                let file = match File::open(&path_clone) {
                    Ok(file) => Box::new(file),
                    Err(e) => {
                        eprintln!("Error opening audio file: {}", e);
                        *is_playing_clone.lock().unwrap() = false;
                        return;
                    }
                };

                let mss = MediaSourceStream::new(file, Default::default());

                let hint = Hint::new();
                let format_opts: FormatOptions = Default::default();
                let metadata_opts: MetadataOptions = Default::default();
                let decoder_opts: DecoderOptions = Default::default();

                let probed = match symphonia::default::get_probe().format(
                    &hint,
                    mss,
                    &format_opts,
                    &metadata_opts,
                ) {
                    Ok(probed) => probed,
                    Err(e) => {
                        eprintln!("Error probing audio format: {}", e);
                        *is_playing_clone.lock().unwrap() = false;
                        return;
                    }
                };

                let mut format = probed.format;

                let track = match format.default_track() {
                    Some(track) => track,
                    None => {
                        eprintln!("No default track found in audio file");
                        *is_playing_clone.lock().unwrap() = false;
                        return;
                    }
                };

                let mut decoder = match symphonia::default::get_codecs()
                    .make(&track.codec_params, &decoder_opts)
                {
                    Ok(decoder) => decoder,
                    Err(e) => {
                        eprintln!("Error creating decoder: {}", e);
                        *is_playing_clone.lock().unwrap() = false;
                        return;
                    }
                };

                if let Some(n_frames) = track.codec_params.n_frames {
                    if let Some(rate) = track.codec_params.sample_rate {
                        let total_secs = n_frames as f64 / rate as f64;
                        *total_duration_clone.lock().unwrap() = Duration::from_secs_f64(total_secs);
                    } else {
                        *total_duration_clone.lock().unwrap() = Duration::from_secs(180); // default 3 minutes
                    }
                } else {
                    *total_duration_clone.lock().unwrap() = Duration::from_secs(180);
                }

                let track_id = track.id;

                let host = cpal::default_host();
                let device = match host.default_output_device() {
                    Some(device) => device,
                    None => {
                        eprintln!("No default audio output device available");
                        *is_playing_clone.lock().unwrap() = false;
                        return;
                    }
                };

                let packet = match format.next_packet() {
                    Ok(packet) => packet,
                    Err(e) => {
                        eprintln!("Error getting first audio packet: {}", e);
                        *is_playing_clone.lock().unwrap() = false;
                        return;
                    }
                };

                let audio_buf = match decoder.decode(&packet) {
                    Ok(buf) => buf,
                    Err(e) => {
                        eprintln!("Error decoding first audio packet: {}", e);
                        *is_playing_clone.lock().unwrap() = false;
                        return;
                    }
                };

                let spec = *audio_buf.spec();
                let sample_rate = cpal::SampleRate(spec.rate);
                let channels = spec.channels.count() as u16;

                let config = cpal::StreamConfig {
                    channels,
                    sample_rate,
                    buffer_size: cpal::BufferSize::Default,
                };

                let mut sample_buf = SampleBuffer::<f32>::new(audio_buf.capacity() as u64, spec);

                sample_buf.copy_interleaved_ref(audio_buf);

                {
                    let mut buffer = audio_buffer_clone.lock().unwrap();
                    for &sample in sample_buf.samples().iter() {
                        buffer.push_back(sample);
                    }
                }

                let callback_is_paused = Arc::clone(&is_paused_clone);
                let callback_audio_buffer = Arc::clone(&audio_buffer_clone);
                let callback_volume = Arc::clone(&volume_clone);
                let callback_underruns = Arc::clone(&underruns_clone);

                let stream = match device.build_output_stream(
                    &config,
                    move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                        if *callback_is_paused.lock().unwrap() {
                            for sample in data.iter_mut() {
                                *sample = 0.0;
                            }
                            return;
                        }

                        let mut buffer = callback_audio_buffer.lock().unwrap();
                        let volume = *callback_volume.lock().unwrap();

                        if buffer.len() < data.len() {
                            let mut underruns = callback_underruns.lock().unwrap();
                            *underruns += 1;

                            for sample in data.iter_mut() {
                                *sample = 0.0;
                            }
                        } else {
                            for sample in data.iter_mut() {
                                *sample = buffer.pop_front().unwrap_or(0.0) * volume;
                            }
                        }
                    },
                    |err| eprintln!("An error occurred on the output audio stream: {}", err),
                    None,
                ) {
                    Ok(stream) => stream,
                    Err(e) => {
                        eprintln!("Error building audio output stream: {}", e);
                        *is_playing_clone.lock().unwrap() = false;
                        return;
                    }
                };

                if let Err(e) = stream.play() {
                    eprintln!("Error starting audio playback: {}", e);
                    *is_playing_clone.lock().unwrap() = false;
                    return;
                }

                // buffer kinda important here cuz otherwise the music be choppy af. 2 secs prolly
                // good enuf
                let max_buffer_size = spec.rate as usize * spec.channels.count() * 2; // 2 sec
                let mut last_packet_decoded = false;

                loop {
                    if *should_stop_clone.lock().unwrap()
                        || EXIT_NOW.load(std::sync::atomic::Ordering::SeqCst)
                    {
                        *is_playing_clone.lock().unwrap() = false;
                        break;
                    }

                    if *is_paused_clone.lock().unwrap() {
                        thread::sleep(Duration::from_millis(10));
                        continue;
                    }

                    let current_buffer_len = {
                        let buffer = audio_buffer_clone.lock().unwrap();
                        buffer.len()
                    };

                    if current_buffer_len > max_buffer_size - (spec.channels.count() * 1024) {
                        thread::sleep(Duration::from_millis(10));
                        continue;
                    }

                    if !last_packet_decoded {
                        match format.next_packet() {
                            Ok(packet) => {
                                if packet.track_id() != track_id {
                                    continue;
                                }

                                match decoder.decode(&packet) {
                                    Ok(audio_buf) => {
                                        sample_buf.copy_interleaved_ref(audio_buf);

                                        {
                                            let mut buffer = audio_buffer_clone.lock().unwrap();
                                            for &sample in sample_buf.samples().iter() {
                                                buffer.push_back(sample);
                                            }
                                        }
                                    }
                                    Err(symphonia::core::errors::Error::DecodeError(_)) => (),
                                    Err(_) => {
                                        last_packet_decoded = true;
                                    }
                                }
                            }
                            Err(_) => {
                                last_packet_decoded = true;
                            }
                        }
                    } else {
                        let buffer_empty = {
                            let buffer = audio_buffer_clone.lock().unwrap();
                            buffer.is_empty()
                        };

                        if buffer_empty {
                            *is_playing_clone.lock().unwrap() = false;
                            break;
                        }

                        thread::sleep(Duration::from_millis(100));
                    }
                }
            }));
        }
    }

    pub fn stop(&mut self) {
        *self.should_stop.lock().unwrap() = true;
        *self.is_playing.lock().unwrap() = false;
        *self.is_paused.lock().unwrap() = false;

        let mut buffer = self.audio_buffer.lock().unwrap();
        buffer.clear();

        if let Some(handle) = self.thread_handle.take() {
            let timeout = Duration::from_millis(200);
            let start = Instant::now();

            while start.elapsed() < timeout {
                if handle.is_finished() {
                    break;
                }
                thread::sleep(Duration::from_millis(10));
            }

            match handle.join() {
                Ok(_) => (),
                Err(e) => eprintln!("Error joining audio thread: {:?}", e),
            }
        }

        self._stream = None;
    }

    pub fn update_position(&self) {
        let mut position = self.current_position.lock().unwrap();
        let is_playing = *self.is_playing.lock().unwrap();
        let is_paused = *self.is_paused.lock().unwrap();

        if is_playing && !is_paused {
            let total = *self.total_duration.lock().unwrap();

            if let Some(start_time) = *self.playback_started.lock().unwrap() {
                *position = start_time.elapsed();

                if *position > total {
                    *position = total;
                }
            }
        }
    }

    pub fn get_progress(&self) -> f32 {
        let position = *self.current_position.lock().unwrap();
        let total = *self.total_duration.lock().unwrap();

        if total.as_secs() == 0 {
            return 0.0;
        }

        position.as_secs_f32() / total.as_secs_f32()
    }

    pub fn format_time(&self) -> String {
        let position = *self.current_position.lock().unwrap();
        let total = *self.total_duration.lock().unwrap();

        let position_secs = position.as_secs();
        let total_secs = total.as_secs();

        let position_min = position_secs / 60;
        let position_sec = position_secs % 60;
        let total_min = total_secs / 60;
        let total_sec = total_secs % 60;

        format!(
            "{:02}:{:02}/{:02}:{:02}",
            position_min, position_sec, total_min, total_sec
        )
    }

    pub fn is_playing(&self) -> bool {
        *self.is_playing.lock().unwrap()
    }

    pub fn current_song_name(&self) -> Option<String> {
        self.current_song.lock().unwrap().clone()
    }

    pub fn toggle_pause(&mut self) {
        let mut is_paused = self.is_paused.lock().unwrap();

        if *is_paused {
            *is_paused = false;

            // Update playback_started to account for paused time
            let mut playback_started = self.playback_started.lock().unwrap();
            let current_position = *self.current_position.lock().unwrap();
            *playback_started = Some(Instant::now() - current_position);
        } else {
            *is_paused = true;
        }
    }

    pub fn is_paused(&self) -> bool {
        *self.is_paused.lock().unwrap()
    }

    pub fn seek_forward(&mut self, _seconds: f32) {
        unimplemented!();
    }

    pub fn seek_backward(&mut self, _seconds: f32) {
        unimplemented!();
    }

    pub fn increase_volume(&mut self, amount: f32) {
        let mut volume = self.current_volume.lock().unwrap();
        *volume = (*volume + amount).min(1.0);
    }

    pub fn decrease_volume(&mut self, amount: f32) {
        let mut volume = self.current_volume.lock().unwrap();
        *volume = (*volume - amount).max(0.0);
    }

    pub fn get_volume(&self) -> f32 {
        *self.current_volume.lock().unwrap()
    }

    /*
    pub fn restart(&mut self) {
        let current_path = self.current_path.lock().unwrap().clone();
        self.play_song(current_path);
    }

    pub fn jump_to_percent(&mut self, percent: f32) {
        let total = *self.total_duration.lock().unwrap();
        let target_duration = Duration::from_secs_f32(total.as_secs_f32() * percent.clamp(0.0, 1.0));

        let mut position = self.current_position.lock().unwrap();
        *position = target_duration;

        let mut playback_started = self.playback_started.lock().unwrap();
        *playback_started = Some(Instant::now() - target_duration);
    }
    */
}

impl Drop for AudioPlayer {
    fn drop(&mut self) {
        self.stop();
    }
}
