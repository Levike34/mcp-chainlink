use k3_wasm_sdk::http::Response;

use serde::Deserialize;

/// retrieves aruguments from input string and matched type T
pub fn parse_args<'a, T>(json_str: &'a str) -> Result<T, Response<Vec<u8>>>
where
    T: Deserialize<'a> + std::fmt::Debug,
{
    match serde_json::from_str(&json_str) {
        Ok(parsed) => Ok(parsed),
        Err(err) => {
            let error_message = format!("Failed to parse JSON: {}", err);
            eprintln!("{}", &error_message);
            return Err(Response::builder()
                .status(400)
                .header("Content-Type", "text/plain")
                .body(error_message.into_bytes())
                .unwrap());
        }
    }
}
