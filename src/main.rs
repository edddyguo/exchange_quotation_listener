#![feature(slice_take)]
#![feature(async_fn_in_trait)]
#![feature(core_intrinsics)]

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
mod draw;

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
use std::intrinsics::atomic_cxchg_release_seqcst;
use std::ops::{Deref, Div, Mul, Sub};
use std::sync::{Arc, RwLock};
use tokio::runtime::Runtime;
use crate::SellReason::{AStrongSignal_V2, AVeryStrongSignal, AVeryStrongSignal_V2, SequentialTakeOrder, StartGoDown, TwoMiddleSignal_V2};

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

impl Kline {
    pub fn is_raise(&self) -> bool {
        if self.open_price.to_f32() < self.close_price.to_f32() {
            true
        } else {
            false
        }
    }

    pub fn is_strong_raise(&self) -> bool {
        let diaowei_up_distance = self.high_price.to_f32() - self.close_price.to_f32();
        let diaowei_down_distance = self.close_price.to_f32() - self.low_price.to_f32();
        if self.close_price.to_f32() > self.open_price.to_f32()
            && diaowei_down_distance / diaowei_up_distance >= 5.0
        {
            true
        } else {
            false
        }
    }
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
    sell_price: f32,
    buy_price: Option<f32>,
    amount: f32,
    top_bar: Kline,
    is_took: bool, //是否已经下单
}

#[derive(Debug)]
pub struct StrategyEffect {
    sell_reason: String,
    txs: u32,
    win_txs: u32,
    lose_txs: u32,
    total_profit: f32,
    win_ratio: f32,
}

impl StrategyEffect {
    pub fn new(sell_reason: SellReason) -> Self {
        let reason_str: &str = sell_reason.into();
        StrategyEffect {
            sell_reason: reason_str.to_owned(),
            txs: 0,
            win_txs: 0,
            lose_txs: 0,
            total_profit: 0.0,
            win_ratio: 0.0,
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
    let mut times = 0u64;
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
                     //StrategyEffect::new(AStrongSignal_V2),
                     StrategyEffect::new(TwoMiddleSignal),
                     //StrategyEffect::new(TwoMiddleSignal_V2),
                     //StrategyEffect::new(ThreeContinuousSignal),
                     StrategyEffect::new(AVeryStrongSignal),
                     //StrategyEffect::new(AVeryStrongSignal_V2),
                ];
            for effect in all_reason_total_profit {
                let taker_type = TakeType {
                    pair: pair.symbol.clone(),
                    sell_reason: SellReason::from(effect.sell_reason.as_str()),
                };
                let _ = strategy::buy(&mut take_order_pair, taker_type, &line_datas, true).await.unwrap();
            }
            let _ = strategy::sell(&mut take_order_pair, &line_datas, &pair, balance, true).await;
        }
        //严格等待到下一分钟
        let distance_next_minute_time = 60000 - get_unix_timestamp_ms() % 60000;
        std::thread::sleep(std::time::Duration::from_millis(distance_next_minute_time as u64 + 1000u64));
        times += 1;
        if times % 30 == 0 {
            notify_lark(format!("System run normally {} times",times)).await.unwrap();
        }
        warn!("complete listen all pairs,and start next minute");
    }
}

pub async fn execute_back_testing(
    year:u32,
    history_data: HashMap<Symbol, Vec<Kline>>,
    month: u8,
) -> Vec<(SellReason, f32, u32)> {
    let balance = 10.0;
    let mut take_order_pair: HashMap<TakeType, Vec<TakeOrderInfo>> = HashMap::new();
    ///reason,total_profit,txs
    let mut all_reason_total_profit: Vec<(SellReason, f32, u32)> =
        vec![(AStrongSignal, 0.0, 0), (TwoMiddleSignal, 0.0, 0)];
    //let mut all_reason_total_profit: Vec<(SellReason, f32,u32)> = vec![(AStrongSignal, 0.0,0)];
    let eth_klines = load_history_data_by_pair(year,"ETHUSDT", month).await;
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


pub async fn execute_back_testing2(year:u32,month: u8) -> Vec<StrategyEffect> {
    let balance = 10.0;
    let mut take_order_pair: HashMap<TakeType, Vec<TakeOrderInfo>> = HashMap::new();
    ///reason,total_profit,txs
    let mut all_reason_total_profit: Vec<StrategyEffect> =
        vec![
                       StrategyEffect::new(AStrongSignal),
                       //StrategyEffect::new(AStrongSignal_V2),
                       StrategyEffect::new(TwoMiddleSignal),
                       //StrategyEffect::new(TwoMiddleSignal_V2),
                       StrategyEffect::new(ThreeContinuousSignal),
                       StrategyEffect::new(AVeryStrongSignal),
                       //StrategyEffect::new(AVeryStrongSignal_V2),
                       //StrategyEffect::new(StartGoDown),
        ];
    let all_pairs = list_all_pair().await;
    let eth_klines = load_history_data_by_pair(year,"ETHUSDT", month).await;
    for (index,pair) in all_pairs.iter().enumerate() {
        //for test recent kline
        /*
        if pair.symbol != "ZILUSDT" {
            continue
        }
        let kline_url = format!(
            "https://api.binance.com/api/v3/klines?symbol={}&interval=1m&limit={}",
            pair.symbol.as_str(),
            10000
        );
        let klines = try_get::<Vec<Kline>>(kline_url).await.to_vec();
        */
        warn!("date({}-{}):start test index {} symbol {}", year,month,index,pair.symbol.as_str());
        let klines = load_history_data_by_pair(year,&pair.symbol, month).await;
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
                // fixme：间隔2小时之后的buy为最后一次，此时再统计盈利
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
                    } else {
                        effect.lose_txs += 1;
                    }
                    info!("tmp:year {} month {} ,detail {:?}",year,month,effect);
                }
                //当前reason下：0、还没加入观察列表，1、还没开始下卖单，2、已经下卖单但不符合平仓条件
                //无论是否下单，都继续sell筛选，sell里面保证没有重复下单
                /* if is_took {
                    continue;
                }*/
            }


           /* if eth_klines[index + 350].open_price.to_f32() / eth_klines[index].open_price.to_f32() > 1.03
                || (index >= 360 && eth_klines[index + 350].open_price.to_f32() / eth_klines[index - 350].open_price.to_f32() > 1.05)
            {
                continue;
            }*/


            let _ = strategy::sell(&mut take_order_pair, &line_datas, &pair, balance, false).await;
        }
    }
    return all_reason_total_profit;
}

pub async fn execute_back_testing3(year:u32,month: u8) -> Vec<StrategyEffect> {
    let balance = 10.0;
    let mut take_order_pair: HashMap<TakeType, Vec<TakeOrderInfo>> = HashMap::new();
    ///reason,total_profit,txs
    let mut all_reason_total_profit: Vec<StrategyEffect> =
        vec![
            /*            StrategyEffect::new(AStrongSignal),
                         StrategyEffect::new(TwoMiddleSignal),
                         StrategyEffect::new(ThreeContinuousSignal),
                         StrategyEffect::new(AVeryStrongSignal),*/
            StrategyEffect::new(SequentialTakeOrder)
        ];
    //let mut all_reason_total_profit: Vec<(SellReason, f32,u32)> = vec![(AStrongSignal, 0.0,0)];
    let all_pairs = list_all_pair().await;
    let eth_klines = load_history_data_by_pair(year,"ETHUSDT", month).await;
    for pair in all_pairs.iter() {
        warn!("start test {}", pair.symbol.as_str());
        let klines = load_history_data_by_pair(year,&pair.symbol, month).await;
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
                // fixme：间隔2小时之后的buy为最后一次，此时再统计盈利
                let (_is_took, profit) =
                    strategy::buy(&mut take_order_pair, take_type, &line_datas, false)
                        .await
                        .unwrap();
                effect.total_profit += profit;
                //只有下了卖单和买单的才统计收益
                if profit != 0.0 {
                    //effect.total_profit -= 0.0008;
                    effect.txs += 1;
                    if profit > 0.0 {
                        effect.win_txs += 1;
                    } else {
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


            if eth_klines[index + 350].open_price.to_f32() / eth_klines[index].open_price.to_f32() > 1.03
                || (index >= 360 && eth_klines[index + 350].open_price.to_f32() / eth_klines[index - 350].open_price.to_f32() > 1.05)
            {
                continue;
            }


            let _ = strategy::sell(&mut take_order_pair, &line_datas, &pair, balance, false).await;
        }
    }
    return all_reason_total_profit;
}

//开多仓逻辑,1天后平仓
pub async fn execute_back_testing4(year:u32,month: u8) -> Vec<StrategyEffect> {
    let mut take_order_pair: HashMap<String, (u64, f32)> = HashMap::new();
    ///reason,total_profit,txs
    let mut all_reason_total_profit: Vec<StrategyEffect> = vec![StrategyEffect::new(SellReason::Buy1)];
    let all_pairs = list_all_pair().await;
    let mut profit_info = (0u32,0u32,0u32,0.0f32);
    for pair in all_pairs.iter() {
        warn!("start test {}", pair.symbol.as_str());
        let klines = load_history_data_by_pair(year,&pair.symbol, month).await;
        if klines.is_empty() {
            continue;
        }
        let mut index = 0;
        //txs,win_txs,lose_txs,per_ratio,total_ratio
        for bar in &klines[359..] {
            let line_datas = &klines[index..(index + 360)];
            index += 1;
            let last_bar  = line_datas[358].clone();
            let current_bar  = line_datas[359].clone();
            let bar_1h_ago  = line_datas[345].clone();

            let mut remote_klines = line_datas[0..=340].to_owned();
            let mut recent_klines = line_datas[342..].to_owned();
            let (remote_average_price, remote_average_volume) = get_average_info(&remote_klines[..]);
            let (recent_average_price, recent_average_volume) = get_average_info(&recent_klines[..]);
            match take_order_pair.clone().get_mut(&pair.symbol) {
                None => {
                    let huge_bar = line_datas[345].clone();
                    if huge_bar.volume.to_f32() / remote_average_volume >= 40.0
                        && huge_bar.volume.to_f32() / recent_average_volume > 3.0
                        && huge_bar.close_price.to_f32() / remote_average_price >= 1.01
                        && recent_average_price / huge_bar.close_price.to_f32() >= 1.005
                    {
                        take_order_pair.insert(pair.symbol.clone(), (current_bar.open_time, current_bar.open_price.to_f32()));
                        //todo: start buy
                    }
                }
                Some(info) => {
                    //6小时强制平多单
                  /*  if last_bar.open_time - info.to_owned().0 >  20 * 60 * 1000
                        && last_bar.close_price.to_f32() < bar_1h_ago.close_price.to_f32()*/
                    if last_bar.close_price.to_f32() < bar_1h_ago.close_price.to_f32()
                    {
                        //start sell
                        take_order_pair.remove(&pair.symbol);
                        //txs,win_txs,lose_txs,per_ratio,total_ratio
                        profit_info.0 += 1;
                        let raise_ratio = (current_bar.open_price.to_f32() - info.to_owned().1) / info.to_owned().1;
                        if raise_ratio > 0.0 {
                            profit_info.1 += 1
                        }else {
                            profit_info.2 += 1
                        }
                        profit_info.3 += raise_ratio;
                        info!("tmp0002:pair {},startDate {},endDate {},raise_ratio {}",pair.symbol,timestamp2date(info.to_owned().0),timestamp2date(current_bar.open_time),raise_ratio);
                    }
                }
            }
        }

    }
    info!("tmp0003:year {} month {} txs {},win_txs {},lose_txs {},total_ratio {}",
        year,month,profit_info.0,profit_info.1,profit_info.2,profit_info.3);
    return all_reason_total_profit;
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
        .subcommand(App::new("back_testing3"))
        .subcommand(App::new("back_testing4"))
        .subcommand(App::new("download_history_kline"))
        .get_matches();
    match matches.subcommand() {
        Some(("real_trading", _sub_matches)) => {
            println!("real_trading");
            excute_real_trading().await;
        }
        Some(("back_testing", _sub_matches)) => {
            println!("back_testing");
            for year in 2020u32..2023u32 {
                for month in 1..=12 {
                    let history_data = load_history_data(year,month).await;
                    let datas = execute_back_testing(year,history_data, month).await;
                    for (reason, total_profit, txs) in datas {
                        let reason_str: &str = reason.into();
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
        }
        Some(("back_testing2", _sub_matches)) => {
            println!("back_testing2");
            for year in 2021u32..=2023u32 {
                let months = if year == 2023 {
                    1..=2
                }else {
                    1..=12
                };
                rayon::scope(|scope| {
                    for month in months {
                        scope.spawn(move |_| {
                            let rt = Runtime::new().unwrap();
                            rt.block_on(async move {
                                let datas = execute_back_testing2(year,month).await;
                                for data in datas {
                                    warn!("finally: year {} month {},detail {:?}",year,month,data);
                                }
                            });
                        });
                    }
                });
            }
        }
        //开多仓的尝试：失败
        Some(("back_testing3", _sub_matches)) => {
            println!("back_testing3");
            for year in 2020u32..2023u32 {
                rayon::scope(|scope| {
                    for month in 1..=12 {
                        scope.spawn(move |_| {
                            let rt = Runtime::new().unwrap();
                            rt.block_on(async move {
                                let datas = execute_back_testing3(year,month).await;
                                for data in datas {
                                    warn!("finally: month {},detail {:?}",month,data);
                                }
                            });
                        });
                    }
                });
            }
        }
        //连续开空单的尝试：希望统一开空逻辑：暂时失败
        Some(("back_testing4", _sub_matches)) => {
            println!("back_testing4");
            for year in 2020..2023 {
                rayon::scope(|scope| {
                    for month in 1..=12 {
                        scope.spawn(move |_| {
                            let rt = Runtime::new().unwrap();
                            rt.block_on(async move {
                                execute_back_testing4(year,month).await;
                            });
                        });
                    }
                });
            }
        }
        Some(("download_history_kline", _sub_matches)) => {
            println!("download_history_kline");
            for year in 2020u32..2023u32 {
                download_history_data(year).await
            }
        }
        _ => {}
    }
    Ok(())
}
