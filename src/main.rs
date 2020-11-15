use color_eyre::eyre::{Result, WrapErr};
use dotenv::dotenv;

use flow::{run_workflow, select, Config, State};

fn main() -> Result<()> {
    color_eyre::install().unwrap();
    dotenv().ok();

    let Config { workflows, jira } =
        Config::load("flow.toml").wrap_err("Could not load config file at flow.toml")?;
    let workflow = select(workflows, "Select a workflow")?;
    let state = State::new(jira);
    run_workflow(workflow, state)
}
