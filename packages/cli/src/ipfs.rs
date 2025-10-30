use reqwest::multipart;
use serde::Deserialize;

/// Result of uploading a file to IPFS
#[derive(Debug, Clone)]
pub struct IpfsFile {
    /// The content identifier (CID) of the uploaded file
    pub cid: String,

    /// The IPFS URI (e.g., "ipfs://Qm...")
    pub uri: String,

    /// The gateway URL for accessing the file via HTTP
    pub gateway_url: String,
}

/// Upload a single file to an IPFS HTTP API and return the CID and URIs.
impl IpfsFile {
    pub async fn upload(
        bytes: Vec<u8>,
        filename: &str,
        // The base URL for the IPFS API (e.g., "http://127.0.0.1:5001")
        api_base: &str,
        // The base URL for the IPFS gateway (e.g., "http://127.0.0.1:8080")
        gateway_base: &str,
        wrap_with_directory: bool,
    ) -> anyhow::Result<Self> {
        // Request CIDv1 with base32 encoding for modern, case-insensitive URIs
        // pin=true keeps the file in the local IPFS repository
        // Strip trailing slash from api_base to avoid double slashes
        let api_base = api_base.trim_end_matches('/');
        let url = format!(
            "{}/api/v0/add?cid-version=1&pin=true&wrap-with-directory={}",
            api_base, wrap_with_directory
        );

        let part = multipart::Part::bytes(bytes)
            .file_name(filename.to_string())
            .mime_str("application/octet-stream")?;

        let form = multipart::Form::new().part("file", part);

        let client = reqwest::Client::new();

        let resp = client
            .post(&url)
            .multipart(form)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to connect to IPFS API at {}: {}", api_base, e))?;

        // The /api/v0/add endpoint returns NDJSON (newline-delimited JSON)
        // - For a single file: one JSON line with the file's CID
        // - With wrap-with-directory=true: two lines (file CID, then directory CID)
        // We want the last line, which is the root CID
        let text = resp
            .error_for_status()
            .map_err(|e| anyhow::anyhow!("IPFS API error: {}", e))?
            .text()
            .await?;

        let last_line = text
            .lines()
            .filter(|l| !l.trim().is_empty())
            .next_back()
            .ok_or_else(|| anyhow::anyhow!("Empty response from IPFS API"))?;

        /// Response from the IPFS Kubo API's `/api/v0/add` endpoint.
        /// Kubo returns field names in PascalCase, so we use serde rename.
        #[derive(Debug, Deserialize)]
        struct AddResponse {
            #[serde(rename = "Name")]
            _name: Option<String>,

            #[serde(rename = "Hash")]
            hash: String, // The CID

            #[serde(rename = "Size")]
            _size: Option<String>,
        }

        let parsed: AddResponse = serde_json::from_str(last_line)
            .map_err(|e| anyhow::anyhow!("Failed to parse IPFS response: {}", e))?;

        // Build the URIs based on whether we wrapped with a directory
        // Strip trailing slash from gateway_base to avoid double slashes
        let gateway_base = gateway_base.trim_end_matches('/');
        let (cid, uri, gateway_url) = if wrap_with_directory {
            // When wrapping, the last line contains the directory CID
            // The URI should include the filename as a path
            let root_cid = parsed.hash;
            let uri = format!("ipfs://{}/{}", root_cid, filename);
            let gateway = format!("{}/ipfs/{}/{}", gateway_base, root_cid, filename);
            (root_cid, uri, gateway)
        } else {
            // Direct file upload - the CID points directly to the file content
            let cid = parsed.hash;
            let uri = format!("ipfs://{}", cid);
            let gateway = format!("{}/ipfs/{}", gateway_base, cid);
            (cid, uri, gateway)
        };

        Ok(Self {
            cid,
            uri,
            gateway_url,
        })
    }
}
