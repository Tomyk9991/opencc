use scraper::{Html, Selector};

use crate::{
    client_error::{ClientError, FetchError},
    model::{Markers, Model},
};

/// URL of the Go page with the usage table.
pub const GO_URL: &str = "https://opencode.ai/de/go";

/// Client for the opencode.ai Go page.
///
/// Exposes exactly **one** endpoint ([`OpenCodeClient::models`]) returning all
/// models of the comparison table as [`Model`]s.
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
    /// release: downloaded from the URL) and parses all model entries.
    ///
    /// Returns an error (instead of an empty list) when nothing could be
    /// parsed, so the CLI can print a user-friendly message.
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

    /// Parses models from raw HTML.
    ///
    /// Supports the current layout (`go-plan-chart` comparison table) and,
    /// as a fallback, the legacy layout (`data-slot="model-row"` divs).
    /// Returns [`FetchError::Empty`] when neither yields a row.
    fn parse(html: &str) -> Result<Vec<Model>, FetchError> {
        let doc = Html::parse_document(html);

        let models = Self::parse_current(&doc)?;
        if !models.is_empty() {
            return Ok(models);
        }

        let legacy = Self::parse_legacy(&doc)?;
        if !legacy.is_empty() {
            return Ok(legacy);
        }

        Err(FetchError::Empty)
    }

    /// Current layout: `<figure data-component="go-plan-chart">` table.
    ///
    /// ```html
    /// <tr>
    ///   <th scope="row"><bdi>Kimi K3</bdi></th>
    ///   <td data-slot="number"><span>110</span><span>440</span></td>  <!-- Go / Go Plus -->
    ///   <td data-slot="number"><span>15 $</span>...</td>              <!-- monthly usage -->
    /// </tr>
    /// ```
    /// Markers live in `<span data-slot="labels">` as `<small>` (e.g. `Neu`,
    /// `begrenzte Zeit`) and `<a>` (e.g. `begrenzte Regionen`).
    /// Free models show `∞` instead of a number.
    fn parse_current(doc: &Html) -> Result<Vec<Model>, FetchError> {
        let row_sel = Selector::parse(r#"tbody tr"#)?;
        let name_sel = Selector::parse(r#"th[scope="row"] bdi"#)?;
        let number_td_sel = Selector::parse(r#"td[data-slot="number"]"#)?;
        let number_span_sel = Selector::parse(r#"span"#)?;
        let label_small_sel = Selector::parse(r#"[data-slot="labels"] small"#)?;
        let label_link_sel = Selector::parse(r#"[data-slot="labels"] a"#)?;

        let models = doc
            .select(&row_sel)
            .filter_map(|row| {
                let raw_name = row
                    .select(&name_sel)
                    .next()
                    .map(|el| el.text().collect::<String>())
                    .unwrap_or_default();
                let name = raw_name.split_whitespace().collect::<Vec<_>>().join(" ");
                if name.is_empty() {
                    return None; // header row / axis row
                }

                // First `td[data-slot="number"]`, first `<span>` = Go tier value.
                let raw_requests = row
                    .select(&number_td_sel)
                    .next()
                    .and_then(|td| td.select(&number_span_sel).next())
                    .map(|el| el.text().collect::<String>())
                    .unwrap_or_default();
                let unlimited = raw_requests.contains('∞');
                let digits: String = raw_requests
                    .chars()
                    .filter(|c| c.is_ascii_digit())
                    .collect();
                let num_requests = digits.parse::<u32>().unwrap_or(0);

                let mut markers: Vec<String> = row
                    .select(&label_small_sel)
                    .map(|el| {
                        el.text()
                            .collect::<String>()
                            .split_whitespace()
                            .collect::<Vec<_>>()
                            .join(" ")
                    })
                    .chain(row.select(&label_link_sel).map(|el| {
                        el.text()
                            .collect::<String>()
                            .split_whitespace()
                            .collect::<Vec<_>>()
                            .join(" ")
                    }))
                    .filter(|m| !m.is_empty())
                    .collect();
                markers.sort();
                markers.dedup();

                Some(Model {
                    id: Self::slugify(&name),
                    name,
                    num_requests,
                    unlimited,
                    markers: Markers::new(markers),
                })
            })
            .collect();

        Ok(models)
    }

    /// Legacy layout: `<div data-slot="model-row" data-model="...">` rows.
    /// Kept as a fallback so old fixtures / cached pages keep working.
    fn parse_legacy(doc: &Html) -> Result<Vec<Model>, FetchError> {
        let row_sel = Selector::parse(r#"[data-slot="model-row"]"#)?;
        let name_sel = Selector::parse(r#"[data-slot="model"] bdi"#)?;
        let requests_sel = Selector::parse(r#"[data-slot="requests"] bdi"#)?;
        let badge_sel = Selector::parse(r#"[data-slot="badge"]"#)?;
        let region_sel = Selector::parse(r#"[data-slot="region"]"#)?;

        let models = doc
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
                // Free models show `∞` (`data-unlimited` row attribute, `title="unbegrenzt"`).
                let unlimited =
                    row.value().attr("data-unlimited").is_some() || raw_requests.contains('∞');
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
                    id: if id.is_empty() {
                        Self::slugify(&name)
                    } else {
                        id
                    },
                    name,
                    num_requests,
                    unlimited,
                    markers: Markers::new(markers),
                })
            })
            .collect();

        Ok(models)
    }

    /// Derives a stable id from the display name, e.g.
    /// `"Muse Spark 1.3 Contributor"` -> `"muse-spark-1.3-contributor"`.
    /// Dots are kept for compatibility with the previous `data-model` slugs.
    fn slugify(name: &str) -> String {
        let mut slug = String::with_capacity(name.len());
        let mut prev_dash = true; // trim leading separators
        for c in name.to_lowercase().chars() {
            if c.is_ascii_alphanumeric() || c == '.' {
                slug.push(c);
                prev_dash = false;
            } else if !prev_dash {
                slug.push('-');
                prev_dash = true;
            }
        }
        slug.trim_matches('-').to_owned()
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
    fn parses_current_fixture_rows() {
        let html = include_str!("../HtmlResponse.html");
        let models = OpenCodeClient::parse(html).expect("parse failed");

        assert_eq!(models.len(), 11, "expected 11 comparison-table rows");

        let kimi = models
            .iter()
            .find(|m| m.name == "Kimi K3")
            .expect("missing Kimi K3");
        assert_eq!(kimi.num_requests, 110);
        assert_eq!(kimi.id, "kimi-k3");

        let muse = models
            .iter()
            .find(|m| m.name == "Muse Spark 1.3 Contributor")
            .expect("missing Muse Spark 1.3 Contributor");
        assert_eq!(muse.num_requests, 45300);
        assert_eq!(muse.id, "muse-spark-1.3-contributor");
        assert!(
            muse.markers.iter().any(|m| m == "begrenzte Regionen"),
            "missing marker 'begrenzte Regionen'"
        );

        let flash = models
            .iter()
            .find(|m| m.name == "DeepSeek V4.1 Flash")
            .expect("missing DeepSeek V4.1 Flash");
        assert_eq!(flash.num_requests, 26000);
        assert!(
            flash.markers.iter().any(|m| m == "Neu"),
            "missing marker 'Neu'"
        );

        let free = models
            .iter()
            .find(|m| m.id == "space-bunny-free")
            .expect("missing Space Bunny Free");
        assert_eq!(free.name, "Space Bunny Free");
        assert!(free.unlimited, "space-bunny-free must be unlimited");
        assert_eq!(free.num_requests, 0);
    }

    #[test]
    fn parses_legacy_model_rows() {
        let html = r#"
            <div data-slot="model-row" data-model="muse-spark-1.3-contributor">
              <div data-slot="model"><bdi>Muse Spark 1.3 Contributor</bdi></div>
              <div data-slot="requests"><bdi>45.300</bdi></div>
            </div>
            <div data-slot="model-row" data-model="union-alpha">
              <div data-slot="model"><bdi>Union Alpha Free</bdi></div>
              <div data-slot="requests"><bdi>&#8734;</bdi></div>
            </div>
        "#;
        let models = OpenCodeClient::parse(html).expect("legacy parse failed");
        assert_eq!(models.len(), 2);
        assert_eq!(models[0].num_requests, 45300);
        assert!(!models[0].unlimited);
        assert!(models[1].unlimited);
    }

    #[test]
    fn empty_html_returns_empty_error() {
        let err = OpenCodeClient::parse("<html><body></body></html>").unwrap_err();
        assert!(matches!(err, FetchError::Empty));
    }

    #[tokio::test]
    async fn endpoint_uses_fixture_in_test() {
        let client = OpenCodeClient::new();
        let models = client.models().await.expect("endpoint failed");
        assert_eq!(models.len(), 11);
    }
}
