use crate::constant::{BNB_API_KEY, RECV_WINDOW};
use crate::get_unix_timestamp_ms;
use crate::utils::hmac_sha256_sign;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue, CONTENT_TYPE};
///获取u本位账号基本信息（可用余额）
///
///
/// proxychains4 curl -H "X-MBX-APIKEY: HHjYprQmyfp7JWqChuNiNyd32JEtD16M10mL9LhnU79fq38Wk75NU3rzu9m0KyTq" -X GET 'https://fapi.binance.com/fapi/v2/balance?recvWindow=500000&timestamp=1674871881000&signature=ae73d85fe9f161cce3bce0d3537341e66be7534b4f76c80057793e34635e284f' |jq
//use serde_derive::Deserialize;
//use serde_derive::Serialize;
use serde::{Deserialize, Serialize};

pub type Balances = Vec<Balance>;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Balance {
    pub account_alias: String,
    pub asset: String,
    pub balance: String,
    pub cross_wallet_balance: String,
    pub cross_un_pnl: String,
    pub available_balance: String,
    pub max_withdraw_amount: String,
    pub margin_available: bool,
    pub update_time: i64,
}

pub async fn get_usdt_balance() -> f32 {
    let mut headers = HeaderMap::new();
    headers.insert(
        HeaderName::from_static("x-mbx-apikey"),
        HeaderValue::from_static(BNB_API_KEY),
    );
    //todo: 对get的参数进行签名
    let request_parameter = format!(
        "recvWindow={}&timestamp={}",
        RECV_WINDOW,
        get_unix_timestamp_ms()
    );
    let signature = hmac_sha256_sign(&request_parameter);
    let url = format!(
        "https://fapi.binance.com/fapi/v2/balance?{}&signature={}",
        request_parameter, signature
    );
    //todo: 1、签名 2、curl -H
    let client = reqwest::Client::new();
    let line_data = client
        .get(url)
        .headers(headers)
        .send()
        .await
        .unwrap()
        .json::<Balances>()
        .await
        .unwrap();
    let balance_value = line_data
        .iter()
        .map(|x| x.available_balance.parse::<f32>().unwrap())
        .sum::<f32>();
    //println!("{:?}", balance_value);
    balance_value
}

#[cfg(test)]
mod tests {
    use crate::account::get_usdt_balance;

    #[tokio::test]
    async fn get_available_balance() {
        get_usdt_balance().await;
    }
}
