use std::fmt::Display;

/// Ein Modell aus der Go-Nutzungstabelle (`data-slot="model-row"`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Model {
    /// Slug aus `data-model`, z. B. `"muse-spark-1.3-contributor"`.
    pub id: String,
    /// Anzeigename, z. B. `"Muse Spark 1.3 Contributor"`.
    pub name: String,
    /// Geschätzte Anfragen / 5 Std., z. B. `45300` (aus `"45.300"`).
    pub num_requests: u32,
    /// Zusätzliche Marker, z. B. `["Neu", "4× Nutzung"]` oder `["begrenzte Regionen"]`.
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
