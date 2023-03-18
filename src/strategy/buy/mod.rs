pub enum BuyReason {
    MoreThan2h,
    VolumeTooFewInRecentBars,
}
/*#[async_trait]
pub trait BuyStrategy {
    fn name() -> BuyReason;
    async fn condition_passed(&self) -> Result<(bool, f32), Box<dyn std::error::Error>>;
}*/
