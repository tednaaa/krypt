mod binance;

trait Exchange {
	fn get_funding_rate_info(&self, symbol: &str) -> anyhow::Result<FundingRateInfo>;
	fn get_open_interest_info(&self, symbol: &str) -> anyhow::Result<OpenInterestInfo>;
}

struct FundingRateInfo {
	funding_rate: String,
	funding_interval_hours: String,
	funding_rate_percent_change_1_hour: String,
	funding_rate_percent_change_4_hours: String,
}

struct OpenInterestInfo {
	open_interest_percent_change_15_minutes: String,
	open_interest_percent_change_1_hour: String,
	open_interest_percent_change_4_hours: String,
	open_interest_percent_change_30_days: String,
}
