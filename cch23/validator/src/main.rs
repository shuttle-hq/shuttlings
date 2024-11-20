use cch23_validator::{args::ValidatorArgs, run, SUPPORTED_CHALLENGES};
use clap::{CommandFactory, FromArgMatches};
use shuttlings::{SubmissionState, SubmissionUpdate};
use uuid::Uuid;

#[tokio::main]
async fn main() {
    let c = ValidatorArgs::command();
    let m = c
        .mut_arg("numbers", |a| a.allow_negative_numbers(true))
        .get_matches();
    let args = ValidatorArgs::from_arg_matches(&m).unwrap();

    println!(
        "\
⋆｡°✩ ⋆⁺｡˚⋆˙‧₊✩₊‧˙⋆˚｡⁺⋆ ✩°｡⋆°✩ ⋆⁺｡˚⋆˙‧₊✩₊‧˙⋆˚｡⁺⋆ ✩°｡⋆
.・゜゜・・゜゜・．                .・゜゜・・゜゜・．
｡･ﾟﾟ･          SHUTTLE CCH23 VALIDATOR          ･ﾟﾟ･｡
.・゜゜・・゜゜・．                .・゜゜・・゜゜・．
⋆｡°✩ ⋆⁺｡˚⋆˙‧₊✩₊‧˙⋆˚｡⁺⋆ ✩°｡⋆°✩ ⋆⁺｡˚⋆˙‧₊✩₊‧˙⋆˚｡⁺⋆ ✩°｡⋆
"
    );

    let (tx, mut rx) = tokio::sync::mpsc::channel::<SubmissionUpdate>(32);

    let get_printer = |summary: bool| async move {
        let mut tasks_completed = 0;
        let mut days_completed = 0;
        let mut bonus = 0;
        while let Some(s) = rx.recv().await {
            match s {
                SubmissionUpdate::State(state) => {
                    match state {
                        SubmissionState::Done => {
                            tasks_completed = 0;
                        }
                        _ => (),
                    };
                }
                SubmissionUpdate::TaskCompleted(completed, bp) => {
                    tasks_completed += 1;
                    println!("Task {}: completed 🎉", tasks_completed);
                    if bp > 0 {
                        bonus += bp;
                        println!("Bonus points: {} ✨", bp);
                    }
                    if completed {
                        days_completed += 1;
                        println!("Core tasks completed ✅");
                    }
                }
                SubmissionUpdate::LogLine(line) => {
                    println!("{line}");
                }
                _ => (),
            }
        }
        if summary {
            println!();
            println!();
            println!(
                "Completed {} challenges and gathered a total of {} bonus points.",
                days_completed, bonus
            );
        }
    };

    let nums = if !args.challenge.numbers.is_empty() {
        args.challenge.numbers.as_ref()
    } else {
        SUPPORTED_CHALLENGES
    };

    let printer = tokio::task::spawn(get_printer(nums.len() > 1));

    for num in nums {
        println!();
        println!("Validating Challenge {num}...");
        println!();
        run(
            args.url.trim_end_matches('/').to_owned(),
            Uuid::nil(),
            *num,
            tx.clone(),
        )
        .await;
        // give the receiver time to print everything from the previous challenge
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    }

    drop(tx);
    printer.await.unwrap();
}
