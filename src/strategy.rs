use crate::{
    get_last_bar_shape_score, get_last_bar_volume_score, get_raise_bar_num,
    is_break_through_market, notify_lark, recent_kline_shape_score, take_order, timestamp2date,
    Kline, MathOperation, MathOperation2, Symbol, TakeOrderInfo, KLINE_NUM_FOR_FIND_SIGNAL,
};
use std::collections::HashMap;
use std::ops::{Div, Mul, Sub};

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
        info!("found_break_signal：pair_symbol {}", pair_symbol);
        let broken_line_datas = &line_datas[340..360];
        let shape_score = get_last_bar_shape_score(broken_line_datas.to_owned());
        let volume_score = get_last_bar_volume_score(broken_line_datas.to_owned());
        //8-17。多一个作为价格比较的基准
        let recent_shape_score = recent_kline_shape_score(broken_line_datas[7..=17].to_vec());

        //总分分别是：7分，5分，10分
        //分为三种情况：强信号直接下单，弱信号加入观测名单，弱信号且已经在观查名单且距离观察名单超过五分钟的就下单，
        info!(
            "------: market {},shape_score {},volume_score {},recent_shape_score {}",
            pair_symbol, shape_score, volume_score, recent_shape_score
        );
        if take_order_pair2.get(pair_symbol).is_none()
            && shape_score >= 4
            && volume_score >= 3
            && recent_shape_score >= 5
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
            //强信号或者二次拉升
            if (shape_score >= 5 && volume_score >= 5 && recent_shape_score >= 5)
                || take_order_pair2.get(pair_symbol).is_some()
            {
                if is_real_trading {
                    take_order(pair_symbol.to_string(), taker_amount, "SELL".to_string()).await;
                }
                let order_info = TakeOrderInfo {
                    take_time: now,
                    price,
                    amount: taker_amount,
                    is_took: true,
                };
                take_order_pair2.insert(pair_symbol.to_string(), order_info);
                push_text = format!("take_sell_order: market {},shape_score {},volume_score {},recent_shape_score {},taker_amount {}",
                                    pair_symbol, shape_score, volume_score, recent_shape_score, taker_amount
                );
            } else {
                let order_info = TakeOrderInfo {
                    take_time: now,
                    price,
                    amount: 0.0, //not care
                    is_took: false,
                };
                take_order_pair2.insert(pair_symbol.to_string(), order_info);
                push_text = format!("add_observe_list: market {},shape_score {},volume_score {},recent_shape_score {},taker_amount {}",
                                    pair_symbol, shape_score, volume_score, recent_shape_score, taker_amount
                );
            }
            warn!("now {} , {}",timestamp2date(now),push_text);
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
        info!("Have no obvious break signal");
    }
    Ok(false)
}

//执行平单策略，并且返回是否继续和收益
pub async fn buy(
    take_order_pair2: &mut HashMap<String, TakeOrderInfo>,
    pair_symbol: &str,
    line_datas: &[Kline],
    is_real_trading: bool,
) -> Result<(bool,f32), Box<dyn std::error::Error>> {
    let now = line_datas[359].open_time + 1000;
    match take_order_pair2.get(pair_symbol) {
        None => {}
        Some(take_info) => {
            if take_info.is_took == true {
                let price_raise_ratio = line_datas[KLINE_NUM_FOR_FIND_SIGNAL - 1]
                    .open_price
                    .to_f32()
                    / take_info.price;
                //20X情况下：0.4个点止损,高峰之后根据20根k线之后，价格是否大于10根之前的价格5次这种情况就止盈
                if line_datas[KLINE_NUM_FOR_FIND_SIGNAL - 20].open_time > take_info.take_time
                        && get_raise_bar_num(&line_datas[KLINE_NUM_FOR_FIND_SIGNAL - 20..]) >= 6
                {
                    let push_text = format!(
                        "take_buy_order: market {},price_raise_ratio {}",
                        pair_symbol, price_raise_ratio
                    );
                    //fixme: 这里remove会报错
                    //take_order_pair2.remove(pair_symbol);
                    if is_real_trading {
                        take_order(pair_symbol.to_string(), take_info.amount, "BUY".to_string())
                            .await;
                        notify_lark(push_text).await?;
                    } else {
                        warn!("now {} , {}",timestamp2date(now),push_text);
                    }
                    take_order_pair2.remove(pair_symbol);
                    return Ok((true,1.0 - price_raise_ratio));
                } else if now.sub(take_info.take_time) < 1200000 {
                    //20分钟内不允许再次下单
                    return Ok((true,0.0));
                } else {
                }
            } else {
                //加入观察列表五分钟内不在观察，40分钟内仍没有二次拉起的则将其移除观察列表
                if now.sub(take_info.take_time) < 300000 {
                    return Ok((true,0.0));
                } else if now.sub(take_info.take_time) > 1200000 {
                    take_order_pair2.remove(pair_symbol);
                } else {
                }
            }
        }
    }
    Ok((false,0.0))
}
