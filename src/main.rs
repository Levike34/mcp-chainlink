use k3_wasm_macros::{define_k3_write_inputs, http_handler};
use k3_wasm_sdk::{
    get_args,
    http::{post, Request, Response, StatusCode},
};
use serde::{Deserialize, Serialize};
extern crate k3_json;
use k3_json::parse_args;

define_k3_write_inputs!();

mod feeds;
use feeds::FEEDS_JSON;

const INFURA_API_KEY: &str = "aed643c08a214986a65912d60569ffb1";

// Feeds structure
#[derive(Debug, Serialize, Deserialize)]
struct FeedsData(std::collections::HashMap<String, ChainData>);

#[derive(Debug, Serialize, Deserialize)]
struct ChainData {
    baseUrl: String,
    feeds: Vec<FeedEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
struct FeedEntry {
    name: String,
    proxyAddress: String,
    feedCategory: String,
}

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

#[http_handler]
pub fn post(_req: Request<Vec<u8>>) -> Response<Vec<u8>> {
    let json_str = unsafe { get_args(ARG_PTR, ARG_LEN) };

    let args: MCPRequest = match parse_args(&json_str) {
        Ok(parsed) => parsed,
        Err(response) => return response,
    };

    // Parse feeds.json once
    let feeds: FeedsData = serde_json::from_str(FEEDS_JSON).unwrap();

    // Match on tool_name
    let response = match args.tool_name.as_str() {
        "getLatestPrice" => get_latest_price(&feeds, &args),
        "queryPriceByRound" => query_price_by_round(&feeds, &args),
        "listSupportedChains" => list_supported_chains(&feeds, &args),
        "listSupportedFeedsByChain" => list_supported_feeds_by_chain(&feeds, &args),
        "listSupportedFeeds" => list_supported_feeds(&feeds, &args),
        "listAvailableTools" => list_available_tools(&args),
        _ => MCPResponse {
            request_id: args.request_id.clone(),
            content: vec![ContentItem {
                content_type: "text".to_string(),
                text: format!("Unknown tool: {}", args.tool_name),
            }],
            is_error: true,
        },
    };

    let response_json = serde_json::to_string(&response).unwrap();

    Response::builder()
        .status(StatusCode::OK)
        .body(response_json.into_bytes())
        .unwrap()
}

fn get_latest_price(feeds: &FeedsData, req: &MCPRequest) -> MCPResponse {
    let pair = req
        .parameters
        .get("pair")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let chain = req
        .parameters
        .get("chain")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_lowercase();

    if let Some(chain_data) = feeds.0.get(&chain) {
        if let Some(feed) = chain_data
            .feeds
            .iter()
            .find(|f| f.name.eq_ignore_ascii_case(pair))
        {
            // Call Chainlink proxy contract using RPC directly
            let price = call_chainlink_proxy(&chain_data.baseUrl, &feed.proxyAddress);
            return build_success(
                req,
                &format!(
                    "{{\"chain\": \"{}\", \"pair\": \"{}\", \"price\": {}}}",
                    chain, pair, price
                ),
            );
        }
    }

    build_error(req, &format!("Pair {} not found on chain {}", pair, chain))
}

fn query_price_by_round(feeds: &FeedsData, req: &MCPRequest) -> MCPResponse {
    // For simplicity in this WASM version, we don't support querying old rounds (as onchain RPC needs more infra)
    build_error(
        req,
        "Historical queryPriceByRound not supported in WASM version",
    )
}

fn list_supported_chains(feeds: &FeedsData, req: &MCPRequest) -> MCPResponse {
    let chains: Vec<String> = feeds.0.keys().cloned().collect();
    build_success(req, &chains.join(", "))
}

fn list_supported_feeds_by_chain(feeds: &FeedsData, req: &MCPRequest) -> MCPResponse {
    let chain = req
        .parameters
        .get("chain")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_lowercase();

    if let Some(chain_data) = feeds.0.get(&chain) {
        let feed_names: Vec<String> = chain_data.feeds.iter().map(|f| f.name.clone()).collect();
        build_success(req, &feed_names.join(", "))
    } else {
        build_error(req, &format!("Unknown chain: {}", chain))
    }
}

fn list_supported_feeds(feeds: &FeedsData, req: &MCPRequest) -> MCPResponse {
    let mut out = String::new();
    for (chain, chain_data) in &feeds.0 {
        let names: Vec<String> = chain_data.feeds.iter().map(|f| f.name.clone()).collect();
        out += &format!("- {}: {}\n", chain, names.join(", "));
    }
    build_success(req, &out)
}

// =============================
// Helper builders

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

// =============================
// Chainlink proxy contract call

fn call_chainlink_proxy(base_url: &str, proxy_address: &str) -> f64 {
    // This would call your RPC node to get latestRoundData.answer
    let url = format!("{}/{}", base_url, INFURA_API_KEY);
    let headers = "Content-Type: application/json".to_string();

    let payload = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [{
            "to": proxy_address,
            "data": "0xfeaf968c" // latestRoundData selector
        }, "latest"],
        "id": 1
    })
    .to_string();

    match post(&url, "POST", payload.as_bytes(), &headers) {
        Some(response_bytes) => {
            let json: serde_json::Value =
                serde_json::from_slice(&response_bytes).unwrap_or_default();
            let result_hex = json["result"].as_str().unwrap_or("0x");
            parse_chainlink_price_from_result(result_hex)
        }
        None => -1.0,
    }
}

fn parse_chainlink_price_from_result(result_hex: &str) -> f64 {
    if result_hex == "0x" {
        return -1.0;
    }

    let hex = result_hex.trim_start_matches("0x");
    if hex.len() < 64 * 5 {
        return -1.0;
    }

    // The "answer" starts at byte offset 32 -> hex chars offset 64
    let answer_hex = &hex[64..128];

    // Parse as i256 (signed 256-bit int, because answer is int256)
    let answer_bytes = hex::decode(answer_hex).unwrap_or(vec![]);
    if answer_bytes.len() != 32 {
        return -1.0;
    }

    // Handle signed big-endian 256-bit integer
    use num_bigint::BigInt;
    use num_bigint::Sign;
    use num_traits::ToPrimitive;

    let bigint = BigInt::from_signed_bytes_be(&answer_bytes);
    let price = bigint.to_f64().unwrap_or(-1.0);

    price
}

fn list_available_tools(req: &MCPRequest) -> MCPResponse {
    build_success(
        req,
        "Available tools: getLatestPrice, queryPriceByRound, listSupportedChains, listSupportedFeedsByChain, listSupportedFeeds",
    )
}

k3_wasm_macros::init!();
