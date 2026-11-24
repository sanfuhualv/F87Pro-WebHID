use crate::{
    audio::{Analyzer, AudioOptions},
    hid::Device,
    model::{self, Cached, CachedSummary},
    protocol,
    streaming::{PacketBudget, SEND_BUDGET},
};
use serde::Serialize;
use std::{
    collections::VecDeque,
    sync::{
        atomic::{AtomicU64, Ordering},
        mpsc, Arc, Mutex,
    },
    thread,
    time::{Duration, Instant},
};

#[derive(Clone, Serialize, Default)]
pub struct Snapshot {
    pub running: bool,
    pub connected: bool,
    pub preview_only: bool,
    pub fps: f32,
    pub packets: usize,
    pub frame: usize,
    pub message: String,
    pub current: Option<CachedSummary>,
    pub colors: Vec<[u8; 3]>,
    pub audio: bool,
    pub audio_latency_ms: f32,
    pub audio_idle: bool,
}
enum Command {
    Play(Box<Cached>, u64),
    Stop(u64),
    Quit(mpsc::Sender<()>),
    Begin(String, u64, mpsc::Sender<Result<u64, String>>),
    Send(String, u64, [u8; 20], mpsc::Sender<Result<(), String>>),
    End(String, u64, mpsc::Sender<Result<(), String>>),
    Detach(String),
    AudioStart(mpsc::Sender<Result<(), String>>),
    AudioStop(mpsc::Sender<Result<(), String>>),
}
#[derive(Default)]
struct InputQueue {
    owner: Option<(String, u64)>,
    reports: VecDeque<Vec<u8>>,
}
struct Lease {
    owner: String,
    token: u64,
    touched: Instant,
}
pub struct Controller {
    tx: mpsc::Sender<Command>,
    shared: Arc<Mutex<Snapshot>>,
    generation: Arc<AtomicU64>,
    inputs: Arc<Mutex<InputQueue>>,
    audio: Arc<Mutex<Option<Arc<Analyzer>>>>,
    next_lease: AtomicU64,
}
impl Controller {
    pub fn new(preview_only: bool) -> Self {
        let (tx, rx) = mpsc::channel();
        let shared = Arc::new(Mutex::new(Snapshot {
            preview_only,
            message: "自动查找 2.4G 接收器；通过分享码下载后播放".into(),
            ..Default::default()
        }));
        let generation = Arc::new(AtomicU64::new(0));
        let s = shared.clone();
        let g = generation.clone();
        let inputs = Arc::new(Mutex::new(InputQueue::default()));
        let input_worker = inputs.clone();
        let audio = Arc::new(Mutex::new(None));
        let audio_worker = audio.clone();
        thread::Builder::new()
            .name("f87-sequence-player".into())
            .spawn(move || worker(rx, s, g, preview_only, input_worker, audio_worker))
            .expect("player thread");
        Self {
            tx,
            shared,
            generation,
            inputs,
            audio,
            next_lease: AtomicU64::new(1),
        }
    }
    pub fn snapshot(&self) -> Snapshot {
        self.shared.lock().unwrap().clone()
    }
    pub fn hid_begin(&self, owner: String) -> Result<u64, String> {
        let token = self.next_lease.fetch_add(1, Ordering::SeqCst);
        let (tx, rx) = mpsc::channel();
        self.tx
            .send(Command::Begin(owner, token, tx))
            .map_err(|e| e.to_string())?;
        rx.recv_timeout(Duration::from_secs(2))
            .map_err(|e| e.to_string())?
    }
    pub fn hid_send(&self, owner: String, token: u64, raw: Vec<u8>) -> Result<(), String> {
        let report = protocol::validate_report(&raw)?;
        let (tx, rx) = mpsc::channel();
        self.tx
            .send(Command::Send(owner, token, report, tx))
            .map_err(|e| e.to_string())?;
        rx.recv_timeout(Duration::from_secs(2))
            .map_err(|e| e.to_string())?
    }
    pub fn hid_end(&self, owner: String, token: u64) -> Result<(), String> {
        let (tx, rx) = mpsc::channel();
        self.tx
            .send(Command::End(owner, token, tx))
            .map_err(|e| e.to_string())?;
        rx.recv_timeout(Duration::from_secs(2))
            .map_err(|e| e.to_string())?
    }
    pub fn hid_detach(&self, owner: String) {
        let _ = self.tx.send(Command::Detach(owner));
    }
    pub fn hid_poll(&self, owner: &str, token: u64) -> Vec<Vec<u8>> {
        let mut input = self.inputs.lock().unwrap();
        if input
            .owner
            .as_ref()
            .is_some_and(|(o, t)| o == owner && *t == token)
        {
            input.reports.drain(..).collect()
        } else {
            Vec::new()
        }
    }
    pub fn play(&self, animation: Cached) -> Result<(), String> {
        animation.validate()?;
        let rev = self.generation.fetch_add(1, Ordering::SeqCst) + 1;
        self.tx
            .send(Command::Play(Box::new(animation), rev))
            .map_err(|e| e.to_string())
    }
    pub fn stop(&self) -> Result<(), String> {
        let rev = self.generation.fetch_add(1, Ordering::SeqCst) + 1;
        self.tx.send(Command::Stop(rev)).map_err(|e| e.to_string())
    }
    pub fn shutdown(&self) {
        self.generation.fetch_add(1, Ordering::SeqCst);
        let (tx, rx) = mpsc::channel();
        let _ = self.tx.send(Command::Quit(tx));
        let _ = rx.recv_timeout(Duration::from_secs(2));
    }
    pub fn audio_start(&self, options: AudioOptions) -> Result<(), String> {
        {
            let mut g = self.audio.lock().unwrap();
            if let Some(a) = g.as_ref() { a.configure(options); return Ok(()); }
            else { *g = Some(Analyzer::spawn(options)?); }
        }
        let (tx, rx) = mpsc::channel();
        self.tx.send(Command::AudioStart(tx)).map_err(|e| e.to_string())?;
        rx.recv_timeout(Duration::from_secs(5)).map_err(|e| e.to_string())?
    }
    pub fn audio_stop(&self) -> Result<(), String> {
        if let Some(a) = self.audio.lock().unwrap().take() {
            a.stop();
        }
        let (tx, rx) = mpsc::channel();
        self.tx.send(Command::AudioStop(tx)).map_err(|e| e.to_string())?;
        rx.recv_timeout(Duration::from_secs(5)).map_err(|e| e.to_string()).unwrap_or(Ok(()))
    }
    pub fn audio_status(&self) -> (bool, Option<String>) {
        let g = self.audio.lock().unwrap();
        match g.as_ref() {
            Some(a) => (true, a.failed()),
            None => (false, None),
        }
    }
}
fn black(device: &Option<Device>) {
    if let Some(d) = device {
        let _ = d.send(&protocol::packet(&[0, 0, 0], 2, 1, 0));
    }
}
fn worker(
    rx: mpsc::Receiver<Command>,
    shared: Arc<Mutex<Snapshot>>,
    generation: Arc<AtomicU64>,
    preview_only: bool,
    inputs: Arc<Mutex<InputQueue>>,
    audio: Arc<Mutex<Option<Arc<Analyzer>>>>,
) {
    let keys = model::keys();
    let mut s = shared.lock().unwrap().clone();
    let mut device = None;
    let mut animation: Option<Box<Cached>> = None;
    let mut packets = Vec::new();
    let mut colors = Vec::new();
    let mut origin = Instant::now();
    let mut next = origin;
    let mut connect_at = origin - Duration::from_secs(3);
    let mut stats_at = origin;
    let mut frames = 0;
    let mut revision = 0;
    let mut lease: Option<Lease> = None;
    let mut audio_next = origin;
    let mut audio_budget = PacketBudget::new();
    loop {
        // Rust's Windows sleep uses a high-resolution timer; channel timeout
        // waits can otherwise round short audio waits to a system tick.
        let command = rx.try_recv();
        if matches!(command, Err(mpsc::TryRecvError::Empty)) {
            thread::sleep(Duration::from_millis(if lease.is_some() { 1 }
                else if s.audio_idle && s.audio { 20 } else if s.running || s.audio { 1 } else { 20 }));
        }
        match command.map_err(|e| match e {
            mpsc::TryRecvError::Empty => mpsc::RecvTimeoutError::Timeout,
            mpsc::TryRecvError::Disconnected => mpsc::RecvTimeoutError::Disconnected,
        }) {
            Ok(Command::Quit(done)) => {
                if (s.running || s.audio) && lease.is_none() {
                    black(&device);
                }
                drop(device);
                let _ = done.send(());
                break;
            }
            Ok(Command::Play(a, rev)) => {
                if rev != generation.load(Ordering::SeqCst) {
                    continue;
                }
                revision = rev;
                if let Some(prev) = audio.lock().unwrap().take() {
                    prev.stop();
                }
                s.audio = false;
                colors = a
                    .sequence
                    .frames
                    .iter()
                    .map(|f| protocol::frame_colors(f, &keys))
                    .collect::<Vec<_>>();
                packets = colors
                    .iter()
                    .map(|c| protocol::encode(c, 24))
                    .collect::<Vec<_>>();
                s.current = Some(CachedSummary::from(a.as_ref()));
                animation = Some(a);
                s.running = true;
                s.frame = 0;
                s.fps = 0.;
                origin = Instant::now();
                next = origin;
                stats_at = origin;
                frames = 0;
            }
            Ok(Command::Begin(owner, token, reply)) => {
                if lease.is_some() {
                    let _ = reply.send(Err("设备正由另一个网页事务使用，请稍后重试".into()));
                } else if device.is_none() && !preview_only {
                    let _ = reply.send(Err("接收器尚未连接".into()));
                } else {
                    lease = Some(Lease {
                        owner: owner.clone(),
                        token,
                        touched: Instant::now(),
                    });
                    *inputs.lock().unwrap() = InputQueue {
                        owner: Some((owner, token)),
                        reports: VecDeque::new(),
                    };
                    let _ = reply.send(Ok(token));
                }
            }
            Ok(Command::Send(owner, token, raw, reply)) => {
                let result = if let Some(active) = lease
                    .as_mut()
                    .filter(|l| l.owner == owner && l.token == token)
                {
                    active.touched = Instant::now();
                    if [4, 9, 2, 1, 0x88].contains(&raw[1]) {
                        s.running = false;
                        s.fps = 0.;
                        if let Some(prev) = audio.lock().unwrap().take() {
                            prev.stop();
                        }
                        s.audio = false;
                    }
                    if preview_only {
                        Ok(())
                    } else {
                        device
                            .as_ref()
                            .ok_or_else(|| "接收器已断开".into())
                            .and_then(|d| d.send(&raw))
                    }
                } else {
                    Err("设备事务已失效，未发送报告".into())
                };
                let _ = reply.send(result);
            }
            Ok(Command::End(owner, token, reply)) => {
                if lease
                    .as_ref()
                    .is_some_and(|l| l.owner == owner && l.token == token)
                {
                    lease = None;
                    *inputs.lock().unwrap() = InputQueue::default();
                    let _ = reply.send(Ok(()));
                } else {
                    let _ = reply.send(Err("设备事务已失效".into()));
                }
            }
            Ok(Command::Detach(owner)) => {
                if lease.as_ref().is_some_and(|l| l.owner == owner) {
                    lease = None;
                    *inputs.lock().unwrap() = InputQueue::default();
                }
            }
            Ok(Command::Stop(rev)) => {
                if rev != generation.load(Ordering::SeqCst) {
                    continue;
                }
                revision = rev;
                if s.running && lease.is_none() {
                    black(&device);
                }
                s.running = false;
                s.fps = 0.;
                s.message = "已停止；可从本机缓存重新选择播放".into();
            }
            Ok(Command::AudioStart(reply)) => {
                if audio.lock().unwrap().is_some() {
                    s.running = false;
                    s.fps = 0.;
                    s.audio = true;
                    s.audio_idle = false;
                    s.frame = 0;
                    frames = 0;
                    stats_at = Instant::now();
                    audio_next = stats_at;
                    audio_budget = PacketBudget::new();
                    let _ = reply.send(Ok(()));
                } else {
                    let _ = reply.send(Err("音频采集不可用".into()));
                }
            }
            Ok(Command::AudioStop(reply)) => {
                if s.audio && !s.audio_idle && lease.is_none() {
                    black(&device);
                }
                s.audio = false;
                s.message = "音频响应已停止".into();
                let _ = reply.send(Ok(()));
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                if s.running || s.audio {
                    black(&device);
                }
                break;
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }
        if lease
            .as_ref()
            .is_some_and(|l| l.touched.elapsed() > Duration::from_secs(15))
        {
            lease = None;
            *inputs.lock().unwrap() = InputQueue::default();
        }
        if !preview_only && device.is_none() && connect_at.elapsed() >= Duration::from_secs(2) {
            connect_at = Instant::now();
            match Device::open() {
                Ok(d) => {
                    device = Some(d);
                    s.message = "接收器已自动连接".into();
                }
                Err(e) => s.message = e,
            }
        }
        s.connected = device.is_some();
        let mut read_error = None;
        if let Some(d) = device.as_mut() {
            for _ in 0..128 {
                match d.poll() {
                    Ok(Some(raw)) => {
                        if lease.is_some() {
                            let mut q = inputs.lock().unwrap();
                            if q.reports.len() >= 256 {
                                q.reports.pop_front();
                            }
                            q.reports.push_back(raw.to_vec());
                        }
                    }
                    Ok(None) => break,
                    Err(e) => {
                        read_error = Some(e);
                        break;
                    }
                }
            }
        }
        if let Some(e) = read_error {
            device = None;
            lease = None;
            *inputs.lock().unwrap() = InputQueue::default();
            s.connected = false;
            s.message = format!("接收器断开，自动重连：{e}");
            connect_at = Instant::now();
        }
        if s.running
            && lease.is_none()
            && revision == generation.load(Ordering::SeqCst)
            && Instant::now() >= next
        {
            let a = animation.as_ref().unwrap();
            let index = a.sequence.index_at(origin.elapsed().as_secs_f64());
            s.frame = index + 1;
            s.colors = colors[index].clone();
            s.packets = packets[index].len();
            let mut sent = preview_only;
            if let Some(d) = &device {
                let mut failure = None;
                sent = true;
                for (i, p) in packets[index].iter().enumerate() {
                    if i > 0 && a.sequence.gap > 0 {
                        thread::sleep(Duration::from_millis(a.sequence.gap as u64));
                    }
                    if generation.load(Ordering::SeqCst) != revision {
                        sent = false;
                        break;
                    }
                    if let Err(e) = d.send(&p) {
                        failure = Some(e);
                        sent = false;
                        break;
                    }
                }
                if let Some(e) = failure {
                    device = None;
                    s.connected = false;
                    connect_at = Instant::now();
                    s.message = format!("发送失败，自动重连：{e}");
                }
            }
            if sent {
                frames += 1;
                s.message = if preview_only {
                    "预览中 · 未打开 HID".into()
                } else {
                    "本地后台播放中 · 关闭网页窗口仍会继续".into()
                };
            }
            if stats_at.elapsed() >= Duration::from_secs(1) {
                s.fps = frames as f32 / stats_at.elapsed().as_secs_f32();
                frames = 0;
                stats_at = Instant::now();
            }
            let step = 1. / a.sequence.fps as f64;
            next = origin
                + Duration::from_secs_f64(
                    (origin.elapsed().as_secs_f64() / step)
                        .floor()
                        .mul_add(step, step),
                );
        }
        let analyzer = audio.lock().unwrap().clone();
        if let Some(a) = analyzer {
            if let Some(err) = a.failed() {
                a.stop();
                *audio.lock().unwrap() = None;
                s.audio = false;
                s.message = format!("音频响应已停止：{err}");
            } else if lease.is_none() && Instant::now() >= audio_next {
                let frame_started = Instant::now();
                let interval = Duration::from_secs_f64(1.0 / a.options().fps as f64);
                let send_budget = SEND_BUDGET.min(interval.mul_f64(0.85));
                if !a.should_send() {
                    s.audio_idle = true;
                    s.fps = 0.0;
                    s.packets = 0;
                    s.colors = vec![[0; 3]; keys.len()];
                    s.message = "音频监听中".into();
                    frames = 0; stats_at = Instant::now();
                    audio_next = frame_started + interval;
                    *shared.lock().unwrap() = s.clone();
                    continue;
                }
                s.audio_idle = false;
                let limit = ((send_budget.as_secs_f64() / audio_budget.packet_time().as_secs_f64()).floor() as usize).clamp(1, audio_budget.limit());
                let frame = protocol::encode_bounded_frame(&a.tick(&keys), limit);
                s.audio = true;
                s.packets = frame.reports.len();
                let send_started = Instant::now();
                let mut completed = 0;
                let mut complete = true;
                let mut result = Ok(());
                if !preview_only {
                    if let Some(d) = &device {
                        for report in &frame.reports {
                            let remaining = send_budget.saturating_sub(frame_started.elapsed());
                            if remaining < audio_budget.packet_time().max(Duration::from_millis(1)) { complete = false; break; }
                            result = d.send_realtime(report, remaining.as_millis() as u32);
                            if result.is_err() { complete = false; break; }
                            completed += 1;
                        }
                        audio_budget.observe(send_started.elapsed(), completed, complete);
                    }
                }
                s.audio_latency_ms = frame_started.elapsed().as_secs_f32() * 1000.0;
                match result {
                    Ok(()) if complete => {
                        frames += 1;
                        s.frame += 1;
                        s.colors = frame.colors;
                        s.message = if preview_only || !s.connected {
                            "音频频谱 · 自适应包数 · 预览模式".into()
                        } else { "音频频谱 · 自适应包数".into() };
                    }
                    Ok(()) => { s.message = "音频帧超时，已丢弃并降低包数".into(); }
                    Err(e) if e.starts_with("HID 写入超时") => {
                        s.message = "音频帧超时，已取消写入并降低包数".into();
                    }
                    Err(e) => {
                        device = None;
                        s.connected = false;
                        connect_at = Instant::now();
                        s.message = format!("音频发送失败，自动重连：{e}");
                    }
                }
                if stats_at.elapsed() >= Duration::from_secs(1) {
                    s.fps = frames as f32 / stats_at.elapsed().as_secs_f32();
                    frames = 0;
                    stats_at = Instant::now();
                }
                // Skip overdue frames instead of queuing a catch-up burst. Work and
                // USB latency are included in this frame's 33.3 ms budget.
                audio_next = (frame_started + interval).max(Instant::now());
            }
        } else if s.audio {
            s.audio = false;
        }
        *shared.lock().unwrap() = s.clone();
    }
}
