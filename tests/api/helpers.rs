use std::sync::LazyLock;

use argon2::{password_hash::SaltString, Argon2, PasswordHasher};
use fake::Fake;
use hyper::StatusCode;
use serde::Serialize;
use sqlx::MySqlPool;
use uuid::Uuid;
use wiremock::MockServer;
use zero2prod::{
    configuration::get_configuration,
    domain::UserId,
    routes::PasswordFormData,
    startup::Application,
    telementry::{get_subscriber, init_subscriber},
};

static TRACING: LazyLock<()> = LazyLock::new(|| {
    let default_filter_level = "debug".to_string();
    let subscriber_name = "test".to_string();
    match std::env::var("TEST_LOG") {
        Ok(var) if var.to_lowercase() == "enabled" => {
            let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::stdout);
            init_subscriber(subscriber);
        }
        _ => {
            let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::sink);
            init_subscriber(subscriber);
        }
    }
});

#[derive(Debug)]
pub struct TestApp {
    pub address: String,
    pub db_pool: sqlx::MySqlPool,
    pub email_server: MockServer,
    pub port: u16,
    pub test_user: TestUser,
    // `cookie_store` is enabled and `redirect` is disabled
    pub api_client: reqwest::Client,
}

#[derive(Debug)]
pub struct TestUser {
    pub user_id: UserId,
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone)]
pub struct ConfirmationLink {
    pub html: reqwest::Url,
    #[allow(dead_code)]
    pub plain_text: reqwest::Url,
}

impl TestApp {
    pub async fn post_subscriptions(&self, body: impl Into<reqwest::Body>) -> reqwest::Response {
        self.api_client
            .post(format!("{}/subscriptions", &self.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("Failed to execute request")
    }

    pub async fn post_publish_newsletters(&self, form: &serde_json::Value) -> reqwest::Response {
        self.api_client
            .post(format!("{}/admin/newsletters", self.address))
            .form(form)
            .send()
            .await
            .expect("Failed to execute request")
    }

    pub async fn get_publish_newsletters(&self) -> reqwest::Response {
        self.api_client
            .get(format!("{}/admin/newsletters", &self.address))
            .send()
            .await
            .expect("Failed to execute request")
    }

    pub async fn get_publish_newsletters_html(&self) -> String {
        self.get_publish_newsletters().await.text().await.unwrap()
    }

    pub async fn get_confirmation_links(
        &self,
        email_request: &wiremock::Request,
    ) -> ConfirmationLink {
        let body: serde_json::Value = serde_json::from_slice(&email_request.body).unwrap();

        let get_link = |s: &str| {
            let links: Vec<_> = linkify::LinkFinder::new()
                .links(s)
                .filter(|l| *l.kind() == linkify::LinkKind::Url)
                .collect();
            assert_eq!(1, links.len());
            let raw_link = links[0].as_str().to_owned();
            let mut confirmation_link = reqwest::Url::parse(&raw_link).unwrap();
            assert_eq!("127.0.0.1", confirmation_link.host_str().unwrap());
            confirmation_link.set_port(Some(self.port)).unwrap();
            confirmation_link
        };

        let html = get_link(body["HtmlBody"].as_str().unwrap());
        let plain_text = get_link(body["TextBody"].as_str().unwrap());

        ConfirmationLink { html, plain_text }
    }

    pub async fn post_login(&self, body: &(impl Serialize + ?Sized)) -> reqwest::Response {
        self.api_client
            .post(format!("{}/login", self.address))
            .form(body)
            .send()
            .await
            .expect("Failed to execute request")
    }

    pub async fn get_login(&self) -> reqwest::Response {
        self.api_client
            .get(format!("{}/login", self.address))
            .send()
            .await
            .expect("Failed to execute request")
    }

    pub async fn get_login_html(&self) -> String {
        self.get_login().await.text().await.unwrap()
    }

    pub async fn get_admin_dashboard(&self) -> reqwest::Response {
        self.api_client
            .get(format!("{}/admin/dashboard", self.address))
            .send()
            .await
            .expect("Failed to execute request")
    }

    pub async fn get_admin_dashboard_html(&self) -> String {
        self.get_admin_dashboard().await.text().await.unwrap()
    }

    pub async fn get_change_password(&self) -> reqwest::Response {
        self.api_client
            .get(format!("{}/admin/password", self.address))
            .send()
            .await
            .expect("Failed to execute request")
    }

    pub async fn get_change_password_html(&self) -> String {
        self.get_change_password().await.text().await.unwrap()
    }

    pub async fn post_change_password<Body>(&self, body: &Body) -> reqwest::Response
    where
        Body: Serialize,
    {
        self.api_client
            .post(format!("{}/admin/password", self.address))
            .form(body)
            .send()
            .await
            .expect("Failed to execute request")
    }

    pub async fn post_logout(&self) -> reqwest::Response {
        self.api_client
            .post(format!("{}/admin/logout", self.address))
            .send()
            .await
            .expect("Failed to execute request")
    }
}

impl TestUser {
    pub fn generate() -> Self {
        Self {
            user_id: UserId::new_v4(),
            username: fake::faker::name::en::Name().fake(),
            password: Uuid::new_v4().into(),
        }
    }

    async fn store(&self, pool: &MySqlPool) {
        let salt = SaltString::generate(&mut rand::thread_rng());
        let password_hash = Argon2::default()
            .hash_password(self.password.as_bytes(), &salt)
            .unwrap()
            .to_string();

        sqlx::query!(
            r#"INSERT INTO users (user_id, username, password_hash) VALUES (? ,?, ?)"#,
            self.user_id,
            self.username,
            password_hash
        )
        .execute(pool)
        .await
        .expect("Failed to store test user");
    }

    pub async fn login(&self, app: &TestApp) {
        app.post_login(&serde_json::json!({
          "username": self.username,
          "password": self.password
        }))
        .await;
    }
}

pub async fn spawn_test_app(pool: MySqlPool) -> TestApp {
    LazyLock::force(&TRACING);

    let email_server = MockServer::start().await;

    let configuration = {
        let mut c = get_configuration().expect("Failed to read configuration.");
        c.database.database_name = pool
            .connect_options()
            .get_database()
            .expect("Failed to get the current database name")
            .to_owned();
        c.application.port = 0;
        c.email_client.base_url = email_server.uri().as_str().try_into().unwrap();
        c
    };

    let app = Application::build_with_db(configuration, pool)
        .await
        .expect("Failed to build the test application");
    let port = app.port();
    let address = format!("http://127.0.0.1:{}", app.port());
    let db_pool = app.db_pool();

    tokio::spawn(app.run());

    let test_user = TestUser::generate();
    test_user.store(&db_pool).await;

    let api_client = reqwest::ClientBuilder::new()
        .cookie_store(true)
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();

    TestApp {
        address,
        db_pool,
        email_server,
        port,
        test_user,
        api_client,
    }
}

#[track_caller]
pub fn assert_is_redirect_to(response: &reqwest::Response, location: &str) {
    assert_eq!(StatusCode::SEE_OTHER, response.status());
    assert_eq!(
        location,
        response
            .headers()
            .get("Location")
            .expect("Location is missing in headers")
    )
}

#[track_caller]
pub fn assert_publish_is_successful(html_page: &str) {
    assert!(html_page.contains(
        "The newsletter issue has been accepted - \
emails will go out shortly!",
    ))
}

pub fn generate_random_change_password_form() -> serde_json::Value {
    let current_password = Uuid::new_v4().to_string();
    let new_password = Uuid::new_v4().to_string();

    serde_json::json!({
      "current_password": current_password,
      "new_password": new_password,
      "new_password_check": new_password
    })
}

pub fn build_change_password_form(
    current_password: String,
    new_password: String,
    new_passwrod_check: String,
) -> PasswordFormData {
    PasswordFormData {
        current_password: current_password.into(),
        new_password: new_password.into(),
        new_password_check: new_passwrod_check.into(),
    }
}
