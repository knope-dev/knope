---
default: docs
---

#### Document conflict between package names and go module names

It is possible to write a `knope.toml` file which will cause conflicting tags during the `Release` step if you have `go.mod` files in nested directories. [This is now documented](https://knope-dev.github.io/knope/config/step/Release.html).
