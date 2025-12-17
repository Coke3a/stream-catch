use crate::domain::entities::live_accounts::LiveAccountEntity;
use anyhow::Result;
use serde_json::json;
use thirtyfour::error::WebDriverResult;
use thirtyfour::extensions::cdp::ChromeDevTools;
use thirtyfour::prelude::ElementWaitable;
use thirtyfour::prelude::Key;
use thirtyfour::{By, DesiredCapabilities, WebDriver};
use tracing::info;
use url::Url;

pub async fn add_account_recording_engine(
    insert_urls: String,
    account_entities: Vec<LiveAccountEntity>,
) -> Result<(Vec<LiveAccountEntity>, Option<Vec<LiveAccountEntity>>)> {
    let driver = initialize_driver().await?;
    access_url(&driver).await?;
    add_account(&driver, insert_urls).await?;
    let mut added_accounts = Vec::new();
    let mut failed_accounts = Vec::new();
    for account in account_entities {
        if check_account_is_added(&driver, &account.account_id, &account.platform).await? {
            info!("Account {} already added", account.account_id);
            added_accounts.push(account);
        } else {
            info!("Account {} not added", account.account_id);
            // screenshot_debug(&driver, &format!("failed_{}.png", account.account_id)).await?;
            failed_accounts.push(account);
        }
    }
    driver.quit().await?;
    Ok((added_accounts, Some(failed_accounts)))
}

async fn initialize_driver() -> Result<WebDriver> {
    let caps = DesiredCapabilities::chrome();
    let driver = WebDriver::new("http://selenium-chrome:4444", caps).await?;
    driver.maximize_window().await?;
    Ok(driver)
}

async fn access_url(driver: &WebDriver) -> Result<()> {
    let devtools = ChromeDevTools::new(driver.handle.clone());
    devtools.execute_cdp("Network.enable").await?;
    devtools
        .execute_cdp_with_params(
            "Network.setExtraHTTPHeaders",
            json!({"headers": {"Authorization": "Basic dXNlcm5hbWU6cGFzc3dvcmQ="}}),
        )
        .await?;
    let url_str = format!("http://orec-orec-1:5202/channels/");
    let url = Url::parse(&url_str)?;
    driver.goto(url.as_str()).await?;
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    Ok(())
}

async fn add_account(driver: &WebDriver, insert_urls: String) -> Result<()> {
    let add_button = driver.find(By::Css("button.MuiButtonBase-root.MuiButton-root.MuiButton-contained.MuiButton-containedPrimary.MuiButton-sizeSmall.MuiButton-containedSizeSmall.MuiButton-colorPrimary.MuiButton-root.MuiButton-contained.MuiButton-containedPrimary.MuiButton-sizeSmall.MuiButton-containedSizeSmall.MuiButton-colorPrimary.css-1l10thz")).await?;
    add_button.click().await?;
    let input_tag = driver
        .find(By::Css(
            "div.MuiDialogContent-root.css-1nbx5hx > div > div > div",
        ))
        .await?;
    input_tag.wait_until().clickable().await?;
    input_tag.click().await?;
    let name_input = driver.find(By::Css("textarea#url")).await?;
    name_input.send_keys(insert_urls).await?;
    let add_confirm_button = driver.find(By::Css("button.MuiButtonBase-root.MuiButton-root.MuiButton-contained.MuiButton-containedPrimary.MuiButton-sizeMedium.MuiButton-containedSizeMedium.MuiButton-colorPrimary.MuiButton-root.MuiButton-contained.MuiButton-containedPrimary.MuiButton-sizeMedium.MuiButton-containedSizeMedium.MuiButton-colorPrimary.css-5y2zdi")).await?;
    add_confirm_button.click().await?;
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    Ok(())
}

async fn check_account_is_added(
    driver: &WebDriver,
    username: &str,
    platform: &str,
) -> WebDriverResult<bool> {
    let search_element = driver
        .find(By::Css(
            "div.MuiBox-root.css-67s2z9 > div > div > div > div > input",
        ))
        .await?;
    search_element.send_keys(Key::Control + "a").await?;
    search_element.send_keys(Key::Backspace).await?;
    search_element.send_keys(username).await?;
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    let xpath = format!(
        "//div[contains(@class,'virtuoso-grid-item')]
         [.//a[.='{username}']]
         [.//span[.='{platform}']]",
        username = username,
        platform = platform,
    );
    let cards = driver.find_all(By::XPath(&xpath)).await?;
    Ok(!cards.is_empty())
}

// screenshot for debug
// async fn screenshot_debug(driver: &WebDriver, file_name: &str) -> Result<()> {
//     let png_bytes = driver.screenshot_as_png().await?;
//     tokio::fs::write(file_name, &png_bytes).await?;
//     println!("Saved {}", file_name);
//     Ok(())
// }
