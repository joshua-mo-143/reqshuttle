use sqlx::PgPool;
use std::time::{Duration as StdDuration};
use chrono::{naive::NaiveDate, Local, Days};
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
async fn main(
	#[shuttle_shared_db::Postgres] db: PgPool,
) -> Result<CustomService, shuttle_runtime::Error> {
    let ctx = Client::new();
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

async fn scrape(ctx: Client, db: PgPool) -> Result<(), String> {
    loop {
        let mut vec: Vec<Product> = Vec::new();
        let mut pagenum = 1;
        loop {
            let url = format!("https://www.amazon.com/s?k=raspberry+pi&page={pagenum}");

            let res = match ctx.get(url).send().await {
		Ok(res) => res,
		Err(e) => {
		error!("Error while attempting to send HTTP request: {e}");
		break	
			}};
		
	   let res = match res.text().await {
		Ok(res) => res,
		Err(e) => {
		error!("Error while attempting to send HTTP request: {e}");
		}};

            let html = Html::parse_fragment(&res);
            let selector = Selector::parse("div[data-component-type='s-search-result']").unwrap();

            if html.select(&selector).count() == 0 {
                break;
            };

            for entry in html.select(&selector) {
                let price_selector = Selector::parse("span.a-price > span.a-offscreen").unwrap();
                let productname_selector = Selector::parse("h2 > a").unwrap();
		let name = entry.select(&productname_selector).next().expect("Couldn't find the product name").text.next().unwrap().to_string();
                let price_text = entry
                    .select(&price_selector)
                    .map(|x| x.text().next().unwrap().to_string())
                    .collect::<Vec<String>>();
                let scraped_at = Local::now().date_naive();
		let link = entry
                        .select(&productname_selector)
                        .map(|link| {
                            format!("https://amazon.co.uk{}", link.value().attr("href").unwrap())
                        })
                        .collect::<String>();

                vec.push(Product {
                    name,
                    price: price_text[0].clone(),
                    old_price: Some(price_text[1].clone()),
                    link,
                    scraped_at,
                });
            }
            pagenum += 1;
            std::thread::sleep(StdDuration::from_secs(20));
        }
	
	let transaction = db.begin().await.unwrap();
	
	for product in vec {
	if let Err(e) = sqlx::query("INSERT INTO 
		products
		(name, price, old_price, link, scraped_at)
		VALUES
		($1, $2, $3, $4, $5)
		")
		.bind(product.name)
		.bind(product.price)
		.bind(product.old_price)
		.bind(product.link)
		.bind(product.scraped_at)
		.execute(&db)
		.await
		.unwrap() {
		error!("There was an error: {e}");
		error!("This web scraper will now shut down.");
		transaction.rollback().await.unwrap();
		break
	}
	}
	transaction.commit().await.unwrap();
	
    let tomorrow_midnight = Local::now()
        .checked_add_days(Days::new(1))
        .unwrap()
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .unwrap();
    let now = Local::now().naive_local();

   let duration_to_midnight =  tomorrow_midnight.signed_duration_since(now).to_std().unwrap();
    sleep(Duration::from_secs(duration_to_midnight.as_secs())).await;
    }


    Ok(())
}
