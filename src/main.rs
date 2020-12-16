use warp::{Filter, Rejection};

mod enums;
mod pool;

type WebResult<T> = std::result::Result<T, Rejection>;
type Result<T> = std::result::Result<T, enums::Error>;

const REDIS_CON_STRING: &str = "redis://127.0.0.1/";

#[tokio::main]
async fn main() {
    let mobc_pool = pool::connect().await.expect("can't create Mobc pool");

    let mobc_route = warp::path!("mobc")
        .and(pool::with_mobc_pool(mobc_pool.clone()))
        .and_then(pool::handler);

    warp::serve(mobc_route).run(([0, 0, 0, 0], 8080)).await;
}
