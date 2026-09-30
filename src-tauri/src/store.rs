//! Local RP profile store: character sheet, contacts, chronicle, rumors.
//!
//! One JSON document at `<config>/daedric-companion/profile.json`, cached in a
//! Mutex, written whole on every mutation (tmp + rename so a crash mid-write
//! never leaves a torn file). All dates the user types are freeform — RP
//! servers run on Tamrielic dates ("4E 201, 15th of Last Seed"), not epoch.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Skill {
    pub name: String,
    #[serde(default)]
    pub level: u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Character {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub race: String,
    #[serde(default)]
    pub sex: String,
    #[serde(default)]
    pub birthsign: String,
    #[serde(default)]
    pub deity: String,
    #[serde(default)]
    pub faction: String,
    #[serde(default)]
    pub rank: String,
    #[serde(default)]
    pub occupation: String,
    #[serde(default)]
    pub level: Option<u32>,
    #[serde(default)]
    pub health: Option<u32>,
    #[serde(default)]
    pub magicka: Option<u32>,
    #[serde(default)]
    pub stamina: Option<u32>,
    #[serde(default)]
    pub skills: Vec<Skill>,
    #[serde(default)]
    pub appearance: String,
    #[serde(default)]
    pub personality: String,
    #[serde(default)]
    pub backstory: String,
    #[serde(default)]
    pub notes: String,
    /// Freeform in-game date, e.g. "4E 201, 15th of Last Seed".
    #[serde(default)]
    pub ingame_date: String,
}

/// Relationship ladder: -2 hated, -1 cold, 0 stranger, +1 warm, +2 sworn.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contact {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub race: String,
    #[serde(default)]
    pub faction: String,
    #[serde(default)]
    pub role: String,
    /// Where we met (freeform).
    #[serde(default)]
    pub met_at: String,
    #[serde(default)]
    pub first_met: String,
    #[serde(default)]
    pub last_seen: String,
    #[serde(default)]
    pub relationship: i8,
    #[serde(default)]
    pub alive: bool,
    #[serde(default)]
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JournalEntry {
    pub id: String,
    /// Epoch millis — real time, for ordering and display.
    pub created: u64,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub location: String,
    #[serde(default)]
    pub body: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rumor {
    pub id: String,
    pub created: u64,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub done: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Profile {
    #[serde(default)]
    pub character: Character,
    #[serde(default)]
    pub contacts: Vec<Contact>,
    #[serde(default)]
    pub journal: Vec<JournalEntry>,
    #[serde(default)]
    pub rumors: Vec<Rumor>,
}

pub struct Store {
    path: PathBuf,
    profile: Mutex<Profile>,
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Timestamp ids — unique enough for a single-user local store.
fn new_id() -> String {
    format!("{:x}", now_ms())
}

impl Store {
    pub fn load(config_dir: PathBuf) -> Self {
        let dir = config_dir.join("daedric-companion");
        let _ = fs::create_dir_all(&dir);
        let path = dir.join("profile.json");
        let profile = fs::read_to_string(&path)
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default();
        Store {
            path,
            profile: Mutex::new(profile),
        }
    }

    fn save(&self, profile: &Profile) -> Result<(), String> {
        let tmp = self.path.with_extension("json.tmp");
        let text = serde_json::to_string_pretty(profile).map_err(|e| e.to_string())?;
        fs::write(&tmp, text).map_err(|e| e.to_string())?;
        fs::rename(&tmp, &self.path).map_err(|e| e.to_string())
    }

    pub fn profile(&self) -> Profile {
        self.profile.lock().unwrap().clone()
    }

    pub fn save_character(&self, character: Character) -> Result<(), String> {
        let mut p = self.profile.lock().unwrap();
        p.character = character;
        self.save(&p)
    }

    /// Upsert a contact; empty id mints a new one. Returns the stored record.
    pub fn save_contact(&self, mut contact: Contact) -> Result<Contact, String> {
        let mut p = self.profile.lock().unwrap();
        if contact.id.is_empty() {
            contact.id = new_id();
        }
        if let Some(existing) = p.contacts.iter_mut().find(|c| c.id == contact.id) {
            *existing = contact.clone();
        } else {
            p.contacts.push(contact.clone());
        }
        self.save(&p)?;
        Ok(contact)
    }

    pub fn delete_contact(&self, id: &str) -> Result<(), String> {
        let mut p = self.profile.lock().unwrap();
        p.contacts.retain(|c| c.id != id);
        self.save(&p)
    }

    pub fn add_journal(
        &self,
        title: String,
        location: String,
        body: String,
    ) -> Result<JournalEntry, String> {
        let entry = JournalEntry {
            id: new_id(),
            created: now_ms(),
            title,
            location,
            body,
        };
        let mut p = self.profile.lock().unwrap();
        p.journal.push(entry.clone());
        // newest first
        p.journal.sort_by(|a, b| b.created.cmp(&a.created));
        self.save(&p)?;
        Ok(entry)
    }

    pub fn delete_journal(&self, id: &str) -> Result<(), String> {
        let mut p = self.profile.lock().unwrap();
        p.journal.retain(|e| e.id != id);
        self.save(&p)
    }

    pub fn add_rumor(&self, text: String, source: String) -> Result<Rumor, String> {
        let rumor = Rumor {
            id: new_id(),
            created: now_ms(),
            text,
            source,
            done: false,
        };
        let mut p = self.profile.lock().unwrap();
        p.rumors.push(rumor.clone());
        self.save(&p)?;
        Ok(rumor)
    }

    pub fn toggle_rumor(&self, id: &str) -> Result<(), String> {
        let mut p = self.profile.lock().unwrap();
        if let Some(r) = p.rumors.iter_mut().find(|r| r.id == id) {
            r.done = !r.done;
        }
        self.save(&p)
    }

    pub fn delete_rumor(&self, id: &str) -> Result<(), String> {
        let mut p = self.profile.lock().unwrap();
        p.rumors.retain(|r| r.id != id);
        self.save(&p)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_store(tag: &str) -> Store {
        let dir = std::env::temp_dir().join(format!("daedric-store-{tag}-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        Store::load(dir)
    }

    #[test]
    fn character_roundtrip() {
        let s = temp_store("char");
        let c = Character {
            name: "Svana Far-Shield".to_string(),
            race: "Nord".to_string(),
            level: Some(12),
            ..Default::default()
        };
        s.save_character(c).unwrap();
        let loaded = s.profile();
        assert_eq!(loaded.character.name, "Svana Far-Shield");
        assert_eq!(loaded.character.level, Some(12));
        // file is real and reloadable (Store::load appends daedric-companion/ itself)
        let config_dir = s.path.parent().unwrap().parent().unwrap().to_path_buf();
        let s2 = Store::load(config_dir);
        assert_eq!(s2.profile().character.race, "Nord");
    }

    #[test]
    fn contact_upsert_and_delete() {
        let s = temp_store("contact");
        let c = s
            .save_contact(Contact {
                id: String::new(),
                name: "Balimund".to_string(),
                relationship: 1,
                alive: true,
                ..Default::default()
            })
            .unwrap();
        assert!(!c.id.is_empty());
        let mut c2 = c.clone();
        c2.relationship = 2;
        s.save_contact(c2).unwrap();
        assert_eq!(s.profile().contacts.len(), 1);
        assert_eq!(s.profile().contacts[0].relationship, 2);
        s.delete_contact(&c.id).unwrap();
        assert!(s.profile().contacts.is_empty());
    }

    #[test]
    fn journal_newest_first() {
        let s = temp_store("journal");
        s.add_journal("first".into(), String::new(), String::new())
            .unwrap();
        std::thread::sleep(std::time::Duration::from_millis(2));
        s.add_journal("second".into(), String::new(), String::new())
            .unwrap();
        let p = s.profile();
        assert_eq!(p.journal[0].title, "second");
        assert_eq!(p.journal[1].title, "first");
    }

    #[test]
    fn rumor_toggle() {
        let s = temp_store("rumor");
        let r = s
            .add_rumor("bandits on the north road".into(), "barkeep".into())
            .unwrap();
        s.toggle_rumor(&r.id).unwrap();
        assert!(s.profile().rumors[0].done);
        s.delete_rumor(&r.id).unwrap();
        assert!(s.profile().rumors.is_empty());
    }
}

// Contacts/journal/rumors derive Default via field defaults + manual impls.
impl Default for Contact {
    fn default() -> Self {
        Contact {
            id: String::new(),
            name: String::new(),
            race: String::new(),
            faction: String::new(),
            role: String::new(),
            met_at: String::new(),
            first_met: String::new(),
            last_seen: String::new(),
            relationship: 0,
            alive: true,
            notes: String::new(),
        }
    }
}

impl Default for JournalEntry {
    fn default() -> Self {
        JournalEntry {
            id: String::new(),
            created: 0,
            title: String::new(),
            location: String::new(),
            body: String::new(),
        }
    }
}

impl Default for Rumor {
    fn default() -> Self {
        Rumor {
            id: String::new(),
            created: 0,
            text: String::new(),
            source: String::new(),
            done: false,
        }
    }
}
