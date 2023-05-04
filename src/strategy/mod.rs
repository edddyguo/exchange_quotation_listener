mod buy;
pub(crate) mod sell;

use crate::constant::WEEK;
use crate::kline::volume_too_few;
use crate::strategy::sell::a_strong_signal::ASS;
use crate::strategy::sell::two_middle_signal::TMS;
use crate::strategy::sell::{SellReason, SellStrategy};
use crate::{get_average_info, get_huge_volume_bar_num, get_last_bar_shape_score,
            get_last_bar_volume_score, get_raise_bar_num,
            notify_lark, recent_kline_shape_score, strategy,
            take_order, timestamp2date, Kline, MathOperation, MathOperation2, Pair, Symbol,
            TakeOrderInfo, TakeType, INCREASE_PRICE_LEVEL2, INCREASE_VOLUME_LEVEL2,
            KLINE_NUM_FOR_FIND_SIGNAL, MAX_PROFIT_LOSE_RATIO};
use std::collections::HashMap;
use std::ops::{Deref, Div, Mul, Sub};
use rust_decimal::prelude::ToPrimitive;
use crate::strategy::sell::a_strong_signal_v2::ASS_V2;
use crate::strategy::sell::a_very_strong_signal::AVSS;
use crate::strategy::sell::a_very_strong_signal_v2::AVSS_V2;
use crate::strategy::sell::sequential_take_order::STO;
use crate::strategy::sell::start_go_down::SGD;
use crate::strategy::sell::three_continuous_signal::TCS;
use crate::strategy::sell::two_middle_signal_v2::TMS_V2;

pub struct OrderData {
    pair: String,
    sell_reason: SellReason,
    sell_time: u64,
    buy_time: u64,
    increase_ratio: f32,
}
/*macro_rules! signal_exist {
	($t:expr) => {
        take_order_pair.get(&TakeType {
        pair: pair_symbol.to_string(),
        sell_reason: $t,
        }).is_some()
    }
}*/

//是否突破：分别和远期（1小时）和中期k线（30m）进行对比取低值
async fn is_break_through_market(market: &str, line_datas: &[Kline]) -> bool {
    assert_eq!(line_datas.len(), KLINE_NUM_FOR_FIND_SIGNAL);
    //选351个，后边再剔除量最大的
    let mut recent_klines = line_datas[0..=340].to_owned();
    let broken_klines = &line_datas[339..=358];
    assert_eq!(recent_klines.len(), 341);
    assert_eq!(broken_klines.len(), 20);
    let (recent_average_price, recent_average_volume) = get_average_info(&recent_klines[..]);
    //价格以当前high为准
    let current_price = line_datas[KLINE_NUM_FOR_FIND_SIGNAL - 1]
        .high_price
        .to_f32();
    //最近2小时，交易量不能有大于准顶量1.5倍的
    for (index, bar) in line_datas[..358].iter().enumerate() {
        if index <= 340 && bar.volume.to_f32().div(100.0) > line_datas[358].volume.to_f32() {
            return false;
        } else if index > 340 && bar.volume.to_f32().div(50.0) > line_datas[358].volume.to_f32() {
            return false;
        }
    }

    //交易量要大部分bar都符合要求
    let mut recent_huge_volume_bars_num =
        get_huge_volume_bar_num(broken_klines, recent_average_volume, INCREASE_VOLUME_LEVEL2);

    let recent_price_increase_rate =
        (current_price - recent_average_price).div(recent_average_price);

    debug!(
        "judge_break_signal market {},recent_price_increase_rate {},recent_huge_volume_bars_num {}
    ",
        market, recent_price_increase_rate, recent_huge_volume_bars_num
    );
    debug!(
        "market {},start {} ,end {}: recent_price_increase_rate {},recent_huge_volume_bars_num {}",
        market,
        timestamp2date(line_datas[0].open_time),
        timestamp2date(line_datas[359].open_time),
        recent_price_increase_rate,
        recent_huge_volume_bars_num
    );
    if recent_price_increase_rate >= INCREASE_PRICE_LEVEL2 && recent_huge_volume_bars_num >= 4 {
        return true;
    }
    false
}

//配合buy放宽顶部信号条件
pub async fn sell(
    take_order_pair: &mut HashMap<TakeType, Vec<TakeOrderInfo>>,
    line_datas: &[Kline],
    pair: &Symbol,
    balance: f32,
    is_real_trading: bool,
) -> Result<bool, Box<dyn std::error::Error>> {
    let pair_symbol = pair.symbol.as_str();

    let tms_len = take_order_pair.get(&TakeType {
        pair: pair_symbol.to_string(),
        sell_reason: SellReason::TwoMiddleSignal,
    }).map_or(0,|x| x.len());

    let ass_exist = take_order_pair.get(&TakeType {
        pair: pair_symbol.to_string(),
        sell_reason: SellReason::AStrongSignal,
    }).is_some();

    let avss_exist = take_order_pair.get(&TakeType {
        pair: pair_symbol.to_string(),
        sell_reason: SellReason::AVeryStrongSignal,
    }).is_some();

    /*    let tms_v2_exist = take_order_pair.get(&TakeType {
            pair: pair_symbol.to_string(),
            sell_reason: SellReason::TwoMiddleSignal_V2,
        }).is_some();*/


    let tcs_exist = take_order_pair.get(&TakeType {
        pair: pair_symbol.to_string(),
        sell_reason: SellReason::ThreeContinuousSignal,
    }).and_then(|x| Some(x.last().unwrap().is_took));;
    /**
    let sgd_exist = take_order_pair.get(&TakeType {
        pair: pair_symbol.to_string(),
        sell_reason: SellReason::StartGoDown,
    }).is_some();
     */


    let is_break = is_break_through_market(pair_symbol, &line_datas).await;

    //以倒数第二根的open，作为信号发现价格，以倒数第一根的open为实际下单价格
    let price = line_datas[359].open_price.parse::<f32>().unwrap();
    let mut taker_amount = balance
        .mul(20.0)
        .div(1000.0)
        .div(price)
        .to_fix(pair.quantity_precision as u32);
    let take_amount_time = vec![1.0f32, 2.0, 4.0, 8.0,16.0 ,24.0,48.0,96.0,96.0,96.0,96.0,96.0];
    taker_amount *= take_amount_time[tms_len];

    //todo: 将其中通用的计算逻辑拿出来
    //if !ass_exist && is_break {
    /***
    if is_break {
            ASS::condition_passed(take_order_pair, line_datas, pair, taker_amount, price, is_real_trading).await?;
    }
    **/
    //if !avss_exist && is_break{
    /***
    if is_break{
        AVSS::condition_passed(take_order_pair, line_datas, pair, taker_amount, price, is_real_trading).await?;
    }

    ***/

    TMS::condition_passed(take_order_pair, line_datas, pair, taker_amount, price, is_real_trading).await?;
    /***
    if tms_exist.is_none() && is_break
        //|| tms_exist.is_some() && tms_exist.unwrap() == false
        || tms_exist.is_some()
    {
        TMS::condition_passed(take_order_pair, line_datas, pair, taker_amount, price, is_real_trading).await?;
    }
    */
    /*
        if !tms_v2_exist && is_break
            || tms_v2_exist
        {
            TMS_V2::condition_passed(take_order_pair, line_datas, pair, taker_amount, price, is_real_trading).await?;
        }
    */

    /***
    if tcs_exist.is_none() && is_break
        //|| tcs_exist.is_some() && tcs_exist.unwrap() == false
        || tcs_exist.is_some()
    {
        TCS::condition_passed(take_order_pair, line_datas, pair, taker_amount, price, is_real_trading).await?;
    }
    ***/

    /*
   if !sgd_exist && is_break
        || sgd_exist
    {
        SGD::condition_passed(take_order_pair, line_datas, pair, taker_amount, price, is_real_trading).await?;
    }
    */
    //STO::condition_passed(take_order_pair, line_datas, pair, taker_amount, price, is_real_trading).await?;

    Ok(true)
}

//(Pair,SellReason)
//不处理返回值对：多次确认的逻辑没有影响，对单次确认的来说，有可能造成短期多次下单，单这个也是没毛病的
pub async fn buy(
    take_order_pair: &mut HashMap<TakeType, Vec<TakeOrderInfo>>,
    taker_type: TakeType,
    line_datas: &[Kline],
    is_real_trading: bool,
    eth_is_strong: bool,
) -> Result<(bool, f32), Box<dyn std::error::Error>> {
    let now = line_datas[359].open_time + 1000;
    match take_order_pair.get(&taker_type) {
        None => {}
        Some(take_infos) => {
            let take_info = take_infos.last().unwrap();
            if take_info.is_took == true {
                let sell_reason = taker_type.clone().sell_reason;
                let current_price = line_datas[KLINE_NUM_FOR_FIND_SIGNAL - 1]
                    .open_price
                    .to_f32();
                //以标准信号后续3根为止，所以判断的bar的index为 line_datas[KLINE_NUM_FOR_FIND_SIGNAL - 5]
                let signal_bal = line_datas[KLINE_NUM_FOR_FIND_SIGNAL - 5].to_owned();
                let remote_average= get_average_info(&line_datas[345..355]);
                let recent_average = get_average_info(&line_datas[356..=358]);

                let interval_from_take =
                    line_datas[KLINE_NUM_FOR_FIND_SIGNAL - 1].open_time - take_info.take_time;
                //三种情况平仓1、顶后三根有小于五分之一的，2，20根之后看情况止盈利
                let (can_buy, buy_reason) = if
                //line_datas[KLINE_NUM_FOR_FIND_SIGNAL - 2].open_time <= take_info.take_time + 1000 * 60 * 3 //顶后三根
                //&& line_datas[KLINE_NUM_FOR_FIND_SIGNAL - 2].volume.to_f32() <= take_info.top_bar.volume.to_f32().div(6.0)
                //line_datas[KLINE_NUM_FOR_FIND_SIGNAL - 2].close_price > take_info.top_bar.close_price
                /*                line_datas[KLINE_NUM_FOR_FIND_SIGNAL - 40].open_time > take_info.take_time
                                    && signal_bal.is_strong_raise()
                                    && signal_bal.volume.to_f32() * 4.0 > take_info.top_bar.volume.to_f32()
                                    && signal_bal.volume.to_f32() / 6.0 > remote_average.1
                                    && signal_bal.close_price.to_f32() < recent_average.0*/
                //(sell_reason == SellReason::AVeryStrongSignal || sell_reason == SellReason::AStrongSignal)
                //  && line_datas[KLINE_NUM_FOR_FIND_SIGNAL - 2].close_price.to_f32() >= take_info.top_bar.high_price.to_f32()
                sell_reason == SellReason::StartGoDown && get_raise_bar_num(&line_datas[KLINE_NUM_FOR_FIND_SIGNAL - 30..]) >= 15
                {
                    (true, "too few volume in last 3 bars")
                    //} else if volume_too_few(&line_datas[350..],take_info.top_bar.volume.to_f32())
                    //{
                    //    (true,"last 10 bars volume too few")
                } else if (sell_reason == SellReason::AVeryStrongSignal
                    || sell_reason == SellReason::AStrongSignal
                    || sell_reason == SellReason::AStrongSignal_V2
                    || sell_reason == SellReason::AVeryStrongSignal_V2
                )
                    &&line_datas[KLINE_NUM_FOR_FIND_SIGNAL - 120].open_time > take_info.take_time
                    && get_raise_bar_num(&line_datas[KLINE_NUM_FOR_FIND_SIGNAL - 30..]) >= 10
                {
                    (true, "Positive income and held it for two hour，and price start increase")
                } else if (sell_reason == SellReason::TwoMiddleSignal || sell_reason == SellReason::ThreeContinuousSignal)
                    &&line_datas[KLINE_NUM_FOR_FIND_SIGNAL - 240].open_time > take_info.take_time
                    && get_raise_bar_num(&line_datas[KLINE_NUM_FOR_FIND_SIGNAL - 30..]) >= 10
                {
                    (true, "Positive income and held it for two hour，and price start increase")
                }else if eth_is_strong {
                    (true, "eth is strong")
                } else {
                    (false, "")
                };
                if can_buy {
                    let mut total_amount = 0.0f32;
                    let mut total_raise_price = 0.0f32;
                    let mut order_num = 0u32;
                    let mut profit_detail = Vec::new();
                    /***
                    for take_info in take_infos {
                        if take_info.is_took == true{
                            total_amount += take_info.amount;
                            let current_profit = (current_price - take_info.sell_price).div(take_info.sell_price);
                            total_raise_price += current_profit;
                            profit_detail.push(current_profit);
                            order_num += 1;
                        }
                    }
                    ***/
                    let mut index = 0usize;
                    let take_amount_time = vec![1.0f32, 2.0, 4.0, 8.0,16.0 ,24.0,48.0,96.0,96.0,96.0,96.0,96.0];

                    for (_index,take_info) in take_infos.iter().enumerate() {
                        if take_info.is_took == true{
                            total_amount += take_info.amount;
                            let current_profit = (current_price - take_info.sell_price).div(take_info.sell_price) * take_amount_time[index];
                            total_raise_price += current_profit;
                            profit_detail.push(current_profit);
                            order_num += 1;
                            index += 1;
                        }
                    }

                    unsafe {
                        if total_raise_price > MAX_PROFIT_LOSE_RATIO.1 {
                            MAX_PROFIT_LOSE_RATIO = (now,total_raise_price);
                            warn!("now {} market {},MAX_PROFIT_LOSE_RATIO {}",timestamp2date(now),taker_type.pair,MAX_PROFIT_LOSE_RATIO.1);
                        }
                        //超过7天没继续更新的强制更新
                        if now > MAX_PROFIT_LOSE_RATIO.0 + 7 * 24 * 60 * 60 * 1000 &&  now < MAX_PROFIT_LOSE_RATIO.0 + 14 * 24 * 60 * 60 * 1000{
                            MAX_PROFIT_LOSE_RATIO = (now,0.0);
                        }
                    }
                    //如果当前仍处于亏损状态，则就一直等待
                    let (_average_price,average_volume) = get_average_info(&line_datas[0..358]);
                    let top_volume = take_info.top_bar.volume.to_f32();
                    //处于亏损但是，处于前6个小时内或者交易量仍然没有萎靡的就继续持仓
                    if eth_is_strong == false && total_raise_price >= -0.05 {
                        /***
                        if line_datas[0].open_time < take_info.take_time {
                            return Ok((false, 0.0));
                        }
                        if average_volume.mul(80.0) > top_volume {
                            return Ok((false, 0.0));
                        }
                        ***/
                        return Ok((false, 0.0));
                    }

                    let first_open_time = take_infos.first().unwrap().take_time;
                    let spend_time = (now - first_open_time).div(24*60*60*1000);//day
                    //和多久之前的比较，比较多少根？
                    let sell_reason_str:&str = sell_reason.into();
                    let push_text = format!(
                        "strategy:spend {} day, order_num {},profit_detail {:?}, buy_reason <<{}>>,sell_reason <<{}>>:: take_buy_order: market {},price_raise_ratio {}",
                        spend_time,order_num,profit_detail,buy_reason, sell_reason_str, taker_type.pair, total_raise_price);
                    //fixme: 这里remove会报错
                    //take_order_pair2.remove(pair_symbol);
                    if is_real_trading {
                        take_order(
                            taker_type.pair.clone(),
                            total_amount,
                            "BUY".to_string(),
                        )
                            .await;
                        notify_lark(push_text.clone()).await?;
                    }
                    //info!("data0001: now {} market {},detail {:?},sell_info {:?}",timestamp2date(now),taker_type.pair,push_text,take_infos);
                    info!("data0001: now {} market {},detail {:?}",timestamp2date(now),taker_type.pair,push_text);
                    take_order_pair.remove(&taker_type);
                    return Ok((true, -total_raise_price));
                } else {
                    return Ok((true, 0.0));
                }
            } else {
                //加入观察列表五分钟内不在观察，2小时内仍没有二次拉起的则将其移除观察列表
                if now.sub(take_info.take_time) > 4 * 60 * 60 * 1000 {
                    take_order_pair.remove(&taker_type);
                }
            }
        }
    }
    Ok((false, 0.0))
}

//连续的下单尝试拉低下单成本
/*pub async fn buy(
    take_order_pair: &mut HashMap<TakeType, Vec<TakeOrderInfo>>,
    taker_type: TakeType,
    line_datas: &[Kline],
    is_real_trading: bool,
) -> Result<(bool, f32), Box<dyn std::error::Error>> {
    let now = line_datas[359].open_time + 1000;
    match take_order_pair.get(&taker_type) {
        None => {}
        Some(take_infos) => {
            let last_take_info = take_infos.last().unwrap();
            //三种情况平仓1、顶后三根有小于五分之一的，2，20根之后看情况止盈利
            let (can_buy, buy_reason)= if line_datas[KLINE_NUM_FOR_FIND_SIGNAL - 120].open_time
                > last_take_info.take_time
                && get_raise_bar_num(&line_datas[KLINE_NUM_FOR_FIND_SIGNAL - 30..]) >= 10
            {
                (
                    true,
                    "Positive income and held it for two hour，and price start increase",
                )
            } else {
                (false, "")
            };
            if can_buy {
                //和多久之前的比较，比较多少根？
                let sell_reason_str: &str = taker_type.clone().sell_reason.into();
                let push_text = format!(
                    "strategy2: buy_reason <<{}>>,sell_reason <<{}>>:: take_buy_order: market {}",
                    buy_reason, sell_reason_str, taker_type.pair);
                //fixme: 这里remove会报错
                //take_order_pair2.remove(pair_symbol);
                if is_real_trading {
                    take_order(
                        taker_type.pair.clone(),
                        last_take_info.amount * take_infos.len() as f32,
                        "BUY".to_string(),
                    )
                        .await;
                    notify_lark(push_text.clone()).await?;
                }
                //计算总的收入
                let mut batch_profit = 0.0f32;
                let current_price = line_datas[KLINE_NUM_FOR_FIND_SIGNAL - 1]
                    .open_price
                    .to_f32();
                let mut detail_profits = Vec::new();
                for take_info in take_infos{
                    let price_raise_ratio = current_price / take_info.price;
                    let iterm_profit = 1.0 - price_raise_ratio - 0.0008;
                    batch_profit += iterm_profit;
                    detail_profits.push((iterm_profit,timestamp2date(take_info.take_time)));
                }
                info!("data0001: now {} market {},total_profit {},detail {:?}",timestamp2date(now),taker_type.pair,batch_profit,detail_profits);
                take_order_pair.remove(&taker_type);
                //warn!("now {} , {}", timestamp2date(now), push_text);
                return Ok((true, batch_profit));
            } else {
                return Ok((true, 0.0));
            }
        }
    }
    Ok((false, 0.0))
}*/

//连续下单的第二个版本
//10分钟内有发现第二根就不平仓,如果10根中，5根上涨就平仓,10分钟内，有五根大于顶点的,
// 10分钟外，就直接现有的30根10个向上的逻辑,以最后一次sell为准,观察周期为2小时
/*pub async fn buy(
    take_order_pair: &mut HashMap<TakeType, Vec<TakeOrderInfo>>,
    taker_type: TakeType,
    line_datas: &[Kline],
    is_real_trading: bool,
) -> Result<(bool, f32), Box<dyn std::error::Error>> {
    let now = line_datas[359].open_time + 1000;
    match take_order_pair.get_mut(&taker_type) {
        None => {}
        Some(take_infos) => {
            let last_take_info = take_infos.last().unwrap();
            //The last one must be order which haven't buy
            if last_take_info.buy_price.is_some() {
                //todo:这里是下sell单之后3小时之内的插眼结束，实际要的是buy之后的3小时结束，有差别但是不大
                if line_datas[KLINE_NUM_FOR_FIND_SIGNAL - 180].open_time > last_take_info.take_time
                {
                    take_order_pair.remove(&taker_type);
                }
                return Ok((false, 0.0));
            }

            //三种情况平仓1、顶后三根有小于五分之一的，2，20根之后看情况止盈利
            let (can_buy, buy_reason) = if line_datas[KLINE_NUM_FOR_FIND_SIGNAL - 30].open_time
                > last_take_info.take_time
                && get_raise_bar_num(&line_datas[KLINE_NUM_FOR_FIND_SIGNAL - 30..]) >= 10
            {
                (
                    true,
                    "Positive income and held it for two hour，and price start increase",
                )
                //最近30根中有五根大于最后一次顶点的
            } else if line_datas[KLINE_NUM_FOR_FIND_SIGNAL - 30].open_time
                <= last_take_info.take_time
                && line_datas[KLINE_NUM_FOR_FIND_SIGNAL - 30..]
                .iter()
                .filter(|&k| k.open_time > last_take_info.take_time && k.close_price.to_f32() > last_take_info.sell_price)
                .collect::<Vec<&Kline>>().len() >= 5
            {
                (
                    true,
                    "buy reason: 最近30根中有五根大于最后一次顶点的",
                )
            } else {
                (false, "")
            };
            if can_buy {
                //和多久之前的比较，比较多少根？
                let sell_reason_str: &str = taker_type.clone().sell_reason.into();
                let push_text = format!(
                    "strategy2: buy_reason <<{}>>,sell_reason <<{}>>:: take_buy_order: market {}",
                    buy_reason, sell_reason_str, taker_type.pair);
                //fixme: 这里remove会报错
                //take_order_pair2.remove(pair_symbol);
                if is_real_trading {
                    take_order(
                        taker_type.pair.clone(),
                        last_take_info.amount * take_infos.len() as f32,
                        "BUY".to_string(),
                    )
                        .await;
                    notify_lark(push_text.clone()).await?;
                }
                //计算总的收入
                let mut batch_profit = 0.0f32;
                let current_price = line_datas[KLINE_NUM_FOR_FIND_SIGNAL - 1]
                    .open_price
                    .to_f32();
                let mut detail_profits = Vec::new();
                for take_info in take_infos.iter_mut() {
                    if take_info.buy_price.is_none() {
                        take_info.buy_price = Some(line_datas[359].open_price.to_f32())
                    }else {
                        //之前已经统计过的数据不再统计
                        continue;
                    }
                    let price_raise_ratio = current_price / take_info.sell_price;
                    let iterm_profit = 1.0 - price_raise_ratio - 0.0008;
                    batch_profit += iterm_profit;
                    detail_profits.push((iterm_profit, timestamp2date(take_info.take_time)));
                }
                info!("data0001: now {} market {},total_profit {},detail {:?}",timestamp2date(now),taker_type.pair,batch_profit,detail_profits);
                //warn!("now {} , {}", timestamp2date(now), push_text);
                return Ok((true, batch_profit));
            } else {
                return Ok((true, 0.0));
            }
        }
    }
    Ok((false, 0.0))
}*/