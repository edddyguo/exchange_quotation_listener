use crate::{try_get, Kline};
/// 算出来对应的k线形态和做空信号得分
//GET /fapi/v1/ticker/price
//symbol	STRING	NO	交易对
use serde::Deserialize;
use serde::Serialize;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Price {
    pub symbol: String,
    pub price: String,
    pub time: i64,
}

async fn try_get2(kline_url: String) -> Price{
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
    let price_info = try_get2(url).await;
    //println!("get_current_price result {:?}",res);
    //let line_datas = try_get(kline_url).await;

    price_info.price.parse::<f32>().unwrap()
}

//根据之前10根的k线情况给分
pub fn recent_kline_shape_score(bars: Vec<Kline>) -> u8 {
    assert_eq!(bars.len(), 11, "must be 10 item");
    let mut score = 0.0f32;
    //1分钟k线中拥有五连阳的
    for (index, line_data) in bars.iter().enumerate() {
        //if (index > 0 && line_data.close_price <= bars[index - 1].close_price)
        //    || line_data.close_price <= line_data.open_price
        if index > 0
            && line_data.close_price >= line_data.open_price
            && line_data.close_price >= bars[index - 1].close_price
        {
            score += 1.0;
        }
    }
    score.floor() as u8
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
