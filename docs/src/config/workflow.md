# Workflow

A workflow is the entrypoint to doing work with Dobby. Once you start running `dobby` you must immediately select a workflow (by name) to be executed.

Each workflow is defined in the `[[workflows]]` array in your [dobby.toml][config] file. Each entry contains a `name` attribute which is how the workflow will be displayed when running `dobby`. There is also an array of [steps][step] declared as `[[workflows.steps]]` which define the individual actions to take.

## Example

```toml
# dobby.toml

[[workflows]]
name = "My First Workflow"
    [[workflows.steps]]
    # First step details here
    [[workflows.steps]]
    # second step details here
```

## See Also

- [Step] for details on how each `[[workflows.steps]]` is defined.

[config]: ./config.md
[step]: ./step/step.md
