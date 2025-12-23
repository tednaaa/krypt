pub fn calculate_percent_change(old: f64, new: f64) -> String {
	if old == 0.0 {
		return "0".to_string();
	}
	let change = ((new - old) / old.abs()) * 100.0;
	format!("{change:.4}")
}
