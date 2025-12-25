use std::thread;
use std::time::Duration;

use headless_chrome::Browser;
use headless_chrome::protocol::cdp::Page;

pub fn login_coinglass(login: &str, password: &str) -> anyhow::Result<()> {
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

	let jpeg_data = tab.capture_screenshot(Page::CaptureScreenshotFormatOption::Jpeg, None, None, true)?;
	std::fs::write("screenshot.jpeg", jpeg_data)?;

	Ok(())
}
