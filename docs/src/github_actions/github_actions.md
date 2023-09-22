# GitHub Actions

There are a lot of fun ways to use Knope in GitHub Actions! This section will show you some common patterns—if you have any questions or suggestions, please open a [discussion](https://github.com/knope-dev/knope/discussions)!

## Installing Knope

Knope is available as a GitHub Action, so you can install it like this:

```yaml
- uses: knope-dev/action@v2.0.0
  with:
    version: 0.11.0
```

See more details and all available options in [the action repo](https://github.com/marketplace/actions/install-knope).

## Recipes

- [Trigger a release by manually running a GitHub Actions workflow](./workflow_dispatch.md)
- [Maintain a Pull Request which previews the release, trigger the release by merging it](./pull_request.md)
