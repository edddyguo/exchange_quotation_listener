use hmac::digest::core_api::TruncSide;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use crate::constant::{BNB_API_KEY, RECV_WINDOW};
use crate::get_unix_timestamp_ms;
use crate::utils::hmac_sha256_sign;

///taker order or cancel
///
/// proxychains4 curl -H "X-MBX-APIKEY: HHjYprQmyfp7JWqChuNiNyd32JEtD16M10mL9LhnU79fq38Wk75NU3rzu9m0KyTq" -X
/// POST 'https://fapi.binance.com/fapi/v1/order?
/// symbol=BTCUSDT&side=BUY&type=LIMIT&quantity=0.001&price=9000&timeInForce=GTC&recvWindow=5000000&timestamp=1674862278000
/// &signature=f65a9f4dfc1b6cb697380471a25c4862d013ba3e804b2dc39858af5efffab194'
pub enum Side {
    Sell,
    Buy
}

// todo:暂时只考虑空单市场价成交，后续可丰富
pub async fn take_order(symbol: String,amount:f32){
    let mut headers = HeaderMap::new();
    headers.insert(HeaderName::from_static("x-mbx-apikey"), HeaderValue::from_static(BNB_API_KEY));

    let request_parameter = format!("symbol={}&side=SELL&type=MARKET&quantity={}&recvWindow={}&timestamp={}",symbol,amount,RECV_WINDOW,get_unix_timestamp_ms());
    let signature = hmac_sha256_sign(&request_parameter);
    //https://fapi.binance.com/fapi/v1/order
    let url = format!("https://fapi.binance.com/fapi/v1/order?{}&signature={}", request_parameter,signature);

    let client = reqwest::Client::new();
    let res = client.post(url)
        .headers(headers)
        .send()
        .await
        .unwrap();
    println!("take order result {:?}",res);
    //todo: 下单结果判断
}

#[cfg(test)]
mod tests {
    use std::ops::Div;
    use crate::get_usdt_balance;
    use crate::order::{Side, take_order};

    #[tokio::test]
    async fn test_take_order() {
        let amount = get_usdt_balance().await.div(20.0f32);
        take_order("RLCUSDT".to_string(), 5.0).await;
    }
}