---
title: ChangeSet
---

A set of [change files][change file] in the `.changeset` directory.
When creating a release, Knope combines every [change file] in the changeset (along with any [conventional commits])
to generate the changelog and decide the next version for each [package].

Check out these tutorials for hands-on experience with changesets:

- [Releasing basic projects](/tutorials/releasing-basic-projects)
- [Releasing multiple packages](/tutorials/releasing-multiple-packages)

:::note
Changesets are based on the NodeJS-oriented [Changesets](https://github.com/changesets/changesets)
and should be compatible if you are migrating from that project.
There are a few differences between the two—notably,
this project doesn't require a `.changeset/config.json` nor a `package.json` file
(it works for all languages, not just JavaScript, including Deno projects that only have `deno.json`).

For more on the differences, check out the [Rust changesets docs](https://github.com/knope-dev/changesets)
:::

[change file]: /reference/concepts/change-file
[conventional commits]: /reference/concepts/conventional-commits
[package]: /reference/concepts/package
