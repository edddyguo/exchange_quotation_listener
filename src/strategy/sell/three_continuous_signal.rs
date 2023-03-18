use super::SellReason;
use super::SellStrategy;
use crate::ex_info::Symbol;
use crate::{
    get_last_bar_shape_score, get_last_bar_volume_score, notify_lark, recent_kline_shape_score,
    take_order, Kline, MathOperation, MathOperation2, Pair, TakeOrderInfo, TakeType,
};
use std::collections::HashMap;
use std::error::Error;
use std::ops::{Div, Mul};

struct TCS {}

// 三连阴对交易量有要求不能低于20分之一
impl TCS {
    fn name() -> SellReason {
        SellReason::ThreeContinuousSignal
    }

    pub async fn condition_passed(
        take_order_pair2: &mut HashMap<TakeType, Vec<TakeOrderInfo>>,
        line_datas: &[Kline],
        pair: &Symbol,
        balance: f32,
        is_real_trading: bool,
    ) -> Result<bool, Box<dyn Error>> {
        todo!()
    }
}
