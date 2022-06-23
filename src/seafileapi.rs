use log::debug;
// These require the `serde` dependency.
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Mutex;
use bytes::Bytes;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthResponse {
    pub token: String,
}

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

#[derive(Debug)]
pub struct SeafileAPI {
    client: reqwest::blocking::Client,
    authorization: Mutex<Option<String>>,
    libraries: Mutex<Option<Vec<Library>>>,
    server: String,
    username: String,
    password: String,
}

impl SeafileAPI {
    pub fn new(server: &str, username: &str, password: &str) -> Self {
        Self {
            client: reqwest::blocking::Client::new(),
            authorization: Mutex::new(None),
            libraries: Mutex::new(None),
            server: server.to_string(),
            username: username.to_string(),
            password: password.to_string(),
        }
    }

    fn login(&self) -> Result<String> {
        {
            let auth = self.authorization.lock().unwrap();
            if let Some(a) = &*auth {
                return Ok(a.to_string());
            };
        }
        let params = [("username", &self.username), ("password", &self.password)];
        let url = format!("{}/api2/auth-token/", self.server);
        let res = self.client.post(&url).form(&params).send()?;
        let body: AuthResponse = res.json()?;

        println!("Body:\n\n{:#?}", body);

        let mut authorization = String::from("Token ");
        authorization.push_str(&body.token);

        debug!("Authorization: {}", authorization);
        {
            let mut auth = self.authorization.lock().unwrap();
            *auth = Some(authorization.clone());
        }

        Ok(authorization)
    }

    pub fn get_libraries(&self) -> Result<Vec<Library>> {
        {
            let libraries = self.libraries.lock().unwrap();
            if let Some(l) = &*libraries {
                return Ok(l.to_vec());
            }
        }
        debug!("self: {:?}", &self);
        let authorization = self.login()?;
        debug!("self: {:?}", &self);
        let url = format!("{}/api2/repos/", self.server);
        let res = self
            .client
            .get(&url)
            .header("Authorization", &authorization)
            .send()?;
        debug!("response headers: {:?}", res.headers());
        let body: Vec<Library> = res.json()?;
        {
            let mut libraries = self.libraries.lock().unwrap();
            *libraries = Some(body.clone());
        }
        Ok(body)
    }

    pub fn get_library_content(
        &self,
        id: &str,
        path: &Path,
    ) -> Result<Vec<LibraryEntry>> {
        debug!("self: {:?}", &self);
        let authorization = self.login()?;
        debug!("self: {:?}", &self);
        let url = format!("{}/api2/repos/{}/dir/", self.server, id);

        debug!("url: {}, p: {:?}, {:?}", url, path, [("p", path)]);

        let res = self
            .client
            .get(&url)
            //.query(&[("t","f"),("p","/")])
            .query(&[("p", path)])
            .header("Authorization", &authorization)
            .send()?;

        let body: Vec<LibraryEntry> = res.json()?;
        Ok(body)
    }
    
    pub fn create_file(&self, id: &str, path: &Path) -> Result<String> {
        debug!("self: {:?}", &self);
        let authorization = self.login()?;
        debug!("self: {:?}", &self);
        let url = format!("{}/api2/repos/{}/file/", self.server, id);

        debug!("url: {}, p: {:?}, {:?}", url, path, [("p", path)]);

        let res = self
            .client
            .post(&url)
            .body("operation=create")
            .header(reqwest::header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            //.query(&[("t","f"),("p","/")])
            .query(&[("p", path)])
            .header("Authorization", &authorization)
            .send()?;

        let body: String = res.text()?;
        Ok(body)
	}
    
    pub fn create_new_directory(&self, id: &str, path: &Path) -> Result<String> {
        debug!("self: {:?}", &self);
        let authorization = self.login()?;
        debug!("self: {:?}", &self);
        let url = format!("{}/api2/repos/{}/dir/", self.server, id);

        debug!("url: {}, p: {:?}, {:?}", url, path, [("p", path)]);

        let res = self
            .client
            .post(&url)
            .body("operation=mkdir")
            .header(reqwest::header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            //.query(&[("t","f"),("p","/")])
            .query(&[("p", path)])
            .header("Authorization", &authorization)
            .send()?;

        let body: String = res.text()?;
        Ok(body)
	}
	
	pub fn delete_directory(&self, id: &str, path: &Path) -> Result<String> {
        debug!("self: {:?}", &self);
        let authorization = self.login()?;
        debug!("self: {:?}", &self);
        let url = format!("{}/api2/repos/{}/dir/", self.server, id);

        debug!("url: {}, p: {:?}, {:?}", url, path, [("p", path)]);

        let res = self
            .client
            .delete(&url)
            .query(&[("p", path)])
            .header("Authorization", &authorization)
            .send()?;

        let body: String = res.text()?;
        Ok(body)
	}
    
    pub fn get_download_link(&self, id: &str, path: &Path) -> Result<String> {
        debug!("self: {:?}", &self);
        let authorization = self.login()?;
        debug!("self: {:?}", &self);
        let url = format!("{}/api2/repos/{}/file/", self.server, id);

        debug!("url: {}, {:?}, {:?}", url, ("p", path), ("reuse", 1));

        let res = self
            .client
            .get(&url)
            //.query(&[("t","f"),("p","/")])
            .query(&[("p", path)])
            .query(&[("reuse", 1)])
            .header("Authorization", &authorization)
            .send()?;

        let body: String = res.json()?;
        Ok(body)
	}
	
	pub fn download(&self, uri: &str) -> Result<Bytes> {
		let res = self.client.get(uri).send()?;
		let body = res.bytes()?;
		Ok(body)
	}
}

/*
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // This will POST a body of `foo=bar&baz=quux`
    let params = [("foo", "bar"), ("baz", "quux")];
    let client = reqwest::Client::new();
    let res = client.get("http://192.168.0.32/api2/ping")
        .form(&params)
        .send()
        .await?;
    let body = res.text().await?;

    println!("Body:\n\n{}", body);

    let params = [("username", "havvoric@gmail.com"), ("password", "Alpha3wyrd")];
    let res = client.post("http://192.168.0.32/api2/auth-token/")
        .form(&params)
        .send()
        .await?;
    let body: AuthResponse = res.json().await?;

    println!("Body:\n\n{:#?}", body);

    let mut a = String::from("Token ");
    a.push_str(&body.token);

    println!("Authorization: {}", a);

    let res = client.get("http://192.168.0.32/api2/repos/")
        .header("Authorization", &a)
        .send()
        .await?;
    let mut body: Vec<Library> = res.json().await?;

    body = body.into_iter().filter(|entry| entry.name == String::from("Household")).collect();

    println!("Body:\n\n{:#?}", body);

    let id = match body.get(0) {
        Some(lib) => &lib.id,
        _ => return Ok(())
    };

    let u = format!("{}/{}/dir/", "http://192.168.0.32/api2/repos", id);

    println!("url: {}", u);

    let res = client.get(&u)
        .query(&[("t","f"),("p","/Wedding stuff")])
        .header("Authorization", &a)
        .send()
        .await?;

    let body: Vec<LibraryEntry> = res.json().await?;

    println!("Body:\n\n{:#?}", body);

    let fname = match body.get(0) {
        Some(f) => &f.name,
        _ => return Ok(())
    };

    let u = format!("{}/{}/file/", "http://192.168.0.32/api2/repos", id);

    let res = client.get(&u)
        .query(&[("p", format!("/Wedding stuff/{}", fname))])
        .header("Authorization", &a)
        .send()
        .await?;

    let body: String = res.text().await?;

    println!("Body:\n\n{:#?}", body);

    let res = client.get(&u)
        .query(&[("p", format!("/Wedding stuff/Wedding Prep/{}", "teeny colourful cake.jpg"))])
        .header("Authorization", &a)
        .send()
        .await?;

    let body: String = res.text().await?;

    println!("Body:\n\n{:#?}", body);

    Ok(())
}
*/
