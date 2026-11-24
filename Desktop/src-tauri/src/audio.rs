use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use crate::{model::Key, spectrum::{self, BANDS, WINDOW}};


use windows::Win32::Media::Audio::{
    eConsole, eRender, IAudioCaptureClient, IAudioClient,
    AUDCLNT_BUFFERFLAGS_SILENT, AUDCLNT_SHAREMODE_SHARED, AUDCLNT_STREAMFLAGS_LOOPBACK,
    MMDeviceEnumerator, IMMDeviceEnumerator, WAVEFORMATEXTENSIBLE,
};
use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED};

const HISTORY: usize = WINDOW * 2;

#[derive(Clone, Copy)]
pub struct AudioOptions {
    pub color: [u8; 3],
    pub fps: u32,
    pub gain: f32,
    pub keep_sending: bool,
}

pub struct Analyzer {
    stop: Arc<AtomicBool>,
    state: Arc<State>,
}

struct State {
    samples: Mutex<Vec<f32>>,
    updated: Mutex<Option<Instant>>,
    sample_rate: AtomicU32,
    smoothed: Mutex<[f32; BANDS]>,
    options: Mutex<AudioOptions>,
    last_sound: Mutex<Instant>,
    error: Mutex<String>,
}

impl Analyzer {
    pub fn spawn(options: AudioOptions) -> Result<Arc<Analyzer>, String> {
        let stop = Arc::new(AtomicBool::new(false));
        let state = Arc::new(State {
            samples: Mutex::new(Vec::with_capacity(HISTORY)),
            smoothed: Mutex::new([0.0; BANDS]),
            options: Mutex::new(options),
            last_sound: Mutex::new(Instant::now()),
            updated: Mutex::new(None),
            sample_rate: AtomicU32::new(48000),
            error: Mutex::new(String::new()),
        });
        let (s, st) = (state.clone(), stop.clone());
        thread::Builder::new()
            .name("f87-audio-loopback".to_string())
            .spawn(move || {
                if let Err(e) = capture(&st, &s) {
                    *s.error.lock().unwrap() = e;
                }
            })
            .map_err(|e| e.to_string())?;
        Ok(Arc::new(Analyzer { stop, state }))
    }

    pub fn configure(&self, options: AudioOptions) { *self.state.options.lock().unwrap() = options; }
    pub fn options(&self) -> AudioOptions { *self.state.options.lock().unwrap() }
    pub fn should_send(&self) -> bool {
        crate::streaming::should_send_audio(self.options().keep_sending, self.state.last_sound.lock().unwrap().elapsed())
    }

    pub fn stop(&self) {
        self.stop.store(true, Ordering::Relaxed);
    }

    pub fn failed(&self) -> Option<String> {
        let err = self.state.error.lock().unwrap().clone();
        if err.is_empty() { None } else { Some(err) }
    }

    pub fn tick(&self, keys: &[Key]) -> Vec<[u8; 3]> {
        let fresh = self.state.updated.lock().unwrap().is_some_and(|t| t.elapsed() < Duration::from_millis(150));
        let samples = self.state.samples.lock().unwrap();
        let target = if fresh && samples.len() >= WINDOW {
            spectrum::levels_with_gain(&samples[samples.len() - WINDOW..], self.state.sample_rate.load(Ordering::Relaxed), self.options().gain)
        } else { [0.0; BANDS] };
        drop(samples);
        let mut bands = self.state.smoothed.lock().unwrap();
        spectrum::smooth(&mut bands, &target);
        let min_x = keys.iter().map(|k| k.x).fold(f32::INFINITY, f32::min);
        let max_x = keys.iter().map(|k| k.x).fold(f32::NEG_INFINITY, f32::max);
        let min_y = keys.iter().map(|k| k.y).fold(f32::INFINITY, f32::min);
        let max_y = keys.iter().map(|k| k.y).fold(f32::NEG_INFINITY, f32::max);
        let color = self.options().color;
        keys.iter().map(|k| spectrum::mono_color(&bands, (k.x-min_x)/(max_x-min_x).max(1.0), (k.y-min_y)/(max_y-min_y).max(1.0), color)).collect()
    }
}

fn capture(stop: &AtomicBool, state: &State) -> Result<(), String> {
    unsafe {
        CoInitializeEx(None, COINIT_MULTITHREADED).ok().map_err(|e| e.to_string())?;
        let enumerator: IMMDeviceEnumerator =
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).map_err(|e| e.to_string())?;
        let device = enumerator.GetDefaultAudioEndpoint(eRender, eConsole).map_err(|e| e.to_string())?;
        let client: IAudioClient = device.Activate(CLSCTX_ALL, None).map_err(|e| e.to_string())?;
        let format = client.GetMixFormat().map_err(|e| e.to_string())?;
        let channels = (*format).nChannels.max(1) as usize;
        state.sample_rate.store((*format).nSamplesPerSec, Ordering::Relaxed);
        let bits = (*format).wBitsPerSample;
        let tag = if (*format).wFormatTag == 0xfffe && (*format).cbSize >= 22 {
            let subtype = (*(format as *const WAVEFORMATEXTENSIBLE)).SubFormat;
            if subtype == windows::core::GUID::from_u128(0x00000003_0000_0010_8000_00aa00389b71) { 3 }
            else if subtype == windows::core::GUID::from_u128(0x00000001_0000_0010_8000_00aa00389b71) { 1 }
            else { 0 }
        } else { (*format).wFormatTag };
        if !((tag == 3 && bits == 32) || (tag == 1 && [16, 24, 32].contains(&bits))) {
            return Err(format!("不支持的音频格式：tag={tag}, bits={bits}"));
        }
        let sample_bytes = bits as usize / 8;
        let frame_bytes = (*format).nBlockAlign as usize;
        if frame_bytes < channels * sample_bytes { return Err("音频帧格式无效".into()); }
        client
            .Initialize(AUDCLNT_SHAREMODE_SHARED, AUDCLNT_STREAMFLAGS_LOOPBACK, 10_000_000, 0, format, None)
            .map_err(|e| e.to_string())?;
        client.Start().map_err(|e| e.to_string())?;
        let capture: IAudioCaptureClient = client.GetService().map_err(|e| e.to_string())?;
        let mut held_frames: Vec<f32> = Vec::new();
        loop {
            if stop.load(Ordering::Relaxed) { break; }
            let mut got_any = false;
            loop {
                let packet = capture.GetNextPacketSize().map_err(|e| e.to_string())?;
                if packet == 0 { break; }
                let mut data = std::ptr::null_mut::<u8>();
                let mut frames = 0u32;
                let mut flags = 0u32;
                if capture.GetBuffer(&mut data, &mut frames, &mut flags, None, None).is_err() { break; }
                let count = frames as usize;
                if count > 0 {
                    if flags & AUDCLNT_BUFFERFLAGS_SILENT.0 as u32 != 0 {
                        held_frames.extend(std::iter::repeat(0.0).take(count));
                    } else {
                        let bytes = std::slice::from_raw_parts(data, count * frame_bytes);
                        for frame in bytes.chunks_exact(frame_bytes) {
                            let mut sum = 0.0;
                            for ch in 0..channels {
                                let b = &frame[ch * sample_bytes..][..sample_bytes];
                                let value = if tag == 3 {
                                    f32::from_le_bytes(b.try_into().unwrap())
                                } else { match bits {
                                    16 => i16::from_le_bytes(b.try_into().unwrap()) as f32 / 32768.0,
                                    24 => (i32::from_le_bytes([0, b[0], b[1], b[2]]) >> 8) as f32 / 8388608.0,
                                    _ => i32::from_le_bytes(b.try_into().unwrap()) as f32 / 2147483648.0,
                                }};
                                sum += if value.is_finite() { value.clamp(-1.0, 1.0) } else { 0.0 };
                            }
                            held_frames.push(sum / channels as f32);
                        }
                    }
                    got_any = true;
                }
                capture.ReleaseBuffer(frames).map_err(|e| e.to_string())?;
            }
            if got_any {
                let rms = (held_frames.iter().map(|v| v * v).sum::<f32>() / held_frames.len().max(1) as f32).sqrt();
                if rms > 0.0002 { *state.last_sound.lock().unwrap() = Instant::now(); }
                *state.updated.lock().unwrap() = Some(Instant::now());
                let mut g = state.samples.lock().unwrap();
                g.extend(held_frames.drain(..));
                if g.len() > HISTORY { let cut = g.len() - HISTORY; g.drain(0..cut); }
            }
            thread::sleep(Duration::from_millis(4));
        }
        let _ = client.Stop();
        Ok(())
    }
}
