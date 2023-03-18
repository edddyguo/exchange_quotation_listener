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

struct RIS {}

impl RIS {
    fn name() -> SellReason {
        SellReason::RaiseIsStop
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
