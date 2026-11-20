//! Membaca live chat YouTube tanpa API key (cara yang sama dipakai popout chat di browser).

use crate::i18n::{self, tr};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::time::{Duration, Instant};

const UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36";
const COOKIE: &str = "SOCS=CAI; CONSENT=YES+";
const DEFAULT_CLIENT_VERSION: &str = "2.20241101.01.00";

#[derive(Clone, Copy, PartialEq)]
pub enum Role {
    Normal,
    Member,
    Moderator,
    Owner,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Kind {
    Text,
    Paid,
    Membership,
}

pub struct ChatItem {
    pub id: String,
    pub author: String,
    pub text: String,
    pub role: Role,
    pub kind: Kind,
    pub amount: Option<String>,
}

pub enum Event {
    Status { gen: u64, text: String },
    Items { gen: u64, items: Vec<ChatItem> },
}

enum Target {
    Video(String),
    Channel(String),
}

/// Jalankan pembaca chat di thread sendiri. Thread berhenti otomatis
/// kalau `current` sudah tidak sama dengan `gen` (user menekan Hubungkan lagi).
pub fn spawn(
    input: String,
    gen: u64,
    current: Arc<AtomicU64>,
    tx: Sender<Event>,
    ctx: eframe::egui::Context,
) {
    std::thread::spawn(move || {
        let mut w = Worker { gen, current, tx, ctx, last_status: String::new() };
        w.run(&input);
    });
}

struct Worker {
    gen: u64,
    current: Arc<AtomicU64>,
    tx: Sender<Event>,
    ctx: eframe::egui::Context,
    last_status: String,
}

impl Worker {
    fn alive(&self) -> bool {
        self.current.load(Ordering::SeqCst) == self.gen
    }

    fn status(&mut self, s: impl Into<String>) {
        let s = s.into();
        if s == self.last_status || !self.alive() {
            return;
        }
        self.last_status = s.clone();
        let _ = self.tx.send(Event::Status { gen: self.gen, text: s });
        self.ctx.request_repaint();
    }

    fn items(&self, items: Vec<ChatItem>) {
        if items.is_empty() || !self.alive() {
            return;
        }
        let _ = self.tx.send(Event::Items { gen: self.gen, items });
        self.ctx.request_repaint();
    }

    /// Tidur sambil tetap mengecek apakah thread ini masih dipakai.
    fn sleep(&self, d: Duration) -> bool {
        let end = Instant::now() + d;
        while Instant::now() < end {
            if !self.alive() {
                return false;
            }
            std::thread::sleep(Duration::from_millis(200));
        }
        self.alive()
    }

    fn run(&mut self, input: &str) {
        let Some(target) = parse_input(input) else {
            self.status(tr("Unrecognized input. Enter a @channel or a YouTube live link.", "Input tidak dikenali. Isi @namachannel atau link live YouTube."));
            return;
        };
        let agent = ureq::AgentBuilder::new()
            .timeout(Duration::from_secs(15))
            .user_agent(UA)
            .build();
        while self.alive() {
            if let Err(e) = self.session(&agent, &target) {
                self.status(e);
            }
            if !self.sleep(Duration::from_secs(15)) {
                return;
            }
        }
    }

    fn session(&mut self, agent: &ureq::Agent, target: &Target) -> Result<(), String> {
        let vid = match target {
            Target::Video(v) => v.clone(),
            Target::Channel(url) => find_live(agent, url)?,
        };
        let mut chat = open_chat(agent, &vid)?;
        self.status(tr("Connected to live chat", "Terhubung ke live chat"));
        let initial = std::mem::take(&mut chat.initial);
        let skip = initial.len().saturating_sub(5);
        self.items(initial.into_iter().skip(skip).collect());

        let mut fails = 0;
        loop {
            if !self.sleep(Duration::from_millis(chat.wait_ms)) {
                return Ok(());
            }
            match chat.poll(agent) {
                Ok(Some(items)) => {
                    fails = 0;
                    self.items(items);
                }
                Ok(None) => {
                    return Err(tr("Live chat has ended. Waiting for the next stream...", "Live chat sudah berakhir. Menunggu live berikutnya...").into())
                }
                Err(e) => {
                    fails += 1;
                    if fails >= 5 {
                        return Err(format!("{} ({e}). {}", tr("Connection lost", "Koneksi terputus"), tr("Retrying...", "Mencoba lagi...")));
                    }
                }
            }
        }
    }
}

struct Chat {
    cont: String,
    version: String,
    key: Option<String>,
    wait_ms: u64,
    initial: Vec<ChatItem>,
}

impl Chat {
    /// `Ok(None)` artinya live chat sudah selesai.
    fn poll(&mut self, agent: &ureq::Agent) -> Result<Option<Vec<ChatItem>>, String> {
        let url = match &self.key {
            Some(k) => format!(
                "https://www.youtube.com/youtubei/v1/live_chat/get_live_chat?key={k}&prettyPrint=false"
            ),
            None => "https://www.youtube.com/youtubei/v1/live_chat/get_live_chat?prettyPrint=false".into(),
        };
        let body = json!({
            "context": { "client": { "clientName": "WEB", "clientVersion": self.version, "hl": i18n::youtube_hl() } },
            "continuation": self.cont,
        });
        let text = agent
            .post(&url)
            .set("Content-Type", "application/json")
            .set("Cookie", COOKIE)
            .send_string(&body.to_string())
            .map_err(|e| e.to_string())?
            .into_string()
            .map_err(|e| e.to_string())?;
        let v: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
        let lcc = &v["continuationContents"]["liveChatContinuation"];
        if lcc.is_null() {
            return Ok(None);
        }
        match first_continuation(&lcc["continuations"]) {
            Some((c, t)) => {
                self.cont = c;
                self.wait_ms = t.unwrap_or(2000).clamp(1000, 5000);
            }
            None => return Ok(None),
        }
        Ok(Some(parse_actions(&lcc["actions"])))
    }
}

fn get(agent: &ureq::Agent, url: &str) -> Result<String, String> {
    agent
        .get(url)
        .set("Accept-Language", tr("en,id;q=0.8", "id,en;q=0.8"))
        .set("Cookie", COOKIE)
        .call()
        .map_err(|e| format!("{}: {e}", tr("Could not reach YouTube", "Gagal menghubungi YouTube")))?
        .into_string()
        .map_err(|e| e.to_string())
}

fn find_live(agent: &ureq::Agent, channel_url: &str) -> Result<String, String> {
    let html = get(agent, &format!("{channel_url}/live"))?;
    if let Some(c) = between(&html, "<link rel=\"canonical\" href=\"", "\"") {
        if let Some(id) = video_id_from_url(c) {
            return Ok(id);
        }
    }
    Err(tr("Channel is not live yet. Checking again every 15 seconds...", "Channel belum live. Mengecek lagi tiap 15 detik...").into())
}

fn open_chat(agent: &ureq::Agent, vid: &str) -> Result<Chat, String> {
    let html = get(agent, &format!("https://www.youtube.com/live_chat?is_popout=1&v={vid}"))?;
    let data = extract_initial_data(&html).ok_or(tr("Could not read the YouTube chat page", "Gagal membaca halaman chat YouTube"))?;
    let lcr = &data["contents"]["liveChatRenderer"];
    if lcr.is_null() {
        return Err(tr("Live chat not available yet (not live, or chat is off). Retrying...", "Live chat belum tersedia (belum live / chat dimatikan). Mencoba lagi...").into());
    }
    // subMenuItems[1] = "Live chat" (semua pesan), bukan "Top chat" yang disaring.
    let all_chat = lcr["header"]["liveChatHeaderRenderer"]["viewSelector"]["sortFilterSubMenuRenderer"]
        ["subMenuItems"][1]["continuation"]["reloadContinuationData"]["continuation"]
        .as_str()
        .map(String::from);
    let cont = all_chat
        .or_else(|| first_continuation(&lcr["continuations"]).map(|(c, _)| c))
        .ok_or(tr("Chat continuation data not found", "Data continuation chat tidak ditemukan"))?;
    Ok(Chat {
        cont,
        version: between(&html, "\"INNERTUBE_CLIENT_VERSION\":\"", "\"")
            .unwrap_or(DEFAULT_CLIENT_VERSION)
            .to_string(),
        key: between(&html, "\"INNERTUBE_API_KEY\":\"", "\"").map(String::from),
        wait_ms: 1000,
        initial: parse_actions(&lcr["actions"]),
    })
}

fn extract_initial_data(html: &str) -> Option<Value> {
    for marker in ["window[\"ytInitialData\"]", "var ytInitialData", "ytInitialData ="] {
        let Some(i) = html.find(marker) else { continue };
        let rest = &html[i..];
        let Some(j) = rest.find('{') else { continue };
        let mut it = serde_json::Deserializer::from_str(&rest[j..]).into_iter::<Value>();
        if let Some(Ok(v)) = it.next() {
            return Some(v);
        }
    }
    None
}

fn first_continuation(v: &Value) -> Option<(String, Option<u64>)> {
    let obj = v[0].as_object()?;
    obj.values().find_map(|d| {
        d["continuation"]
            .as_str()
            .map(|c| (c.to_string(), d["timeoutMs"].as_u64()))
    })
}

fn parse_actions(v: &Value) -> Vec<ChatItem> {
    let mut out = Vec::new();
    let Some(arr) = v.as_array() else { return out };
    for a in arr {
        let Some(obj) = a["addChatItemAction"]["item"].as_object() else { continue };
        for (name, r) in obj {
            let (kind, text) = match name.as_str() {
                "liveChatTextMessageRenderer" => (Kind::Text, runs(&r["message"])),
                "liveChatPaidMessageRenderer" => (Kind::Paid, runs(&r["message"])),
                "liveChatPaidStickerRenderer" => (Kind::Paid, tr("sent a Super Sticker", "mengirim Super Sticker").to_string()),
                "liveChatMembershipItemRenderer" => {
                    let mut t = runs(&r["headerSubtext"]);
                    let m = runs(&r["message"]);
                    if !m.is_empty() {
                        if !t.is_empty() {
                            t.push_str(" — ");
                        }
                        t.push_str(&m);
                    }
                    (Kind::Membership, t)
                }
                _ => continue,
            };
            out.push(ChatItem {
                id: r["id"].as_str().unwrap_or_default().to_string(),
                author: r["authorName"]["simpleText"].as_str().unwrap_or("?").to_string(),
                text,
                role: role(r),
                kind,
                amount: r["purchaseAmountText"]["simpleText"].as_str().map(String::from),
            });
        }
    }
    out
}

fn runs(v: &Value) -> String {
    if let Some(s) = v["simpleText"].as_str() {
        return s.to_string();
    }
    let mut s = String::new();
    for r in v["runs"].as_array().into_iter().flatten() {
        if let Some(t) = r["text"].as_str() {
            s.push_str(t);
        } else if let Some(e) = r.get("emoji") {
            if e["isCustomEmoji"].as_bool() == Some(true) {
                if let Some(sc) = e["shortcuts"][0].as_str() {
                    s.push_str(sc);
                }
            } else if let Some(id) = e["emojiId"].as_str() {
                s.push_str(id);
            }
        }
    }
    s
}

fn role(r: &Value) -> Role {
    let mut role = Role::Normal;
    for b in r["authorBadges"].as_array().into_iter().flatten() {
        let br = &b["liveChatAuthorBadgeRenderer"];
        match br["icon"]["iconType"].as_str() {
            Some("OWNER") => return Role::Owner,
            Some("MODERATOR") => role = Role::Moderator,
            _ => {
                if !br["customThumbnail"].is_null() && role == Role::Normal {
                    role = Role::Member;
                }
            }
        }
    }
    role
}

fn between<'a>(s: &'a str, start: &str, end: &str) -> Option<&'a str> {
    let i = s.find(start)? + start.len();
    let j = s[i..].find(end)?;
    Some(&s[i..i + j])
}

fn take_id(s: &str) -> Option<String> {
    let id: String = s
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .collect();
    (id.len() == 11).then_some(id)
}

fn video_id_from_url(s: &str) -> Option<String> {
    for pat in ["v=", "youtu.be/", "/live/", "/shorts/", "/embed/"] {
        if let Some(i) = s.find(pat) {
            if let Some(id) = take_id(&s[i + pat.len()..]) {
                return Some(id);
            }
        }
    }
    None
}

fn parse_input(s: &str) -> Option<Target> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    if let Some(id) = video_id_from_url(s) {
        return Some(Target::Video(id));
    }
    let seps = |c: char| c == '/' || c == '?' || c == '#';
    if s.starts_with('@') {
        let handle = s.split(seps).next()?;
        return Some(Target::Channel(format!("https://www.youtube.com/{handle}")));
    }
    if let Some(i) = s.find("youtube.com/") {
        let segs: Vec<&str> = s[i + 12..].split(seps).filter(|x| !x.is_empty()).collect();
        match segs.first() {
            Some(f) if f.starts_with('@') => {
                return Some(Target::Channel(format!("https://www.youtube.com/{f}")))
            }
            Some(&f) if (f == "channel" || f == "c" || f == "user") && segs.len() > 1 => {
                return Some(Target::Channel(format!("https://www.youtube.com/{f}/{}", segs[1])))
            }
            _ => {}
        }
    }
    if s.len() == 11 {
        return take_id(s).map(Target::Video);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_inputs() {
        assert!(matches!(parse_input("@LofiGirl"), Some(Target::Channel(u)) if u == "https://www.youtube.com/@LofiGirl"));
        assert!(matches!(parse_input("https://www.youtube.com/@abc/streams"), Some(Target::Channel(u)) if u.ends_with("/@abc")));
        assert!(matches!(parse_input("https://www.youtube.com/watch?v=jfKfPfyJRdk"), Some(Target::Video(v)) if v == "jfKfPfyJRdk"));
        assert!(matches!(parse_input("https://youtube.com/live/jfKfPfyJRdk?si=x"), Some(Target::Video(v)) if v == "jfKfPfyJRdk"));
        assert!(matches!(parse_input("https://youtu.be/jfKfPfyJRdk"), Some(Target::Video(_))));
        assert!(parse_input("halo").is_none());
    }

    /// Uji koneksi ke live 24/7 publik: cargo test --release -- --ignored --nocapture
    #[test]
    #[ignore]
    fn live_chat_network() {
        let agent = ureq::AgentBuilder::new().timeout(Duration::from_secs(15)).user_agent(UA).build();
        let vid = find_live(&agent, "https://www.youtube.com/@LofiGirl").expect("find_live");
        println!("video id: {vid}");
        let mut chat = open_chat(&agent, &vid).expect("open_chat");
        println!("initial: {} pesan", chat.initial.len());
        let mut total = 0;
        for _ in 0..4 {
            std::thread::sleep(Duration::from_millis(chat.wait_ms));
            let items = chat.poll(&agent).expect("poll").expect("chat ended");
            for it in &items {
                println!("[{}] {}: {}", it.id.len(), it.author, it.text);
            }
            total += items.len();
        }
        println!("total pesan baru: {total}");
        assert!(total > 0, "tidak ada pesan diterima");
    }
}
