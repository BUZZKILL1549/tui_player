use std::fs::File;
use std::path::PathBuf;
use std::io::BufReader;
use rodio::{Decoder, OutputStream, Sink};
use std::thread;
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub struct AudioPlayer {
    sink: Arc<Mutex<Option<Sink>>>,
    _stream: Option<OutputStream>,
    stream_handle: Option<rodio::OutputStreamHandle>,
    thread_handle: Option<thread::JoinHandle<()>>,
    should_stop: Arc<Mutex<bool>>,
}

impl AudioPlayer {
    pub fn new() -> Self {
        AudioPlayer {
            sink: Arc::new(Mutex::new(None)),
            _stream: None,
            stream_handle: None,
            thread_handle: None,
            should_stop: Arc::new(Mutex::new(false)),
        }
    }
    
    pub fn play_song(&mut self, file_path: Option<PathBuf>) {
        self.stop();
        
        if let Some(path) = file_path {
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
            
            self.thread_handle = Some(thread::spawn(move || {
                let new_sink = match Sink::try_new(&stream_handle_clone) {
                    Ok(sink) => sink,
                    Err(e) => {
                        eprintln!("Error creating audio sink: {}", e);
                        return;
                    }
                };
                
                *sink_clone.lock().unwrap() = Some(new_sink);
                
                let sink_ref = sink_clone.lock().unwrap();
                let sink = match &*sink_ref {
                    Some(s) => s,
                    None => return, // hopefully doesnt happen
                };
                
                let file = match File::open(&path_clone) {
                    Ok(file) => BufReader::new(file),
                    Err(e) => {
                        eprintln!("Error opening audio file: {}", e);
                        return;
                    }
                };
                
                let source = match Decoder::new(file) {
                    Ok(source) => source,
                    Err(e) => {
                        eprintln!("Error decoding audio file: {}", e);
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
                        break;
                    }
                    
                    if let Some(sink) = &*sink_clone.lock().unwrap() {
                        if sink.empty() {
                            break;
                        }
                    } else {
                        break;
                    }
                    
                    thread::sleep(Duration::from_millis(100));
                }
            }));
        }
    }
    
    pub fn stop(&mut self) {
        *self.should_stop.lock().unwrap() = true;
        
        if let Some(sink) = &mut *self.sink.lock().unwrap() {
            sink.stop();
        }
        
        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }
        
        *self.sink.lock().unwrap() = None;
    }
}

impl Drop for AudioPlayer {
    fn drop(&mut self) {
        self.stop();
    }
}
