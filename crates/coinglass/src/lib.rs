use std::thread;
use std::time::Duration;

use headless_chrome::protocol::cdp::Page;
use headless_chrome::{Browser, LaunchOptions};

pub fn login(login: &str, password: &str) -> anyhow::Result<()> {
	let browser = Browser::default()?;

	let tab = browser.new_tab()?;

	// tab.navigate_to("https://www.coinglass.com/login")?;
	tab.navigate_to("https://www.coinglass.com/tv")?;
	tab.wait_for_element(".tv-head-item")?;
	thread::sleep(Duration::from_secs(5));

	// tab.wait_for_element("input[name='email']")?.click()?;
	// tab.type_str(login)?;

	// tab.wait_for_element("input[name='password']")?.click()?;
	// tab.type_str(password)?;

	// tab.wait_for_element("button.MuiButton-root:nth-child(6)")?.click()?;

	Ok(())
}

pub fn get_chart_screenshot(pair: &str) -> anyhow::Result<Vec<u8>> {
	let launch_options = LaunchOptions::default_builder()
		.window_size(Some((1920, 1080)))
		.build()
		.map_err(|error| anyhow::anyhow!("Failed to build LaunchOptions: {error}"))?;

	let browser = Browser::new(launch_options)?;
	let tab = browser.new_tab()?;

	tab.navigate_to(&format!("https://www.coinglass.com/tv/Binance_{pair}"))?;
	tab.wait_for_element(".tv-head-item")?;
	thread::sleep(Duration::from_secs(5));

	let screenshot = tab.capture_screenshot(
		Page::CaptureScreenshotFormatOption::Jpeg,
		None, // quality (optional for JPEG)
		None, // clip rectangle (None = full page)
		true, // from_surface (better quality)
	)?;

	Ok(screenshot)
}
