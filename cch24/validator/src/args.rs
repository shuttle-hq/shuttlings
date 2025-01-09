use clap::{Args, Parser};

#[derive(Debug, Parser)]
#[command(version)]
pub struct ValidatorArgs {
    #[command(flatten)]
    pub challenge: ChallengeArgs,
    /// The base URL to test against
    #[arg(long, short, default_value = "http://127.0.0.1:8000")]
    pub url: String,
}

#[derive(Debug, Clone, Args)]
#[group(required = true, multiple = false)]
pub struct ChallengeArgs {
    /// The challenge numbers to validate
    pub numbers: Vec<String>,
    /// Validate all challenges
    #[arg(long)]
    pub all: bool,
}
