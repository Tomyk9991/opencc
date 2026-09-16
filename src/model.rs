use std::fmt::Display;

/// A model from the Go usage table (`data-slot="model-row"`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Model {
    /// Slug from `data-model`, e.g. `"muse-spark-1.3-contributor"`.
    pub id: String,
    /// Display name, e.g. `"Muse Spark 1.3 Contributor"`.
    pub name: String,
    /// Estimated requests / 5h, e.g. `45300` (from `"45.300"`).
    pub num_requests: u32,
    /// Additional markers, e.g. `["Neu", "4× Nutzung"]` or `["begrenzte Regionen"]`.
    pub markers: Markers,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Markers {
    markers: Vec<String>,
}

impl Markers {
    pub fn new(markers: Vec<String>) -> Self {
        Self { markers }
    }

    #[allow(dead_code)]
    pub fn iter(&self) -> std::slice::Iter<'_, String> {
        self.markers.iter()
    }
}

impl Display for Markers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            if !self.markers.is_empty() {
                format!(" [{}]", self.markers.join(", "))
            } else {
                String::new()
            }
        )
    }
}

impl Display for Model {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} ({}): {}{}",
            self.name, self.id, self.num_requests, self.markers
        )
    }
}
