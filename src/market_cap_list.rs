use crate::constant::{BNB_API_KEY, RECV_WINDOW};
use crate::utils::hmac_sha256_sign;
use crate::{get_unix_timestamp_ms, try_get};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue, CONTENT_TYPE};
///获取u本位账号基本信息（可用余额）
///
///
/// curl -H "X-CMC_PRO_API_KEY: 92e6f509-4ef9-4ee1-8e54-4e3a732df2e9" -H "Accept: application/json"  -d "limit=150" -G https://pro-api.coinmarketcap.com/v1/cryptocurrency/listings/latest|jq|grep symbol//use serde_derive::Deserialize;
//use serde_derive::Serialize;
use serde::{Deserialize, Serialize};
use std::ops::Deref;

//use serde_derive::Deserialize;
//use serde_derive::Serialize;
//use serde_json::Value;
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Root {
    pub status: Status,
    pub data: Vec<Daum>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub timestamp: String,
    #[serde(rename = "error_code")]
    pub error_code: i64,
    #[serde(rename = "error_message")]
    pub error_message: Option<String>,
    pub elapsed: i64,
    #[serde(rename = "credit_count")]
    pub credit_count: i64,
    pub notice: Option<String>,
    #[serde(rename = "total_count")]
    pub total_count: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Daum {
    pub id: i64,
    pub name: String,
    pub symbol: String,
    pub slug: String,
    #[serde(rename = "num_market_pairs")]
    pub num_market_pairs: i64,
    #[serde(rename = "date_added")]
    pub date_added: String,
    pub tags: Vec<String>,
    #[serde(rename = "max_supply")]
    pub max_supply: Option<i64>,
    #[serde(rename = "circulating_supply")]
    pub circulating_supply: f64,
    #[serde(rename = "total_supply")]
    pub total_supply: f64,
    #[serde(rename = "infinite_supply")]
    pub infinite_supply: bool,
    pub platform: Option<Platform>,
    #[serde(rename = "cmc_rank")]
    pub cmc_rank: i64,
    #[serde(rename = "self_reported_circulating_supply")]
    pub self_reported_circulating_supply: Option<i64>,
    #[serde(rename = "self_reported_market_cap")]
    pub self_reported_market_cap: Option<f64>,
    #[serde(rename = "tvl_ratio")]
    pub tvl_ratio:Option<f64>,
    #[serde(rename = "last_updated")]
    pub last_updated: String,
    pub quote: Quote,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Platform {
    pub id: i64,
    pub name: String,
    pub symbol: String,
    pub slug: String,
    #[serde(rename = "token_address")]
    pub token_address: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Quote {
    #[serde(rename = "USD")]
    pub usd: Usd,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Usd {
    pub price: f64,
    #[serde(rename = "volume_24h")]
    pub volume_24h: f64,
    #[serde(rename = "volume_change_24h")]
    pub volume_change_24h: f64,
    #[serde(rename = "percent_change_1h")]
    pub percent_change_1h: f64,
    #[serde(rename = "percent_change_24h")]
    pub percent_change_24h: f64,
    #[serde(rename = "percent_change_7d")]
    pub percent_change_7d: f64,
    #[serde(rename = "percent_change_30d")]
    pub percent_change_30d: f64,
    #[serde(rename = "percent_change_60d")]
    pub percent_change_60d: f64,
    #[serde(rename = "percent_change_90d")]
    pub percent_change_90d: f64,
    #[serde(rename = "market_cap")]
    pub market_cap: f64,
    #[serde(rename = "market_cap_dominance")]
    pub market_cap_dominance: f64,
    #[serde(rename = "fully_diluted_market_cap")]
    pub fully_diluted_market_cap: f64,
    pub tvl: Option<f64>,
    #[serde(rename = "last_updated")]
    pub last_updated: String,
}


pub async fn market_cap_list(limit: u32) -> Vec<String> {
    let mut headers = HeaderMap::new();
    headers.insert(
        HeaderName::from_static("x-cmc_pro_api_key"),
        HeaderValue::from_static("92e6f509-4ef9-4ee1-8e54-4e3a732df2e9"),
    );

    let request_parameter = format!("limit={}", limit);
    let url = format!(
        "https://pro-api.coinmarketcap.com/v1/cryptocurrency/listings/latest?{}",
        request_parameter
    );
    let client = reqwest::Client::new();
    let res = client
        .get(url)
        .headers(headers)
        .send()
        .await
        .unwrap()
        .json::<Root>()
        .await
        .unwrap();
    res.data.iter().map(|x| format!("{}USDT",x.symbol)).collect()
}
