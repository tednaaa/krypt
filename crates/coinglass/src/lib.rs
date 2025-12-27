use std::ffi::OsStr;
use std::thread;
use std::time::Duration;

use headless_chrome::protocol::cdp::Page::{self, CaptureScreenshotFormatOption};
use headless_chrome::{Browser, LaunchOptions};

pub fn login(login: &str, password: &str) -> anyhow::Result<()> {
	let launch_options = LaunchOptions::default_builder()
    .headless(true)  // Keep true
    .window_size(Some((1920, 1080)))
    .args(vec![
       OsStr::new("--headless=new"),
        OsStr::new("--no-sandbox"),
        OsStr::new("--disable-setuid-sandbox"),
        OsStr::new("--disable-infobars"),
        OsStr::new("--disable-blink-features=AutomationControlled"),
        OsStr::new("--user-agent=Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/128.0.0.0 Safari/537.36"),
    ])
    .build()?;

	let browser = Browser::new(launch_options)?;
	let tab = browser.new_tab()?;

	tab.evaluate(r"Object.defineProperty(navigator, 'webdriver', { get: () => false });", false)?;

	tab.evaluate(
		r"
    // Spoof plugins and languages
    Object.defineProperty(navigator, 'plugins', { get: () => [1, 2, 3, 4, 5] });
    Object.defineProperty(navigator, 'languages', { get: () => ['en-US', 'en'] });
    // Remove chrome.runtime indicators
    delete navigator.__proto__.webdriver;
    ",
		false,
	)?;

	tab.navigate_to("https://www.coinglass.com/login")?;
	tab.wait_for_element("input[name='email']")?.click()?;
	tab.type_str(login)?;
	tab.wait_for_element("input[name='password']")?.click()?;
	tab.type_str(password)?;
	tab.wait_for_element("button.MuiButton-root:nth-child(6)")?.click()?;

	let screenshot = tab.capture_screenshot(
		Page::CaptureScreenshotFormatOption::Jpeg,
		None, // quality (optional for JPEG)
		None, // clip rectangle (None = full page)
		true, // from_surface (better quality)
	)?;
	std::fs::write("login_screenshot.jpg", &screenshot)?;

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

pub fn get_liquidation_heatmap_screenshot(coin: &str) -> anyhow::Result<Vec<u8>> {
	let launch_options = LaunchOptions::default_builder()
		.window_size(Some((1920, 1080)))
		.build()
		.map_err(|error| anyhow::anyhow!("Failed to build LaunchOptions: {error}"))?;

	let browser = Browser::new(launch_options)?;
	let tab = browser.new_tab()?;

	let viewport = tab
		.navigate_to(&format!("https://www.coinglass.com/pro/futures/LiquidationHeatMap?type=pair&coin={coin}"))?
		.wait_for_element("canvas")?
		.get_box_model()?
		.margin_viewport();

	// thread::sleep(Duration::from_secs(3));

	let screenshot = tab.capture_screenshot(CaptureScreenshotFormatOption::Jpeg, None, Some(viewport), true)?;

	Ok(screenshot)
}
