#[derive(Debug)]
pub enum SubmissionState {
    Waiting,
    Running,
    Done,
    Error,
}
impl std::fmt::Display for SubmissionState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

#[derive(Debug)]
pub enum SubmissionUpdate {
    /// State update
    State(SubmissionState),
    /// bool is true if this task was the last core task, int is amount of bonus points
    TaskCompleted(bool, i32),
    /// Append line to log
    LogLine(String),
    /// Save changes to db
    Save,
}
impl From<SubmissionState> for SubmissionUpdate {
    fn from(value: SubmissionState) -> Self {
        Self::State(value)
    }
}
impl From<(bool, i32)> for SubmissionUpdate {
    fn from((b, i): (bool, i32)) -> Self {
        Self::TaskCompleted(b, i)
    }
}
impl From<String> for SubmissionUpdate {
    fn from(value: String) -> Self {
        Self::LogLine(value)
    }
}
