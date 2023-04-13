use super::SellReason;
use super::SellStrategy;
use crate::ex_info::Symbol;
use crate::utils::timestamp2date;
use crate::{
    get_last_bar_shape_score, get_last_bar_volume_score, notify_lark, recent_kline_shape_score,
    take_order, Kline, MathOperation, MathOperation2, Pair, TakeOrderInfo, TakeType,
};
use std::collections::HashMap;
use std::error::Error;
use std::ops::{Div, Mul};

pub struct ASS_V2 {}

impl ASS_V2 {
    fn name() -> SellReason {
        SellReason::AStrongSignal_V2
    }

    pub async fn condition_passed(
        take_order_pair: &mut HashMap<TakeType, Vec<TakeOrderInfo>>,
        line_datas: &[Kline],
        pair: &Symbol,
        taker_amount: f32,
        price:f32,
        is_real_trading: bool,
    ) -> Result<bool, Box<dyn Error>> {
        let pair_symbol = pair.symbol.as_str();
        let now = line_datas[359].open_time + 1000;
        let half_hour_inc_ratio =
            (line_datas[357].open_price.to_f32() - line_datas[327].open_price.to_f32()).div(30.0);
        let ten_minutes_inc_ratio =
            (line_datas[357].open_price.to_f32() - line_datas[347].open_price.to_f32()).div(10.0);

        let broken_line_datas = &line_datas[339..359];
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
        if shape_score >= 6
            && volume_score >= 6
            && recent_shape_score >= 6
            && line_datas[358].open_price.to_f32() > line_datas[358].close_price.to_f32()
            && line_datas[357].close_price.to_f32() > line_datas[358].close_price.to_f32()
            && line_datas[358].volume.to_f32().mul(3.0) > line_datas[357].volume.to_f32()
        {
            let inc_ratio_distance = ten_minutes_inc_ratio.div(half_hour_inc_ratio);
            if inc_ratio_distance < 1.4 {
                warn!(
                    "strategy3-{}-{}-deny: inc_ratio_distance {}",
                    pair_symbol,
                    timestamp2date(now),
                    inc_ratio_distance
                );
                return Ok(false);
            } else {
                warn!(
                    "strategy3-{}-{}-allow: inc_ratio_distance {}",
                    pair_symbol,
                    timestamp2date(now),
                    inc_ratio_distance
                );
            }

            //以倒数第二根的open，作为信号发现价格，以倒数第一根的open为实际下单价格
            let mut push_text = "".to_string();
            if is_real_trading {
                take_order(pair_symbol.to_string(), taker_amount, "SELL".to_string()).await;
            }
            let order_info = TakeOrderInfo {
                take_time: now,
                sell_price: price,
                buy_price: None,
                amount: taker_amount,
                top_bar: broken_line_datas[18].clone(),
                is_took: true,
            };
            let take_type = TakeType {
                pair: pair_symbol.to_string(),
                sell_reason: Self::name(),
            };
            take_order_pair.insert(take_type, vec![order_info]);
            push_text = format!("reason {}: take_sell_order: market {},shape_score {},volume_score {},recent_shape_score {},taker_amount {}",
                                <&str>::from(Self::name()),pair_symbol, shape_score, volume_score, recent_shape_score, taker_amount
            );
            warn!("now {}, {}", timestamp2date(now), push_text);
            if is_real_trading {
                notify_lark(push_text).await?;
            }
        } else {
                debug!("Have no take order signal,below is detail score:\
                                     market {},shape_score {},volume_score {},recent_shape_score {}",
                                    pair_symbol,shape_score,volume_score,recent_shape_score)
        }
        Ok(false)
    }
}
