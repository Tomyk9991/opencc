use scraper::{Html, Selector};

use crate::{
    client_error::{ClientError, FetchError},
    model::{Markers, Model},
};

/// URL of the Go page with the usage table (`data-slot="model-row"`).
pub const GO_URL: &str = "https://opencode.ai/de/go";

/// Client for the opencode.ai Go page.
///
/// Exposes exactly **one** endpoint ([`OpenCodeClient::models`]) returning all
/// `data-slot="model-row"` rows as [`Model`]s.
#[derive(Debug, Clone)]
pub struct OpenCodeClient {
    #[allow(dead_code)] // only used on the release path (download)
    http: reqwest::Client,
    #[allow(dead_code)] // only used on the release path (download)
    url: String,
}

impl OpenCodeClient {
    /// Creates a client for [`GO_URL`]
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::new(),
            url: GO_URL.to_owned(),
        }
    }

    /// Single endpoint: loads the HTML (dev/test: local `HtmlResponse.html`,
    /// release: downloaded from the URL) and parses all `model-row` entries.
    pub async fn models(&self) -> Result<Vec<Model>, ClientError> {
        let html = self.fetch_html().await?;
        Ok(Self::parse(&html)?)
    }

    /// HTML source: local file during development & tests, download otherwise.
    #[cfg(any(debug_assertions, test))]
    async fn fetch_html(&self) -> Result<String, ClientError> {
        Ok(include_str!("../HtmlResponse.html").to_owned())
    }

    /// HTML source: local file during development & tests, download otherwise.
    #[cfg(not(any(debug_assertions, test)))]
    async fn fetch_html(&self) -> Result<String, ClientError> {
        let body = self.http.get(&self.url).send().await?.text().await?;
        Ok(body)
    }

    /// Parses all `data-slot="model-row"` rows from raw HTML.
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

                // German thousands separator: "45.300" -> 45300.
                // Uses the current <bdi> value, not a possibly struck-through <s> old value.
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

                // Badges like "Neu" / "4× Nutzung" ...
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

                // ... plus region hint (e.g. Muse Spark: "begrenzte Regionen").
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
        let models = OpenCodeClient::parse(html).expect("parse failed");

        assert_eq!(models.len(), 10, "expected 10 model-row rows");

        let muse = models
            .iter()
            .find(|m| m.name == "Muse Spark 1.3 Contributor")
            .expect("missing Muse Spark 1.3 Contributor");
        assert_eq!(muse.num_requests, 45300);
        assert_eq!(muse.id, "muse-spark-1.3-contributor");

        let flash = models
            .iter()
            .find(|m| m.name == "DeepSeek V4.1 Flash")
            .expect("missing DeepSeek V4.1 Flash");
        assert_eq!(flash.num_requests, 26000);
        assert!(
            flash.markers.iter().any(|m| m == "Neu"),
            "missing marker 'Neu'"
        );
        assert!(
            flash.markers.iter().any(|m| m == "4× Nutzung"),
            "missing marker '4× Nutzung'"
        );
    }

    #[tokio::test]
    async fn endpoint_uses_fixture_in_test() {
        let client = OpenCodeClient::new();
        let models = client.models().await.expect("endpoint failed");
        assert_eq!(models.len(), 10);
    }
}
