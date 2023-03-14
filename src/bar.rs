use crate::utils::MathOperation2;
use crate::{get_average_info, Kline};
use log::{debug, error, info, log_enabled, Level};
use std::ops::{Div, Mul};

///根据最近10根的k线中是否出现2根大于index-5的情况来决定是否平仓
pub fn get_raise_bar_num(bars: &[Kline]) -> u8 {
    //assert_eq!(bars.len(), 60);
    let mut num = 0u8;
    for (index, bar) in bars.iter().enumerate() {
        if index >= 10 && bar.close_price > bars[index - 10].close_price {
            //warn!("index {} ,bar.close_price {} > bars[index - 10].close_price {}",index,bar.close_price, bars[index - 10].close_price);
            num += 1;
        }
    }
    num
}

//获取k线中巨量交易的k线
pub fn get_huge_volume_bar_num(bars: &[Kline], min_volume: f32, ration: f32) -> u8 {
    //except the last bar
    let (average_volume,average_price) = get_average_info(&bars[14..19]);
    let mut huge_volume_bars_num = 0;
    for (index, bar) in bars.iter().enumerate() {
        let increase_volume = (bar.volume.to_f32() - min_volume).div(min_volume);
        //临近10根的量大于远处的平均值五倍，算巨量
        if index >= 10 && increase_volume > ration {
            huge_volume_bars_num += 1;
            //临近5根中如果小于平均值的五分之一则，信号失效，不允许突破中太少的量
        } else if index >= 15 && bar.volume.to_f32().mul(5.0) < average_volume {
            return 0u8;
        }
    }
    huge_volume_bars_num
}

/// 根据bar的数据得出对应的单根形态
//当前分数计算是根据空单预期计算的，满分五分，强吊尾为6分，强阳线为0分
pub fn get_last_bar_shape_score(bars: Vec<Kline>) -> u8 {
    let last_bar = bars.as_slice().get(18).unwrap().to_owned();
    let last_bar_len = last_bar.high_price.to_f32() - last_bar.low_price.to_f32();
    let pre_last_bar = bars.as_slice().get(17).unwrap().to_owned();
    let pre_last_bar_len = pre_last_bar.high_price.to_f32() - pre_last_bar.low_price.to_f32();

    let mut score = 0;
    let mut score_detail = "score_detail: ".to_string();
    //收尾比之前低
    if last_bar.close_price.to_f32() < pre_last_bar.close_price.to_f32() {
        score += 1;
        score_detail = format!("{},B:+1", score_detail);
    }
    //当前有触顶部
    if last_bar.high_price.to_f32() > pre_last_bar.high_price.to_f32() {
        score += 1;
        score_detail = format!("{},C:+1", score_detail);
    }

    //击穿上一根的启动价格
    //todo: 和B项的判断是否重复了？
    /*    if last_bar.close_price.to_f32() < pre_last_bar.open_price.to_f32() {
        score += 1;
        score_detail = format!("{},D:+1",score_detail);
    }*/

    //最后一根的长度大于前一根
    if last_bar_len / pre_last_bar_len > 1.0 {
        score += 1;
        score_detail = format!("{},E:+1", score_detail);
    }

    //阴线
    if last_bar.open_price.to_f32() > last_bar.close_price.to_f32() {
        score += 1;
        score_detail = format!("{},A:+1", score_detail);
    }else{
        score = 0;
    }

    //如果是上吊尾形态+2
    //Have no take order signal,below is detail score:market MANAUSDT,shape_score 0,volume_score 4,recent_shape_score 6
    // data_0002 1675364632830//这个时候的形态计算不对
    let diaowei_ratio = last_bar.high_price.to_f32()
        - last_bar.close_price.to_f32() / last_bar.close_price.to_f32()
        - last_bar.low_price.to_f32();

    let diaowei_up_distance = last_bar.high_price.to_f32() - last_bar.close_price.to_f32();
    let diaowei_down_distance = last_bar.close_price.to_f32() - last_bar.low_price.to_f32();

    if diaowei_down_distance == 0.0 || diaowei_up_distance / diaowei_down_distance > 3.0 {
        score += 3;
        score_detail = format!("{},F:+2", score_detail);
        //如果open等于high，而且close不等于low，则可能是有抄底资金进入,谨慎打分
    } else if diaowei_up_distance / diaowei_down_distance > 2.0 {
        score += 1;
        score_detail = format!("{},F:+1", score_detail);
    } else if diaowei_up_distance / diaowei_down_distance <= 1.0 {
        score += 0;
        score_detail = format!("{},F:+0", score_detail);
    } else {
        score = 0;
        score_detail = format!("{},F:=0", score_detail);
    }
    info!("score_detail {}", score_detail);
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
