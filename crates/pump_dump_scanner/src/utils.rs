pub(crate) fn extract_coin_from_pair(pair: &str) -> &str {
	pair.strip_suffix("USDT").unwrap_or(pair)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn when_connected_usdt() {
		let pair = "ZBTUSDT";
		let coin = extract_coin_from_pair(pair);
		assert_eq!(coin, "ZBT");
	}
}
