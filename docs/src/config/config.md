# Config

This is the top level structure that your `knope.toml` must adhere to to be valid. If you don't have a `knope.toml` in the working directory, or it isn't valid, you'll get an error right off the bat.

## Example

```toml
[[workflows]]
name = "First Workflow"
# Details here

[[workflows]]
name = "Second Workflow"
# Details here

[jira]
# Jira config here

[github]
# GitHub config here
```

When you first start `knope`, you will be asked to select a [workflow] to run. In the above example, this would look something like:

```
? Select a workflow
> First Workflow
  Second Workflow
```

You can use your arrow keys to then select an option to run. The `>` symbol indicates which workflow is selected. Pressing the `Enter` key on your keyboard will run the workflow.

## See Also

- [Workflows][workflow] for details on defining entries to the `[[workflows]]` array
- [Jira](./jira.md) for details on defining `[jira]`
- [GitHub](./github.md) for details on defining `[github]`

[workflow]: ./workflow.md
