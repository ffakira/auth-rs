use dotenv::dotenv;
use mongodb::{error::Error as MongoError, options::ClientOptions, Client, Database};
use std::{env, io};

pub async fn mongo_client() -> Result<(Client, Database), MongoError> {
    dotenv().ok();
    let mongo_uri = env::var("MONGO_URI").map_err(|err| {
        io::Error::new(
            io::ErrorKind::Other,
            format!("Failed to get MONGO_URI: {}", err),
        )
    })?;
    let database = env::var("MONGO_DATABASE").map_err(|err| {
        io::Error::new(
            io::ErrorKind::Other,
            format!("Failed to get MONGO_DATABASE: {}", err),
        )
    })?;

    let client_options = ClientOptions::parse(&mongo_uri).await.map_err(|err| {
        io::Error::new(
            io::ErrorKind::Other,
            format!("Failed to parse client options: {}", err),
        )
    })?;

    let client = Client::with_options(client_options)?;
    let db = client.database(&database);

    Ok((client, db))
}
