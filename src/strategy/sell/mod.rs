pub mod a_strong_signal;
pub mod raise_is_stop;
pub mod three_continuous_signal;
pub mod two_middle_signal;

use crate::ex_info::Symbol;
use async_trait::async_trait;
use std::collections::HashMap;
use crate::{Kline, Pair, TakeOrderInfo};

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct TakeType{
    pub pair: String,
    pub sell_reason:SellReason
}

//context is the bar detail
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum SellReason{
    AStrongSignal,
    TwoMiddleSignal,
    ThreeContinuousSignal,
    RaiseIsStop
}

impl SellReason {
    pub fn to_string(&self) -> String{
        match self {
            SellReason::AStrongSignal => {
                "AStrongSignal".to_string()
            }
            SellReason::TwoMiddleSignal => {
                "TwoMiddleSignal".to_string()
            }
            SellReason::ThreeContinuousSignal => {
                "ThreeContinuousSignal".to_string()
            }
            SellReason::RaiseIsStop => {
                "RaiseIsStop".to_string()
            }
        }
    }
}

#[async_trait]
pub trait SellStrategy {
    fn name() -> SellReason;
    //fixme: 目前trait的async函数实现有问题
    async fn condition_passed<'a>(take_order_pair2: &'a mut HashMap<TakeType, Vec<TakeOrderInfo>>,
                              line_datas: &[Kline],
                              pair: &Symbol,
                              balance: f32,
                              is_real_trading: bool) -> Result<bool, Box<dyn std::error::Error>>;
}