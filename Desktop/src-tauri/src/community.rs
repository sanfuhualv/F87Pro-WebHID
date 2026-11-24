use crate::model::{
    valid_cache_key, valid_code, valid_id, valid_name, Animation, Cached, CachedSummary, Sequence,
};
use std::os::windows::ffi::OsStrExt;
use std::{fs, io::Write, path::PathBuf, sync::Mutex, time::Instant};
use tauri::Url;
use windows::{
    core::{w, PCWSTR},
    Win32::{
        Networking::WinHttp::*,
        Storage::FileSystem::{MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH},
    },
};

pub const FRONTEND: &str = "https://sanfuhualv.com/F87ProWebHID/";
pub const API_BASE: &str = "https://api.sanfuhualv.com/f87pro-webhid";
pub struct Config {
    pub frontend: Url,
    pub api_base: String,
}
impl Config {
    pub fn new() -> Self {
        Self {
            frontend: Url::parse(FRONTEND).expect("valid frontend URL"),
            api_base: API_BASE.into(),
        }
    }
}

struct HttpHandle(*mut std::ffi::c_void);
impl HttpHandle {
    fn new(p: *mut std::ffi::c_void) -> Result<Self, String> {
        if p.is_null() {
            Err(windows::core::Error::from_win32().to_string())
        } else {
            Ok(Self(p))
        }
    }
}
impl Drop for HttpHandle {
    fn drop(&mut self) {
        unsafe {
            let _ = WinHttpCloseHandle(self.0);
        }
    }
}
fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}
#[derive(serde::Serialize)]
pub struct HttpResponse {
    pub status: u32,
    pub body: String,
}
fn request(
    url: &str,
    method: &str,
    body: &str,
    headers: &str,
    max_bytes: usize,
) -> Result<(u32, Vec<u8>), String> {
    unsafe {
        let url = Url::parse(url).map_err(|e| e.to_string())?;
        let session = HttpHandle::new(WinHttpOpen(
            w!("F87Desktop/0.2"),
            WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY,
            PCWSTR::null(),
            PCWSTR::null(),
            0,
        ))?;
        WinHttpSetTimeouts(session.0, 5000, 5000, 5000, 5000).map_err(|e| e.to_string())?;
        let host = wide(url.host_str().ok_or("缺少服务域名")?);
        let connection = HttpHandle::new(WinHttpConnect(
            session.0,
            PCWSTR(host.as_ptr()),
            url.port_or_known_default().ok_or("服务端口无效")?,
            0,
        ))?;
        let path = wide(&format!(
            "{}{}",
            url.path(),
            url.query().map(|q| format!("?{q}")).unwrap_or_default()
        ));
        let verb = wide(method);
        let secure = if url.scheme() == "https" {
            WINHTTP_FLAG_SECURE
        } else {
            WINHTTP_OPEN_REQUEST_FLAGS(0)
        };
        let request = HttpHandle::new(WinHttpOpenRequest(
            connection.0,
            PCWSTR(verb.as_ptr()),
            PCWSTR(path.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            std::ptr::null(),
            secure,
        ))?;
        WinHttpSetOption(
            Some(request.0),
            WINHTTP_OPTION_REDIRECT_POLICY,
            Some(&WINHTTP_OPTION_REDIRECT_POLICY_NEVER.to_ne_bytes()),
        )
        .map_err(|e| e.to_string())?;
        let header_data: Vec<u16> = headers.encode_utf16().collect();
        WinHttpSendRequest(
            request.0,
            if header_data.is_empty() {
                None
            } else {
                Some(&header_data)
            },
            if body.is_empty() {
                None
            } else {
                Some(body.as_ptr().cast())
            },
            body.len() as u32,
            body.len() as u32,
            0,
        )
        .map_err(|e| format!("请求失败：{e}"))?;
        WinHttpReceiveResponse(request.0, std::ptr::null_mut()).map_err(|e| e.to_string())?;
        let mut status = 0u32;
        let mut len = 4;
        WinHttpQueryHeaders(
            request.0,
            WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER,
            PCWSTR::null(),
            Some((&mut status as *mut u32).cast()),
            &mut len,
            std::ptr::null_mut(),
        )
        .map_err(|e| e.to_string())?;
        let start = Instant::now();
        let mut out = Vec::new();
        let mut chunk = [0u8; 8192];
        loop {
            let mut n = 0;
            WinHttpReadData(
                request.0,
                chunk.as_mut_ptr().cast(),
                chunk.len() as u32,
                &mut n,
            )
            .map_err(|e| e.to_string())?;
            if n == 0 {
                break;
            }
            if out.len() + n as usize > max_bytes {
                return Err("社区响应超过大小限制".into());
            }
            if start.elapsed().as_secs() > 20 {
                return Err("作品下载超时".into());
            }
            out.extend_from_slice(&chunk[..n as usize]);
        }
        Ok((status, out))
    }
}
fn get(url: &str) -> Result<Vec<u8>, String> {
    let (status, body) = request(url, "GET", "", "", 256 * 1024)?;
    if status == 200 {
        Ok(body)
    } else {
        Err(match status {
            404 => "作品或分享码不存在 / 已被删除".into(),
            401 | 403 => "此作品无法匿名下载，请在网页登录后载入播放".into(),
            _ => format!("下载失败，HTTP {status}"),
        })
    }
}
pub fn proxy(
    api_base: &str,
    path: &str,
    method: &str,
    body: &str,
    headers: &std::collections::BTreeMap<String, String>,
    cookies: &str,
) -> Result<HttpResponse, String> {
    if !path.starts_with("/api/")
        || path.contains("..")
        || path.len() > 2048
        || !path
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"/-_?=&".contains(&c))
        || !["GET", "POST", "DELETE"].contains(&method)
        || body.len() > 256 * 1024
    {
        return Err("社区请求不在允许范围内".into());
    }
    let mut h=format!("Origin: https://sanfuhualv.com\r\nReferer: {FRONTEND}\r\nContent-Type: application/json\r\nAccept: application/json\r\n");
    for (k, v) in headers {
        if ["x-csrf-token", "idempotency-key"].contains(&k.to_ascii_lowercase().as_str()) {
            if v.contains(['\r', '\n']) || v.len() > 256 {
                return Err("社区请求头无效".into());
            }
            h.push_str(&format!("{k}: {v}\r\n"));
        }
    }
    if !cookies.is_empty() {
        if cookies.contains(['\r', '\n']) {
            return Err("会话 Cookie 无效".into());
        }
        h.push_str(&format!("Cookie: {cookies}\r\n"));
    }
    let (status, bytes) = request(&format!("{api_base}{path}"), method, body, &h, 512 * 1024)?;
    Ok(HttpResponse {
        status,
        body: String::from_utf8(bytes).map_err(|e| e.to_string())?,
    })
}

pub struct Library {
    root: PathBuf,
    lock: Mutex<()>,
}
impl Library {
    pub fn new() -> Self {
        let dir = std::env::var_os("F87_DESKTOP_DATA_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                std::env::var_os("LOCALAPPDATA")
                    .map(PathBuf::from)
                    .unwrap_or_else(std::env::temp_dir)
                    .join("F87ProDesktop")
            });
        Self::at(dir.join("animations"))
    }
    fn at(root: PathBuf) -> Self {
        Self {
            root,
            lock: Mutex::new(()),
        }
    }
    pub fn save(&self, a: &Cached) -> Result<(), String> {
        a.validate()?;
        let _guard = self.lock.lock().unwrap();
        fs::create_dir_all(&self.root).map_err(|e| e.to_string())?;
        let target = self.root.join(format!("{}.json", a.key));
        if !target.exists()
            && fs::read_dir(&self.root)
                .map_err(|e| e.to_string())?
                .filter_map(Result::ok)
                .filter(|f| f.path().extension().is_some_and(|e| e == "json"))
                .count()
                >= 100
        {
            return Err("本机缓存已达到 100 份，请整理缓存目录后重试".into());
        }
        let bytes = serde_json::to_vec(a).map_err(|e| e.to_string())?;
        if bytes.len() > 512 * 1024 {
            return Err("动画缓存过大".into());
        }
        let temp = target.with_extension("tmp");
        let mut f = fs::File::create(&temp).map_err(|e| e.to_string())?;
        f.write_all(&bytes)
            .and_then(|_| f.sync_all())
            .map_err(|e| e.to_string())?;
        drop(f);
        let src: Vec<u16> = temp.as_os_str().encode_wide().chain(Some(0)).collect();
        let dst: Vec<u16> = target.as_os_str().encode_wide().chain(Some(0)).collect();
        unsafe {
            MoveFileExW(
                PCWSTR(src.as_ptr()),
                PCWSTR(dst.as_ptr()),
                MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
            )
            .map_err(|e| e.to_string())
        }
    }
    pub fn read(&self, key: &str) -> Result<Cached, String> {
        if !valid_cache_key(key) {
            return Err("缓存标识无效".into());
        }
        let p = self.root.join(format!("{key}.json"));
        if fs::metadata(&p).map_err(|e| e.to_string())?.len() > 512 * 1024 {
            return Err("缓存文件过大".into());
        }
        let a: Cached = serde_json::from_slice(&fs::read(p).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        a.validate()?;
        if a.key != key {
            return Err("缓存标识不匹配".into());
        }
        Ok(a)
    }
    pub fn list(&self) -> Vec<CachedSummary> {
        let mut list = Vec::new();
        if let Ok(entries) = fs::read_dir(&self.root) {
            for e in entries.flatten() {
                if e.path().extension().is_some_and(|x| x == "json") {
                    if let Some(key) = e.path().file_stem().and_then(|v| v.to_str()) {
                        if let Ok(a) = self.read(key) {
                            list.push(CachedSummary::from(&a));
                        }
                    }
                }
            }
        }
        list.sort_by(|a, b| a.name.cmp(&b.name));
        list
    }
}
pub fn parse_download(
    bytes: &[u8],
    code: Option<&str>,
    id: Option<&str>,
) -> Result<Cached, String> {
    let a: Animation = serde_json::from_slice(bytes).map_err(|e| format!("作品数据无效：{e}"))?;
    a.validate()?;
    let key = if let Some(c) = code {
        if !valid_code(c) || a.visibility != "unlisted" || a.share_code.as_deref() != Some(c) {
            return Err("分享码与返回作品不匹配".into());
        }
        format!("share-{c}")
    } else {
        let id = id.ok_or("缺少作品标识")?;
        if !valid_id(id) || a.id != id || a.visibility != "public" {
            return Err("公开作品信息不匹配".into());
        }
        format!("public-{id}")
    };
    Ok(Cached {
        key,
        name: a.name,
        author: a.author,
        share_code: a.share_code,
        online_id: Some(a.id),
        sequence: a.sequence,
    })
}
pub fn download(api_base: &str, code: Option<&str>, id: Option<&str>) -> Result<Cached, String> {
    let route = if let Some(code) = code {
        if !valid_code(code) {
            return Err("请输入完整的 22 位分享码".into());
        }
        format!("shares/{code}")
    } else {
        let id = id.ok_or("缺少作品标识")?;
        if !valid_id(id) {
            return Err("作品标识无效".into());
        }
        format!("animations/{id}")
    };
    parse_download(&get(&format!("{api_base}/api/{route}"))?, code, id)
}
pub fn draft(name: String, sequence: Sequence) -> Result<Cached, String> {
    if !valid_name(&name) {
        return Err("动画名称无效".into());
    }
    sequence.validate()?;
    Ok(Cached {
        key: "draft".into(),
        name,
        author: String::new(),
        share_code: None,
        online_id: None,
        sequence,
    })
}

