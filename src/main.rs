use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct AvgPrice {
    mins: u32,
    price: String
}

#[derive(Debug, Serialize, Deserialize)]
struct Text {
    text: String,
}
#[derive(Debug, Serialize, Deserialize)]
struct Msg {
    msg_type: String,
    content: Text,
}

/***
[
  [
    1499040000000,      // Kline open time
    "0.01634790",       // Open price
    "0.80000000",       // High price
    "0.01575800",       // Low price
    "0.01577100",       // Close price
    "148976.11427815",  // Volume
    1499644799999,      // Kline Close time
    "2434.19055334",    // Quote asset volume
    308,                // Number of trades
    "1756.87402397",    // Taker buy base asset volume
    "28.46694368",      // Taker buy quote asset volume
    "0"                 // Unused field, ignore.
  ]
]
 */

#[derive(Debug, Serialize, Deserialize)]
struct Kline {
    open_time: u64,
    open_price: String,
    high_price:String,
    low_price:String,
    close_price:String,
    volume:String,
    kline_close_time:u64,
    quote_asset_volume:String,
    number_of_trades:u32,
    taker_buy_base_asset_volume:String,
    taker_buy_quote_asset_volume:String,
    unused_field:String,
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    //https://api.binance.com/api/v3/avgPrice?symbol=BNBUSDT
    let resp = reqwest::get("https://api.binance.com/api/v3/klines?symbol=BNBUSDT&interval=5m&limit=1")
        .await?
        .json::<Vec<Kline>>()
        .await?;
    println!("{:#?}", resp);

    let data = Msg{
        msg_type: "text".to_string(),
        content: Text {
            text: "11".to_string()
        }
    };
    let client = reqwest::Client::new();
    let res = client.post("https://open.larksuite.com/open-apis/bot/v2/hook/56188918-b6b5-4029-9fdf-8a45a86d06a3")
        .json(&data)
        .header( "Content-type","application/json")
        .header("charset","utf-8")
        .send()
        .await?;
    //send to lark
    println!("{:#?}", res);
    Ok(())
}