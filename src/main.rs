use chrono::Days;
use chrono::Local;
use reqwest::Client;
use reqwest::StatusCode;
use scraper::{Html, Selector};
use sqlx::PgPool;
use std::net::SocketAddr;
use std::thread::sleep as StdSleep;
use std::time::Duration as StdDuration;
use tokio::time::{sleep as TokioSleep, Duration as TokioDuration};
use tracing::{debug, error};

const USER_AGENT: &str = "Mozilla/5.0 (iPad; CPU OS 12_2 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Mobile/15E148";

pub struct CustomService {
    ctx: Client,
    db: PgPool,
}

#[derive(Clone, Debug)]
pub struct Product {
    name: String,
    price: String,
    old_price: Option<String>,
    link: String,
}

#[shuttle_runtime::main]
async fn axum(
    #[shuttle_shared_db::Postgres] db: PgPool,
) -> Result<CustomService, shuttle_runtime::Error> {
    sqlx::migrate!().run(&db).await.unwrap();

    let ctx = Client::builder().user_agent(USER_AGENT).build().unwrap();
    Ok(CustomService { ctx, db })
}

#[shuttle_runtime::async_trait]
impl shuttle_runtime::Service for CustomService {
    async fn bind(mut self, _addr: SocketAddr) -> Result<(), shuttle_runtime::Error> {
        scrape(self.ctx, self.db).await.unwrap();
        error!("Something's happened! The scraper shouldn't finish.");
        Ok(())
    }
}

pub async fn scrape(ctx: Client, db: PgPool) -> Result<(), String> {
    debug!("Starting scraper...");
    loop {
        let mut vec: Vec<Product> = Vec::new();
        let mut pagenum = 1;
        let mut retry_attempts = 0;
        loop {
            let url = format!("https://www.amazon.com/s?k=raspberry+pi&page={pagenum}");

            let res = match ctx.get(url).send().await {
                Ok(res) => res,
                Err(e) => {
                    error!("Something went wrong while fetching from url: {e}");
                    StdSleep(StdDuration::from_secs(15));
                    continue;
                }
            };

            if res.status() == StatusCode::SERVICE_UNAVAILABLE {
                error!("Amazon returned a 503 at page {pagenum}");
                retry_attempts += 1;
                if retry_attempts >= 10 {
                    error!("It looks like Amazon is blocking us! We will rest for an hour.");
                    StdSleep(StdDuration::from_secs(3600));
                    continue;
                } else {
                    StdSleep(StdDuration::from_secs(15));
                    continue;
                }
            }

            let body = match res.text().await {
                Ok(res) => res,
                Err(e) => {
                    error!("Something went wrong while turning data to text: {e}");
                    StdSleep(StdDuration::from_secs(15));
                    continue;
                }
            };

            debug!("Page {pagenum} was scraped");
            let html = Html::parse_fragment(&body);
            let selector =
                Selector::parse("div[data-component-type= ' s-search-result ' ]").unwrap();

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
                });
            }
            pagenum += 1;
            retry_attempts = 0;
            StdSleep(StdDuration::from_secs(15));
        }

        let transaction = db.begin().await.unwrap();

        for product in vec {
            if let Err(e) = sqlx::query(
                "INSERT INTO 
        products
       (name, price, old_price, link, scraped_at)
       VALUES
       ($1, $2, $3, $4, $5)
      ",
            )
            .bind(product.name)
            .bind(product.price)
            .bind(product.old_price)
            .bind(product.link)
            .execute(&db)
            .await
            {
                error!("There was an error: {e}");
                error!("This web scraper will now shut down.");
                break;
            }
        }
        transaction.commit().await.unwrap();

        // get the local time, add a day then get the NaiveDate and set a time of 00:00 to it
        let tomorrow_midnight = Local::now()
            .checked_add_days(Days::new(1))
            .unwrap()
            .date_naive()
            .and_hms_opt(0, 0, 0)
            .unwrap();

        // get the local time now
        let now = Local::now().naive_local();

        // check the amount of time between now and midnight tomorrow
        let duration_to_midnight = tomorrow_midnight
            .signed_duration_since(now)
            .to_std()
            .unwrap();

        // sleep for the required time
        TokioSleep(TokioDuration::from_secs(duration_to_midnight.as_secs())).await;
    }
    Ok(())
}
