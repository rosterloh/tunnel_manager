# Gardin Tunnel Manager

Application to connect to the localproxy tunnel

Tasks are defined in `mise.toml`; run `mise tasks` to list them.

### Testing

The mock-based tests need the `test-utils` feature, which `mise run test` enables.

```shell
mise run test

# Run tests whose name matches a filter
mise run test login
```

### Packaging

```shell
mise run package
```

### Updating localproxy

The binaries in `bin/` are built from
[aws-iot-securetunneling-localproxy](https://github.com/aws-samples/aws-iot-securetunneling-localproxy).

```shell
# Linux: builds in Docker from a local checkout (../aws-iot-securetunneling-localproxy
# by default, or use --source / LOCALPROXY_SOURCE)
mise run localproxy:build v3.3.1

# Windows: needs MSVC, so this builds on GitHub Actions and downloads the result
mise run localproxy:windows v3.3.1
```
