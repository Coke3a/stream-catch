use anyhow::Result;
use serde_json::json;
use thirtyfour::extensions::cdp::ChromeDevTools;
use thirtyfour::prelude::ElementWaitable;
use thirtyfour::{By, DesiredCapabilities, WebDriver};
use url::Url;

pub async fn add_account_recording_engine(insert_urls: String) -> Result<()> {
    let driver = initialize_driver().await?;
    access_url(&driver).await?;
    add_account(&driver, insert_urls).await?;
    screenshot_debug(&driver).await?; // for debug
    driver.quit().await?;
    Ok(())
}

async fn initialize_driver() -> Result<WebDriver> {
    let caps = DesiredCapabilities::chrome();
    let driver = WebDriver::new("http://localhost:4444", caps).await?;
    driver.maximize_window().await?;
    Ok(driver)
}

async fn screenshot_debug(driver: &WebDriver) -> Result<()> {
    let png_bytes = driver.screenshot_as_png().await?;
    tokio::fs::write("screenshot.png", &png_bytes).await?;
    println!("Saved screenshot.png");
    Ok(())
}

async fn access_url(driver: &WebDriver) -> Result<()> {
    let devtools = ChromeDevTools::new(driver.handle.clone());
    devtools.execute_cdp("Network.enable").await?;
    devtools.execute_cdp_with_params("Network.setExtraHTTPHeaders", json!({"headers": {"Authorization": "Basic dXNlcm5hbWU6cGFzc3dvcmQ="}})).await?;
    let url_str = format!("http://orec:5202/channels/");
    let url = Url::parse(&url_str)?;
    driver.goto(url.as_str()).await?;
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    Ok(())
}

async fn add_account(driver: &WebDriver, insert_urls: String) -> Result<()> {
    let add_button = driver.find(By::Css("button.MuiButtonBase-root.MuiButton-root.MuiButton-contained.MuiButton-containedPrimary.MuiButton-sizeSmall.MuiButton-containedSizeSmall.MuiButton-colorPrimary.MuiButton-root.MuiButton-contained.MuiButton-containedPrimary.MuiButton-sizeSmall.MuiButton-containedSizeSmall.MuiButton-colorPrimary.css-1l10thz")).await?;
    add_button.click().await?;
    let input_tag = driver.find(By::Css("div.MuiDialogContent-root.css-1nbx5hx > div > div > div")).await?;
    input_tag.wait_until().clickable().await?;
    input_tag.click().await?;
    let name_input = driver.find(By::Css("textarea#url")).await?;
    name_input.send_keys(insert_urls).await?;
    let add_confirm_button = driver.find(By::Css("button.MuiButtonBase-root.MuiButton-root.MuiButton-contained.MuiButton-containedPrimary.MuiButton-sizeMedium.MuiButton-containedSizeMedium.MuiButton-colorPrimary.MuiButton-root.MuiButton-contained.MuiButton-containedPrimary.MuiButton-sizeMedium.MuiButton-containedSizeMedium.MuiButton-colorPrimary.css-5y2zdi")).await?;
    add_confirm_button.click().await?;
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    Ok(())
}