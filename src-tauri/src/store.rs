//! Local RP profile store.
//!
//! One JSON document at `<config>/daedric-companion/profile.json`. The on-disk
//! shape is a roster of character books (sheet, contacts, chronicle, rumors,
//! purse, factions, sessions). `profile()` flattens the *active* book so the
//! shell keeps reading `character` / `contacts` / `journal` / `rumors`.
//!
//! A pre-roster file (top-level `character` and no `characters` key) is wrapped
//! into a single book on load and rewritten. Dates the user types stay
//! freeform — RP runs on Tamrielic dates, not epoch.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

static ID_SEQ: AtomicU64 = AtomicU64::new(1);

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

/// One sitting. `ended == None` means the clock is still running.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaySession {
    pub id: String,
    pub started: u64,
    #[serde(default)]
    pub ended: Option<u64>,
    #[serde(default)]
    pub note: String,
}

/// Septim ledger line. Positive = in, negative = out.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoinEntry {
    pub id: String,
    pub created: u64,
    pub amount: i64,
    #[serde(default)]
    pub note: String,
    #[serde(default)]
    pub counterparty: String,
}

/// Faction attitude: -2 hostile, -1 cold, 0 neutral, +1 friendly, +2 honored.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactionStanding {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub standing: i8,
    #[serde(default)]
    pub rank: String,
    #[serde(default)]
    pub notes: String,
}

/// A job with a septim reward. `paid` flips once, the first time status
/// becomes `done` with a positive reward. Reopening does not refund.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Contract {
    pub id: String,
    #[serde(default)]
    pub created: u64,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub giver: String,
    #[serde(default)]
    pub detail: String,
    #[serde(default)]
    pub location: String,
    #[serde(default)]
    pub reward: i64,
    /// "open" | "done" | "failed"
    #[serde(default = "status_open")]
    pub status: String,
    #[serde(default)]
    pub paid: bool,
}

/// One piece of kit. `slot` is freeform ("right hand", "chest", "pack").
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KitItem {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub slot: String,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub equipped: bool,
}

/// A place worth remembering. `last_visited` is a freeform Tamrielic date.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Place {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub region: String,
    #[serde(default)]
    pub last_visited: String,
    #[serde(default)]
    pub notes: String,
}

fn status_open() -> String {
    "open".into()
}

fn normalize_status(status: &str) -> String {
    match status.trim().to_ascii_lowercase().as_str() {
        "done" => "done".into(),
        "failed" => "failed".into(),
        _ => "open".into(),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CharacterBook {
    #[serde(default)]
    id: String,
    #[serde(default)]
    character: Character,
    #[serde(default)]
    contacts: Vec<Contact>,
    #[serde(default)]
    journal: Vec<JournalEntry>,
    #[serde(default)]
    rumors: Vec<Rumor>,
    #[serde(default)]
    purse: Vec<CoinEntry>,
    #[serde(default)]
    factions: Vec<FactionStanding>,
    #[serde(default)]
    sessions: Vec<PlaySession>,
    #[serde(default)]
    contracts: Vec<Contract>,
    #[serde(default)]
    kit: Vec<KitItem>,
    #[serde(default)]
    places: Vec<Place>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Document {
    #[serde(default)]
    active_id: String,
    #[serde(default)]
    characters: Vec<CharacterBook>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RosterEntry {
    pub id: String,
    pub name: String,
}

/// View of the active book, plus the roster for the switcher.
/// Flattened keys match the pre-roster frontend.
#[derive(Debug, Clone, Serialize)]
pub struct Profile {
    pub active_id: String,
    pub roster: Vec<RosterEntry>,
    pub character: Character,
    pub contacts: Vec<Contact>,
    pub journal: Vec<JournalEntry>,
    pub rumors: Vec<Rumor>,
    pub purse: Vec<CoinEntry>,
    pub factions: Vec<FactionStanding>,
    pub sessions: Vec<PlaySession>,
    pub contracts: Vec<Contract>,
    pub kit: Vec<KitItem>,
    pub places: Vec<Place>,
    pub purse_balance: i64,
    pub time_on_field_ms: u64,
}

pub struct Store {
    path: PathBuf,
    doc: Mutex<Document>,
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn new_id() -> String {
    let n = ID_SEQ.fetch_add(1, Ordering::Relaxed);
    format!("{:x}{:x}", now_ms(), n)
}

fn display_name(name: &str) -> String {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        "Unnamed".to_string()
    } else {
        trimmed.to_string()
    }
}

impl CharacterBook {
    fn blank() -> Self {
        CharacterBook {
            id: new_id(),
            character: Character::default(),
            contacts: Vec::new(),
            journal: Vec::new(),
            rumors: Vec::new(),
            purse: Vec::new(),
            factions: Vec::new(),
            sessions: Vec::new(),
            contracts: Vec::new(),
            kit: Vec::new(),
            places: Vec::new(),
        }
    }
}

impl Default for Document {
    fn default() -> Self {
        let book = CharacterBook::blank();
        Document {
            active_id: book.id.clone(),
            characters: vec![book],
        }
    }
}

fn normalize(doc: &mut Document) {
    if doc.characters.is_empty() {
        let book = CharacterBook::blank();
        doc.active_id = book.id.clone();
        doc.characters.push(book);
        return;
    }
    for book in &mut doc.characters {
        if book.id.is_empty() {
            book.id = new_id();
        }
    }
    if !doc.characters.iter().any(|c| c.id == doc.active_id) {
        doc.active_id = doc.characters[0].id.clone();
    }
}

fn legacy_wrap(obj: &serde_json::Map<String, Value>) -> Document {
    let character = obj
        .get("character")
        .cloned()
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_default();
    let contacts = obj
        .get("contacts")
        .cloned()
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_default();
    let journal = obj
        .get("journal")
        .cloned()
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_default();
    let rumors = obj
        .get("rumors")
        .cloned()
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_default();
    let book = CharacterBook {
        id: new_id(),
        character,
        contacts,
        journal,
        rumors,
        purse: Vec::new(),
        factions: Vec::new(),
        sessions: Vec::new(),
        contracts: Vec::new(),
        kit: Vec::new(),
        places: Vec::new(),
    };
    Document {
        active_id: book.id.clone(),
        characters: vec![book],
    }
}

/// `Some` when the value is an object we understand. `None` leaves a corrupt
/// file on disk untouched.
fn migrate(value: Value) -> Option<Document> {
    let obj = value.as_object()?.clone();
    if obj.contains_key("characters") {
        let mut doc: Document = serde_json::from_value(Value::Object(obj)).ok()?;
        normalize(&mut doc);
        return Some(doc);
    }
    let mut doc = legacy_wrap(&obj);
    normalize(&mut doc);
    Some(doc)
}

fn active_mut(doc: &mut Document) -> &mut CharacterBook {
    normalize(doc);
    let id = doc.active_id.clone();
    doc.characters.iter_mut().find(|c| c.id == id).unwrap()
}

fn field_ms(sessions: &[PlaySession], now: u64) -> u64 {
    sessions
        .iter()
        .map(|s| s.ended.unwrap_or(now).saturating_sub(s.started))
        .sum()
}

impl Store {
    pub fn load(config_dir: PathBuf) -> Self {
        let dir = config_dir.join("daedric-companion");
        let _ = fs::create_dir_all(&dir);
        let path = dir.join("profile.json");
        let raw = fs::read_to_string(&path).ok();
        let (doc, rewrite) = match &raw {
            None => (Document::default(), false),
            Some(text) => match serde_json::from_str::<Value>(text) {
                Err(_) => (Document::default(), false),
                Ok(value) => {
                    let legacy = value
                        .as_object()
                        .map(|o| !o.contains_key("characters"))
                        .unwrap_or(true);
                    match migrate(value) {
                        Some(doc) => (doc, legacy),
                        None => (Document::default(), false),
                    }
                }
            },
        };
        let store = Store {
            path,
            doc: Mutex::new(doc),
        };
        if rewrite {
            if let Ok(doc) = store.doc.lock() {
                let _ = store.save_doc(&doc);
            }
        }
        store
    }

    fn save_doc(&self, doc: &Document) -> Result<(), String> {
        let tmp = self.path.with_extension("json.tmp");
        let text = serde_json::to_string_pretty(doc).map_err(|e| e.to_string())?;
        fs::write(&tmp, text).map_err(|e| e.to_string())?;
        fs::rename(&tmp, &self.path).map_err(|e| e.to_string())
    }

    pub fn profile(&self) -> Profile {
        let doc = self.doc.lock().unwrap();
        let mut doc = doc.clone();
        normalize(&mut doc);
        let book = doc
            .characters
            .iter()
            .find(|c| c.id == doc.active_id)
            .unwrap();
        let now = now_ms();
        let mut purse = book.purse.clone();
        purse.sort_by(|a, b| b.created.cmp(&a.created));
        let mut sessions = book.sessions.clone();
        sessions.sort_by(|a, b| b.started.cmp(&a.started));
        let mut factions = book.factions.clone();
        factions.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        Profile {
            active_id: doc.active_id.clone(),
            roster: doc
                .characters
                .iter()
                .map(|c| RosterEntry {
                    id: c.id.clone(),
                    name: display_name(&c.character.name),
                })
                .collect(),
            character: book.character.clone(),
            contacts: book.contacts.clone(),
            journal: book.journal.clone(),
            rumors: book.rumors.clone(),
            purse_balance: book.purse.iter().map(|e| e.amount).sum(),
            purse,
            factions,
            time_on_field_ms: field_ms(&book.sessions, now),
            sessions,
            contracts: {
                let mut v = book.contracts.clone();
                v.sort_by(|a, b| b.created.cmp(&a.created));
                v
            },
            kit: {
                let mut v = book.kit.clone();
                v.sort_by(|a, b| {
                    b.equipped
                        .cmp(&a.equipped)
                        .then(a.name.to_lowercase().cmp(&b.name.to_lowercase()))
                });
                v
            },
            places: {
                let mut v = book.places.clone();
                v.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
                v
            },
        }
    }

    pub fn save_character(&self, character: Character) -> Result<(), String> {
        let mut doc = self.doc.lock().unwrap();
        active_mut(&mut doc).character = character;
        self.save_doc(&doc)
    }

    pub fn create_character(&self, name: String) -> Result<Profile, String> {
        let mut doc = self.doc.lock().unwrap();
        let mut book = CharacterBook::blank();
        book.character.name = name;
        doc.active_id = book.id.clone();
        doc.characters.push(book);
        self.save_doc(&doc)?;
        drop(doc);
        Ok(self.profile())
    }

    pub fn switch_character(&self, id: &str) -> Result<Profile, String> {
        let mut doc = self.doc.lock().unwrap();
        if !doc.characters.iter().any(|c| c.id == id) {
            return Err("no such character".into());
        }
        doc.active_id = id.to_string();
        self.save_doc(&doc)?;
        drop(doc);
        Ok(self.profile())
    }

    pub fn delete_character(&self, id: &str) -> Result<Profile, String> {
        let mut doc = self.doc.lock().unwrap();
        if doc.characters.len() <= 1 {
            return Err("the last name stays on the roll".into());
        }
        let before = doc.characters.len();
        doc.characters.retain(|c| c.id != id);
        if doc.characters.len() == before {
            return Err("no such character".into());
        }
        if !doc.characters.iter().any(|c| c.id == doc.active_id) {
            doc.active_id = doc.characters[0].id.clone();
        }
        self.save_doc(&doc)?;
        drop(doc);
        Ok(self.profile())
    }

    pub fn save_contact(&self, mut contact: Contact) -> Result<Contact, String> {
        let mut doc = self.doc.lock().unwrap();
        if contact.id.is_empty() {
            contact.id = new_id();
        }
        let book = active_mut(&mut doc);
        if let Some(existing) = book.contacts.iter_mut().find(|c| c.id == contact.id) {
            *existing = contact.clone();
        } else {
            book.contacts.push(contact.clone());
        }
        self.save_doc(&doc)?;
        Ok(contact)
    }

    pub fn delete_contact(&self, id: &str) -> Result<(), String> {
        let mut doc = self.doc.lock().unwrap();
        active_mut(&mut doc).contacts.retain(|c| c.id != id);
        self.save_doc(&doc)
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
        let mut doc = self.doc.lock().unwrap();
        let book = active_mut(&mut doc);
        book.journal.push(entry.clone());
        book.journal.sort_by(|a, b| b.created.cmp(&a.created));
        self.save_doc(&doc)?;
        Ok(entry)
    }

    pub fn delete_journal(&self, id: &str) -> Result<(), String> {
        let mut doc = self.doc.lock().unwrap();
        active_mut(&mut doc).journal.retain(|e| e.id != id);
        self.save_doc(&doc)
    }

    pub fn add_rumor(&self, text: String, source: String) -> Result<Rumor, String> {
        let rumor = Rumor {
            id: new_id(),
            created: now_ms(),
            text,
            source,
            done: false,
        };
        let mut doc = self.doc.lock().unwrap();
        active_mut(&mut doc).rumors.push(rumor.clone());
        self.save_doc(&doc)?;
        Ok(rumor)
    }

    pub fn toggle_rumor(&self, id: &str) -> Result<(), String> {
        let mut doc = self.doc.lock().unwrap();
        if let Some(r) = active_mut(&mut doc).rumors.iter_mut().find(|r| r.id == id) {
            r.done = !r.done;
        }
        self.save_doc(&doc)
    }

    pub fn delete_rumor(&self, id: &str) -> Result<(), String> {
        let mut doc = self.doc.lock().unwrap();
        active_mut(&mut doc).rumors.retain(|r| r.id != id);
        self.save_doc(&doc)
    }

    pub fn start_session(&self) -> Result<PlaySession, String> {
        let mut doc = self.doc.lock().unwrap();
        let book = active_mut(&mut doc);
        if book.sessions.iter().any(|s| s.ended.is_none()) {
            return Err("a session is already on the field".into());
        }
        let session = PlaySession {
            id: new_id(),
            started: now_ms(),
            ended: None,
            note: String::new(),
        };
        book.sessions.push(session.clone());
        self.save_doc(&doc)?;
        Ok(session)
    }

    pub fn stop_session(&self, note: String) -> Result<PlaySession, String> {
        let mut doc = self.doc.lock().unwrap();
        let book = active_mut(&mut doc);
        let session = book
            .sessions
            .iter_mut()
            .find(|s| s.ended.is_none())
            .ok_or_else(|| "no session on the field".to_string())?;
        let ended = now_ms().max(session.started);
        session.ended = Some(ended);
        if !note.trim().is_empty() {
            session.note = note.trim().to_string();
        }
        let stored = session.clone();
        self.save_doc(&doc)?;
        Ok(stored)
    }

    pub fn delete_session(&self, id: &str) -> Result<(), String> {
        let mut doc = self.doc.lock().unwrap();
        active_mut(&mut doc).sessions.retain(|s| s.id != id);
        self.save_doc(&doc)
    }

    pub fn add_coin(
        &self,
        amount: i64,
        note: String,
        counterparty: String,
    ) -> Result<CoinEntry, String> {
        if amount == 0 {
            return Err("amount has to be at least 1".into());
        }
        let entry = CoinEntry {
            id: new_id(),
            created: now_ms(),
            amount,
            note,
            counterparty,
        };
        let mut doc = self.doc.lock().unwrap();
        active_mut(&mut doc).purse.push(entry.clone());
        self.save_doc(&doc)?;
        Ok(entry)
    }

    pub fn delete_coin(&self, id: &str) -> Result<(), String> {
        let mut doc = self.doc.lock().unwrap();
        active_mut(&mut doc).purse.retain(|e| e.id != id);
        self.save_doc(&doc)
    }

    pub fn save_faction(&self, mut faction: FactionStanding) -> Result<FactionStanding, String> {
        faction.standing = faction.standing.clamp(-2, 2);
        let mut doc = self.doc.lock().unwrap();
        if faction.id.is_empty() {
            faction.id = new_id();
        }
        let book = active_mut(&mut doc);
        if let Some(existing) = book.factions.iter_mut().find(|f| f.id == faction.id) {
            *existing = faction.clone();
        } else {
            book.factions.push(faction.clone());
        }
        self.save_doc(&doc)?;
        Ok(faction)
    }

    pub fn delete_faction(&self, id: &str) -> Result<(), String> {
        let mut doc = self.doc.lock().unwrap();
        active_mut(&mut doc).factions.retain(|f| f.id != id);
        self.save_doc(&doc)
    }

    fn write_contract(book: &mut CharacterBook, mut contract: Contract) -> Contract {
        if contract.id.is_empty() {
            contract.id = new_id();
        }
        if contract.created == 0 {
            contract.created = now_ms();
        }
        contract.status = normalize_status(&contract.status);
        if contract.reward < 0 {
            contract.reward = 0;
        }
        let prior = book.contracts.iter().find(|c| c.id == contract.id);
        if let Some(prior) = prior {
            contract.paid = prior.paid;
            if contract.created == 0 {
                contract.created = prior.created;
            }
        }
        if contract.status == "done" && !contract.paid && contract.reward > 0 {
            let note = if contract.title.trim().is_empty() {
                "contract".to_string()
            } else {
                contract.title.trim().to_string()
            };
            book.purse.push(CoinEntry {
                id: new_id(),
                created: now_ms(),
                amount: contract.reward,
                note,
                counterparty: contract.giver.trim().to_string(),
            });
            contract.paid = true;
        }
        if let Some(slot) = book.contracts.iter_mut().find(|c| c.id == contract.id) {
            contract.created = slot.created;
            *slot = contract.clone();
        } else {
            book.contracts.push(contract.clone());
        }
        contract
    }

    pub fn save_contract(&self, contract: Contract) -> Result<Contract, String> {
        if contract.title.trim().is_empty() {
            return Err("a contract needs a title".into());
        }
        let mut doc = self.doc.lock().unwrap();
        let stored = Self::write_contract(active_mut(&mut doc), contract);
        self.save_doc(&doc)?;
        Ok(stored)
    }

    pub fn set_contract_status(&self, id: &str, status: String) -> Result<Contract, String> {
        let mut doc = self.doc.lock().unwrap();
        let book = active_mut(&mut doc);
        let mut contract = book
            .contracts
            .iter()
            .find(|c| c.id == id)
            .cloned()
            .ok_or_else(|| "no such contract".to_string())?;
        contract.status = status;
        let stored = Self::write_contract(book, contract);
        self.save_doc(&doc)?;
        Ok(stored)
    }

    pub fn delete_contract(&self, id: &str) -> Result<(), String> {
        let mut doc = self.doc.lock().unwrap();
        active_mut(&mut doc).contracts.retain(|c| c.id != id);
        self.save_doc(&doc)
    }

    pub fn save_kit(&self, mut item: KitItem) -> Result<KitItem, String> {
        if item.name.trim().is_empty() {
            return Err("a kit piece needs a name".into());
        }
        let mut doc = self.doc.lock().unwrap();
        if item.id.is_empty() {
            item.id = new_id();
        }
        let book = active_mut(&mut doc);
        if let Some(slot) = book.kit.iter_mut().find(|k| k.id == item.id) {
            *slot = item.clone();
        } else {
            book.kit.push(item.clone());
        }
        self.save_doc(&doc)?;
        Ok(item)
    }

    pub fn delete_kit(&self, id: &str) -> Result<(), String> {
        let mut doc = self.doc.lock().unwrap();
        active_mut(&mut doc).kit.retain(|k| k.id != id);
        self.save_doc(&doc)
    }

    pub fn save_place(&self, mut place: Place) -> Result<Place, String> {
        if place.name.trim().is_empty() {
            return Err("a place needs a name".into());
        }
        let mut doc = self.doc.lock().unwrap();
        if place.id.is_empty() {
            place.id = new_id();
        }
        let book = active_mut(&mut doc);
        if let Some(slot) = book.places.iter_mut().find(|p| p.id == place.id) {
            *slot = place.clone();
        } else {
            book.places.push(place.clone());
        }
        self.save_doc(&doc)?;
        Ok(place)
    }

    pub fn delete_place(&self, id: &str) -> Result<(), String> {
        let mut doc = self.doc.lock().unwrap();
        active_mut(&mut doc).places.retain(|p| p.id != id);
        self.save_doc(&doc)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_store(tag: &str) -> Store {
        let dir = std::env::temp_dir().join(format!(
            "daedric-store-{tag}-{}-{}",
            std::process::id(),
            new_id()
        ));
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

    #[test]
    fn legacy_file_migrates_without_dropping_the_sheet() {
        let dir = std::env::temp_dir().join(format!("daedric-store-legacy-{}", new_id()));
        let nested = dir.join("daedric-companion");
        fs::create_dir_all(&nested).unwrap();
        fs::write(
            nested.join("profile.json"),
            r#"{"character":{"name":"Svana Far-Shield","race":"Nord"},"contacts":[{"id":"c1","name":"Balimund","alive":true}],"journal":[],"rumors":[]}"#,
        )
        .unwrap();
        let s = Store::load(dir.clone());
        assert_eq!(s.profile().character.name, "Svana Far-Shield");
        assert_eq!(s.profile().contacts.len(), 1);
        assert_eq!(s.profile().roster.len(), 1);
        let on_disk = fs::read_to_string(nested.join("profile.json")).unwrap();
        assert!(on_disk.contains("\"characters\""));
        let s2 = Store::load(dir);
        assert_eq!(s2.profile().character.race, "Nord");
        assert_eq!(s2.profile().contacts[0].name, "Balimund");
    }

    #[test]
    fn switch_isolates_purse_and_journal() {
        let s = temp_store("switch");
        s.save_character(Character {
            name: "Svana".into(),
            ..Default::default()
        })
        .unwrap();
        s.add_coin(250, "bounty".into(), "jarl".into()).unwrap();
        s.add_journal("the road".into(), "Whiterun".into(), String::new())
            .unwrap();
        let id_a = s.profile().active_id.clone();
        s.create_character("Brynjolf".into()).unwrap();
        let alt = s.profile();
        assert_eq!(alt.character.name, "Brynjolf");
        assert_eq!(alt.purse_balance, 0);
        assert!(alt.journal.is_empty());
        s.switch_character(&id_a).unwrap();
        let back = s.profile();
        assert_eq!(back.character.name, "Svana");
        assert_eq!(back.purse_balance, 250);
        assert_eq!(back.journal.len(), 1);
    }

    #[test]
    fn last_character_cannot_be_retired() {
        let s = temp_store("last");
        let id = s.profile().active_id.clone();
        let err = s.delete_character(&id).unwrap_err();
        assert!(err.contains("last name"));
        s.create_character("Alt".into()).unwrap();
        s.delete_character(&s.profile().active_id).unwrap();
        assert_eq!(s.profile().roster.len(), 1);
    }

    #[test]
    fn session_start_stop_and_reject_double_start() {
        let s = temp_store("session");
        let open = s.start_session().unwrap();
        assert!(open.ended.is_none());
        assert!(s.start_session().is_err());
        let closed = s.stop_session("cleared a fort".into()).unwrap();
        assert!(closed.ended.unwrap() >= closed.started);
        assert_eq!(closed.note, "cleared a fort");
        assert!(s.profile().time_on_field_ms >= closed.ended.unwrap() - closed.started);
        s.delete_session(&closed.id).unwrap();
        assert!(s.profile().sessions.is_empty());
    }

    #[test]
    fn purse_rejects_zero_and_sums() {
        let s = temp_store("purse");
        assert!(s.add_coin(0, String::new(), String::new()).is_err());
        s.add_coin(100, "loot".into(), String::new()).unwrap();
        s.add_coin(-40, "inn".into(), "barkeep".into()).unwrap();
        assert_eq!(s.profile().purse_balance, 60);
        let id = s.profile().purse[0].id.clone();
        s.delete_coin(&id).unwrap();
        assert_eq!(s.profile().purse.len(), 1);
    }

    #[test]
    fn faction_upsert_clamps_standing() {
        let s = temp_store("faction");
        let f = s
            .save_faction(FactionStanding {
                id: String::new(),
                name: "Companions".into(),
                standing: 9,
                rank: "shield-sibling".into(),
                notes: String::new(),
            })
            .unwrap();
        assert_eq!(s.profile().factions[0].standing, 2);
        let mut edited = f.clone();
        edited.rank = "harbinger".into();
        s.save_faction(edited).unwrap();
        assert_eq!(s.profile().factions.len(), 1);
        assert_eq!(s.profile().factions[0].rank, "harbinger");
        s.delete_faction(&f.id).unwrap();
        assert!(s.profile().factions.is_empty());
    }

    #[test]
    fn contract_pays_the_purse_once() {
        let s = temp_store("contract");
        let c = s
            .save_contract(Contract {
                id: String::new(),
                title: "clear the fort".into(),
                giver: "jarl".into(),
                reward: 100,
                status: "open".into(),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(s.profile().purse_balance, 0);
        s.set_contract_status(&c.id, "done".into()).unwrap();
        assert_eq!(s.profile().purse_balance, 100);
        assert!(s.profile().contracts[0].paid);
        s.set_contract_status(&c.id, "done".into()).unwrap();
        assert_eq!(s.profile().purse_balance, 100);
        s.set_contract_status(&c.id, "open".into()).unwrap();
        assert_eq!(s.profile().purse_balance, 100);
        assert!(s.profile().contracts[0].paid);
    }

    #[test]
    fn kit_and_place_follow_the_active_character() {
        let s = temp_store("kitplace");
        s.save_character(Character {
            name: "Svana".into(),
            ..Default::default()
        })
        .unwrap();
        s.save_kit(KitItem {
            id: String::new(),
            name: "steel sword".into(),
            slot: "right hand".into(),
            equipped: true,
            notes: String::new(),
        })
        .unwrap();
        s.save_place(Place {
            id: String::new(),
            name: "Whiterun".into(),
            region: "Whiterun Hold".into(),
            last_visited: "4E 201, 15th of Last Seed".into(),
            notes: String::new(),
        })
        .unwrap();
        let id_a = s.profile().active_id.clone();
        s.create_character("Alt".into()).unwrap();
        assert!(s.profile().kit.is_empty());
        assert!(s.profile().places.is_empty());
        s.switch_character(&id_a).unwrap();
        assert_eq!(s.profile().kit[0].name, "steel sword");
        assert_eq!(s.profile().places[0].name, "Whiterun");
    }
}

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

impl Default for FactionStanding {
    fn default() -> Self {
        FactionStanding {
            id: String::new(),
            name: String::new(),
            standing: 0,
            rank: String::new(),
            notes: String::new(),
        }
    }
}
