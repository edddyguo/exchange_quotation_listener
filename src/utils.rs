use chrono::Utc;

///
pub fn get_unix_timestamp_ms() -> i64 {
    let now = Utc::now();
    now.timestamp_millis()
}

#[cfg(test)]
mod tests{
    use crate::get_unix_timestamp_ms;

    #[test]
    fn test_get_unix_timestamp_ms(){
        let time  = get_unix_timestamp_ms();
        println!("{:?}",time)
    }
}