use k3_wasm_macros::{define_k3_write_inputs, http_handler};
use k3_wasm_sdk::{
    get_args,
    http::{Request, Response, StatusCode},
};
use serde::{Deserialize, Serialize};
extern crate k3_json;
use k3_json::parse_args;

define_k3_write_inputs!();

// MCP request format
#[derive(Debug, Serialize, Deserialize)]
struct MCPRequest {
    tool_name: String,
    parameters: serde_json::Value,
    request_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct MCPResponse {
    request_id: String,
    content: Vec<ContentItem>,
    is_error: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct ContentItem {
    #[serde(rename = "type")]
    content_type: String,
    text: String,
}

const UNIBLOCK_API_KEY: &str = "O0KTd-fbWo5ShbceBCbpJXf8EWcf4T8jM_dsFDSoN0Y";

#[http_handler]
pub fn post(_req: Request<Vec<u8>>) -> Response<Vec<u8>> {
    let json_str = unsafe { get_args(ARG_PTR, ARG_LEN) };

    let args: MCPRequest = match parse_args(&json_str) {
        Ok(parsed) => parsed,
        Err(response) => return response,
    };

    // Match on tool_name
    let response = match args.tool_name.as_str() {
        "getBTCPrice" => get_btc_price(&args),
        "listAvailableTools" => list_available_tools(&args),
        _ => build_error(&args, &format!("Unknown tool: {}", args.tool_name)),
    };

    let response_json = serde_json::to_string(&response).unwrap();

    Response::builder()
        .status(StatusCode::OK)
        .body(response_json.into_bytes())
        .unwrap()
}

fn get_btc_price(req: &MCPRequest) -> MCPResponse {
    let url = "https://api.uniblock.dev/uni/v1/market-data/price?symbol=BTC";
    let headers = format!(
        "Content-Type: application/json, x-api-key: {}",
        UNIBLOCK_API_KEY
    )
    .to_string();

    match k3_wasm_sdk::http::get_auth(url, &headers) {
        Some(response_bytes) => {
            let json: serde_json::Value =
                serde_json::from_slice(&response_bytes).unwrap_or_default();
            let price = json["BTC"]["usd"].as_f64().unwrap_or(-1.0);
            build_success(req, &format!("BTC price: {}", price))
        }
        None => build_error(req, "Failed to fetch BTC price from Uniblock"),
    }
}

fn build_success(req: &MCPRequest, result: &str) -> MCPResponse {
    MCPResponse {
        request_id: req.request_id.clone(),
        content: vec![ContentItem {
            content_type: "text".to_string(),
            text: result.to_string(),
        }],
        is_error: false,
    }
}

fn build_error(req: &MCPRequest, error_msg: &str) -> MCPResponse {
    MCPResponse {
        request_id: req.request_id.clone(),
        content: vec![ContentItem {
            content_type: "text".to_string(),
            text: error_msg.to_string(),
        }],
        is_error: true,
    }
}

fn list_available_tools(req: &MCPRequest) -> MCPResponse {
    build_success(req, "Available tools: getBTCPrice")
}

k3_wasm_macros::init!();
