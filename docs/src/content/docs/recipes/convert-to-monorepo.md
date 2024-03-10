1. Reorganize your existing package into the new structure.
2. Replace `package` with `packages.<package-name>` in `knope.toml`
3. Update `versioned_files` and `changelog`, make sure any custom commands will still work.
4. Create a tag on the same commit as the previous release with `<package-name>/v<version>`
5. Add in the new package
6. Create an initial version tag for the new package (e.g., `knope-versioned-files/v0.0.0`)