use crate::{get_last_bar_shape_score, get_last_bar_volume_score, get_raise_bar_num, notify_lark, recent_kline_shape_score, take_order, timestamp2date, Kline, MathOperation, MathOperation2, Symbol, TakeOrderInfo, KLINE_NUM_FOR_FIND_SIGNAL, get_average_info, get_huge_volume_bar_num, INCREASE_VOLUME_LEVEL2, INCREASE_PRICE_LEVEL2};
use std::collections::HashMap;
use std::ops::{Div, Mul, Sub};
use crate::constant::WEEK;
use crate::kline::volume_too_few;

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
    for (index,bar) in line_datas[..358].iter().enumerate() {
        if index <= 340 && bar.volume.to_f32().div(10.0) > line_datas[358].volume.to_f32() {
            return false;
        }else if index > 340 && bar.volume.to_f32().div(5.0) > line_datas[358].volume.to_f32() {
            return false;
        }
    }

    //交易量要大部分bar都符合要求
    let mut recent_huge_volume_bars_num =
        get_huge_volume_bar_num(broken_klines, recent_average_volume, INCREASE_VOLUME_LEVEL2);
    //let mut recent_huge_volume_bars_num = get_huge_volume_bar_num(broken_klines, recent_average_volume, 1.0);

    let recent_price_increase_rate =
        (current_price - recent_average_price).div(recent_average_price);

    debug!(
        "judge_break_signal market {},recent_price_increase_rate {},recent_huge_volume_bars_num {}
    ",
        market, recent_price_increase_rate, recent_huge_volume_bars_num
    );
    debug!("market {},start {} ,end {}: recent_price_increase_rate {},recent_huge_volume_bars_num {}"
    ,market
    ,timestamp2date(line_datas[0].open_time)
    ,timestamp2date(line_datas[359].open_time)
    ,recent_price_increase_rate
    ,recent_huge_volume_bars_num);
    if recent_price_increase_rate >= INCREASE_PRICE_LEVEL2 && recent_huge_volume_bars_num >= 4 {
        return true;
    }
    false
}

//配合buy放宽顶部信号条件
pub async fn sell(
    take_order_pair2: &mut HashMap<String, TakeOrderInfo>,
    line_datas: &[Kline],
    pair: &Symbol,
    balance: f32,
    is_real_trading: bool,
) -> Result<bool, Box<dyn std::error::Error>> {
    let pair_symbol = pair.symbol.as_str();
    let now = line_datas[359].open_time + 1000;
    if is_break_through_market(pair_symbol, &line_datas).await {
        info!("found_break_signal3：pair_symbol {}", pair_symbol);
        let half_hour_inc_ratio =  (line_datas[358].open_price.to_f32() - line_datas[328].open_price.to_f32()).div(30.0);
        let ten_minutes_inc_ratio =  (line_datas[358].open_price.to_f32() - line_datas[348].open_price.to_f32()).div(10.0);

        let broken_line_datas = &line_datas[340..360];
        let shape_score = get_last_bar_shape_score(broken_line_datas.to_owned());
        let volume_score = get_last_bar_volume_score(broken_line_datas.to_owned());
        //8-17。多一个作为价格比较的基准
        let recent_shape_score = recent_kline_shape_score(broken_line_datas[7..=17].to_vec());

        //总分分别是：7分，5分，10分
        //分为三种情况：强信号直接下单，弱信号加入观测名单，弱信号且已经在观查名单且距离观察名单超过五分钟的就下单，
        debug!(
            "------: market {},shape_score {},volume_score {},recent_shape_score {}",
            pair_symbol, shape_score, volume_score, recent_shape_score
        );
        if
            shape_score >= 4
            && volume_score >= 3
            && recent_shape_score >= 6
        {
            //以倒数第二根的open，作为信号发现价格，以倒数第一根的open为实际下单价格
            let price = broken_line_datas[19].open_price.parse::<f32>().unwrap();

            //default lever ratio is 20x,每次2成仓位20倍
            let taker_amount = balance
                .mul(20.0)
                .div(10.0)
                .div(price)
                .to_fix(pair.quantity_precision as u32);
            let mut push_text = "".to_string();
            let take_info = take_order_pair2.get(pair_symbol);
            //二次拉升才下单,并且量大于2倍
            if take_info.is_some() && broken_line_datas[18].volume.to_f32().div(1.1) > take_info.unwrap().top_bar.volume.to_f32()
            {

                let inc_ratio_distance = ten_minutes_inc_ratio.div(half_hour_inc_ratio);
                if inc_ratio_distance < 1.2 {
                    warn!("strategy2-{}-{}-deny: inc_ratio_distance {}",
                    pair_symbol,timestamp2date(now),inc_ratio_distance);
                    return Ok(false);
                }else {
                    warn!("strategy2-{}-{}-allow: inc_ratio_distance {}",
                    pair_symbol,timestamp2date(now),inc_ratio_distance);
                }



                if is_real_trading {
                    take_order(pair_symbol.to_string(), taker_amount, "SELL".to_string()).await;
                }
                let order_info = TakeOrderInfo {
                    take_time: now,
                    price,
                    amount: taker_amount,
                    top_bar: broken_line_datas[18].clone(),
                    is_took: true,
                };
                take_order_pair2.insert(pair_symbol.to_string(), order_info);
                push_text = format!("strategy2: take_sell_order: market {},shape_score {},volume_score {},recent_shape_score {},taker_amount {}",
                                    pair_symbol, shape_score, volume_score, recent_shape_score, taker_amount
                );
            } else {
                let order_info = TakeOrderInfo {
                    take_time: now,
                    price,
                    amount: taker_amount, //not care
                    top_bar: broken_line_datas[18].clone(),
                    is_took: false,
                };
                take_order_pair2.insert(pair_symbol.to_string(), order_info);
                push_text = format!("add_observe_list: market {},shape_score {},volume_score {},recent_shape_score {},taker_amount {}",
                                    pair_symbol, shape_score, volume_score, recent_shape_score, taker_amount
                );
            }
            warn!("now {}, {}",timestamp2date(now),push_text);
            if is_real_trading {
                notify_lark(push_text).await?;
            }
        } else {
            info!("Have no take order signal,\
                     below is detail score:market {},shape_score {},volume_score {},recent_shape_score {}",
                              pair_symbol,shape_score,volume_score,recent_shape_score
                     );
        }
    } else {
        debug!("Have no obvious break signal");
    }
    Ok(false)
}


//下单之后判断交易量，临近的三根必须大于五分之一，否则就大概率不是顶
pub async fn buy(
    take_order_pair2: &mut HashMap<String, TakeOrderInfo>,
    pair_symbol: &str,
    line_datas: &[Kline],
    is_real_trading: bool,
) -> Result<(bool, f32), Box<dyn std::error::Error>> {
    let now = line_datas[359].open_time + 1000;
    match take_order_pair2.get(pair_symbol) {
        None => {}
        Some(take_info) => {
            if take_info.is_took == true {
                let price_raise_ratio = line_datas[KLINE_NUM_FOR_FIND_SIGNAL - 1]
                    .open_price
                    .to_f32()
                    / take_info.price;

                let interval_from_take = line_datas[KLINE_NUM_FOR_FIND_SIGNAL - 1].open_time - take_info.take_time;
                //三种情况平仓1、顶后三根有小于五分之一的，2，20根之后看情况止盈利
                let (can_buy, buy_reason) = if line_datas[KLINE_NUM_FOR_FIND_SIGNAL - 2].open_time <= take_info.take_time + 1000 * 60 * 3 //顶后三根
                    && line_datas[KLINE_NUM_FOR_FIND_SIGNAL - 2].volume.to_f32() <= take_info.top_bar.volume.to_f32().div(6.0)
                {
                    (true,"too few volume in last 3 bars")
                //} else if volume_too_few(&line_datas[350..],take_info.top_bar.volume.to_f32())
                //{
                //    (true,"last 10 bars volume too few")
                } else if line_datas[KLINE_NUM_FOR_FIND_SIGNAL - 120].open_time > take_info.take_time
                //    && price_raise_ratio < 1.0
                    && get_raise_bar_num(&line_datas[KLINE_NUM_FOR_FIND_SIGNAL - 30..]) >= 10
                {
                    (true,"Positive income and held it for two hour，and price start increase")
                //}else if interval_from_take >  WEEK {
                //    (true,"hold order for a weak,have to stop it")
                }else {
                    (false,"")
                };
                if can_buy {//和多久之前的比较，比较多少根？
                    let push_text = format!(
                        "strategy2: buy_reason <<{}>>:: take_buy_order: market {},interval_from_take {}({}),price_raise_ratio {}",
                        buy_reason,pair_symbol, interval_from_take,timestamp2date(interval_from_take),price_raise_ratio
                    );
                    //fixme: 这里remove会报错
                    //take_order_pair2.remove(pair_symbol);
                    if is_real_trading {
                        take_order(pair_symbol.to_string(), take_info.amount, "BUY".to_string())
                            .await;
                        notify_lark(push_text.clone()).await?;
                    }
                    take_order_pair2.remove(pair_symbol);
                    warn!("now {} , {}",timestamp2date(now),push_text);
                    return Ok((true, 1.0 - price_raise_ratio));
                } else {
                    return Ok((true, 0.0));
                }
            } else {
                //加入观察列表五分钟内不在观察，2小时内仍没有二次拉起的则将其移除观察列表
               if now.sub(take_info.take_time) > 4 * 60 * 60 * 1000 {
                    take_order_pair2.remove(pair_symbol);
               }
            }
        }
    }
    Ok((false, 0.0))
}
