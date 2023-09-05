use axum::{routing::get, Router};
use chrono::{naive::NaiveDate, Local};
use reqwest::Client;
use scraper::{Html, Selector};
use std::net::SocketAddr;
use tokio::time::{sleep, Duration};
use tracing::error;

pub struct CustomService {
    ctx: Client,
}

#[derive(Clone, Debug)]
pub struct Product {
    name: String,
    price: String,
    old_price: Option<String>,
    link: String,
    scraped_at: NaiveDate,
}

#[shuttle_runtime::main]
async fn axum() -> Result<CustomService, shuttle_runtime::Error> {
    let ctx = Client::new();
    Ok(CustomService { ctx })
}

#[shuttle_runtime::async_trait]
impl shuttle_runtime::Service for CustomService {
    async fn bind(mut self, _addr: SocketAddr) -> Result<(), shuttle_runtime::Error> {
        scrape(self.ctx).await.unwrap();
        error!("Something's happened! The scraper shouldn't finish.");
        Ok(())
    }
}

pub async fn scrape(ctx: Client) -> Result<(), String> {
    loop {
        let mut vec: Vec<Product> = Vec::new();
        let mut pagenum = 1;
        loop {
            let url = format!("https://www.amazon.com/s?k=raspberry+pi&page={pagenum}");

            let res = ctx.get(url).send().await.unwrap().text().await.unwrap();

            let html = Html::parse_fragment(&res);
            let selector = Selector::parse("div[data-component-type='s-search-result']").unwrap();

            if html.select(&selector).count() == 0 {
                break;
            };

            for entry in html.select(&selector) {
                let price_selector = Selector::parse("span.a-price > span.a-offscreen").unwrap();
                let productname_selector = Selector::parse("h2 > a").unwrap();

                let price_text = entry
                    .select(&price_selector)
                    .map(|x| x.text().next().unwrap().to_string())
                    .collect::<Vec<String>>();
                let today = Local::now().date_naive();

                vec.push(Product {
                    name: entry
                        .select(&productname_selector)
                        .next()
                        .expect("Couldn't find the product name!")
                        .text()
                        .next()
                        .unwrap()
                        .to_string(),
                    price: price_text[0].clone(),
                    old_price: Some(price_text[1].clone()),
                    link: entry
                        .select(&productname_selector)
                        .map(|link| {
                            format!("https://amazon.co.uk{}", link.value().attr("href").unwrap())
                        })
                        .collect::<String>(),
                    scraped_at: today,
                });
            }
            pagenum += 1;
            sleep(Duration::from_secs(20)).await;
        }
    }

    todo!();
}
