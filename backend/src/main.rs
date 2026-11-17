use std::net::SocketAddr;
use std::sync::Arc;

use migration::{Migrator, MigratorTrait};
use tracing_subscriber::EnvFilter;

use dive_manager_backend::config::Config;
use dive_manager_backend::mailer::Mailer;
use dive_manager_backend::rate_limit::RateLimiter;
use dive_manager_backend::{app, bootstrap, maintenance, AppState};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            // sqlx logs every statement at INFO — too noisy for production.
            EnvFilter::try_from_default_env().unwrap_or_else(|_| "info,sqlx=warn".into()),
        )
        .init();

    if let Err(error) = run().await {
        tracing::error!(error, "fatal");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), String> {
    let config = Config::load()?;
    let db = sea_orm::Database::connect(&config.database.url)
        .await
        .map_err(|e| format!("database connection failed: {e}"))?;

    let command = std::env::args().nth(1).unwrap_or_else(|| "serve".into());
    match command.as_str() {
        "migrate" => {
            Migrator::up(&db, None)
                .await
                .map_err(|e| format!("migration failed: {e}"))?;
            tracing::info!("migrations applied");
            Ok(())
        }
        "serve" => serve(config, db).await,
        other => Err(format!(
            "unknown command: {other} (expected: serve | migrate)"
        )),
    }
}

async fn serve(config: Config, db: sea_orm::DatabaseConnection) -> Result<(), String> {
    if config.database.migrate_on_boot {
        Migrator::up(&db, None)
            .await
            .map_err(|e| format!("migration failed: {e}"))?;
    }

    bootstrap::bootstrap_global_admins(&db, &config)
        .await
        .map_err(|e| format!("bootstrap failed: {e}"))?;

    let mailer = Mailer::from_config(config.smtp.as_ref())?;
    if matches!(mailer, Mailer::DevLog) {
        tracing::warn!("no SMTP configured: emails are logged, not sent");
    }

    maintenance::spawn_cleanup_task(db.clone());

    let addr: SocketAddr = format!("{}:{}", config.server.host, config.server.port)
        .parse()
        .map_err(|e| format!("invalid server address: {e}"))?;
    let state = AppState {
        db,
        config: Arc::new(config),
        mailer: Arc::new(mailer),
        rate: Arc::new(RateLimiter::default()),
        oidc: Default::default(),
    };
    let router = app::build_router(state);

    tracing::info!(%addr, "dive-manager listening");
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| format!("bind failed: {e}"))?;
    axum::serve(
        listener,
        router.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .map_err(|e| format!("server error: {e}"))
}
