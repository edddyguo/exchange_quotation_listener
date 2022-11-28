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
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /***
     data = {
        "msg_type": "text",
        "content": {
            "text": "你好"
        }
    }
    */
    //https://api.binance.com/api/v3/avgPrice?symbol=BNBUSDT
    let resp = reqwest::get("https://api.binance.com/api/v3/avgPrice?symbol=BNBUSDT")
        .await?
        .json::<AvgPrice>()
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