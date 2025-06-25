# Gardin Tunnel Manager

Application to connect to the localproxy tunnel

### Testing

To run tests use the `test-utils` feature

```shell
# Run all tests with mocking enabled
cargo test --features test-utils

# Run specific test categories
cargo test error_tests
cargo test integration_tests
cargo test aws_business_logic_tests
cargo test performance_tests
```