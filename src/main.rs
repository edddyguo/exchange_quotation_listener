#![feature(slice_take)]
extern crate core;
#[macro_use]
extern crate log;

mod account;
mod bar;
mod constant;
mod ex_info;
mod filters;
mod history_data;
mod kline;
mod order;
mod strategy;
mod utils;
mod strategy2;
mod strategy3;

use crate::account::get_usdt_balance;
use crate::bar::{
    get_huge_volume_bar_num, get_last_bar_shape_score, get_last_bar_volume_score, get_raise_bar_num,
};
use crate::constant::{
    BROKEN_UP_INTERVALS, INCREASE_PRICE_LEVEL1, INCREASE_PRICE_LEVEL2, INCREASE_VOLUME_LEVEL1,
    INCREASE_VOLUME_LEVEL2, KLINE_NUM_FOR_FIND_SIGNAL,
};
use crate::ex_info::{list_all_pair, Symbol};
use crate::filters::Root;
use crate::history_data::{download_history_data, load_history_data, load_history_data_by_pair};
use crate::kline::{get_average_info, get_current_price, recent_kline_shape_score};
use crate::order::take_order;
use crate::utils::{get_unix_timestamp_ms, timestamp2date, MathOperation, MathOperation2};
use chrono::prelude::*;
use clap::{App, ArgMatches};
use log::{debug, error, info, log_enabled, Level};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;
use std::ops::{Div, Mul, Sub};

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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Kline {
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

//symbol -> order time,price,amount
#[derive(Debug, Serialize)]
pub struct TakeOrderInfo {
    take_time: u64,
    //如果没下单，则以30分钟内尝试检测是否再次拉升
    price: f32,
    amount: f32,
    top_bar: Kline,
    is_took: bool, //是否已经下单
}

//todo: 不只是kline，用泛型弄
async fn try_get(kline_url: String) -> Vec<Kline> {
    let mut line_data;
    loop {
        match reqwest::get(&kline_url).await {
            Ok(res) => {
                //println!("url {},res {:?}", kline_url,res);
                let res_str = format!("{:?}", res);
                match res.json::<Vec<Kline>>().await {
                    Ok(data) => {
                        line_data = data;
                        break;
                    }
                    Err(error) => {
                        //println!("reqwest res string: {:?}",res_str);
                        warn!(
                            "res deserialize happened error {},and raw res {}",
                            error.to_string(),
                            res_str
                        );
                    }
                }
            }
            Err(error) => {
                warn!("reqwest get happened error {}", error.to_string());
            }
        }
        std::thread::sleep(std::time::Duration::from_secs_f32(1.0));
    }
    line_data
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
            //"https://open.larksuite.com/open-apis/bot/v2/hook/83874fa0-1316-4cc2-8e88-7f8fd9d5d5e9",
            "https://open.larksuite.com/open-apis/bot/v2/hook/f1011068-13f1-4258-a98d-3c65a0449bb0",
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

pub async fn excute_real_trading() {
    let all_pairs = list_all_pair().await;
    let balance = get_usdt_balance().await;
    let mut take_order_pair2: HashMap<String, TakeOrderInfo> = HashMap::new();
    loop {
        for (index, pair) in all_pairs.clone().into_iter().enumerate() {
            let kline_url = format!(
                "https://api.binance.com/api/v3/klines?symbol={}&interval=1m&limit={}",
                pair.symbol.as_str(),
                KLINE_NUM_FOR_FIND_SIGNAL
            );
            let line_datas = try_get(kline_url).await;
            let now = get_unix_timestamp_ms() as u64;
            //todo: 目前人工维护已下单数据，后期考虑链上获取
            match strategy::buy(
                &mut take_order_pair2,
                pair.symbol.as_str(),
                &line_datas,
                true,
            )
                .await
            {
                Ok((true, _)) => {
                    continue;
                }
                Ok((false, _)) => {}
                Err(_) => {}
            }
            let _ = strategy::sell(&mut take_order_pair2, &line_datas, &pair, balance, true).await;
            //todo: wait util next kline generate
        }
        info!("complete listen all pairs");
        //保证每次顶多一次下单、平仓
        //todo: 可以更精确的堵塞，等待当前k线结束
        std::thread::sleep(std::time::Duration::from_secs_f32(26.0));
    }
}

pub async fn execute_back_testing(history_data: HashMap<Symbol, Vec<Kline>>) -> (f32, u32) {
    let balance = 10.0;
    let mut txs = 0u32;
    let mut take_order_pair: HashMap<String, TakeOrderInfo> = HashMap::new();
    let mut total_profit = 0.0;
    for (pair, klines) in history_data {
        warn!("start test {}", pair.symbol.as_str());
        let mut index = 0;
        for bar in &klines[359..] {
            let line_datas = &klines[index..(index + 360)];
            index += 1;
            assert_eq!(bar.open_time, line_datas[359].open_time);
            match strategy3::buy(
                &mut take_order_pair,
                pair.symbol.as_str(),
                &line_datas,
                false,
            )
                .await
            {
                Ok((true, profit)) => {
                    total_profit += profit;
                    if profit != 0.0 {
                        total_profit -= 0.0008;
                        txs += 2;
                    }
                    continue;
                }
                Ok((false, _)) => {}
                Err(error) => { warn!("{}",error.to_string()) }
            }
            let _ = strategy3::sell(&mut take_order_pair, &line_datas, &pair, balance, false).await;
            if index >= 50000 {
                break;
            }
        }
        //test one symbol
        //break;
        warn!("total_profit {},total txs {}",total_profit,txs);
    }
    return (total_profit, txs);
}

pub async fn execute_back_testing2(month: u8) -> (f32, u32){
    let balance = 10.0;
    let mut txs = 0u32;
    let mut take_order_pair: HashMap<String, TakeOrderInfo> = HashMap::new();
    let mut total_profit = 0.0;
    let all_pairs = list_all_pair().await;
    for pair in all_pairs.iter() {
        warn!("start test {}", pair.symbol.as_str());
        let klines = load_history_data_by_pair(&pair.symbol, month).await;
        if klines.is_empty(){
            continue;
        }
        let mut index = 0;
        for bar in &klines[359..] {
            let line_datas = &klines[index..(index + 360)];
            index += 1;
            assert_eq!(bar.open_time, line_datas[359].open_time);
            match strategy2::buy(
                &mut take_order_pair,
                pair.symbol.as_str(),
                &line_datas,
                false,
            )
                .await
            {
                Ok((true, profit)) => {
                    total_profit += profit;
                    if profit != 0.0 {
                        total_profit -= 0.0008;
                        txs += 2;
                    }
                    continue;
                }
                Ok((false, _)) => {}
                Err(error) => { warn!("{}",error.to_string()) }
            }
            let _ = strategy2::sell(&mut take_order_pair, &line_datas, &pair, balance, false).await;
            if index >= 50000 {
                break;
            }
        }
        //test one symbol
        //break;
        warn!("total_profit {},total txs {}",total_profit,txs);
    }
    return (total_profit, txs);
}

//binance-doc: https://binance-docs.github.io/apidocs/spot/en/#public-api-definitions
//策略：1h的k线，涨幅百分之1，量增加2倍
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", timestamp2date(get_unix_timestamp_ms() as u64));
    env_logger::init();
    let matches = App::new("bot")
        .version("1.0")
        .about("Does awesome things")
        .subcommand(App::new("real_trading"))
        .subcommand(App::new("back_testing"))
        .subcommand(App::new("back_testing2"))
        .subcommand(App::new("download_history_kline"))
        .get_matches();
    match matches.subcommand() {
        Some(("real_trading", _sub_matches)) => {
            println!("real_trading");
            //excute_real_trading().await;
        }
        Some(("back_testing", _sub_matches)) => {
            println!("back_testing");
            for month in 1..=12 {
                let history_data = load_history_data(month).await;
                let (total_profit, txs) = execute_back_testing(history_data).await;
                warn!("month {} total_profit {},total txs {}",month,total_profit,txs);
            }
        }
        Some(("back_testing2", _sub_matches)) => {
            println!("back_testing2");
            for month in 1..=1 {
                let (total_profit, txs) = execute_back_testing2(month).await;
                warn!("month {} total_profit {},total txs {}",month,total_profit,txs);            }
        }
        Some(("download_history_kline", _sub_matches)) => {
            println!("download_history_kline");
            download_history_data().await
        }
        _ => {}
    }
    Ok(())
}
