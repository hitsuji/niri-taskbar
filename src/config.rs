use std::collections::HashMap;

use itertools::Itertools;
use regex::Regex;
use serde::{Deserialize, Deserializer};

/// The taskbar configuration.
#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(default)]
    apps: HashMap<String, Vec<AppConfig>>,
    #[serde(default)]
    notifications: Notifications,
    #[serde(
        default = "default_resize_presets",
        deserialize_with = "deserialise_resize_presets"
    )]
    resize_presets: Vec<ResizePreset>,
    #[serde(default)]
    show_all_outputs: bool,
}

/// A column width preset offered in the button context menu.
#[derive(Debug, Clone, PartialEq)]
pub struct ResizePreset {
    pub label: String,
    /// The proportion of the working area, in the range (0, 1].
    pub proportion: f64,
}

#[derive(Debug, Deserialize)]
pub struct Notifications {
    #[serde(default = "default_true")]
    enabled: bool,
    #[serde(default)]
    map_app_ids: HashMap<String, String>,
    #[serde(default = "default_true")]
    use_desktop_entry: bool,
    #[serde(default)]
    use_fuzzy_matching: bool,
}

impl Default for Notifications {
    fn default() -> Self {
        Self {
            enabled: true,
            map_app_ids: Default::default(),
            use_desktop_entry: true,
            use_fuzzy_matching: Default::default(),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            apps: Default::default(),
            notifications: Default::default(),
            resize_presets: default_resize_presets(),
            show_all_outputs: Default::default(),
        }
    }
}

fn default_true() -> bool {
    true
}

/// The defaults match the `preset-column-widths` in Niri's default configuration.
fn default_resize_presets() -> Vec<ResizePreset> {
    [("33%", 1.0 / 3.0), ("50%", 0.5), ("66%", 2.0 / 3.0)]
        .into_iter()
        .map(|(label, proportion)| ResizePreset {
            label: label.to_string(),
            proportion,
        })
        .collect()
}

impl Config {
    /// Returns all possible CSS classes that a particular application might have set.
    pub fn app_classes(&self, app_id: &str) -> Vec<&str> {
        self.apps
            .get(app_id)
            .map(|configs| {
                configs
                    .iter()
                    .map(|config| config.class.as_str())
                    .collect_vec()
            })
            .unwrap_or_default()
    }

    /// Returns the actual CSS classes that should be set for the given application and title.
    pub fn app_matches<'a>(
        &'a self,
        app_id: &str,
        title: &'a str,
    ) -> Box<dyn Iterator<Item = &'a str> + 'a> {
        match self.apps.get(app_id) {
            Some(configs) => Box::new(
                configs
                    .iter()
                    .filter(|config| config.re.is_match(title))
                    .map(|config| config.class.as_str()),
            ),
            None => Box::new(std::iter::empty()),
        }
    }

    /// Returns true if notification support is enabled.
    pub fn notifications_enabled(&self) -> bool {
        self.notifications.enabled
    }

    /// Returns any mapping that might exist for this app ID.
    pub fn notifications_app_map(&self, app_id: &str) -> Option<&'_ str> {
        self.notifications
            .map_app_ids
            .get(app_id)
            .map(String::as_str)
    }

    /// Returns true if notification support should use the desktop entry as a
    /// fallback.
    pub fn notifications_use_desktop_entry(&self) -> bool {
        self.notifications.use_desktop_entry
    }

    pub fn notifications_use_fuzzy_matching(&self) -> bool {
        self.notifications.use_fuzzy_matching
    }

    /// Returns the column width presets, ordered by proportion.
    pub fn resize_presets(&self) -> &[ResizePreset] {
        &self.resize_presets
    }

    pub fn show_all_outputs(&self) -> bool {
        self.show_all_outputs
    }
}

#[derive(Deserialize, Debug)]
struct AppConfig {
    #[serde(rename = "match", deserialize_with = "deserialise_regex")]
    re: Regex,
    class: String,
}

fn deserialise_regex<'de, D>(de: D) -> Result<Regex, D::Error>
where
    D: Deserializer<'de>,
{
    Regex::new(&String::deserialize(de)?).map_err(serde::de::Error::custom)
}

fn deserialise_resize_presets<'de, D>(de: D) -> Result<Vec<ResizePreset>, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(resize_presets_from_map(
        HashMap::<String, f64>::deserialize(de)?,
    ))
}

fn resize_presets_from_map(map: HashMap<String, f64>) -> Vec<ResizePreset> {
    // JSON objects don't reliably preserve their order, so we sort by proportion instead.
    map.into_iter()
        .filter_map(|(label, proportion)| {
            if proportion > 0.0 && proportion <= 1.0 {
                Some(ResizePreset { label, proportion })
            } else {
                tracing::warn!(
                    label,
                    proportion,
                    "ignoring resize preset outside the range (0, 1]"
                );
                None
            }
        })
        .sorted_by(|a, b| {
            a.proportion
                .total_cmp(&b.proportion)
                .then_with(|| a.label.cmp(&b.label))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resize_presets_are_sorted_and_validated() {
        let presets = resize_presets_from_map(HashMap::from([
            ("Full".to_string(), 1.0),
            ("25%".to_string(), 0.25),
            ("Bad".to_string(), 1.5),
            ("Zero".to_string(), 0.0),
            ("50%".to_string(), 0.5),
        ]));

        assert_eq!(
            presets.iter().map(|p| p.label.as_str()).collect_vec(),
            ["25%", "50%", "Full"]
        );
    }

    #[test]
    fn resize_presets_default_to_niri_defaults() {
        assert_eq!(
            Config::default()
                .resize_presets()
                .iter()
                .map(|p| p.label.as_str())
                .collect_vec(),
            ["33%", "50%", "66%"]
        );
    }
}
