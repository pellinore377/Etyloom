#[tokio::main]
async fn main() -> anyhow::Result<()> {
    etyloom_web::server::run().await
}
