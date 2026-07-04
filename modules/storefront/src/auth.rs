use anyhow::Result;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use myth_wire::new_id;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const JWT_SECRET: &[u8] = b"vaultforge-dev-secret-change-in-prod";
const STARTING_CREDITS: u32 = 200;

// ── Data model ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerUser {
    pub id: String,
    pub username: String,
    pub display_name: String,
    pub avatar: String,
    pub credits: u32,
    pub purchased_blueprint_ids: Vec<String>,
    pub password_hash: String,
    pub created_at: i64,
}

impl ServerUser {
    pub fn new(username: impl Into<String>, display_name: impl Into<String>, password: &str) -> Self {
        let now = chrono::Utc::now().timestamp_millis();
        ServerUser {
            id: new_id(),
            username: username.into(),
            display_name: display_name.into(),
            avatar: "⬡".to_string(),
            credits: STARTING_CREDITS,
            purchased_blueprint_ids: Vec::new(),
            password_hash: bcrypt::hash(password, bcrypt::DEFAULT_COST).unwrap_or_default(),
            created_at: now,
        }
    }

    pub fn verify_password(&self, password: &str) -> bool {
        bcrypt::verify(password, &self.password_hash).unwrap_or(false)
    }
}

// ── JWT claims ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,  // user id
    pub username: String,
    pub exp: usize,   // expiry unix timestamp
}

pub fn issue_token(user: &ServerUser) -> Result<String> {
    let exp = chrono::Utc::now()
        .checked_add_signed(chrono::Duration::days(30))
        .unwrap()
        .timestamp() as usize;
    let claims = Claims { sub: user.id.clone(), username: user.username.clone(), exp };
    let token = encode(&Header::default(), &claims, &EncodingKey::from_secret(JWT_SECRET))?;
    Ok(token)
}

pub fn verify_token(token: &str) -> Result<Claims> {
    let data = decode::<Claims>(token, &DecodingKey::from_secret(JWT_SECRET), &Validation::default())?;
    Ok(data.claims)
}

// ── In-memory user store ──────────────────────────────────────────────────────

#[derive(Debug, Default)]
pub struct UserStore {
    /// user_id → ServerUser
    pub users: HashMap<String, ServerUser>,
    /// username → user_id  (index for fast login lookup)
    username_index: HashMap<String, String>,
}

impl UserStore {
    pub fn register(&mut self, username: &str, display_name: &str, password: &str) -> Result<String, &'static str> {
        let uname_lower = username.to_lowercase();
        if self.username_index.contains_key(&uname_lower) {
            return Err("username already taken");
        }
        if username.len() < 3 {
            return Err("username must be at least 3 characters");
        }
        let user = ServerUser::new(uname_lower.clone(), display_name, password);
        let id = user.id.clone();
        self.username_index.insert(uname_lower, id.clone());
        self.users.insert(id.clone(), user);
        Ok(id)
    }

    pub fn login(&self, username: &str, password: &str) -> Option<&ServerUser> {
        let id = self.username_index.get(&username.to_lowercase())?;
        let user = self.users.get(id)?;
        if user.verify_password(password) { Some(user) } else { None }
    }

    pub fn get(&self, user_id: &str) -> Option<&ServerUser> {
        self.users.get(user_id)
    }

    pub fn get_mut(&mut self, user_id: &str) -> Option<&mut ServerUser> {
        self.users.get_mut(user_id)
    }

    pub fn spend_credits(&mut self, user_id: &str, amount: u32) -> Result<u32, &'static str> {
        let user = self.users.get_mut(user_id).ok_or("user not found")?;
        if user.credits < amount {
            return Err("insufficient credits");
        }
        user.credits -= amount;
        Ok(user.credits)
    }
}

// ── Request / response bodies ─────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct RegisterBody {
    pub username: String,
    pub display_name: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginBody {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub user_id: String,
    pub username: String,
    pub display_name: String,
    pub credits: u32,
}

#[derive(Debug, Serialize)]
pub struct MeResponse {
    pub user_id: String,
    pub username: String,
    pub display_name: String,
    pub avatar: String,
    pub credits: u32,
    pub purchased: Vec<String>,
}

impl From<&ServerUser> for MeResponse {
    fn from(u: &ServerUser) -> Self {
        MeResponse {
            user_id: u.id.clone(),
            username: u.username.clone(),
            display_name: u.display_name.clone(),
            avatar: u.avatar.clone(),
            credits: u.credits,
            purchased: u.purchased_blueprint_ids.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_and_login() {
        let mut store = UserStore::default();
        store.register("xyrona", "Xyrona Prime", "pass123").unwrap();
        assert!(store.login("xyrona", "pass123").is_some());
        assert!(store.login("xyrona", "wrongpass").is_none());
    }

    #[test]
    fn duplicate_username_rejected() {
        let mut store = UserStore::default();
        store.register("lumina", "Lumina", "pass").unwrap();
        assert!(store.register("lumina", "Lumina2", "pass").is_err());
        // Case-insensitive
        assert!(store.register("LUMINA", "Lumina3", "pass").is_err());
    }

    #[test]
    fn jwt_roundtrip() {
        let user = ServerUser::new("test", "Test User", "pw");
        let token = issue_token(&user).unwrap();
        let claims = verify_token(&token).unwrap();
        assert_eq!(claims.sub, user.id);
        assert_eq!(claims.username, "test");
    }

    #[test]
    fn spend_credits() {
        let mut store = UserStore::default();
        let uid = store.register("buyer", "Buyer", "pw").unwrap();
        assert_eq!(store.spend_credits(&uid, 50).unwrap(), 150);
        assert!(store.spend_credits(&uid, 500).is_err());
    }
}
