#![feature(slice_take)]
extern crate core;

mod constant;
mod filters;

use crate::constant::PERP_MARKET;
use crate::filters::Root;
use chrono::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ops::Div;

//15分钟粒度，价格上涨百分之1，量上涨10倍（暂时5倍）可以触发预警
//监控所有开了永续合约的交易对

#[derive(Debug, Serialize, Deserialize)]
struct AvgPrice {
    mins: u32,
    price: String,
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
    high_price: String,
    low_price: String,
    close_price: String,
    volume: String,
    kline_close_time: u64,
    quote_asset_volume: String,
    number_of_trades: u32,
    taker_buy_base_asset_volume: String,
    taker_buy_quote_asset_volume: String,
    unused_field: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct RateLimits {
    rateLimitType: String,
    interval: String,
    intervalNum: u8,
    limit: u32,
}

//仅仅使用usdt交易对
async fn get_all_market() -> Vec<String> {
    let line_data = reqwest::get("https://api.binance.com/api/v3/exchangeInfo")
        .await
        .unwrap()
        .json::<Root>()
        .await
        .unwrap();
    let des_market = line_data
        .symbols
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

async fn try_get(kline_url: String) -> Vec<Kline> {
    let mut line_data;
    loop {
        match reqwest::get(&kline_url).await {
            Ok(res) => {
                line_data = res.json::<Vec<Kline>>().await.unwrap();
                break;
            }
            Err(error) => {
                println!("Happened error {}", error.to_string())
            }
        }
    }
    line_data
}

//判断是否是突破形态，根据30分钟k线是否巨量
async fn is_break_through_market(market: &str) -> (f32,f32) {
    let kline_url = format!(
        "https://api.binance.com/api/v3/klines?symbol={}&interval=30m&limit=20",
        market
    );
    let line_data = try_get(kline_url).await;
    //大于前4个总和
    let recent_klines = line_data.as_slice().take(..19).unwrap();
    let recent_volume = recent_klines
        .iter()
        .map(|x| x.volume.parse::<f32>().unwrap())
        .sum::<f32>()
        .div(19.0f32);

    let recent_price = recent_klines
        .iter()
        .map(|x| x.close_price.parse::<f32>().unwrap())
        .sum::<f32>()
        .div(19.0f32);

    println!(
        "recent_price {} ,recent_volume {}",
        recent_price, recent_volume
    );

    //let last_close_price = line_data[0].close_price.parse::<f32>().unwrap();
    //let last_volume =  line_data[0].volume.parse::<f32>().unwrap();
    let current_price = line_data[19].close_price.parse::<f32>().unwrap();
    let current_volume = line_data[19].volume.parse::<f32>().unwrap();

    let increase_price = (current_price - recent_price).div(recent_price);
    let increase_volume = (current_volume - recent_volume).div(recent_volume);
    println!(
        "increase_price {},increase_volume {},current_price {},current_volume {}",
        increase_price, increase_volume, current_price, current_volume
    );
    //listen increase 1% 6% volume
    /*if increase_price > 0.01 && increase_volume > 8.0 {
        return true;
    } else {
        false
    }*/
    (increase_price,increase_volume)
}

//推送消息给lark机器人
async fn notify_lark(pushed_msg: String) -> Result<(), Box<dyn std::error::Error>> {
    //println!("increase_ratio {},increase_volume {}",increase_price,increase_volume);
    let data = Msg {
        msg_type: "text".to_string(),
        content: Text { text: pushed_msg },
    };
    let client = reqwest::Client::new();
    let res = client
        .post(
            "https://open.larksuite.com/open-apis/bot/v2/hook/56188918-b6b5-4029-9fdf-8a45a86d06a3",
        )
        .json(&data)
        .header("Content-type", "application/json")
        .header("charset", "utf-8")
        .send()
        .await?;
    //send to lark
    println!("{:#?}", res.status());
    Ok(())
}

//判断是否五连阳
async fn is_many_increase_times(market: &str, limit: u8) -> bool {
    let kline_url = format!(
        "https://api.binance.com/api/v3/klines?symbol={}&interval=5m&limit={}",
        market,limit
    );
    let line_datas = try_get(kline_url).await;
    for (index, line_data) in line_datas.iter().enumerate() {
        if index > 0 && line_data.close_price < line_datas[index - 1].close_price {
            return false;
        }
    }
    true
}

//binance-doc: https://binance-docs.github.io/apidocs/spot/en/#public-api-definitions
//策略：1h的k线，涨幅百分之1，量增加2倍
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    //https://api.binance.com/api/v3/avgPrice?symbol=BNBUSDT
    //let markets = get_all_market().await;
    //let markets = PERP_MARKET;
    loop {
        println!("data_0001 {}", get_unix_timestamp_ms());
        for (index, &market) in PERP_MARKET.iter().enumerate() {
            println!("index {},market {}", index, market);
            //根据涨幅和量分为不同的信号强度
            /***
                信号级别           条件
                ***         : 2%价格涨幅 +  8倍交易量  + 5连涨
                **          :       介于* 和 *** 之间的情况 //todo
                *           : 1%价格涨幅 +  5倍交易量  + 3连涨
             */
            let (increase_price,increase_volume) = is_break_through_market(market).await;
            if increase_price > 0.02 && increase_volume > 8.0 && is_many_increase_times(market,5).await{
                    //notify_lark(market).await?
                let push_text = format!("捕捉到 *** 信号: market {},increase_price {},increase_volume {}",
                                    market,increase_price,increase_volume
                );
                notify_lark(push_text).await?
            }else if increase_price > 0.01 && increase_volume > 5.0 && is_many_increase_times(market,3).await{
                let push_text = format!("捕捉到 * 信号: market {},increase_price {},increase_volume {}",
                                    market,increase_price,increase_volume
                );
                notify_lark(push_text).await?
            }else {
                println!("Have no obvious break signal");
            }
        }
        println!("data_0002 {}", get_unix_timestamp_ms());
        std::thread::sleep(std::time::Duration::from_secs_f32(40.0));
    }
    Ok(())
}
