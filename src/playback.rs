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
    current_song: Arc<Mutex<Option<String>>>,
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
            current_song: Arc::new(Mutex::new(None)),
        }
    }
    
    pub fn play_song(&mut self, file_path: Option<PathBuf>) {
        self.stop();
        
        if let Some(path) = file_path {
            *self.current_position.lock().unwrap() = Duration::from_secs(0);
            *self.is_playing.lock().unwrap() = true;
            *self.playback_started.lock().unwrap() = Some(Instant::now());
            
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
            
            self.thread_handle = Some(thread::spawn(move || {
                let new_sink = match Sink::try_new(&stream_handle_clone) {
                    Ok(sink) => sink,
                    Err(e) => {
                        eprintln!("Error creating audio sink: {}", e);
                        *is_playing_clone.lock().unwrap() = false;
                        return;
                    }
                };
                
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
        
        if is_playing {
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
}

impl Drop for AudioPlayer {
    fn drop(&mut self) {
        self.stop();
    }
}
