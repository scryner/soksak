use std::sync::Arc;

use anyhow::Context as _;
use gpui::prelude::*;
use gpui::*;
use notify::{RecursiveMode, Watcher};
use soksak_lib::config::{RunConfig, load_run_config};

#[derive(Clone)]
pub struct ProfileManager {
    profiles: Vec<String>,
    current_profile: Option<String>,
    config_cache: std::collections::HashMap<String, RunConfig>,
    _watcher: Arc<std::sync::Mutex<Option<notify::RecommendedWatcher>>>,
}

pub enum ProfileEvent {
    ProfilesChanged,
}

impl EventEmitter<ProfileEvent> for ProfileManager {}

impl ProfileManager {
    pub fn new(cx: &mut App) -> Entity<Self> {
        let (tx, rx) = async_channel::unbounded();

        let mut watcher = notify::recommended_watcher(move |res| {
            let _ = tx.try_send(res);
        })
        .ok();

        if let Some(watcher) = watcher.as_mut() {
            if let Some(home) = dirs::home_dir() {
                let profiles_dir = home.join(".soksak/profiles");
                if profiles_dir.exists() {
                    let _ = watcher.watch(&profiles_dir, RecursiveMode::NonRecursive);
                }
            }
        }

        let watcher_store = Arc::new(std::sync::Mutex::new(watcher));

        let model = cx.new(|cx| {
            let mut manager = Self {
                profiles: vec!["Default".to_string()],
                current_profile: Some("Default".to_string()),
                config_cache: std::collections::HashMap::new(),
                _watcher: watcher_store,
            };
            manager.reload_profiles(cx);
            manager
        });

        let weak_model = model.downgrade();

        cx.spawn(|cx_ref: &mut AsyncApp| {
            let cx = cx_ref.clone();
            async move {
                while let Ok(res) = rx.recv().await {
                    match res {
                        Ok(_) => {
                            cx.update(|cx| {
                                if let Some(model) = weak_model.upgrade() {
                                    model.update(cx, |manager, cx| {
                                        manager.reload_profiles(cx);
                                    });
                                }
                            })
                            .ok();
                        }
                        Err(e) => eprintln!("watch error: {:?}", e),
                    }
                }
            }
        })
        .detach();

        model
    }

    pub fn reload_profiles(&mut self, cx: &mut Context<Self>) {
        let mut profiles = vec!["Default".to_string()];
        let mut cache = std::collections::HashMap::new();

        if let Ok(home) = dirs::home_dir().context("Could not find home directory") {
            let profiles_dir = home.join(".soksak/profiles");
            if profiles_dir.exists() {
                if let Ok(entries) = std::fs::read_dir(profiles_dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if let Some(ext) = path.extension() {
                            if ext == "yaml" || ext == "yml" {
                                if let Some(stem) = path.file_stem() {
                                    let name = stem.to_string_lossy().to_string();
                                    println!("filepath: {}", path.to_string_lossy());

                                    // Try to parse to verify it's a valid RunConfig
                                    match load_run_config(&path) {
                                        Ok(config) => {
                                            profiles.push(name.clone());
                                            cache.insert(name, config);
                                        }
                                        Err(e) => {
                                            eprintln!("Failed to parse profile: {}", e);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        self.profiles = profiles;
        self.config_cache = cache;

        // Ensure current profile still exists, else fallback to Default
        if let Some(current) = &self.current_profile {
            if !self.profiles.contains(current) {
                self.current_profile = Some("Default".to_string());
            }
        } else {
            self.current_profile = Some("Default".to_string());
        }

        cx.notify();
        cx.emit(ProfileEvent::ProfilesChanged);
    }

    pub fn get_profiles(&self) -> &Vec<String> {
        &self.profiles
    }

    pub fn get_current_profile(&self) -> Option<&String> {
        self.current_profile.as_ref()
    }

    pub fn select_profile(&mut self, name: String, cx: &mut Context<Self>) {
        if self.profiles.contains(&name) {
            self.current_profile = Some(name);
            cx.notify();
        }
    }
}

impl Render for ProfileManager {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}
