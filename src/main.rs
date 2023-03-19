#![feature(slice_take)]
#![feature(async_fn_in_trait)]
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
mod strategy2;
mod strategy3;
mod utils;

use crate::account::get_usdt_balance;
use crate::bar::{
    get_huge_volume_bar_num, get_last_bar_shape_score, get_last_bar_volume_score, get_raise_bar_num,
};
use crate::constant::{
    BROKEN_UP_INTERVALS, INCREASE_PRICE_LEVEL1, INCREASE_PRICE_LEVEL2, INCREASE_VOLUME_LEVEL1,
    INCREASE_VOLUME_LEVEL2, KLINE_NUM_FOR_FIND_SIGNAL,
};
use crate::strategy::sell::SellReason;

use crate::ex_info::{list_all_pair, Symbol};
use crate::filters::Root;
use crate::history_data::{download_history_data, load_history_data, load_history_data_by_pair};
use crate::kline::{get_average_info, get_current_price, recent_kline_shape_score};
use crate::order::take_order;
use crate::strategy::sell::SellReason::{
    AStrongSignal, RaiseIsStop, ThreeContinuousSignal, TwoMiddleSignal,
};
use crate::strategy::sell::TakeType;
use crate::utils::{get_unix_timestamp_ms, timestamp2date, MathOperation, MathOperation2};
use chrono::prelude::*;
use clap::{App, ArgMatches};
use log::{debug, error, info, log_enabled, Level};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;
use std::ops::{Deref, Div, Mul, Sub};
use std::sync::{Arc, RwLock};
use tokio::runtime::Runtime;
use crate::SellReason::AVeryStrongSignal;

//15分钟粒度，价格上涨百分之1，量上涨10倍（暂时5倍）可以触发预警
//监控所有开了永续合约的交易对

type Pair = String;

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

#[derive(Debug)]
pub struct StrategyEffect{
    sell_reason:String,
    txs:u32,
    win_txs:u32,
    lose_txs:u32,
    total_profit:f32,
    win_ratio:f32,
}

impl StrategyEffect{
    pub fn new(sell_reason:SellReason) -> Self{
        let reason_str:&str = sell_reason.into();
        StrategyEffect{
            sell_reason:reason_str.to_owned(),
            txs: 0,
            win_txs: 0,
            lose_txs: 0,
            total_profit: 0.0,
            win_ratio: 0.0
        }
    }
}

//todo: 不只是kline，用泛型弄
async fn try_get<DATA_TYPE: for<'a> Deserialize<'a>>(kline_url: String) -> Box<DATA_TYPE> {
    let mut line_data;
    loop {
        match reqwest::get(&kline_url).await {
            Ok(res) => {
                //println!("url {},res {:?}", kline_url,res);
                let res_str = format!("{:?}", res);
                match res.json::<DATA_TYPE>().await {
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
                        std::thread::sleep(std::time::Duration::from_secs_f32(1.0));
                    }
                }
            }
            Err(error) => {
                warn!("reqwest get happened error {}", error.to_string());
                std::thread::sleep(std::time::Duration::from_secs_f32(1.0));
            }
        }
    }
    Box::new(line_data)
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
    let mut take_order_pair: HashMap<TakeType, Vec<TakeOrderInfo>> = HashMap::new();
    loop {
        let balance = get_usdt_balance().await;
        for (index, pair) in all_pairs.clone().into_iter().enumerate() {
            let kline_url = format!(
                "https://api.binance.com/api/v3/klines?symbol={}&interval=1m&limit={}",
                pair.symbol.as_str(),
                KLINE_NUM_FOR_FIND_SIGNAL
            );
            let line_datas = try_get::<Vec<Kline>>(kline_url).await.to_vec();
            let mut all_reason_total_profit: Vec<StrategyEffect> =
                vec![StrategyEffect::new(AStrongSignal),
                     StrategyEffect::new(TwoMiddleSignal),
                     StrategyEffect::new(ThreeContinuousSignal),
                     StrategyEffect::new(AVeryStrongSignal)
                ];
            for effect in all_reason_total_profit {
                let taker_type = TakeType {
                    pair: pair.symbol.clone(),
                    sell_reason: SellReason::from(effect.sell_reason.as_str()),
                };
                let _ = strategy::buy(&mut take_order_pair, taker_type, &line_datas, true, ).await.unwrap();

            }
            let _ = strategy::sell(&mut take_order_pair, &line_datas, &pair, balance, true).await;
        }
        //严格等待到下一分钟
        let distance_next_minute_time = 60000 - get_unix_timestamp_ms() % 60000;
        std::thread::sleep(std::time::Duration::from_millis(distance_next_minute_time as u64 + 1000u64));
        warn!("complete listen all pairs,and start next minute");
    }
}

pub async fn execute_back_testing(
    history_data: HashMap<Symbol, Vec<Kline>>,
    month: u8,
) -> Vec<(SellReason, f32, u32)> {
    let balance = 10.0;
    let mut take_order_pair: HashMap<TakeType, Vec<TakeOrderInfo>> = HashMap::new();
    ///reason,total_profit,txs
    let mut all_reason_total_profit: Vec<(SellReason, f32, u32)> =
        vec![(AStrongSignal, 0.0, 0), (TwoMiddleSignal, 0.0, 0)];
    //let mut all_reason_total_profit: Vec<(SellReason, f32,u32)> = vec![(AStrongSignal, 0.0,0)];
    let eth_klines = load_history_data_by_pair("ETHUSDT", month).await;
    for (pair, klines) in history_data {
        warn!(
            "start test {},klines size {}",
            pair.symbol.as_str(),
            klines.len()
        );
        let mut index = 0;
        for bar in &klines[359..] {
            let line_datas = &klines[index..(index + 360)];
            index += 1;
            assert_eq!(bar.open_time, line_datas[359].open_time);
            for (reason, total_profit, txs) in all_reason_total_profit.iter_mut() {
                let take_type = TakeType {
                    pair: pair.symbol.clone(),
                    sell_reason: reason.clone(),
                };
                // fixme：_is_took 是否已经不需要了？
                let (_is_took, profit) =
                    strategy::buy(&mut take_order_pair, take_type, &line_datas, false)
                        .await
                        .unwrap();
                *total_profit += profit;
                //只有下了卖单和买单的才统计收益
                if profit != 0.0 {
                    *total_profit -= 0.0008;
                    *txs += 2;
                    info!(
                        "all_reason_total_profit total_profit {} txs {}",
                        *total_profit, *txs
                    );
                }
                //当前reason下：0、还没加入观察列表，1、还没开始下卖单，2、已经下卖单但不符合平仓条件
                //无论是否下单，都继续sell筛选，sell里面保证没有重复下单
                /* if is_took {
                    continue;
                }*/
            }

            //避开eth的强势时间的信号
            if eth_klines[index + 350].open_price.to_f32() / eth_klines[index].open_price.to_f32()
                > 1.03
            {
                continue;
            }

            let _ = strategy::sell(&mut take_order_pair, &line_datas, &pair, balance, false).await;
            if index >= 50000 {
                break;
            }
        }
    }
    return all_reason_total_profit;
}

pub async fn execute_back_testing2(month: u8) -> Vec<StrategyEffect> {
    let balance = 10.0;
    let mut take_order_pair: HashMap<TakeType, Vec<TakeOrderInfo>> = HashMap::new();
    ///reason,total_profit,txs
    let mut all_reason_total_profit: Vec<StrategyEffect> =
        vec![StrategyEffect::new(AStrongSignal),
             StrategyEffect::new(TwoMiddleSignal),
             StrategyEffect::new(ThreeContinuousSignal),
             StrategyEffect::new(AVeryStrongSignal)
        ];
    //let mut all_reason_total_profit: Vec<(SellReason, f32,u32)> = vec![(AStrongSignal, 0.0,0)];
    let all_pairs = list_all_pair().await;
    let eth_klines = load_history_data_by_pair("ETHUSDT", month).await;
    for pair in all_pairs.iter() {
        warn!("start test {}", pair.symbol.as_str());
        let klines = load_history_data_by_pair(&pair.symbol, month).await;
        if klines.is_empty() {
            continue;
        }
        let mut index = 0;
        for bar in &klines[359..] {
            let line_datas = &klines[index..(index + 360)];
            index += 1;

            assert_eq!(bar.open_time, line_datas[359].open_time);
            for effect in all_reason_total_profit.iter_mut() {
                let take_type = TakeType {
                    pair: pair.symbol.clone(),
                    sell_reason: SellReason::from(effect.sell_reason.as_str()),
                };
                // fixme：_is_took 是否已经不需要了？
                let (_is_took, profit) =
                    strategy::buy(&mut take_order_pair, take_type, &line_datas, false)
                        .await
                        .unwrap();
                effect.total_profit += profit;
                //只有下了卖单和买单的才统计收益
                if profit != 0.0 {
                    effect.total_profit -= 0.0008;
                    effect.txs += 1;
                    if profit > 0.0 {
                        effect.win_txs += 1;
                    }else {
                        effect.lose_txs += 1;
                    }
                    info!("tmp:month {} ,detail {:?}",month,effect);
                }
                //当前reason下：0、还没加入观察列表，1、还没开始下卖单，2、已经下卖单但不符合平仓条件
                //无论是否下单，都继续sell筛选，sell里面保证没有重复下单
                /* if is_took {
                    continue;
                }*/
            }

            /***
            if eth_klines[index + 350].open_price.to_f32() / eth_klines[index].open_price.to_f32() > 1.03 {
                continue;
            }
             */

            let _ = strategy::sell(&mut take_order_pair, &line_datas, &pair, balance, false).await;
        }
    }
    return all_reason_total_profit;
}

/*pub async fn execute_back_testing3(month: u8) -> Vec<(SellReason, f32, u32)> {
    let balance = 10.0;
    let mut take_order_pair: HashMap<TakeType, Vec<TakeOrderInfo>> = HashMap::new();
    ///reason,total_profit,txs
    let mut all_reason_total_profit: Vec<(SellReason, f32, u32)> = vec![(AStrongSignal, 0.0, 0), (TwoMiddleSignal, 0.0, 0)];
    let all_reason_total_profit = Arc::new(RwLock::new(all_reason_total_profit));
    //let mut all_reason_total_profit: Vec<(SellReason, f32,u32)> = vec![(AStrongSignal, 0.0,0)];
    let all_pairs = list_all_pair().await;
    let eth_klines = load_history_data_by_pair("ETHUSDT", month).await;
    rayon::scope(|scope| {
        let all_reason_total_profit = all_reason_total_profit.clone();
        let eth_klines = eth_klines.clone();
        scope.spawn(move |_| {
            'loop_pair:for pair in all_pairs.iter() {
                let rt = Runtime::new().unwrap();
                rt.block_on(async move {
                    let klines = load_history_data_by_pair(&pair.symbol, month).await;
                    if klines.is_empty() {
                        return;
                    }
                    let mut index = 0;
                    for bar in &klines[359..] {
                        let line_datas = &klines[index..(index + 360)];
                        index += 1;
                        if eth_klines[index + 350].open_price.to_f32() / eth_klines[index].open_price.to_f32() > 1.03 {
                            continue;
                        }
                        assert_eq!(bar.open_time, line_datas[359].open_time);
                        for (reason, total_profit, txs) in all_reason_total_profit.clone().write().unwrap().iter_mut() {
                            let take_type = TakeType {
                                pair: pair.symbol.clone(),
                                sell_reason: reason.clone(),
                            };
                            // fixme：_is_took 是否已经不需要了？
                            let (_is_took, profit) = strategy::buy(
                                &mut take_order_pair,
                                take_type,
                                &line_datas,
                                false,
                            ).await.unwrap();
                            *total_profit += profit;
                            //只有下了卖单和买单的才统计收益
                            if profit != 0.0 {
                                *total_profit -= 0.0008;
                                *txs += 2;
                                info!("all_reason_total_profit total_profit {} txs {}",*total_profit,*txs);
                            }
                            //当前reason下：0、还没加入观察列表，1、还没开始下卖单，2、已经下卖单但不符合平仓条件
                            //无论是否下单，都继续sell筛选，sell里面保证没有重复下单
                            /* if is_took {
                                 continue;
                             }*/
                        }

                        let _ = strategy::sell(&mut take_order_pair, &line_datas, &pair, balance, false).await;
                    }
                });
            }
        });
    });

    return all_reason_total_profit.read().unwrap().deref().to_vec();
}*/

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
        .subcommand(App::new("back_testing3"))
        .subcommand(App::new("download_history_kline"))
        .get_matches();
    match matches.subcommand() {
        Some(("real_trading", _sub_matches)) => {
            println!("real_trading");
            excute_real_trading().await;
        }
        Some(("back_testing", _sub_matches)) => {
            println!("back_testing");
            for month in 1..=12 {
                let history_data = load_history_data(month).await;
                let datas = execute_back_testing(history_data, month).await;
                for (reason, total_profit, txs) in datas {
                    let reason_str:&str = reason.into();
                    warn!(
                        "month {},reason {}, total_profit {},total txs {}",
                        month,
                        reason_str,
                        total_profit,
                        txs
                    );
                }
            }
        }
        Some(("back_testing2", _sub_matches)) => {
            println!("back_testing2");
            for month in 1..=12 {
                let datas = execute_back_testing2(month).await;
                for data in datas {
                    warn!("finally: month {},detail {:?}",month,data);
                }
            }
        }
        Some(("back_testing3", _sub_matches)) => {
            println!("back_testing3");
            rayon::scope(|scope| {
                for month in 1..=12 {
                    scope.spawn(move |_| {
                        let rt = Runtime::new().unwrap();
                        rt.block_on(async move {
                            let datas = execute_back_testing2(month).await;
                            for data in datas {
                                warn!("finally: month {},detail {:?}",month,data);
                            }
                        });
                    });
                }
            });
        }
        Some(("download_history_kline", _sub_matches)) => {
            println!("download_history_kline");
            download_history_data().await
        }
        _ => {}
    }
    Ok(())
}
