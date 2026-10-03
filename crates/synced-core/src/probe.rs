use reqwest::header::{ACCEPT_RANGES, CONTENT_DISPOSITION, CONTENT_LENGTH, ETAG};
use reqwest::{Client, StatusCode};
use url::Url;

use crate::error::CoreError;

/// Metadata describing remote resource capabilities.
#[derive(Debug, Clone)]
pub struct ProbeInfo {
    pub url: String,
    pub file_name: String,
    pub content_length: Option<u64>,
    pub accept_ranges: bool,
    pub etag: Option<String>,
}

/// Probes a URL to discover server capabilities, resource size, and file name.
pub async fn probe_url(client: &Client, url: &str) -> Result<ProbeInfo, CoreError> {
    Url::parse(url)?;

    let head_response = client.head(url).send().await;
    let response = match head_response {
        Ok(resp) if resp.status().is_success() => resp,
        _ => {
            client
                .get(url)
                .header(reqwest::header::RANGE, "bytes=0-0")
                .send()
                .await?
        }
    };

    let status = response.status();
    if !status.is_success() && status != StatusCode::PARTIAL_CONTENT {
        return Err(CoreError::HttpStatus(status));
    }

    let headers = response.headers();
    let content_length = extract_content_length(headers, status);
    let accept_ranges = check_range_support(status, headers);
    let etag = headers.get(ETAG).and_then(|v| v.to_str().ok()).map(String::from);
    let effective_url = response.url().clone();
    let file_name = extract_filename(headers, &effective_url);

    Ok(ProbeInfo {
        url: effective_url.to_string(),
        file_name,
        content_length,
        accept_ranges,
        etag,
    })
}

/// Determines whether the server supports partial byte range requests.
pub fn check_range_support(status: StatusCode, headers: &reqwest::header::HeaderMap) -> bool {
    if status == StatusCode::PARTIAL_CONTENT {
        return true;
    }

    headers
        .get(ACCEPT_RANGES)
        .and_then(|v| v.to_str().ok())
        .map(|v| v.eq_ignore_ascii_case("bytes"))
        .unwrap_or(false)
}

/// Extracts content length from standard headers or Content-Range response.
pub fn extract_content_length(
    headers: &reqwest::header::HeaderMap,
    status: StatusCode,
) -> Option<u64> {
    if status == StatusCode::PARTIAL_CONTENT {
        if let Some(val) = headers.get(reqwest::header::CONTENT_RANGE) {
            if let Ok(val_str) = val.to_str() {
                if let Some(total_str) = val_str.rsplit('/').next() {
                    if let Ok(total) = total_str.trim().parse::<u64>() {
                        return Some(total);
                    }
                }
            }
        }
    }

    headers
        .get(CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u64>().ok())
}

/// Determines file name from Content-Disposition header or fallback URL path.
pub fn extract_filename(headers: &reqwest::header::HeaderMap, url: &Url) -> String {
    if let Some(disposition) = headers.get(CONTENT_DISPOSITION) {
        if let Ok(disp_str) = disposition.to_str() {
            for part in disp_str.split(';') {
                let trimmed = part.trim();
                if let Some(name) = trimmed.strip_prefix("filename=") {
                    return name.trim_matches('"').trim_matches('\'').to_string();
                }
            }
        }
    }

    url.path_segments()
        .and_then(|segments| segments.last())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "download.bin".to_string())
}
