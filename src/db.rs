use mongodb::{
    Client,
    options::ClientOptions,
    Database,
    error::Error,
};

pub async fn mongo_client() -> Result<(Client, Database), Error> {
    let client_options: ClientOptions = match ClientOptions::parse("mongodb://localhost:27017").await {
        Ok(options) => options,
        Err(e) => return Err(e),
    };
    
    let client: Client = match Client::with_options(client_options) {
        Ok(client) => client,
        Err(e) => return Err(e),
    };

    let db: Database = client.database("rust");

    Ok((client, db))
}
