use dotenv::dotenv;
use lettre::message::{header::ContentType, Message};
use lettre::transport::smtp::authentication::Credentials;
use lettre::transport::smtp::SmtpTransport;
use lettre::Transport;
use std::env;
use std::fmt::Display;
use tera::{Context, Tera};

struct EmailConfig {
    smtp_server: String,
    smtp_username: String,
    smtp_password: String,
    from_email: String,
    smtp_port: u16,
}

#[derive(Debug)]
pub enum EmailError {
    ConfigError(String),
    ConnectionError(String),
    TemplateError(String),
    SendError(String),
}

impl Display for EmailError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EmailError::ConfigError(e) => write!(f, "ConfigError: {}", e),
            EmailError::ConnectionError(e) => write!(f, "ConnectionError: {}", e),
            EmailError::TemplateError(e) => write!(f, "TemplateError: {}", e),
            EmailError::SendError(e) => write!(f, "SendError: {}", e),
        }
    }
}

impl std::error::Error for EmailError {}

fn load_email_config() -> std::result::Result<EmailConfig, EmailError> {
    dotenv().ok();
    let smtp_server = env::var("SMTP_SERVER")
        .map_err(|err| EmailError::ConfigError(format!("Error loading SMTP_SERVER: {}", err)))?;
    let smtp_port = env::var("SMTP_PORT")
        .map_err(|err| EmailError::ConfigError(format!("Error loading SMTP_PORT: {}", err)))?
        .parse::<u16>()
        .map_err(|err| EmailError::ConfigError(format!("Error parsing SMTP_PORT: {}", err)))?;
    let smtp_username = env::var("SMTP_USERNAME")
        .map_err(|err| EmailError::ConfigError(format!("Error loading SMTP_USERNAME: {}", err)))?;
    let smtp_password = env::var("SMTP_PASSWORD")
        .map_err(|err| EmailError::ConfigError(format!("Error loading SMTP_PASSWORD: {}", err)))?;
    let from_email = env::var("FROM_EMAIL")
        .map_err(|err| EmailError::ConfigError(format!("Error loading FROM_EMAIL: {}", err)))?;

    Ok(EmailConfig {
        smtp_server,
        smtp_username,
        smtp_password,
        from_email,
        smtp_port,
    })
}

pub async fn send_email_confirmation(to: &str, otp_code: &str) -> std::result::Result<(), EmailError> {
    let email_config = match load_email_config() {
        Ok(config) => config,
        Err(err) => return Err(err),
    };

    let mailer = match SmtpTransport::starttls_relay(&email_config.smtp_server) {
        Ok(mailer) => mailer,
        Err(err) => {
            return Err(EmailError::ConnectionError(format!(
                "Error connecting to SMTP server: {}",
                err
            )))
        }
    }
    .credentials(Credentials::new(
        email_config.smtp_username.clone(),
        email_config.smtp_password.clone(),
    ))
    .port(email_config.smtp_port)
    .build();

    let tera = match Tera::new("src/templates/en/*.html") {
        Ok(t) => t,
        Err(e) => {
            return Err(EmailError::TemplateError(format!(
                "Error creating template engine: {}",
                e
            )))
        }
    };

    let mut context = Context::new();
    context.insert("otp_code", &otp_code);

    let rendered_template = match tera.render("confirm_email.html", &context) {
        Ok(t) => t,
        Err(e) => {
            return Err(EmailError::TemplateError(format!(
                "Error rendering template: {}",
                e
            )))
        }
    };

    let email = Message::builder()
        .from(email_config.from_email.parse().unwrap())
        .to(to.parse().unwrap())
        .subject("Confirm your email")
        .header(ContentType::TEXT_HTML)
        .body(rendered_template)
        .unwrap();

    match mailer.send(&email) {
        Err(err) => Err(EmailError::SendError(format!(
            "Error sending email: {}",
            err
        ))),
        Ok(_) => Ok(()),
    }
}
