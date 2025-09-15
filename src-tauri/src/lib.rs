use serde::{Deserialize, Serialize};
use std::fs;
use std::env;
use log::{info, warn};
use tauri::{Manager, Emitter}; // Emitter'ı ekle
use tauri::webview::WebviewBuilder; // unstable
use tauri::menu::{Menu, Submenu, MenuItem, PredefinedMenuItem, AboutMetadataBuilder};
use futures::StreamExt; // StreamExt'i ekle
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use rusqlite::{Connection, params};
use redis::Client as RedisClient;
use redis::RedisError;
use redis::cmd;
use sha2::{Sha256, Digest};

#[derive(Debug, Default)]
pub struct AppState {
    page_cache: Mutex<HashMap<String, CachedPage>>, // URL -> CachedPage
    tab_ids: Mutex<HashSet<String>>,               // Active webview ids
    current_urls: Mutex<HashMap<String, String>>,  // tab_id -> current url
    last_active_tab: Mutex<Option<String>>,        // last focused/used tab id
}

#[derive(Debug)]
pub struct ChatStore {
    conn: Mutex<Connection>,
}

impl ChatStore {
    pub fn new(db_path: &str) -> Result<Self, String> {
        let conn = Connection::open(db_path).map_err(|e| format!("DB açılamadı: {}", e))?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS chat_session (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                url TEXT NOT NULL,
                created_at INTEGER NOT NULL
            );
             CREATE INDEX IF NOT EXISTS idx_chat_url ON chat_session(url);
             CREATE TABLE IF NOT EXISTS chat_message (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id INTEGER NOT NULL,
                role TEXT NOT NULL, -- user|assistant|system
                content TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                FOREIGN KEY(session_id) REFERENCES chat_session(id) ON DELETE CASCADE
             );
             CREATE INDEX IF NOT EXISTS idx_msg_session ON chat_message(session_id);
             CREATE TABLE IF NOT EXISTS app_settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS popular_site (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                title TEXT NOT NULL,
                url TEXT NOT NULL,
                color TEXT,
                icon TEXT,
                sort_order INTEGER NOT NULL DEFAULT 0
             );
             CREATE INDEX IF NOT EXISTS idx_popular_sort ON popular_site(sort_order);
             "
        ).map_err(|e| format!("DB tablo oluşturma hatası: {}", e))?;
        // Varsayılan popüler siteleri tek seferlik ekle
        if let Ok(mut stmt) = conn.prepare("SELECT COUNT(*) FROM popular_site") {
            if let Ok(mut rows) = stmt.query([]) {
                if let Some(row) = rows.next().unwrap_or(None) {
                    let count: i64 = row.get(0).unwrap_or(0);
                    if count == 0 {
                        let defaults: &[(&str, &str, &str, &str, i64)] = &[
                            ("Google", "https://google.com", "#4285F4", "fab fa-google", 1),
                            ("YouTube", "https://youtube.com", "#FF0000", "fab fa-youtube", 2),
                            ("GitHub", "https://github.com", "#333333", "fab fa-github", 3),
                            ("Stack Overflow", "https://stackoverflow.com", "#F48024", "fab fa-stack-overflow", 4),
                            ("Wikipedia", "https://wikipedia.org", "#000000", "fab fa-wikipedia-w", 5),
                            ("Reddit", "https://reddit.com", "#FF4500", "fab fa-reddit", 6),
                            ("Twitter", "https://twitter.com", "#1DA1F2", "fab fa-twitter", 7),
                            ("LinkedIn", "https://linkedin.com", "#0077B5", "fab fa-linkedin", 8),
                        ];
                        for (title, url, color, icon, sort) in defaults {
                            let _ = conn.execute(
                                "INSERT INTO popular_site(title, url, color, icon, sort_order) VALUES (?1, ?2, ?3, ?4, ?5)",
                                params![title, url, color, icon, sort]
                            );
                        }
                    }
                }
            }
        }
        Ok(Self { conn: Mutex::new(conn) })
    }

    pub fn clear_all(&self) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|_| "DB kilidi".to_string())?;
        conn.execute("DELETE FROM chat_message", params![])
            .map_err(|e| format!("chat_message temizlenemedi: {}", e))?;
        conn.execute("DELETE FROM chat_session", params![])
            .map_err(|e| format!("chat_session temizlenemedi: {}", e))?;
        Ok(())
    }

    pub fn upsert_session(&self, url: &str) -> Result<i64, String> {
        let conn = self.conn.lock().map_err(|_| "DB kilidi".to_string())?;
        // Son 24 saat içinde aynı url için mevcut oturum varsa onu kullan
        let now = chrono::Utc::now().timestamp();
        let day_ago = now - 24 * 3600;
        if let Ok(mut stmt) = conn.prepare("SELECT id FROM chat_session WHERE url = ?1 AND created_at > ?2 ORDER BY id DESC LIMIT 1") {
            if let Ok(mut rows) = stmt.query(params![url, day_ago]) {
                if let Some(row) = rows.next().unwrap_or(None) {
                    let id: i64 = row.get(0).unwrap_or(0);
                    if id > 0 { return Ok(id); }
                }
            }
        }
        conn.execute("INSERT INTO chat_session(url, created_at) VALUES (?1, ?2)", params![url, now])
            .map_err(|e| format!("session insert hatası: {}", e))?;
        Ok(conn.last_insert_rowid())
    }

    pub fn add_message(&self, session_id: i64, role: &str, content: &str) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|_| "DB kilidi".to_string())?;
        let now = chrono::Utc::now().timestamp();
        conn.execute(
            "INSERT INTO chat_message(session_id, role, content, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![session_id, role, content, now]
        ).map_err(|e| format!("message insert hatası: {}", e))?;
        Ok(())
    }

    pub fn get_messages(&self, session_id: i64, limit: i64) -> Result<Vec<(String, String)>, String> {
        let conn = self.conn.lock().map_err(|_| "DB kilidi".to_string())?;
        let mut stmt = conn.prepare("SELECT role, content FROM chat_message WHERE session_id = ?1 ORDER BY id ASC LIMIT ?2")
            .map_err(|e| format!("select prepare hatası: {}", e))?;
        let rows = stmt.query_map(params![session_id, limit], |row| {
            let role: String = row.get(0)?;
            let content: String = row.get(1)?;
            Ok((role, content))
        }).map_err(|e| format!("query_map hatası: {}", e))?;
        let mut out = Vec::new();
        for r in rows { out.push(r.map_err(|e| e.to_string())?); }
        Ok(out)
    }

    // New: generic settings helpers
    pub fn get_setting(&self, key: &str) -> Result<Option<String>, String> {
        let conn = self.conn.lock().map_err(|_| "DB kilidi".to_string())?;
        let mut stmt = conn.prepare("SELECT value FROM app_settings WHERE key = ?1")
            .map_err(|e| format!("settings select prepare hatası: {}", e))?;
        let mut rows = stmt.query(params![key])
            .map_err(|e| format!("settings query hatası: {}", e))?;
        if let Some(row) = rows.next().unwrap_or(None) {
            let val: String = row.get(0).unwrap_or_default();
            Ok(Some(val))
        } else {
            Ok(None)
        }
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|_| "DB kilidi".to_string())?;
        conn.execute(
            "INSERT INTO app_settings(key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value]
        ).map_err(|e| format!("settings upsert hatası: {}", e))?;
        Ok(())
    }

    pub fn clear_for_url(&self, url: &str) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|_| "DB kilidi".to_string())?;
        // İlgili session id'lerini bul
        let mut ids: Vec<i64> = Vec::new();
        {
            let mut stmt = conn
                .prepare("SELECT id FROM chat_session WHERE url = ?1")
                .map_err(|e| format!("session select hatası: {}", e))?;
            let mut rows = stmt
                .query(params![url])
                .map_err(|e| format!("session query hatası: {}", e))?;
            while let Some(row) = rows.next().unwrap_or(None) {
                let id: i64 = row.get(0).unwrap_or(0);
                if id > 0 { ids.push(id); }
            }
        }
        // Mesajları sil
        for sid in &ids {
            conn.execute("DELETE FROM chat_message WHERE session_id = ?1", params![sid])
                .map_err(|e| format!("mesaj silme hatası: {}", e))?;
        }
        // Oturumları sil
        conn.execute("DELETE FROM chat_session WHERE url = ?1", params![url])
            .map_err(|e| format!("session silme hatası: {}", e))?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct RedisLogger {
    url: String,
    stream_key: String,
}

impl RedisLogger {
    pub fn from_env() -> Self {
        // Öncelik sırası: NEXUS_REDIS_URL -> REDIS_URL -> bileşenlerden oluştur
        let url = std::env::var("NEXUS_REDIS_URL")
            .ok()
            .or_else(|| std::env::var("REDIS_URL").ok())
            .or_else(|| {
                let host = std::env::var("NEXUS_REDIS_HOST").ok()?;
                let port = std::env::var("NEXUS_REDIS_PORT").ok().unwrap_or_else(|| "6379".to_string());
                let password = std::env::var("NEXUS_REDIS_PASSWORD").ok()?;
                let username = std::env::var("NEXUS_REDIS_USERNAME").ok().unwrap_or_else(|| "default".to_string());
                Some(format!("rediss://{}:{}@{}:{}", username, password, host, port))
            })
            .unwrap_or_default();

        let stream_key = std::env::var("NEXUS_REDIS_STREAM")
            .unwrap_or_else(|_| "nexus:logs".to_string());

        Self { url, stream_key }
    }

    pub fn with_url(url: &str) -> Self {
        Self { url: url.to_string(), stream_key: "nexus:logs".to_string() }
    }

    pub fn log_json(&self, event: &str, payload: serde_json::Value) {
        if self.url.is_empty() { return; }
        let mut obj = match payload {
            serde_json::Value::Object(map) => map,
            _ => serde_json::Map::new(),
        };
        obj.insert("event".to_string(), serde_json::Value::String(event.to_string()));
        obj.insert("ts".to_string(), serde_json::Value::String(chrono::Utc::now().to_rfc3339()));
        let json_str = serde_json::Value::Object(obj).to_string();

        let url = self.url.clone();
        let key = self.stream_key.clone();

        std::thread::spawn(move || {
            // Bağlantı denemesi helper
            let do_try = |u: &str| -> Result<(), String> {
                let client = RedisClient::open(u).map_err(|e| format!("client: {}", e))?;
                let mut conn = client.get_connection().map_err(|e| format!("conn: {}", e))?;
                let res: Result<String, RedisError> = cmd("XADD")
                    .arg(&key)
                    .arg("*")
                    .arg("json")
                    .arg(&json_str)
                    .query(&mut conn);
                res.map(|_| ()).map_err(|e| format!("xadd: {}", e))
            };

            // Aday URL'ler: 1) verilen URL 2) insecure 3) kullanıcı adı olmadan 4) kullanıcı adı olmadan + insecure
            let mut candidates: Vec<String> = Vec::new();
            candidates.push(url.clone());
            let insecure_primary = if url.contains('?') { format!("{}&insecure=true", url) } else { format!("{}?insecure=true", url) };
            candidates.push(insecure_primary);
            if let Ok(mut parsed) = url::Url::parse(&url) {
                let _ = parsed.set_username("");
                let no_user = parsed.to_string();
                candidates.push(no_user.clone());
                let insecure_no_user = if no_user.contains('?') { format!("{}&insecure=true", no_user) } else { format!("{}?insecure=true", no_user) };
                candidates.push(insecure_no_user);
            }

            let mut last_err: Option<String> = None;
            for cand in candidates {
                match do_try(&cand) {
                    Ok(_) => { last_err = None; break; }
                    Err(e) => { warn!("Redis log yazımı başarısız ({}): {}", &cand, e); last_err = Some(e); }
                }
            }
            if let Some(e) = last_err { warn!("Redis log yazımı denemeleri başarısız: {}", e); }
        });
    }

    pub fn save_string(&self, key: &str, value: &str, ttl_seconds: Option<usize>) {
        if self.url.is_empty() { return; }
        let url = self.url.clone();
        let key = key.to_string();
        let val = value.to_string();
        std::thread::spawn(move || {
            let do_try = |u: &str| -> Result<(), String> {
                let client = RedisClient::open(u).map_err(|e| format!("client: {}", e))?;
                let mut conn = client.get_connection().map_err(|e| format!("conn: {}", e))?;
                let mut cmd_builder = cmd("SET");
                cmd_builder.arg(&key).arg(&val);
                if let Some(ttl) = ttl_seconds { cmd_builder.arg("EX").arg(ttl); }
                let res: Result<String, RedisError> = cmd_builder.query(&mut conn);
                res.map(|_| ()).map_err(|e| format!("set: {}", e))
            };
            let mut candidates: Vec<String> = Vec::new();
            candidates.push(url.clone());
            let insecure_primary = if url.contains('?') { format!("{}&insecure=true", url) } else { format!("{}?insecure=true", url) };
            candidates.push(insecure_primary);
            if let Ok(mut parsed) = url::Url::parse(&url) {
                let _ = parsed.set_username("");
                let no_user = parsed.to_string();
                candidates.push(no_user.clone());
                let insecure_no_user = if no_user.contains('?') { format!("{}&insecure=true", no_user) } else { format!("{}?insecure=true", no_user) };
                candidates.push(insecure_no_user);
            }
            let mut last_err: Option<String> = None;
            for cand in candidates {
                match do_try(&cand) {
                    Ok(_) => { last_err = None; break; }
                    Err(e) => { warn!("Redis save_string başarısız ({}): {}", &cand, e); last_err = Some(e); }
                }
            }
            if let Some(e) = last_err { warn!("Redis save_string denemeleri başarısız: {}", e); }
        });
    }
}

#[derive(Debug, Clone)]
struct CachedPage {
    content: String,
    source: String,
    fetched_at: Instant,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OllamaModel {
    pub name: String,
    pub size: Option<u64>,
    pub modified_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OllamaModelsResponse {
    pub models: Vec<OllamaModelDetail>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OllamaModelDetail {
    pub name: String,
    pub model: String,
    pub size: u64,
    pub modified_at: String,
    pub digest: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FirecrawlRequest {
    pub url: String,
    pub formats: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FirecrawlResponse {
    pub success: bool,
    pub data: Option<FirecrawlData>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FirecrawlData {
    pub markdown: Option<String>,
    pub html: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OllamaRequest {
    pub model: String,
    pub prompt: String,
    pub stream: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OllamaResponse {
    pub response: String,
    pub done: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OllamaStreamResponse {
    pub model: String,
    pub created_at: String,
    pub response: String,
    pub done: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PageInfo {
    pub title: String,
    pub favicon: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PopularSite {
    pub id: i64,
    pub title: String,
    pub url: String,
    pub color: String,
    pub icon: String,
    pub sort_order: i64,
}

// Additional ChatStore impl for Popular Sites
impl ChatStore {
    pub fn list_popular_sites(&self) -> Result<Vec<PopularSite>, String> {
        let conn = self.conn.lock().map_err(|_| "DB kilidi".to_string())?;
        let mut stmt = conn.prepare("SELECT id, title, url, color, icon, sort_order FROM popular_site ORDER BY sort_order ASC, id ASC")
            .map_err(|e| format!("popular select prepare: {}", e))?;
        let rows = stmt.query_map([], |row| {
            Ok(PopularSite {
                id: row.get(0)?,
                title: row.get(1)?,
                url: row.get(2)?,
                color: row.get(3).unwrap_or_else(|_| String::new()),
                icon: row.get(4).unwrap_or_else(|_| String::new()),
                sort_order: row.get(5).unwrap_or(0),
            })
        }).map_err(|e| format!("popular query_map: {}", e))?;
        let mut out = Vec::new();
        for r in rows { out.push(r.map_err(|e| e.to_string())?); }
        Ok(out)
    }

    pub fn save_popular_site(&self, id: Option<i64>, title: &str, url: &str, color: Option<String>, icon: Option<String>, sort_order: Option<i64>) -> Result<i64, String> {
        let conn = self.conn.lock().map_err(|_| "DB kilidi".to_string())?;
        let so = sort_order.unwrap_or(0);
        let color_ref = color.as_deref().unwrap_or("");
        let icon_ref = icon.as_deref().unwrap_or("");
        if let Some(idv) = id {
            conn.execute(
                "UPDATE popular_site SET title=?1, url=?2, color=?3, icon=?4, sort_order=?5 WHERE id=?6",
                params![title, url, color_ref, icon_ref, so, idv]
            ).map_err(|e| format!("popular update: {}", e))?;
            Ok(idv)
        } else {
            conn.execute(
                "INSERT INTO popular_site(title, url, color, icon, sort_order) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![title, url, color_ref, icon_ref, so]
            ).map_err(|e| format!("popular insert: {}", e))?;
            Ok(conn.last_insert_rowid())
        }
    }

    pub fn delete_popular_site(&self, id: i64) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|_| "DB kilidi".to_string())?;
        conn.execute("DELETE FROM popular_site WHERE id=?1", params![id]).map_err(|e| format!("popular delete: {}", e))?;
        Ok(())
    }

    pub fn reorder_popular_sites(&self, ids: Vec<i64>) -> Result<(), String> {
        let mut conn = self.conn.lock().map_err(|_| "DB kilidi".to_string())?;
        let tx = conn.transaction().map_err(|e| format!("txn: {}", e))?;
        for (idx, id) in ids.iter().enumerate() {
            tx.execute("UPDATE popular_site SET sort_order=?1 WHERE id=?2", params![ (idx as i64) + 1, id ])
                .map_err(|e| format!("reorder update: {}", e))?;
        }
        tx.commit().map_err(|e| format!("txn commit: {}", e))?;
        Ok(())
    }
}

// Tauri commands for Popular Sites
#[tauri::command]
fn get_popular_sites(store: tauri::State<'_, ChatStore>) -> Result<Vec<PopularSite>, String> {
    store.list_popular_sites()
}

#[tauri::command]
fn save_popular_site(
    store: tauri::State<'_, ChatStore>,
    id: Option<i64>,
    title: String,
    url: String,
    color: Option<String>,
    icon: Option<String>,
    sort_order: Option<i64>,
) -> Result<i64, String> {
    store.save_popular_site(id, &title, &url, color, icon, sort_order)
}

#[tauri::command]
fn delete_popular_site(store: tauri::State<'_, ChatStore>, id: i64) -> Result<(), String> {
    store.delete_popular_site(id)
}

#[tauri::command]
fn reorder_popular_sites(store: tauri::State<'_, ChatStore>, ids: Vec<i64>) -> Result<(), String> {
    store.reorder_popular_sites(ids)
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OpenRouterStreamResponse {
    pub model: String,
    pub created_at: String,
    pub response: String,
    pub done: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenRouterModelsResponse {
    data: Vec<OpenRouterModelInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenRouterModelInfo {
    id: String,
    // other fields ignored
}


// API anahtarını oku
fn read_api_key() -> Result<String, String> {
    Ok("fc-4d73624123e2456396d20be0a85c6850".to_string())
}

fn read_openrouter_api_key() -> Result<String, String> {
    Ok("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".to_string())
}

// Ollama modellerini getir
#[tauri::command]
async fn get_ollama_models(store: tauri::State<'_, ChatStore>) -> Result<Vec<OllamaModel>, String> {
    fn default_base() -> String { "http://localhost:11434".to_string() }
    let base = store
        .get_setting("ollama_base_url")
        .unwrap_or(None)
        .unwrap_or_else(default_base);
    let tags_url = format!("{}/api/tags", base.trim_end_matches('/'));
    let client = reqwest::Client::new();
    
    match client
        .get(&tags_url)
        .send()
        .await
    {
        Ok(response) => {
            if response.status().is_success() {
                let response_text = response.text().await
                    .map_err(|e| format!("Response text alma hatası: {}", e))?;
                
                println!("Ollama API response: {}", response_text);
                
                let models_response: OllamaModelsResponse = serde_json::from_str(&response_text)
                    .map_err(|e| format!("JSON parse hatası: {} - Response: {}", e, response_text))?;
                
                let models: Vec<OllamaModel> = models_response
                    .models
                    .into_iter()
                    .map(|model| OllamaModel {
                        name: model.name,
                        size: Some(model.size),
                        modified_at: Some(model.modified_at),
                    })
                    .collect();
                
                println!("Parsed models: {:?}", models);
                Ok(models)
            } else {
                Err(format!("Ollama API hatası: {} @ {}", response.status(), tags_url))
            }
        }
        Err(e) => {
            if e.is_connect() {
                Err("Ollama bağlantısı kurulamadı. 'ollama serve' çalışıyor mu?".to_string())
            } else {
                Err(format!("Ollama isteği hatası: {}", e))
            }
        }
    }
}

#[tauri::command]
async fn get_openrouter_models() -> Result<Vec<OllamaModel>, String> {
    let api_key = read_openrouter_api_key()?;
    let client = reqwest::Client::new();
    let url = "https://openrouter.ai/api/v1/models";
    let resp = client
        .get(url)
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await
        .map_err(|e| format!("OpenRouter model isteği hatası: {}", e))?;
    if !resp.status().is_success() {
        return Err(format!("OpenRouter API hatası: {}", resp.status()));
    }
    let text = resp.text().await.map_err(|e| format!("Yanıt okunamadı: {}", e))?;
    let parsed: OpenRouterModelsResponse = serde_json::from_str(&text)
        .map_err(|e| format!("Model listesi parse edilemedi: {} - {}", e, text))?;
    let models: Vec<OllamaModel> = parsed
        .data
        .into_iter()
        .map(|m| OllamaModel { name: m.id, size: None, modified_at: None })
        .collect();
    Ok(models)
}

async fn scrape_with_scrape_endpoint(url: &String, client: &reqwest::Client, api_key: &String) -> Result<String, String> {
    info!("Önce /scrape deneniyor: {}", url);
    let request_body = serde_json::json!({ "url": url });
    let response = client
        .post("https://api.firecrawl.dev/v0/scrape")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await
        .map_err(|e| format!("/scrape isteği başarısız: {}", e))?;

    let status = response.status();
    let text = response.text().await.map_err(|e| format!("/scrape yanıtı okunamadı: {}", e))?;
    info!("/scrape durumu: {}, yanıt (ilk 200): {}", status, &text[..text.len().min(200)]);

    if status.is_success() {
        let json_res: serde_json::Value = serde_json::from_str(&text).map_err(|e| format!("/scrape JSON parse hatası: {}", e))?;
        if let Some(data) = json_res.get("data") {
            if let Some(markdown) = data.get("markdown").and_then(|m| m.as_str()) {
                if !markdown.trim().is_empty() {
                    return Ok(markdown.to_string());
                }
            }
        }
    }
    
    // /scrape başarısız olursa, hatayı /crawl'a geçmek için sinyal olarak kullan.
    Err(format!("/scrape başarısız oldu veya içerik boş. Durum: {}", status))
}


async fn scrape_with_crawl_endpoint(url: &String, client: &reqwest::Client, api_key: &String) -> Result<String, String> {
    info!("/scrape başarısız oldu, /crawl deneniyor: {}", url);
    
    // Adım 1: Crawl işini başlat
    let crawl_request_body = serde_json::json!({ "url": url });
    let crawl_response = client
        .post("https://api.firecrawl.dev/v0/crawl")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&crawl_request_body)
        .send()
        .await
        .map_err(|e| format!("Firecrawl /crawl isteği başarısız: {}", e))?;

    if !crawl_response.status().is_success() {
        return Err(format!("Firecrawl /crawl başlatma hatası: {}", crawl_response.status()));
    }
    
    let crawl_response_json: serde_json::Value = crawl_response.json().await.map_err(|e| format!("Firecrawl /crawl yanıtı JSON'a çevrilemedi: {}", e))?;
    let job_id = crawl_response_json["jobId"].as_str().ok_or("Crawl yanıtında jobId bulunamadı")?.to_string();
    info!("Crawl işi başlatıldı, jobId: {}", job_id);

    // Adım 2: Durumu kontrol et
    let max_retries = 20;
    for i in 0..max_retries {
        info!("Crawl durumu kontrol ediliyor... Deneme {}", i + 1);
        let status_url = format!("https://api.firecrawl.dev/v0/crawl/status/{}", job_id);
        let status_response = client
            .get(&status_url)
            .header("Authorization", format!("Bearer {}", api_key))
            .send()
            .await
            .map_err(|e| format!("Firecrawl durum kontrolü başarısız: {}", e))?;

        if !status_response.status().is_success() {
            // Durum kontrolü geçici olarak başarısız olabilir, beklemeye devam et
            warn!("Durum kontrolü geçici hata verdi: {}. Tekrar denenecek.", status_response.status());
            tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
            continue;
        }

        let status_json: serde_json::Value = status_response.json().await.map_err(|e| format!("Firecrawl durum yanıtı JSON'a çevrilemedi: {}", e))?;
        if let Some(status) = status_json["status"].as_str() {
            match status {
                "completed" => {
                    info!("Crawl tamamlandı!");
                    if let Some(data) = status_json.get("data") {
                        if let Some(markdown) = data.get("markdown").and_then(|m| m.as_str()) {
                            if !markdown.trim().is_empty() { return Ok(markdown.to_string()); }
                        }
                    }
                    return Err("Crawl tamamlandı ancak içerik alınamadı.".to_string());
                },
                "crawling" => {
                    info!("Crawl devam ediyor...");
                    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
                },
                "failed" => return Err(format!("Firecrawl işi başarısız oldu: {}", status_json["error"].as_str().unwrap_or("Bilinmeyen hata"))),
                _ => return Err(format!("Bilinmeyen crawl durumu: {}", status)),
            }
        } else {
            return Err("Durum yanıtında 'status' alanı bulunamadı.".to_string());
        }
    }
    
    Err("Firecrawl işi zaman aşımına uğradı.".to_string())
}


// Simple HTTP fetch fallback (Firecrawl alternatifi)
async fn simple_http_fetch(url: &str) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36")
        .timeout(std::time::Duration::from_secs(10))
        .build().map_err(|e| format!("HTTP client oluşturulamadı: {}", e))?;
    
    let response = client.get(url).send().await.map_err(|e| format!("HTTP isteği başarısız: {}", e))?;
    if !response.status().is_success() {
        return Err(format!("HTTP {}", response.status()));
    }
    
    let html = response.text().await.map_err(|e| format!("HTTP yanıt okunamadı: {}", e))?;
    
    // Basit HTML temizleme (script/style etiketlerini kaldır)
    let mut clean = html.clone();
    // Regex kullanmak için önce basit string replace yapalım
    let patterns = [
        ("<script", "</script>"),
        ("<style", "</style>"),
        ("<!--", "-->"),
    ];
    
    for (start, end) in patterns {
        while let Some(start_pos) = clean.find(start) {
            if let Some(end_pos) = clean[start_pos..].find(end) {
                let full_end = start_pos + end_pos + end.len();
                clean.replace_range(start_pos..full_end, " ");
            } else {
                break;
            }
        }
    }
    
    // HTML etiketlerini kaldır (basit yöntem)
    let mut result = String::new();
    let mut in_tag = false;
    for ch in clean.chars() {
        if ch == '<' { in_tag = true; }
        else if ch == '>' { in_tag = false; }
        else if !in_tag { result.push(ch); }
    }
    
    // Fazla boşlukları temizle
    let words: Vec<&str> = result.split_whitespace().collect();
    Ok(words.join(" "))
}

// İçerik + kaynak etiketi döndürür: (content, source_label)
async fn scrape_page_content(url: String) -> Result<(String, String), String> {
    // Özel durum: YouTube sayfaları Firecrawl tarafından çoğunlukla engelleniyor.
    // Bu durumda oEmbed + og:meta etiketlerinden hafif bir özet dene.
    if is_youtube_url(&url) {
        match scrape_youtube_light(&url).await {
            Ok(md) => return Ok((md, "youtube_light".to_string())),
            Err(e) => {
                warn!("YouTube özel scraper başarısız: {} - genel akışa devam.", e);
            }
        }
    }

    // Önce Firecrawl dene
    if let Ok(api_key) = read_api_key() {
        let client = reqwest::Client::new();
        // Önce hızlı olan /scrape'i dene
        match scrape_with_scrape_endpoint(&url, &client, &api_key).await {
            Ok(markdown) => return Ok((markdown, "firecrawl_scrape".to_string())),
            Err(e) => {
                warn!("Firecrawl /scrape başarısız: {} - /crawl deneniyor.", e);
                // /crawl dene
                match scrape_with_crawl_endpoint(&url, &client, &api_key).await {
                    Ok(markdown) => return Ok((markdown, "firecrawl_crawl".to_string())),
                    Err(e2) => {
                        warn!("Firecrawl /crawl de başarısız: {} - HTTP fallback.", e2);
                    }
                }
            }
        }
    } else {
        warn!("Firecrawl API anahtarı yok - HTTP fallback.");
    }
    
    // Firecrawl başarısız olduysa: agresif HTML çıkarımı dene
    if let Ok((content, extracted)) = aggressive_html_extract(&url).await {
        if extracted {
            info!("Agresif HTML çıkarımı başarılı: {} ({} char)", url, content.len());
            return Ok((content, "aggressive_html".to_string()));
        }
    }

    // Firecrawl başarısız - basit HTTP fetch kullan
    info!("HTTP fallback ile sayfa çekiliyor: {}", url);
    simple_http_fetch(&url).await.map(|c| (c, "http_fallback".to_string()))
}

fn is_youtube_url(url: &str) -> bool {
    // Alan adını sağlam şekilde kontrol et
    if let Ok(parsed) = url::Url::parse(url) {
        if let Some(host) = parsed.host_str() {
            let h = host.to_lowercase();
            if h.ends_with("youtube.com") || h == "youtu.be" { return true; }
        }
    }
    // Fallback basit kontrol
    let lu = url.to_lowercase();
    lu.contains("youtube.com") || lu.contains("youtu.be/")
}

#[derive(Debug, Serialize, Deserialize)]
struct YoutubeOEmbed {
    title: Option<String>,
    author_name: Option<String>,
    author_url: Option<String>,
    thumbnail_url: Option<String>,
}

async fn scrape_youtube_light(url: &str) -> Result<String, String> {
    // 1) OEmbed ile başlık/kanal bilgisi al
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/117.0 Safari/537.36")
        .timeout(std::time::Duration::from_secs(10))
        .build().map_err(|e| format!("HTTP client oluşturulamadı: {}", e))?;

    let oembed_url = format!("https://www.youtube.com/oembed?url={}&format=json", url);
    let mut title: Option<String> = None;
    let mut author: Option<String> = None;

    if let Ok(resp) = client.get(&oembed_url).send().await {
        if resp.status().is_success() {
            if let Ok(text) = resp.text().await {
                if let Ok(oe) = serde_json::from_str::<YoutubeOEmbed>(&text) {
                    title = oe.title;
                    author = oe.author_name;
                }
            }
        }
    }

    // 2) Sayfa HTML'inden meta başlık/açıklama/öneri video çek
    let (desc, og_title, site_name, first_video) = match client.get(url).send().await {
        Ok(resp) if resp.status().is_success() => {
            match resp.text().await {
                Ok(html) => {
                    let d = find_meta_og_description(&html);
                    let t = find_meta_property(&html, "og:title");
                    let s = find_meta_property(&html, "og:site_name");
                    let v = find_first_watch_link(&html);
                    (d, t, s, v)
                },
                Err(_) => (None, None, None, None),
            }
        }
        _ => (None, None, None, None),
    };

    // 3) Basit markdown çıktısı oluştur
    let mut out = String::new();
    out.push_str("[YouTube Video]\n\n");
    if let Some(t) = title.clone().or(og_title.clone()) { out.push_str(&format!("Başlık: {}\n", t)); }
    if let Some(a) = author.clone() { out.push_str(&format!("Kanal: {}\n", a)); }
    if author.is_none() {
        if let Some(sn) = site_name.clone() { out.push_str(&format!("Site: {}\n", sn)); }
    }
    out.push_str(&format!("URL: {}\n\n", url));
    if let Some(d) = desc.clone() {
        out.push_str("Açıklama (kısa):\n");
        out.push_str(&d);
        out.push_str("\n");
    }
    if let Some(v) = first_video.clone() { out.push_str(&format!("Örnek video: https://www.youtube.com/watch?v={}\n", v)); }

    if out.trim().is_empty() {
        Err("YouTube içeriği çıkarılamadı".to_string())
    } else {
        Ok(out)
    }
}

fn find_meta_og_description(html: &str) -> Option<String> {
    // Basit dize arama: property="og:description" ve content="..."
    let key = "property=\"og:description\"";
    let key_alt = "property='og:description'";
    if let Some(idx) = html.find(key).or_else(|| html.find(key_alt)) {
        // Tag başlangıcını geriye doğru ara
        let start = html[..idx].rfind("<meta").unwrap_or(0);
        let end = html[idx..].find('>').map(|p| idx + p).unwrap_or(html.len());
        let segment = &html[start..end];
        // content="..." değerini al
        if let Some(val) = extract_attr_value(segment, "content") { return Some(val); }
    }
    // Alternatif: name="description"
    let key2 = "name=\"description\"";
    let key2_alt = "name='description'";
    if let Some(idx) = html.find(key2).or_else(|| html.find(key2_alt)) {
        let start = html[..idx].rfind("<meta").unwrap_or(0);
        let end = html[idx..].find('>').map(|p| idx + p).unwrap_or(html.len());
        let segment = &html[start..end];
        if let Some(val) = extract_attr_value(segment, "content") { return Some(val); }
    }
    None
}

fn find_meta_property(html: &str, property: &str) -> Option<String> {
    // property="..."
    let key = format!("property=\"{}\"", property);
    let key_alt = format!("property='{}'", property);
    if let Some(idx) = html.find(&key).or_else(|| html.find(&key_alt)) {
        let start = html[..idx].rfind("<meta").unwrap_or(0);
        let end = html[idx..].find('>').map(|p| idx + p).unwrap_or(html.len());
        let segment = &html[start..end];
        if let Some(val) = extract_attr_value(segment, "content") { return Some(val); }
    }
    None
}

fn find_first_watch_link(html: &str) -> Option<String> {
    // Basit arama: watch?v=VIDEOID
    if let Some(pos) = html.find("watch?v=") {
        let rest = &html[pos + 8..];
        // VideoID tipik olarak 11 karakter, ancak burada '&' veya '"' gelene kadar alalım
        let mut id = String::new();
        for ch in rest.chars() {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' { id.push(ch); }
            else { break; }
            if id.len() >= 20 { break; } // güvenlik sınırı
        }
        if !id.is_empty() { return Some(id); }
    }
    None
}
fn extract_attr_value(tag_segment: &str, attr: &str) -> Option<String> {
    // attr="..."
    let pat1 = format!("{}=\"", attr);
    if let Some(p) = tag_segment.find(&pat1) {
        let rest = &tag_segment[p + pat1.len()..];
        if let Some(end) = rest.find('"') { return Some(html_unescape(&rest[..end])); }
    }
    // attr='...'
    let pat2 = format!("{}='", attr);
    if let Some(p) = tag_segment.find(&pat2) {
        let rest = &tag_segment[p + pat2.len()..];
        if let Some(end) = rest.find('\'') { return Some(html_unescape(&rest[..end])); }
    }
    None
}

fn html_unescape(s: &str) -> String {
    // Çok basit kaç çözme: &amp; &quot; &apos; &lt; &gt;
    s.replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
}

// ---- Genel amaçlı agresif çıkarım ----
async fn aggressive_html_extract(url: &str) -> Result<(String, bool), String> {
    let html = match http_get_html(url).await {
        Ok(h) => h,
        Err(e) => return Err(e),
    };

    let mut out = String::new();

    // Başlık
    if let Some(title) = find_tag_text(&html, "title") {
        out.push_str(&format!("# {}\n\n", title.trim()));
    }

    // Canonical ve amphtml
    if let Some(canon) = find_link_rel_href(&html, "canonical") {
        out.push_str(&format!("Canonical: {}\n\n", canon));
    }
    let amp_link = find_link_rel_href(&html, "amphtml");

    // Meta açıklamalar
    if let Some(desc) = find_meta_property(&html, "og:description").or_else(|| find_meta_name(&html, "description")) {
        out.push_str("Özet:\n");
        out.push_str(&desc);
        out.push_str("\n\n");
    }

    if let Some(site_name) = find_meta_property(&html, "og:site_name") {
        out.push_str(&format!("Site: {}\n\n", site_name));
    }

    // JSON-LD denemesi
    if let Some(ld_blocks) = find_json_ld_blocks(&html) {
        for block in ld_blocks.into_iter().take(3) {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&block) {
                let headline = json.get("headline").and_then(|v| v.as_str()).unwrap_or("");
                let description = json.get("description").and_then(|v| v.as_str()).unwrap_or("");
                let article_body = json.get("articleBody").and_then(|v| v.as_str()).unwrap_or("");
                if !headline.is_empty() { out.push_str(&format!("Başlık (LD): {}\n\n", headline)); }
                if !description.is_empty() {
                    out.push_str("Açıklama (LD):\n");
                    out.push_str(description);
                    out.push_str("\n\n");
                }
                if !article_body.is_empty() {
                    out.push_str("İçerik (LD):\n");
                    out.push_str(article_body);
                    out.push_str("\n\n");
                }
            }
        }
    }

    // Başlıklar (h1-h3)
    let mut headings = Vec::new();
    headings.extend(find_all_tag_texts(&html, "h1").into_iter().take(3));
    headings.extend(find_all_tag_texts(&html, "h2").into_iter().take(3));
    headings.extend(find_all_tag_texts(&html, "h3").into_iter().take(3));
    if !headings.is_empty() {
        out.push_str("Başlıklar:\n");
        for h in headings { out.push_str(&format!("- {}\n", h)); }
        out.push_str("\n");
    }

    // Paragraflar (ilk ~5000 karaktere kadar)
    let mut body_acc = String::new();
    for p in find_all_tag_texts(&html, "p") {
        if body_acc.len() > 5000 { break; }
        let t = p.trim();
        if t.len() >= 20 { // çok kısa parçaları atla
            body_acc.push_str(t);
            body_acc.push_str("\n\n");
        }
    }
    if !body_acc.is_empty() {
        out.push_str("İçerik:\n");
        out.push_str(&body_acc);
    }

    // AMP varsa ve mevcut içerik kısa ise deneyelim
    if out.len() < 800 && amp_link.is_some() {
        if let Some(amp_url) = amp_link {
            if let Ok(amp_html) = http_get_html(&amp_url).await {
                let mut amp_body = String::new();
                for p in find_all_tag_texts(&amp_html, "p") {
                    if amp_body.len() > 6000 { break; }
                    let t = p.trim();
                    if t.len() >= 20 { amp_body.push_str(t); amp_body.push_str("\n\n"); }
                }
                if amp_body.len() > body_acc.len() {
                    out.push_str("\n[AMP İçerik]\n");
                    out.push_str(&amp_body);
                }
            }
        }
    }

    let extracted = out.trim().len() > 100; // anlamlı bir şey çıktı mı?
    Ok((out, extracted))
}

async fn http_get_html(url: &str) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/117.0 Safari/537.36")
        .timeout(std::time::Duration::from_secs(10))
        .build().map_err(|e| format!("HTTP client oluşturulamadı: {}", e))?;
    let response = client.get(url).send().await.map_err(|e| format!("HTTP isteği başarısız: {}", e))?;
    if !response.status().is_success() { return Err(format!("HTTP {}", response.status())); }
    response.text().await.map_err(|e| format!("HTTP yanıt okunamadı: {}", e))
}

fn find_tag_text(html: &str, tag: &str) -> Option<String> {
    let l = html.to_lowercase();
    let open = format!("<{}", tag);
    if let Some(mut start) = l.find(&open) {
        // '>' konumunu bul
        if let Some(gt) = l[start..].find('>') { start += gt + 1; } else { return None; }
        let close = format!("</{}>", tag);
        if let Some(end_rel) = l[start..].find(&close) {
            let end = start + end_rel;
            let inner = &html[start..end];
            return Some(strip_tags(inner));
        }
    }
    None
}

fn find_all_tag_texts(html: &str, tag: &str) -> Vec<String> {
    let mut out = Vec::new();
    let l = html.to_lowercase();
    let open = format!("<{}", tag);
    let close = format!("</{}>", tag);
    let mut idx = 0;
    while let Some(pos) = l[idx..].find(&open) {
        let start_tag = idx + pos;
        let after_tag = match l[start_tag..].find('>') { Some(p) => start_tag + p + 1, None => break };
        if let Some(end_rel) = l[after_tag..].find(&close) {
            let end = after_tag + end_rel;
            let inner = &html[after_tag..end];
            let text = strip_tags(inner).trim().to_string();
            if !text.is_empty() { out.push(text); }
            idx = end + close.len();
        } else { break; }
    }
    out
}

fn strip_tags(html: &str) -> String {
    let mut result = String::new();
    let mut in_tag = false;
    for ch in html.chars() {
        if ch == '<' { in_tag = true; }
        else if ch == '>' { in_tag = false; }
        else if !in_tag { result.push(ch); }
    }
    // Fazla boşluk temizliği
    let parts: Vec<&str> = result.split_whitespace().collect();
    parts.join(" ")
}

fn find_meta_name(html: &str, name: &str) -> Option<String> {
    let key = format!("name=\"{}\"", name);
    let key_alt = format!("name='{}'", name);
    if let Some(idx) = html.find(&key).or_else(|| html.find(&key_alt)) {
        let start = html[..idx].rfind("<meta").unwrap_or(0);
        let end = html[idx..].find('>').map(|p| idx + p).unwrap_or(html.len());
        let segment = &html[start..end];
        if let Some(val) = extract_attr_value(segment, "content") { return Some(val); }
    }
    None
}

fn find_link_rel_href(html: &str, rel: &str) -> Option<String> {
    // <link rel="rel" href="...">
    let rel1 = format!("rel=\"{}\"", rel);
    let rel2 = format!("rel='{}'", rel);
    let mut idx = 0usize;
    while let Some(pos) = html[idx..].find("<link") {
        let start = idx + pos;
        let end = html[start..].find('>').map(|p| start + p).unwrap_or(html.len());
        let segment = &html[start..end];
        if segment.contains(&rel1) || segment.contains(&rel2) {
            if let Some(h) = extract_attr_value(segment, "href") { return Some(h); }
        }
        idx = end + 1;
    }
    None
}

fn find_json_ld_blocks(html: &str) -> Option<Vec<String>> {
    let mut out = Vec::new();
    let mut idx = 0usize;
    loop {
        if let Some(pos) = html[idx..].find("<script") {
            let start = idx + pos;
            let end_tag = html[start..].find('>').map(|p| start + p).unwrap_or(html.len());
            let segment = &html[start..end_tag];
            if segment.contains("type=\"application/ld+json\"") || segment.contains("type='application/ld+json'") {
                // Kapanışı bul
                if let Some(close_pos) = html[end_tag..].find("</script>") {
                    let content = &html[end_tag + 1..end_tag + close_pos];
                    out.push(content.trim().to_string());
                    idx = end_tag + close_pos + "</script>".len();
                    continue;
                } else { break; }
            }
            idx = end_tag + 1;
        } else { break; }
    }
    if out.is_empty() { None } else { Some(out) }
}


// Ollama'ya soru sor - chat API ile sistem prompt desteği
#[derive(Debug, Serialize, Deserialize)]
struct OllamaChatMessage { role: String, content: String }

#[derive(Debug, Serialize, Deserialize)]
struct OllamaChatRequest { model: String, messages: Vec<OllamaChatMessage>, stream: bool }

#[derive(Debug, Serialize, Deserialize)]
struct OllamaChatStreamChunk { model: Option<String>, message: Option<OllamaChatMessage>, response: Option<String>, done: Option<bool> }

async fn query_ollama_with_content(
    window: tauri::Window,
    store: &tauri::State<'_, ChatStore>,
    content: String,
    question: String,
    model: String,
    history: Vec<(String, String)>,
) -> Result<String, String> {
    let client = reqwest::Client::new();

    // Aynı Instruction.md'yi kullanarak sistem prompt üret
    let system_prompt = read_instruction();
    let user_content = format!(
        "Aşağıdaki web sayfası içeriğini analiz et ve sorulan soruya bu içeriğe dayanarak cevap ver:\n\n---\n\nWEB SAYFASI İÇERİĞİ (özetlenmiş):\n\n{}\n\n---\n\nSORU: {}\n\n---\n\nCevabı Türkçe ve kısa, net üret.",
        &content[..content.len().min(8000)],
        question
    );

    // Geçmişi role-based mesajlara çevir
    let mut messages: Vec<OllamaChatMessage> = Vec::new();
    messages.push(OllamaChatMessage { role: "system".to_string(), content: system_prompt });
    for (role, text) in history {
        let r = match role.as_str() {
            "assistant" => "assistant",
            "system" => "system",
            _ => "user",
        };
        messages.push(OllamaChatMessage { role: r.to_string(), content: text });
    }
    // Güncel kullanıcı mesajını en sona ekle
    messages.push(OllamaChatMessage { role: "user".to_string(), content: user_content });

    let request_body = OllamaChatRequest { model: model.to_string(), messages, stream: true };

    info!("Ollama chat (stream) çağrısı: model={}", model);

    fn default_base() -> String { "http://localhost:11434".to_string() }
    let base = store
        .get_setting("ollama_base_url")
        .unwrap_or(None)
        .unwrap_or_else(default_base);
    let chat_url = format!("{}/api/chat", base.trim_end_matches('/'));

    let response = client
        .post(&chat_url)
        .json(&request_body)
        .send()
        .await
        .map_err(|e| format!("Ollama'ya bağlanılamadı: {}", e))?;
    
    let mut stream = response.bytes_stream();

    // Chunk sınırlarında JSON satırları bölünebildiği için birikimli buffer kullan
    let mut buffer = String::new();
    let mut final_text = String::new();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("Stream'den chunk okunamadı: {}", e))?;
        let chunk_str = std::str::from_utf8(&chunk).map_err(|e| format!("Chunk UTF-8'e çevrilemedi: {}", e))?;
        
        buffer.push_str(chunk_str);
        
        // Tamamlanmış satırları işle (Ollama her event'i '\n' ile bitirir)
        loop {
            if let Some(pos) = buffer.find('\n') {
                // Satırı kopyala, ardından buffer'ı kısalt
                let raw_line = buffer[..pos].to_string();
                buffer.drain(..=pos);
                let line = raw_line.trim().to_string();

                if line.is_empty() { continue; }
                // Önce chat chunk'ı olarak dene
                if let Ok(chat_chunk) = serde_json::from_str::<OllamaChatStreamChunk>(&line) {
                    let mut delta = String::new();
                    if let Some(msg) = chat_chunk.message {
                        if !msg.content.is_empty() { delta.push_str(&msg.content); }
                    }
                    if delta.is_empty() {
                        if let Some(resp) = chat_chunk.response { delta.push_str(&resp); }
                    }
                    if !delta.is_empty() {
                        final_text.push_str(&delta);
                        let evt = OllamaStreamResponse {
                            model: model.clone(),
                            created_at: chrono::Utc::now().to_rfc3339(),
                            response: delta,
                            done: chat_chunk.done.unwrap_or(false),
                        };
                        window.emit("ollama-stream", &evt).map_err(|e| e.to_string())?;
                    }
                    if chat_chunk.done.unwrap_or(false) {
                        info!("Ollama stream tamamlandı.");
                        return Ok(final_text);
                    }
                } else {
                    // Eski generate formatı fallback
                    match serde_json::from_str::<OllamaStreamResponse>(&line) {
                        Ok(ollama_response) => {
                            if !ollama_response.response.is_empty() {
                                final_text.push_str(&ollama_response.response);
                            }
                            window.emit("ollama-stream", &ollama_response).map_err(|e| e.to_string())?;
                            if ollama_response.done {
                                info!("Ollama stream tamamlandı.");
                                return Ok(final_text);
                            }
                        }
                        Err(e) => {
                            warn!("Stream satırı parse edilemedi: {} | Satır: '{}'", e, line);
                        }
                    }
                }
            } else {
                // Tam satır yok; bir sonraki chunk'ı bekle
                break;
            }
        }
    }
    
    // Akış bittiğinde buffer'da kalan son satır varsa dene
    let leftover = buffer.trim();
    if !leftover.is_empty() {
        if let Ok(chat_chunk) = serde_json::from_str::<OllamaChatStreamChunk>(leftover) {
            let mut delta = String::new();
            if let Some(msg) = chat_chunk.message { if !msg.content.is_empty() { delta.push_str(&msg.content); } }
            if delta.is_empty() { if let Some(resp) = chat_chunk.response { delta.push_str(&resp); } }
            if !delta.is_empty() {
                final_text.push_str(&delta);
                let evt = OllamaStreamResponse { model: model.clone(), created_at: chrono::Utc::now().to_rfc3339(), response: delta, done: chat_chunk.done.unwrap_or(false) };
                window.emit("ollama-stream", &evt).map_err(|e| e.to_string())?;
                if chat_chunk.done.unwrap_or(false) { return Ok(final_text); }
            }
        } else if let Ok(gen_chunk) = serde_json::from_str::<OllamaStreamResponse>(leftover) {
            if !gen_chunk.response.is_empty() { final_text.push_str(&gen_chunk.response); }
            window.emit("ollama-stream", &gen_chunk).map_err(|e| e.to_string())?;
            if gen_chunk.done { return Ok(final_text); }
        } else {
            warn!("Akış bitti ama kalan veri parse edilemedi: '{}'", leftover);
        }
    }
    
    info!("Stream beklenmedik şekilde sonlandı.");
    Ok(final_text)
}

async fn query_openrouter_with_content(window: tauri::Window, content: String, question: String, model: String, system_prompt: String) -> Result<String, String> {
    let api_key = read_openrouter_api_key()?;
    let client = reqwest::Client::new();

    // Kaliteli free modeller (güncel OpenRouter listesi) - en güçlüler en üstte
    let preferred_free_models = vec![
        "openai/gpt-oss-20b:free",
        "openai/gpt-oss-120b:free",
        "moonshotai/kimi-dev-72b:free"
    ];

    // Aday modeller: önce istenen model, sonra kaliteli free modeller, sonra diğerleri
    let mut candidates: Vec<String> = vec![model.clone()];
    
    // Önce kaliteli free modelleri ekle
    for preferred in &preferred_free_models {
        if *preferred != model {
            candidates.push(preferred.to_string());
        }
    }
    
    // Son olarak diğer ':free' modelleri ekle (nvidia vs.)
    if let Ok(models) = get_openrouter_models().await {
        for m in models {
            if m.name.ends_with(":free") && m.name != model && !preferred_free_models.contains(&m.name.as_str()) {
                candidates.push(m.name);
            }
        }
    }

    let mut last_error: Option<String> = None;
    for (idx, cand) in candidates.iter().enumerate() {
        if idx > 0 {
            log::warn!("OpenRouter fallback denemesi: {}", cand);
            window.emit("openrouter-model-fallback", &serde_json::json!({"to": cand})).ok();
        }

        // Mesaj içeriği (Gemini uyumlu tek 'user')
        let combined = format!(
            "TALİMATLAR:\n{}\n\nWEB SAYFASI İÇERİĞİ (özetlenmiş):\n{}\n\nSORU:\n{}\n\nLütfen kısa ve net cevap ver.",
            system_prompt,
            &content[..content.len().min(8000)],
            question
        );
        let body = serde_json::json!({
            "model": cand,
            "stream": true,
            "messages": [
                {"role": "user", "content": combined}
            ]
        });

        let response = client
            .post("https://openrouter.ai/api/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .header("Accept", "text/event-stream")
            .header("HTTP-Referer", "http://localhost/")
            .header("Referer", "http://localhost/")
            .header("X-Title", "Nexus Browser")
            .json(&body)
            .send()
            .await;

        let response = match response {
            Ok(r) => r,
            Err(e) => { last_error = Some(format!("İstek gönderilemedi: {}", e)); continue; }
        };

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            // 429/503 -> bir sonraki adaya geç
            if status.as_u16() == 429 || status.as_u16() == 503 {
                last_error = Some(format!("HTTP {}: {}", status, text));
                continue;
            } else {
                return Err(format!("OpenRouter HTTP {}: {}", status, text));
            }
        }

        let mut stream = response.bytes_stream();
        let mut buffer = String::new();
        let mut final_text = String::new();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| format!("SSE chunk okunamadı: {}", e))?;
            let chunk_str = std::str::from_utf8(&chunk).map_err(|e| format!("Chunk UTF-8'e çevrilemedi: {}", e))?;
            buffer.push_str(chunk_str);

            loop {
                if let Some(pos) = buffer.find('\n') {
                    let raw_line = buffer[..pos].to_string();
                    buffer.drain(..=pos);
                    let line = raw_line.trim().to_string();
                    if line.is_empty() { continue; }
                    let data_prefix = "data:";
                    if !line.starts_with(data_prefix) { continue; }
                    let payload = line[data_prefix.len()..].trim();
                    if payload == "[DONE]" { break; }
                    if payload.is_empty() { continue; }
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(payload) {
                        if let Some(err_obj) = json.get("error") {
                            let msg = if err_obj.is_object() { err_obj.get("message").and_then(|m| m.as_str()).unwrap_or("Bilinmeyen hata") } else { err_obj.as_str().unwrap_or("Bilinmeyen hata") };
                            let evt = OpenRouterStreamResponse { model: cand.clone(), created_at: chrono::Utc::now().to_rfc3339(), response: format!("[HATA] {}", msg), done: false };
                            window.emit("openrouter-stream", &evt).ok();
                        }
                        let mut delta_text = json
                            .get("choices").and_then(|c| c.as_array()).and_then(|arr| arr.get(0))
                            .and_then(|c0| c0.get("delta")).and_then(|d| d.get("content")).and_then(|c| c.as_str()).unwrap_or("");
                        if delta_text.is_empty() {
                            delta_text = json
                                .get("choices").and_then(|c| c.as_array()).and_then(|arr| arr.get(0))
                                .and_then(|c0| c0.get("message")).and_then(|m| m.get("content")).and_then(|c| c.as_str()).unwrap_or("");
                        }
                        if !delta_text.is_empty() {
                            final_text.push_str(delta_text);
                            let evt = OpenRouterStreamResponse { model: cand.clone(), created_at: chrono::Utc::now().to_rfc3339(), response: delta_text.to_string(), done: false };
                            window.emit("openrouter-stream", &evt).ok();
                        }
                    }
                } else { break; }
            }
        }

        // Stream bitti; içerik yoksa non-stream fallback dene
        if final_text.is_empty() {
            let combined = format!(
                "TALİMATLAR:\n{}\n\nWEB SAYFASI İÇERİĞİ (özetlenmiş):\n{}\n\nSORU:\n{}\n\nLütfen kısa ve net cevap ver.",
                system_prompt,
                &content[..content.len().min(8000)],
                question
            );
            let fallback_body = serde_json::json!({
                "model": cand,
                "stream": false,
                "messages": [ {"role": "user", "content": combined} ]
            });
            let resp = client
                .post("https://openrouter.ai/api/v1/chat/completions")
                .header("Authorization", format!("Bearer {}", api_key))
                .header("Content-Type", "application/json")
                .header("HTTP-Referer", "http://localhost/")
                .header("Referer", "http://localhost/")
                .header("X-Title", "Nexus Browser")
                .json(&fallback_body)
                .send().await;
            match resp {
                Ok(r) => {
                    let text = r.text().await.unwrap_or_default();
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                        let content_text = json
                            .get("choices").and_then(|c| c.as_array()).and_then(|arr| arr.get(0))
                            .and_then(|c0| c0.get("message")).and_then(|m| m.get("content")).and_then(|c| c.as_str()).unwrap_or("");
                        if !content_text.is_empty() {
                            final_text.push_str(content_text);
                            let evt = OpenRouterStreamResponse { model: cand.clone(), created_at: chrono::Utc::now().to_rfc3339(), response: content_text.to_string(), done: false };
                            window.emit("openrouter-stream", &evt).ok();
                        }
                    }
                }
                Err(e) => { last_error = Some(format!("Fallback isteği hatası: {}", e)); }
            }
        }

        let done_evt = OpenRouterStreamResponse { model: cand.clone(), created_at: chrono::Utc::now().to_rfc3339(), response: String::new(), done: true };
        window.emit("openrouter-stream", &done_evt).ok();

        if !final_text.is_empty() { return Ok(final_text); }
        // Aksi halde bir sonraki adayı dene
    }

    Err(last_error.unwrap_or_else(|| "Tüm modellerde rate-limit veya boş yanıt".to_string()))
}

// Ana soru sorma komutu
#[tauri::command]
async fn ask_question(window: tauri::Window, state: tauri::State<'_, AppState>, store: tauri::State<'_, ChatStore>, logger: tauri::State<'_, RedisLogger>, mut url: String, question: String, model: String) -> Result<(), String> {
    // Her zaman aktif sekmenin güncel URL'ini prefer et
    let original_url = url.clone();
    let mut effective_url: Option<String> = None;
    if let Ok(last) = state.last_active_tab.lock() {
        if let Some(tab) = &*last {
            if let Ok(map) = state.current_urls.lock() {
                effective_url = map.get(tab).cloned();
            }
        }
    }
    if let Some(u) = effective_url { url = u; }
    info!("'ask_question' komutu başlatıldı. URL: {} (orijinal: {})", url, original_url);

    // Cache kontrolü
    let ttl = Duration::from_secs(300); // 5 dakika TTL
    let mut use_cached = false;
    let mut cached_content: Option<String> = None;
    let mut cached_source: Option<String> = None;
    {
        if let Ok(cache) = state.page_cache.lock() {
            if let Some(entry) = cache.get(&url) {
                if entry.fetched_at.elapsed() < ttl {
                    info!("Cache hit: URL içeriği TTL içinde. Yeniden scrape edilmeyecek.");
                    use_cached = true;
                    cached_content = Some(entry.content.clone());
                    cached_source = Some(entry.source.clone());
                } else {
                    info!("Cache expired: URL içeriği süresi dolmuş. Yeniden scrape edilecek.");
                }
            }
        }
    }

    // Adım 1: Sayfayı scrape et (veya cache)
    let (content, source_label, from_cache) = if use_cached {
        (
            cached_content.unwrap_or_default(),
            cached_source.unwrap_or_else(|| "cache_unknown".to_string()),
            true,
        )
    } else {
        let (fresh, source) = scrape_page_content(url.clone()).await?;
        // Cache'e yaz ve basit boyut limiti uygula
        if let Ok(mut cache) = state.page_cache.lock() {
            if cache.len() > 16 {
                if let Some(first_key) = cache.keys().next().cloned() {
                    cache.remove(&first_key);
                }
            }
            cache.insert(url.clone(), CachedPage { content: fresh.clone(), source: source.clone(), fetched_at: Instant::now() });
        }
        (fresh, source, false)
    };

    // Detaylı log + frontend'e bilgi gönderimi
    let preview_len = content.len().min(2000);
    let preview = &content[..preview_len];
    info!(
        "MODEL KAYNAK ÖZETI | mode=ollama | url={} | source={} | from_cache={} | length={} | preview='{}'",
        url, source_label, from_cache, content.len(), preview.replace('\n', " ")
    );
    window.emit(
        "content-source",
        &serde_json::json!({
            "mode": "ollama",
            "url": url,
            "source": source_label,
            "from_cache": from_cache,
            "length": content.len(),
            "preview": preview
        })
    ).ok();

    // Redis: scraping sonucu logu + içerik blob kaydı
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    let sha = hasher.finalize();
    let sha_hex = hex::encode(sha);
    let content_key = format!("nexus:content:{}", sha_hex);
    logger.save_string(&content_key, &content, Some(60 * 60)); // 1 saat TTL
    logger.log_json("scrape_result", serde_json::json!({
        "mode": "ollama",
        "url": url,
        "source": source_label,
        "from_cache": from_cache,
        "content_length": content.len(),
        "content_key": content_key,
        "content_sha256": sha_hex,
        "content_preview": content
    }));

    // Son 10 mesajı geçmiş olarak topla (role-based kullanacağız)
    let session_id = store.upsert_session(&url)?;
    let history_pairs = store.get_messages(session_id, 10)?; // (role, content)
    // Adım 2: Ollama'ya sor (stream olarak) ve nihai cevabı al
    let assistant_text = query_ollama_with_content(window.clone(), &store, content.clone(), question.clone(), model.clone(), history_pairs).await?;

    // Mesajları DB'ye kaydet
    store.add_message(session_id, "user", &question)?;
    store.add_message(session_id, "assistant", &assistant_text)?;

    // Redis: model cevabı logu
    logger.log_json("model_answer", serde_json::json!({
        "mode": "ollama",
        "url": url,
        "model": model,
        "answer_preview": &assistant_text[..assistant_text.len().min(1000)],
        "content_source": source_label
    }));

    Ok(())
}

fn read_instruction() -> String {
    // Proje build edildiğinde dışarıdan dosya okuma sorunlarını önlemek için talimatları doğrudan koda gömüyoruz.
    r#"
# Rol ve Amaç
Nexus Browser adlı bir tarayıcıda çalışan bir Yapay Zeka Web Tarayıcısısın. Amacın, kullanıcıların ziyaret ettiği web sayfalarından veri analiz etmek ve bu verilere dayanarak kullanıcının siteyle ilgili sorduğu sorulara mümkün olduğunca doğru ve kapsamlı cevaplar sunmaktır.

# Talimatlar
- Kullanıcı aynı web sitesiyle ilgili başka bir soru sorduğunda, tekrar veri çekmene gerek yoktur. Önbellekteki (cache) mevcut veriyi incelemeye devam edebilirsin.
- Her zaman önce sana sağlanan sayfa içeriğini referans al.
- Kaynak sayfadan alıntı yaparken bilgileri özetle ve açık, net ifadeler kullan.
- Bilinmeyen konularda varsayımda bulunma, "bilmiyorum" demekten çekinme.
- Cevaplarını, sana sunulan metne sadık kalarak detaylı, bilgilendirici ve kapsamlı bir şekilde oluştur.

İşlem sırasında aşağıdaki ilkelere uy:
- Yanıtlarında yalnızca genel ve güvenli bilgiler sun; özel veya kişisel bilgiler (PII) içeren içerikleri yanıtlama.
- Kullanıcıya sorduğu soruyla ilgili olabildiğince detaylı cevap ver.

Kullanıcı chat alanına aşağıdaki gibi komutlar yazarsa, cevap stilini ona göre ayarla:
- /ozetle — Websitesi verilerini kısa özetle
- /acikla — Websitesi verilerini detaylı açıkla
- /madde — Websitesi verilerini maddeler halinde yaz
- /kaynakekle — Websitesindeki kaynakları belirt
- /kisalt — Önceki cevabını daha kısa yaz
- /uzat — Önceki cevabını daha detaylı yaz
"#.to_string()
}

#[tauri::command]
async fn ask_question_openrouter(window: tauri::Window, state: tauri::State<'_, AppState>, store: tauri::State<'_, ChatStore>, logger: tauri::State<'_, RedisLogger>, mut url: String, question: String, model: String) -> Result<(), String> {
    let original_url = url.clone();
    let mut effective_url: Option<String> = None;
    if let Ok(last) = state.last_active_tab.lock() {
        if let Some(tab) = &*last {
            if let Ok(map) = state.current_urls.lock() {
                effective_url = map.get(tab).cloned();
            }
        }
    }
    if let Some(u) = effective_url { url = u; }
    info!("'ask_question_openrouter' komutu başlatıldı. URL: {} (orijinal: {}) | model: {}", url, original_url, model);
    // Cache/scrape aynı mantık
    let ttl = Duration::from_secs(300);
    let mut use_cached = false;
    let mut cached_content: Option<String> = None;
    let mut cached_source: Option<String> = None;
    {
        if let Ok(cache) = state.page_cache.lock() {
            if let Some(entry) = cache.get(&url) {
                if entry.fetched_at.elapsed() < ttl {
                    use_cached = true;
                    cached_content = Some(entry.content.clone());
                    cached_source = Some(entry.source.clone());
                }
            }
        }
    }
    let (content, source_label, from_cache) = if use_cached {
        (
            cached_content.unwrap_or_default(),
            cached_source.unwrap_or_else(|| "cache_unknown".to_string()),
            true,
        )
    } else {
        let (fresh, source) = scrape_page_content(url.clone()).await?;
        if let Ok(mut cache) = state.page_cache.lock() {
            if cache.len() > 16 {
                if let Some(first_key) = cache.keys().next().cloned() { cache.remove(&first_key); }
            }
            cache.insert(url.clone(), CachedPage { content: fresh.clone(), source: source.clone(), fetched_at: Instant::now() });
        }
        (fresh, source, false)
    };

    // Detaylı log + frontend'e bilgi gönderimi
    let preview_len = content.len().min(2000);
    let preview = &content[..preview_len];
    info!(
        "MODEL KAYNAK ÖZETI | mode=openrouter | url={} | source={} | from_cache={} | length={} | preview='{}'",
        url, source_label, from_cache, content.len(), preview.replace('\n', " ")
    );
    window.emit(
        "content-source",
        &serde_json::json!({
            "mode": "openrouter",
            "url": url,
            "source": source_label,
            "from_cache": from_cache,
            "length": content.len(),
            "preview": preview
        })
    ).ok();

    // Redis: scraping sonucu logu + içerik blob kaydı
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    let sha = hasher.finalize();
    let sha_hex = hex::encode(sha);
    let content_key = format!("nexus:content:{}", sha_hex);
    logger.save_string(&content_key, &content, Some(60 * 60)); // 1 saat TTL
    logger.log_json("scrape_result", serde_json::json!({
        "mode": "openrouter",
        "url": url,
        "source": source_label,
        "from_cache": from_cache,
        "content_length": content.len(),
        "content_key": content_key,
        "content_sha256": sha_hex,
        "content_preview": content
    }));

    let system_prompt = read_instruction();
    let assistant_text = query_openrouter_with_content(window.clone(), content.clone(), question.clone(), model.clone(), system_prompt).await?;

    // Mesajları DB'ye kaydet (Ollama ile aynı mantık)
    let session_id = store.upsert_session(&url)?;
    store.add_message(session_id, "user", &question)?;
    store.add_message(session_id, "assistant", &assistant_text)?;

    // Redis: model cevabı logu
    logger.log_json("model_answer", serde_json::json!({
        "mode": "openrouter",
        "url": url,
        "model": model,
        "answer_preview": &assistant_text[..assistant_text.len().min(1000)],
        "content_source": source_label
    }));
    Ok(())
}

// Settings commands
#[tauri::command]
fn get_ollama_base_url(state: tauri::State<'_, ChatStore>) -> Result<String, String> {
    Ok(state
        .get_setting("ollama_base_url")?
        .unwrap_or_else(|| "http://localhost:11434".to_string()))
}

#[tauri::command]
fn set_ollama_base_url(state: tauri::State<'_, ChatStore>, value: String) -> Result<(), String> {
    // Basit doğrulama: boş olmasın
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err("Ollama URL'i boş olamaz".to_string());
    }
    state.set_setting("ollama_base_url", trimmed)?;
    Ok(())
}

#[tauri::command]
async fn reposition_webview(window: tauri::Window, tab_id: String, x: f64, y: f64, width: f64, height: f64) -> Result<(), String> {
    if let Some(webview) = window.get_webview(&tab_id) {
        webview.set_position(tauri::LogicalPosition { x, y }).map_err(|e| e.to_string())?;
        webview.set_size(tauri::LogicalSize { width, height }).map_err(|e| e.to_string())?;
        return Ok(());
    }
    Err(format!("Webview not found: {}", tab_id))
}

#[tauri::command]
async fn show_only_tab(window: tauri::Window, state: tauri::State<'_, AppState>, tab_id: String) -> Result<(), String> {
    let ids: Vec<String> = {
        let set = state.tab_ids.lock().map_err(|_| "tab set lock".to_string())?;
        set.iter().cloned().collect()
    };
    for id in ids {
        if let Some(webview) = window.get_webview(&id) {
            if id == tab_id { webview.show().map_err(|e| e.to_string())?; }
            else { webview.hide().map_err(|e| e.to_string())?; }
        }
    }
    if let Ok(mut last) = state.last_active_tab.lock() { *last = Some(tab_id); }
    Ok(())
}

// keep old show/hide commands for compatibility (no-op)
#[tauri::command]
async fn show_webview(_window: tauri::Window) -> Result<(), String> { Ok(()) }
#[tauri::command]
async fn hide_webview(_window: tauri::Window) -> Result<(), String> { Ok(()) }

// Sistemin varsayılan tarayıcısında URL aç
#[tauri::command]
async fn open_in_browser(mut url: String) -> Result<(), String> {
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        url = format!("https://{}", url);
    }
    info!("Harici tarayıcıda açılıyor: {}", url);
    open::that(url).map_err(|e| format!("Sistem tarayıcısında açılamadı: {}", e))?;
    Ok(())
}

#[tauri::command]
async fn open_or_navigate_browser_tab(window: tauri::Window, state: tauri::State<'_, AppState>, tab_id: String, url: String) -> Result<(), String> {
    // 1) Varsa mevcut webview'i yeniden kullan
    if let Some(existing) = window.get_webview(&tab_id) {
        let js_command = format!("window.location.href = '{}'", &url);
        existing.eval(&js_command).map_err(|e| e.to_string())?;

        let monitor_js = format!(r#"
            (function() {{
                let currentUrl = window.location.href;
                const tabId = '{}';
                function checkUrl() {{
                    if (window.location.href !== currentUrl) {{
                        currentUrl = window.location.href;
                        window.__TAURI__.core.invoke('notify_url_change', {{ tabId: tabId, url: currentUrl }}).catch(console.error);
                    }}
                }}
                setInterval(checkUrl, 500);
                window.addEventListener('popstate', checkUrl);
                setTimeout(() => {{
                    window.__TAURI__.core.invoke('notify_url_change', {{ tabId: tabId, url: currentUrl }}).catch(console.error);
                }}, 1000);
            }})();
        "#, tab_id);
        existing.eval(&monitor_js).map_err(|e| e.to_string())?;
        // State: aktif sekme ve URL'i güncelle
        if let Ok(mut last) = state.last_active_tab.lock() { *last = Some(tab_id.clone()); }
        if let Ok(mut map) = state.current_urls.lock() { map.insert(tab_id.clone(), url.clone()); }
        return Ok(());
    }

    // 2) Yarış koşullarını engellemek için oluşturma guard'ı kullan
    let already_in_set = {
        let mut set = state.tab_ids.lock().map_err(|_| "tab set lock".to_string())?;
        if set.contains(&tab_id) { true } else { set.insert(tab_id.clone()); false }
    };

    if already_in_set {
        // Muhtemelen başka bir görev oluşturuyor; kısa bir süre bekle ve tekrar dene
        for _ in 0..10 {
            if let Some(existing) = window.get_webview(&tab_id) {
                let js_command = format!("window.location.href = '{}'", &url);
                existing.eval(&js_command).map_err(|e| e.to_string())?;
                let monitor_js = format!(r#"
                    (function() {{
                        let currentUrl = window.location.href;
                        const tabId = '{}';
                        function checkUrl() {{
                            if (window.location.href !== currentUrl) {{
                                currentUrl = window.location.href;
                                window.__TAURI__.core.invoke('notify_url_change', {{ tabId: tabId, url: currentUrl }}).catch(console.error);
                            }}
                        }}
                        setInterval(checkUrl, 500);
                        window.addEventListener('popstate', checkUrl);
                        setTimeout(() => {{
                            window.__TAURI__.core.invoke('notify_url_change', {{ tabId: tabId, url: currentUrl }}).catch(console.error);
                        }}, 1000);
                    }})();
                "#, tab_id);
                existing.eval(&monitor_js).map_err(|e| e.to_string())?;
                return Ok(());
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
        // Hala yoksa, aşağıda oluşturmayı dene
    }

    // 3) Yeni webview oluştur
    let parsed = tauri::Url::parse(&url).map_err(|e| format!("URL parse hatası: {}", e))?;
    let builder = WebviewBuilder::new(&tab_id, tauri::WebviewUrl::External(parsed));
    match window.add_child(
        builder,
        tauri::LogicalPosition::new(0.0, 0.0),
        tauri::LogicalSize::new(1.0, 1.0),
    ) {
        Ok(webview) => {
            let monitor_js = format!(r#"
                (function() {{
                    let currentUrl = window.location.href;
                    const tabId = '{}';
                    function checkUrl() {{
                        if (window.location.href !== currentUrl) {{
                            currentUrl = window.location.href;
                            window.__TAURI__.core.invoke('notify_url_change', {{ tabId: tabId, url: currentUrl }}).catch(console.error);
                        }}
                    }}
                    setInterval(checkUrl, 500);
                    window.addEventListener('popstate', checkUrl);
                    setTimeout(() => {{
                        window.__TAURI__.core.invoke('notify_url_change', {{ tabId: tabId, url: currentUrl }}).catch(console.error);
                    }}, 1000);
                }})();
            "#, tab_id);
            webview.eval(&monitor_js).map_err(|e| format!("JavaScript inject hatası: {}", e))?;
            // State: aktif sekme ve URL'i güncelle
            if let Ok(mut last) = state.last_active_tab.lock() { *last = Some(tab_id.clone()); }
            if let Ok(mut map) = state.current_urls.lock() { map.insert(tab_id.clone(), url.clone()); }
            Ok(())
        }
        Err(e) => {
            // Oluşturma başarısız: guard'ı geri al ve eğer arada oluştuysa onu kullan
            if let Ok(mut set) = state.tab_ids.lock() { set.remove(&tab_id); }
            if let Some(existing) = window.get_webview(&tab_id) {
                let js_command = format!("window.location.href = '{}'", &url);
                existing.eval(&js_command).map_err(|e| e.to_string())?;
                let monitor_js = format!(r#"
                    (function() {{
                        let currentUrl = window.location.href;
                        const tabId = '{}';
                        function checkUrl() {{
                            if (window.location.href !== currentUrl) {{
                                currentUrl = window.location.href;
                                window.__TAURI__.core.invoke('notify_url_change', {{ tabId: tabId, url: currentUrl }}).catch(console.error);
                            }}
                        }}
                        setInterval(checkUrl, 500);
                        window.addEventListener('popstate', checkUrl);
                        setTimeout(() => {{
                            window.__TAURI__.core.invoke('notify_url_change', {{ tabId: tabId, url: currentUrl }}).catch(console.error);
                        }}, 1000);
                    }})();
                "#, tab_id);
                existing.eval(&monitor_js).map_err(|e| e.to_string())?;
                Ok(())
            } else {
                Err(format!("Child webview oluşturulamadı: {}", e))
            }
        }
    }
}

#[tauri::command]
async fn navigate_back(window: tauri::Window, tab_id: String) -> Result<(), String> {
    if let Some(webview) = window.get_webview(&tab_id) {
        webview.eval("history.back()").map_err(|e| e.to_string())?;
        return Ok(());
    }
    Err("Geri gidilemiyor".to_string())
}

#[tauri::command]
async fn navigate_forward(window: tauri::Window, tab_id: String) -> Result<(), String> {
    if let Some(webview) = window.get_webview(&tab_id) {
        webview.eval("history.forward()").map_err(|e| e.to_string())?;
        return Ok(());
    }
    Err("İleri gidilemiyor".to_string())
}

#[tauri::command]
async fn reload_page(window: tauri::Window, tab_id: String) -> Result<(), String> {
    if let Some(webview) = window.get_webview(&tab_id) {
        webview.reload().map_err(|e| e.to_string())?;
        return Ok(());
    }
    Err("Sayfa yenilenemiyor".to_string())
}

#[tauri::command]
async fn save_chat_message(state: tauri::State<'_, ChatStore>, url: String, role: String, content: String) -> Result<(), String> {
    let session_id = state.upsert_session(&url)?;
    state.add_message(session_id, &role, &content)?;
    Ok(())
}

#[tauri::command]
async fn load_chat_messages(state: tauri::State<'_, ChatStore>, url: String, limit: Option<i64>) -> Result<Vec<(String, String)>, String> {
    let session_id = state.upsert_session(&url)?; // varsa aç, yoksa oluştur
    state.get_messages(session_id, limit.unwrap_or(50))
}

#[tauri::command]
#[allow(non_snake_case)]
async fn notify_url_change(window: tauri::Window, state: tauri::State<'_, AppState>, tabId: String, url: String) -> Result<(), String> {
    // Frontend'e URL değişikliği bildir
    window.emit("webview-navigation", serde_json::json!({
        "tabId": tabId,
        "url": url
    })).map_err(|e| e.to_string())?;
    // Backend state'e güncel URL'i yaz
    if let Ok(mut map) = state.current_urls.lock() { map.insert(tabId.clone(), url.clone()); }
    if let Ok(mut last) = state.last_active_tab.lock() { *last = Some(tabId.clone()); }
    info!("URL değişti: tab={} url={} ", tabId, url);
    Ok(())
}

#[tauri::command]
async fn get_page_info(window: tauri::Window, tab_id: String, url: String) -> Result<PageInfo, String> {
    if let Some(webview) = window.get_webview(&tab_id) {
        // Get page title via JavaScript
        let title_js = r#"
            (function() {
                return document.title || '';
            })()
        "#;
        
        // Get favicon via JavaScript 
        let favicon_js = r#"
            (function() {
                var link = document.querySelector("link[rel*='icon']");
                if (link) {
                    var href = link.href;
                    if (href.startsWith('/')) {
                        var baseUrl = window.location.origin;
                        return baseUrl + href;
                    }
                    return href;
                }
                // Fallback to default favicon location
                return window.location.origin + '/favicon.ico';
            })()
        "#;

        let title_result = webview.eval(title_js);
        let favicon_result = webview.eval(favicon_js);

        let title = match title_result {
            Ok(_) => {
                // For title, we'll use a simpler approach - just hostname
                match url::Url::parse(&url) {
                    Ok(parsed_url) => {
                        parsed_url.host_str().unwrap_or("Yeni Sekme").to_string()
                    }
                    Err(_) => "Yeni Sekme".to_string()
                }
            }
            Err(_) => "Yeni Sekme".to_string()
        };

        let favicon = match favicon_result {
            Ok(_) => {
                // For favicon, generate a default one based on domain
                match url::Url::parse(&url) {
                    Ok(parsed_url) => {
                        if let Some(host) = parsed_url.host_str() {
                            format!("https://www.google.com/s2/favicons?domain={}&sz=16", host)
                        } else {
                            "".to_string()
                        }
                    }
                    Err(_) => "".to_string()
                }
            }
            Err(_) => "".to_string()
        };

        // Emit events for real-time updates
        let _ = window.emit("page-title-changed", serde_json::json!({
            "tabId": tab_id,
            "title": title
        }));

        let _ = window.emit("page-favicon-changed", serde_json::json!({
            "tabId": tab_id,
            "favicon": favicon
        }));

        Ok(PageInfo { title, favicon })
    } else {
        Err("Webview bulunamadı".to_string())
    }
}

#[tauri::command]
async fn clear_cache_for_url(state: tauri::State<'_, AppState>, store: tauri::State<'_, ChatStore>, url: String) -> Result<(), String> {
    // Bellek cache'ini temizle
    if let Ok(mut cache) = state.page_cache.lock() {
        cache.remove(&url);
    }
    // DB oturum ve mesajlarını temizle
    store.clear_for_url(&url)?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Chat verilerini kalıcı tutmamak için açılışta temizle
    let store = ChatStore::new("chat.db").expect("chat db başlatılamadı");
    let _ = store.clear_all();

    // Redis logger'ı sabit URL ile başlat (TLS - rediss)
    let redis_logger = RedisLogger::with_url(
        "redis://default:z4sLzsAMiOQgeD7jbpEvs1lPDPHdVxXI@redis-10386.crce175.eu-north-1-1.ec2.redns.redis-cloud.com:10386"
    );

    tauri::Builder::default()
        .manage(AppState::default())
        .manage(store)
        .manage(redis_logger)
        .invoke_handler(tauri::generate_handler![
            get_ollama_models,
            get_openrouter_models,
            ask_question,
            ask_question_openrouter,
            get_popular_sites,
            save_popular_site,
            delete_popular_site,
            reorder_popular_sites,
            clear_cache_for_url,
            open_in_browser,
            open_or_navigate_browser_tab,
            navigate_back,
            navigate_forward,
            reload_page,
            reposition_webview,
            show_webview,
            hide_webview,
            show_only_tab,
            save_chat_message,
            load_chat_messages,
            get_ollama_base_url,
            set_ollama_base_url,
            get_page_info,
            notify_url_change
        ])
        .on_page_load(|window, payload| {
            let tab_id = window.label().to_string();
            let url = payload.url();
            
            // Notify frontend and backend about the URL change
            let _ = window.emit("webview-navigation", serde_json::json!({ "tabId": &tab_id, "url": url }));
            if let Some(state) = window.try_state::<AppState>() {
                 if let Ok(mut map) = state.current_urls.lock() { map.insert(tab_id.clone(), url.to_string()); }
                 if let Ok(mut last) = state.last_active_tab.lock() { *last = Some(tab_id.clone()); }
            }

            // Re-inject the URL monitoring script
            let monitor_js = format!(r#"
                (function() {{
                    if (window.__url_monitor_injected) return;
                    window.__url_monitor_injected = true;
                    let currentUrl = '{}';
                    const tabId = '{}';
                    function checkUrl() {{
                        if (window.location.href !== currentUrl) {{
                            currentUrl = window.location.href;
                            try {{ window.__TAURI__.core.invoke('notify_url_change', {{ tabId: tabId, url: currentUrl }}); }} catch (e) {{ console.error(e); }}
                        }}
                    }}
                    setInterval(checkUrl, 500);
                    window.addEventListener('popstate', checkUrl);
                }})();
            "#, url, tab_id);
            let _ = window.eval(&monitor_js);
        })
        .setup(|app| {
            // Devtools otomatik açma kaldırıldı. Option+Cmd+I ile aç/kapat.

            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            // Build application menu with Settings
            let pkg = app.package_info();
            let about_md = AboutMetadataBuilder::new()
                .name(Some(pkg.name.clone()))
                .version(Some(pkg.version.to_string()))
                .build();

            // Settings (Preferences) item - add CmdOrCtrl+, as accelerator
            let settings_item = MenuItem::with_id(app, "open-settings", "Settings…", true, Some("CmdOrCtrl+,"))?;

            let app_submenu = Submenu::with_items(
                app,
                pkg.name.clone(),
                true,
                &[
                    &PredefinedMenuItem::about(app, None, Some(about_md))?,
                    &PredefinedMenuItem::separator(app)?,
                    &settings_item,
                    &PredefinedMenuItem::separator(app)?,
                    &PredefinedMenuItem::quit(app, None)?,
                ],
            )?;

            // Minimal menu: only App submenu (macOS shows it in the app menu)
            let menu = Menu::with_items(app, &[&app_submenu])?;
            let _ = app.set_menu(menu)?;
            Ok(())
        })
        .on_menu_event(|app, event| {
            // Handle Settings click
            if event.id().as_ref() == "open-settings" {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.emit("open-settings", true);
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
