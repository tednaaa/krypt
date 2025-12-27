use std::ffi::OsStr;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use headless_chrome::protocol::cdp::Page::{self, CaptureScreenshotFormatOption};
use headless_chrome::{Browser, LaunchOptions, Tab};

pub struct Coinglass {
	browser: Browser,
	tab: Arc<Tab>,
}

impl Coinglass {
	pub fn new() -> anyhow::Result<Self> {
		let launch_options = LaunchOptions::default_builder()
	    .headless(true)
	    .window_size(Some((1920, 1920)))
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

		Ok(Self { browser, tab })
	}

	pub fn login(&self, login: &str, password: &str) -> anyhow::Result<()> {
		self.tab.navigate_to("https://www.coinglass.com/login")?;
		self.tab.wait_for_element("input[name='email']")?.click()?;
		self.tab.type_str(login)?;
		self.tab.wait_for_element("input[name='password']")?.click()?;
		self.tab.type_str(password)?;
		self.tab.wait_for_element("button.MuiButton-root:nth-child(6)")?.click()?;

		let screenshot = self.tab.capture_screenshot(
			Page::CaptureScreenshotFormatOption::Jpeg,
			None, // quality (optional for JPEG)
			None, // clip rectangle (None = full page)
			true, // from_surface (better quality)
		)?;
		std::fs::write("login_screenshot.jpg", &screenshot)?;

		Ok(())
	}

	pub fn get_chart_screenshot(&self, pair: &str) -> anyhow::Result<Vec<u8>> {
		self.tab.navigate_to(&format!("https://www.coinglass.com/tv/Binance_{pair}"))?;
		self.tab.wait_for_element(".tv-head-item")?;
		thread::sleep(Duration::from_secs(5));

		let screenshot = self.tab.capture_screenshot(Page::CaptureScreenshotFormatOption::Jpeg, None, None, true)?;

		Ok(screenshot)
	}

	pub fn get_liquidation_heatmap_screenshot(&self, coin: &str) -> anyhow::Result<Vec<u8>> {
		let viewport = self
			.tab
			.navigate_to(&format!("https://www.coinglass.com/pro/futures/LiquidationHeatMap?type=pair&coin={coin}"))?
			.wait_for_element("canvas")?
			.get_box_model()?
			.margin_viewport();

		let screenshot = self.tab.capture_screenshot(CaptureScreenshotFormatOption::Jpeg, None, Some(viewport), true)?;

		Ok(screenshot)
	}
}
