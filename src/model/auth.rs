use serde::Deserialize;
use std::fs;

#[derive(Debug, Deserialize, Clone)]
pub struct OpenCodeAuth {
    pub ollama: Option<ApiAuth>,
    pub google: Option<ApiAuth>,
    pub opencode: Option<ApiAuth>,
    pub huggingface: Option<ApiAuth>,
    pub modal: Option<ApiAuth>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ApiAuth {
    pub r#type: String,
    pub key: String,
}

pub fn load_opencode_auth() -> Option<OpenCodeAuth> {
    let home = home::home_dir()?;
    let auth_path = home.join(".local/share/opencode/auth.json");
    if !auth_path.exists() {
        return None;
    }

    let content = fs::read_to_string(auth_path).ok()?;
    serde_json::from_str(&content).ok()
}

pub fn get_google_token() -> Option<String> {
    let auth = load_opencode_auth()?;
    auth.google.map(|g| g.key)
}

pub fn get_opencode_key(provider_name: &str) -> Option<String> {
    let auth = load_opencode_auth()?;
    match provider_name {
        "ollama" => auth.ollama.map(|a| a.key),
        "opencode" => auth.opencode.map(|a| a.key),
        "huggingface" => auth.huggingface.map(|a| a.key),
        "modal" => auth.modal.map(|a| a.key),
        _ => None,
    }
}
