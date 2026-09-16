use scraper::{Html, Selector};

use crate::{
    client_error::{ClientError, FetchError},
    model::{Markers, Model},
};

/// URL der Go-Seite mit der Nutzungstabelle (`data-slot="model-row"`).
pub const GO_URL: &str = "https://opencode.ai/de/go";

/// Client für die opencode.ai-Go-Seite.
///
/// Besitzt genau **einen** Endpunkt ([`OpenCodeClient::models`]), der alle
/// `data-slot="model-row"`-Zeilen als [`Model`]s liefert.
#[derive(Debug, Clone)]
pub struct OpenCodeClient {
    #[allow(dead_code)] // nur im Release-Pfad (Download) genutzt
    http: reqwest::Client,
    #[allow(dead_code)] // nur im Release-Pfad (Download) genutzt
    url: String,
}

impl OpenCodeClient {
    /// Erstellt einen Client für [`GO_URL`].
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::new(),
            url: GO_URL.to_owned(),
        }
    }

    /// Genau ein Endpunkt: lädt die HTML (Dev/Test: lokale `HtmlResponse.html`,
    /// Release: dynamisch per URL) und parst alle `model-row`-Einträge.
    pub async fn models(&self) -> Result<Vec<Model>, ClientError> {
        let html = self.fetch_html().await?;
        Ok(Self::parse(&html)?)
    }

    /// HTML-Quelle: Entwicklung & Tests -> lokale Datei, danach -> Download.
    #[cfg(any(debug_assertions, test))]
    async fn fetch_html(&self) -> Result<String, ClientError> {
        // Nur solange wir entwickeln/testen: fest verdrahtete Fixture-Datei.
        Ok(include_str!("../HtmlResponse.html").to_owned())
    }

    /// HTML-Quelle: Entwicklung & Tests -> lokale Datei, danach -> Download.
    #[cfg(not(any(debug_assertions, test)))]
    async fn fetch_html(&self) -> Result<String, ClientError> {
        // Produktivpfad: HTML dynamisch über die URL herunterladen.
        let body = self.http.get(&self.url).send().await?.text().await?;
        Ok(body)
    }

    /// Parst alle `data-slot="model-row"`-Zeilen aus rohem HTML.
    fn parse(html: &str) -> Result<Vec<Model>, FetchError> {
        let doc = Html::parse_document(html);
        let row_sel = Selector::parse(r#"[data-slot="model-row"]"#)?;
        let name_sel = Selector::parse(r#"[data-slot="model"] bdi"#)?;
        let requests_sel = Selector::parse(r#"[data-slot="requests"] bdi"#)?;
        let badge_sel = Selector::parse(r#"[data-slot="badge"]"#)?;
        let region_sel = Selector::parse(r#"[data-slot="region"]"#)?;

        let model = doc
            .select(&row_sel)
            .filter_map(|row| {
                let raw_name = row
                    .select(&name_sel)
                    .next()
                    .map(|el| el.text().collect::<String>())
                    .unwrap_or_default();
                // "Muse Spark 1.3\n   Contributor" -> "Muse Spark 1.3 Contributor"
                let name = raw_name.split_whitespace().collect::<Vec<_>>().join(" ");
                if name.is_empty() {
                    return None;
                }

                // Deutsche Tausendertrennung: "45.300" -> 45300.
                // Es wird bewusst der aktuelle <bdi>-Wert genommen,
                // nicht ein ggf. vorhandener durchgestrichener <s>-Altpreis.
                let raw_requests = row
                    .select(&requests_sel)
                    .next()
                    .map(|el| el.text().collect::<String>())
                    .unwrap_or_default();
                let digits: String = raw_requests
                    .chars()
                    .filter(|c| c.is_ascii_digit())
                    .collect();
                let num_requests = digits.parse::<u32>().unwrap_or(0);

                // Marker wie "Neu" / "4× Nutzung" (Badges) ...
                let mut markers: Vec<String> = row
                    .select(&badge_sel)
                    .map(|b| {
                        b.text()
                            .collect::<String>()
                            .split_whitespace()
                            .collect::<Vec<_>>()
                            .join(" ")
                    })
                    .filter(|m| !m.is_empty())
                    .collect();

                // ... plus Regions-Hinweis (z. B. Muse Spark: "begrenzte Regionen").
                if let Some(region) = row.select(&region_sel).next() {
                    let label = region
                        .value()
                        .attr("title")
                        .or_else(|| region.value().attr("aria-label"))
                        .unwrap_or("begrenzte Regionen")
                        .split_whitespace()
                        .collect::<Vec<_>>()
                        .join(" ");
                    if !label.is_empty() && !markers.contains(&label) {
                        markers.push(label);
                    }
                }

                let id = row
                    .value()
                    .attr("data-model")
                    .unwrap_or_default()
                    .to_owned();

                Some(Model {
                    id,
                    name,
                    num_requests,
                    markers: Markers::new(markers),
                })
            })
            .collect();

        Ok(model)
    }
}

impl Default for OpenCodeClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_fixture_rows() {
        let html = include_str!("../HtmlResponse.html");
        let models = OpenCodeClient::parse(html).expect("parsen schlägt fehl");

        assert_eq!(models.len(), 10, "erwartet 10 model-row-Zeilen");

        let muse = models
            .iter()
            .find(|m| m.name == "Muse Spark 1.3 Contributor")
            .expect("Muse Spark 1.3 Contributor fehlt");
        assert_eq!(muse.num_requests, 45300);
        assert_eq!(muse.id, "muse-spark-1.3-contributor");

        let flash = models
            .iter()
            .find(|m| m.name == "DeepSeek V4.1 Flash")
            .expect("DeepSeek V4.1 Flash fehlt");
        assert_eq!(flash.num_requests, 26000);
        assert!(
            flash.markers.iter().any(|m| m == "Neu"),
            "Marker 'Neu' fehlt"
        );
        assert!(
            flash.markers.iter().any(|m| m == "4× Nutzung"),
            "Marker '4× Nutzung' fehlt"
        );
    }

    #[tokio::test]
    async fn endpoint_uses_fixture_in_test() {
        let client = OpenCodeClient::new();
        let models = client.models().await.expect("Endpunkt schlägt fehl");
        assert_eq!(models.len(), 10);
    }
}
