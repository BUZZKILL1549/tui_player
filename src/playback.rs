use std::fs::File;
use std::path::PathBuf;
use std::io::BufReader;
use rodio::{Decoder, OutputStream, Sink, Source};
use std::thread;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub struct AudioPlayer {
    sink: Arc<Mutex<Option<Sink>>>,
    _stream: Option<OutputStream>,
    stream_handle: Option<rodio::OutputStreamHandle>,
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
}

impl AudioPlayer {
    pub fn new() -> Self {
        AudioPlayer {
            sink: Arc::new(Mutex::new(None)),
            _stream: None,
            stream_handle: None,
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
            
            if let Some(filename) = path.file_name() {
                if let Some(name) = filename.to_str() {
                    *self.current_song.lock().unwrap() = Some(name.to_owned());
                }
            }
            
            if self._stream.is_none() {
                match OutputStream::try_default() {
                    Ok((stream, handle)) => {
                        self._stream = Some(stream);
                        self.stream_handle = Some(handle);
                    },
                    Err(e) => {
                        eprintln!("Error creating audio output stream: {}", e);
                        return;
                    }
                }
            }
            
            *self.should_stop.lock().unwrap() = false;
            
            let should_stop_clone = Arc::clone(&self.should_stop);
            let sink_clone = Arc::clone(&self.sink);
            let path_clone = path.clone();
            let stream_handle_clone = self.stream_handle.as_ref().unwrap().clone();
            let total_duration_clone = Arc::clone(&self.total_duration);
            let is_playing_clone = Arc::clone(&self.is_playing);
            let volume_clone = Arc::clone(&self.current_volume);
            
            self.thread_handle = Some(thread::spawn(move || {
                let new_sink = match Sink::try_new(&stream_handle_clone) {
                    Ok(sink) => sink,
                    Err(e) => {
                        eprintln!("Error creating audio sink: {}", e);
                        *is_playing_clone.lock().unwrap() = false;
                        return;
                    }
                };
                
                let volume = *volume_clone.lock().unwrap();
                new_sink.set_volume(volume);
                
                *sink_clone.lock().unwrap() = Some(new_sink);
                
                let file = match File::open(&path_clone) {
                    Ok(file) => BufReader::new(file),
                    Err(e) => {
                        eprintln!("Error opening audio file: {}", e);
                        *is_playing_clone.lock().unwrap() = false;
                        return;
                    }
                };
                
                let source = match Decoder::new(file) {
                    Ok(source) => {
                        if let Some(duration) = source.total_duration() {
                            *total_duration_clone.lock().unwrap() = duration;
                        } else {
                            *total_duration_clone.lock().unwrap() = Duration::from_secs(180); // making 3 minutes as default
                        }
                        source
                    },
                    Err(e) => {
                        eprintln!("Error decoding audio file: {}", e);
                        *is_playing_clone.lock().unwrap() = false;
                        return;
                    }
                };
                
                let sink_ref = sink_clone.lock().unwrap();
                let sink = match &*sink_ref {
                    Some(s) => s,
                    None => {
                        *is_playing_clone.lock().unwrap() = false;
                        return;
                    }
                };
                
                sink.append(source);
                drop(sink_ref);
                
                loop {
                    if *should_stop_clone.lock().unwrap() {
                        if let Some(sink) = &mut *sink_clone.lock().unwrap() {
                            sink.stop();
                        }
                        *is_playing_clone.lock().unwrap() = false;
                        break;
                    }
                    
                    if let Some(sink) = &*sink_clone.lock().unwrap() {
                        if sink.empty() {
                            *is_playing_clone.lock().unwrap() = false;
                            break;
                        }
                    } else {
                        *is_playing_clone.lock().unwrap() = false;
                        break;
                    }
                    
                    thread::sleep(Duration::from_millis(100));
                }
            }));
        }
    }
    
    pub fn stop(&mut self) {
        *self.should_stop.lock().unwrap() = true;
        *self.is_playing.lock().unwrap() = false;
        *self.is_paused.lock().unwrap() = false;
        
        if let Some(sink) = &mut *self.sink.lock().unwrap() {
            sink.stop();
        }
        
        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }
        
        *self.sink.lock().unwrap() = None;
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
        
        format!("{:02}:{:02}/{:02}:{:02}", position_min, position_sec, total_min, total_sec)
    }
    
    pub fn is_playing(&self) -> bool {
        *self.is_playing.lock().unwrap()
    }
    
    pub fn current_song_name(&self) -> Option<String> {
        self.current_song.lock().unwrap().clone()
    }
    
    pub fn toggle_pause(&mut self) {
        let mut is_paused = self.is_paused.lock().unwrap();
        
        if let Some(sink) = &*self.sink.lock().unwrap() {
            if *is_paused {
                sink.play();
                *is_paused = false;
                
                // Update playback_started to account for paused time
                let mut playback_started = self.playback_started.lock().unwrap();
                let current_position = *self.current_position.lock().unwrap();
                *playback_started = Some(Instant::now() - current_position);
            } else {
                sink.pause();
                *is_paused = true;
            }
        }
    }
    
    pub fn is_paused(&self) -> bool {
        *self.is_paused.lock().unwrap()
    }
    
    pub fn seek_forward(&mut self, _seconds: f32) {
        // ill figure this shit out later
        unimplemented!();
    }
    
    pub fn seek_backward(&mut self, _seconds: f32) {
        // this too
        unimplemented!();
    }
    
    pub fn increase_volume(&mut self, amount: f32) {
        let mut volume = self.current_volume.lock().unwrap();
        *volume = (*volume + amount).min(1.0);
        
        if let Some(sink) = &*self.sink.lock().unwrap() {
            sink.set_volume(*volume);
        }
    }
    
    pub fn decrease_volume(&mut self, amount: f32) {
        let mut volume = self.current_volume.lock().unwrap();
        *volume = (*volume - amount).max(0.0);
        
        if let Some(sink) = &*self.sink.lock().unwrap() {
            sink.set_volume(*volume);
        }
    }
    
    pub fn get_volume(&self) -> f32 {
        *self.current_volume.lock().unwrap()
    }
    
    /*
    commented to skip compiler warnings
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
