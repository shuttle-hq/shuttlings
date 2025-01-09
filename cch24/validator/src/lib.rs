pub mod args;

use chrono::{DateTime, TimeDelta, Utc};
use html_compare_rs::{HtmlCompareOptions, HtmlComparer};
use jsonwebtoken::decode_header;
use reqwest::{
    header::{self, HeaderValue},
    multipart::{Form, Part},
    redirect::Policy,
    Client, StatusCode,
};
use serde_json::json;
use shuttlings::{SubmissionState, SubmissionUpdate};
use tokio::{
    sync::mpsc::Sender,
    time::{sleep, Duration},
};
use tracing::info;
use uuid::Uuid;

pub const SUPPORTED_CHALLENGES: &[&str] = &["-1", "2", "5", "9", "12", "16", "19", "23"];
pub const SUBMISSION_TIMEOUT: u64 = 60;

pub async fn run(url: String, id: Uuid, number: &str, tx: Sender<SubmissionUpdate>) {
    info!(%id, %url, %number, "Starting submission");

    tx.send(SubmissionState::Running.into()).await.unwrap();
    tx.send(SubmissionUpdate::Save).await.unwrap();

    tokio::select! {
        _ = validate(url.as_str(), number, tx.clone()) => (),
        _ = sleep(Duration::from_secs(SUBMISSION_TIMEOUT)) => {
            // if the validation task timed out
            info!(%id, %url, %number, "Submission timed out");
            tx.send("Timed out".to_owned().into()).await.unwrap();
            tx.send(SubmissionState::Done.into()).await.unwrap();
            tx.send(SubmissionUpdate::Save).await.unwrap();
        },
    };
    info!(%id, %url, %number, "Completed submission");
}

/// Task number and Test number in the current challenge
type TaskTest = (i32, i32);
/// If failure, return tuple with task number and test number that failed
type ValidateResult = std::result::Result<(), TaskTest>;

pub async fn validate(url: &str, number: &str, tx: Sender<SubmissionUpdate>) {
    let txc = tx.clone();
    if let Err((task, test)) = match number {
        "-1" => validate_minus1(url, txc).await,
        "2" => validate_2(url, txc).await,
        "5" => validate_5(url, txc).await,
        "9" => validate_9(url, txc).await,
        "12" => validate_12(url, txc).await,
        "16" => validate_16(url, txc).await,
        "19" => validate_19(url, txc).await,
        "23" => validate_23(url, txc).await,
        _ => {
            tx.send(
                format!("Validating Challenge {number} is not supported yet! Check for updates.")
                    .into(),
            )
            .await
            .unwrap();
            return;
        }
    } {
        info!(%url, %number, %task, %test, "Submission failed");
        tx.send(format!("Task {task}: test #{test} failed 🟥").into())
            .await
            .unwrap();
    }
    tx.send(SubmissionState::Done.into()).await.unwrap();
    tx.send(SubmissionUpdate::Save).await.unwrap();
}

fn new_client_base() -> reqwest::ClientBuilder {
    reqwest::ClientBuilder::new()
        .http1_only()
        .connect_timeout(Duration::from_secs(3))
        .redirect(Policy::limited(3))
        .referer(false)
        .timeout(Duration::from_secs(60))
}
fn new_client() -> reqwest::Client {
    new_client_base().build().unwrap()
}
fn new_client_with_cookies() -> reqwest::Client {
    new_client_base().cookie_store(true).build().unwrap()
}

macro_rules! assert_status {
    ($res:expr, $test:expr, $expected_status:expr) => {
        if $res.status() != $expected_status {
            return Err($test);
        }
    };
}

macro_rules! assert_text {
    ($res:expr, $test:expr, $expected_text:expr) => {
        if $res.text().await.map_err(|_| $test)? != $expected_text {
            return Err($test);
        }
    };
}

macro_rules! assert_json {
    ($res:expr, $test:expr, $expected_json:expr) => {
        if $res.json::<serde_json::Value>().await.map_err(|_| $test)? != $expected_json {
            return Err($test);
        }
    };
}

macro_rules! assert_text_starts_with {
    ($res:expr, $test:expr, $expected_text:expr) => {
        if !$res
            .text()
            .await
            .map_err(|_| $test)?
            .starts_with($expected_text)
        {
            return Err($test);
        }
    };
}

macro_rules! assert_ {
    ($test:expr, $expected_true:expr) => {
        if !$expected_true {
            return Err($test);
        }
    };
}

macro_rules! assert_eq_ {
    ($test:expr, $left:expr, $right:expr) => {
        if $left != $right {
            return Err($test);
        }
    };
}

macro_rules! assert_neq_ {
    ($test:expr, $left:expr, $right:expr) => {
        if $left == $right {
            return Err($test);
        }
    };
}

async fn validate_minus1(base_url: &str, tx: Sender<SubmissionUpdate>) -> ValidateResult {
    let client = new_client();
    let mut test: TaskTest;
    // TASK 1: respond 200 with Hello, bird!
    test = (1, 1);
    let url = &format!("{}/", base_url);
    let res = client.get(url).send().await.map_err(|_| test)?;
    assert_status!(res, test, StatusCode::OK);
    assert_text!(res, test, "Hello, bird!");
    // TASK 1 DONE
    tx.send((true, 0).into()).await.unwrap();
    tx.send(SubmissionUpdate::Save).await.unwrap();

    // TASK 2: respond 302
    test = (2, 1);
    let url = &format!("{}/-1/seek", base_url);
    let client_no_redir = reqwest::ClientBuilder::new()
        .http1_only()
        .connect_timeout(Duration::from_secs(3))
        .redirect(Policy::none())
        .referer(false)
        .timeout(Duration::from_secs(60))
        .build()
        .unwrap();
    let res = client_no_redir.get(url).send().await.map_err(|_| test)?;
    assert_status!(res, test, StatusCode::FOUND);
    if res.headers().get(header::LOCATION)
        != Some(&HeaderValue::from_static(
            "https://www.youtube.com/watch?v=9Gc4QTqslN4",
        ))
    {
        return Err(test);
    }
    // TASK 2 DONE
    tx.send((false, 0).into()).await.unwrap();

    Ok(())
}

async fn validate_2(base_url: &str, tx: Sender<SubmissionUpdate>) -> ValidateResult {
    let client = new_client();
    let mut test: TaskTest;
    // TASK 1: Ipv4 dest
    test = (1, 1);
    let url = &format!("{}/2/dest?from=10.0.0.0&key=1.2.3.255", base_url);
    let res = client.get(url).send().await.map_err(|_| test)?;
    assert_text!(res, test, "11.2.3.255");
    test = (1, 2);
    let url = &format!("{}/2/dest?from=128.128.33.0&key=255.0.255.33", base_url);
    let res = client.get(url).send().await.map_err(|_| test)?;
    assert_text!(res, test, "127.128.32.33");
    test = (1, 3);
    let url = &format!("{}/2/dest?from=192.168.0.1&key=72.96.8.7", base_url);
    let res = client.get(url).send().await.map_err(|_| test)?;
    assert_text!(res, test, "8.8.8.8");
    // TASK 1 DONE
    tx.send((false, 0).into()).await.unwrap();
    tx.send(SubmissionUpdate::Save).await.unwrap();

    // TASK 2: Ipv4 key
    test = (2, 1);
    let url = &format!("{}/2/key?from=10.0.0.0&to=11.2.3.255", base_url);
    let res = client.get(url).send().await.map_err(|_| test)?;
    assert_text!(res, test, "1.2.3.255");
    test = (2, 2);
    let url = &format!("{}/2/key?from=128.128.33.0&to=127.128.32.33", base_url);
    let res = client.get(url).send().await.map_err(|_| test)?;
    assert_text!(res, test, "255.0.255.33");
    test = (2, 3);
    let url = &format!("{}/2/key?from=192.168.0.1&to=8.8.8.8", base_url);
    let res = client.get(url).send().await.map_err(|_| test)?;
    assert_text!(res, test, "72.96.8.7");
    // TASK 2 DONE
    tx.send((true, 0).into()).await.unwrap();
    tx.send(SubmissionUpdate::Save).await.unwrap();

    // TASK 3: Ipv6
    test = (3, 1);
    let url = &format!("{}/2/v6/dest?from=fe80::1&key=5:6:7::3333", base_url);
    let res = client.get(url).send().await.map_err(|_| test)?;
    assert_text!(res, test, "fe85:6:7::3332");
    test = (3, 2);
    let url = &format!(
        "{}/2/v6/dest?from=aaaa:0:0:0::aaaa&key=ffff:ffff:c:0:0:c:1234:ffff",
        base_url
    );
    let res = client.get(url).send().await.map_err(|_| test)?;
    assert_text!(res, test, "5555:ffff:c::c:1234:5555");
    test = (3, 3);
    let url = &format!(
        "{}/2/v6/dest?from=feed:beef:deaf:bad:cafe::&key=::dab:bed:ace:dad",
        base_url
    );
    let res = client.get(url).send().await.map_err(|_| test)?;
    assert_text!(res, test, "feed:beef:deaf:bad:c755:bed:ace:dad");
    test = (3, 4);
    let url = &format!("{}/2/v6/key?from=fe80::1&to=fe85:6:7::3332", base_url);
    let res = client.get(url).send().await.map_err(|_| test)?;
    assert_text!(res, test, "5:6:7::3333");
    test = (3, 5);
    let url = &format!(
        "{}/2/v6/key?from=aaaa::aaaa&to=5555:ffff:c:0:0:c:1234:5555",
        base_url
    );
    let res = client.get(url).send().await.map_err(|_| test)?;
    assert_text!(res, test, "ffff:ffff:c::c:1234:ffff");
    test = (3, 6);
    let url = &format!(
        "{}/2/v6/key?from=feed:beef:deaf:bad:cafe::&to=feed:beef:deaf:bad:c755:bed:ace:dad",
        base_url
    );
    let res = client.get(url).send().await.map_err(|_| test)?;
    assert_text!(res, test, "::dab:bed:ace:dad");
    // TASK 3 DONE
    tx.send((false, 50).into()).await.unwrap();
    tx.send(SubmissionUpdate::Save).await.unwrap();

    Ok(())
}

async fn validate_5(base_url: &str, tx: Sender<SubmissionUpdate>) -> ValidateResult {
    let client = new_client();
    let mut test: TaskTest;
    let url = &format!("{}/5/manifest", base_url);
    const CT: &str = "Content-Type";
    const TOML: &str = "application/toml";
    const YAML: &str = "application/yaml";
    const JSON: &str = "application/json";
    // TASK 1: order list
    test = (1, 1);
    let res = client
        .post(url)
        .header(CT, TOML)
        .body(
            r#"
[package]
name = "not-a-gift-order"
authors = ["Not Santa"]
keywords = ["Christmas 2024"]

[[package.metadata.orders]]
item = "Toy car"
quantity = 2

[[package.metadata.orders]]
item = "Lego brick"
quantity = 230
"#,
        )
        .send()
        .await
        .map_err(|_| test)?;
    assert_status!(res, test, StatusCode::OK);
    assert_text!(res, test, "Toy car: 2\nLego brick: 230");
    test = (1, 2);
    let res = client
        .post(url)
        .header(CT, TOML)
        .body(
            r#"
[package]
name = "coal-in-a-bowl"
authors = ["H4CK3R_13E7"]
keywords = ["Christmas 2024"]

[[package.metadata.orders]]
item = "Coal"
quantity = "Hahaha get rekt"
"#,
        )
        .send()
        .await
        .map_err(|_| test)?;
    assert_status!(res, test, StatusCode::NO_CONTENT);
    test = (1, 3);
    let res = client
        .post(url)
        .header(CT, TOML)
        .body(
            r#"
[package]
name = "coal-in-a-bowl"
authors = ["H4CK3R_13E7"]
keywords = ["Christmas 2024"]

package.metadata.orders = []
"#,
        )
        .send()
        .await
        .map_err(|_| test)?;
    assert_status!(res, test, StatusCode::NO_CONTENT);
    test = (1, 4);
    let res = client
        .post(url)
        .header(CT, TOML)
        .body(
            r#"
[package]
name = "not-a-gift-order"
authors = ["Not Santa"]
keywords = ["Christmas 2024"]

[[package.metadata.orders]]
item = "Toy car"
quantity = 2

[[package.metadata.orders]]
item = "Lego brick"
quantity = 1.5

[[package.metadata.orders]]
item = "Doll"
quantity = 2

[[package.metadata.orders]]
quantity = 5
item = "Cookie:::\n"

[[package.metadata.orders]]
item = "Thing"
count = 3
"#,
        )
        .send()
        .await
        .map_err(|_| test)?;
    assert_status!(res, test, StatusCode::OK);
    assert_text!(res, test, "Toy car: 2\nDoll: 2\nCookie:::\n: 5");
    // TASK 1 DONE
    tx.send((false, 0).into()).await.unwrap();
    tx.send(SubmissionUpdate::Save).await.unwrap();

    // TASK 2: manifest parsing
    test = (2, 1);
    let res = client
        .post(url)
        .header(CT, TOML)
        .body(
            r#"
[package]
name = false
authors = ["Not Santa"]
keywords = ["Christmas 2024"]
"#,
        )
        .send()
        .await
        .map_err(|_| test)?;
    assert_status!(res, test, StatusCode::BAD_REQUEST);
    assert_text!(res, test, "Invalid manifest");
    test = (2, 2);
    let res = client
        .post(url)
        .header(CT, TOML)
        .body(
            r#"
[package]
name = "not-a-gift-order"
authors = ["Not Santa"]
keywords = ["Christmas 2024"]

[profile.release]
incremental = "stonks"
"#,
        )
        .send()
        .await
        .map_err(|_| test)?;
    assert_status!(res, test, StatusCode::BAD_REQUEST);
    assert_text!(res, test, "Invalid manifest");
    test = (2, 3);
    let res = client
        .post(url)
        .header(CT, TOML)
        .body(
            r#"
[package]
name = "big-chungus"
version = "2.0.24"
edition = "2024"
resolver = "2"
readme.workspace = true
keywords = ["Christmas 2024"]

[dependencies]
shuttle-runtime = "1.0.0+when"

[target.shuttlings.dependencies]
cch24-validator = "5+more"

[profile.release]
incremental = false

[package.metadata.stuff]
thing = ["yes", "no"]
"#,
        )
        .send()
        .await
        .map_err(|_| test)?;
    assert_status!(res, test, StatusCode::NO_CONTENT);
    test = (2, 4);
    let res = client
        .post(url)
        .header(CT, TOML)
        .body(
            r#"
[package]
name = "chig-bungus"
edition = "2023"

[workspace.dependencies]
shuttle-bring-your-own-cloud = "0.0.0"
"#,
        )
        .send()
        .await
        .map_err(|_| test)?;
    assert_status!(res, test, StatusCode::BAD_REQUEST);
    assert_text!(res, test, "Invalid manifest");
    test = (2, 5);
    let res = client
        .post(url)
        .header(CT, TOML)
        .body(
            r#"
[package]
name = "chig-bungus"

[workspace]
resolver = "135"

[workspace.dependencies]
shuttle-bring-your-own-cloud = "0.0.0"
"#,
        )
        .send()
        .await
        .map_err(|_| test)?;
    assert_status!(res, test, StatusCode::BAD_REQUEST);
    assert_text!(res, test, "Invalid manifest");
    // TASK 2 DONE
    tx.send((false, 0).into()).await.unwrap();
    tx.send(SubmissionUpdate::Save).await.unwrap();

    // TASK 3: keyword
    test = (3, 1);
    let res = client
        .post(url)
        .header(CT, TOML)
        .body(
            r#"
[package]
name = "grass"
authors = ["A vegan cow"]
keywords = ["Moooooo"]
"#,
        )
        .send()
        .await
        .map_err(|_| test)?;
    assert_status!(res, test, StatusCode::BAD_REQUEST);
    assert_text!(res, test, "Magic keyword not provided");
    test = (3, 2);
    let res = client
        .post(url)
        .header(CT, TOML)
        .body(
            r#"
[package]
name = "chig-bungus"

[workspace]
resolver = "2"

[workspace.dependencies]
shuttle-bring-your-own-cloud = "0.0.0"
"#,
        )
        .send()
        .await
        .map_err(|_| test)?;
    assert_status!(res, test, StatusCode::BAD_REQUEST);
    assert_text!(res, test, "Magic keyword not provided");
    test = (3, 3);
    let res = client
        .post(url)
        .header(CT, TOML)
        .body(
            r#"
[package]
name = "slurp"
authors = ["A crazy cow"]
keywords = ["MooOooooooOOOOoo00oo=oOooooo", "Mew", "Moh", "Christmas 2024"]
metadata.orders = [{ item = "Milk 🥛", quantity = 1 }]
"#,
        )
        .send()
        .await
        .map_err(|_| test)?;
    assert_status!(res, test, StatusCode::OK);
    assert_text!(res, test, "Milk 🥛: 1");
    test = (3, 4);
    let res = client
        .post(url)
        .header(CT, TOML)
        .body(
            r#"
[package]
name = "snow"
authors = ["The Cow of Christmas"]
keywords = ["Moooooo Merry Christmas 2024"]
"#,
        )
        .send()
        .await
        .map_err(|_| test)?;
    assert_status!(res, test, StatusCode::BAD_REQUEST);
    assert_text!(res, test, "Magic keyword not provided");
    // TASK 3 DONE
    tx.send((true, 0).into()).await.unwrap();
    tx.send(SubmissionUpdate::Save).await.unwrap();

    // TASK 4: Yaml, Json
    test = (4, 1);
    let res = client
        .post(url)
        .header(CT, "text/html")
        .body("<h1>Hello, bird!</h1>")
        .send()
        .await
        .map_err(|_| test)?;
    assert_status!(res, test, StatusCode::UNSUPPORTED_MEDIA_TYPE);
    test = (4, 2);
    let res = client
        .post(url)
        .header(CT, YAML)
        .body(
            r#"
package:
  name: big-chungus-sleigh
  version: "2.0.24"
  metadata:
    orders:
      - item: "Toy train"
        quantity: 5
      - item: "Toy car"
        quantity: 3
  rust-version: "1.69"
  keywords:
    - "Christmas 2024"
"#,
        )
        .send()
        .await
        .map_err(|_| test)?;
    assert_status!(res, test, StatusCode::OK);
    assert_text!(res, test, "Toy train: 5\nToy car: 3");
    test = (4, 3);
    let res = client
        .post(url)
        .header(CT, YAML)
        .body(
            r#"
package:
  name: big-chungus-sleigh
  metadata:
    orders:
      - item: "Toy train"
        quantity: 5
      - item: "Coal"
      - item: "Horse"
        quantity: 2
  keywords:
    - "Christmas 2024"
"#,
        )
        .send()
        .await
        .map_err(|_| test)?;
    assert_status!(res, test, StatusCode::OK);
    assert_text!(res, test, "Toy train: 5\nHorse: 2");
    test = (4, 4);
    let res = client
        .post(url)
        .header(CT, YAML)
        .body(
            r#"
package:
  name: big-chungus-sleigh
  metadata:
    orders:
      - item: "Toy train"
        quantity: 5
  rust-version: true
  keywords:
    - "Christmas 2024"
"#,
        )
        .send()
        .await
        .map_err(|_| test)?;
    assert_status!(res, test, StatusCode::BAD_REQUEST);
    assert_text!(res, test, "Invalid manifest");
    test = (4, 5);
    let res = client
        .post(url)
        .header(CT, JSON)
        .body(
            r#"
{
  "package": {
    "name": "big-chungus-sleigh",
    "version": "2.0.24",
    "metadata": {
      "orders": [
        {
          "item": "Toy train",
          "quantity": 5
        },
        {
          "item": "Toy car",
          "quantity": 3
        }
      ]
    },
    "rust-version": "1.69",
    "keywords": [
      "Christmas 2024"
    ]
  }
}
"#,
        )
        .send()
        .await
        .map_err(|_| test)?;
    assert_status!(res, test, StatusCode::OK);
    assert_text!(res, test, "Toy train: 5\nToy car: 3");
    test = (4, 6);
    let res = client
        .post(url)
        .header(CT, JSON)
        .body(
            r#"
{
  "package": {
    "name": "big-chungus-sleigh",
    "metadata": {
      "orders": [
        {
          "item": "Toy train",
          "quantity": 5
        },
        {
          "item": "Coal"
        },
        {
          "item": "Horse",
          "quantity": 2
        }
      ]
    },
    "keywords": [
      "Christmas 2024"
    ]
  }
}
"#,
        )
        .send()
        .await
        .map_err(|_| test)?;
    assert_status!(res, test, StatusCode::OK);
    assert_text!(res, test, "Toy train: 5\nHorse: 2");
    test = (4, 7);
    let res = client
        .post(url)
        .header(CT, JSON)
        .body(
            r#"
{
  "package": {
    "name": "big-chungus-sleigh",
    "metadata": {
      "orders": [
        {
          "item": "Toy train",
          "quantity": 5
        }
      ]
    }
  }
}
"#,
        )
        .send()
        .await
        .map_err(|_| test)?;
    assert_status!(res, test, StatusCode::BAD_REQUEST);
    assert_text!(res, test, "Magic keyword not provided");
    // TASK 4 DONE
    tx.send((false, 70).into()).await.unwrap();
    tx.send(SubmissionUpdate::Save).await.unwrap();

    Ok(())
}

async fn validate_9(base_url: &str, tx: Sender<SubmissionUpdate>) -> ValidateResult {
    let client = new_client();
    let mut test: TaskTest;
    // TASK 1: leaky bucket
    test = (1, 1);
    let url = &format!("{}/9/milk", base_url);
    let start = Utc::now();
    let res = client.post(url).send().await.map_err(|_| test)?;
    assert_status!(res, test, StatusCode::OK);
    assert_text!(res, test, "Milk withdrawn\n");
    let res = client.post(url).send().await.map_err(|_| test)?;
    assert_status!(res, test, StatusCode::OK);
    assert_text!(res, test, "Milk withdrawn\n");
    let res = client.post(url).send().await.map_err(|_| test)?;
    assert_status!(res, test, StatusCode::OK);
    assert_text!(res, test, "Milk withdrawn\n");
    let res = client.post(url).send().await.map_err(|_| test)?;
    assert_status!(res, test, StatusCode::OK);
    assert_text!(res, test, "Milk withdrawn\n");
    let res = client.post(url).send().await.map_err(|_| test)?;
    assert_status!(res, test, StatusCode::OK);
    assert_text!(res, test, "Milk withdrawn\n");
    let end = Utc::now();
    if end - start > TimeDelta::milliseconds(500) {
        tx.send(SubmissionUpdate::LogLine(
            "Info: High network latency detected. This test is timing-sensitive and might therefore fail.".to_owned()
        ))
        .await
        .unwrap();
    }
    let res = client.post(url).send().await.map_err(|_| test)?;
    assert_status!(res, test, StatusCode::TOO_MANY_REQUESTS);
    assert_text!(res, test, "No milk available\n");
    sleep(Duration::from_secs(1)).await;
    let res = client.post(url).send().await.map_err(|_| test)?;
    assert_status!(res, test, StatusCode::OK);
    assert_text!(res, test, "Milk withdrawn\n");
    let res = client.post(url).send().await.map_err(|_| test)?;
    assert_status!(res, test, StatusCode::TOO_MANY_REQUESTS);
    assert_text!(res, test, "No milk available\n");
    sleep(Duration::from_secs(2)).await;
    let res = client.post(url).send().await.map_err(|_| test)?;
    assert_status!(res, test, StatusCode::OK);
    assert_text!(res, test, "Milk withdrawn\n");
    let res = client.post(url).send().await.map_err(|_| test)?;
    assert_status!(res, test, StatusCode::OK);
    assert_text!(res, test, "Milk withdrawn\n");
    let res = client.post(url).send().await.map_err(|_| test)?;
    assert_status!(res, test, StatusCode::TOO_MANY_REQUESTS);
    assert_text!(res, test, "No milk available\n");
    let res = client.post(url).send().await.map_err(|_| test)?;
    assert_status!(res, test, StatusCode::TOO_MANY_REQUESTS);
    assert_text!(res, test, "No milk available\n");
    // TASK 1 DONE
    tx.send((false, 0).into()).await.unwrap();
    tx.send(SubmissionUpdate::Save).await.unwrap();

    // reset bucket
    sleep(Duration::from_secs(5)).await;

    // TASK 2: gallons
    test = (2, 1);
    let res = client
        .post(url)
        .json(&json!({"liters": 2}))
        .send()
        .await
        .map_err(|_| test)?;
    assert_status!(res, test, StatusCode::OK);
    let j = res.json::<serde_json::Value>().await.map_err(|_| test)?;
    assert_!(
        test,
        j.as_object().is_some_and(|o| o.len() == 1
            && o.get("gallons").is_some_and(|g| g
                .as_f64()
                .is_some_and(|f| (f / 0.5283441 - 1.0).abs() < 0.0001)))
    );
    test = (2, 2);
    let res = client
        .post(url)
        .json(&json!({"gallons": -2.000000000000001}))
        .send()
        .await
        .map_err(|_| test)?;
    assert_status!(res, test, StatusCode::OK);
    let j = res.json::<serde_json::Value>().await.map_err(|_| test)?;
    assert_!(
        test,
        j.as_object().is_some_and(|o| o.len() == 1
            && o.get("liters").is_some_and(|g| g
                .as_f64()
                .is_some_and(|f| (f / -7.5708237 - 1.0).abs() < 0.0001)))
    );
    test = (2, 3);
    let res = client.post(url).send().await.map_err(|_| test)?;
    assert_status!(res, test, StatusCode::OK);
    assert_text!(res, test, "Milk withdrawn\n");
    test = (2, 4);
    let res = client
        .post(url)
        .json(&json!({}))
        .send()
        .await
        .map_err(|_| test)?;
    assert_status!(res, test, StatusCode::BAD_REQUEST);
    test = (2, 5);
    let res = client
        .post(url)
        .json(&json!({"liters": 0, "gallons": 1337}))
        .send()
        .await
        .map_err(|_| test)?;
    assert_status!(res, test, StatusCode::BAD_REQUEST);
    test = (2, 6);
    let res = client.post(url).send().await.map_err(|_| test)?;
    assert_status!(res, test, StatusCode::TOO_MANY_REQUESTS);
    assert_text!(res, test, "No milk available\n");
    test = (2, 7);
    sleep(Duration::from_secs(1)).await;
    let res = client
        .post(url)
        .header("Content-Type", "application/json")
        .body("")
        .send()
        .await
        .map_err(|_| test)?;
    assert_status!(res, test, StatusCode::BAD_REQUEST);
    test = (2, 8);
    sleep(Duration::from_secs(1)).await;
    let res = client
        .post(url)
        .header("Content-Type", "application/json")
        .body("")
        .send()
        .await
        .map_err(|_| test)?;
    assert_status!(res, test, StatusCode::BAD_REQUEST);
    test = (2, 9);
    sleep(Duration::from_secs(1)).await;
    let res = client
        .post(url)
        .header("Content-Type", "application/json")
        .body("{'liters':0}")
        .send()
        .await
        .map_err(|_| test)?;
    assert_status!(res, test, StatusCode::BAD_REQUEST);
    test = (2, 10);
    sleep(Duration::from_secs(1)).await;
    let res = client
        .post(url)
        // (incoming f32 is truncated)
        .json(&json!({"liters": 123123123123.0}))
        .send()
        .await
        .map_err(|_| test)?;
    assert_status!(res, test, StatusCode::OK);
    let j = res.json::<serde_json::Value>().await.map_err(|_| test)?;
    assert_!(
        test,
        j.as_object().is_some_and(|o| o.len() == 1
            && o.get("gallons").is_some_and(|g| g
                .as_f64()
                .is_some_and(|f| (f / 32525687000.0 - 1.0).abs() < 0.0001)))
    );
    test = (2, 11);
    sleep(Duration::from_secs(1)).await;
    let res = client
        .post(url)
        .header("Content-Type", "text/html")
        .body(r#"{"liters":0}"#)
        .send()
        .await
        .map_err(|_| test)?;
    assert_status!(res, test, StatusCode::OK);
    assert_text!(res, test, "Milk withdrawn\n");
    // TASK 2 DONE
    tx.send((false, 0).into()).await.unwrap();
    tx.send(SubmissionUpdate::Save).await.unwrap();

    // reset bucket
    sleep(Duration::from_secs(5)).await;

    // TASK 3: litres/pints
    test = (3, 1);
    let res = client
        .post(url)
        .json(&json!({"litres": 7.4}))
        .send()
        .await
        .map_err(|_| test)?;
    assert_status!(res, test, StatusCode::OK);
    let j = res.json::<serde_json::Value>().await.map_err(|_| test)?;
    assert_!(
        test,
        j.as_object().is_some_and(|o| o.len() == 1
            && o.get("pints").is_some_and(|g| g
                .as_f64()
                .is_some_and(|f| (f / 13.02218 - 1.0).abs() < 0.0001)))
    );
    test = (3, 2);
    let res = client
        .post(url)
        .json(&json!({"pints": 32630.25}))
        .send()
        .await
        .map_err(|_| test)?;
    assert_status!(res, test, StatusCode::OK);
    let j = res.json::<serde_json::Value>().await.map_err(|_| test)?;
    assert_!(
        test,
        j.as_object().is_some_and(|o| o.len() == 1
            && o.get("litres").is_some_and(|g| g
                .as_f64()
                .is_some_and(|f| (f / 18542.508 - 1.0).abs() < 0.0001)))
    );
    test = (3, 3);
    let res = client
        .post(url)
        .json(&json!({"litres": -0.0}))
        .send()
        .await
        .map_err(|_| test)?;
    assert_status!(res, test, StatusCode::OK);
    let j = res.json::<serde_json::Value>().await.map_err(|_| test)?;
    assert_!(
        test,
        j.as_object().is_some_and(|o| o.len() == 1
            && o.get("pints")
                .is_some_and(|g| g.as_f64().is_some_and(|f| f == 0.0)))
    );
    test = (3, 4);
    let res = client
        .post(url)
        .json(&json!({"litres": 7.4, "liters": 7.4}))
        .send()
        .await
        .map_err(|_| test)?;
    assert_status!(res, test, StatusCode::BAD_REQUEST);
    test = (3, 5);
    let res = client
        .post(url)
        .json(r#"{"litres": 7.4, "litres": 7.6}"#)
        .send()
        .await
        .map_err(|_| test)?;
    assert_status!(res, test, StatusCode::BAD_REQUEST);
    test = (3, 6);
    sleep(Duration::from_secs(1)).await;
    let res = client
        .post(url)
        .json(&json!({"gallons": 2, "pints": 0}))
        .send()
        .await
        .map_err(|_| test)?;
    assert_status!(res, test, StatusCode::BAD_REQUEST);
    test = (3, 7);
    let res = client.post(url).send().await.map_err(|_| test)?;
    assert_status!(res, test, StatusCode::TOO_MANY_REQUESTS);
    assert_text!(res, test, "No milk available\n");
    // TASK 3 DONE
    tx.send((true, 0).into()).await.unwrap();
    tx.send(SubmissionUpdate::Save).await.unwrap();

    // TASK 4: refill
    test = (4, 1);
    let refill_url = &format!("{}/9/refill", base_url);
    let res = client.post(refill_url).send().await.map_err(|_| test)?;
    assert_status!(res, test, StatusCode::OK);
    test = (4, 2);
    let res = client.post(url).send().await.map_err(|_| test)?;
    assert_status!(res, test, StatusCode::OK);
    assert_text!(res, test, "Milk withdrawn\n");
    let res = client.post(url).send().await.map_err(|_| test)?;
    assert_status!(res, test, StatusCode::OK);
    assert_text!(res, test, "Milk withdrawn\n");
    let res = client.post(url).send().await.map_err(|_| test)?;
    assert_status!(res, test, StatusCode::OK);
    assert_text!(res, test, "Milk withdrawn\n");
    let res = client.post(url).send().await.map_err(|_| test)?;
    assert_status!(res, test, StatusCode::OK);
    assert_text!(res, test, "Milk withdrawn\n");
    let res = client.post(url).send().await.map_err(|_| test)?;
    assert_status!(res, test, StatusCode::OK);
    assert_text!(res, test, "Milk withdrawn\n");
    let res = client.post(url).send().await.map_err(|_| test)?;
    assert_status!(res, test, StatusCode::TOO_MANY_REQUESTS);
    assert_text!(res, test, "No milk available\n");
    let res = client.post(refill_url).send().await.map_err(|_| test)?;
    assert_status!(res, test, StatusCode::OK);
    let res = client.post(url).send().await.map_err(|_| test)?;
    assert_status!(res, test, StatusCode::OK);
    assert_text!(res, test, "Milk withdrawn\n");
    let res = client.post(url).send().await.map_err(|_| test)?;
    assert_status!(res, test, StatusCode::OK);
    assert_text!(res, test, "Milk withdrawn\n");
    let res = client.post(url).send().await.map_err(|_| test)?;
    assert_status!(res, test, StatusCode::OK);
    assert_text!(res, test, "Milk withdrawn\n");
    let res = client.post(url).send().await.map_err(|_| test)?;
    assert_status!(res, test, StatusCode::OK);
    assert_text!(res, test, "Milk withdrawn\n");
    let res = client.post(url).send().await.map_err(|_| test)?;
    assert_status!(res, test, StatusCode::OK);
    assert_text!(res, test, "Milk withdrawn\n");
    let res = client.post(url).send().await.map_err(|_| test)?;
    assert_status!(res, test, StatusCode::TOO_MANY_REQUESTS);
    assert_text!(res, test, "No milk available\n");
    let res = client.post(url).send().await.map_err(|_| test)?;
    assert_status!(res, test, StatusCode::TOO_MANY_REQUESTS);
    assert_text!(res, test, "No milk available\n");
    // TASK 4 DONE
    tx.send((false, 75).into()).await.unwrap();
    tx.send(SubmissionUpdate::Save).await.unwrap();

    Ok(())
}

async fn validate_12(base_url: &str, tx: Sender<SubmissionUpdate>) -> ValidateResult {
    let client = new_client();
    let mut test: TaskTest;
    // TASK 1: board and reset
    test = (1, 1);
    let reset_url = &format!("{}/12/reset", base_url);
    let res = client.post(reset_url).send().await.map_err(|_| test)?;
    assert_status!(res, test, StatusCode::OK);
    assert_text!(
        res,
        test,
        "\
⬜⬛⬛⬛⬛⬜
⬜⬛⬛⬛⬛⬜
⬜⬛⬛⬛⬛⬜
⬜⬛⬛⬛⬛⬜
⬜⬜⬜⬜⬜⬜
"
    );
    test = (1, 2);
    let board_url = &format!("{}/12/board", base_url);
    let res = client.get(board_url).send().await.map_err(|_| test)?;
    assert_status!(res, test, StatusCode::OK);
    assert_text!(
        res,
        test,
        "\
⬜⬛⬛⬛⬛⬜
⬜⬛⬛⬛⬛⬜
⬜⬛⬛⬛⬛⬜
⬜⬛⬛⬛⬛⬜
⬜⬜⬜⬜⬜⬜
"
    );
    // TASK 1 DONE
    tx.send((false, 0).into()).await.unwrap();
    tx.send(SubmissionUpdate::Save).await.unwrap();

    // TASK 2: gameplay
    test = (2, 1);
    let res = client.post(reset_url).send().await.map_err(|_| test)?;
    assert_status!(res, test, StatusCode::OK);
    assert_text!(
        res,
        test,
        "\
⬜⬛⬛⬛⬛⬜
⬜⬛⬛⬛⬛⬜
⬜⬛⬛⬛⬛⬜
⬜⬛⬛⬛⬛⬜
⬜⬜⬜⬜⬜⬜
"
    );
    async fn place(
        client: &Client,
        base_url: &str,
        test: TaskTest,
        team: &str,
        col: i32,
    ) -> Result<reqwest::Response, TaskTest> {
        client
            .post(format!("{}/12/place/{}/{}", base_url, team, col))
            .send()
            .await
            .map_err(|_| test)
    }
    let res = place(&client, base_url, test, "cookie", 1).await?;
    assert_status!(res, test, StatusCode::OK);
    assert_text!(
        res,
        test,
        "\
⬜⬛⬛⬛⬛⬜
⬜⬛⬛⬛⬛⬜
⬜⬛⬛⬛⬛⬜
⬜🍪⬛⬛⬛⬜
⬜⬜⬜⬜⬜⬜
"
    );
    let res = place(&client, base_url, test, "cookie", 1).await?;
    assert_status!(res, test, StatusCode::OK);
    assert_text!(
        res,
        test,
        "\
⬜⬛⬛⬛⬛⬜
⬜⬛⬛⬛⬛⬜
⬜🍪⬛⬛⬛⬜
⬜🍪⬛⬛⬛⬜
⬜⬜⬜⬜⬜⬜
"
    );
    let res = place(&client, base_url, test, "cookie", 1).await?;
    assert_status!(res, test, StatusCode::OK);
    assert_text!(
        res,
        test,
        "\
⬜⬛⬛⬛⬛⬜
⬜🍪⬛⬛⬛⬜
⬜🍪⬛⬛⬛⬜
⬜🍪⬛⬛⬛⬜
⬜⬜⬜⬜⬜⬜
"
    );
    let res = place(&client, base_url, test, "cookie", 1).await?;
    assert_status!(res, test, StatusCode::OK);
    assert_text!(
        res,
        test,
        "\
⬜🍪⬛⬛⬛⬜
⬜🍪⬛⬛⬛⬜
⬜🍪⬛⬛⬛⬜
⬜🍪⬛⬛⬛⬜
⬜⬜⬜⬜⬜⬜
🍪 wins!
"
    );
    let res = place(&client, base_url, test, "cookie", 1).await?;
    assert_status!(res, test, StatusCode::SERVICE_UNAVAILABLE);
    assert_text!(
        res,
        test,
        "\
⬜🍪⬛⬛⬛⬜
⬜🍪⬛⬛⬛⬜
⬜🍪⬛⬛⬛⬜
⬜🍪⬛⬛⬛⬜
⬜⬜⬜⬜⬜⬜
🍪 wins!
"
    );
    let res = place(&client, base_url, test, "milk", 2).await?;
    assert_status!(res, test, StatusCode::SERVICE_UNAVAILABLE);
    assert_text!(
        res,
        test,
        "\
⬜🍪⬛⬛⬛⬜
⬜🍪⬛⬛⬛⬜
⬜🍪⬛⬛⬛⬜
⬜🍪⬛⬛⬛⬜
⬜⬜⬜⬜⬜⬜
🍪 wins!
"
    );
    let res = client.get(board_url).send().await.map_err(|_| test)?;
    assert_status!(res, test, StatusCode::OK);
    assert_text!(
        res,
        test,
        "\
⬜🍪⬛⬛⬛⬜
⬜🍪⬛⬛⬛⬜
⬜🍪⬛⬛⬛⬜
⬜🍪⬛⬛⬛⬜
⬜⬜⬜⬜⬜⬜
🍪 wins!
"
    );
    test = (2, 2);
    let res = client.post(reset_url).send().await.map_err(|_| test)?;
    assert_status!(res, test, StatusCode::OK);
    assert_text!(
        res,
        test,
        "\
⬜⬛⬛⬛⬛⬜
⬜⬛⬛⬛⬛⬜
⬜⬛⬛⬛⬛⬜
⬜⬛⬛⬛⬛⬜
⬜⬜⬜⬜⬜⬜
"
    );
    let res = place(&client, base_url, test, "cookie", 1).await?;
    assert_status!(res, test, StatusCode::OK);
    let res = place(&client, base_url, test, "milk", 2).await?;
    assert_status!(res, test, StatusCode::OK);
    let res = place(&client, base_url, test, "cookie", 2).await?;
    assert_status!(res, test, StatusCode::OK);
    let res = place(&client, base_url, test, "milk", 3).await?;
    assert_status!(res, test, StatusCode::OK);
    let res = place(&client, base_url, test, "milk", 3).await?;
    assert_status!(res, test, StatusCode::OK);
    let res = place(&client, base_url, test, "cookie", 3).await?;
    assert_status!(res, test, StatusCode::OK);
    let res = place(&client, base_url, test, "milk", 4).await?;
    assert_status!(res, test, StatusCode::OK);
    let res = place(&client, base_url, test, "milk", 4).await?;
    assert_status!(res, test, StatusCode::OK);
    let res = place(&client, base_url, test, "milk", 4).await?;
    assert_status!(res, test, StatusCode::OK);
    let res = place(&client, base_url, test, "cookie", 4).await?;
    assert_status!(res, test, StatusCode::OK);
    assert_text!(
        res,
        test,
        "\
⬜⬛⬛⬛🍪⬜
⬜⬛⬛🍪🥛⬜
⬜⬛🍪🥛🥛⬜
⬜🍪🥛🥛🥛⬜
⬜⬜⬜⬜⬜⬜
🍪 wins!
"
    );
    tokio::time::sleep(Duration::from_millis(1000)).await;
    test = (2, 3);
    let res = client.post(reset_url).send().await.map_err(|_| test)?;
    assert_status!(res, test, StatusCode::OK);
    assert_text!(
        res,
        test,
        "\
⬜⬛⬛⬛⬛⬜
⬜⬛⬛⬛⬛⬜
⬜⬛⬛⬛⬛⬜
⬜⬛⬛⬛⬛⬜
⬜⬜⬜⬜⬜⬜
"
    );
    let res = place(&client, base_url, test, "cookie", 1).await?;
    assert_status!(res, test, StatusCode::OK);
    let res = place(&client, base_url, test, "cookie", 1).await?;
    assert_status!(res, test, StatusCode::OK);
    let res = place(&client, base_url, test, "cookie", 1).await?;
    assert_status!(res, test, StatusCode::OK);
    let res = place(&client, base_url, test, "milk", 1).await?;
    assert_status!(res, test, StatusCode::OK);
    let res = place(&client, base_url, test, "milk", 2).await?;
    assert_status!(res, test, StatusCode::OK);
    let res = place(&client, base_url, test, "milk", 2).await?;
    assert_status!(res, test, StatusCode::OK);
    let res = place(&client, base_url, test, "milk", 2).await?;
    assert_status!(res, test, StatusCode::OK);
    let res = place(&client, base_url, test, "cookie", 2).await?;
    assert_status!(res, test, StatusCode::OK);
    let res = place(&client, base_url, test, "cookie", 3).await?;
    assert_status!(res, test, StatusCode::OK);
    let res = place(&client, base_url, test, "cookie", 3).await?;
    assert_status!(res, test, StatusCode::OK);
    let res = place(&client, base_url, test, "cookie", 3).await?;
    assert_status!(res, test, StatusCode::OK);
    let res = place(&client, base_url, test, "milk", 3).await?;
    assert_status!(res, test, StatusCode::OK);
    let res = place(&client, base_url, test, "milk", 4).await?;
    assert_status!(res, test, StatusCode::OK);
    let res = place(&client, base_url, test, "milk", 4).await?;
    assert_status!(res, test, StatusCode::OK);
    let res = place(&client, base_url, test, "milk", 4).await?;
    assert_status!(res, test, StatusCode::OK);
    let res = place(&client, base_url, test, "cookie", 4).await?;
    assert_status!(res, test, StatusCode::OK);
    assert_text!(
        res,
        test,
        "\
⬜🥛🍪🥛🍪⬜
⬜🍪🥛🍪🥛⬜
⬜🍪🥛🍪🥛⬜
⬜🍪🥛🍪🥛⬜
⬜⬜⬜⬜⬜⬜
No winner.
"
    );
    tokio::time::sleep(Duration::from_millis(1000)).await;
    test = (2, 4);
    let res = client.post(reset_url).send().await.map_err(|_| test)?;
    assert_status!(res, test, StatusCode::OK);
    assert_text!(
        res,
        test,
        "\
⬜⬛⬛⬛⬛⬜
⬜⬛⬛⬛⬛⬜
⬜⬛⬛⬛⬛⬜
⬜⬛⬛⬛⬛⬜
⬜⬜⬜⬜⬜⬜
"
    );
    let res = place(&client, base_url, test, "cookie", 1).await?;
    assert_status!(res, test, StatusCode::OK);
    let res = place(&client, base_url, test, "milk", 2).await?;
    assert_status!(res, test, StatusCode::OK);
    let res = place(&client, base_url, test, "cookie", 2).await?;
    assert_status!(res, test, StatusCode::OK);
    let res = place(&client, base_url, test, "milk", 2).await?;
    assert_status!(res, test, StatusCode::OK);
    let res = place(&client, base_url, test, "milk", 1).await?;
    assert_status!(res, test, StatusCode::OK);
    let res = place(&client, base_url, test, "cookie", 1).await?;
    assert_status!(res, test, StatusCode::OK);
    let res = place(&client, base_url, test, "cookie", 3).await?;
    assert_status!(res, test, StatusCode::OK);
    let res = place(&client, base_url, test, "milk", 3).await?;
    assert_status!(res, test, StatusCode::OK);
    let res = place(&client, base_url, test, "milk", 2).await?;
    assert_status!(res, test, StatusCode::OK);
    let res = place(&client, base_url, test, "milk", 4).await?;
    assert_status!(res, test, StatusCode::OK);
    let res = place(&client, base_url, test, "cookie", 3).await?;
    assert_status!(res, test, StatusCode::OK);
    let res = place(&client, base_url, test, "milk", 1).await?;
    assert_status!(res, test, StatusCode::OK);
    assert_text!(
        res,
        test,
        "\
⬜🥛🥛⬛⬛⬜
⬜🍪🥛🍪⬛⬜
⬜🥛🍪🥛⬛⬜
⬜🍪🥛🍪🥛⬜
⬜⬜⬜⬜⬜⬜
🥛 wins!
"
    );
    test = (2, 5);
    let res = place(&client, base_url, test, "milk", 4).await?;
    assert_status!(res, test, StatusCode::SERVICE_UNAVAILABLE);
    let res = client.post(reset_url).send().await.map_err(|_| test)?;
    assert_status!(res, test, StatusCode::OK);
    let res = place(&client, base_url, test, "cookie", 0).await?;
    assert_status!(res, test, StatusCode::BAD_REQUEST);
    let res = place(&client, base_url, test, "cookie", 5).await?;
    assert_status!(res, test, StatusCode::BAD_REQUEST);
    let res = place(&client, base_url, test, "cookie", -2).await?;
    assert_status!(res, test, StatusCode::BAD_REQUEST);
    let res = client
        .post(format!("{}/12/place/cookie/one", base_url))
        .send()
        .await
        .map_err(|_| test)?;
    assert_status!(res, test, StatusCode::BAD_REQUEST);
    let res = place(&client, base_url, test, "plastic", 1).await?;
    assert_status!(res, test, StatusCode::BAD_REQUEST);
    // TASK 2 DONE
    tx.send((true, 0).into()).await.unwrap();
    tx.send(SubmissionUpdate::Save).await.unwrap();

    // TASK 3: random
    test = (3, 1);
    let url = &format!("{}/12/random-board", base_url);
    let res = client.post(reset_url).send().await.map_err(|_| test)?;
    assert_status!(res, test, StatusCode::OK);
    let res = client.get(url).send().await.map_err(|_| test)?;
    assert_status!(res, test, StatusCode::OK);
    assert_text_starts_with!(
        res,
        test,
        "\
⬜🍪🍪🍪🍪⬜
⬜🥛🍪🍪🥛⬜
⬜🥛🥛🥛🥛⬜
⬜🍪🥛🍪🥛⬜
⬜⬜⬜⬜⬜⬜
"
    );
    let res = client.get(url).send().await.map_err(|_| test)?;
    assert_status!(res, test, StatusCode::OK);
    assert_text_starts_with!(
        res,
        test,
        "\
⬜🍪🥛🍪🍪⬜
⬜🥛🍪🥛🍪⬜
⬜🥛🍪🍪🍪⬜
⬜🍪🥛🥛🥛⬜
⬜⬜⬜⬜⬜⬜
"
    );
    let res = client.get(url).send().await.map_err(|_| test)?;
    assert_status!(res, test, StatusCode::OK);
    assert_text_starts_with!(
        res,
        test,
        "\
⬜🍪🍪🥛🍪⬜
⬜🍪🥛🍪🍪⬜
⬜🥛🍪🍪🥛⬜
⬜🍪🥛🍪🍪⬜
⬜⬜⬜⬜⬜⬜
"
    );
    let res = client.get(url).send().await.map_err(|_| test)?;
    assert_status!(res, test, StatusCode::OK);
    assert_text_starts_with!(
        res,
        test,
        "\
⬜🥛🍪🍪🥛⬜
⬜🥛🍪🍪🍪⬜
⬜🍪🥛🥛🥛⬜
⬜🍪🥛🍪🥛⬜
⬜⬜⬜⬜⬜⬜
"
    );
    let res = client.get(url).send().await.map_err(|_| test)?;
    assert_status!(res, test, StatusCode::OK);
    assert_text_starts_with!(
        res,
        test,
        "\
⬜🥛🥛🥛🍪⬜
⬜🍪🍪🍪🥛⬜
⬜🥛🍪🍪🥛⬜
⬜🍪🥛🥛🍪⬜
⬜⬜⬜⬜⬜⬜
"
    );
    let res = client.post(reset_url).send().await.map_err(|_| test)?;
    assert_status!(res, test, StatusCode::OK);
    let res = client.get(url).send().await.map_err(|_| test)?;
    assert_status!(res, test, StatusCode::OK);
    assert_text_starts_with!(
        res,
        test,
        "\
⬜🍪🍪🍪🍪⬜
⬜🥛🍪🍪🥛⬜
⬜🥛🥛🥛🥛⬜
⬜🍪🥛🍪🥛⬜
⬜⬜⬜⬜⬜⬜
"
    );
    // TASK 3 DONE
    tx.send((false, 75).into()).await.unwrap();
    tx.send(SubmissionUpdate::Save).await.unwrap();

    Ok(())
}

async fn validate_16(base_url: &str, tx: Sender<SubmissionUpdate>) -> ValidateResult {
    let mut test: TaskTest;
    // TASK 1: jwt cookie
    test = (1, 1);
    let url1 = &format!("{}/16/wrap", base_url);
    let url2 = &format!("{}/16/unwrap", base_url);
    let client = new_client_with_cookies();
    let payload = json!({"cookie": "yum"});
    let res = client
        .post(url1)
        .json(&payload)
        .send()
        .await
        .map_err(|_| test)?;
    assert_status!(res, test, StatusCode::OK);
    let h = res
        .headers()
        .get(header::SET_COOKIE)
        .ok_or(test)?
        .to_str()
        .map_err(|_| test)?;
    let h = h.strip_prefix("gift=").ok_or(test)?;
    decode_header(h).map_err(|_| test)?;
    let res = client.get(url2).send().await.map_err(|_| test)?;
    assert_status!(res, test, StatusCode::OK);
    assert_text!(res, test, serde_json::to_string(&payload).unwrap());
    test = (1, 2);
    let c1 = new_client_with_cookies();
    let c2 = new_client_with_cookies();
    let c3 = new_client_with_cookies();
    let p1 = json!({"recipient": "p1", "gifts": ["Toy train", "Caramel corn", "Potato"]});
    let p2 = json!({"recipient": "p2", "gifts": ["Toy train", "Caramel corn", "Potato"]});
    let p3 = json!({"recipient": "p3", "gifts": ["Toy train", "Caramel corn", "Potato"]});
    let res = c1.post(url1).json(&p1).send().await.map_err(|_| test)?;
    assert_status!(res, test, StatusCode::OK);
    let res = c2.post(url1).json(&p2).send().await.map_err(|_| test)?;
    assert_status!(res, test, StatusCode::OK);
    let res = c3.post(url1).json(&p3).send().await.map_err(|_| test)?;
    assert_status!(res, test, StatusCode::OK);
    let res = c1.get(url2).send().await.map_err(|_| test)?;
    assert_status!(res, test, StatusCode::OK);
    assert_json!(res, test, p1);
    let res = c3.get(url2).send().await.map_err(|_| test)?;
    assert_status!(res, test, StatusCode::OK);
    assert_json!(res, test, p3);
    test = (1, 3);
    let client = new_client();
    let res = client.get(url2).send().await.map_err(|_| test)?;
    assert_status!(res, test, StatusCode::BAD_REQUEST);
    test = (1, 4);
    let client = new_client();
    let res = client
        .get(url2)
        .header("Cookie", "candy=5")
        .send()
        .await
        .map_err(|_| test)?;
    assert_status!(res, test, StatusCode::BAD_REQUEST);
    // TASK 1 DONE
    tx.send((true, 0).into()).await.unwrap();
    tx.send(SubmissionUpdate::Save).await.unwrap();

    // TASK 2: decode
    let client = new_client();
    let url = &format!("{}/16/decode", base_url);
    test = (2, 1);
    let res = client
        .post(url)
        .body(
            "eyJ0eXAiOiJKV1QiLCJhbGciOiJSUzI1NiJ9.eyJyZWluZGVlclNuYWNrIjoiY2Fycm90cyIsInNhbnRhSGF0Q29sb3IiOiJyZWQiLCJzbm93R2xvYmVDb2xsZWN0aW9uIjo1LCJzdG9ja2luZ1N0dWZmZXJzIjpbInlvLXlvIiwiY2FuZHkiLCJrZXljaGFpbiJdLCJ0cmVlSGVpZ2h0Ijo3fQ.EoWSlwZIMHdtd96U_FkfQ9SkbzskSvgEaRpsUeZQFJixDW57vZud_k-MK1R1LEGoJRPGttJvG_5ewdK9O46OuaGW4DHIOWIFLxSYFTJBdFMVmAWC6snqartAFr2U-LWxTwJ09WNpPBcL67YCx4HQsoGZ2mxRVNIKxR7IEfkZDhmpDkiAUbtKyn0H1EVERP1gdbzHUGpLd7wiuzkJnjenBgLPifUevxGPgj535cp8I6EeE4gLdMEm3lbUW4wX_GG5t6_fDAF4URfiAOkSbiIW6lKcSGD9MBVEGps88lA2REBEjT4c7XHw4Tbxci2-knuJm90zIA9KX92t96tF3VFKEA"
        )
        .send()
        .await
        .map_err(|_| test)?;
    assert_status!(res, test, StatusCode::OK);
    assert_json!(
        res,
        test,
        json!({"stockingStuffers":["yo-yo","candy","keychain"],"reindeerSnack":"carrots","treeHeight":7,"santaHatColor":"red","snowGlobeCollection":5})
    );
    test = (2, 2);
    let res = client
        .post(url)
        .body(
            "eyJ0eXAiOiJKV1QiLCJhbGciOiJSUzI1NiJ9.eyJnaWZ0cyI6WyJDb2FsIl19.DaVXV_czINRO1Cvhw33YSPSpV7_TYTqp7gIB_XiVl5fh3K9zkmDItBFLxJHyb7TRw_CGrAYwfinxn6_Dn9MMhp8d3tc-UnRskOxNHpqwU9EcbDtn31uHStT5sLfzdK0fdAc1XUJnr-9dbiGiYARO9YK7HAijdR8bCRMtvMUgIHsumWHO5BEE4CCeVgypzkebsoaev495OE0VNCfn1rSbTKR12xiIFoPCZALV9_slqoZvO59K0x8DSppx7uHApGjXvS6JmyjVgMJNuJoPrIYzc0nytVCa5uLjYIadS2inw7Sty1Jj-sLi8AgtYCXcpyB59MUXNP5xze_Sat8hmQ_NzQ"
        )
        .send()
        .await
        .map_err(|_| test)?;
    assert_status!(res, test, StatusCode::UNAUTHORIZED);
    test = (2, 3);
    let res = client
        .post(url)
        .body(
            "eyJ0eXAiOiJKV1QiLCJhbGciOiJSUzI1NiJ9.eyJjYW5kbGVTY2VudHMiOlsicGluZSIsImNpbm5hbW9uIiwidmFuaWxsYSJdLCJmZXN0aXZlU29ja3MiOjEyLCJnaWZ0VGFncyI6WyJwZXJzb25hbGl6ZWQiLCJibGFuayIsInNwYXJrbHkiXSwiZ2luZ2VyYnJlYWRIb3VzZUtpdHMiOjMsImhvdENvY29hU3RvY2siOjI1fQ.GgYB9NXomy-s_lzmoRC-BFHUvrSMjDMcZ4jFCre6NaPJA2fKr--cadxerpody-H5wV19N2zguNb5gr6dt7-suegC8D2ANe9mExohY9tuqgGKRJdLqtmb8U91T_iRg2kyAyhrv3HlSUHQP3sxvAO7jcwLtbePQehtzb6Hv9tZqNCojxMJmAhrJxz41fnD9wvTsEZVpQVwo21C-GIpZKRUGJnaL6OU9IAY6D4PMUr4X9OjEC1zSdQWpYUW_8CHrGNYPVg-6ZpdEvkejxZGTwPg8pMPPSxRa6g0v7Scx-50pgjcP15VK2OUaF9xce7MReJOgI2dxtF35DpYT-UNsIWDKg"
        )
        .send()
        .await
        .map_err(|_| test)?;
    assert_status!(res, test, StatusCode::OK);
    assert_json!(
        res,
        test,
        json!({"giftTags":["personalized","blank","sparkly"],"hotCocoaStock":25,"candleScents":["pine","cinnamon","vanilla"],"gingerbreadHouseKits":3,"festiveSocks":12})
    );
    test = (2, 4);
    let res = client
        .post(url)
        .body(
            "eyJ0eXAiOiJKV1QiLCJhbGciOiJSUzUxMiJ9.eyJjYXJvbGluZ1JvdXRlIjpbIk1haW4gU3RyZWV0IiwiRWxtIEF2ZW51ZSIsIkJha2VyIFN0cmVldCJdLCJjb29raWVSZWNpcGVzIjpbInN1Z2FyIGNvb2tpZXMiLCJzbmlja2VyZG9vZGxlcyIsInNob3J0YnJlYWQiXSwiZmVzdGl2ZVB1bmNoSW5ncmVkaWVudHMiOlsiY3JhbmJlcnJ5IGp1aWNlIiwiZ2luZ2VyIGFsZSIsIm9yYW5nZSBzbGljZXMiXSwiZmlyZXBsYWNlTWFudGxlRGVjb3IiOlsiZ2FybGFuZCIsInN0b2NraW5ncyIsImNhbmRsZXMiXSwiZ2lmdENhcmRPcHRpb25zIjpbImJvb2tzdG9yZSIsImNvZmZlZSBzaG9wIiwib25saW5lIHJldGFpbGVyIl0sImhvbGlkYXlDYXJkTGlzdCI6WyJmYW1pbHkiLCJmcmllbmRzIiwiY293b3JrZXJzIl0sIm51dGNyYWNrZXJDb2xsZWN0aW9uU2l6ZXMiOnsibGFyZ2UiOjEsIm1lZGl1bSI6Mywic21hbGwiOjV9LCJzbm93bWFuQnVpbGRpbmdLaXRzIjo0fQ.ZAThp4qXSV1eY8swvPa9OmQrTglgILGWHzR_DN-gslN1dYNPszb2Hy322hiHIht_ASdXcV7-LNatS-P1yIpg7YnIRpZUgg5_Cb3uvucuna0npqfV3U3tTeqDAikPCs5bc7pWjawVscvabJjDm-WPCwLe9o4YMCSFb_XPra6lAHARRrMyqms2PjjdBE3WcUT_wYQq7WwgChXCXHMCOa1XoKIMoegSesYdSXNbbrckDvwdty9GsASCHaX9TAIY4TNdSdl3RanqDlrRDdwjvs5A9dQUul-JzHLxvSodJAGqxxPODNG_P1l0KRlmlVZVZSRqgFC_wH3sziHyVsM1WayjWQ"
        )
        .send()
        .await
        .map_err(|_| test)?;
    assert_status!(res, test, StatusCode::OK);
    assert_json!(
        res,
        test,
        json!({"cookieRecipes":["sugar cookies","snickerdoodles","shortbread"],"fireplaceMantleDecor":["garland","stockings","candles"],"snowmanBuildingKits":4,"holidayCardList":["family","friends","coworkers"],"nutcrackerCollectionSizes":{"small":5,"medium":3,"large":1},"festivePunchIngredients":["cranberry juice","ginger ale","orange slices"],"carolingRoute":["Main Street","Elm Avenue","Baker Street"],"giftCardOptions":["bookstore","coffee shop","online retailer"]})
    );
    test = (2, 5);
    let res = client
        .post(url)
        .body(
            "eyJ0eXAiOiJKV1QiLCJhbGciOiJSUzI1NiJ9.eyJlbHZlcyI6WyJKaW5nbGUiLCJUd2lua2xlIiwiVGluc2VsIl0sImdpZnRGYWN0b3J5Ijp7ImxvY2F0aW9uIjoiTm9ydGggUG9sZSIsIm91dHB1dFBlckhvdXIiOjUwMDB9LCJnaWZ0SWRlYXMiOlsidG95IHRyYWluIiwiYWN0aW9uIGZpZ3VyZSIsInRlZGR5IGJlYXIiLCJsZWdvIHNldCJdLCJyZWluZGVlciI6IlJ1ZG9scGgiLCJzYW50YSI6eyJhZ2UiOjE3NTAsIm5hbWUiOiJLcmlzIEtyaW5nbGUifSwic3VycHJpc2VFbGVtZW50Ijp0cnVlLCJ3aXNobGlzdCI6eyJicm90aGVyIjoidmlkZW8gZ2FtZSIsImRhZCI6InNvY2tzIiwibW9tIjoiY2hvY29sYXRlcyIsInNpc3RlciI6InBvcCBjdWx0dXJlIHBvc3RlciJ9LCJ3cmFwcGluZyI6eyJwYXBlclR5cGVzIjpbImdsb3NzeSIsIm1hdHRlIiwic3BhcmtsZSJdfX0.lQDLhwqrWAn8jPV-lzPuEQE7fFt30yao5M7jADhg3ipwRYYOB8g9sT5TrIufKKCMpNk8qxxgZX9rGJrGVqmdVLRXmyMMgxhiVuboxtI8RlhAEgzNQR6z7G3OWJ-ZccOEHVjdXBQwtpQeLMwoDDHK6UnVsWSrLai5n-VI87QOyxz_2VVj_cR9mtsSEU9rMxZBly1KD5-f-pQHwOczOlAerdp-bgQpANH6uR94AQGENMRQaY7tr_ldh5DNpP9gL0K3oZD3HbEBvYv8OS498mq_09BqVFrp9nmgB4JGhYzNqyFbad8f52sdBRle-ewNR55uxDHq6e10IdJQ_PR34gGPjw"
        )
        .send()
        .await
        .map_err(|_| test)?;
    assert_status!(res, test, StatusCode::UNAUTHORIZED);
    test = (2, 6);
    let res = client
        .post(url)
        .body(
            "eyJ0eXAiOiJKV1QiLCJhbGciOiJSUzI1NiJ9.eyJjYW5keUNhbmVTdG9jayI6MTUwMCwiY2Fyb2xQbGF5bGlzdCI6WyJKaW5nbGUgQmVsbHMiLCJTaWxlbnQgTmlnaHQiLCJEZWNrIHRoZSBIYWxscyJdLCJmYXZvcml0ZUNvb2tpZXMiOlsiY2hvY29sYXRlIGNoaXAiLCJvYXRtZWFsIHJhaXNpbiIsImdpbmdlcmJyZWFkIl0sImdpZnRFeGNoYW5nZVJ1bGVzIjp7Im1heEJ1ZGdldCI6NTAsInRoZW1lIjoiaGFuZG1hZGUifSwicmVpbmRlZXJOYW1lcyI6WyJEYXNoZXIiLCJEYW5jZXIiLCJQcmFuY2VyIiwiVml4ZW4iLCJDb21ldCIsIkN1cGlkIiwiRG9ubmVyIiwiQmxpdHplbiJdLCJzZWNyZXRTYW50YSI6eyJkcmF3RGF0ZSI6IjIwMjMtMTItMDEiLCJwYXJ0aWNpcGFudHMiOlsiQWxpY2UiLCJCb2IiLCJDaGFybGllIl19LCJzbGVpZ2giOnsiY29sb3IiOiJyZWQiLCJmdWVsVHlwZSI6Im1hZ2ljIGR1c3QifSwidHJlZURlY29yYXRpb25zIjpbImxpZ2h0cyIsImJhdWJsZXMiLCJ0aW5zZWwiLCJzdGFyIl19.MGtse2G55XIZTSWa2IdNI6YCKsFKsGEonkH0iIlRUuELY6nBdPnLpI4oFEB4-yK8j2eVcQALS3J3YbVUk-LLpIazaVJ5uJ9r-VvBNZqe_Uih8GQjVmINMEHdQwh6v2T2h4FLOqs2wap4SS6q25BVz2v0urycbCo_6IiHvswgkqRk9ZBA_bFDXEKRCoKLdgcNxnYRbkbLVvOzVpvhHFRYOsiwBxBiMakkjp3ZmvV5vaMQaSFUsmW9CHoU0ffbdwOwyMUXrxphSYB7h4OAZeudnZa7ntoOZ6J3PJQCTvgU7llffTPcdoO6LVoXSD8hiIfvJWPKgsOgasyG_xEQmfGcsA"
        )
        .send()
        .await
        .map_err(|_| test)?;
    assert_status!(res, test, StatusCode::UNAUTHORIZED);
    for (test, txt) in [
        ((2, 7), "eyJ0eXAiOiJKV1QiLCJhbGciOiJSUzI1NiJ9.eyJyZWluZGVlclNuYWNrIjoiY2Fycm90cyIsInNhbnRhSGF0Q29sb3IiOiJyZWQiLCJzbm93R2xvYmVDb2xsZWN0aW9uIjo1LCJzdG9ja2luZ1N0dWZmZXJzIjpbInlvLXlvIiwiY2FuZHkiLCJrZXljaGFpbiJdLCJ0cmVlSGVpZ2h0Ijo3fQEoWSlwZIMHdtd96U_FkfQ9SkbzskSvgEaRpsUeZQFJixDW57vZud_k-MK1R1LEGoJRPGttJvG_5ewdK9O46OuaGW4DHIOWIFLxSYFTJBdFMVmAWC6snqartAFr2U-LWxTwJ09WNpPBcL67YCx4HQsoGZ2mxRVNIKxR7IEfkZDhmpDkiAUbtKyn0H1EVERP1gdbzHUGpLd7wiuzkJnjenBgLPifUevxGPgj535cp8I6EeE4gLdMEm3lbUW4wX_GG5t6_fDAF4URfiAOkSbiIW6lKcSGD9MBVEGps88lA2REBEjT4c7XHw4Tbxci2-knuJm90zIA9KX92t96tF3VFKEA"),
        ((2, 8), "eyJ0eXAiOiJKV1QiLCJhbGci0iJSUzI1NiJ9.eyJyZWluZGVlclNuYWNrIjoiY2Fycm90cyIsInNhbnRhSGF0Q29sb3IiOiJyZWQiLCJzbm93R2xvYmVDb2xsZWN0aW9uIjo1LCJzdG9ja2luZ1N0dWZmZXJzIjpbInlvLXlvIiwiY2FuZHkiLCJrZXljaGFpbiJdLCJ0cmVlSGVpZ2h0Ijo3fQ.EoWSlwZIMHdtd96U_FkfQ9SkbzskSvgEaRpsUeZQFJixDW57vZud_k-MK1R1LEGoJRPGttJvG_5ewdK9O46OuaGW4DHIOWIFLxSYFTJBdFMVmAWC6snqartAFr2U-LWxTwJ09WNpPBcL67YCx4HQsoGZ2mxRVNIKxR7IEfkZDhmpDkiAUbtKyn0H1EVERP1gdbzHUGpLd7wiuzkJnjenBgLPifUevxGPgj535cp8I6EeE4gLdMEm3lbUW4wX_GG5t6_fDAF4URfiAOkSbiIW6lKcSGD9MBVEGps88lA2REBEjT4c7XHw4Tbxci2-knuJm90zIA9KX92t96tF3VFKEA"),
        ((2, 9), "eyJ0eXAiOiJKV1QiLCJhbGciOiJSUzI1NiJ9.eyJy.ZWluZGVlclNuYWNrIjoiY2Fycm90cyIsInNhbnRhSGF0Q29sb3IiOiJyZWQiLCJzbm93R2xvYmVDb2xsZWN0aW9uIjo1LCJzdG9ja2luZ1N0dWZmZXJzIjpbInlvLXlvIiwiY2FuZHkiLCJrZXljaGFpbiJdLCJ0cmVlSGVpZ2h0Ijo3fQ.EoWSlwZIMHdtd96U_FkfQ9SkbzskSvgEaRpsUeZQFJixDW57vZud_k-MK1R1LEGoJRPGttJvG_5ewdK9O46OuaGW4DHIOWIFLxSYFTJBdFMVmAWC6snqartAFr2U-LWxTwJ09WNpPBcL67YCx4HQsoGZ2mxRVNIKxR7IEfkZDhmpDkiAUbtKyn0H1EVERP1gdbzHUGpLd7wiuzkJnjenBgLPifUevxGPgj535cp8I6EeE4gLdMEm3lbUW4wX_GG5t6_fDAF4URfiAOkSbiIW6lKcSGD9MBVEGps88lA2REBEjT4c7XHw4Tbxci2-knuJm90zIA9KX92t96tF3VFKEA"),
        ((2, 10), "eyJ0eXAiOiJKV1QiLCJhbGciOiJSUzI1NiJ9.ImNvYWwi.cTlGrCeHzvweR-b7U1PZn3fpNk5P_C8wjTo2s93itoYdzeJwUunHTfPY9MJ3Mmif_2MDveSf7b_xID4fRhnXzEBNblIXtlfoNE1lWGPurOvB8udxxJk30qM6sG-ldK79TKzt784ok1ecyuAP94vMjKK861YUoqq5bfZdr9YwIq0chJOx0RZG0zY2OS7VVoOG-SbOssHb-eZKysCt-r8zrIwJGXoSe6H5ZYX7dN5l9CbJ6t29D89I0SkZj2TI2unBG5UueXIw6VukwREzDPTKJTdh6AbnMRwoi7GGIlayhUaFtAGPrlnS2razOmAWndtSv9rDNELJirN2AQ7iyRbqyg"),
    ] {
        let res = client.post(url).body(txt).send().await.map_err(|_| test)?;
        assert_status!(res, test, StatusCode::BAD_REQUEST);
    }
    // TASK 2 DONE
    tx.send((false, 200).into()).await.unwrap();
    tx.send(SubmissionUpdate::Save).await.unwrap();

    Ok(())
}

async fn validate_19(base_url: &str, tx: Sender<SubmissionUpdate>) -> ValidateResult {
    let client = new_client();
    let mut test: TaskTest;
    // TASK 1: CRUD
    test = (1, 1);
    let reset_url = &format!("{}/19/reset", base_url);
    let cite_url = &format!("{}/19/cite", base_url);
    let remove_url = &format!("{}/19/remove", base_url);
    let undo_url = &format!("{}/19/undo", base_url);
    let draft_url = &format!("{}/19/draft", base_url);
    let res = client.post(reset_url).send().await.map_err(|_| test)?;
    assert_status!(res, test, StatusCode::OK);

    async fn validate_quote(
        res: reqwest::Response,
        test: (i32, i32),
        sent: &serde_json::Value,
        version: i64,
    ) -> Result<Uuid, TaskTest> {
        quote_matches(
            test,
            sent,
            &res.json::<serde_json::Value>().await.map_err(|_| test)?,
            version,
        )
        .await
    }
    async fn quote_matches(
        test: (i32, i32),
        exp: &serde_json::Value,
        act: &serde_json::Value,
        version: i64,
    ) -> Result<Uuid, TaskTest> {
        assert_eq_!(test, act.as_object().ok_or(test)?.len(), 5);
        assert_!(test, act.get("author") == exp.get("author"));
        assert_!(test, act.get("quote") == exp.get("quote"));
        assert_!(
            test,
            act.get("version")
                .is_some_and(|v| v.as_i64().is_some_and(|v| v == version))
        );
        act.get("created_at")
            .ok_or(test)?
            .as_str()
            .ok_or(test)?
            .parse::<DateTime<Utc>>()
            .map_err(|_| test)?;
        let id: Uuid = act
            .get("id")
            .ok_or(test)?
            .as_str()
            .ok_or(test)?
            .parse()
            .map_err(|_| test)?;

        Ok(id)
    }

    let quote1 = json!({"author":"Santa","quote":"Ho ho ho! Spread cheer and kindness, for that's the true magic of the season!"});
    let quote2 = json!({"author":"Santa's best elf","quote":"In the glow of snow and twinkling light, dreams take flight on a magical night!"});
    let quote3 = json!({"author":"Dasher","quote":"Whoosh and clatter, my hooves pitter-patter!"});
    let quote4 = json!({"author":"Polar Bear","quote":"Roar!"});
    let res = client
        .post(draft_url)
        .json(&quote1)
        .send()
        .await
        .map_err(|_| test)?;
    assert_status!(res, test, StatusCode::CREATED);
    let id = validate_quote(res, test, &quote1, 1).await?;

    let res = client
        .get(format!("{}/{}", cite_url, id))
        .send()
        .await
        .map_err(|_| test)?;
    assert_status!(res, test, StatusCode::OK);
    validate_quote(res, test, &quote1, 1).await?;

    let res = client
        .put(format!("{}/{}", undo_url, id))
        .json(&quote2)
        .send()
        .await
        .map_err(|_| test)?;
    assert_status!(res, test, StatusCode::OK);
    let id2 = validate_quote(res, test, &quote2, 2).await?;
    assert_eq_!(test, id, id2);

    let res = client
        .delete(format!("{}/{}", remove_url, id))
        .send()
        .await
        .map_err(|_| test)?;
    assert_status!(res, test, StatusCode::OK);
    validate_quote(res, test, &quote2, 2).await?;

    let res = client
        .get(format!("{}/{}", cite_url, id))
        .send()
        .await
        .map_err(|_| test)?;
    assert_status!(res, test, StatusCode::NOT_FOUND);

    test = (1, 2);
    let res = client
        .post(draft_url)
        .json(&quote1)
        .send()
        .await
        .map_err(|_| test)?;
    assert_status!(res, test, StatusCode::CREATED);
    let id = validate_quote(res, test, &quote1, 1).await?;
    let res = client
        .post(draft_url)
        .json(&quote1)
        .send()
        .await
        .map_err(|_| test)?;
    assert_status!(res, test, StatusCode::CREATED);
    let id2 = validate_quote(res, test, &quote1, 1).await?;
    assert_neq_!(test, id, id2);

    let res = client
        .put(format!("{}/{}", undo_url, id))
        .json(&quote2)
        .send()
        .await
        .map_err(|_| test)?;
    assert_status!(res, test, StatusCode::OK);
    validate_quote(res, test, &quote2, 2).await?;
    let res = client
        .get(format!("{}/{}", cite_url, id2))
        .send()
        .await
        .map_err(|_| test)?;
    assert_status!(res, test, StatusCode::OK);
    validate_quote(res, test, &quote1, 1).await?;
    let res = client
        .get(format!("{}/{}", cite_url, id))
        .send()
        .await
        .map_err(|_| test)?;
    assert_status!(res, test, StatusCode::OK);
    validate_quote(res, test, &quote2, 2).await?;

    let res = client
        .put(format!("{}/{}", undo_url, id))
        .json(&quote3)
        .send()
        .await
        .map_err(|_| test)?;
    assert_status!(res, test, StatusCode::OK);
    validate_quote(res, test, &quote3, 3).await?;
    let res = client
        .put(format!("{}/{}", undo_url, id))
        .json(&quote1)
        .send()
        .await
        .map_err(|_| test)?;
    assert_status!(res, test, StatusCode::OK);
    validate_quote(res, test, &quote1, 4).await?;

    test = (1, 3);
    let res = client
        .put(format!(
            "{}/{}",
            undo_url, "00000000-0000-0000-0000-000000000000"
        ))
        .json(&quote4)
        .send()
        .await
        .map_err(|_| test)?;
    assert_status!(res, test, StatusCode::NOT_FOUND);
    let res = client
        .delete(format!(
            "{}/{}",
            remove_url, "00000000-0000-0000-0000-000000000000"
        ))
        .send()
        .await
        .map_err(|_| test)?;
    assert_status!(res, test, StatusCode::NOT_FOUND);
    let res = client
        .get(format!(
            "{}/{}",
            cite_url, "00000000-0000-0000-0000-000000000000"
        ))
        .send()
        .await
        .map_err(|_| test)?;
    assert_status!(res, test, StatusCode::NOT_FOUND);
    let res = client
        .put(format!("{}/{}", undo_url, "1234"))
        .json(&quote4)
        .send()
        .await
        .map_err(|_| test)?;
    assert_status!(res, test, StatusCode::BAD_REQUEST);

    // TASK 1 DONE
    tx.send((true, 0).into()).await.unwrap();
    tx.send(SubmissionUpdate::Save).await.unwrap();

    // TASK 2: paginator
    test = (2, 1);
    let list_url = &format!("{}/19/list", base_url);
    async fn validate_quotes(
        res: reqwest::Response,
        test: (i32, i32),
        sent: &[(&serde_json::Value, i64)],
        page: i64,
    ) -> Result<Option<String>, TaskTest> {
        let json = res.json::<serde_json::Value>().await.map_err(|_| test)?;
        assert_!(
            test,
            json.get("page")
                .is_some_and(|v| v.as_i64().is_some_and(|v| v == page))
        );
        let quotes = json.get("quotes").ok_or(test)?.as_array().ok_or(test)?;
        for ((v, version), quote) in sent.iter().zip(quotes.iter()) {
            quote_matches(test, v, quote, *version).await?;
        }
        let next_token: Option<String> =
            serde_json::from_value(json.get("next_token").ok_or(test)?.clone())
                .map_err(|_| test)?;
        if let Some(t) = next_token.as_ref() {
            if t.chars().any(|c| !c.is_ascii_alphanumeric()) || t.len() != 16 {
                return Err(test);
            }
        }
        Ok(next_token)
    }
    let res = client.get(list_url).send().await.map_err(|_| test)?;
    assert_status!(res, test, StatusCode::OK);
    let n = validate_quotes(res, test, &[(&quote1, 4), (&quote1, 1)], 1).await?;
    assert_!(test, n.is_none());

    let res = client
        .post(draft_url)
        .json(&quote3)
        .send()
        .await
        .map_err(|_| test)?;
    assert_status!(res, test, StatusCode::CREATED);
    let id3 = validate_quote(res, test, &quote3, 1).await?;
    let res = client
        .post(draft_url)
        .json(&quote3)
        .send()
        .await
        .map_err(|_| test)?;
    assert_status!(res, test, StatusCode::CREATED);
    validate_quote(res, test, &quote3, 1).await?;

    let res = client.get(list_url).send().await.map_err(|_| test)?;
    assert_status!(res, test, StatusCode::OK);
    let n = validate_quotes(res, test, &[(&quote1, 4), (&quote1, 1), (&quote3, 1)], 1).await?;
    assert_!(test, n.is_some());
    let res = client
        .get(format!("{}?token={}", list_url, n.unwrap()))
        .send()
        .await
        .map_err(|_| test)?;
    assert_status!(res, test, StatusCode::OK);
    let n = validate_quotes(res, test, &[(&quote3, 1)], 2).await?;
    assert_!(test, n.is_none());

    test = (2, 2);
    let res = client
        .delete(format!("{}/{}", remove_url, id3))
        .send()
        .await
        .map_err(|_| test)?;
    assert_status!(res, test, StatusCode::OK);
    validate_quote(res, test, &quote3, 1).await?;
    let res = client.get(list_url).send().await.map_err(|_| test)?;
    assert_status!(res, test, StatusCode::OK);
    let n = validate_quotes(res, test, &[(&quote1, 4), (&quote1, 1), (&quote3, 1)], 1).await?;
    assert_!(test, n.is_none());

    test = (2, 3);
    let page1 = &[(&quote1, 4), (&quote1, 1), (&quote3, 1)];
    let page2 = &[(&quote2, 1), (&quote2, 1), (&quote3, 1)];
    let page3 = &[(&quote2, 1), (&quote3, 1), (&quote1, 1)];
    for &(q, v) in page2.iter().chain(page3.iter()) {
        let res = client
            .post(draft_url)
            .json(q)
            .send()
            .await
            .map_err(|_| test)?;
        assert_status!(res, test, StatusCode::CREATED);
        validate_quote(res, test, q, v).await?;
    }

    let res = client.get(list_url).send().await.map_err(|_| test)?;
    assert_status!(res, test, StatusCode::OK);
    let n = validate_quotes(res, test, page1, 1).await?;
    assert_!(test, n.is_some());
    let res = client
        .get(format!("{}?token={}", list_url, n.unwrap()))
        .send()
        .await
        .map_err(|_| test)?;
    assert_status!(res, test, StatusCode::OK);
    let n = validate_quotes(res, test, page2, 2).await?;
    assert_!(test, n.is_some());
    let res = client
        .get(format!("{}?token={}", list_url, n.unwrap()))
        .send()
        .await
        .map_err(|_| test)?;
    assert_status!(res, test, StatusCode::OK);
    let n = validate_quotes(res, test, page3, 3).await?;
    assert_!(test, n.is_none());

    test = (2, 4);
    let res = client
        .get(format!("{}?token=asd987f69as87d6q", list_url))
        .send()
        .await
        .map_err(|_| test)?;
    assert_status!(res, test, StatusCode::BAD_REQUEST);

    test = (2, 5);
    let res = client.get(list_url).send().await.map_err(|_| test)?;
    assert_status!(res, test, StatusCode::OK);
    let n1 = validate_quotes(res, test, page1, 1).await?;
    assert_!(test, n1.is_some());

    let res = client.get(list_url).send().await.map_err(|_| test)?;
    assert_status!(res, test, StatusCode::OK);
    let n2 = validate_quotes(res, test, page1, 1).await?;
    assert_!(test, n2.is_some());

    let res = client
        .get(format!("{}?token={}", list_url, n1.unwrap()))
        .send()
        .await
        .map_err(|_| test)?;
    assert_status!(res, test, StatusCode::OK);
    let n1 = validate_quotes(res, test, page2, 2).await?;
    assert_!(test, n1.is_some());
    let res = client
        .get(format!("{}?token={}", list_url, n1.unwrap()))
        .send()
        .await
        .map_err(|_| test)?;
    assert_status!(res, test, StatusCode::OK);
    let n1 = validate_quotes(res, test, page3, 3).await?;
    assert_!(test, n1.is_none());

    let res = client
        .get(format!("{}?token={}", list_url, n2.unwrap()))
        .send()
        .await
        .map_err(|_| test)?;
    assert_status!(res, test, StatusCode::OK);
    let n2 = validate_quotes(res, test, page2, 2).await?;
    assert_!(test, n2.is_some());
    let res = client
        .get(format!("{}?token={}", list_url, n2.unwrap()))
        .send()
        .await
        .map_err(|_| test)?;
    assert_status!(res, test, StatusCode::OK);
    let n2 = validate_quotes(res, test, page3, 3).await?;
    assert_!(test, n2.is_none());

    // TASK 2 DONE
    tx.send((false, 75).into()).await.unwrap();

    Ok(())
}

async fn validate_23(base_url: &str, tx: Sender<SubmissionUpdate>) -> ValidateResult {
    let client = new_client();
    let mut test: TaskTest;
    // TASK 1: serve
    test = (1, 1);
    let url = &format!("{}/assets/23.html", base_url);
    let res = client.get(url).send().await.map_err(|_| test)?;
    assert_status!(res, test, StatusCode::OK);
    if res.text().await.map_err(|_| test)?.len() != 7163 {
        return Err(test);
    }
    // TASK 1 DONE
    tx.send((false, 0).into()).await.unwrap();
    tx.send(SubmissionUpdate::Save).await.unwrap();

    let comparer = HtmlComparer::with_options(HtmlCompareOptions {
        ignore_whitespace: true,
        ignore_attributes: false,
        ignored_attributes: Default::default(),
        ignore_text: false,
        ignore_comments: true,
        ignore_sibling_order: false,
        ignore_style_contents: false,
    });
    macro_rules! assert_html {
        ($res:expr, $test:expr, $comp:expr, $expected_html:expr) => {
            if !$comp
                .compare($expected_html, &$res.text().await.map_err(|_| $test)?)
                .is_ok_and(|t| t)
            {
                return Err($test);
            }
        };
    }
    // TASK 2: star
    test = (2, 1);
    let url = &format!("{}/23/star", base_url);
    let res = client.get(url).send().await.map_err(|_| test)?;
    assert_status!(res, test, StatusCode::OK);
    assert_html!(res, test, comparer, r#"<div id="star" class="lit"></div>"#);
    // TASK 2 DONE
    tx.send((false, 0).into()).await.unwrap();
    tx.send(SubmissionUpdate::Save).await.unwrap();

    // TASK 3: present
    test = (3, 1);
    let res = client
        .get(format!("{}/23/present/red", base_url))
        .send()
        .await
        .map_err(|_| test)?;
    assert_status!(res, test, StatusCode::OK);
    assert_html!(
        res,
        test,
        comparer,
        r#"<div class="present red" hx-get="/23/present/blue" hx-swap="outerHTML"><div class="ribbon"></div><div class="ribbon"></div><div class="ribbon"></div><div class="ribbon"></div></div>"#
    );
    let res = client
        .get(format!("{}/23/present/blue", base_url))
        .send()
        .await
        .map_err(|_| test)?;
    assert_status!(res, test, StatusCode::OK);
    assert_html!(
        res,
        test,
        comparer,
        r#"<div class="present blue" hx-get="/23/present/purple" hx-swap="outerHTML"><div class="ribbon"></div><div class="ribbon"></div><div class="ribbon"></div><div class="ribbon"></div></div>"#
    );
    let res = client
        .get(format!("{}/23/present/purple", base_url))
        .send()
        .await
        .map_err(|_| test)?;
    assert_status!(res, test, StatusCode::OK);
    assert_html!(
        res,
        test,
        comparer,
        r#"<div class="present purple" hx-get="/23/present/red" hx-swap="outerHTML"><div class="ribbon"></div><div class="ribbon"></div><div class="ribbon"></div><div class="ribbon"></div></div>"#
    );
    test = (3, 2);
    let res = client
        .get(format!("{}/23/present/green", base_url))
        .send()
        .await
        .map_err(|_| test)?;
    assert_status!(res, test, StatusCode::IM_A_TEAPOT);
    // TASK 3 DONE
    tx.send((false, 0).into()).await.unwrap();
    tx.send(SubmissionUpdate::Save).await.unwrap();

    // TASK 4: ornament
    test = (4, 1);
    let res = client
        .get(format!("{}/23/ornament/on/1", base_url))
        .send()
        .await
        .map_err(|_| test)?;
    assert_status!(res, test, StatusCode::OK);
    assert_html!(
        res,
        test,
        comparer,
        r#"<div class="ornament on" id="ornament1" hx-trigger="load delay:2s once" hx-get="/23/ornament/off/1" hx-swap="outerHTML"></div>"#
    );
    let res = client
        .get(format!("{}/23/ornament/off/1", base_url))
        .send()
        .await
        .map_err(|_| test)?;
    assert_status!(res, test, StatusCode::OK);
    assert_html!(
        res,
        test,
        comparer,
        r#"<div class="ornament" id="ornament1" hx-trigger="load delay:2s once" hx-get="/23/ornament/on/1" hx-swap="outerHTML"></div>"#
    );
    let res = client
        .get(format!("{}/23/ornament/off/100", base_url))
        .send()
        .await
        .map_err(|_| test)?;
    assert_status!(res, test, StatusCode::OK);
    assert_html!(
        res,
        test,
        comparer,
        r#"<div class="ornament" id="ornament100" hx-trigger="load delay:2s once" hx-get="/23/ornament/on/100" hx-swap="outerHTML"></div>"#
    );
    test = (4, 2);
    let res = client
        .get(format!("{}/23/ornament/on/the_prettiest_one", base_url))
        .send()
        .await
        .map_err(|_| test)?;
    assert_status!(res, test, StatusCode::OK);
    assert_html!(
        res,
        test,
        comparer,
        r#"<div class="ornament on" id="ornamentthe_prettiest_one" hx-trigger="load delay:2s once" hx-get="/23/ornament/off/the_prettiest_one" hx-swap="outerHTML"></div>"#
    );
    test = (4, 3);
    let res = client
        .get(format!("{}/23/ornament/maybe-on/1", base_url))
        .send()
        .await
        .map_err(|_| test)?;
    assert_status!(res, test, StatusCode::IM_A_TEAPOT);
    // TASK 4 DONE
    tx.send((false, 0).into()).await.unwrap();
    tx.send(SubmissionUpdate::Save).await.unwrap();

    // TASK 5: injection
    test = (5, 1);
    let res = client
        .get(format!(
            "{}/23/ornament/on/%22%3E%3Cscript%3Ealert%28%22Spicy%20soup%21%22%29%3C%2Fscript%3E",
            base_url
        ))
        .send()
        .await
        .map_err(|_| test)?;
    assert_status!(res, test, StatusCode::OK);
    assert_html!(
        res,
        test,
        comparer,
        r#"<div class="ornament on" id="ornament&quot;&gt;&lt;script&gt;alert(&quot;Spicy soup!&quot;)&lt;/script&gt;" hx-trigger="load delay:2s once" hx-get="/23/ornament/off/&quot;&gt;&lt;script&gt;alert(&quot;Spicy soup!&quot;)&lt;/script&gt;" hx-swap="outerHTML"></div>"#
    );
    // TASK 5 DONE
    tx.send((true, 0).into()).await.unwrap();
    tx.send(SubmissionUpdate::Save).await.unwrap();

    // TASK 6: lockfile
    test = (6, 1);
    let url = &format!("{}/23/lockfile", base_url);
    let form = Form::new().part(
        "lockfile",
        Part::bytes(
            r#"[[package]]
name = "shuttle-runtime"
version = "0.49.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "337789faa0372648a8ac286b2f92a53121fe118f12e29009ac504872a5413cc6"

[[package]]
name = "shuttle-service"
version = "0.49.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "22ba454b13e4e29b5b892a62c334360a571de5a25c936283416c94328427dd57"
"#
            .as_bytes(),
        )
        .file_name("Cargo.lock")
        .mime_str("application/octet-stream")
        .unwrap(),
    );
    let res = client
        .post(url)
        .multipart(form)
        .send()
        .await
        .map_err(|_| test)?;
    assert_status!(res, test, StatusCode::OK);
    assert_html!(
        res,
        test,
        comparer,
        r#"
<div style="background-color:#337789;top:250px;left:160px;"></div>
<div style="background-color:#22ba45;top:75px;left:19px;"></div>
"#
    );
    test = (6, 2);
    let form = Form::new().part(
        "lockfile",
        Part::bytes(
            r#"# This file is automatically @generated by Cargo.
# It is not intended for manual editing.
version = 4

[[package]]
name = "addr2line"
version = "0.24.2"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "dfbe277e56a376000877090da837660b4427aad530e3028d44e0bffe4f89a1c1"
dependencies = [
 "gimli",
]

[[package]]
name = "adler2"
version = "2.0.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "512761e0bb2578dd7380c6baaa0f4ce03e84f95e960231d1dec8bf4d7d6e2627"

[[package]]
name = "ahash"
version = "0.8.11"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "e89da841a80418a9b391ebaea17f5c112ffaaa96f621d2c285b5174da76b9011"
dependencies = [
 "cfg-if",
 "once_cell",
 "version_check",
 "zerocopy",
]

[[package]]
name = "aho-corasick"
version = "1.1.3"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "8e60d3430d3a69478ad0993f19238d2df97c507009a52b3c10addcd7f6bcb916"
dependencies = [
 "memchr",
]

[[package]]
name = "allocator-api2"
version = "0.2.21"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "683d7910e743518b0e34f1186f92494becacb047c7b6bf616c96772180fef923"

[[package]]
name = "android-tzdata"
version = "0.1.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "e999941b234f3131b00bc13c22d06e8c5ff726d1b6318ac7eb276997bbb4fef0"

[[package]]
name = "android_system_properties"
version = "0.1.5"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "819e7219dbd41043ac279b19830f2efc897156490d7fd6ea916720117ee66311"
dependencies = [
 "libc",
]

[[package]]
name = "anyhow"
version = "1.0.93"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "4c95c10ba0b00a02636238b814946408b1322d5ac4760326e6fb8ec956d85775"

[[package]]
name = "askama"
version = "0.12.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "b79091df18a97caea757e28cd2d5fda49c6cd4bd01ddffd7ff01ace0c0ad2c28"
dependencies = [
 "askama_derive",
 "askama_escape",
 "humansize",
 "num-traits",
 "percent-encoding",
]

[[package]]
name = "askama_axum"
version = "0.4.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "a41603f7cdbf5ac4af60760f17253eb6adf6ec5b6f14a7ed830cf687d375f163"
dependencies = [
 "askama",
 "axum-core 0.4.5",
 "http 1.1.0",
]

[[package]]
name = "askama_derive"
version = "0.12.5"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "19fe8d6cb13c4714962c072ea496f3392015f0989b1a2847bb4b2d9effd71d83"
dependencies = [
 "askama_parser",
 "basic-toml",
 "mime",
 "mime_guess",
 "proc-macro2",
 "quote",
 "serde",
 "syn 2.0.89",
]

[[package]]
name = "askama_escape"
version = "0.10.3"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "619743e34b5ba4e9703bba34deac3427c72507c7159f5fd030aea8cac0cfe341"

[[package]]
name = "askama_parser"
version = "0.2.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "acb1161c6b64d1c3d83108213c2a2533a342ac225aabd0bda218278c2ddb00c0"
dependencies = [
 "nom",
]

[[package]]
name = "async-stream"
version = "0.3.6"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "0b5a71a6f37880a80d1d7f19efd781e4b5de42c88f0722cc13bcb6cc2cfe8476"
dependencies = [
 "async-stream-impl",
 "futures-core",
 "pin-project-lite",
]

[[package]]
name = "async-stream-impl"
version = "0.3.6"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "c7c24de15d275a1ecfd47a380fb4d5ec9bfe0933f309ed5e705b775596a3574d"
dependencies = [
 "proc-macro2",
 "quote",
 "syn 2.0.89",
]

[[package]]
name = "async-trait"
version = "0.1.83"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "721cae7de5c34fbb2acd27e21e6d2cf7b886dce0c27388d46c4e6c47ea4318dd"
dependencies = [
 "proc-macro2",
 "quote",
 "syn 2.0.89",
]

[[package]]
name = "atoi"
version = "2.0.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "f28d99ec8bfea296261ca1af174f24225171fea9664ba9003cbebee704810528"
dependencies = [
 "num-traits",
]

[[package]]
name = "autocfg"
version = "1.4.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "ace50bade8e6234aa140d9a2f552bbee1db4d353f69b8217bc503490fc1a9f26"

[[package]]
name = "axum"
version = "0.6.20"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "3b829e4e32b91e643de6eafe82b1d90675f5874230191a4ffbc1b336dec4d6bf"
dependencies = [
 "async-trait",
 "axum-core 0.3.4",
 "bitflags 1.3.2",
 "bytes",
 "futures-util",
 "http 0.2.12",
 "http-body 0.4.6",
 "hyper 0.14.31",
 "itoa",
 "matchit",
 "memchr",
 "mime",
 "percent-encoding",
 "pin-project-lite",
 "rustversion",
 "serde",
 "sync_wrapper 0.1.2",
 "tower 0.4.13",
 "tower-layer",
 "tower-service",
]

[[package]]
name = "axum"
version = "0.7.9"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "edca88bc138befd0323b20752846e6587272d3b03b0343c8ea28a6f819e6e71f"
dependencies = [
 "async-trait",
 "axum-core 0.4.5",
 "bytes",
 "futures-util",
 "http 1.1.0",
 "http-body 1.0.1",
 "http-body-util",
 "hyper 1.5.1",
 "hyper-util",
 "itoa",
 "matchit",
 "memchr",
 "mime",
 "percent-encoding",
 "pin-project-lite",
 "rustversion",
 "serde",
 "serde_json",
 "serde_path_to_error",
 "serde_urlencoded",
 "sync_wrapper 1.0.2",
 "tokio",
 "tower 0.5.1",
 "tower-layer",
 "tower-service",
 "tracing",
]

[[package]]
name = "axum-core"
version = "0.3.4"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "759fa577a247914fd3f7f76d62972792636412fbfd634cd452f6a385a74d2d2c"
dependencies = [
 "async-trait",
 "bytes",
 "futures-util",
 "http 0.2.12",
 "http-body 0.4.6",
 "mime",
 "rustversion",
 "tower-layer",
 "tower-service",
]

[[package]]
name = "axum-core"
version = "0.4.5"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "09f2bd6146b97ae3359fa0cc6d6b376d9539582c7b4220f041a33ec24c226199"
dependencies = [
 "async-trait",
 "bytes",
 "futures-util",
 "http 1.1.0",
 "http-body 1.0.1",
 "http-body-util",
 "mime",
 "pin-project-lite",
 "rustversion",
 "sync_wrapper 1.0.2",
 "tower-layer",
 "tower-service",
 "tracing",
]

[[package]]
name = "axum-extra"
version = "0.9.6"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "c794b30c904f0a1c2fb7740f7df7f7972dfaa14ef6f57cb6178dc63e5dca2f04"
dependencies = [
 "axum 0.7.9",
 "axum-core 0.4.5",
 "bytes",
 "cookie",
 "fastrand",
 "futures-util",
 "http 1.1.0",
 "http-body 1.0.1",
 "http-body-util",
 "mime",
 "multer",
 "pin-project-lite",
 "serde",
 "tower 0.5.1",
 "tower-layer",
 "tower-service",
]

[[package]]
name = "backtrace"
version = "0.3.74"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "8d82cb332cdfaed17ae235a638438ac4d4839913cc2af585c3c6746e8f8bee1a"
dependencies = [
 "addr2line",
 "cfg-if",
 "libc",
 "miniz_oxide",
 "object",
 "rustc-demangle",
 "windows-targets 0.52.6",
]

[[package]]
name = "base64"
version = "0.21.7"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "9d297deb1925b89f2ccc13d7635fa0714f12c87adce1c75356b39ca9b7178567"

[[package]]
name = "base64"
version = "0.22.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "72b3254f16251a8381aa12e40e3c4d2f0199f8c6508fbecb9d91f575e0fbb8c6"

[[package]]
name = "base64ct"
version = "1.6.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "8c3c1a368f70d6cf7302d78f8f7093da241fb8e8807c05cc9e51a125895a6d5b"

[[package]]
name = "basic-toml"
version = "0.1.9"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "823388e228f614e9558c6804262db37960ec8821856535f5c3f59913140558f8"
dependencies = [
 "serde",
]

[[package]]
name = "bitflags"
version = "1.3.2"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "bef38d45163c2f1dde094a7dfd33ccf595c92905c8f8f4fdc18d06fb1037718a"

[[package]]
name = "bitflags"
version = "2.6.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "b048fb63fd8b5923fc5aa7b340d8e156aec7ec02f0c78fa8a6ddc2613f6f71de"
dependencies = [
 "serde",
]

[[package]]
name = "block-buffer"
version = "0.10.4"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "3078c7629b62d3f0439517fa394996acacc5cbc91c5a20d8c658e77abd503a71"
dependencies = [
 "generic-array",
]

[[package]]
name = "bumpalo"
version = "3.16.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "79296716171880943b8470b5f8d03aa55eb2e645a4874bdbb28adb49162e012c"

[[package]]
name = "byteorder"
version = "1.5.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "1fd0f2584146f6f2ef48085050886acf353beff7305ebd1ae69500e27c67f64b"

[[package]]
name = "bytes"
version = "1.8.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "9ac0150caa2ae65ca5bd83f25c7de183dea78d4d366469f148435e2acfbad0da"

[[package]]
name = "shuttlings-cch24"
version = "0.1.0"
dependencies = [
 "askama",
 "askama_axum",
 "axum 0.7.9",
 "axum-extra",
 "cargo-manifest",
 "chrono",
 "ipnet",
 "jsonwebtoken",
 "leaky-bucket",
 "rand",
 "serde",
 "serde_json",
 "serde_yml",
 "shuttle-axum",
 "shuttle-runtime",
 "shuttle-shared-db",
 "sqlx",
 "tokio",
 "toml",
 "tower-http",
 "uuid",
]

[[package]]
name = "signal-hook"
version = "0.3.17"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "8621587d4798caf8eb44879d42e56b9a93ea5dcd315a6487c357130095b62801"
dependencies = [
 "libc",
 "signal-hook-registry",
]
"#
            .as_bytes(),
        )
        .file_name("Cargo.lock")
        .mime_str("application/octet-stream")
        .unwrap(),
    );
    let res = client
        .post(url)
        .multipart(form)
        .send()
        .await
        .map_err(|_| test)?;
    assert_status!(res, test, StatusCode::OK);
    assert_html!(
        res,
        test,
        comparer,
        r#"
<div style="background-color:#dfbe27;top:126px;left:86px;"></div>
<div style="background-color:#512761;top:224px;left:187px;"></div>
<div style="background-color:#e89da8;top:65px;left:168px;"></div>
<div style="background-color:#8e60d3;top:67px;left:13px;"></div>
<div style="background-color:#683d79;top:16px;left:231px;"></div>
<div style="background-color:#e99994;top:27px;left:35px;"></div>
<div style="background-color:#819e72;top:25px;left:219px;"></div>
<div style="background-color:#4c95c1;top:11px;left:160px;"></div>
<div style="background-color:#b79091;top:223px;left:24px;"></div>
<div style="background-color:#a41603;top:247px;left:205px;"></div>
<div style="background-color:#19fe8d;top:108px;left:177px;"></div>
<div style="background-color:#619743;top:227px;left:75px;"></div>
<div style="background-color:#acb116;top:28px;left:107px;"></div>
<div style="background-color:#0b5a71;top:166px;left:243px;"></div>
<div style="background-color:#c7c24d;top:225px;left:93px;"></div>
<div style="background-color:#721cae;top:125px;left:229px;"></div>
<div style="background-color:#f28d99;top:236px;left:139px;"></div>
<div style="background-color:#ace50b;top:173px;left:232px;"></div>
<div style="background-color:#3b829e;top:78px;left:50px;"></div>
<div style="background-color:#edca88;top:188px;left:19px;"></div>
<div style="background-color:#759fa5;top:119px;left:162px;"></div>
<div style="background-color:#09f2bd;top:97px;left:70px;"></div>
<div style="background-color:#c794b3;top:12px;left:144px;"></div>
<div style="background-color:#8d82cb;top:51px;left:44px;"></div>
<div style="background-color:#9d297d;top:235px;left:25px;"></div>
<div style="background-color:#72b325;top:79px;left:22px;"></div>
<div style="background-color:#8c3c1a;top:54px;left:143px;"></div>
<div style="background-color:#823388;top:226px;left:40px;"></div>
<div style="background-color:#bef38d;top:69px;left:22px;"></div>
<div style="background-color:#b048fb;top:99px;left:253px;"></div>
<div style="background-color:#3078c7;top:98px;left:155px;"></div>
<div style="background-color:#792967;top:22px;left:23px;"></div>
<div style="background-color:#1fd0f2;top:88px;left:65px;"></div>
<div style="background-color:#9ac015;top:12px;left:170px;"></div>
<div style="background-color:#862158;top:125px;left:71px;"></div>
"#
    );
    test = (6, 3);
    let form = Form::new().part(
        "blockfile",
        Part::bytes(r#"MINE DIAMONDS!!!!"#.as_bytes())
            .file_name("Cargo.block")
            .mime_str("application/octet-stream")
            .unwrap(),
    );
    let res = client
        .post(url)
        .multipart(form)
        .send()
        .await
        .map_err(|_| test)?;
    assert_status!(res, test, StatusCode::BAD_REQUEST);
    test = (6, 4);
    let form = Form::new();
    let res = client
        .post(url)
        .multipart(form)
        .send()
        .await
        .map_err(|_| test)?;
    assert_status!(res, test, StatusCode::BAD_REQUEST);
    test = (6, 5);
    let form = Form::new().part(
        "lockfile",
        Part::bytes(
            "[[package]]
checksum = \"337789faa0372648a8ac286b2f92a53121fe118f12e29009ac504872a5413cc6\"
\x00\x00"
                .as_bytes(),
        )
        .file_name("Cargo.lock")
        .mime_str("application/octet-stream")
        .unwrap(),
    );
    let res = client
        .post(url)
        .multipart(form)
        .send()
        .await
        .map_err(|_| test)?;
    assert_status!(res, test, StatusCode::BAD_REQUEST);
    test = (6, 5);
    let form = Form::new().part(
        "lockfile",
        Part::bytes(
            r#"[[package]]
checksum = "337789faa0372648a8ac286b2f92a53121fe118f12e29009ac504872a5413cc6"
fn jingle_bells(volume: f32) -> Result<Sound<DingDong>, MusicError> { ... }
"#
            .as_bytes(),
        )
        .file_name("Cargo.lock")
        .mime_str("application/octet-stream")
        .unwrap(),
    );
    let res = client
        .post(url)
        .multipart(form)
        .send()
        .await
        .map_err(|_| test)?;
    assert_status!(res, test, StatusCode::BAD_REQUEST);
    test = (6, 6);
    let form = Form::new().part(
        "lockfile",
        Part::bytes(
            r#"[[package]]
checksum = "337789faa0372648a8ac286b2f92a53121fe118f12e29009ac504872a5413cc6"
"#
            .as_bytes(),
        )
        .file_name("Cargo.lock")
        .mime_str("application/octet-stream")
        .unwrap(),
    );
    let res = client
        .post(url)
        .multipart(form)
        .send()
        .await
        .map_err(|_| test)?;
    assert_status!(res, test, StatusCode::OK);
    assert_html!(
        res,
        test,
        comparer,
        r#"
<div style="background-color:#337789;top:250px;left:160px;"></div>
"#
    );
    test = (6, 7);
    let form = Form::new().part(
        "lockfile",
        Part::bytes(
            r#"[[package]]
checksum = [ "cookie", "milk", "hot cocoa" ]
"#
            .as_bytes(),
        )
        .file_name("Cargo.lock")
        .mime_str("application/octet-stream")
        .unwrap(),
    );
    let res = client
        .post(url)
        .multipart(form)
        .send()
        .await
        .map_err(|_| test)?;
    assert_status!(res, test, StatusCode::BAD_REQUEST);
    test = (6, 8);
    let form = Form::new().part(
        "lockfile",
        Part::bytes(
            r#"[[package]]
checksum = "337789faa0"
"#
            .as_bytes(),
        )
        .file_name("Cargo.lock")
        .mime_str("application/octet-stream")
        .unwrap(),
    );
    let res = client
        .post(url)
        .multipart(form)
        .send()
        .await
        .map_err(|_| test)?;
    assert_status!(res, test, StatusCode::OK);
    assert_html!(
        res,
        test,
        comparer,
        r#"
<div style="background-color:#337789;top:250px;left:160px;"></div>
"#
    );
    test = (6, 9);
    let form = Form::new().part(
        "lockfile",
        Part::bytes(
            r#"[[package]]
checksum = "337789faa0"
"#
            .as_bytes(),
        )
        .file_name("Cargo.lock")
        .mime_str("application/octet-stream")
        .unwrap(),
    );
    let res = client
        .post(url)
        .multipart(form)
        .send()
        .await
        .map_err(|_| test)?;
    assert_status!(res, test, StatusCode::OK);
    assert_html!(
        res,
        test,
        comparer,
        r#"
<div style="background-color:#337789;top:250px;left:160px;"></div>
"#
    );
    test = (6, 10);
    let form = Form::new().part(
        "lockfile",
        Part::bytes(
            r#"[[package]]
checksum = "337789FAA0"
"#
            .as_bytes(),
        )
        .file_name("Cargo.lock")
        .mime_str("application/octet-stream")
        .unwrap(),
    );
    let res = client
        .post(url)
        .multipart(form)
        .send()
        .await
        .map_err(|_| test)?;
    assert_status!(res, test, StatusCode::OK);
    assert_html!(
        res,
        test,
        comparer,
        r#"
<div style="background-color:#337789;top:250px;left:160px;"></div>
"#
    );
    test = (6, 11);
    let form = Form::new().part(
        "lockfile",
        Part::bytes(
            r#"[[package]]
checksum = "3377QQFAA0"
"#
            .as_bytes(),
        )
        .file_name("Cargo.lock")
        .mime_str("application/octet-stream")
        .unwrap(),
    );
    let res = client
        .post(url)
        .multipart(form)
        .send()
        .await
        .map_err(|_| test)?;
    assert_status!(res, test, StatusCode::UNPROCESSABLE_ENTITY);
    test = (6, 12);
    let form = Form::new().part(
        "lockfile",
        Part::bytes(
            r#"[[package]]
checksum = "BEEF"
"#
            .as_bytes(),
        )
        .file_name("Cargo.lock")
        .mime_str("application/octet-stream")
        .unwrap(),
    );
    let res = client
        .post(url)
        .multipart(form)
        .send()
        .await
        .map_err(|_| test)?;
    assert_status!(res, test, StatusCode::UNPROCESSABLE_ENTITY);

    // TASK 6 DONE
    tx.send((false, 100).into()).await.unwrap();
    tx.send(SubmissionUpdate::Save).await.unwrap();

    Ok(())
}
