use crate::utils::MathOperation2;
use crate::Kline;
use std::ops::Div;
use log::{debug, error, log_enabled, info, Level};



/// 根据bar的数据得出对应的单根形态
//当前分数计算是根据空单预期计算的，满分五分，强吊尾为6分，强阳线为0分

pub fn get_last_bar_shape_score(bars: Vec<Kline>) -> u8 {
    let last_bar = bars.as_slice().get(18).unwrap().to_owned();
    let last_bar_len = last_bar.high_price.to_f32() - last_bar.low_price.to_f32();
    let pre_last_bar = bars.as_slice().get(17).unwrap().to_owned();
    let pre_last_bar_len = pre_last_bar.high_price.to_f32() - pre_last_bar.low_price.to_f32();

    let mut score = 0;
    //阴线
    if last_bar.open_price.to_f32() > last_bar.close_price.to_f32() {
        score += 1
    }
    //收尾比之前低
    if last_bar.close_price.to_f32() < pre_last_bar.close_price.to_f32() {
        score += 1
    }
    //当前有触顶部
    if last_bar.high_price.to_f32() > pre_last_bar.high_price.to_f32() {
        score += 1
    }

    //击穿上一根的启动价格
    if last_bar.close_price.to_f32() < pre_last_bar.open_price.to_f32() {
        score += 1
    }

    //最后一根的长度大于前一根
    if last_bar_len / pre_last_bar_len > 1.0 {
        score += 1
    }

    //如果是上吊尾形态+2
    //Have no take order signal,below is detail score:market MANAUSDT,shape_score 0,volume_score 4,recent_shape_score 6
    // data_0002 1675364632830//这个时候的形态计算不对
    let diaowei_ratio = last_bar.high_price.to_f32()
        - last_bar.close_price.to_f32() / last_bar.close_price.to_f32()
        - last_bar.low_price.to_f32();
    if last_bar.close_price.to_f32() == last_bar.low_price.to_f32() || diaowei_ratio > 2.0 {
        score += 2;
        //如果open等于high，而且close不等于low，则可能是有抄底资金进入,谨慎打分
    } else if last_bar.open_price.to_f32() == last_bar.high_price.to_f32() {
        if diaowei_ratio > 3.0 {
            score += 1;
        } else {
            score = 0;
            info!("last bar shape get zero score,last bar detail {:?}",last_bar);
        }
    } else {
    }
    score
}

//达到最近五根1分钟k线的2倍的为五分，1.2倍的为4分，0.8倍为3分，其他为0
//最后一根为当前未完成的，不计算
pub fn get_last_bar_volume_score(bars: Vec<Kline>) -> u8 {
    let last_bar = bars.as_slice().get(18).unwrap().to_owned();
    let recent_volume = bars[13..=17]
        .iter()
        .map(|x| x.volume.parse::<f32>().unwrap())
        .sum::<f32>()
        .div(5.0f32);

    if last_bar.volume.to_f32() / recent_volume >= 2.0 {
        5
    } else if last_bar.volume.to_f32() / recent_volume >= 1.2 {
        4
    } else if last_bar.volume.to_f32() / recent_volume >= 0.8 {
        3
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn is_strong_signal() {
        todo!()
    }
}
