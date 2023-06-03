use std::thread::sleep_ms;

use log::info;
use serde::{Deserialize, Serialize};
use upgraded_giggle::seafileapi::AuthResponse;

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct Library {
    permission: String,
    encrypted: bool,
    pub mtime: u64,
    owner: String,
    pub id: String,
    pub size: u64,
    pub name: String,
    #[serde(rename = "type")]
    library_type: String,
    #[serde(rename = "virtual", default)]
    is_virtual: bool,
    #[serde(default)]
    desc: String,
    root: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LibraryEntry {
    id: String,
    pub mtime: u64,
    #[serde(default)]
    pub size: u64,
    pub name: String,
    permission: String,
    #[serde(rename = "type")]
    pub entry_type: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    log4rs::init_file("log4rs.yml", Default::default()).unwrap();
    // This will POST a body of `foo=bar&baz=quux`
    let params = [("foo", "bar"), ("baz", "quux")];
    let client = reqwest::Client::new();
    let res = client
        .get("http://seafilepi/api2/ping")
        .form(&params)
        .send()
        .await?;
    let body = res.text().await?;

    println!("Body:\n\n{}", body);

    let params = [
        ("username", "havvoric@gmail.com"),
        ("password", "Alpha3wyrd"),
    ];
    let res = client
        .post("http://seafilepi/api2/auth-token/")
        .form(&params)
        .send()
        .await?;
    let body: AuthResponse = res.json().await?;

    println!("Body:\n\n{:#?}", body);

    let mut a = String::from("Token ");
    a.push_str(&body.token);

    println!("Authorization: {}", a);

    let res = client
        .get("http://seafilepi/api2/repos/")
        .header("Authorization", &a)
        .send()
        .await?;
    let mut body: Vec<Library> = res.json().await?;

    body = body
        .into_iter()
        .filter(|entry| entry.name == *"Household")
        .collect();

    println!("Body:\n\n{:#?}", body);

    let id = match body.get(0) {
        Some(lib) => &lib.id,
        _ => return Ok(()),
    };

    let u = format!("{}/{}/dir/", "http://seafilepi/api2/repos", id);

    println!("url: {}", u);

    let res = client
        .get(&u)
        .query(&[("t", "f"), ("p", "/Wedding stuff")])
        .header("Authorization", &a)
        .send()
        .await?;

    let body: Vec<LibraryEntry> = res.json().await?;

    println!("Body:\n\n{:#?}", body);

    let fname = match body.get(0) {
        Some(f) => &f.name,
        _ => return Ok(()),
    };

    let u = format!("{}/{}/file/", "http://seafilepi/api2/repos", id);

    let res = client
        .get(&u)
        .query(&[("p", format!("/Wedding stuff/{}", fname))])
        .header("Authorization", &a)
        .send()
        .await?;

    let body: String = res.text().await?;

    println!("Body:\n\n{:#?}", body);

    let res = client
        .get(&u)
        .query(&[(
            "p",
            format!("/Wedding stuff/Wedding Prep/{}", "teeny colourful cake.jpg"),
        )])
        .header("Authorization", &a)
        .send()
        .await?;

    let body: String = res.text().await?;

    println!("Body:\n\n{:#?}", body);

    loop {
        let res = client
            .get(&u)
            .query(&[(
                "p",
                format!("/Wedding stuff/Wedding Prep/{}", "teeny colourful cake.jpg"),
            )])
            .header("Authorization", &a)
            .send()
            .await?;

        let body: String = res.json().await?;

        info!(r#"Body: {:#?}"#, body);
        for (i, c) in body.chars().enumerate() {
            info!("{i} {c}");
        }
        sleep_ms(10000);
    }

    Ok(())
}
