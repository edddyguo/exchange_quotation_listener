use crate::{try_get, Kline, MathOperation2};
/// 算出来对应的k线形态和做空信号得分
//GET /fapi/v1/ticker/price
//symbol	STRING	NO	交易对
use serde::Deserialize;
use serde::Serialize;
use std::ops::{Deref, Div};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Price {
    pub symbol: String,
    pub price: String,
    pub time: i64,
}

async fn try_get2(kline_url: String) -> Price {
    let mut line_data;
    loop {
        match reqwest::get(&kline_url).await {
            Ok(res) => {
                //println!("url {},res {:?}", kline_url,res);
                let res_str = format!("{:?}", res);
                match res.json::<Price>().await {
                    Ok(data) => {
                        line_data = data;
                        break;
                    }
                    Err(error) => {
                        //println!("reqwest res string: {:?}",res_str);
                        println!(
                            "res deserialize happened error {},and raw res {}",
                            error.to_string(),
                            res_str
                        );
                    }
                }
            }
            Err(error) => {
                println!("reqwest get happened error {}", error.to_string());
            }
        }
    }
    line_data
}

pub async fn get_current_price(symbol: &str) -> f32 {
    let url = format!(
        "https://fapi.binance.com/fapi/v1/ticker/price?symbol={}",
        symbol
    );
    //let price_info = try_get2(url).await;
    let price_info = try_get::<Price>(url).await;

    //println!("get_current_price result {:?}",res);
    //let line_datas = try_get(kline_url).await;

    price_info.price.parse::<f32>().unwrap()
}

//根据之前10根的k线情况给分
pub fn recent_kline_shape_score(bars: Vec<Kline>) -> u8 {
    assert_eq!(bars.len(), 11, "must be 10 item");
    let mut score = 0u8;
    let mut score_tmp = 0u8;
    //1分钟k线中拥有五连阳的
    for (index, line_data) in bars.iter().enumerate() {
        //if (index > 0 && line_data.close_price <= bars[index - 1].close_price)
        //    || line_data.close_price <= line_data.open_price
        if index > 0
            && line_data.close_price >= line_data.open_price
            && line_data.close_price >= bars[index - 1].close_price
        {
            //最后五根必须至少三根阳
            if index >= 6 {
                score_tmp += 1;
            }
            score += 1;
        }
    }
    if score_tmp <= 2 {
        score = 0;
    }
    score
}

//获取数据的平均价格和成交量
pub fn get_average_info(klines: &[Kline]) -> (f32, f32) {
    let mut klines = klines.to_owned();
    klines.sort_by(|a, b| a.volume.to_f32().partial_cmp(&b.volume.to_f32()).unwrap());
    let len = klines.len();
    //剔除最大的那根
    let klines = &klines[0..len - 1];

    let average_volume = klines
        .iter()
        .map(|x| x.volume.parse::<f32>().unwrap())
        .sum::<f32>()
        .div(klines.len() as f32);

    let average_price = klines
        .iter()
        .map(|x| x.close_price.parse::<f32>().unwrap())
        .sum::<f32>()
        .div(klines.len() as f32);
    (average_price, average_volume)
}

//判断最近的交易量是否足够低迷，在平仓的时候使用
pub fn volume_too_few(klines: &[Kline], reference_volume: f32) -> bool {
    let mut sum_volume = 0.0;
    for kline in klines {
        if kline.volume.to_f32() > reference_volume.div(8.0) {
            return false;
        }
        sum_volume += kline.volume.to_f32();
    }
    if sum_volume.div(klines.len() as f32) > reference_volume.div(10.0) {
        false
    } else {
        true
    }
}

#[cfg(test)]
mod tests {
    use crate::kline::get_current_price;

    #[test]
    fn is_strong_signal() {
        todo!()
    }

    #[tokio::test]
    async fn test_get_current_price() {
        get_current_price("RLCUSDT").await;
    }
}
