pub mod args;

use std::{ops::Deref, sync::Arc};

use base64::{engine::general_purpose, Engine};
use futures_util::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use reqwest::{
    header::{HeaderValue, CONTENT_TYPE},
    multipart::{Form, Part},
    redirect::Policy,
    StatusCode,
};
pub use shuttlings;
use shuttlings::{SubmissionState, SubmissionUpdate};
use tokio::{
    net::TcpStream,
    sync::mpsc::Sender,
    time::{sleep, Duration},
};
use tokio_tungstenite::{tungstenite::Message, MaybeTlsStream, WebSocketStream};
use tracing::info;
use uuid::Uuid;

pub const SUPPORTED_CHALLENGES: &[i32] =
    &[-1, 1, 4, 5, 6, 7, 8, 11, 12, 13, 14, 15, 18, 19, 20, 21, 22];
pub const SUBMISSION_TIMEOUT: u64 = 60;

pub async fn run(url: String, id: Uuid, number: i32, tx: Sender<SubmissionUpdate>) {
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

pub async fn validate(url: &str, number: i32, tx: Sender<SubmissionUpdate>) {
    if !SUPPORTED_CHALLENGES.contains(&number) {
        tx.send(
            format!("Validating Challenge {number} is not supported yet! Check for updates.")
                .into(),
        )
        .await
        .unwrap();
        return;
    }
    let txc = tx.clone();
    if let Err((task, test)) = match number {
        -1 => validate_minus1(url, txc).await,
        1 => validate_1(url, txc).await,
        4 => validate_4(url, txc).await,
        5 => validate_5(url, txc).await,
        6 => validate_6(url, txc).await,
        7 => validate_7(url, txc).await,
        8 => validate_8(url, txc).await,
        11 => validate_11(url, txc).await,
        12 => validate_12(url, txc).await,
        13 => validate_13(url, txc).await,
        14 => validate_14(url, txc).await,
        15 => validate_15(url, txc).await,
        18 => validate_18(url, txc).await,
        19 => validate_19(url, txc).await,
        20 => validate_20(url, txc).await,
        21 => validate_21(url, txc).await,
        22 => validate_22(url, txc).await,
        _ => unreachable!(),
    } {
        info!(%url, %number, %task, %test, "Submission failed");
        tx.send(format!("Task {task}: test #{test} failed 🟥").into())
            .await
            .unwrap();
    }
    tx.send(SubmissionState::Done.into()).await.unwrap();
    tx.send(SubmissionUpdate::Save).await.unwrap();
}

fn new_client() -> reqwest::Client {
    reqwest::ClientBuilder::new()
        .http1_only()
        .connect_timeout(Duration::from_secs(3))
        .redirect(Policy::limited(3))
        .referer(false)
        .timeout(Duration::from_secs(60))
        .build()
        .unwrap()
}

async fn validate_minus1(base_url: &str, tx: Sender<SubmissionUpdate>) -> ValidateResult {
    let client = new_client();
    let mut test: TaskTest;
    // TASK 1: respond 200
    test = (1, 1);
    let url = &format!("{}/", base_url);
    let res = client.get(url).send().await.map_err(|_| test)?;
    if res.status() != StatusCode::OK {
        return Err(test);
    }
    // TASK 1 DONE
    tx.send((true, 0).into()).await.unwrap();
    tx.send(SubmissionUpdate::Save).await.unwrap();

    // TASK 2: respond 500
    test = (2, 1);
    let url = &format!("{}/-1/error", base_url);
    let res = client.get(url).send().await.map_err(|_| test)?;
    if res.status() != StatusCode::INTERNAL_SERVER_ERROR {
        return Err(test);
    }
    // TASK 2 DONE
    tx.send((false, 0).into()).await.unwrap();

    Ok(())
}

async fn validate_1(base_url: &str, tx: Sender<SubmissionUpdate>) -> ValidateResult {
    let client = new_client();
    let mut test: TaskTest;
    // TASK 1: basic formula
    test = (1, 1);
    let url = &format!("{}/1/2/3", base_url);
    let res = client.get(url).send().await.map_err(|_| test)?;
    let text = res.text().await.map_err(|_| test)?;
    if text != "1" {
        return Err(test);
    }
    test = (1, 2);
    let url = &format!("{}/1/12/16", base_url);
    let res = client.get(url).send().await.map_err(|_| test)?;
    let text = res.text().await.map_err(|_| test)?;
    if text != "21952" {
        return Err(test);
    }
    // TASK 1 DONE
    tx.send((true, 0).into()).await.unwrap();
    tx.send(SubmissionUpdate::Save).await.unwrap();

    // TASK 2: multiple and zero and negative numbers
    test = (2, 1);
    let url = &format!("{}/1/3/5/7/9", base_url);
    let res = client.get(url).send().await.map_err(|_| test)?;
    let text = res.text().await.map_err(|_| test)?;
    if text != "512" {
        return Err(test);
    }
    test = (2, 2);
    let url = &format!("{}/1/0/0/0", base_url);
    let res = client.get(url).send().await.map_err(|_| test)?;
    let text = res.text().await.map_err(|_| test)?;
    if text != "0" {
        return Err(test);
    }
    test = (2, 3);
    let url = &format!("{}/1/-3/1", base_url);
    let res = client.get(url).send().await.map_err(|_| test)?;
    let text = res.text().await.map_err(|_| test)?;
    if text != "-64" {
        return Err(test);
    }
    test = (2, 4);
    let url = &format!("{}/1/3/5/7/9/2/13/12/16/18", base_url);
    let res = client.get(url).send().await.map_err(|_| test)?;
    let text = res.text().await.map_err(|_| test)?;
    if text != "729" {
        return Err(test);
    }
    tx.send((false, 100).into()).await.unwrap();

    Ok(())
}

async fn validate_4(base_url: &str, tx: Sender<SubmissionUpdate>) -> ValidateResult {
    let client = new_client();
    let mut test: TaskTest;
    // TASK 1
    test = (1, 1);
    let url = &format!("{}/4/strength", base_url);
    let res = client
        .post(url)
        .json(&serde_json::json!([
            {
              "name": "Zeus",
              "strength": 8
            },
            {
              "name": "Oner",
              "strength": 6
            },
            {
              "name": "Faker",
              "strength": 7
            },
            {
              "name": "Gumayusi",
              "strength": 6
            },
            {
              "name": "Keria",
              "strength": 6
            }
        ]))
        .send()
        .await
        .map_err(|_| test)?;
    let text = res.text().await.map_err(|_| test)?;
    if text != "33" {
        return Err(test);
    }
    // TASK 1 DONE
    tx.send((true, 0).into()).await.unwrap();
    tx.send(SubmissionUpdate::Save).await.unwrap();

    // TASK 2
    test = (2, 1);
    let url = &format!("{}/4/contest", base_url);
    let res = client
        .post(url)
        .json(&serde_json::json!([
        {
            "name": "Zeus",
            "strength": 8,
            "speed": 51.2,
            "height": 81,
            "antler_width": 31,
            "snow_magic_power": 311,
            "favorite_food": "pizza",
            "cAnD13s_3ATeN-yesT3rdAy": 4
        },
        {
            "name": "Oner",
            "strength": 6,
            "speed": 41.3,
            "height": 51,
            "antler_width": 30,
            "snow_magic_power": 321,
            "favorite_food": "burger",
            "cAnD13s_3ATeN-yesT3rdAy": 1
        },
        {
            "name": "Faker",
            "strength": 7,
            "speed": 50,
            "height": 50,
            "antler_width": 37,
            "snow_magic_power": 6667,
            "favorite_food": "broccoli",
            "cAnD13s_3ATeN-yesT3rdAy": 1
        },
        {
            "name": "Gumayusi",
            "strength": 6,
            "speed": 60.1,
            "height": 50,
            "antler_width": 34,
            "snow_magic_power": 2323,
            "favorite_food": "pizza",
            "cAnD13s_3ATeN-yesT3rdAy": 1
        },
        {
            "name": "Keria",
            "strength": 6,
            "speed": 48.2,
            "height": 65,
            "antler_width": 33,
            "snow_magic_power": 5014,
            "favorite_food": "wok",
            "cAnD13s_3ATeN-yesT3rdAy": 5
        }
        ]))
        .send()
        .await
        .map_err(|_| test)?;
    let json = res.json::<serde_json::Value>().await.map_err(|_| test)?;
    if json
        != serde_json::json!({
            "fastest":"Speeding past the finish line with a strength of 6 is Gumayusi",
            "tallest":"Zeus is standing tall with his 31 cm wide antlers",
            "magician":"Faker could blast you away with a snow magic power of 6667",
            "consumer":"Keria ate lots of candies, but also some wok"
        })
    {
        return Err(test);
    }
    tx.send((false, 150).into()).await.unwrap();

    Ok(())
}

async fn validate_5(base_url: &str, tx: Sender<SubmissionUpdate>) -> ValidateResult {
    // TASK 1
    let t = JSONTester::new(format!("{}/5?offset=0&limit=8", base_url));
    t.test(
        (1, 1),
        &serde_json::json!(["Ava", "Caleb", "Mia", "Owen", "Lily", "Ethan", "Zoe", "Nolan"]),
        StatusCode::OK,
        &serde_json::json!(["Ava", "Caleb", "Mia", "Owen", "Lily", "Ethan", "Zoe", "Nolan"]),
    )
    .await?;
    let t = JSONTester::new(format!("{}/5?offset=10&limit=4", base_url));
    t.test(
        (1, 2),
        &serde_json::json!([
            "Ava", "Caleb", "Mia", "Owen", "Lily", "Ethan", "Zoe", "Nolan", "Harper", "Lucas",
            "Stella", "Mason", "Olivia", "Wyatt", "Isabella", "Logan",
        ]),
        StatusCode::OK,
        &serde_json::json!(["Stella", "Mason", "Olivia", "Wyatt"]),
    )
    .await?;
    // TASK 1 DONE
    tx.send((true, 0).into()).await.unwrap();
    tx.send(SubmissionUpdate::Save).await.unwrap();

    // TASK 2
    let t = JSONTester::new(format!("{}/5?offset=0&limit=5", base_url));
    t.test(
        (2, 1),
        &serde_json::json!([]),
        StatusCode::OK,
        &serde_json::json!([]),
    )
    .await?;
    let t = JSONTester::new(format!("{}/5", base_url));
    t.test(
        (2, 2),
        &serde_json::json!(["Alice", "Bob", "Charlie", "David"]),
        StatusCode::OK,
        &serde_json::json!(["Alice", "Bob", "Charlie", "David"]),
    )
    .await?;
    let t = JSONTester::new(format!("{}/5?offset=2", base_url));
    t.test(
        (2, 3),
        &serde_json::json!(["Alice", "Bob", "Charlie", "David"]),
        StatusCode::OK,
        &serde_json::json!(["Charlie", "David"]),
    )
    .await?;
    let t = JSONTester::new(format!("{}/5?offset=2&limit=0", base_url));
    t.test(
        (2, 4),
        &serde_json::json!(["Alice", "Bob", "Charlie", "David"]),
        StatusCode::OK,
        &serde_json::json!([]),
    )
    .await?;
    let t = JSONTester::new(format!("{}/5?split=6", base_url));
    t.test(
        (2, 5),
        &serde_json::json!([
            "Alice", "Bob", "Charlie", "David", "Eva", "Frank", "Grace", "Hank", "Ivy", "Jack",
            "Katie", "Liam", "Mia", "Nathan", "Olivia", "Paul", "Quinn", "Rachel", "Samuel",
            "Tara", "Aria", "Jackson"
        ]),
        StatusCode::OK,
        &serde_json::json!([
            ["Alice", "Bob", "Charlie", "David", "Eva", "Frank"],
            ["Grace", "Hank", "Ivy", "Jack", "Katie", "Liam"],
            ["Mia", "Nathan", "Olivia", "Paul", "Quinn", "Rachel"],
            ["Samuel", "Tara", "Aria", "Jackson"]
        ]),
    )
    .await?;
    let t = JSONTester::new(format!("{}/5?offset=2&limit=4&split=1", base_url));
    t.test(
        (2, 6),
        &serde_json::json!([
            "Alice", "Bob", "Charlie", "David", "Alice", "Bob", "Charlie", "David"
        ]),
        StatusCode::OK,
        &serde_json::json!([["Charlie"], ["David"], ["Alice"], ["Bob"],]),
    )
    .await?;
    let t = JSONTester::new(format!("{}/5?limit=0", base_url));
    t.test(
        (2, 7),
        &serde_json::json!(["Alice", "Bob", "Charlie", "David"]),
        StatusCode::OK,
        &serde_json::json!([]),
    )
    .await?;
    let t = JSONTester::new(format!("{}/5?offset=0&limit=0", base_url));
    t.test(
        (2, 8),
        &serde_json::json!(["Alice", "Bob", "Charlie", "David"]),
        StatusCode::OK,
        &serde_json::json!([]),
    )
    .await?;
    tx.send((false, 150).into()).await.unwrap();

    Ok(())
}

async fn validate_6(base_url: &str, tx: Sender<SubmissionUpdate>) -> ValidateResult {
    let client = new_client();
    let mut test: TaskTest;
    let url = &format!("{}/6", base_url);
    // TASK 1: elf
    test = (1, 1);
    let res = client
        .post(url)
        .body("elf elf elf")
        .send()
        .await
        .map_err(|_| test)?;
    let json = res.json::<serde_json::Value>().await.map_err(|_| test)?;
    if json["elf"] != serde_json::Value::Number(3.into()) {
        return Err(test);
    }
    test = (1, 2);
    let res = client
        .post(url)
        .body("In the quirky town of Elf stood an enchanting shop named 'The Elf & Shelf.' Managed by Wally, a mischievous elf with a knack for crafting exquisite shelves, the shop was a bustling hub of elf after elf who wanter to see their dear elf in Belfast.")
        .send()
        .await
        .map_err(|_| test)?;
    let json = res.json::<serde_json::Value>().await.map_err(|_| test)?;
    if json["elf"] != serde_json::Value::Number(6.into()) {
        return Err(test);
    }
    // TASK 1 DONE
    tx.send((true, 0).into()).await.unwrap();
    tx.send(SubmissionUpdate::Save).await.unwrap();

    // TASK 2: more strings
    test = (2, 1);
    let res = client
        .post(url)
        .body("elf elf elf on a shelf")
        .send()
        .await
        .map_err(|_| test)?;
    let json = res.json::<serde_json::Value>().await.map_err(|_| test)?;
    if json
        != serde_json::json!({
            "elf":4,
            "elf on a shelf":1,
            "shelf with no elf on it":0
        })
    {
        return Err(test);
    }
    test = (2, 2);
    let res = client
        .post(url)
        .body("In Belfast I heard an elf on a shelf on a shelf on a ")
        .send()
        .await
        .map_err(|_| test)?;
    let json = res.json::<serde_json::Value>().await.map_err(|_| test)?;
    if json
        != serde_json::json!({
            "elf":4,
            "elf on a shelf":2,
            "shelf with no elf on it":0
        })
    {
        return Err(test);
    }
    test = (2, 3);
    let res = client
        .post(url)
        .body("Somewhere in Belfast under a shelf store but above the shelf realm there's an elf on a shelf on a shelf on a shelf on a elf on a shelf on a shelf on a shelf on a shelf on a elf on a elf on a elf on a shelf on a ")
        .send()
        .await
        .map_err(|_| test)?;
    let json = res.json::<serde_json::Value>().await.map_err(|_| test)?;
    if json
        != serde_json::json!({
            "elf":16,
            "elf on a shelf":8,
            "shelf with no elf on it":2
        })
    {
        return Err(test);
    }
    // TASK 2 DONE
    tx.send((false, 200).into()).await.unwrap();

    Ok(())
}

async fn validate_7(base_url: &str, tx: Sender<SubmissionUpdate>) -> ValidateResult {
    let client = new_client();
    let mut test: TaskTest;
    // TASK 1
    test = (1, 1);
    let url = &format!("{}/7/decode", base_url);
    let data = serde_json::json!({
        "recipe": {
            "flour": 4,
            "sugar": 3,
            "butter": 3,
            "baking powder": 1,
            "raisins": 50
        },
    });
    let b64 = general_purpose::STANDARD.encode(serde_json::to_vec(&data).unwrap());
    let res = client
        .get(url)
        .header("Cookie", format!("recipe={b64}"))
        .send()
        .await
        .map_err(|_| test)?;
    let json = res.json::<serde_json::Value>().await.map_err(|_| test)?;
    if json != data {
        return Err(test);
    }
    test = (1, 2);
    let data = serde_json::json!({
        "recipe": {
            "peanuts": 26,
            "dough": 37,
            "extra salt": 1,
            "raisins": 50
        },
    });
    let b64 = general_purpose::STANDARD.encode(serde_json::to_vec(&data).unwrap());
    let res = client
        .get(url)
        .header("Cookie", format!("recipe={b64}"))
        .send()
        .await
        .map_err(|_| test)?;
    let json = res.json::<serde_json::Value>().await.map_err(|_| test)?;
    if json != data {
        return Err(test);
    }
    // TASK 1 DONE
    tx.send((true, 0).into()).await.unwrap();
    tx.send(SubmissionUpdate::Save).await.unwrap();

    // TASK 2
    let url = &format!("{}/7/bake", base_url);
    let test_bake = |test: (i32, i32), i: serde_json::Value, o: serde_json::Value| async move {
        let client = new_client();
        let b64 = general_purpose::STANDARD.encode(serde_json::to_vec(&i).unwrap());
        let res = client
            .get(url)
            .header("Cookie", format!("recipe={b64}"))
            .send()
            .await
            .map_err(|_| test)?;
        let json = res.json::<serde_json::Value>().await.map_err(|_| test)?;
        if json != o {
            return Err(test);
        }
        Ok(())
    };
    test = (2, 1);
    test_bake(
        test,
        serde_json::json!({
            "recipe": {
                "flour": 35,
                "sugar": 56,
                "butter": 3,
                "baking powder": 1001,
                "chocolate chips": 55
            },
            "pantry": {
                "flour": 4045,
                "sugar": 9606,
                "butter": 99, // will land at 0
                "baking powder": 8655432,
                "chocolate chips": 4587
            }
        }),
        serde_json::json!({
            "cookies": 33,
            "pantry": {
                "flour": 2890,
                "sugar": 7758,
                "butter": 0,
                "baking powder": 8622399,
                "chocolate chips": 2772
            }
        }),
    )
    .await?;
    test = (2, 2);
    test_bake(
        test,
        serde_json::json!({
            "recipe": {
                "flour": 35,
                "sugar": 56,
                "butter": 3,
                "baking powder": 1001,
                "chocolate chips": 55
            },
            "pantry": {
                "flour": 4045,
                "sugar": 7606,
                "butter": 100,
                "baking powder": 865543211516164409i64,
                "chocolate chips": 4587
            }
        }),
        serde_json::json!({
            "cookies": 33,
            "pantry": {
                "flour": 2890,
                "sugar": 5758,
                "butter": 1,
                "baking powder": 865543211516131376i64,
                "chocolate chips": 2772
            }
        }),
    )
    .await?;
    // TASK 2 DONE
    tx.send((false, 120).into()).await.unwrap();
    tx.send(SubmissionUpdate::Save).await.unwrap();

    // TASK 3
    test = (3, 1);
    test_bake(
        test,
        serde_json::json!({
            "recipe": {
                "chicken": 1,
            },
            "pantry": {
                "chicken": 0,
            }
        }),
        serde_json::json!({
            "cookies": 0,
            "pantry": {
                "chicken": 0,
            }
        }),
    )
    .await?;
    test = (3, 2);
    test_bake(
        test,
        serde_json::json!({
            "recipe": {
                "cocoa bean": 1,
                "chicken": 0,
            },
            "pantry": {
                "cocoa bean": 5,
                "corn": 5,
                "cucumber": 0,
            }
        }),
        serde_json::json!({
            "cookies": 5,
            "pantry": {
                "cocoa bean": 0,
                "corn": 5,
                "cucumber": 0,
            }
        }),
    )
    .await?;
    test = (3, 3);
    test_bake(
        test,
        serde_json::json!({
            "recipe": {
                "cocoa bean": 1,
                "chicken": 0,
            },
            "pantry": {
                "cocoa bean": 5,
                "chicken": 0,
            }
        }),
        serde_json::json!({
            "cookies": 5,
            "pantry": {
                "cocoa bean": 0,
                "chicken": 0,
            }
        }),
    )
    .await?;
    test = (3, 4);
    test_bake(
        test,
        serde_json::json!({
            "recipe": {
                "cocoa bean": 1,
                "chicken": 0,
            },
            "pantry": {
                "cocoa bean": 5,
            }
        }),
        serde_json::json!({
            "cookies": 5,
            "pantry": {
                "cocoa bean": 0,
            }
        }),
    )
    .await?;
    // TASK 3 DONE
    tx.send((false, 100).into()).await.unwrap();

    Ok(())
}

async fn validate_8(base_url: &str, tx: Sender<SubmissionUpdate>) -> ValidateResult {
    let client = new_client();
    let mut test: TaskTest;
    let tol = 0.001f64;
    // TASK 1
    test = (1, 1);
    let url = &format!("{}/8/weight/225", base_url);
    let res = client.get(url).send().await.map_err(|_| test)?;
    let text = res.text().await.map_err(|_| test)?;
    let num: f64 = text.parse().map_err(|_| test)?;
    if !(num.is_finite() && (num - 16f64).abs() < tol) {
        return Err(test);
    }
    test = (1, 2);
    let url = &format!("{}/8/weight/393", base_url);
    let res = client.get(url).send().await.map_err(|_| test)?;
    let text = res.text().await.map_err(|_| test)?;
    let num: f64 = text.parse().map_err(|_| test)?;
    if !(num.is_finite() && (num - 5.2f64).abs() < tol) {
        return Err(test);
    }
    test = (1, 3);
    let url = &format!("{}/8/weight/92", base_url);
    let res = client.get(url).send().await.map_err(|_| test)?;
    let text = res.text().await.map_err(|_| test)?;
    let num: f64 = text.parse().map_err(|_| test)?;
    if !(num.is_finite() && (num - 0.1f64).abs() < tol) {
        return Err(test);
    }
    // TASK 1 DONE
    tx.send((true, 0).into()).await.unwrap();
    tx.send(SubmissionUpdate::Save).await.unwrap();

    // TASK 2
    test = (2, 1);
    let url = &format!("{}/8/drop/383", base_url);
    let res = client.get(url).send().await.map_err(|_| test)?;
    let text = res.text().await.map_err(|_| test)?;
    let num: f64 = text.parse().map_err(|_| test)?;
    if !(num.is_finite() && (num - 13316.953480432378f64).abs() < tol) {
        return Err(test);
    }
    test = (2, 2);
    let url = &format!("{}/8/drop/16", base_url);
    let res = client.get(url).send().await.map_err(|_| test)?;
    let text = res.text().await.map_err(|_| test)?;
    let num: f64 = text.parse().map_err(|_| test)?;
    if !(num.is_finite() && (num - 25.23212238397714f64).abs() < tol) {
        return Err(test);
    }
    test = (2, 3);
    let url = &format!("{}/8/drop/143", base_url);
    let res = client.get(url).send().await.map_err(|_| test)?;
    let text = res.text().await.map_err(|_| test)?;
    let num: f64 = text.parse().map_err(|_| test)?;
    if !(num.is_finite() && (num - 6448.2090536830465f64).abs() < tol) {
        return Err(test);
    }
    // TASK 2 DONE
    tx.send((false, 160).into()).await.unwrap();

    Ok(())
}

async fn validate_11(base_url: &str, tx: Sender<SubmissionUpdate>) -> ValidateResult {
    let client = new_client();
    let mut test: TaskTest;
    // TASK 1
    test = (1, 1);
    let url = &format!("{}/11/assets/decoration.png", base_url);
    let res = client.get(url).send().await.map_err(|_| test)?;
    let headers = res.headers();
    if !headers
        .get("content-type")
        .is_some_and(|v| v == "image/png")
    {
        return Err(test);
    }
    if !headers.get("content-length").is_some_and(|v| v == "787297") {
        return Err(test);
    }
    let bytes = res.bytes().await.map_err(|_| test)?;
    const EXPECTED: &[u8] = include_bytes!("../assets/decoration.png");
    if bytes.to_vec().as_slice() != EXPECTED {
        return Err(test);
    }
    // TASK 1 DONE
    tx.send((true, 0).into()).await.unwrap();
    tx.send(SubmissionUpdate::Save).await.unwrap();

    // TASK 2
    test = (2, 1);
    let url = &format!("{}/11/red_pixels", base_url);
    let form = Form::new().part(
        "image",
        Part::bytes(include_bytes!("../assets/decoration2.png").as_slice())
            .file_name("decoration2.png")
            .mime_str("image/png")
            .unwrap(),
    );
    let res = client
        .post(url)
        .multipart(form)
        .send()
        .await
        .map_err(|_| test)?;
    let text = res.text().await.map_err(|_| test)?;
    if text != "152107" {
        return Err(test);
    }
    test = (2, 2);
    let form = Form::new().part(
        "image",
        Part::bytes(include_bytes!("../assets/decoration3.png").as_slice())
            .file_name("decoration3.png")
            .mime_str("image/png")
            .unwrap(),
    );
    let res = client
        .post(url)
        .multipart(form.into())
        .send()
        .await
        .map_err(|_| test)?;
    let text = res.text().await.map_err(|_| test)?;
    if text != "40263" {
        return Err(test);
    }
    test = (2, 3);
    let form = Form::new().part(
        "image",
        Part::bytes(include_bytes!("../assets/decoration4.png").as_slice())
            .file_name("decoration4.png")
            .mime_str("image/png")
            .unwrap(),
    );
    let res = client
        .post(url)
        .multipart(form.into())
        .send()
        .await
        .map_err(|_| test)?;
    let text = res.text().await.map_err(|_| test)?;
    if text != "86869" {
        return Err(test);
    }
    // TASK 2 DONE
    tx.send((false, 200).into()).await.unwrap();

    Ok(())
}

async fn validate_12(base_url: &str, tx: Sender<SubmissionUpdate>) -> ValidateResult {
    let client = new_client();
    let mut test: TaskTest;
    // TASK 1
    test = (1, 1);
    let url = &format!("{}/12/save/cch23", base_url);
    let res = client.post(url).send().await.map_err(|_| test)?;
    if res.status() != StatusCode::OK {
        return Err(test);
    }
    sleep(Duration::from_secs(2)).await;
    let url = &format!("{}/12/load/cch23", base_url);
    let res = client.get(url).send().await.map_err(|_| test)?;
    let text = res.text().await.map_err(|_| test)?;
    if text != "2" {
        return Err(test);
    }
    sleep(Duration::from_secs(2)).await;
    let url = &format!("{}/12/load/cch23", base_url);
    let res = client.get(url).send().await.map_err(|_| test)?;
    let text = res.text().await.map_err(|_| test)?;
    if text != "4" {
        return Err(test);
    }
    test = (1, 2);
    let url = &format!("{}/12/save/alpha", base_url);
    let res = client.post(url).send().await.map_err(|_| test)?;
    if res.status() != StatusCode::OK {
        return Err(test);
    }
    sleep(Duration::from_secs(2)).await;
    let url = &format!("{}/12/save/omega", base_url);
    let res = client.post(url).send().await.map_err(|_| test)?;
    if res.status() != StatusCode::OK {
        return Err(test);
    }
    sleep(Duration::from_secs(2)).await;
    let url = &format!("{}/12/load/alpha", base_url);
    let res = client.get(url).send().await.map_err(|_| test)?;
    let text = res.text().await.map_err(|_| test)?;
    if text != "4" {
        return Err(test);
    }
    let url = &format!("{}/12/save/alpha", base_url);
    let res = client.post(url).send().await.map_err(|_| test)?;
    if res.status() != StatusCode::OK {
        return Err(test);
    }
    sleep(Duration::from_secs(1)).await;
    let url = &format!("{}/12/load/omega", base_url);
    let res = client.get(url).send().await.map_err(|_| test)?;
    let text = res.text().await.map_err(|_| test)?;
    if text != "3" {
        return Err(test);
    }
    let url = &format!("{}/12/load/alpha", base_url);
    let res = client.get(url).send().await.map_err(|_| test)?;
    let text = res.text().await.map_err(|_| test)?;
    if text != "1" {
        return Err(test);
    }
    // TASK 1 DONE
    tx.send((true, 0).into()).await.unwrap();
    tx.send(SubmissionUpdate::Save).await.unwrap();

    // TASK 2
    test = (2, 1);
    let url = &format!("{}/12/ulids", base_url);
    let res = client
        .post(url)
        .json(&serde_json::json!([
            "01BJQ0E1C3Z56ABCD0E11HYX4M",
            "01BJQ0E1C3Z56ABCD0E11HYX5N",
            "01BJQ0E1C3Z56ABCD0E11HYX6Q",
            "01BJQ0E1C3Z56ABCD0E11HYX7R",
            "01BJQ0E1C3Z56ABCD0E11HYX8P"
        ]))
        .send()
        .await
        .map_err(|_| test)?;
    let json = res.json::<serde_json::Value>().await.map_err(|_| test)?;
    if json
        != serde_json::json!([
            "015cae07-0583-f94c-a5b1-a070431f7516",
            "015cae07-0583-f94c-a5b1-a070431f74f8",
            "015cae07-0583-f94c-a5b1-a070431f74d7",
            "015cae07-0583-f94c-a5b1-a070431f74b5",
            "015cae07-0583-f94c-a5b1-a070431f7494"
        ])
    {
        return Err(test);
    }
    test = (2, 2);
    let res = client
        .post(url)
        .json(&serde_json::json!([]))
        .send()
        .await
        .map_err(|_| test)?;
    let json = res.json::<serde_json::Value>().await.map_err(|_| test)?;
    if json != serde_json::json!([]) {
        return Err(test);
    }
    // TASK 2 DONE
    tx.send((false, 100).into()).await.unwrap();
    tx.send(SubmissionUpdate::Save).await.unwrap();

    // TASK 3
    test = (3, 1);
    let ids = serde_json::json!([
        "00WEGGF0G0J5HEYXS3D7RWZGV8",
        "76EP4G39R8JD1N8AQNYDVJBRCF",
        "018CJ7KMG0051CDCS3B7BFJ3AK",
        "00Y986KPG0AMGB78RD45E9109K",
        "010451HTG0NYWMPWCEXG6AJ8F2",
        "01HH9SJEG0KY16H81S3N1BMXM4",
        "01HH9SJEG0P9M22Z9VGHH9C8CX",
        "017F8YY0G0NQA16HHC2QT5JD6X",
        "03QCPC7P003V1NND3B3QJW72QJ"
    ]);
    let url = &format!("{}/12/ulids/5", base_url);
    let res = client.post(url).json(&ids).send().await.map_err(|_| test)?;
    let json = res.json::<serde_json::Value>().await.map_err(|_| test)?;
    if json
        != serde_json::json!({
            "christmas eve": 3,
            "weekday": 1,
            "in the future": 2,
            "LSB is 1": 5
        })
    {
        return Err(test);
    }
    test = (3, 2);
    let url = &format!("{}/12/ulids/0", base_url);
    let res = client.post(url).json(&ids).send().await.map_err(|_| test)?;
    let json = res.json::<serde_json::Value>().await.map_err(|_| test)?;
    if json
        != serde_json::json!({
            "christmas eve": 3,
            "weekday": 0,
            "in the future": 2,
            "LSB is 1": 5
        })
    {
        return Err(test);
    }
    test = (3, 3);
    let url = &format!("{}/12/ulids/2", base_url);
    let res = client
        .post(url)
        .json(&serde_json::json!(["04BJK8N300BAMR9SQQWPWHVYKZ"]))
        .send()
        .await
        .map_err(|_| test)?;
    let json = res.json::<serde_json::Value>().await.map_err(|_| test)?;
    if json
        != serde_json::json!({
            "christmas eve": 1,
            "weekday": 1,
            "in the future": 1,
            "LSB is 1": 1
        })
    {
        return Err(test);
    }
    // TASK 3 DONE
    tx.send((false, 200).into()).await.unwrap();

    Ok(())
}

async fn validate_13(base_url: &str, tx: Sender<SubmissionUpdate>) -> ValidateResult {
    let client = new_client();
    let mut test: TaskTest;
    // TASK 1
    test = (1, 1);
    let url = &format!("{}/13/sql", base_url);
    let res = client.get(url).send().await.map_err(|_| test)?;
    let text = res.text().await.map_err(|_| test)?;
    if text != "20231213" {
        return Err(test);
    }
    // TASK 1 DONE
    tx.send((false, 0).into()).await.unwrap();
    tx.send(SubmissionUpdate::Save).await.unwrap();

    // TASK 2
    test = (2, 1);
    let reset_url = &format!("{}/13/reset", base_url);
    let order_url = &format!("{}/13/orders", base_url);
    let total_url = &format!("{}/13/orders/total", base_url);
    let res = client.post(reset_url).send().await.map_err(|_| test)?;
    if res.status() != StatusCode::OK {
        return Err(test);
    }
    let res = client
        .post(order_url)
        .json(&serde_json::json!([
            {"id":1,"region_id":2,"gift_name":"Toy Train","quantity":5},
            {"id":2,"region_id":2,"gift_name":"Doll","quantity":8},
            {"id":3,"region_id":3,"gift_name":"Action Figure","quantity":12},
            {"id":4,"region_id":4,"gift_name":"Board Game","quantity":10},
            {"id":5,"region_id":2,"gift_name":"Teddy Bear","quantity":6},
            {"id":6,"region_id":3,"gift_name":"Toy Train","quantity":3},
        ]))
        .send()
        .await
        .map_err(|_| test)?;
    if res.status() != StatusCode::OK {
        return Err(test);
    }
    let res = client.get(total_url).send().await.map_err(|_| test)?;
    let json = res.json::<serde_json::Value>().await.map_err(|_| test)?;
    if json != serde_json::json!({"total": 44}) {
        return Err(test);
    }
    test = (2, 2);
    let res = client
        .post(order_url)
        .json(&serde_json::json!([
            {"id":123,"region_id":6,"gift_name":"Unknown","quantity":333},
        ]))
        .send()
        .await
        .map_err(|_| test)?;
    if res.status() != StatusCode::OK {
        return Err(test);
    }
    let res = client.get(total_url).send().await.map_err(|_| test)?;
    let json = res.json::<serde_json::Value>().await.map_err(|_| test)?;
    if json != serde_json::json!({"total": 377}) {
        return Err(test);
    }
    // TASK 2 DONE
    tx.send((true, 0).into()).await.unwrap();
    tx.send(SubmissionUpdate::Save).await.unwrap();

    // TASK 3
    test = (3, 1);
    let popular_url = &format!("{}/13/orders/popular", base_url);
    let res = client.post(reset_url).send().await.map_err(|_| test)?;
    if res.status() != StatusCode::OK {
        return Err(test);
    }
    let res = client.get(popular_url).send().await.map_err(|_| test)?;
    let json = res.json::<serde_json::Value>().await.map_err(|_| test)?;
    if json != serde_json::json!({"popular": null}) {
        return Err(test);
    }
    test = (3, 2);
    let res = client
        .post(order_url)
        .json(&serde_json::json!([
            {"id":1,"region_id":2,"gift_name":"Lego Rocket","quantity":12},
            {"id":2,"region_id":2,"gift_name":"Action Figure","quantity":18},
            {"id":3,"region_id":5,"gift_name":"Toy Train","quantity":19},
            {"id":4,"region_id":5,"gift_name":"Lego Rocket","quantity":12},
            {"id":5,"region_id":4,"gift_name":"Toy Train","quantity":15},
            {"id":6,"region_id":2,"gift_name":"Toy Train","quantity":7},
            {"id":7,"region_id":3,"gift_name":"Toy Train","quantity":19},
            {"id":8,"region_id":4,"gift_name":"Action Figure","quantity":8},
            {"id":9,"region_id":2,"gift_name":"Toy Axe","quantity":15},
            {"id":10,"region_id":4,"gift_name":"Toy Axe","quantity":1},
            {"id":11,"region_id":2,"gift_name":"Toy Train","quantity":17},
            {"id":12,"region_id":4,"gift_name":"Toy Train","quantity":5},
            {"id":13,"region_id":4,"gift_name":"Sweater","quantity":20},
            {"id":14,"region_id":4,"gift_name":"Action Figure","quantity":7},
            {"id":15,"region_id":2,"gift_name":"Toy Train","quantity":16},
            {"id":16,"region_id":3,"gift_name":"Action Figure","quantity":12},
            {"id":17,"region_id":4,"gift_name":"Toy Axe","quantity":2},
            {"id":18,"region_id":3,"gift_name":"Toy Train","quantity":9},
            {"id":19,"region_id":2,"gift_name":"Sweater","quantity":9},
            {"id":20,"region_id":5,"gift_name":"Toy Train","quantity":9},
            {"id":21,"region_id":4,"gift_name":"Action Figure","quantity":11},
            {"id":22,"region_id":3,"gift_name":"Toy Train","quantity":7},
            {"id":23,"region_id":2,"gift_name":"Action Figure","quantity":5},
            {"id":24,"region_id":4,"gift_name":"Action Figure","quantity":17},
            {"id":25,"region_id":5,"gift_name":"Lego Rocket","quantity":6},
            {"id":26,"region_id":2,"gift_name":"Sweater","quantity":5},
            {"id":27,"region_id":5,"gift_name":"Toy Train","quantity":4},
            {"id":28,"region_id":4,"gift_name":"Lego Rocket","quantity":8},
            {"id":29,"region_id":2,"gift_name":"Toy Train","quantity":3},
            {"id":30,"region_id":4,"gift_name":"Toy Axe","quantity":20},
            {"id":31,"region_id":2,"gift_name":"Action Figure","quantity":5},
            {"id":32,"region_id":2,"gift_name":"Lego Rocket","quantity":10},
            {"id":33,"region_id":5,"gift_name":"Toy Train","quantity":4},
            {"id":34,"region_id":2,"gift_name":"Toy Axe","quantity":14},
            {"id":35,"region_id":3,"gift_name":"Action Figure","quantity":18},
            {"id":36,"region_id":5,"gift_name":"Toy Axe","quantity":10},
            {"id":37,"region_id":4,"gift_name":"Lego Rocket","quantity":6},
            {"id":38,"region_id":4,"gift_name":"Action Figure","quantity":16},
            {"id":39,"region_id":4,"gift_name":"Toy Axe","quantity":15},
            {"id":40,"region_id":5,"gift_name":"Lego Rocket","quantity":15},
            {"id":41,"region_id":5,"gift_name":"Action Figure","quantity":7},
            {"id":42,"region_id":3,"gift_name":"Action Figure","quantity":16},
            {"id":43,"region_id":3,"gift_name":"Toy Train","quantity":8},
            {"id":44,"region_id":4,"gift_name":"Action Figure","quantity":13},
            {"id":45,"region_id":3,"gift_name":"Lego Rocket","quantity":12},
            {"id":46,"region_id":3,"gift_name":"Toy Train","quantity":1},
            {"id":47,"region_id":2,"gift_name":"Toy Train","quantity":11},
            {"id":48,"region_id":5,"gift_name":"Action Figure","quantity":1},
            {"id":49,"region_id":4,"gift_name":"Toy Train","quantity":13},
            {"id":50,"region_id":5,"gift_name":"Action Figure","quantity":16},
            {"id":51,"region_id":4,"gift_name":"Toy Axe","quantity":19},
            {"id":52,"region_id":2,"gift_name":"Toy Train","quantity":14},
            {"id":53,"region_id":3,"gift_name":"Action Figure","quantity":16},
        ]))
        .send()
        .await
        .map_err(|_| test)?;
    if res.status() != StatusCode::OK {
        return Err(test);
    }
    let res = client.get(popular_url).send().await.map_err(|_| test)?;
    let json = res.json::<serde_json::Value>().await.map_err(|_| test)?;
    if json != serde_json::json!({"popular": "Action Figure"}) {
        return Err(test);
    }
    // TASK 3 DONE
    tx.send((false, 100).into()).await.unwrap();

    Ok(())
}

async fn validate_14(base_url: &str, tx: Sender<SubmissionUpdate>) -> ValidateResult {
    let client = new_client();
    let mut test: TaskTest;
    // TASK 1
    test = (1, 1);
    let url = &format!("{}/14/unsafe", base_url);
    let res = client
        .post(url)
        .json(&serde_json::json!({"content": "Bing Chilling 🥶🍦"}))
        .send()
        .await
        .map_err(|_| test)?;
    let text = res.text().await.map_err(|_| test)?;
    if text
        != "\
<html>
  <head>
    <title>CCH23 Day 14</title>
  </head>
  <body>
    Bing Chilling 🥶🍦
  </body>
</html>"
    {
        return Err(test);
    }
    test = (1, 2);
    let res = client
        .post(url)
        .json(&serde_json::json!({"content": r#"<script>alert("XSS Attack Success!")</script>"#}))
        .send()
        .await
        .map_err(|_| test)?;
    let text = res.text().await.map_err(|_| test)?;
    if text
        != "\
<html>
  <head>
    <title>CCH23 Day 14</title>
  </head>
  <body>
    <script>alert(\"XSS Attack Success!\")</script>
  </body>
</html>"
    {
        return Err(test);
    }
    // TASK 1 DONE
    tx.send((true, 0).into()).await.unwrap();
    tx.send(SubmissionUpdate::Save).await.unwrap();

    // TASK 2
    test = (2, 1);
    let url = &format!("{}/14/safe", base_url);
    let res = client
        .post(url)
        .json(&serde_json::json!({"content": r#"<script>alert("XSS Attack Failed!")</script>"#}))
        .send()
        .await
        .map_err(|_| test)?;
    let text = res.text().await.map_err(|_| test)?;
    if text
        != "\
<html>
  <head>
    <title>CCH23 Day 14</title>
  </head>
  <body>
    &lt;script&gt;alert(&quot;XSS Attack Failed!&quot;)&lt;/script&gt;
  </body>
</html>"
    {
        return Err(test);
    }
    // TASK 2 DONE
    tx.send((false, 100).into()).await.unwrap();

    Ok(())
}

struct JSONTester {
    client: reqwest::Client,
    url: String,
}

impl JSONTester {
    fn new(url: String) -> Self {
        Self {
            client: new_client(),
            url,
        }
    }
    async fn test(
        &self,
        test: TaskTest,
        i: &serde_json::Value,
        code: StatusCode,
        o: &serde_json::Value,
    ) -> ValidateResult {
        let res = self
            .client
            .post(&self.url)
            .json(i)
            .send()
            .await
            .map_err(|_| test)?;
        if res.status() != code {
            return Err(test);
        }
        let json = res.json::<serde_json::Value>().await.map_err(|_| test)?;
        if json != *o {
            return Err(test);
        }
        Ok(())
    }
}

async fn validate_15(base_url: &str, tx: Sender<SubmissionUpdate>) -> ValidateResult {
    // TASK 1
    let t = JSONTester::new(format!("{}/15/nice", base_url));
    t.test(
        (1, 1),
        &serde_json::json!({"input": "hello there"}),
        StatusCode::OK,
        &serde_json::json!({"result": "nice"}),
    )
    .await?;
    t.test(
        (1, 2),
        &serde_json::json!({"input": "he77o there"}),
        StatusCode::BAD_REQUEST,
        &serde_json::json!({"result": "naughty"}),
    )
    .await?;
    t.test(
        (1, 3),
        &serde_json::json!({"input": "hello"}),
        StatusCode::BAD_REQUEST,
        &serde_json::json!({"result": "naughty"}),
    )
    .await?;
    t.test(
        (1, 4),
        &serde_json::json!({"input": "hello xylophone"}),
        StatusCode::BAD_REQUEST,
        &serde_json::json!({"result": "naughty"}),
    )
    .await?;
    t.test(
        (1, 5),
        &serde_json::json!({"input": "password"}),
        StatusCode::BAD_REQUEST,
        &serde_json::json!({"result": "naughty"}),
    )
    .await?;
    let test = (1, 6);
    let res = new_client()
        .post(format!("{}/15/nice", base_url))
        .header(CONTENT_TYPE, HeaderValue::from_static("application/json"))
        .body("WooooOOOooOOOoooOO 👻")
        .send()
        .await
        .map_err(|_| test)?;
    if res.status() != StatusCode::BAD_REQUEST {
        return Err(test);
    }
    // TASK 1 DONE
    tx.send((true, 0).into()).await.unwrap();
    tx.send(SubmissionUpdate::Save).await.unwrap();

    // TASK 2
    let t = JSONTester::new(format!("{}/15/game", base_url));
    t.test(
        (2, 1),
        &serde_json::json!({"input": "mario"}),
        StatusCode::BAD_REQUEST,
        &serde_json::json!({"result": "naughty", "reason": "8 chars"}),
    )
    .await?;
    t.test(
        (2, 2),
        &serde_json::json!({"input": "mariobro"}),
        StatusCode::BAD_REQUEST,
        &serde_json::json!({"result": "naughty", "reason": "more types of chars"}),
    )
    .await?;
    t.test(
        (2, 3),
        &serde_json::json!({"input": "EEEEEEEEEEE"}),
        StatusCode::BAD_REQUEST,
        &serde_json::json!({"result": "naughty", "reason": "more types of chars"}),
    )
    .await?;
    t.test(
        (2, 4),
        &serde_json::json!({"input": "E3E3E3E3E3E"}),
        StatusCode::BAD_REQUEST,
        &serde_json::json!({"result": "naughty", "reason": "more types of chars"}),
    )
    .await?;
    t.test(
        (2, 5),
        &serde_json::json!({"input": "e3E3e#eE#ee3#EeE3"}),
        StatusCode::BAD_REQUEST,
        &serde_json::json!({"result": "naughty", "reason": "55555"}),
    )
    .await?;
    t.test(
        (2, 6),
        &serde_json::json!({"input": "Password12345"}),
        StatusCode::BAD_REQUEST,
        &serde_json::json!({"result": "naughty", "reason": "math is hard"}),
    )
    .await?;
    t.test(
        (2, 7),
        &serde_json::json!({"input": "2 00 2 3 OOgaBooga"}),
        StatusCode::BAD_REQUEST,
        &serde_json::json!({"result": "naughty", "reason": "math is hard"}),
    )
    .await?;
    t.test(
        (2, 8),
        &serde_json::json!({"input": "2+2/2-8*8 = 1-2000 OOgaBooga"}),
        StatusCode::NOT_ACCEPTABLE,
        &serde_json::json!({"result": "naughty", "reason": "not joyful enough"}),
    )
    .await?;
    t.test(
        (2, 9),
        &serde_json::json!({"input": "2000.23.A yoyoj"}),
        StatusCode::NOT_ACCEPTABLE,
        &serde_json::json!({"result": "naughty", "reason": "not joyful enough"}),
    )
    .await?;
    t.test(
        (2, 10),
        &serde_json::json!({"input": "2000.23.A joy joy"}),
        StatusCode::NOT_ACCEPTABLE,
        &serde_json::json!({"result": "naughty", "reason": "not joyful enough"}),
    )
    .await?;
    t.test(
        (2, 11),
        &serde_json::json!({"input": "2000.23.A joyo"}),
        StatusCode::NOT_ACCEPTABLE,
        &serde_json::json!({"result": "naughty", "reason": "not joyful enough"}),
    )
    .await?;
    t.test(
        (2, 12),
        &serde_json::json!({"input": "2000.23.A j  ;)  o  ;)  y "}),
        StatusCode::UNAVAILABLE_FOR_LEGAL_REASONS,
        &serde_json::json!({"result": "naughty", "reason": "illegal: no sandwich"}),
    )
    .await?;
    t.test(
        (2, 13),
        &serde_json::json!({"input": "2020.3.A j  ;)  o  ;)  y"}),
        StatusCode::UNAVAILABLE_FOR_LEGAL_REASONS,
        &serde_json::json!({"result": "naughty", "reason": "illegal: no sandwich"}),
    )
    .await?;
    t.test(
        (2, 14),
        &serde_json::json!({"input": "2000.23.A j  ;)  o  ;)  y AzA"}),
        StatusCode::RANGE_NOT_SATISFIABLE,
        &serde_json::json!({"result": "naughty", "reason": "outranged"}),
    )
    .await?;
    t.test(
        (2, 15),
        &serde_json::json!({"input": "2000.23.A j  ;)  o  ;)  y⥿ AzA"}),
        StatusCode::RANGE_NOT_SATISFIABLE,
        &serde_json::json!({"result": "naughty", "reason": "outranged"}),
    )
    .await?;
    t.test(
        (2, 16),
        &serde_json::json!({"input": "2000.23.A j  ;)  o  ;)  y ⦄AzA"}),
        StatusCode::UPGRADE_REQUIRED,
        &serde_json::json!({"result": "naughty", "reason": "😳"}),
    )
    .await?;
    t.test(
        (2, 17),
        &serde_json::json!({"input": "2000.23.A j  🥶  o  🍦  y ⦄AzA"}),
        StatusCode::IM_A_TEAPOT,
        &serde_json::json!({"result": "naughty", "reason": "not a coffee brewer"}),
    )
    .await?;
    t.test(
        (2, 18),
        &serde_json::json!({"input": "2000.23.A j ⦖⦖⦖⦖⦖⦖⦖⦖ 🥶  o  🍦  y ⦄AzA"}),
        StatusCode::OK,
        &serde_json::json!({"result": "nice", "reason": "that's a nice password"}),
    )
    .await?;
    // TASK 2 DONE
    tx.send((false, 400).into()).await.unwrap();

    Ok(())
}

struct RegionGiftTester {
    client: reqwest::Client,
    reset_url: String,
    regions_url: String,
    orders_url: String,
    final_url: String,
}

impl RegionGiftTester {
    async fn test(
        &self,
        test: TaskTest,
        i1: &serde_json::Value,
        i2: &serde_json::Value,
        o: &serde_json::Value,
    ) -> ValidateResult {
        let res = self
            .client
            .post(&self.reset_url)
            .send()
            .await
            .map_err(|_| test)?;
        if res.status() != StatusCode::OK {
            return Err(test);
        }
        let res = self
            .client
            .post(&self.regions_url)
            .json(i1)
            .send()
            .await
            .map_err(|_| test)?;
        if res.status() != StatusCode::OK {
            return Err(test);
        }
        let res = self
            .client
            .post(&self.orders_url)
            .json(i2)
            .send()
            .await
            .map_err(|_| test)?;
        if res.status() != StatusCode::OK {
            return Err(test);
        }
        let res = self
            .client
            .get(&self.final_url)
            .send()
            .await
            .map_err(|_| test)?;
        if res.status() != StatusCode::OK {
            return Err(test);
        }
        let json = res.json::<serde_json::Value>().await.map_err(|_| test)?;
        if json != *o {
            return Err(test);
        }
        Ok(())
    }
}

async fn validate_18(base_url: &str, tx: Sender<SubmissionUpdate>) -> ValidateResult {
    // TASK 1
    let t = RegionGiftTester {
        client: new_client(),
        reset_url: format!("{}/18/reset", base_url),
        regions_url: format!("{}/18/regions", base_url),
        orders_url: format!("{}/18/orders", base_url),
        final_url: format!("{}/18/regions/total", base_url),
    };
    t.test(
        (1, 1),
        &serde_json::json!([{"id":1,"name":"North Pole"}]),
        &serde_json::json!([]),
        &serde_json::json!([]),
    )
    .await?;
    t.test(
        (1, 2),
        &serde_json::json!([]),
        &serde_json::json!([{"id":1,"region_id":2,"gift_name":"Board Game","quantity":5}]),
        &serde_json::json!([]),
    )
    .await?;
    t.test(
        (1, 3),
        &serde_json::json!([{"id":1,"name":"A"}]),
        &serde_json::json!([{"id":1,"region_id":1,"gift_name":"A","quantity":1}]),
        &serde_json::json!([{"region":"A","total":1}]),
    )
    .await?;
    t.test(
        (1, 4),
        &serde_json::json!([{"id":1,"name":"A"}]),
        &serde_json::json!([
            {"id":1,"region_id":1,"gift_name":"A","quantity":1},
            {"id":2,"region_id":1,"gift_name":"A","quantity":1},
            {"id":3,"region_id":1,"gift_name":"A","quantity":1}
        ]),
        &serde_json::json!([{"region":"A","total":3}]),
    )
    .await?;
    t.test(
        (1, 5),
        &serde_json::json!([
            {"id":1,"name":"A"},
            {"id":2,"name":"B"}
        ]),
        &serde_json::json!([
            {"id":1,"region_id":1,"gift_name":"A","quantity":1},
            {"id":2,"region_id":1,"gift_name":"A","quantity":1},
            {"id":3,"region_id":2,"gift_name":"B","quantity":1}
        ]),
        &serde_json::json!([
            {"region":"A","total":2},
            {"region":"B","total":1}
        ]),
    )
    .await?;
    t.test(
        (1, 6),
        &serde_json::json!([
            {"id":1,"name":"A"},
            {"id":2,"name":"B"}
        ]),
        &serde_json::json!([
            {"id":1,"region_id":1,"gift_name":"A","quantity":1},
            {"id":2,"region_id":1,"gift_name":"A","quantity":1},
            {"id":3,"region_id":3,"gift_name":"C","quantity":1}
        ]),
        &serde_json::json!([{"region":"A","total":2}]),
    )
    .await?;
    t.test(
        (1, 7),
        &serde_json::json!([{"id":1,"name":"A"}]),
        &serde_json::json!([{"id":1,"region_id":1,"gift_name":"A","quantity":555555555}]),
        &serde_json::json!([{"region":"A","total":555555555}]),
    )
    .await?;
    t.test(
        (1, 8),
        &serde_json::json!([{"id":-1,"name":"A"}]),
        &serde_json::json!([
            {"id":-1,"region_id":-1,"gift_name":"A","quantity":-1},
            {"id":0,"region_id":-1,"gift_name":"A","quantity":1}
        ]),
        &serde_json::json!([{"region":"A","total":0}]),
    )
    .await?;
    // TASK 1 DONE
    tx.send((true, 0).into()).await.unwrap();
    tx.send(SubmissionUpdate::Save).await.unwrap();

    // TASK 2
    let t = RegionGiftTester {
        client: new_client(),
        reset_url: format!("{}/18/reset", base_url),
        regions_url: format!("{}/18/regions", base_url),
        orders_url: format!("{}/18/orders", base_url),
        final_url: format!("{}/18/regions/top_list/2", base_url),
    };
    t.test(
        (2, 1),
        &serde_json::json!([]),
        &serde_json::json!([]),
        &serde_json::json!([]),
    )
    .await?;
    t.test(
        (2, 2),
        &serde_json::json!([{"id":1,"name":"A"}]),
        &serde_json::json!([]),
        &serde_json::json!([{"region":"A","top_gifts":[]}]),
    )
    .await?;
    t.test(
        (2, 3),
        &serde_json::json!([]),
        &serde_json::json!([{"id":1,"region_id":2,"gift_name":"B","quantity":5}]),
        &serde_json::json!([]),
    )
    .await?;
    t.test(
        (2, 4),
        &serde_json::json!([{"id":1,"name":"A"}]),
        &serde_json::json!([{"id":1,"region_id":2,"gift_name":"B","quantity":5}]),
        &serde_json::json!([{"region":"A","top_gifts":[]}]),
    )
    .await?;
    t.test(
        (2, 5),
        &serde_json::json!([{"id":1,"name":"A"}]),
        &serde_json::json!([
            {"id":1,"region_id":1,"gift_name":"B","quantity":10},
            {"id":2,"region_id":1,"gift_name":"A","quantity":5},
            {"id":3,"region_id":1,"gift_name":"A","quantity":5},
            {"id":4,"region_id":1,"gift_name":"C","quantity":9}
        ]),
        &serde_json::json!([{"region":"A","top_gifts":["A","B"]}]),
    )
    .await?;
    let regions = serde_json::json!([
        {"id":1,"name":"North Pole"},
        {"id":2,"name":"Europe"},
        {"id":3,"name":"North America"},
        {"id":4,"name":"South America"},
        {"id":5,"name":"Africa"},
        {"id":6,"name":"Asia"},
        {"id":7,"name":"Oceania"}
    ]);
    t.test(
        (2, 6),
        &regions,
        &serde_json::json!([
            {"id":1,"region_id":2,"gift_name":"Toy Train","quantity":5},
            {"id":2,"region_id":2,"gift_name":"Toy Train","quantity":3},
            {"id":3,"region_id":2,"gift_name":"Doll","quantity":8},
            {"id":4,"region_id":3,"gift_name":"Toy Train","quantity":3},
            {"id":5,"region_id":2,"gift_name":"Teddy Bear","quantity":6},
            {"id":6,"region_id":3,"gift_name":"Action Figure","quantity":12},
            {"id":7,"region_id":4,"gift_name":"Board Game","quantity":10},
            {"id":8,"region_id":3,"gift_name":"Teddy Bear","quantity":1},
            {"id":9,"region_id":3,"gift_name":"Teddy Bear","quantity":2}
        ]),
        &serde_json::json!([
            {"region":"Africa","top_gifts":[]},
            {"region":"Asia","top_gifts":[]},
            {"region":"Europe","top_gifts":["Doll","Toy Train"]},
            {"region":"North America","top_gifts":["Action Figure","Teddy Bear"]},
            {"region":"North Pole","top_gifts":[]},
            {"region":"Oceania","top_gifts":[]},
            {"region":"South America","top_gifts":["Board Game"]},
        ]),
    )
    .await?;
    let t = RegionGiftTester {
        client: new_client(),
        reset_url: format!("{}/18/reset", base_url),
        regions_url: format!("{}/18/regions", base_url),
        orders_url: format!("{}/18/orders", base_url),
        final_url: format!("{}/18/regions/top_list/3", base_url),
    };
    t.test(
        (2, 7),
        &regions,
        &serde_json::json!([
            {"id":1,"region_id":2,"gift_name":"Toy Train","quantity":5},
            {"id":2,"region_id":2,"gift_name":"Toy Train","quantity":3},
            {"id":3,"region_id":2,"gift_name":"Doll","quantity":8},
            {"id":4,"region_id":3,"gift_name":"Toy Train","quantity":3},
            {"id":5,"region_id":2,"gift_name":"Teddy Bear","quantity":6},
            {"id":6,"region_id":3,"gift_name":"Action Figure","quantity":12},
            {"id":7,"region_id":4,"gift_name":"Board Game","quantity":10},
            {"id":8,"region_id":3,"gift_name":"Teddy Bear","quantity":1},
            {"id":9,"region_id":3,"gift_name":"Teddy Bear","quantity":2}
        ]),
        &serde_json::json!([
            {"region":"Africa","top_gifts":[]},
            {"region":"Asia","top_gifts":[]},
            {"region":"Europe","top_gifts":["Doll","Toy Train","Teddy Bear"]},
            {"region":"North America","top_gifts":["Action Figure","Teddy Bear","Toy Train"]},
            {"region":"North Pole","top_gifts":[]},
            {"region":"Oceania","top_gifts":[]},
            {"region":"South America","top_gifts":["Board Game"]},
        ]),
    )
    .await?;
    let t = RegionGiftTester {
        client: new_client(),
        reset_url: format!("{}/18/reset", base_url),
        regions_url: format!("{}/18/regions", base_url),
        orders_url: format!("{}/18/orders", base_url),
        final_url: format!("{}/18/regions/top_list/0", base_url),
    };
    t.test(
        (2, 8),
        &serde_json::json!([{"id":1,"name":"A"}]),
        &serde_json::json!([{"id":1,"region_id":1,"gift_name":"A","quantity":555555555}]),
        &serde_json::json!([{"region":"A","top_gifts":[]}]),
    )
    .await?;
    // TASK 2 DONE
    tx.send((false, 600).into()).await.unwrap();

    Ok(())
}

struct WS {
    test: TaskTest,
    w: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
    r: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
}

impl WS {
    async fn new(test: TaskTest, url: String) -> Result<Self, TaskTest> {
        let (s, _) = tokio_tungstenite::connect_async(url)
            .await
            .map_err(|_| test)?;
        let (w, r) = s.split();

        Ok(Self { test, w, r })
    }

    async fn send(&mut self, msg: impl Into<String>) -> ValidateResult {
        self.w
            .send(Message::Text(msg.into()))
            .await
            .map_err(|_| self.test)
    }

    async fn send_tweet(&mut self, msg: impl Into<String>) -> ValidateResult {
        self.send(serde_json::to_string(&serde_json::json!({"message": msg.into()})).unwrap())
            .await
    }

    async fn recv(&mut self) -> Result<String, TaskTest> {
        let Some(Ok(Message::Text(text))) = self.r.next().await else {
            return Err(self.test);
        };

        Ok(text)
    }

    async fn recv_str(&mut self, exp: &str) -> ValidateResult {
        let text = self.recv().await?;
        if text != exp {
            return Err(self.test);
        }

        Ok(())
    }

    async fn recv_json(&mut self, exp: &serde_json::Value) -> ValidateResult {
        let text = self.recv().await?;
        let json = serde_json::from_str::<serde_json::Value>(&text).map_err(|_| self.test)?;
        if &json != exp {
            return Err(self.test);
        }

        Ok(())
    }

    async fn close(mut self) -> ValidateResult {
        self.w.close().await.map_err(|_| self.test)?;

        Ok(())
    }
}

async fn validate_19(base_url: &str, tx: Sender<SubmissionUpdate>) -> ValidateResult {
    let mut test: TaskTest;
    let ws_base_url = format!(
        "ws{}",
        base_url
            .strip_prefix("http")
            .expect("url to begin with http")
    );
    // TASK 1
    test = (1, 1);
    let mut ws = WS::new(test, format!("{}/19/ws/ping", ws_base_url)).await?;
    ws.send("ping").await?;
    tokio::select! {
        _ = ws.recv() => {
            return Err(test);
        },
        _ = sleep(Duration::from_secs(1)) => (),
    };
    ws.send("serve").await?;
    ws.send("ping").await?;
    ws.recv_str("pong").await?;
    test = (1, 2);
    ws.test = test;
    ws.send("ding").await?;
    tokio::select! {
        _ = ws.recv() => {
            return Err(test);
        },
        _ = sleep(Duration::from_secs(1)) => (),
    };
    test = (1, 3);
    ws.test = test;
    ws.send("ping").await?;
    ws.send("ping").await?;
    ws.recv_str("pong").await?;
    ws.recv_str("pong").await?;
    tokio::select! {
        _ = ws.recv() => {
            return Err(test);
        },
        _ = sleep(Duration::from_millis(500)) => (),
    };
    ws.close().await?;
    // TASK 1 DONE
    tx.send((true, 0).into()).await.unwrap();
    tx.send(SubmissionUpdate::Save).await.unwrap();

    // TASK 2
    let reset_url = &format!("{}/19/reset", base_url);
    let reset = || async move {
        let client = new_client();
        let res = client.post(reset_url).send().await.map_err(|_| ())?;
        if res.status() != StatusCode::OK {
            return Err(());
        }
        Ok(())
    };
    let views_url = &format!("{}/19/views", base_url);
    let ensure_views = |v: u32| async move {
        let client = new_client();
        let res = client.get(views_url).send().await.map_err(|_| ())?;
        let text = res.text().await.map_err(|_| ())?;
        if text != v.to_string() {
            return Err(());
        }
        Ok(())
    };

    test = (2, 1);
    reset().await.map_err(|_| test)?;
    ensure_views(0).await.map_err(|_| test)?;

    test = (2, 2);
    let mut elon = WS::new(test, format!("{}/19/ws/room/1/user/elonmusk", ws_base_url)).await?;
    let s = "Next I'm buying Coca-Cola to put the cocaine back in";
    elon.send_tweet(s).await?;
    elon.recv_json(&serde_json::json!({"user": "elonmusk", "message": s}))
        .await?;
    ensure_views(1).await.map_err(|_| test)?;

    test = (2, 3);
    let s = "I've concocted a whimsical idea to bring a bit of the ol' history back to life by attempting to put the cocaine back in Coca-Cola, rekindling the rebellious spirit of its original formulation";
    elon.send_tweet(s).await?;
    tokio::select! {
        _ = elon.recv() => {
            return Err(test);
        },
        _ = sleep(Duration::from_secs(1)) => (),
    };
    ensure_views(1).await.map_err(|_| test)?;
    elon.close().await?;
    sleep(Duration::from_millis(10)).await;

    test = (2, 4);
    reset().await.map_err(|_| test)?;
    ensure_views(0).await.map_err(|_| test)?;
    let mut a1 = WS::new(test, format!("{}/19/ws/room/44/user/annifrid", ws_base_url)).await?;
    let mut b1 = WS::new(test, format!("{}/19/ws/room/55/user/bjorn", ws_base_url)).await?;
    let mut b2 = WS::new(test, format!("{}/19/ws/room/55/user/benny", ws_base_url)).await?;
    let mut a2 = WS::new(test, format!("{}/19/ws/room/44/user/agnetha", ws_base_url)).await?;
    let l1 = "thank you for the music";
    let l2 = "the songs i'm singing";
    let l3 = "thanks for all";
    let l4 = "the joy they're bringing";
    let l5 = "who can live without it";
    let l6 = "i ask in all honesty";
    let x1 = "uhhhhhhhh?";
    let x2 = "wazzaaaaa?";
    a1.send_tweet(l1).await?;
    sleep(Duration::from_millis(10)).await;
    a2.send_tweet(l2).await?;
    sleep(Duration::from_millis(10)).await;
    a1.send_tweet(l3).await?;
    sleep(Duration::from_millis(10)).await;
    b1.send_tweet(x1).await?;
    sleep(Duration::from_millis(10)).await;
    a2.send_tweet(l4).await?;
    sleep(Duration::from_millis(10)).await;
    a1.send_tweet(l5).await?;
    sleep(Duration::from_millis(10)).await;
    a1.recv_json(&serde_json::json!({"user": "annifrid", "message": l1}))
        .await?;
    a2.recv_json(&serde_json::json!({"user": "annifrid", "message": l1}))
        .await?;
    a1.recv_json(&serde_json::json!({"user": "agnetha", "message": l2}))
        .await?;
    a2.recv_json(&serde_json::json!({"user": "agnetha", "message": l2}))
        .await?;
    a1.recv_json(&serde_json::json!({"user": "annifrid", "message": l3}))
        .await?;
    a2.recv_json(&serde_json::json!({"user": "annifrid", "message": l3}))
        .await?;
    a1.recv_json(&serde_json::json!({"user": "agnetha", "message": l4}))
        .await?;
    a2.recv_json(&serde_json::json!({"user": "agnetha", "message": l4}))
        .await?;
    a1.recv_json(&serde_json::json!({"user": "annifrid", "message": l5}))
        .await?;
    a2.recv_json(&serde_json::json!({"user": "annifrid", "message": l5}))
        .await?;
    sleep(Duration::from_millis(10)).await;
    ensure_views(12).await.map_err(|_| test)?;

    test = (2, 5);
    a1.close().await?;
    a2.send_tweet(l6).await?;
    a2.recv_json(&serde_json::json!({"user": "agnetha", "message": l6}))
        .await?;
    sleep(Duration::from_millis(10)).await;
    ensure_views(13).await.map_err(|_| test)?;

    test = (2, 6);
    let mut a1 = WS::new(test, format!("{}/19/ws/room/55/user/annifrid", ws_base_url)).await?;
    tokio::select! {
        _ = a1.recv() => {
            return Err(test);
        },
        _ = sleep(Duration::from_secs(1)) => (),
    };
    b1.recv_json(&serde_json::json!({"user": "bjorn", "message": x1}))
        .await?;
    b2.recv_json(&serde_json::json!({"user": "bjorn", "message": x1}))
        .await?;
    a1.send_tweet(x2).await?;
    sleep(Duration::from_millis(10)).await;
    b1.close().await?;
    a1.send_tweet(x2).await?;
    b2.recv_json(&serde_json::json!({"user": "annifrid", "message": x2}))
        .await?;
    b2.recv_json(&serde_json::json!({"user": "annifrid", "message": x2}))
        .await?;
    a1.recv_json(&serde_json::json!({"user": "annifrid", "message": x2}))
        .await?;
    a1.recv_json(&serde_json::json!({"user": "annifrid", "message": x2}))
        .await?;
    sleep(Duration::from_millis(10)).await;
    ensure_views(18).await.map_err(|_| test)?;

    test = (2, 7);
    reset().await.map_err(|_| test)?;
    ensure_views(0).await.map_err(|_| test)?;
    // generated with https://github.com/orhun/godsays
    let phrases = Arc::new([
        "you're nuts lift Greek to me cheerful don't mention it I made it that way quit it",
        "just lovely left field king of mars threads do it insane its trivial obviously",
        "not that theres anything wrong surprise surprise you'll see ba ha no you cant off the record Jesus",
        "you don't like it employer joke small talk that's all folks Varoom yikes",
        "Russia grumble failure to communicate Greece enough let me count the ways nut job",
        "don't push it Han shot first Is that so big fish Jedi mind trick you never know game changer",
        "on occassion that's no fun if and only if no more tears cracks me up it was nothing whiner",
        "Wow piety figuratively figuratively you're no fun hot air astrophysics",
        "astounding duck the shoe relax you think you could do better is it just me or What are you doing Dave Bam",
        "to infinity and beyond basket case no more let me count the ways one more time that's for me to know NOT",
        "What I want relax what planet are you from not that theres anything wrong What I want phasors on stun walking",
        "ice cream this might end badly thank you very much I'm not sure catastrophe beam me up food",
        "nope wazz up with that grumble awesome yuck are you sure recipe",
        "I'll let you know FBI wishful thinking jobs what's up Heaven Ghost",
        "take the day off repeat after me scum let's roll I'll ask nicely stuff duck the shoe",
        "not a chance in hell nut job nope heathen air head basically why do I put up with this",
        "chill out I'm not sure sad no news is good news news to me biggot whatcha talkin' 'bout",
        "well obviously That's gonna leave a mark if anything can go wrong debt play exports rose colored glasses",
        "not in my wildest dreams game changer Zzzzzzzz do over how could you look on the brightside You da man",
        "Ivy league oh no a likely story you're lucky face palm what luck I'll think about it",
        "game over homo segway gluttony pwned China test pilot",
        "after a break strip you owe me fight humongous God never happy",
        "take the day off bizarre on occassion just between us I'll think about it application I veto that",
        "spending look out enough is it just me or jealousy debt that's much better",
        "I didn't do it gross Han shot first I had a crazy dream tree hugger LOL music",
        "I have an idea chill you're nuts glorious CIA astrophysics king nun",
        "no more tears left field Ivy league break some woopass on you go ahead make my day don't push it middle class",
        "bizarre fer sure now that I think about it I'll be back in a galaxy far far away holy grail you couldnt navigate yer way circleK",
        "honesty holy grail failure is not an option hi let me count the ways Oh really are you deaf",
        "Isn't that special my precious it'd take a miracle the enquirer hobnob job handyman",
        "dance oh my chill If had my druthers evolution you know a better God could it be   Satan",
        "I'm in suspense Heaven joking experts you owe me That's gonna leave a mark spoiled brat",
        "delicious you should be so lucky basket case chess you couldnt navigate yer way circleK smack some sense into you Yawn",
        "I could swear game changer what would Jesus do just between us news to me Ghost charity",
        "climate I donno threads food What I want roses are red you're so screwed",
        "my precious Okilydokily energy dignity atrocious quit it when hell freezes over",
        "I give up Watch this now you tell me courage love relax you do it",
        "I could swear delightful Catastrophic Success bad why is it King Midas happy",
        "I'll think about it it's hopeless well I never stoked air head I'll ask nicely end",
        "you don't like it You fix it got the life imports rip off computers I don't care",
        "now that I think about it rich I'll let you know humongous let's roll ahh thats much better no way dude",
        "atrocious Hicc up ghastly don't worry hello I could be wrong heathen",
        "chill out ouch fool you couldnt navigate yer way circleK I'm done earnest threads",
        "energy ba ha ghetto I'm the boss boink King Midas you better not",
        "spoiled brat overflow after a break don't push it fabulous chill you don't like it",
        "don't worry other Russia wonderbread ohh thank you endure how high",
        "ridiculous What are you doing Dave crash and burn manufacturing chill gosh thank you very much",
        "how do I put this astronomical I had a crazy dream umm If had my druthers Varoom are you deaf",
        "Han shot first car tiffanies fool Shalom who are you to judge charged",
        "take your pick atheist don't even think about it I was just thinking you talkin' to me conservative scorning",
        "daunting quit it SupremerCourt enough how hard could it be lighten up how could you",
        "it's hopeless you hoser horrendous climate talk to my lawyer enough not that theres anything wrong",
        "I was sleeping nasty do you get a cookie foul job I m prettier than this man praise",
        "glorious Catastrophic Success far out man I don't care soap opera unsung hero hang in there",
        "a screw loose glorious not a chance in hell Greece rum bitty di vice are you feeling lucky",
        "King Midas catastrophe far out man you better not Yes you are vengeful Catastrophic Success",
        "thats right unemployment ouch you know a better God fun atheist joy",
        "'kay I don't care no more patience happy happy joy joy cowardice don't have a cow",
        "relax do I have to hard working happy happy joy joy ouch huh just lovely",
        "ahh thats much better courage China furious its trivial obviously straighten up what would Jesus do",
        "evolution SupremerCourt joy glorious exports hard working Oh Hell No",
        "Boo do not disturb radio smurfs reverse engineer biggot I don't care",
        "courage This is confusing Yawn ahh thats much better you talkin' to me I'm busy Terry",
        "Pullin the dragons tail don't mention it adultery what's up talk to my lawyer try again That's my favorite",
        "praise that's for me to know mission from God incoming endure You get what you pray for charity",
        "Pullin the dragons tail chill out do you get a cookie overflow You fix it what luck just lovely",
        "catastrophe let me count the ways Jesus food I forgot busybody so he sess",
        "what would Jesus do courage now you tell me can you hear me now Shhh rip off okay",
        "not too shabby food That's gonna leave a mark Yawn Ivy league sess me you're so screwed",
        "bye I am not amused unemployment figuratively really gambling look on the brightside",
        "umm what now bring it on petty Hicc up boink hobnob Varoom",
        "the quit ouch quite high mucky muck by the way study",
        "silly human poor I got your back handyman don't have a cow but of course I could swear",
        "One finger salute overflow won't you be my neighbor just lovely industrious Mars place",
        "oops an Irishman is forced to talk to God come and get me bye absolutely failure is not an option do you get a cookie",
        "not the sharpest knife in the drawer what's it to you the enquirer CIA 'kay do you have a problem run away",
        "who's to say zoot what a mess you talkin' to me laziness because I said so okay",
        "one more time ROFLMAO enough said frown happy happy joy joy Zzzzzzzz slumin",
        "nasty who are you to judge application are you insane how about that figuratively eh",
        "rubbish try again the wot courage I hate when that happens thats just wrong",
        "bye hey Mikey he likes it boink geek yep what a nightmare oh no",
        "praying the enquirer no you cant let's see fake nut job failure to communicate",
        "yuck 'kay are you feeling lucky high mucky muck refreshing love not the sharpest knife in the drawer",
        "if and only if unsung hero I'll ask nicely you're nuts pride wrath Zzzzzzzz",
        "shucks NeilDeGrasseTyson courage absolutely charity failure is not an option one more time",
        "by the way industrious boss epic fail oh oh Pope BRB",
        "I'm God and you're not my precious food duck the shoe special case where's the love in a perfect world",
        "adultery I'm impressed break some woopass on you wishful thinking sloth yikes This cant be william wallace",
        "you think I'm joking I donno fer sure computers it figures phasors on stun courage",
        "smurfs I didn't do it kick back catastrophe bickering church That's my favorite",
        "I veto that how could you God is not mocked okay rubbish harder than it looks voodoo",
        "caution Okilydokily really segway outrageous cosmetics thats right",
        "potentially look buddy holy grail joyful honestly pride look buddy",
        "pwned what luck repent lighten up BBC are you sure astrophysics",
        "by the way joy yeah birds naughty blessing whazza matter for you",
        "what's it to you grumble ha Hicc up huh endure money",
        "left field not the sharpest knife in the drawer patience crazy debt because I said so I made it that way",
        "strip wastoid red fang hang in there It grieves me you are my sunshine you'll see",
        "how could you frown you're in big trouble king of mars thats just wrong that's your opinion what planet are you from",
        "you think I'm joking I forgot Greek to me wonderful jobs spunky catastrophe",
        "Okilydokily Give me praise Shhh how high umm what now epic fail mine",
        "quite Wow Shhh driving wot exorbitant Church",
        "whatcha talkin' 'bout chaos look buddy husband good pow Shalom",
        "joking don't have a cow so let it be written you should be so lucky taxes wonderbread spirit",
        "radio dean scream slumin big fish begs the question unemployment red fang",
        "radio Is that your final answer how goes it where's the love unsung hero yep fool",
        "yeah ghetto pardon the french happy middle class what a mess Isn't that special",
        "incoming you better not husband hope driving Watch this thank you very much",
        "I didn't see that sex won't you be my neighbor What take your pick naughty delicious",
        "you're in big trouble hypocrite won't you be my neighbor not in kansas anymore angel joy look on the brightside",
        "money freak joyful bizarre ahh go ahead make my day HolySpirit",
        "Han shot first awesome CIA what's up king of mars what's the plan do you like it",
        "woot ridiculous in a perfect world in other words It's nice being God I was just thinking joker",
        "lying depressing gluttony thank you very much think you could do better charity rip off",
        "how come You da man gosh chaos what a mess frown vengeance",
        "when hell freezes over resume theft I had a crazy dream dude such a scoffer not good Wow",
        "in a perfect world rose colored glasses quite That's gonna leave a mark slumin That's my favorite I have an idea",
        "you don't say I'm not sure what a nightmare well I never be quiet bird fortitude when hell freezes over",
        "scum you're in big trouble you see the light I'm bored who are you to judge because I said so by the way",
        "nevada cheerful vermin threads boss Yes you are I planned that",
        "high mucky muck Isn't that special what a mess mine pet energy that's your opinion",
        "et tu who's to say tattle tale oh my I'm good you good you owe me yuck",
        "praying patience genius I'm in suspense how high Venus I didn't do it",
        "Terry the Mom rum bitty di do it Zap I veto that",
        "hotel I got your back on the otherhand not good chess chill out talk to my lawyer",
        "in a perfect world I'm on a roll Yawn rubbish boss hold on a minute sports",
        "Varoom it'd take a miracle ohh thank you naughty Terry make my day outrageous",
        "atrocious Icarus hate piety one small step phasors on stun take your pick",
        "whazza matter for you not a chance in hell ridiculous whoop there it is little fish hilarious close your eyes",
        "you'll see yep this might end badly news to me red fang that's for me to know you're nuts",
        "what part of God do you not understand what's it to you laziness I donno ha whale beam me up",
        "sess me yep joy hurts my head chaos be happy okay",
        "how about that Pullin the dragons tail prosperity mocking refreshing StephenHawking my bad",
        "boss quite beep beep study dang it population basket case",
        "hobnob no you cant employee jealousy one of the secret words are REMOTE lift uh huh are you deaf",
        "bickering skills thats laughable theres no place like home king of mars repeat after me go ahead make my day",
        "music you should be so lucky in theory no more tears do you know what time it is Angel it's hopeless",
        "couldnt possibly bad ol puddytat husband anger yep atheist et tu",
        "FBI energy lust well I never dance I'm the boss manufacturing",
        "think you could do better gluttony Shalom I didn't see that voodoo Han shot first how could you",
        "virtue experts just between us drama like like vengeance charity",
        "incredibly don't have a cow got the life Russia rufus! basically Is that so",
        "I planned that white trash failure to communicate check this out virtue crash and burn let's see",
        "check this out sloth news to me but of course NOT do it shucks",
        "It grieves me you're no fun cursing rufus! sess me rose colored glasses Church",
        "dance bizarre these cans are defective frown Knock you upside the head no more tears I am not amused",
        "manufacturing adjusted for inflation application Jedi mind trick do I have to praise Venus",
        "I'll let you know you're not all there are you I'm impressed talk to my lawyer abnormal This cant be william wallace frown",
        "Putin This cant be william wallace California rum bitty di end begs the question look buddy",
        "shist Greece failure to communicate you'll see rich left field Mom",
        "thats right you're wonderful you never know really that's your opinion what's up ice cream",
        "class  class  shutup tree hugger news to me just between us ROFLMAO not good not",
        "do it smile You fix it services liberal study I'm God and you're not",
        "chump change I'm feeling nice today thats just wrong you're fired it figures God smack Oy",
        "One finger salute ba ha won't you be my neighbor bring it on don't mention it talk to my lawyer exorbitant",
        "phasors on stun ohh thank you Yes you are how goes it nut job come and get me I got your back",
        "tattle tale you shouldn't have you're wonderful perfect Give me praise I veto that Is that so",
        "fabulous stuff pride Pope You know ordinarily ho ho ho",
        "ouch CIA study application phasors on stun not a chance in hell I'm not sure",
        "energy Isn't that special piety unsung hero guilty downer you owe me",
        "now you tell me no more hypocrite food one small step bad ol puddytat you're not all there are you",
        "depressing Ivy league I was just thinking umm I can't believe it ipod angel",
        "WooHoo place in theory strip African hello a flag on that play",
        "slumin grumble here now I'll get right on it frown If had my druthers over the top",
        "doh naughty joy NeilDeGrasseTyson sports nut job now you tell me",
        "commanded lust Yes you are don't worry recipe nope evolution",
        "manufacturing because I said so pride straighten up I'm on a roll quit it evolution",
        "Mom a likely story I'm off today Is that so don't mention it surprise surprise grumble",
        "arrogant won't you be my neighbor exports act yep Terry I have an idea",
        "reverse engineer I could be wrong news to me nope employee love foul",
        "conservative thank you very much commanded I'll let you know let me count the ways funny theres no place like home",
        "handyman yeah You get what you pray for whale gambling delightful sloth",
        "I'll think about it in theory awful Mom what a mess radio rum bitty di",
        "holy grail glam fortitude have fun depressing who are you to judge take your pick",
        "incoming in a galaxy far far away blessing spirit Pullin the dragons tail computers red fang",
        "beam me up Mom money boss fake prosperity scorning",
        "umm what now one more time nevada completely what's the plan rum bitty di no news is good news",
        "okay exorbitant hopefully mocking is it just me or I pity the fool that's your opinion",
        "because I said so kick back wot vote it's my world Pope charged",
        "money wazz up with that in other words I'm God who the hell are you tattle tale you're lucky don't count on it",
        "small talk genius lying here now mocking other smart",
        "you're lucky smurfs no way dude tree hugger abnormal You da man it's my world",
        "couldn't be better sloth look buddy we ve already got one holy grail take the day off ehheh that's all folks",
        "don't worry relax baffling whoop there it is phasors on stun lighten up I hate when that happens",
        "yeah illogical astrophysics not good busybody bye funny",
        "I hate when that happens food fancy it'd take a miracle shist pick me pick me sloth",
        "check this out wonderful ba ha Moses It's nice being God I don't care abnormal",
        "ipod here now one small step Ivy league that's your opinion you think I'm joking programming",
        "super computer happy GarryKasparov I be like smile God after a break",
        "Oh really it'd take a miracle nut job you owe me Pope holy grail dude such a scoffer",
        "genius humility California holier than thou persistence Isn't that special absetively posilutely",
        "desert break some woopass on you rufus! super computer stuff I'm thrilled the",
        "yep not too shabby voodoo you should be so lucky You da man boss Knock you upside the head",
        "joyful boss you're fired yada yada yada close your eyes look out you'll see",
        "Varoom food don't have a cow run away got the life You know stuff",
        "play is it just me or tiffanies vermin God is not mocked bad what luck",
        "by the way hotel pow study courage I can't believe it I pity the fool",
        "failure is not an option how hard could it be ridiculous what do you want nerd bring it on Dad",
        "spirit king of mars I'm off today threads oh oh what's the plan so he sess",
        "are you feeling lucky do not disturb here now bring it on Bam Dad red fang",
    ]);
    let mut joins = tokio::task::JoinSet::<ValidateResult>::new();
    let mut tasks = vec![];
    let views_url = Arc::new(views_url.clone());
    for i in 0..20 {
        let u = ws_base_url.clone();
        let ps = phrases.clone();
        let views_url = views_url.clone();
        let mut user = WS::new(test, format!("{}/19/ws/room/1/user/{}", u, i)).await?;
        tasks.push(async move {
            for (ii, p) in ps.iter().enumerate() {
                user.send_tweet(*p).await?;
                sleep(Duration::from_millis(1)).await;
                if i == 0 && ii == 100 {
                    let client = new_client();
                    client
                        .get(views_url.deref())
                        .send()
                        .await
                        .map_err(|_| test)?;
                }
            }
            sleep(Duration::from_secs(2)).await;
            user.close().await?;

            Ok(())
        });
    }
    for t in tasks.into_iter() {
        joins.spawn(t);
    }
    while let Some(Ok(r)) = joins.join_next().await {
        r?;
    }
    sleep(Duration::from_millis(100)).await;
    ensure_views(80000).await.map_err(|_| test)?;
    // TASK 2 DONE
    tx.send((false, 500).into()).await.unwrap();

    Ok(())
}

async fn validate_20(base_url: &str, tx: Sender<SubmissionUpdate>) -> ValidateResult {
    let client = new_client();
    let mut test: TaskTest;
    // TASK 1
    test = (1, 1);
    let url = &format!("{}/20/archive_files", base_url);
    let res = client
        .post(url)
        .body(include_bytes!("../assets/northpole20231220.tar").to_vec())
        .send()
        .await
        .map_err(|_| test)?;
    let text = res.text().await.map_err(|_| test)?;
    if text != "6" {
        return Err(test);
    }
    test = (1, 2);
    let url = &format!("{}/20/archive_files_size", base_url);
    let res = client
        .post(url)
        .body(include_bytes!("../assets/northpole20231220.tar").to_vec())
        .send()
        .await
        .map_err(|_| test)?;
    let text = res.text().await.map_err(|_| test)?;
    if text != "1196282" {
        return Err(test);
    }
    // TASK 1 DONE
    tx.send((true, 0).into()).await.unwrap();
    tx.send(SubmissionUpdate::Save).await.unwrap();

    // TASK 2
    test = (2, 1);
    let url = &format!("{}/20/cookie", base_url);
    let res = client
        .post(url)
        .body(include_bytes!("../assets/cookiejar.tar").to_vec())
        .send()
        .await
        .map_err(|_| test)?;
    let text = res.text().await.map_err(|_| test)?;
    if text != "Grinch 71dfab551a1958b35b7436c54b7455dcec99a12c" {
        return Err(test);
    }
    test = (2, 2);
    let url = &format!("{}/20/cookie", base_url);
    let res = client
        .post(url)
        .body(include_bytes!("../assets/lottery.tar").to_vec())
        .send()
        .await
        .map_err(|_| test)?;
    let text = res.text().await.map_err(|_| test)?;
    if text != "elf-27221 6342c1dbdb560f0d5dcaac7566fca51454866664" {
        return Err(test);
    }
    // TASK 2 DONE
    tx.send((false, 350).into()).await.unwrap();

    Ok(())
}

async fn validate_21(base_url: &str, tx: Sender<SubmissionUpdate>) -> ValidateResult {
    let client = new_client();
    let mut test: TaskTest;
    // TASK 1
    test = (1, 1);
    let url = &format!(
        "{}/21/coords/0100111110010011000110011001010101011111000010100011110001011011",
        base_url
    );
    let res = client.get(url).send().await.map_err(|_| test)?;
    let text = res.text().await.map_err(|_| test)?;
    if text != "83°39'54.324''N 30°37'40.584''W" {
        return Err(test);
    }
    test = (1, 2);
    let url = &format!(
        "{}/21/coords/0010000111110000011111100000111010111100000100111101111011000101",
        base_url
    );
    let res = client.get(url).send().await.map_err(|_| test)?;
    let text = res.text().await.map_err(|_| test)?;
    if text != "18°54'55.944''S 47°31'17.976''E" {
        return Err(test);
    }
    test = (1, 3);
    let url = &format!(
        "{}/21/coords/0101110100010001110001111100100111000111100010111100111101110001",
        base_url
    );
    let res = client.get(url).send().await.map_err(|_| test)?;
    let text = res.text().await.map_err(|_| test)?;
    if text != "51°26'57.804''N 99°28'33.204''E" {
        return Err(test);
    }
    // TASK 1 DONE
    tx.send((true, 0).into()).await.unwrap();
    tx.send(SubmissionUpdate::Save).await.unwrap();

    // TASK 2
    test = (2, 1);
    let url = &format!(
        "{}/21/country/0010000111110000011111100000111010111100000100111101111011000101",
        base_url
    );
    let res = client.get(url).send().await.map_err(|_| test)?;
    let text = res.text().await.map_err(|_| test)?;
    if text != "Madagascar" {
        return Err(test);
    }
    test = (2, 2);
    let url = &format!(
        "{}/21/country/0011001000100010100010110001110100000111000010111000100000010101",
        base_url
    );
    let res = client.get(url).send().await.map_err(|_| test)?;
    let text = res.text().await.map_err(|_| test)?;
    if text != "Brunei" {
        return Err(test);
    }
    test = (2, 3);
    let url = &format!(
        "{}/21/country/1001010011001110010011100110001000100110100111001001000100110001",
        base_url
    );
    let res = client.get(url).send().await.map_err(|_| test)?;
    let text = res.text().await.map_err(|_| test)?;
    if text != "Brazil" {
        return Err(test);
    }
    test = (2, 4);
    let url = &format!(
        "{}/21/country/0101110100010001110001111100100111000111100010111100111101110001",
        base_url
    );
    let res = client.get(url).send().await.map_err(|_| test)?;
    let text = res.text().await.map_err(|_| test)?;
    if text != "Mongolia" {
        return Err(test);
    }
    test = (2, 5);
    let url = &format!(
        "{}/21/country/0011100111101001000010001100001100111111101001100110000010101011",
        base_url
    );
    let res = client.get(url).send().await.map_err(|_| test)?;
    let text = res.text().await.map_err(|_| test)?;
    if text != "Nepal" {
        return Err(test);
    }
    test = (2, 6);
    let url = &format!(
        "{}/21/country/0100011111000110101110101100011001101001111111001011000011101111",
        base_url
    );
    let res = client.get(url).send().await.map_err(|_| test)?;
    let text = res.text().await.map_err(|_| test)?;
    if text != "Belgium" {
        return Err(test);
    }
    test = (2, 7);
    let url = &format!(
        "{}/21/country/0100111100110010101001010001010100100110110000100100101011011111",
        base_url
    );
    let res = client.get(url).send().await.map_err(|_| test)?;
    let text = res.text().await.map_err(|_| test)?;
    if text != "Iceland" {
        return Err(test);
    }
    // TASK 2 DONE
    tx.send((false, 300).into()).await.unwrap();

    Ok(())
}

struct TextTester {
    client: reqwest::Client,
    url: String,
}

impl TextTester {
    fn new(url: String) -> Self {
        Self {
            client: new_client(),
            url,
        }
    }
    async fn test(&self, test: TaskTest, i: &str, code: StatusCode, o: &str) -> ValidateResult {
        let res = self
            .client
            .post(&self.url)
            .body(i.to_owned())
            .send()
            .await
            .map_err(|_| test)?;
        if res.status() != code {
            return Err(test);
        }
        let text = res.text().await.map_err(|_| test)?;
        if text != o {
            return Err(test);
        }
        Ok(())
    }
}

async fn validate_22(base_url: &str, tx: Sender<SubmissionUpdate>) -> ValidateResult {
    // TASK 1
    let t = TextTester::new(format!("{}/22/integers", base_url));
    t.test(
        (1, 1),
        "\
1
",
        StatusCode::OK,
        "🎁".repeat(1).as_str(),
    )
    .await?;
    t.test(
        (1, 2),
        "\
1
1
2
2
3
3
4
",
        StatusCode::OK,
        "🎁".repeat(4).as_str(),
    )
    .await?;
    t.test(
        (1, 3),
        "\
1
3
1
2
4
2
3
",
        StatusCode::OK,
        "🎁".repeat(4).as_str(),
    )
    .await?;
    t.test(
        (1, 4),
        "\
11111111111111111111
555555555555555
33333333
68
555555555555555
33333333
4444
11111111111111111111
4444
",
        StatusCode::OK,
        "🎁".repeat(68).as_str(),
    )
    .await?;
    t.test(
        (1, 5),
        include_str!("../assets/numbers.txt"),
        StatusCode::OK,
        "🎁".repeat(120003).as_str(),
    )
    .await?;
    // TASK 1 DONE
    tx.send((true, 0).into()).await.unwrap();
    tx.send(SubmissionUpdate::Save).await.unwrap();

    // TASK 2
    let t = TextTester::new(format!("{}/22/rocket", base_url));
    t.test(
        (2, 1),
        "\
2
0 0 0
0 0 1
1
0 1
",
        StatusCode::OK,
        "1 1.000",
    )
    .await?;
    t.test(
        (2, 2),
        "\
5
0 1 0
-2 2 3
3 -3 -5
1 1 5
4 3 5
4
0 1
2 4
3 4
1 2
",
        StatusCode::OK,
        "3 26.123",
    )
    .await?;
    t.test(
        (2, 3),
        "\
5
0 1 0
-2 2 3
3 -3 -5
1 1 5
4 3 5
5
0 1
1 3
3 4
0 2
2 4
",
        StatusCode::OK,
        "2 18.776",
    )
    .await?;
    t.test(
        (2, 4),
        "\
5
0 1 0
-2 2 3
3 -3 -5
1 1 5
4 3 5
1
0 4
",
        StatusCode::OK,
        "1 6.708",
    )
    .await?;
    t.test(
        (2, 5),
        "\
5
0 1 0
-2 2 3
3 -3 -5
1 1 5
4 3 5
5
0 4
0 1
1 2
2 0
0 3
",
        StatusCode::OK,
        "1 6.708",
    )
    .await?;
    t.test(
        (2, 6),
        "\
21
570 -435 923
672 -762 -218
707 16 640
311 902 47
-963 -399 -773
788 532 -704
703 475 -145
-303 -394 -369
699 -640 952
-341 -221 743
740 -146 544
-424 655 179
-630 161 690
789 -848 -517
-14 -893 551
-48 815 962
528 552 -96
337 983 165
-565 459 -90
81 -476 301
-685 -319 698
24
0 2
2 4
4 6
6 10
10 17
17 20
20 18
18 11
11 7
7 5
5 3
3 0
0 1
1 12
12 13
13 19
19 20
20 16
16 14
14 15
15 9
9 8
8 6
11 16
",
        StatusCode::OK,
        "5 7167.055",
    )
    .await?;
    t.test(
        (2, 7),
        "\
75
570 -435 923
672 -762 -218
707 16 640
311 902 47
-963 -399 -773
788 532 -704
703 475 -145
-303 -394 -369
699 -640 952
-341 -221 743
740 -146 544
-424 655 179
-630 161 690
789 -848 -517
-14 -893 551
-48 815 962
528 552 -96
337 983 165
-565 459 -90
81 -476 301
-685 -319 698
-264 96 361
796 94 402
983 763 -953
711 -221 -866
-578 128 -178
-464 117 304
426 -433 -961
-626 -779 -596
-117 -88 349
880 -286 -527
941 -451 177
627 -832 286
593 370 -436
609 431 -681
-549 -690 447
957 849 -162
189 290 -485
-914 -447 -61
367 731 825
-177 432 -675
-926 -811 198
-379 345 831
-669 -134 804
956 380 -427
213 -954 -357
-806 -663 583
7 -460 374
-384 -797 -404
-793 -333 196
402 175 329
703 9 -926
599 559 -844
64 343 885
-865 -49 -373
-728 880 -164
830 528 -394
931 -782 -365
661 -528 931
-764 34 -289
442 298 983
-899 382 -967
662 361 -85
775 98 -519
202 335 60
474 823 -677
-708 41 127
-974 718 81
443 -526 -945
-279 778 -271
896 26 -902
-977 -233 837
151 -22 -454
824 -472 471
702 871 -244
73
0 1
0 2
0 4
0 5
0 7
1 10
10 11
11 25
12 13
13 27
14 29
15 30
16 17
17 35
18 36
19 6
2 3
20 22
21 19
22 40
23 60
24 42
25 26
26 43
27 28
28 45
29 47
3 12
30 31
31 68
32 50
34 16
35 52
36 54
37 38
38 57
39 21
4 14
40 59
41 23
42 61
43 44
44 63
45 64
46 65
47 46
49 32
49 68
5 15
50 33
51 34
52 70
54 73
55 37
56 74
57 56
58 39
59 58
6 18
63 62
65 66
66 67
67 48
69 51
7 20
70 71
71 72
72 53
73 55
8 1
8 9
9 23
9 24
",
        StatusCode::OK,
        "20 27826.439",
    )
    .await?;
    t.test(
        (2, 8),
        "\
70
788 532 -704
703 475 -145
-303 -394 -369
699 -640 952
-341 -221 743
740 -146 544
-424 655 179
-630 161 690
789 -848 -517
-14 -893 551
-48 815 962
528 552 -96
337 983 165
-565 459 -90
81 -476 301
-685 -319 698
-264 96 361
796 94 402
983 763 -953
711 -221 -866
-578 128 -178
-464 117 304
426 -433 -961
-626 -779 -596
-117 -88 349
880 -286 -527
941 -451 177
627 -832 286
593 370 -436
609 431 -681
-549 -690 447
957 849 -162
189 290 -485
-914 -447 -61
367 731 825
-177 432 -675
-926 -811 198
-379 345 831
-669 -134 804
956 380 -427
213 -954 -357
-806 -663 583
7 -460 374
-384 -797 -404
-793 -333 196
402 175 329
703 9 -926
599 559 -844
64 343 885
-865 -49 -373
-728 880 -164
830 528 -394
931 -782 -365
661 -528 931
-764 34 -289
442 298 983
-899 382 -967
662 361 -85
775 98 -519
202 335 60
474 823 -677
-708 41 127
-974 718 81
443 -526 -945
-279 778 -271
896 26 -902
-977 -233 837
151 -22 -454
824 -472 471
702 871 -244
70
0 10
0 2
0 3
1 22
10 21
11 1
12 11
13 27
14 15
15 4
16 31
17 33
18 19
19 7
2 12
20 53
21 37
22 39
23 40
24 23
25 24
26 25
27 26
27 6
28 14
29 30
3 13
30 46
30 69
31 47
33 18
33 48
34 33
35 34
37 20
38 55
39 56
39 57
4 16
40 59
41 60
42 62
43 63
44 28
44 65
45 29
46 68
47 32
48 49
49 50
5 17
50 51
51 35
53 36
55 54
56 38
57 58
59 41
6 5
60 61
61 42
62 43
63 64
64 44
65 45
67 66
68 67
7 9
8 0
9 8
",
        StatusCode::OK,
        "23 34029.320",
    )
    .await?;
    // TASK 2 DONE
    tx.send((false, 600).into()).await.unwrap();

    Ok(())
}
