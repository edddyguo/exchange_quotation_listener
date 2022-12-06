#![feature(slice_take)]
extern crate core;

mod filters;
mod constant;

use std::collections::HashMap;
use std::ops::Div;
use serde::{Deserialize, Serialize};
use crate::filters::{Root};
use chrono::prelude::*;
use crate::constant::PERP_MARKET;


//15分钟粒度，价格上涨百分之1，量上涨10倍（暂时5倍）可以触发预警
//监控所有开了永续合约的交易对

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

#[derive(Debug, Serialize, Deserialize)]
struct RateLimits{
    rateLimitType: String,
    interval: String,
    intervalNum: u8,
    limit: u32,
}



//仅仅使用usdt交易对
async fn get_all_market() -> Vec<String>{
   let line_data = reqwest::get("https://api.binance.com/api/v3/exchangeInfo")
        .await.unwrap()
        .json::<Root>()
        .await.unwrap();
    let des_market = line_data.symbols
        .iter()
        .filter(|x| x.symbol.contains("USDT"))
        .filter(|x| x.is_margin_trading_allowed == true)
        //过滤永续
        //.filter(|x| !x.permissions.iter().find(|&x| x == "TRD_GRP_005").is_some())
        .map(|x| x.symbol.clone())
        .collect::<Vec<String>>();
    println!("line_data {}", des_market.len());
    des_market
}

pub fn get_unix_timestamp_ms() -> i64 {
    let now = Utc::now();
    now.timestamp_millis()
}

async fn try_get(kline_url: String) -> Vec<Kline>{
    let mut line_data;
    loop {
        match  reqwest::get(&kline_url).await {
            Ok(res) => {
                line_data = res.json::<Vec<Kline>>().await.unwrap();
                break;
            }
            Err(error) => {
                println!("Happened error {}",error.to_string())
            }
        }
    }
    line_data
}

//binance-doc: https://binance-docs.github.io/apidocs/spot/en/#public-api-definitions
//策略：1h的k线，涨幅百分之1，量增加2倍
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    //https://api.binance.com/api/v3/avgPrice?symbol=BNBUSDT
    //let markets = get_all_market().await;
    //let markets = PERP_MARKET;
    loop{
        println!("data_0001 {}",get_unix_timestamp_ms());
        for (index,&market) in PERP_MARKET.iter().enumerate() {
            let kline_url = format!("https://api.binance.com/api/v3/klines?symbol={}&interval=30m&limit=5",market);
            let line_data = try_get(kline_url).await;
            println!("index {},market {}", index,market);
            //大于前4个总和
            let recent_klines = line_data.as_slice().take(..4).unwrap();
            let recent_volume = recent_klines.iter()
                .map(|x| x.volume.parse::<f32>().unwrap())
                .sum::<f32>()
                .div(4.0f32)
                ;

            let recent_price = recent_klines.iter()
                .map(|x| x.close_price.parse::<f32>().unwrap())
                .sum::<f32>()
                .div(4.0f32);

            println!("recent_price {} ,recent_volume {}",recent_price,recent_volume);

            //let last_close_price = line_data[0].close_price.parse::<f32>().unwrap();
            //let last_volume =  line_data[0].volume.parse::<f32>().unwrap();
            let current_price = line_data[4].close_price.parse::<f32>().unwrap();
            let current_volume =  line_data[4].volume.parse::<f32>().unwrap();

            let increase_price = (current_price - recent_price).div(recent_price);
            let increase_volume = (current_volume - recent_volume).div(recent_volume);
            println!("increase_price {},increase_volume {},current_price {},current_volume {}",increase_price,increase_volume,current_price,current_volume);
            //listen increase or reduce 1% and 6% volume
            if (increase_price > 0.01 || increase_price < -0.01)  && increase_volume > 6.0 {
                let pushed_msg = format!("Find market {}, price increase {},volume increase {}",
                                         market,increase_price,increase_volume
                );
                //println!("increase_ratio {},increase_volume {}",increase_price,increase_volume);
                let data = Msg{
                    msg_type: "text".to_string(),
                    content: Text {
                        text: pushed_msg
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
                println!("{:#?}", res.status());
            }

        }
        println!("data_0002 {}",get_unix_timestamp_ms());
    }


    Ok(())
}