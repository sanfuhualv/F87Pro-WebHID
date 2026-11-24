use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Key {
    pub index: u8,
    pub x: f32,
    pub y: f32,
}
pub fn keys() -> Vec<Key> {
    serde_json::from_str(include_str!("../../ui/keys.json")).expect("embedded keyboard")
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Frame {
    #[serde(default)]
    pub id: String,
    pub brightness: u8,
    pub colors: BTreeMap<u8, [u8; 3]>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Sequence {
    pub version: u8,
    pub fps: u8,
    pub gap: u8,
    #[serde(rename = "loopAt")]
    pub loop_at: Option<usize>,
    pub frames: Vec<Frame>,
}
impl Sequence {
    pub fn validate(&self) -> Result<(), String> {
        if self.version != 2
            || !(1..=30).contains(&self.fps)
            || self.gap > 10
            || self.frames.is_empty()
            || self.frames.len() > 120
        {
            return Err("需要 version 2 动画，1–120 帧、1–30 FPS、0–10 ms 包间隔".into());
        }
        if self.loop_at.is_some_and(|i| i >= self.frames.len()) {
            return Err("循环点超出帧范围".into());
        }
        let keys = keys();
        let mut ids = std::collections::HashSet::new();
        for f in &self.frames {
            if f.id.is_empty()
                || f.id.len() > 64
                || !ids.insert(&f.id)
                || !f
                    .id
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
                || f.brightness > 100
                || f.colors.len() != 87
                || keys.iter().any(|k| !f.colors.contains_key(&k.index))
            {
                return Err("帧标识无效，或未包含完整 87 键 RGB 配色 / 0–100 亮度".into());
            }
        }
        Ok(())
    }
    pub fn index_at(&self, seconds: f64) -> usize {
        let tick = (seconds.max(0.) * self.fps as f64).floor() as usize;
        if tick < self.frames.len() {
            tick
        } else {
            let start = self.loop_at.unwrap_or(0);
            start + (tick - self.frames.len()) % (self.frames.len() - start)
        }
    }
}

pub fn valid_code(code: &str) -> bool {
    code.len() == 22
        && code
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
}
pub fn valid_id(id: &str) -> bool {
    id.len() == 32
        && id
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}
pub fn valid_name(name: &str) -> bool {
    !name.trim().is_empty() && name.chars().count() <= 80 && !name.chars().any(char::is_control)
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Animation {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub share_code: Option<String>,
    pub visibility: String,
    pub sequence: Sequence,
}
impl Animation {
    pub fn validate(&self) -> Result<(), String> {
        if !valid_id(&self.id) || !valid_name(&self.name) || self.author.chars().count() > 32 {
            return Err("作品信息无效".into());
        }
        self.sequence.validate()
    }
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Cached {
    pub key: String,
    pub name: String,
    pub author: String,
    pub share_code: Option<String>,
    pub online_id: Option<String>,
    pub sequence: Sequence,
}
impl Cached {
    pub fn validate(&self) -> Result<(), String> {
        if !valid_name(&self.name)
            || !valid_cache_key(&self.key)
            || self.share_code.as_ref().is_some_and(|c| !valid_code(c))
            || self.online_id.as_ref().is_some_and(|id| !valid_id(id))
        {
            return Err("本地缓存信息无效".into());
        }
        self.sequence.validate()
    }
}
pub fn valid_cache_key(key: &str) -> bool {
    key == "draft"
        || key.strip_prefix("share-").is_some_and(valid_code)
        || key.strip_prefix("public-").is_some_and(valid_id)
}
#[derive(Clone, Serialize)]
pub struct CachedSummary {
    pub key: String,
    pub name: String,
    pub author: String,
    pub share_code: Option<String>,
    pub frames: usize,
    pub fps: u8,
}
impl From<&Cached> for CachedSummary {
    fn from(c: &Cached) -> Self {
        Self {
            key: c.key.clone(),
            name: c.name.clone(),
            author: c.author.clone(),
            share_code: c.share_code.clone(),
            frames: c.sequence.frames.len(),
            fps: c.sequence.fps,
        }
    }
}

